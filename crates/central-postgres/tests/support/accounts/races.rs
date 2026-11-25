//! What may happen once is asked for twice at the same time: a pairing's claim, approval
//! and redemption, a password change, a reset code, an address's code, a deletion, the
//! last bytes of a quota. Whatever order the database takes them in, one of them
//! succeeds, and a credential, a session or an object that ended stays ended.
use super::{
    AccountStore, Enrollment, Error, Identity, MasterKey, ObjectContext, ServerKeys, Token,
    context, enroll, login,
};
use central_api::{email::MailCode, reset_code::ResetCode, wire::ResetVault};
use central_postgres::{Credential, Session, appoint_admin};
use sea_orm::{DatabaseConnection, prelude::Uuid};

/// The quota of one account's objects, in bytes.
const OBJECT_QUOTA: i64 = 512 * 1024 * 1024;

async fn count(operator: &DatabaseConnection, sql: &str) -> i64 {
    super::super::scalar(operator, sql).await
}

async fn sessions(operator: &DatabaseConnection, identity: Identity) -> i64 {
    let id = Uuid::from_bytes(identity.account);
    count(
        operator,
        &format!(
            "SELECT count(*)::bigint AS value FROM central_identity.sessions
             WHERE account_id = '{id}' AND expires_at > pg_catalog.clock_timestamp()"
        ),
    )
    .await
}

/// A moved account, restored and closed, with a native session.
async fn moved(store: &AccountStore, keys: &ServerKeys, name: &str) -> (Identity, Token) {
    let identity = enroll(store, keys, name, name).await;
    let native = login(store, keys, identity).await;
    store.confirm_restore(&native, 1).await.unwrap();
    store.close_legacy(name, identity).await.unwrap();
    (identity, native)
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

/// A new password's record and its wrapper of a new master key.
fn password(keys: &ServerKeys, identity: Identity) -> (Vec<u8>, Vec<u8>) {
    let context = context(identity);
    let (enrollment, request) =
        Enrollment::start("synthetic raced password".into(), context).unwrap();
    let response = keys.enrollment_response(context, &request).unwrap();
    let enrolled = enrollment.finish(&response).unwrap();
    let wrapped = enrolled
        .wrapping_key
        .wrap(&MasterKey::generate().unwrap())
        .unwrap();
    (enrolled.upload, wrapped.as_bytes().to_vec())
}

/// The one that succeeded, and what the others were told.
fn one<T, const N: usize>(results: [Result<T, Error>; N]) -> (usize, T, Vec<Error>) {
    let mut won = None;
    let mut refused = Vec::new();
    for (index, result) in results.into_iter().enumerate() {
        match result {
            Ok(value) => {
                assert!(won.is_none(), "two of them succeeded");
                won = Some((index, value));
            }
            Err(error) => refused.push(error),
        }
    }
    let (index, value) = won.expect("one of them succeeds");
    (index, value, refused)
}

pub(super) async fn run(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    enrollment(store, operator, keys).await;
    gateway(store, operator, keys).await;
    pairing(store, operator, keys).await;
    cancelled_pairing(store, operator, keys).await;
    password_change(store, operator, keys).await;
    reset(store, operator, keys).await;
    address(store, operator, keys).await;
    quota(store, operator, keys).await;
    ended_session(store, operator, keys).await;
    deletion(store, operator, keys).await;
}

/// One name, wanted by two moves; one ticket, spent by two enrollments.
async fn enrollment(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let tickets = [Token::generate().unwrap(), Token::generate().unwrap()];
    let (a, b) = tokio::join!(
        store.reserve_legacy("races-name-a", "races-name", None, None, &tickets[0]),
        store.reserve_legacy("races-name-b", "Races-Name", None, None, &tickets[1]),
    );
    let (winner, identity, _) = one([a, b]);
    // The loser's ticket belongs to no reservation.
    assert!(store.enrollment(&tickets[1 - winner]).await.is_err());
    let ticket = &tickets[winner];
    assert!(store.discover("races-name").await.unwrap().is_none());

    let context = context(identity);
    let made: [_; 2] = std::array::from_fn(|n| {
        let (enrollment, request) =
            Enrollment::start(format!("synthetic enrolled {n}"), context).unwrap();
        let response = keys.enrollment_response(context, &request).unwrap();
        let enrolled = enrollment.finish(&response).unwrap();
        let master = MasterKey::generate().unwrap();
        let document = master
            .seal_object(
                ObjectContext::new(context, identity.account, 1).unwrap(),
                b"raced",
            )
            .unwrap();
        let wrapped = enrolled.wrapping_key.wrap(&master).unwrap();
        (enrolled.upload, wrapped.as_bytes().to_vec(), document)
    });
    let (a, b) = tokio::join!(
        store.enroll(ticket, &made[0].0, &made[0].1, &made[0].2, None),
        store.enroll(ticket, &made[1].0, &made[1].1, &made[1].2, None),
    );
    let (winner, (), _) = one([a, b]);
    // The record, its wrapper and the document are one enrollment's, not a mixture.
    let credential = store.credential(identity).await.unwrap().unwrap();
    assert_eq!(credential.record, made[winner].0);
    let session = Token::generate().unwrap();
    let issued = store.issue_session(&credential, &session).await.unwrap();
    assert_eq!(issued.wrapped_master, made[winner].1);
    assert_eq!(
        store.document(&session).await.unwrap(),
        (1, made[winner].2.clone())
    );
    assert!(store.enrollment(ticket).await.is_err());
    assert_eq!(sessions(operator, identity).await, 1);
}

/// One origin, registered by two accounts.
async fn gateway(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let (first, _) = moved(store, keys, "races-gateway-a").await;
    let (second, _) = moved(store, keys, "races-gateway-b").await;
    let pages = [browser(store, first).await, browser(store, second).await];
    let origin = "https://races.invalid";
    let (a, b) = tokio::join!(
        store.add_gateway(&pages[0], origin, "Synthetic first", false),
        store.add_gateway(&pages[1], origin, "Synthetic second", false),
    );
    let (winner, id, refused) = one([a, b]);
    assert_eq!(refused, [Error::Conflict]);
    let owner = Uuid::from_bytes([first, second][winner].account);
    let id = Uuid::from_bytes(id);
    assert_eq!(
        count(
            operator,
            &format!(
                "SELECT count(*)::bigint AS value FROM central_identity.gateways
                 WHERE origin = '{origin}' AND id = '{id}' AND owner_id = '{owner}'"
            )
        )
        .await,
        1
    );
}

fn handle() -> [u8; 32] {
    Token::generate().unwrap().digest()
}

async fn pairing(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let (identity, native) = moved(store, keys, "races-pairing").await;
    let page = browser(store, identity).await;
    // The check is set once, whichever session of the account is first.
    let checks = [[7; 32], [8; 32]];
    let (a, b) = tokio::join!(
        store.set_key_check(Session::Browser(&page), &checks[0]),
        store.set_key_check(Session::Native(&native), &checks[1]),
    );
    let (check, (), refused) = one([a, b]);
    assert_eq!(refused, [Error::Conflict]);
    assert_eq!(
        store.key_check(Session::Native(&native)).await.unwrap(),
        Some(checks[check])
    );
    let link = handle();
    store.start_pairing(&page, &link).await.unwrap();

    // Two devices claim: one is bound, whole, and the other is told of it.
    let claims = [([1; 32], [2; 32], [3; 32]), ([4; 32], [5; 32], [6; 32])];
    let (a, b) = tokio::join!(
        store.claim_pairing(&link, &claims[0].0, &claims[0].1, &claims[0].2),
        store.claim_pairing(&link, &claims[1].0, &claims[1].1, &claims[1].2),
    );
    let (index, _, refused) = one([a, b]);
    assert_eq!(refused, [Error::Conflict]);
    let claim = store.pairing_claim(&page, &link).await.unwrap().unwrap();
    assert_eq!(
        (claim.public_key, claim.challenge, claim.mac),
        claims[index]
    );

    // Two approvals, sealed differently: one seal stands.
    let seals = [[0xaa; 80], [0xbb; 80]];
    let (a, b) = tokio::join!(
        store.approve_pairing(&page, &link, &seals[0]),
        store.approve_pairing(&page, &link, &seals[1]),
    );
    let (seal, (), refused) = one([a, b]);
    assert_eq!(refused, [Error::Rejected]);

    // Three redemptions: one session, with the seal that stands.
    let tokens: [Token; 3] = std::array::from_fn(|_| Token::generate().unwrap());
    let challenge = claim.challenge;
    let (a, b, c) = tokio::join!(
        store.redeem_pairing(&link, &challenge, &tokens[0]),
        store.redeem_pairing(&link, &challenge, &tokens[1]),
        store.redeem_pairing(&link, &challenge, &tokens[2]),
    );
    let (winner, redeemed, refused) = one([a, b, c]);
    assert_eq!(refused, [Error::Rejected; 2]);
    assert_eq!(redeemed.sealed, seals[seal]);
    for (index, token) in tokens.iter().enumerate() {
        assert_eq!(store.document(token).await.is_ok(), index == winner);
    }
    assert!(
        store.document(&native).await.is_err(),
        "the earlier session"
    );
    // The page's own session and the paired one.
    assert_eq!(sessions(operator, identity).await, 2);
}

/// The page cancels while the device redeems: the session or the cancellation, never half.
async fn cancelled_pairing(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let (identity, mut native) = moved(store, keys, "races-cancel").await;
    let page = browser(store, identity).await;
    store
        .set_key_check(Session::Browser(&page), &[7; 32])
        .await
        .unwrap();
    for _ in 0..3 {
        let link = handle();
        store.start_pairing(&page, &link).await.unwrap();
        store
            .claim_pairing(&link, &[1; 32], &[2; 32], &[3; 32])
            .await
            .unwrap();
        store
            .approve_pairing(&page, &link, &[0xcc; 80])
            .await
            .unwrap();
        let token = Token::generate().unwrap();
        let (cancelled, redeemed) = tokio::join!(
            store.cancel_pairing(&page, &link),
            store.redeem_pairing(&link, &[2; 32], &token),
        );
        cancelled.unwrap();
        assert_eq!(store.document(&token).await.is_ok(), redeemed.is_ok());
        assert_eq!(store.document(&native).await.is_ok(), redeemed.is_err());
        if redeemed.is_ok() {
            native = token;
        }
        // Whichever came first, nothing of the pairing is left to redeem.
        assert_eq!(
            store
                .redeem_pairing(&link, &[2; 32], &Token::generate().unwrap())
                .await
                .err(),
            Some(Error::Rejected)
        );
        assert_eq!(sessions(operator, identity).await, 2);
        // A pairing that was started counts against the account's until it expires.
        let id = Uuid::from_bytes(identity.account);
        sea_orm::ConnectionTrait::execute_unprepared(
            operator,
            &format!("DELETE FROM central_identity.pairings WHERE account_id = '{id}'"),
        )
        .await
        .unwrap();
    }
}

async fn password_change(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let (identity, native) = moved(store, keys, "races-password").await;
    let credential = store
        .password_credential(Session::Native(&native))
        .await
        .unwrap();
    let (one_record, one_wrapped) = password(keys, identity);
    let (other_record, other_wrapped) = password(keys, identity);
    let next = [Token::generate().unwrap(), Token::generate().unwrap()];
    // Beside two changes, a sign-in that proved the old password just before.
    let old = Token::generate().unwrap();
    let (a, b, _) = tokio::join!(
        store.change_password(
            Session::Native(&native),
            &credential,
            &one_record,
            &one_wrapped,
            Session::Native(&next[0]),
        ),
        store.change_password(
            Session::Native(&native),
            &credential,
            &other_record,
            &other_wrapped,
            Session::Native(&next[1]),
        ),
        store.issue_session(&credential, Session::Browser(&old)),
    );
    let (winner, (issued, _), _) = one([a, b]);
    let records = [&one_record, &other_record];
    let wrappers = [&one_wrapped, &other_wrapped];
    assert_eq!(&issued.wrapped_master, wrappers[winner]);
    let now = store.credential(identity).await.unwrap().unwrap();
    assert_eq!(now.revision, credential.revision + 1, "changed once");
    assert_eq!(&now.record, records[winner]);
    // The winner's session alone lives: not the loser's, the changer's or the old password's.
    for (index, token) in next.iter().enumerate() {
        assert_eq!(store.document(token).await.is_ok(), index == winner);
    }
    assert!(store.document(&native).await.is_err());
    assert_eq!(
        store.session_status(Session::Browser(&old)).await.err(),
        Some(Error::Rejected)
    );
    assert_eq!(sessions(operator, identity).await, 1);
    // What was proved against the old password issues nothing afterwards.
    assert!(
        store
            .issue_session(&credential, &Token::generate().unwrap())
            .await
            .is_err()
    );
}

async fn reset(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let (admin, _) = moved(store, keys, "races-admin").await;
    appoint_admin(operator, "races-admin").await.unwrap();
    let cookie = browser(store, admin).await;
    let (identity, native) = moved(store, keys, "races-reset").await;
    let code = ResetCode::generate().unwrap();
    let digest = code.digest();
    store
        .issue_reset(&cookie, identity.account, &digest)
        .await
        .unwrap();
    let (credential, _) = store
        .reset_credential("races-reset", &digest)
        .await
        .unwrap();
    let finishes = [password(keys, identity), password(keys, identity)];
    let (a, b) = tokio::join!(
        store.finish_reset(
            &credential,
            &digest,
            &finishes[0].0,
            &finishes[0].1,
            &ResetVault::Keep
        ),
        store.finish_reset(
            &credential,
            &digest,
            &finishes[1].0,
            &finishes[1].1,
            &ResetVault::Keep
        ),
    );
    let (winner, _, _) = one([a, b]);
    let now = store.credential(identity).await.unwrap().unwrap();
    assert_eq!(now.revision, credential.revision + 1, "set once");
    assert_eq!(now.record, finishes[winner].0);
    assert!(store.document(&native).await.is_err());
    assert_eq!(sessions(operator, identity).await, 0);
    assert!(
        store
            .reset_credential("races-reset", &digest)
            .await
            .is_err()
    );
    // The code is spent for a proof made while it was not.
    let late = password(keys, identity);
    let stale = Credential {
        identity,
        revision: now.revision,
        record: now.record,
    };
    assert!(
        store
            .finish_reset(&stale, &digest, &late.0, &late.1, &ResetVault::Keep)
            .await
            .is_err()
    );
}

async fn address(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let (identity, _) = moved(store, keys, "races-address").await;
    let cookie = browser(store, identity).await;
    let session = Session::Browser(&cookie);
    let code = MailCode::generate().unwrap().digest(&identity.account);
    store
        .email_pending(session, "races@example.org", "en", &code)
        .await
        .unwrap();
    let (a, b) = tokio::join!(
        store.email_confirm(session, identity.account, &code),
        store.email_confirm(session, identity.account, &code),
    );
    let (_, (held, told), _) = one([a, b]);
    assert!(told.is_none());
    assert_eq!(held.address.as_deref(), Some("races@example.org"));
    let id = Uuid::from_bytes(identity.account);
    for (table, rows) in [("account_email_pending", 0), ("account_emails", 1)] {
        assert_eq!(
            count(
                operator,
                &format!(
                    "SELECT count(*)::bigint AS value FROM central_identity.{table}
                     WHERE account_id = '{id}'"
                )
            )
            .await,
            rows,
            "{table}"
        );
    }
}

/// Two objects want the quota's last bytes; another account deletes neither of them.
async fn quota(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let (identity, alice) = moved(store, keys, "races-quota").await;
    let (_, bob) = moved(store, keys, "races-quota-other").await;
    let first = store.create_object(&alice).await.unwrap();
    let second = store.create_object(&alice).await.unwrap();
    let id = Uuid::from_bytes(identity.account);
    let set = |bytes: i64| {
        format!("UPDATE central_identity.accounts SET object_bytes = {bytes} WHERE id = '{id}'")
    };
    let held =
        format!("SELECT object_bytes AS value FROM central_identity.accounts WHERE id = '{id}'");
    sea_orm::ConnectionTrait::execute_unprepared(operator, &set(OBJECT_QUOTA - 64))
        .await
        .unwrap();
    let (a, b) = tokio::join!(
        store.write_object(&alice, first.id, 0, &[1; 64]),
        store.write_object(&alice, second.id, 0, &[1; 64]),
    );
    let (winner, revision, refused) = one([a, b]);
    assert_eq!((revision, refused), (1, vec![Error::Busy]));
    assert_eq!(count(operator, &held).await, OBJECT_QUOTA);
    let (kept, empty) = if winner == 0 {
        (first.id, second.id)
    } else {
        (second.id, first.id)
    };
    assert_eq!(store.read_object(&alice, empty).await.unwrap().1, None);
    sea_orm::ConnectionTrait::execute_unprepared(operator, &set(64))
        .await
        .unwrap();

    for (object, revision) in [(kept, 1), (empty, 0)] {
        assert!(store.delete_object(&bob, object, revision).await.is_err());
        assert!(store.read_object(&bob, object).await.is_err());
        assert!(!store.read_object(&alice, object).await.unwrap().0.deleted);
    }
    // A revision no row can hold is refused before it is compared.
    for revision in [u64::MAX, i64::MAX.unsigned_abs() + 1] {
        assert!(
            store
                .write_object(&alice, kept, revision, &[1; 64])
                .await
                .is_err()
        );
        assert!(store.delete_object(&alice, kept, revision).await.is_err());
    }
    // A write and a deletion of the same revision: one of them.
    let (written, deleted) = tokio::join!(
        store.write_object(&alice, kept, 1, &[2; 100]),
        store.delete_object(&alice, kept, 1),
    );
    let (index, revision, _) = one([written, deleted]);
    assert_eq!(revision, 2);
    let (state, content) = store.read_object(&alice, kept).await.unwrap();
    assert_eq!((state.revision, state.deleted), (2, index == 1));
    assert_eq!(content.is_some(), index == 0);
    assert_eq!(
        count(operator, &held).await,
        if index == 0 { 100 } else { 0 },
        "the quota counts what is stored"
    );
}

/// A session ends while it is renewed: it has ended.
async fn ended_session(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let (identity, native) = moved(store, keys, "races-ended").await;
    let page = browser(store, identity).await;
    let (_, out, read) = tokio::join!(
        store.renew_session(&native),
        store.logout(&native),
        store.session_status(Session::Browser(&page)),
    );
    out.unwrap();
    read.unwrap();
    assert!(store.document(&native).await.is_err());
    assert!(store.renew_session(&native).await.is_err());
    // The other kind's session is untouched by it.
    assert_eq!(sessions(operator, identity).await, 1);
    // A token is refused as the other kind's, whatever it would do.
    for wrong in [Session::Native(&page), Session::Browser(&native)] {
        assert!(store.renew_session(wrong).await.is_err());
        assert!(store.session_status(wrong).await.is_err());
        assert!(store.deletion_start(wrong).await.is_err());
    }
    store.session_status(Session::Browser(&page)).await.unwrap();
}

async fn deletion(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let (identity, native) = moved(store, keys, "races-deletion").await;
    let object = store.create_object(&native).await.unwrap();
    let signing = store.credential(identity).await.unwrap().unwrap();
    let (credential, _) = store
        .deletion_start(Session::Native(&native))
        .await
        .unwrap();
    let page = Token::generate().unwrap();
    let (a, b, _, _) = tokio::join!(
        store.delete_account(Session::Native(&native), &credential),
        store.delete_account(Session::Native(&native), &credential),
        store.issue_session(&signing, Session::Browser(&page)),
        store.write_object(&native, object.id, 0, &[1; 64]),
    );
    let (_, (), refused) = one([a, b]);
    assert_eq!(refused, [Error::Rejected]);
    assert!(store.document(&native).await.is_err());
    assert!(store.session_status(Session::Browser(&page)).await.is_err());
    assert!(store.discover("races-deletion").await.unwrap().is_none());
    assert!(store.credential(identity).await.unwrap().is_none());
    let id = Uuid::from_bytes(identity.account);
    for table in ["sessions", "private_objects"] {
        assert_eq!(
            count(
                operator,
                &format!(
                    "SELECT count(*)::bigint AS value FROM central_identity.{table}
                     WHERE account_id = '{id}'"
                )
            )
            .await,
            0,
            "{table}"
        );
    }
    // Neither the old proof nor the old session does anything more.
    assert!(
        store
            .issue_session(&signing, Session::Browser(&page))
            .await
            .is_err()
    );
    assert!(
        store
            .delete_account(Session::Native(&native), &credential)
            .await
            .is_err()
    );
}
