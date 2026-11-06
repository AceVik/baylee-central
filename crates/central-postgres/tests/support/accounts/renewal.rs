//! Migration 000008, renewable sessions: a live session renews to 30 minutes
//! from now, never past 12 hours after its sign-in; an expired, replaced or
//! signed-out session renews nothing; a session renews only as its own kind.
//! The runtime may add a revocation and never rewrite one.
use super::{AccountStore, Admit, Error, Identity, ServerKeys, Token, config, enroll, login};
use central_postgres::{Session, connect_dev};
use sea_orm::{ConnectionTrait, DatabaseConnection};

const NATIVE: &str = "FROM central_identity.sessions s JOIN central_identity.accounts a
     ON a.id = s.account_id WHERE a.username_key = 'renewal' AND s.kind = 'native'";

async fn count(db: &impl ConnectionTrait, sql: &str) -> i64 {
    super::super::scalar(db, sql).await
}

/// The native session's expiry, in Unix seconds.
async fn expiry(operator: &DatabaseConnection) -> i64 {
    let sql = format!("SELECT floor(extract(epoch FROM s.expires_at))::bigint AS value {NATIVE}");
    count(operator, &sql).await
}

/// Moves the native session `ago` into the past (fixed
/// synthetic intervals, never user input): made then, expiring `left` from now.
async fn made(operator: &DatabaseConnection, ago: &str, left: &str) {
    operator
        .execute_unprepared(&format!(
            "UPDATE central_identity.sessions SET created_at = pg_catalog.clock_timestamp() - interval '{ago}',
                 expires_at = pg_catalog.clock_timestamp() + interval '{left}'
             WHERE digest = (SELECT s.digest {NATIVE});"
        ))
        .await
        .unwrap();
}

pub(super) async fn run(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let identity = enroll(store, keys, "renewal", "renewal").await;
    let session = login(store, keys, identity).await;
    store.confirm_restore(&session, 1).await.unwrap();
    // The bridge admits a moved account once its move is closed.
    store.close_legacy("renewal", identity).await.unwrap();
    // A token's cap is the session's 12 hours, which no renewal moves.
    let cap = store
        .gateway_grant(
            &session,
            &["https://renewal.example"],
            false,
            Admit,
            [43; 16],
        )
        .await
        .unwrap()
        .cap;

    // Twenty minutes in, ten left: renewed to 30 minutes from now.
    made(operator, "20 minutes", "10 minutes").await;
    let before = expiry(operator).await;
    let renewed = store.renew_session(&session).await.unwrap();
    assert!(renewed > before, "later than before");
    assert_eq!(expiry(operator).await, renewed);
    assert_eq!(
        count(
            operator,
            &format!(
                "SELECT count(*)::bigint AS value {NATIVE} AND s.expires_at - pg_catalog.clock_timestamp()
                 BETWEEN interval '29 minutes' AND interval '30 minutes'"
            )
        )
        .await,
        1
    );
    assert_eq!(
        store
            .gateway_grant(
                &session,
                &["https://renewal.example"],
                false,
                Admit,
                [44; 16]
            )
            .await
            .unwrap()
            .cap,
        cap,
        "a renewal does not move the cap"
    );

    // Near the cap: renewed to 12 hours after the sign-in, and no further.
    made(operator, "11 hours 50 minutes", "5 minutes").await;
    let capped = store.renew_session(&session).await.unwrap();
    assert_eq!(
        count(
            operator,
            &format!(
                "SELECT count(*)::bigint AS value {NATIVE} AND s.expires_at = s.created_at + interval '12 hours'"
            )
        )
        .await,
        1
    );
    assert_eq!(
        store.renew_session(&session).await.unwrap(),
        capped,
        "the cap does not move"
    );

    // Expired: nothing renews, and nothing moves.
    made(operator, "12 hours", "-1 second").await;
    let expired = expiry(operator).await;
    assert_eq!(
        store.renew_session(&session).await.err(),
        Some(Error::Rejected)
    );
    assert_eq!(expiry(operator).await, expired);

    kinds_and_endings(store, keys, identity).await;
    privileges(operator).await;
}

/// A browser session renews as itself; a native bearer is none, a cookie is
/// no bearer; a replaced or signed-out session renews nothing.
async fn kinds_and_endings(store: &AccountStore, keys: &ServerKeys, identity: Identity) {
    let credential = store.credential(identity).await.unwrap().unwrap();
    let browser = Token::generate().unwrap();
    store
        .issue_session(&credential, Session::Browser(&browser))
        .await
        .unwrap();
    store
        .renew_session(Session::Browser(&browser))
        .await
        .unwrap();
    assert_eq!(
        store.renew_session(&browser).await.err(),
        Some(Error::Rejected),
        "a cookie is no native bearer"
    );
    let first = login(store, keys, identity).await;
    store.renew_session(&first).await.unwrap();
    assert_eq!(
        store.renew_session(Session::Browser(&first)).await.err(),
        Some(Error::Rejected),
        "a native bearer is no browser session"
    );
    let second = login(store, keys, identity).await;
    assert_eq!(
        store.renew_session(&first).await.err(),
        Some(Error::Rejected),
        "a new sign-in ends the old session's renewals"
    );
    store
        .renew_session(Session::Browser(&browser))
        .await
        .unwrap();
    store.logout(&second).await.unwrap();
    assert_eq!(
        store.renew_session(&second).await.err(),
        Some(Error::Rejected)
    );
}

async fn privileges(operator: &DatabaseConnection) {
    let runtime = connect_dev(config("central_runtime", "CENTRAL_TEST_RUNTIME_PASSWORD"))
        .await
        .unwrap();
    // Checked when the statement is planned, so no row has to change. A revocation is
    // added and aged out, and never rewritten.
    for column in ["stream", "seq", "sid", "revoked_at"] {
        assert!(
            runtime
                .execute_unprepared(&format!(
                    "UPDATE central_identity.gateway_revocations SET {column} = {column} WHERE false"
                ))
                .await
                .is_err(),
            "{column}"
        );
    }
    runtime.close().await.unwrap();
    // The store's startup check refuses a wider grant, and a missing one.
    let log = "central_identity.gateway_revocations";
    let sids = "central_identity.gateway_sids";
    for (change, undo) in [
        (
            format!("GRANT UPDATE (sid) ON {log} TO central_runtime"),
            format!("REVOKE UPDATE (sid) ON {log} FROM central_runtime"),
        ),
        (
            format!("GRANT UPDATE ON {log} TO central_runtime"),
            format!("REVOKE UPDATE ON {log} FROM central_runtime"),
        ),
        (
            format!("REVOKE DELETE ON {sids} FROM central_runtime"),
            format!("GRANT DELETE ON {sids} TO central_runtime"),
        ),
    ] {
        // Fixed synthetic statements, never user input.
        operator.execute_unprepared(&change).await.unwrap();
        let refused =
            AccountStore::connect_dev(config("central_runtime", "CENTRAL_TEST_RUNTIME_PASSWORD"))
                .await
                .is_err();
        operator.execute_unprepared(&undo).await.unwrap();
        assert!(refused, "{change}");
    }
    AccountStore::connect_dev(config("central_runtime", "CENTRAL_TEST_RUNTIME_PASSWORD"))
        .await
        .unwrap()
        .close()
        .await
        .unwrap();
}
