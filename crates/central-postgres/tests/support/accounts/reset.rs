//! Migration 000013's reset codes: who may make one, what a code finds, and
//! what keeping or renewing the vault changes and keeps; a spent code, a
//! password change and an expiry each end it.
use super::{
    AccountStore, Enrollment, Error, Identity, MasterKey, ObjectContext, ServerKeys, Token,
    context, enroll, login,
};
use central_api::{reset_code::ResetCode, wire::ResetVault, wire::TicketPurpose};
use central_crypto::vault::RecoveryCode;
use central_postgres::{Credential, Session, Swept, appoint_admin, sweep};
use sea_orm::{ConnectionTrait, DatabaseConnection};

async fn count(db: &impl ConnectionTrait, sql: &str) -> i64 {
    crate::scalar(db, sql).await
}

fn uuid(identity: Identity) -> String {
    sea_orm::prelude::Uuid::from_bytes(identity.account).to_string()
}

async fn browser(store: &AccountStore, identity: Identity) -> Token {
    let credential = store.credential(identity).await.unwrap().unwrap();
    let token = Token::generate().unwrap();
    store
        .issue_session(&credential, Session::Browser(&token))
        .await
        .unwrap();
    token
}

/// What a reset may change, as one comparable row.
#[derive(Debug, PartialEq, Eq)]
struct State {
    revision: i64,
    epoch: i64,
    document_revision: i64,
    document: Vec<u8>,
    recovery: Option<Vec<u8>>,
    restored: bool,
    objects: i64,
    object_bytes: i64,
    sessions: i64,
    code: bool,
}

async fn state(operator: &DatabaseConnection, identity: Identity) -> State {
    let id = uuid(identity);
    let row = operator
        .query_one_raw(sea_orm::Statement::from_string(
            sea_orm::DbBackend::Postgres,
            format!(
                "SELECT credential_revision, vault_epoch, document_revision, document,
                 recovery_wrapped_master, restored, object_bytes, reset_digest IS NOT NULL AS code,
                 (SELECT count(*) FROM central_identity.private_objects WHERE account_id = '{id}') AS objects,
                 (SELECT count(*) FROM central_identity.sessions WHERE account_id = '{id}') AS sessions
                 FROM central_identity.accounts WHERE id = '{id}'"
            ),
        ))
        .await
        .unwrap()
        .unwrap();
    State {
        revision: row.try_get("", "credential_revision").unwrap(),
        epoch: row.try_get("", "vault_epoch").unwrap(),
        document_revision: row.try_get("", "document_revision").unwrap(),
        document: row.try_get("", "document").unwrap(),
        recovery: row.try_get("", "recovery_wrapped_master").unwrap(),
        restored: row.try_get("", "restored").unwrap(),
        objects: row.try_get("", "objects").unwrap(),
        object_bytes: row.try_get("", "object_bytes").unwrap(),
        sessions: row.try_get("", "sessions").unwrap(),
        code: row.try_get("", "code").unwrap(),
    }
}

/// A new password's record, and a master key's wrapper, document and
/// recovery wrapper, for `identity` (whose epoch may be the next one).
/// The password is the one `login` signs in with.
struct Vault {
    record: Vec<u8>,
    wrapped: Vec<u8>,
    document: Vec<u8>,
    recovery: Vec<u8>,
}

fn vault(keys: &ServerKeys, identity: Identity) -> Vault {
    let context = context(identity);
    let (enrollment, request) =
        Enrollment::start("synthetic strong password".into(), context).unwrap();
    let response = keys.enrollment_response(context, &request).unwrap();
    let enrolled = enrollment.finish(&response).unwrap();
    let master = MasterKey::generate().unwrap();
    let document = master
        .seal_object(
            ObjectContext::new(context, identity.account, 1).unwrap(),
            b"a new vault",
        )
        .unwrap();
    let recovery = RecoveryCode::generate()
        .unwrap()
        .wrap(context, &master)
        .unwrap();
    Vault {
        record: enrolled.upload,
        wrapped: enrolled
            .wrapping_key
            .wrap(&master)
            .unwrap()
            .as_bytes()
            .to_vec(),
        document,
        recovery: recovery.as_bytes().to_vec(),
    }
}

fn next(identity: Identity) -> Identity {
    Identity {
        account: identity.account,
        epoch: identity.epoch + 1,
    }
}

pub(super) async fn run(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let admin = enroll(store, keys, "reset-admin", "reset-admin").await;
    appoint_admin(operator, "reset-admin").await.unwrap();
    let cookie = browser(store, admin).await;
    let moved = enroll(store, keys, "reset-moved", "reset-moved").await;
    let native = login(store, keys, moved).await;
    store.confirm_restore(&native, 1).await.unwrap();
    store.close_legacy("reset-moved", moved).await.unwrap();
    issuing(store, operator, keys, admin, moved, &cookie).await;
    keep(store, operator, keys, moved, &cookie).await;
    renewed_registered(store, operator, keys, &cookie).await;
    renewed_moved(store, operator, keys, moved, &cookie).await;
    ended(store, operator, keys, moved, &cookie).await;
}

/// Who may make a code, for whom; a new one replaces the old.
async fn issuing(
    store: &AccountStore,
    operator: &DatabaseConnection,
    keys: &ServerKeys,
    admin: Identity,
    moved: Identity,
    cookie: &Token,
) {
    let code = ResetCode::generate().unwrap();
    let open = enroll(store, keys, "reset-open", "reset-open").await;
    let other = enroll(store, keys, "reset-other", "reset-other").await;
    let other_native = login(store, keys, other).await;
    store.confirm_restore(&other_native, 1).await.unwrap();
    store.close_legacy("reset-other", other).await.unwrap();
    appoint_admin(operator, "reset-other").await.unwrap();
    let pending = Token::generate().unwrap();
    let pending = store
        .reserve_legacy("reset-pending", "reset-pending", None, None, &pending)
        .await
        .unwrap();
    let events = "SELECT count(*)::bigint AS value FROM central_identity.audit_events
                  WHERE action = 'account.reset_issued'";
    let before = count(operator, events).await;
    // Oneself, another administrator, an open move, a pending or unknown account.
    for target in [admin, other, open, pending] {
        assert_eq!(
            store
                .issue_reset(cookie, target.account, &code.digest())
                .await
                .err(),
            Some(Error::Rejected)
        );
    }
    assert_eq!(
        store
            .issue_reset(cookie, [9; 16], &code.digest())
            .await
            .err(),
        Some(Error::Rejected)
    );
    // No administrator, a native bearer, a stale sign-in.
    let plain = browser(store, moved).await;
    let native = Token::generate().unwrap();
    for token in [&plain, &native] {
        assert_eq!(
            store
                .issue_reset(token, moved.account, &code.digest())
                .await
                .err(),
            Some(Error::Rejected)
        );
    }
    let aged = format!(
        "UPDATE central_identity.sessions SET created_at = created_at - interval '31 minutes'
         WHERE account_id = '{}' AND kind = 'browser'",
        uuid(admin)
    );
    operator.execute_unprepared(&aged).await.unwrap();
    assert_eq!(
        store
            .issue_reset(cookie, moved.account, &code.digest())
            .await
            .err(),
        Some(Error::Rejected)
    );
    operator
        .execute_unprepared(&aged.replace("- interval", "+ interval"))
        .await
        .unwrap();
    assert_eq!(
        count(operator, events).await,
        before,
        "refusals write nothing"
    );
    issued(store, operator, moved, cookie, &code, before).await;
}

/// A code made: its audit event, the account row's expiry, what it finds;
/// a second replaces it, and issuing ended no session.
async fn issued(
    store: &AccountStore,
    operator: &DatabaseConnection,
    moved: Identity,
    cookie: &Token,
    code: &ResetCode,
    before: i64,
) {
    let events = "SELECT count(*)::bigint AS value FROM central_identity.audit_events
                  WHERE action = 'account.reset_issued'";
    // A code, its audit event, and the account row's expiry.
    let issued = store
        .issue_reset(cookie, moved.account, &code.digest())
        .await
        .unwrap();
    assert_eq!(count(operator, events).await, before + 1);
    let (row, ..) = store.admin_account(cookie, moved.account).await.unwrap();
    assert_eq!(row.reset_until, Some(issued));
    let in_a_day = count(
        operator,
        "SELECT floor(extract(epoch FROM clock_timestamp() + interval '24 hours'))::bigint AS value",
    )
    .await;
    assert!((in_a_day - 5..=in_a_day).contains(&issued));
    // It finds the account; a wrong code or name does not.
    let (credential, recovery) = store
        .reset_credential("Reset-Moved", &code.digest())
        .await
        .unwrap();
    assert_eq!(credential.identity.account, moved.account);
    assert!(recovery.is_none(), "a moved account without a code");
    for (name, digest) in [
        ("reset-moved", ResetCode::generate().unwrap().digest()),
        ("reset-other", code.digest()),
        ("x", code.digest()),
    ] {
        assert_eq!(
            store.reset_credential(name, &digest).await.err(),
            Some(Error::Rejected)
        );
    }
    // A second code replaces the first.
    let second = ResetCode::generate().unwrap();
    store
        .issue_reset(cookie, moved.account, &second.digest())
        .await
        .unwrap();
    assert!(
        store
            .reset_credential("reset-moved", &code.digest())
            .await
            .is_err()
    );
    assert!(
        store
            .reset_credential("reset-moved", &second.digest())
            .await
            .is_ok()
    );
    // Issuing ended nothing.
    assert_eq!(state(operator, moved).await.sessions, 2);
}

/// Issue a code for `target` and read what it finds.
async fn code(
    store: &AccountStore,
    cookie: &Token,
    target: Identity,
    name: &str,
) -> ([u8; 32], Credential) {
    let code = ResetCode::generate().unwrap();
    store
        .issue_reset(cookie, target.account, &code.digest())
        .await
        .unwrap();
    let (credential, _) = store.reset_credential(name, &code.digest()).await.unwrap();
    (code.digest(), credential)
}

/// Keeping the vault: the credential changes, everything encrypted stays.
async fn keep(
    store: &AccountStore,
    operator: &DatabaseConnection,
    keys: &ServerKeys,
    moved: Identity,
    cookie: &Token,
) {
    let (digest, credential) = code(store, cookie, moved, "reset-moved").await;
    let before = state(operator, moved).await;
    let new = vault(keys, moved);
    for (record, wrapped) in [
        (&new.record[1..], new.wrapped.as_slice()),
        (new.record.as_slice(), &new.wrapped[1..]),
    ] {
        assert_eq!(
            store
                .finish_reset(&credential, &digest, record, wrapped, &ResetVault::Keep)
                .await
                .err(),
            Some(Error::Rejected)
        );
    }
    let stale = Credential {
        identity: moved,
        revision: credential.revision + 1,
        record: credential.record.clone(),
    };
    assert_eq!(
        store
            .finish_reset(
                &stale,
                &digest,
                &new.record,
                &new.wrapped,
                &ResetVault::Keep
            )
            .await
            .err(),
        Some(Error::Rejected)
    );
    assert_eq!(
        state(operator, moved).await,
        before,
        "refusals change nothing"
    );
    store
        .finish_reset(
            &credential,
            &digest,
            &new.record,
            &new.wrapped,
            &ResetVault::Keep,
        )
        .await
        .unwrap();
    let after = state(operator, moved).await;
    assert_eq!(
        after,
        State {
            revision: before.revision + 1,
            sessions: 0,
            code: false,
            ..before
        }
    );
    // The code is spent: the same finish again, and the start, are refused.
    assert_eq!(
        store
            .finish_reset(
                &credential,
                &digest,
                &new.record,
                &new.wrapped,
                &ResetVault::Keep
            )
            .await
            .err(),
        Some(Error::Rejected)
    );
    assert!(
        store
            .reset_credential("reset-moved", &digest)
            .await
            .is_err()
    );
    // The account's credential is the new record. (This test's wrapper holds
    // a key of its own, so it signs in only after the new vault below.)
    assert_eq!(
        store.credential(moved).await.unwrap().unwrap().record,
        new.record
    );
}

/// A registered account's new vault: next epoch, new document and recovery
/// wrapper, no objects, an empty quota, and no gateway ticket until a native
/// client confirms the new document.
async fn renewed_registered(
    store: &AccountStore,
    operator: &DatabaseConnection,
    keys: &ServerKeys,
    cookie: &Token,
) {
    let key = super::register::issue(operator, 1, "reset-registered").await;
    let ticket = Token::generate().unwrap();
    let identity = store
        .register("reset-registered", None, &key.digest(), &ticket)
        .await
        .unwrap();
    let enrolled = super::register::credentials(keys, identity);
    store
        .enroll(
            &ticket,
            &enrolled.upload,
            &enrolled.wrapped,
            &enrolled.document,
            Some(&enrolled.recovery),
        )
        .await
        .unwrap();
    let native = login(store, keys, identity).await;
    store.confirm_restore(&native, 1).await.unwrap();
    let object = store.create_object(&native).await.unwrap();
    store
        .write_object(&native, object.id, 0, &[1; 64])
        .await
        .unwrap();
    let (digest, credential) = code(store, cookie, identity, "reset-registered").await;
    let before = state(operator, identity).await;
    assert!(before.objects == 1 && before.object_bytes > 0 && before.restored);
    let new = vault(keys, next(identity));
    let renew =
        |recovery: Option<Vec<u8>>, document: Vec<u8>| ResetVault::New { document, recovery };
    // A registered account needs its new recovery wrapper; a document and a
    // wrapper must be well-formed.
    for refused in [
        renew(None, new.document.clone()),
        renew(Some(new.recovery[1..].to_vec()), new.document.clone()),
        renew(Some(new.recovery.clone()), vec![1; 40]),
    ] {
        assert_eq!(
            store
                .finish_reset(&credential, &digest, &new.record, &new.wrapped, &refused)
                .await
                .err(),
            Some(Error::Rejected)
        );
    }
    assert_eq!(state(operator, identity).await, before);
    store
        .finish_reset(
            &credential,
            &digest,
            &new.record,
            &new.wrapped,
            &renew(Some(new.recovery.clone()), new.document.clone()),
        )
        .await
        .unwrap();
    assert_eq!(
        state(operator, identity).await,
        State {
            revision: before.revision + 1,
            epoch: before.epoch + 1,
            document_revision: 1,
            document: new.document.clone(),
            recovery: Some(new.recovery.clone()),
            restored: false,
            objects: 0,
            object_bytes: 0,
            sessions: 0,
            code: false,
        }
    );
    // The old identity finds nothing; the new one signs in, takes no ticket
    // until it confirms, then does.
    assert!(store.credential(identity).await.unwrap().is_none());
    let session = login(store, keys, next(identity)).await;
    let ticket = || Token::generate().unwrap();
    assert_eq!(
        store
            .gateway_ticket(&session, "official", &ticket(), TicketPurpose::Admit)
            .await
            .err(),
        Some(Error::Rejected)
    );
    store.confirm_restore(&session, 1).await.unwrap();
    store
        .gateway_ticket(&session, "official", &ticket(), TicketPurpose::Admit)
        .await
        .unwrap();
}

/// A moved account's new vault keeps `restored` (its migration is closed)
/// and may go without a recovery wrapper.
async fn renewed_moved(
    store: &AccountStore,
    operator: &DatabaseConnection,
    keys: &ServerKeys,
    moved: Identity,
    cookie: &Token,
) {
    let (digest, credential) = code(store, cookie, moved, "reset-moved").await;
    let before = state(operator, moved).await;
    let new = vault(keys, next(moved));
    store
        .finish_reset(
            &credential,
            &digest,
            &new.record,
            &new.wrapped,
            &ResetVault::New {
                document: new.document.clone(),
                recovery: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        state(operator, moved).await,
        State {
            revision: before.revision + 1,
            epoch: before.epoch + 1,
            document_revision: 1,
            document: new.document.clone(),
            recovery: None,
            restored: true,
            objects: 0,
            object_bytes: 0,
            sessions: 0,
            code: false,
        }
    );
    login(store, keys, next(moved)).await;
}

/// A password change clears an outstanding code; so does its expiry.
async fn ended(
    store: &AccountStore,
    operator: &DatabaseConnection,
    keys: &ServerKeys,
    moved: Identity,
    cookie: &Token,
) {
    let moved = next(moved);
    let (digest, _) = code(store, cookie, moved, "reset-moved").await;
    let session = login(store, keys, moved).await;
    let credential = store
        .password_credential(Session::Native(&session))
        .await
        .unwrap();
    let new = vault(keys, moved);
    let after_change = Token::generate().unwrap();
    store
        .change_password(
            Session::Native(&session),
            &credential,
            &new.record,
            &new.wrapped,
            Session::Native(&after_change),
        )
        .await
        .unwrap();
    assert!(!state(operator, moved).await.code);
    assert!(
        store
            .reset_credential("reset-moved", &digest)
            .await
            .is_err()
    );
    // An expired code finds nothing.
    let (digest, _) = code(store, cookie, moved, "reset-moved").await;
    operator
        .execute_unprepared(&format!(
            "UPDATE central_identity.accounts SET reset_expires = clock_timestamp() - interval '1 second'
             WHERE id = '{}'",
            uuid(moved)
        ))
        .await
        .unwrap();
    assert!(
        store
            .reset_credential("reset-moved", &digest)
            .await
            .is_err()
    );
    // The operator's sweep clears the expired digest.
    assert!(state(operator, moved).await.code);
    assert_eq!(
        sweep(operator).await,
        Ok(Swept {
            events: 0,
            codes: 1
        })
    );
    assert!(!state(operator, moved).await.code);
}
