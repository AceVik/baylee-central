//! Migration 000010 and the account page: `AccountSummary` reads the browser
//! session's account; `SetRecovery` stores a recovery wrapper for a moved
//! account once its move has finished and within 10 minutes of the sign-in,
//! takes the same bytes again at any time and refuses others; a registered
//! account keeps the wrapper of its registration; `invalidate_legacy` clears
//! a wrapper with the epoch it was bound to.
use super::{AccountStore, Error, Identity, ServerKeys, Token, enroll, login};
use central_api::wire::AccountSource;
use central_postgres::Session;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};

async fn sql(operator: &DatabaseConnection, sql: &str) {
    // Fixed synthetic statements, never user input.
    operator.execute_unprepared(sql).await.unwrap();
}

/// The stored wrapper of `name`, if any.
async fn stored(operator: &DatabaseConnection, name: &str) -> Option<Vec<u8>> {
    operator
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT recovery_wrapped_master FROM central_identity.accounts WHERE username_key = $1",
            [name.into()],
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "recovery_wrapped_master")
        .unwrap()
}

/// Moves `name`'s browser session's sign-in `ago` into the past (a fixed
/// synthetic interval, never user input); its expiry stays.
async fn signed_in(operator: &DatabaseConnection, name: &str, ago: &str) {
    sql(
        operator,
        &format!(
            "UPDATE central_identity.sessions SET created_at = pg_catalog.clock_timestamp() - interval '{ago}'
             WHERE kind = 'browser' AND account_id =
                 (SELECT id FROM central_identity.accounts WHERE username_key = '{name}')"
        ),
    )
    .await;
}

/// A 73-byte wrapper in the stored format: version 1, then `fill`. The server
/// checks only the format; the crypto is `central-web-crypto`'s.
fn wrapper(fill: u8) -> Vec<u8> {
    let mut bytes = vec![fill; 73];
    bytes[0] = 1;
    bytes
}

/// A browser session for `identity`, as a sign-in on the website issues it.
async fn browser(store: &AccountStore, identity: Identity) -> Token {
    let credential = store.credential(identity).await.unwrap().unwrap();
    let token = Token::generate().unwrap();
    store
        .issue_session(&credential, Session::Browser(&token))
        .await
        .unwrap();
    token
}

pub(super) async fn run(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let identity = enroll(store, keys, "recovery-moved", "recovery-moved").await;
    let native = login(store, keys, identity).await;
    let cookie = browser(store, identity).await;
    let session = Session::Browser(&cookie);
    summary(store, identity, &cookie, &native).await;

    // An unfinished move takes no wrapper.
    assert_eq!(
        store.set_recovery(&cookie, &wrapper(2)).await.err(),
        Some(Error::Rejected)
    );
    assert_eq!(stored(operator, "recovery-moved").await, None);
    store.confirm_restore(&native, 1).await.unwrap();
    assert_eq!(
        store.set_recovery(&cookie, &wrapper(2)).await.err(),
        Some(Error::Rejected),
        "restored, but its move is not closed yet"
    );
    store
        .close_legacy("recovery-moved", identity)
        .await
        .unwrap();

    // Not the format, not a browser session, no session: nothing stored.
    let mut version_two = wrapper(2);
    version_two[0] = 2;
    for bytes in [vec![1; 72], vec![1; 74], version_two, Vec::new()] {
        assert_eq!(
            store.set_recovery(&cookie, &bytes).await.err(),
            Some(Error::Rejected)
        );
    }
    assert_eq!(
        store.set_recovery(&native, &wrapper(2)).await.err(),
        Some(Error::Rejected),
        "a native bearer is no browser session"
    );
    assert_eq!(
        store
            .set_recovery(&Token::generate().unwrap(), &wrapper(2))
            .await
            .err(),
        Some(Error::Rejected)
    );
    assert_eq!(stored(operator, "recovery-moved").await, None);

    // Stored once; the same bytes again succeed, also after 10 minutes; other
    // bytes are a conflict and change nothing.
    store.set_recovery(&cookie, &wrapper(2)).await.unwrap();
    assert_eq!(stored(operator, "recovery-moved").await, Some(wrapper(2)));
    assert!(store.account_summary(&cookie).await.unwrap().recovery);
    store.set_recovery(&cookie, &wrapper(2)).await.unwrap();
    assert_eq!(
        store.set_recovery(&cookie, &wrapper(3)).await.err(),
        Some(Error::Conflict)
    );
    signed_in(operator, "recovery-moved", "11 minutes").await;
    store.set_recovery(&cookie, &wrapper(2)).await.unwrap();
    assert_eq!(
        store.set_recovery(&cookie, &wrapper(3)).await.err(),
        Some(Error::Conflict)
    );
    assert_eq!(stored(operator, "recovery-moved").await, Some(wrapper(2)));

    // Without a wrapper, a sign-in 10 minutes old or older stores nothing;
    // under 10 minutes it does.
    sql(
        operator,
        "UPDATE central_identity.accounts SET recovery_wrapped_master = NULL WHERE username_key = 'recovery-moved'",
    )
    .await;
    assert_eq!(
        store.set_recovery(&cookie, &wrapper(4)).await.err(),
        Some(Error::Rejected)
    );
    signed_in(operator, "recovery-moved", "10 minutes").await;
    assert_eq!(
        store.set_recovery(&cookie, &wrapper(4)).await.err(),
        Some(Error::Rejected)
    );
    signed_in(operator, "recovery-moved", "9 minutes 50 seconds").await;
    store.set_recovery(&cookie, &wrapper(4)).await.unwrap();
    assert_eq!(stored(operator, "recovery-moved").await, Some(wrapper(4)));
    // A session that ended stores nothing and reads nothing.
    store.logout(session).await.unwrap();
    assert_eq!(
        store.set_recovery(&cookie, &wrapper(4)).await.err(),
        Some(Error::Rejected)
    );
    assert_eq!(
        store.account_summary(&cookie).await.err(),
        Some(Error::Rejected)
    );
    store.logout(&native).await.unwrap();

    registered(store, operator, keys).await;
    invalidated(store, operator, keys).await;
    concurrent(store, operator, keys).await;
}

/// `AccountSummary`: the browser session's account only, read without
/// renewing anything.
async fn summary(store: &AccountStore, identity: Identity, cookie: &Token, native: &Token) {
    let summary = store.account_summary(cookie).await.unwrap();
    assert_eq!(summary.identity.account, identity.account);
    assert_eq!(summary.identity.epoch, identity.epoch);
    assert_eq!(summary.username, "recovery-moved");
    assert_eq!(summary.source, AccountSource::Legacy);
    assert!(!summary.recovery);
    assert_eq!(
        summary.expires_at - summary.signed_in_at,
        1800,
        "issued 30 minutes after the sign-in"
    );
    let now = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    )
    .unwrap();
    assert!((summary.signed_in_at - now).abs() < 60);
    // A renewal moves the end, never the sign-in.
    store.renew_session(Session::Browser(cookie)).await.unwrap();
    let renewed = store.account_summary(cookie).await.unwrap();
    assert_eq!(renewed.signed_in_at, summary.signed_in_at);
    assert!(renewed.expires_at >= summary.expires_at);
    assert_eq!(
        store.account_summary(native).await.err(),
        Some(Error::Rejected),
        "a native bearer is no browser session"
    );
}

/// A registered account has the wrapper of its registration: the same bytes
/// are `Done`, others a conflict.
async fn registered(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let key = super::register::issue(operator, 1, "recovery-registered").await;
    let ticket = Token::generate().unwrap();
    let identity = store
        .register("recovery-registered", &key.digest(), &ticket)
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
    let cookie = browser(store, identity).await;
    let session = Session::Browser(&cookie);
    let summary = store.account_summary(&cookie).await.unwrap();
    assert_eq!(summary.source, AccountSource::Registered);
    assert!(summary.recovery);
    assert_eq!(summary.username, "recovery-registered");
    store
        .set_recovery(&cookie, &enrolled.recovery)
        .await
        .unwrap();
    assert_eq!(
        store.set_recovery(&cookie, &wrapper(5)).await.err(),
        Some(Error::Conflict)
    );
    assert_eq!(
        stored(operator, "recovery-registered").await,
        Some(enrolled.recovery)
    );
    store.logout(session).await.unwrap();
}

/// `invalidate_legacy` clears a wrapper with the epoch it was bound to;
/// without that, the schema would refuse the credential's removal.
async fn invalidated(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let identity = enroll(store, keys, "recovery-reset", "recovery-reset").await;
    // Only an operator can put a wrapper on an unfinished move; the schema
    // allows it on any enrolled legacy account.
    sql(
        operator,
        "UPDATE central_identity.accounts SET recovery_wrapped_master = decode(repeat('01', 73), 'hex')
         WHERE legacy_id = 'recovery-reset'",
    )
    .await;
    store.invalidate_legacy("recovery-reset").await.unwrap();
    assert_eq!(stored(operator, "recovery-reset").await, None);
    let reserved = store
        .reserve_legacy(
            "recovery-reset",
            "recovery-reset",
            &Token::generate().unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reserved.account, identity.account);
    assert_eq!(reserved.epoch, identity.epoch + 1, "a new vault epoch");
}

/// Two codes sent at once: one is stored, the other is a conflict.
async fn concurrent(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let identity = enroll(store, keys, "recovery-race", "recovery-race").await;
    let native = login(store, keys, identity).await;
    store.confirm_restore(&native, 1).await.unwrap();
    store.close_legacy("recovery-race", identity).await.unwrap();
    let cookie = browser(store, identity).await;
    let session = Session::Browser(&cookie);
    let (six, seven) = (wrapper(6), wrapper(7));
    let (first, second) = tokio::join!(
        store.set_recovery(&cookie, &six),
        store.set_recovery(&cookie, &seven)
    );
    let kept = match (first, second) {
        (Ok(()), Err(Error::Conflict)) => wrapper(6),
        (Err(Error::Conflict), Ok(())) => wrapper(7),
        other => panic!("one stored, one conflict: {other:?}"),
    };
    assert_eq!(stored(operator, "recovery-race").await, Some(kept));
    store.logout(session).await.unwrap();
    store.logout(&native).await.unwrap();
}
