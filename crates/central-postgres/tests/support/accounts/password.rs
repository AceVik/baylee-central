//! The store's half of a password change: whose credential a session may
//! change, and what one commit replaces, ends and keeps.
use super::{
    AccountStore, Credential, Enrollment, Error, Identity, MasterKey, ServerKeys, Token, config,
    context, enroll, login,
};
use central_postgres::Session;
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DbBackend, Statement, TransactionTrait, prelude::Uuid,
};
use std::time::Duration;

/// A new record and a wrapper for it, as a client makes them.
fn upload(keys: &ServerKeys, identity: Identity) -> (Vec<u8>, Vec<u8>) {
    let context = context(identity);
    let (enrollment, request) =
        Enrollment::start("synthetic changed password".into(), context).unwrap();
    let response = keys.enrollment_response(context, &request).unwrap();
    let enrolled = enrollment.finish(&response).unwrap();
    let wrapped = enrolled
        .wrapping_key
        .wrap(&MasterKey::generate().unwrap())
        .unwrap();
    (enrolled.upload, wrapped.as_bytes().to_vec())
}

async fn state(operator: &DatabaseConnection, identity: Identity) -> (i64, i64, i64, Vec<u8>, i64) {
    let account = Uuid::from_bytes(identity.account);
    let row = operator
        .query_one_raw(sea_orm::Statement::from_string(
            sea_orm::DbBackend::Postgres,
            format!(
                "SELECT credential_revision, vault_epoch, document_revision, document,
                 (SELECT count(*) FROM central_identity.sessions WHERE account_id = '{account}') AS sessions
                 FROM central_identity.accounts WHERE id = '{account}'"
            ),
        ))
        .await
        .unwrap()
        .unwrap();
    (
        row.try_get("", "credential_revision").unwrap(),
        row.try_get("", "vault_epoch").unwrap(),
        row.try_get("", "document_revision").unwrap(),
        row.try_get("", "document").unwrap(),
        row.try_get("", "sessions").unwrap(),
    )
}

pub(super) async fn run(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let legacy = "password-store";
    let identity = enroll(store, keys, legacy, "password-store").await;
    let native = login(store, keys, identity).await;
    store.confirm_restore(&native, 1).await.unwrap();
    let browser = Token::generate().unwrap();
    let credential = store.credential(identity).await.unwrap().unwrap();
    store
        .issue_session(&credential, Session::Browser(&browser))
        .await
        .unwrap();
    // While the legacy move is open, no password changes here.
    assert_eq!(
        store
            .password_credential(Session::Native(&native))
            .await
            .err(),
        Some(Error::Rejected)
    );
    store.close_legacy(legacy, identity).await.unwrap();
    let credential = store
        .password_credential(Session::Native(&native))
        .await
        .unwrap();
    assert_eq!(credential.identity.account, identity.account);
    store
        .password_credential(Session::Browser(&browser))
        .await
        .unwrap();
    // A token is a session of its own kind only, and an unknown one none.
    for session in [
        Session::Browser(&native),
        Session::Native(&browser),
        Session::Native(&Token::generate().unwrap()),
    ] {
        assert_eq!(
            store.password_credential(session).await.err(),
            Some(Error::Rejected)
        );
    }
    let before = state(operator, identity).await;
    let (record, wrapped) = upload(keys, identity);
    let next = Token::generate().unwrap();
    let stranger = refusals(store, keys, &credential, &native, &record, &wrapped, &next).await;
    assert_eq!(state(operator, identity).await, before, "nothing changed");
    // The change: both sessions end, one new one of the caller's kind lives.
    let issued = store
        .change_password(
            Session::Native(&native),
            &credential,
            &record,
            &wrapped,
            Session::Native(&next),
        )
        .await
        .unwrap();
    assert_eq!(issued.wrapped_master, wrapped);
    assert_eq!(issued.username, "password-store");
    let after = state(operator, identity).await;
    assert_eq!(after.0, before.0 + 1, "the credential revision moved");
    assert_eq!(
        (after.1, after.2, &after.3),
        (before.1, before.2, &before.3)
    );
    assert_eq!((before.4, after.4), (2, 1));
    for gone in [Session::Native(&native), Session::Browser(&browser)] {
        assert_eq!(
            store.session_status(gone).await.err(),
            Some(Error::Rejected)
        );
    }
    let status = store.session_status(Session::Native(&next)).await.unwrap();
    assert_eq!(status.identity.account, identity.account);
    let changed = store.credential(identity).await.unwrap().unwrap();
    assert_eq!(
        (changed.revision, changed.record),
        (after.0, record.clone())
    );
    // The credential the change replaced changes nothing again, with the new session.
    let again = Token::generate().unwrap();
    assert_eq!(
        store
            .change_password(
                Session::Native(&next),
                &credential,
                &record,
                &wrapped,
                Session::Native(&again),
            )
            .await
            .err(),
        Some(Error::Rejected)
    );
    // The stranger's own session is untouched.
    store.session_status(&stranger).await.unwrap();
    replaced_while_waiting(store, operator, keys, identity).await;
}

/// A change that waited on the account's lock while its session was
/// replaced commits nothing: the session is checked again after the lock.
async fn replaced_while_waiting(
    store: &AccountStore,
    operator: &DatabaseConnection,
    keys: &ServerKeys,
    identity: Identity,
) {
    let observer =
        central_postgres::connect_dev(config("central_runtime", "CENTRAL_TEST_RUNTIME_PASSWORD"))
            .await
            .unwrap();
    let session = Token::generate().unwrap();
    let credential = store.credential(identity).await.unwrap().unwrap();
    store.issue_session(&credential, &session).await.unwrap();
    let credential = store
        .password_credential(Session::Native(&session))
        .await
        .unwrap();
    let before = state(operator, identity).await;
    let (record, wrapped) = upload(keys, identity);
    let tx = operator.begin().await.unwrap();
    tx.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT id FROM central_identity.accounts WHERE id = $1 FOR UPDATE",
        vec![Uuid::from_bytes(identity.account).into()],
    ))
    .await
    .unwrap()
    .unwrap();
    let waiting = store.clone();
    let waiter = tokio::spawn(async move {
        let next = Token::generate().unwrap();
        waiting
            .change_password(
                Session::Native(&session),
                &credential,
                &record,
                &wrapped,
                Session::Native(&next),
            )
            .await
            .map(|_| ())
    });
    super::revocation_race::wait_for_account_lock(&observer).await;
    tx.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "DELETE FROM central_identity.sessions WHERE account_id = $1",
        vec![Uuid::from_bytes(identity.account).into()],
    ))
    .await
    .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(5), waiter)
            .await
            .unwrap()
            .unwrap(),
        Err(Error::Rejected)
    );
    let after = state(operator, identity).await;
    assert_eq!(
        (after.0, after.4),
        (before.0, 0),
        "nothing changed, no session"
    );
    observer.close().await.unwrap();
}

/// Every refusal of a change, each before anything is written: malformed
/// parts, a next session of the other kind, another account's session, a
/// credential changed since the proof began. Returns that other account's
/// session.
async fn refusals(
    store: &AccountStore,
    keys: &ServerKeys,
    credential: &Credential,
    native: &Token,
    record: &[u8],
    wrapped: &[u8],
    next: &Token,
) -> Token {
    let refused: Vec<(Vec<u8>, Vec<u8>, &str)> = vec![
        (
            record[..record.len() - 1].to_vec(),
            wrapped.to_vec(),
            "a short record",
        ),
        (record.to_vec(), wrapped[..72].to_vec(), "a short wrapper"),
        (
            [record.to_vec(), vec![0]].concat(),
            wrapped.to_vec(),
            "a long record",
        ),
    ];
    for (record, wrapped, what) in refused {
        assert_eq!(
            store
                .change_password(
                    Session::Native(native),
                    credential,
                    &record,
                    &wrapped,
                    Session::Native(next),
                )
                .await
                .err(),
            Some(Error::Rejected),
            "{what}"
        );
    }
    // The next session is of the caller's kind.
    assert_eq!(
        store
            .change_password(
                Session::Native(native),
                credential,
                record,
                wrapped,
                Session::Browser(next),
            )
            .await
            .err(),
        Some(Error::Rejected)
    );
    // Another account's session cannot change this account's password.
    let other = enroll(store, keys, "password-other", "password-other").await;
    let stranger = login(store, keys, other).await;
    store.confirm_restore(&stranger, 1).await.unwrap();
    store.close_legacy("password-other", other).await.unwrap();
    assert_eq!(
        store
            .change_password(
                Session::Native(&stranger),
                credential,
                record,
                wrapped,
                Session::Native(next),
            )
            .await
            .err(),
        Some(Error::Rejected)
    );
    // A credential other than the account's (a change since the proof began).
    let stale = Credential {
        identity: credential.identity,
        revision: credential.revision + 1,
        record: credential.record.clone(),
    };
    assert_eq!(
        store
            .change_password(
                Session::Native(native),
                &stale,
                record,
                wrapped,
                Session::Native(next),
            )
            .await
            .err(),
        Some(Error::Rejected)
    );
    stranger
}
