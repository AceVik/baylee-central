//! Migration 000016, pairings and the key check (`docs/web-accounts.md`
//! §"Pairing"): a check is set once and only within two minutes of a password
//! proof; a pairing is started by a browser session of an account with a
//! check, claimed once within two minutes, approved once by the session that
//! started it, and redeemed once: a wrong verifier ends it, and the session it
//! gives counts its 12 hours from the browser session's password proof. An
//! ended pairing, an account at another epoch and a proof too old give nothing.
use super::{AccountStore, Error, ServerKeys, Token, enroll, login};
use central_postgres::{OPEN_PER_ACCOUNT, Session};
use sea_orm::{ConnectionTrait, DatabaseConnection};

async fn scalar(db: &impl ConnectionTrait, sql: &str) -> i64 {
    super::super::scalar(db, sql).await
}

/// A browser session for the account named `name`.
async fn browser(store: &AccountStore, name_identity: central_api::wire::Identity) -> Token {
    let credential = store.credential(name_identity).await.unwrap().unwrap();
    let browser = Token::generate().unwrap();
    store
        .issue_session(&credential, Session::Browser(&browser))
        .await
        .unwrap();
    browser
}

/// Moves the browser session of `name` `ago` into the past (fixed synthetic
/// intervals, never user input), keeping it live.
async fn aged(operator: &DatabaseConnection, name: &str, ago: &str) {
    operator
        .execute_unprepared(&format!(
            "UPDATE central_identity.sessions s SET created_at = pg_catalog.clock_timestamp() - interval '{ago}',
                 expires_at = pg_catalog.clock_timestamp() + interval '10 minutes'
             FROM central_identity.accounts a
             WHERE a.id = s.account_id AND a.username_key = '{name}' AND s.kind = 'browser'"
        ))
        .await
        .unwrap();
}

/// Moves every pairing of `name` `ago` into the past.
async fn pairings_aged(operator: &DatabaseConnection, name: &str, ago: &str) {
    operator
        .execute_unprepared(&format!(
            "UPDATE central_identity.pairings p SET created_at = p.created_at - interval '{ago}',
                 expires_at = p.expires_at - interval '{ago}', proven_at = p.proven_at - interval '{ago}'
             FROM central_identity.accounts a WHERE a.id = p.account_id AND a.username_key = '{name}'"
        ))
        .await
        .unwrap();
}

fn handle() -> [u8; 32] {
    Token::generate().unwrap().digest()
}

async fn now(operator: &DatabaseConnection) -> i64 {
    scalar(
        operator,
        "SELECT floor(extract(epoch FROM pg_catalog.clock_timestamp()))::bigint AS value",
    )
    .await
}

pub(super) async fn run(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let identity = enroll(store, keys, "pairing", "pairing").await;
    let native = login(store, keys, identity).await;
    store.confirm_restore(&native, 1).await.unwrap();
    let page = browser(store, identity).await;
    let late = enroll(store, keys, "pairing-late", "pairing-late").await;

    key_check(store, operator, &native, &page, late).await;
    claimed_and_approved(store, operator, &native, &page, late).await;
    redeemed(store, operator, identity, &native, &page).await;
    refused(store, operator, &page).await;
}

/// The key check: set once, fresh from the password proof.
async fn key_check(
    store: &AccountStore,
    operator: &DatabaseConnection,
    native: &Token,
    page: &Token,
    late: central_api::wire::Identity,
) {
    // None yet; no pairing without one.
    assert_eq!(store.key_check(Session::Browser(page)).await.unwrap(), None);
    assert_eq!(
        store.start_pairing(page, &handle()).await.err(),
        Some(Error::Rejected)
    );
    // Set once, fresh from the password proof; a second is a conflict.
    store
        .set_key_check(Session::Browser(page), &[7; 32])
        .await
        .unwrap();
    assert_eq!(
        store.key_check(native).await.unwrap(),
        Some([7; 32]),
        "the account's, on either kind of session"
    );
    assert_eq!(
        store.set_key_check(native, &[8; 32]).await.err(),
        Some(Error::Conflict),
        "never overwritten"
    );
    // Another account: a check from a password proof older than two minutes
    // is refused, so a stolen session cannot plant one.
    let late_page = browser(store, late).await;
    aged(operator, "pairing-late", "3 minutes").await;
    assert_eq!(
        store
            .set_key_check(Session::Browser(&late_page), &[9; 32])
            .await
            .err(),
        Some(Error::Rejected)
    );
    assert_eq!(
        store
            .set_key_check(&Token::generate().unwrap(), &[9; 32])
            .await
            .err(),
        Some(Error::Rejected),
        "no session, no check"
    );
}

/// Start, claim, approval, and a wrong verifier that ends it all.
async fn claimed_and_approved(
    store: &AccountStore,
    operator: &DatabaseConnection,
    native: &Token,
    page: &Token,
    late: central_api::wire::Identity,
) {
    // Start: a native bearer is no browser session; a browser session is.
    assert_eq!(
        store.start_pairing(native, &handle()).await.err(),
        Some(Error::Rejected)
    );
    let first = handle();
    let ends = store.start_pairing(page, &first).await.unwrap();
    let now = now(operator).await;
    assert!((now + 295..=now + 300).contains(&ends), "five minutes");
    // At most three open for one account.
    for _ in 1..OPEN_PER_ACCOUNT {
        store.start_pairing(page, &handle()).await.unwrap();
    }
    assert_eq!(
        store.start_pairing(page, &handle()).await.err(),
        Some(Error::Busy)
    );

    // No claim yet; another browser session reads nothing of it.
    assert!(store.pairing_claim(page, &first).await.unwrap().is_none());
    let other_page = browser(store, late).await;
    assert_eq!(
        store.pairing_claim(&other_page, &first).await.err(),
        Some(Error::Rejected)
    );
    assert_eq!(
        store.poll_pairing(&first).await.err(),
        Some(Error::Rejected),
        "nothing to poll before a claim"
    );

    // The claim: once.
    assert_eq!(
        store
            .claim_pairing(&first, &[1; 32], &[2; 32], &[3; 32])
            .await
            .unwrap(),
        ends
    );
    assert_eq!(
        store
            .claim_pairing(&first, &[4; 32], &[5; 32], &[6; 32])
            .await
            .err(),
        Some(Error::Conflict),
        "a second claim, as a hijacking app would make it"
    );
    let claim = store.pairing_claim(page, &first).await.unwrap().unwrap();
    assert_eq!(
        (claim.public_key, claim.challenge, claim.mac),
        ([1; 32], [2; 32], [3; 32])
    );
    assert!(
        !store.poll_pairing(&first).await.unwrap(),
        "not approved yet"
    );
    store.cancel_pairing(&other_page, &first).await.unwrap();
    assert!(
        store.pairing_claim(page, &first).await.unwrap().is_some(),
        "a stranger's cookie cancels nothing"
    );

    // Approval: only the starting session, once.
    assert_eq!(
        store.pairing_for_approval(&other_page, &first).await.err(),
        Some(Error::Rejected)
    );
    let approval = store.pairing_for_approval(page, &first).await.unwrap();
    assert_eq!(approval.key_check, [7; 32]);
    assert_eq!(
        store
            .approve_pairing(&other_page, &first, &[0xaa; 80])
            .await
            .err(),
        Some(Error::Rejected),
        "a stranger's cookie approves nothing"
    );
    store
        .approve_pairing(page, &first, &[0xaa; 80])
        .await
        .unwrap();
    assert_eq!(
        store.approve_pairing(page, &first, &[0xbb; 80]).await.err(),
        Some(Error::Rejected),
        "approved once"
    );
    assert!(store.poll_pairing(&first).await.unwrap());

    // A wrong verifier's challenge ends the pairing; the right one comes too late.
    let wrong = Token::generate().unwrap();
    assert_eq!(
        store.redeem_pairing(&first, &[9; 32], &wrong).await.err(),
        Some(Error::Rejected)
    );
    assert_eq!(
        store.redeem_pairing(&first, &[2; 32], &wrong).await.err(),
        Some(Error::Rejected),
        "ended by the wrong verifier"
    );
}

/// A pairing that goes through: the session comes with the sealed key,
/// replaces the account's native session, and counts from the browser
/// session's password proof.
async fn redeemed(
    store: &AccountStore,
    operator: &DatabaseConnection,
    identity: central_api::wire::Identity,
    native: &Token,
    page: &Token,
) {
    let now = now(operator).await;
    store.cancel_pairing(page, &handle()).await.unwrap();
    operator
        .execute_unprepared(
            "DELETE FROM central_identity.pairings p USING central_identity.accounts a
             WHERE a.id = p.account_id AND a.username_key = 'pairing'",
        )
        .await
        .unwrap();
    let second = handle();
    store.start_pairing(page, &second).await.unwrap();
    store
        .claim_pairing(&second, &[1; 32], &[2; 32], &[3; 32])
        .await
        .unwrap();
    store
        .approve_pairing(page, &second, &[0xcc; 80])
        .await
        .unwrap();
    let paired = Token::generate().unwrap();
    let redeemed = store
        .redeem_pairing(&second, &[2; 32], &paired)
        .await
        .unwrap();
    assert_eq!(redeemed.identity.account, identity.account);
    assert_eq!(redeemed.username, "pairing");
    assert_eq!(redeemed.sealed, vec![0xcc; 80]);
    assert_eq!(
        store.session_status(native).await.err(),
        Some(Error::Rejected),
        "the account's earlier native session ended"
    );
    store.document(&paired).await.unwrap();
    assert_eq!(
        scalar(
            operator,
            "SELECT count(*)::bigint AS value FROM central_identity.sessions n
             JOIN central_identity.sessions b ON b.account_id = n.account_id
             WHERE n.kind = 'native' AND b.kind = 'browser' AND n.proven_at = b.created_at
             AND n.account_id = (SELECT id FROM central_identity.accounts WHERE username_key = 'pairing')",
        )
        .await,
        1,
        "the paired session rests on the browser's password proof"
    );
    assert_eq!(
        store
            .redeem_pairing(&second, &[2; 32], &Token::generate().unwrap())
            .await
            .err(),
        Some(Error::Rejected),
        "redeemed once"
    );
    // Renewal never passes the proof's 12 hours.
    let renewed = store.renew_session(&paired).await.unwrap();
    assert!(renewed <= now + 12 * 3600 + 60);
}

/// What gives nothing: another epoch, a proof too old, an ended pairing.
async fn refused(store: &AccountStore, operator: &DatabaseConnection, page: &Token) {
    // The account rotated between approval and redeem: nothing.
    let third = handle();
    store.start_pairing(page, &third).await.unwrap();
    store
        .claim_pairing(&third, &[1; 32], &[2; 32], &[3; 32])
        .await
        .unwrap();
    store
        .approve_pairing(page, &third, &[0xdd; 80])
        .await
        .unwrap();
    operator
        .execute_unprepared(
            "UPDATE central_identity.pairings SET epoch = epoch + 1 WHERE account_id =
             (SELECT id FROM central_identity.accounts WHERE username_key = 'pairing')",
        )
        .await
        .unwrap();
    assert_eq!(
        store
            .redeem_pairing(&third, &[2; 32], &Token::generate().unwrap())
            .await
            .err(),
        Some(Error::Rejected)
    );

    // A password proof too old to leave a minute of its 12 hours gives no session.
    let fourth = handle();
    store.start_pairing(page, &fourth).await.unwrap();
    store
        .claim_pairing(&fourth, &[1; 32], &[2; 32], &[3; 32])
        .await
        .unwrap();
    store
        .approve_pairing(page, &fourth, &[0xee; 80])
        .await
        .unwrap();
    operator
        .execute_unprepared(
            "UPDATE central_identity.pairings SET proven_at = pg_catalog.clock_timestamp() - interval '12 hours'
             WHERE account_id = (SELECT id FROM central_identity.accounts WHERE username_key = 'pairing')",
        )
        .await
        .unwrap();
    assert_eq!(
        store
            .redeem_pairing(&fourth, &[2; 32], &Token::generate().unwrap())
            .await
            .err(),
        Some(Error::Rejected)
    );

    // An ended pairing answers nothing; a late claim is refused.
    let fifth = handle();
    store.start_pairing(page, &fifth).await.unwrap();
    pairings_aged(operator, "pairing", "3 minutes").await;
    assert_eq!(
        store
            .claim_pairing(&fifth, &[1; 32], &[2; 32], &[3; 32])
            .await
            .err(),
        Some(Error::Rejected),
        "past the two minutes of the link's secret"
    );
    pairings_aged(operator, "pairing", "3 minutes").await;
    assert_eq!(
        store.pairing_claim(page, &fifth).await.err(),
        Some(Error::Rejected)
    );
    // The next start sweeps it.
    store.start_pairing(page, &handle()).await.unwrap();
    assert_eq!(
        scalar(
            operator,
            "SELECT count(*)::bigint AS value FROM central_identity.pairings
             WHERE expires_at <= pg_catalog.clock_timestamp()",
        )
        .await,
        0
    );
    // Signing out of the page ends its pairings.
    store.logout(Session::Browser(page)).await.unwrap();
    assert_eq!(
        scalar(
            operator,
            "SELECT count(*)::bigint AS value FROM central_identity.pairings p
             JOIN central_identity.accounts a ON a.id = p.account_id WHERE a.username_key = 'pairing'",
        )
        .await,
        0
    );
}
