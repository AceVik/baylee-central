//! Migration 000026's e-mail addresses (`docs/email.md`): naming one under a
//! fresh sign-in, confirming it with the mailed code and what wrong codes
//! cost, replacing, withdrawing and removing it; a reset code by mail, which
//! an administrator's code outlasts; the sweep; and that deleting the
//! account deletes its addresses. Every address is a reserved one.
use super::{AccountStore, Error, Identity, ServerKeys, Token, enroll, login};
use central_api::{email::MailCode, reset_code::ResetCode};
use central_postgres::{Session, appoint_admin, sweep};
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

/// An enrolled account whose move is finished, and its native session.
async fn moved(store: &AccountStore, keys: &ServerKeys, name: &str) -> (Identity, Token) {
    let identity = enroll(store, keys, name, name).await;
    let native = login(store, keys, identity).await;
    store.confirm_restore(&native, 1).await.unwrap();
    store.close_legacy(name, identity).await.unwrap();
    (identity, native)
}

fn digest(identity: Identity) -> [u8; 32] {
    MailCode::generate().unwrap().digest(&identity.account)
}

/// Name `address` for the session's account and confirm it.
async fn confirm(store: &AccountStore, session: Session<'_>, identity: Identity, address: &str) {
    let code = digest(identity);
    store
        .email_pending(session, address, "en", &code)
        .await
        .unwrap();
    store
        .email_confirm(session, identity.account, &code)
        .await
        .unwrap();
}

async fn rows(operator: &DatabaseConnection, table: &str, identity: Identity) -> i64 {
    count(
        operator,
        &format!(
            "SELECT count(*)::bigint AS value FROM central_identity.{table}
             WHERE account_id = '{}'",
            uuid(identity)
        ),
    )
    .await
}

pub(super) async fn run(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let (owner, native) = moved(store, keys, "mail-owner").await;
    let cookie = browser(store, owner).await;
    naming(store, operator, owner, &cookie, &native).await;
    wrong_codes(store, operator, owner, &cookie).await;
    late_code(store, operator, owner, &cookie).await;
    confirming(store, operator, owner, &cookie).await;
    twins(store, operator, keys, owner, &cookie).await;
    ending(store, operator, owner, &cookie).await;
    mailed_reset(store, operator, keys, owner, &cookie).await;
    deleted_with_the_account(store, operator, keys).await;
}

/// Who may name an address, and what waits afterwards.
async fn naming(
    store: &AccountStore,
    operator: &DatabaseConnection,
    owner: Identity,
    cookie: &Token,
    native: &Token,
) {
    // No session; a native bearer is no browser session and the reverse.
    let stranger = Token::generate().unwrap();
    for session in [
        Session::Browser(&stranger),
        Session::Native(&stranger),
        Session::Browser(native),
        Session::Native(cookie),
    ] {
        assert_eq!(store.email(session).await.err(), Some(Error::Rejected));
        assert_eq!(
            store
                .email_pending(session, "owner@example.org", "en", &digest(owner))
                .await
                .err(),
            Some(Error::Rejected)
        );
        assert_eq!(
            store.email_withdraw(session).await.err(),
            Some(Error::Rejected)
        );
        assert_eq!(
            store.email_remove(session).await.err(),
            Some(Error::Rejected)
        );
    }
    // Both kinds of session read the same account's nothing.
    for session in [Session::Browser(cookie), Session::Native(native)] {
        let held = store.email(session).await.unwrap();
        assert_eq!(held.account, owner.account);
        assert!(held.fresh);
        assert!(held.address.is_none() && held.confirmed_at.is_none());
        assert!(held.pending.is_none() && held.pending_until.is_none());
    }
    // Malformed input is refused before anything is read.
    let session = Session::Browser(cookie);
    for (address, lang) in [
        ("owner", "en"),
        ("owner@localhost", "en"),
        ("owner@example.org\r\nBcc: other@example.org", "en"),
        ("owner@example.org", "EN"),
        ("owner@example.org", ""),
    ] {
        assert_eq!(
            store
                .email_pending(session, address, lang, &digest(owner))
                .await
                .err(),
            Some(Error::Rejected),
            "{lang}"
        );
    }
    // A sign-in 31 minutes old names nothing; it still reads.
    let aged = format!(
        "UPDATE central_identity.sessions SET created_at = created_at - interval '31 minutes'
         WHERE account_id = '{}' AND kind = 'browser'",
        uuid(owner)
    );
    operator.execute_unprepared(&aged).await.unwrap();
    assert!(!store.email(session).await.unwrap().fresh);
    assert_eq!(
        store
            .email_pending(session, "owner@example.org", "en", &digest(owner))
            .await
            .err(),
        Some(Error::Rejected)
    );
    assert_eq!(rows(operator, "account_email_pending", owner).await, 0);
    operator
        .execute_unprepared(&aged.replace("- interval", "+ interval"))
        .await
        .unwrap();
    // A fresh one does, for a quarter of an hour; nothing is confirmed by it.
    let held = store
        .email_pending(session, "owner@example.org", "de", &digest(owner))
        .await
        .unwrap();
    assert_eq!(held.pending.as_deref(), Some("owner@example.org"));
    assert!(held.address.is_none());
    let in_a_quarter = count(
        operator,
        "SELECT floor(extract(epoch FROM clock_timestamp() + interval '15 minutes'))::bigint AS value",
    )
    .await;
    let until = held.pending_until.unwrap();
    assert!(
        (in_a_quarter - 5..=in_a_quarter).contains(&until),
        "{until}"
    );
    // Another address replaces the one that waited.
    let held = store
        .email_pending(session, "second@example.org", "en", &digest(owner))
        .await
        .unwrap();
    assert_eq!(held.pending.as_deref(), Some("second@example.org"));
    assert_eq!(rows(operator, "account_email_pending", owner).await, 1);
    // Nothing of an address or a code is in the Debug output.
    let printed = format!("{held:?}");
    assert!(!printed.contains("second") && !printed.contains("example"));
}

/// A wrong code is counted; the fifth gives the address up. Another
/// account's digest and a code that is over count nothing.
async fn wrong_codes(
    store: &AccountStore,
    operator: &DatabaseConnection,
    owner: Identity,
    cookie: &Token,
) {
    let session = Session::Browser(cookie);
    let tries = format!(
        "SELECT tries::bigint AS value FROM central_identity.account_email_pending
         WHERE account_id = '{}'",
        uuid(owner)
    );
    let right = digest(owner);
    store
        .email_pending(session, "owner@example.org", "en", &right)
        .await
        .unwrap();
    // Made for another account: refused before the code is looked at.
    assert_eq!(
        store.email_confirm(session, [9; 16], &right).await.err(),
        Some(Error::Rejected)
    );
    assert_eq!(count(operator, &tries).await, 0);
    for counted in 1..=4 {
        assert_eq!(
            store
                .email_confirm(session, owner.account, &digest(owner))
                .await
                .err(),
            Some(Error::Rejected)
        );
        assert_eq!(count(operator, &tries).await, counted);
    }
    // A new code starts the count again, and the earlier code is over.
    let renewed = digest(owner);
    assert_eq!(
        store
            .email_renew(session, "other@example.org", "en", &renewed)
            .await
            .err(),
        Some(Error::Rejected)
    );
    store
        .email_renew(session, "owner@example.org", "en", &renewed)
        .await
        .unwrap();
    assert_eq!(count(operator, &tries).await, 0);
    assert_eq!(
        store
            .email_confirm(session, owner.account, &right)
            .await
            .err(),
        Some(Error::Rejected)
    );
    for _ in 0..3 {
        assert!(
            store
                .email_confirm(session, owner.account, &digest(owner))
                .await
                .is_err()
        );
    }
    assert_eq!(count(operator, &tries).await, 4);
    // The fifth wrong code: the address is given up, and the right code
    // finds nothing.
    assert!(
        store
            .email_confirm(session, owner.account, &digest(owner))
            .await
            .is_err()
    );
    assert_eq!(rows(operator, "account_email_pending", owner).await, 0);
    assert_eq!(
        store
            .email_confirm(session, owner.account, &renewed)
            .await
            .err(),
        Some(Error::Rejected)
    );
    assert!(store.email(session).await.unwrap().pending.is_none());
}

/// A code past its end: not shown, not confirmed, not renewed, not counted;
/// the sweep deletes the row.
async fn late_code(
    store: &AccountStore,
    operator: &DatabaseConnection,
    owner: Identity,
    cookie: &Token,
) {
    let session = Session::Browser(cookie);
    let tries = format!(
        "SELECT tries::bigint AS value FROM central_identity.account_email_pending
         WHERE account_id = '{}'",
        uuid(owner)
    );
    let late = digest(owner);
    store
        .email_pending(session, "owner@example.org", "en", &late)
        .await
        .unwrap();
    operator
        .execute_unprepared(&format!(
            "UPDATE central_identity.account_email_pending
             SET expires_at = clock_timestamp() - interval '1 second' WHERE account_id = '{}'",
            uuid(owner)
        ))
        .await
        .unwrap();
    let held = store.email(session).await.unwrap();
    assert!(held.pending.is_none() && held.pending_until.is_none());
    assert_eq!(
        store
            .email_confirm(session, owner.account, &late)
            .await
            .err(),
        Some(Error::Rejected)
    );
    assert_eq!(
        store
            .email_renew(session, "owner@example.org", "en", &digest(owner))
            .await
            .err(),
        Some(Error::Rejected)
    );
    assert_eq!(count(operator, &tries).await, 0);
    assert_eq!(sweep(operator).await.map(|swept| swept.addresses), Ok(1));
    assert_eq!(rows(operator, "account_email_pending", owner).await, 0);
    assert_eq!(sweep(operator).await.map(|swept| swept.addresses), Ok(0));
}

/// The right code makes the address the account's; a second address
/// replaces it and the first is returned to be told; two accounts may hold
/// one address.
async fn confirming(
    store: &AccountStore,
    operator: &DatabaseConnection,
    owner: Identity,
    cookie: &Token,
) {
    let session = Session::Browser(cookie);
    let code = digest(owner);
    store
        .email_pending(session, "owner@example.org", "de", &code)
        .await
        .unwrap();
    let (held, told) = store
        .email_confirm(session, owner.account, &code)
        .await
        .unwrap();
    assert!(told.is_none(), "no address was replaced");
    assert_eq!(held.address.as_deref(), Some("owner@example.org"));
    assert!(held.pending.is_none() && held.pending_until.is_none());
    let now = count(
        operator,
        "SELECT floor(extract(epoch FROM clock_timestamp()))::bigint AS value",
    )
    .await;
    let at = held.confirmed_at.unwrap();
    assert!((now - 5..=now).contains(&at), "{at}");
    assert_eq!(rows(operator, "account_email_pending", owner).await, 0);
    // The code is spent.
    assert_eq!(
        store
            .email_confirm(session, owner.account, &code)
            .await
            .err(),
        Some(Error::Rejected)
    );
    // The account's own address again: a conflict, and nothing waits. The
    // comparison is exact.
    assert_eq!(
        store
            .email_pending(session, "owner@example.org", "en", &digest(owner))
            .await
            .err(),
        Some(Error::Conflict)
    );
    assert_eq!(rows(operator, "account_email_pending", owner).await, 0);
    // A second address waits while the first stays in use, and replaces it
    // when confirmed; the first is told in the language its code was asked in.
    let second = digest(owner);
    let held = store
        .email_pending(session, "second@example.org", "fr", &second)
        .await
        .unwrap();
    assert_eq!(held.address.as_deref(), Some("owner@example.org"));
    assert_eq!(held.pending.as_deref(), Some("second@example.org"));
    let (held, told) = store
        .email_confirm(session, owner.account, &second)
        .await
        .unwrap();
    let told = told.unwrap();
    assert_eq!(
        (told.address.as_str(), told.lang.as_str()),
        ("owner@example.org", "de")
    );
    assert!(!format!("{told:?}").contains("owner"));
    assert_eq!(held.address.as_deref(), Some("second@example.org"));
    assert_eq!(rows(operator, "account_emails", owner).await, 1);
}

/// Two accounts may hold one address, and one account's code confirms
/// nothing for the other.
async fn twins(
    store: &AccountStore,
    operator: &DatabaseConnection,
    keys: &ServerKeys,
    owner: Identity,
    cookie: &Token,
) {
    let session = Session::Browser(cookie);
    let (twin, twin_native) = moved(store, keys, "mail-twin").await;
    confirm(
        store,
        Session::Native(&twin_native),
        twin,
        "second@example.org",
    )
    .await;
    assert_eq!(
        count(
            operator,
            "SELECT count(*)::bigint AS value FROM central_identity.account_emails
             WHERE address = 'second@example.org'"
        )
        .await,
        2
    );
    // One account's code confirms nothing for the other.
    let own = MailCode::generate().unwrap();
    store
        .email_pending(
            session,
            "third@example.org",
            "en",
            &own.digest(&owner.account),
        )
        .await
        .unwrap();
    store
        .email_pending(
            Session::Native(&twin_native),
            "third@example.org",
            "en",
            &digest(twin),
        )
        .await
        .unwrap();
    assert_eq!(
        store
            .email_confirm(
                Session::Native(&twin_native),
                twin.account,
                &own.digest(&twin.account)
            )
            .await
            .err(),
        Some(Error::Rejected)
    );
    assert_eq!(
        store
            .email_confirm(
                Session::Native(&twin_native),
                twin.account,
                &own.digest(&owner.account)
            )
            .await
            .err(),
        Some(Error::Rejected)
    );
    store
        .email_withdraw(Session::Native(&twin_native))
        .await
        .unwrap();
}

/// Withdrawing gives up what waits and keeps the address; removing takes
/// both and returns the address to be told. Neither needs a fresh sign-in.
async fn ending(
    store: &AccountStore,
    operator: &DatabaseConnection,
    owner: Identity,
    cookie: &Token,
) {
    let session = Session::Browser(cookie);
    // `confirming` left a third address waiting.
    assert_eq!(rows(operator, "account_email_pending", owner).await, 1);
    operator
        .execute_unprepared(&format!(
            "UPDATE central_identity.sessions SET created_at = created_at - interval '31 minutes'
             WHERE account_id = '{}' AND kind = 'browser'",
            uuid(owner)
        ))
        .await
        .unwrap();
    let held = store.email_withdraw(session).await.unwrap();
    assert!(held.pending.is_none());
    assert_eq!(held.address.as_deref(), Some("second@example.org"));
    assert_eq!(rows(operator, "account_email_pending", owner).await, 0);
    // Again: nothing waits, and that is no error.
    store.email_withdraw(session).await.unwrap();
    let (held, told) = store.email_remove(session).await.unwrap();
    assert_eq!(told.unwrap().address, "second@example.org");
    assert!(held.address.is_none() && held.confirmed_at.is_none());
    assert_eq!(rows(operator, "account_emails", owner).await, 0);
    // Again: nothing to remove, nobody to tell.
    let (_, told) = store.email_remove(session).await.unwrap();
    assert!(told.is_none());
    operator
        .execute_unprepared(&format!(
            "UPDATE central_identity.sessions SET created_at = created_at + interval '31 minutes'
             WHERE account_id = '{}' AND kind = 'browser'",
            uuid(owner)
        ))
        .await
        .unwrap();
    // Removing takes what waits too.
    confirm(store, session, owner, "owner@example.org").await;
    store
        .email_pending(session, "second@example.org", "en", &digest(owner))
        .await
        .unwrap();
    let (held, _) = store.email_remove(session).await.unwrap();
    assert!(held.address.is_none() && held.pending.is_none());
    assert_eq!(rows(operator, "account_email_pending", owner).await, 0);
}

/// Move every time a reset code was mailed for `identity` into the past.
async fn age_mails(operator: &DatabaseConnection, identity: Identity, by: &str) {
    operator
        .execute_unprepared(&format!(
            "UPDATE central_identity.account_emails SET reset_mailed = ARRAY(
                 SELECT t - interval '{by}' FROM unnest(reset_mailed) t ORDER BY t)
             WHERE account_id = '{}'",
            uuid(identity)
        ))
        .await
        .unwrap();
}

async fn finds(store: &AccountStore, name: &str, code: &ResetCode) -> bool {
    store.reset_credential(name, &code.digest()).await.is_ok()
}

/// A reset code by mail: for whom, how often, and what it never replaces.
async fn mailed_reset(
    store: &AccountStore,
    operator: &DatabaseConnection,
    keys: &ServerKeys,
    owner: Identity,
    cookie: &Token,
) {
    let session = Session::Browser(cookie);
    let code = ResetCode::generate().unwrap();
    // No address: nothing, for the account, an unknown name and no name.
    for name in ["mail-owner", "mail-nobody", "x", ""] {
        assert!(
            store
                .issue_mailed_reset(name, &code.digest())
                .await
                .unwrap()
                .is_none(),
            "{name}"
        );
    }
    confirm(store, session, owner, "owner@example.org").await;
    // The accounts an administrator's code is refused for are refused here
    // too, each with a confirmed address: an administrator's, one whose
    // move is open; and an account that is not enrolled has none.
    let (admin, admin_native) = moved(store, keys, "mail-admin").await;
    confirm(
        store,
        Session::Native(&admin_native),
        admin,
        "admin@example.org",
    )
    .await;
    appoint_admin(operator, "mail-admin").await.unwrap();
    let admin_cookie = browser(store, admin).await;
    let open = enroll(store, keys, "mail-open", "mail-open").await;
    let open_native = login(store, keys, open).await;
    confirm(
        store,
        Session::Native(&open_native),
        open,
        "open@example.org",
    )
    .await;
    store
        .reserve_legacy(
            "mail-pending",
            "mail-pending",
            None,
            None,
            &Token::generate().unwrap(),
        )
        .await
        .unwrap();
    for name in ["mail-admin", "mail-open", "mail-pending"] {
        assert!(
            store
                .issue_mailed_reset(name, &code.digest())
                .await
                .unwrap()
                .is_none(),
            "{name}"
        );
    }
    assert_eq!(
        count(
            operator,
            "SELECT count(*)::bigint AS value FROM central_identity.accounts
             WHERE username_key LIKE 'mail-%' AND reset_digest IS NOT NULL"
        )
        .await,
        0
    );
    // The account with a confirmed address: a code for an hour, and what
    // the mail needs. The name is taken as a sign-in takes it.
    let events = "SELECT count(*)::bigint AS value FROM central_identity.audit_events";
    let before = count(operator, events).await;
    let mailed = store
        .issue_mailed_reset("Mail-Owner", &code.digest())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(mailed.account, owner.account);
    assert_eq!(mailed.username, "mail-owner");
    assert_eq!(mailed.address, "owner@example.org");
    assert!(!format!("{mailed:?}").contains("owner"));
    assert_eq!(count(operator, events).await, before, "nothing is audited");
    let until = count(
        operator,
        &format!(
            "SELECT floor(extract(epoch FROM reset_expires))::bigint AS value
             FROM central_identity.accounts WHERE id = '{}'",
            uuid(owner)
        ),
    )
    .await;
    let in_an_hour = count(
        operator,
        "SELECT floor(extract(epoch FROM clock_timestamp() + interval '1 hour'))::bigint AS value",
    )
    .await;
    assert!((in_an_hour - 5..=in_an_hour).contains(&until), "{until}");
    assert!(finds(store, "mail-owner", &code).await);
    how_often(store, operator, owner, &code).await;
    administrators_code(store, operator, owner, &admin_cookie).await;
    ended_with_the_address(store, operator, owner, cookie, &admin_cookie).await;
}

/// Once in ten minutes and three times in 24 hours; a new mailed code
/// replaces the one before.
async fn how_often(
    store: &AccountStore,
    operator: &DatabaseConnection,
    owner: Identity,
    first: &ResetCode,
) {
    let mails = format!(
        "SELECT cardinality(reset_mailed)::bigint AS value FROM central_identity.account_emails
         WHERE account_id = '{}'",
        uuid(owner)
    );
    // At once, and nine minutes later: nothing, and the first code stays.
    let second = ResetCode::generate().unwrap();
    for wait in ["0 minutes", "9 minutes"] {
        age_mails(operator, owner, wait).await;
        assert!(
            store
                .issue_mailed_reset("mail-owner", &second.digest())
                .await
                .unwrap()
                .is_none()
        );
        assert!(finds(store, "mail-owner", first).await);
    }
    assert_eq!(count(operator, &mails).await, 1);
    // After ten minutes: the second replaces the first.
    age_mails(operator, owner, "2 minutes").await;
    assert!(
        store
            .issue_mailed_reset("mail-owner", &second.digest())
            .await
            .unwrap()
            .is_some()
    );
    assert!(!finds(store, "mail-owner", first).await);
    assert!(finds(store, "mail-owner", &second).await);
    // A third, and then none for the rest of the 24 hours.
    age_mails(operator, owner, "11 minutes").await;
    let third = ResetCode::generate().unwrap();
    assert!(
        store
            .issue_mailed_reset("mail-owner", &third.digest())
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(count(operator, &mails).await, 3);
    let fourth = ResetCode::generate().unwrap();
    for wait in ["11 minutes", "23 hours"] {
        age_mails(operator, owner, wait).await;
        assert!(
            store
                .issue_mailed_reset("mail-owner", &fourth.digest())
                .await
                .unwrap()
                .is_none()
        );
    }
    assert!(finds(store, "mail-owner", &third).await);
    // Half an hour on, the first of the three is more than 24 hours old
    // and the other two are not.
    age_mails(operator, owner, "30 minutes").await;
    assert!(
        store
            .issue_mailed_reset("mail-owner", &fourth.digest())
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(count(operator, &mails).await, 3);
    assert!(finds(store, "mail-owner", &fourth).await);
    // A mail that was not taken after all: the code is taken back, by its
    // digest only, and the attempt stays counted.
    store
        .withdraw_mailed_reset(owner.account, &third.digest())
        .await
        .unwrap();
    assert!(finds(store, "mail-owner", &fourth).await);
    store
        .withdraw_mailed_reset(owner.account, &fourth.digest())
        .await
        .unwrap();
    assert!(!finds(store, "mail-owner", &fourth).await);
    assert_eq!(count(operator, &mails).await, 3);
}

/// A mailed request never replaces an administrator's code that lasts; an
/// administrator's code replaces a mailed one.
async fn administrators_code(
    store: &AccountStore,
    operator: &DatabaseConnection,
    owner: Identity,
    admin_cookie: &Token,
) {
    age_mails(operator, owner, "25 hours").await;
    let issued = ResetCode::generate().unwrap();
    store
        .issue_reset(admin_cookie, owner.account, &issued.digest())
        .await
        .unwrap();
    let mailed = ResetCode::generate().unwrap();
    assert!(
        store
            .issue_mailed_reset("mail-owner", &mailed.digest())
            .await
            .unwrap()
            .is_none(),
        "a mailed request does not replace an administrator's code"
    );
    assert!(finds(store, "mail-owner", &issued).await);
    assert!(!finds(store, "mail-owner", &mailed).await);
    // Taking a mailed code back does not touch it either.
    store
        .withdraw_mailed_reset(owner.account, &mailed.digest())
        .await
        .unwrap();
    assert!(finds(store, "mail-owner", &issued).await);
    // Once the administrator's code is over, a mailed one takes its place.
    let expired = format!(
        "UPDATE central_identity.accounts SET reset_expires = clock_timestamp() - interval '1 second'
         WHERE id = '{}'",
        uuid(owner)
    );
    operator.execute_unprepared(&expired).await.unwrap();
    assert!(
        store
            .issue_mailed_reset("mail-owner", &mailed.digest())
            .await
            .unwrap()
            .is_some()
    );
    assert!(finds(store, "mail-owner", &mailed).await);
    // An administrator's code replaces a mailed one, and is then safe from
    // the next mailed request, although the address's row still names the
    // mailed code's digest.
    store
        .issue_reset(admin_cookie, owner.account, &issued.digest())
        .await
        .unwrap();
    assert!(!finds(store, "mail-owner", &mailed).await);
    age_mails(operator, owner, "25 hours").await;
    assert!(
        store
            .issue_mailed_reset("mail-owner", &ResetCode::generate().unwrap().digest())
            .await
            .unwrap()
            .is_none()
    );
    assert!(finds(store, "mail-owner", &issued).await);
    // The sweep tidies the address's row: the stale digest goes, the old
    // times go, the administrator's code stays.
    sweep(operator).await.unwrap();
    assert_eq!(
        count(
            operator,
            &format!(
                "SELECT count(*)::bigint AS value FROM central_identity.account_emails
                 WHERE account_id = '{}' AND reset_digest IS NULL
                 AND cardinality(reset_mailed) = 0",
                uuid(owner)
            )
        )
        .await,
        1
    );
    assert!(finds(store, "mail-owner", &issued).await);
}

/// A mailed code ends when the address it went to is replaced or removed;
/// an administrator's code does not.
async fn ended_with_the_address(
    store: &AccountStore,
    operator: &DatabaseConnection,
    owner: Identity,
    cookie: &Token,
    admin_cookie: &Token,
) {
    let session = Session::Browser(cookie);
    // `administrators_code` left the administrator's code outstanding.
    let issued = ResetCode::generate().unwrap();
    store
        .issue_reset(admin_cookie, owner.account, &issued.digest())
        .await
        .unwrap();
    confirm(store, session, owner, "second@example.org").await;
    assert!(finds(store, "mail-owner", &issued).await);
    let (_, told) = store.email_remove(session).await.unwrap();
    assert_eq!(told.unwrap().address, "second@example.org");
    assert!(finds(store, "mail-owner", &issued).await);
    // A mailed code, and the address replaced.
    operator
        .execute_unprepared(&format!(
            "UPDATE central_identity.accounts SET reset_digest = NULL, reset_expires = NULL
             WHERE id = '{}'",
            uuid(owner)
        ))
        .await
        .unwrap();
    confirm(store, session, owner, "owner@example.org").await;
    let mailed = ResetCode::generate().unwrap();
    assert!(
        store
            .issue_mailed_reset("mail-owner", &mailed.digest())
            .await
            .unwrap()
            .is_some()
    );
    confirm(store, session, owner, "second@example.org").await;
    assert!(!finds(store, "mail-owner", &mailed).await);
    // The times stay with the account through a replaced address: the next
    // request is inside the ten minutes.
    assert!(
        store
            .issue_mailed_reset("mail-owner", &mailed.digest())
            .await
            .unwrap()
            .is_none()
    );
    // A mailed code, and the address removed.
    age_mails(operator, owner, "11 minutes").await;
    let mailed = store
        .issue_mailed_reset("mail-owner", &mailed.digest())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(mailed.address, "second@example.org");
    store.email_remove(session).await.unwrap();
    assert_eq!(
        count(
            operator,
            &format!(
                "SELECT count(*)::bigint AS value FROM central_identity.accounts
                 WHERE id = '{}' AND reset_digest IS NULL AND reset_expires IS NULL",
                uuid(owner)
            )
        )
        .await,
        1
    );
}

/// Deleting the account deletes its address and the one that waits, as the
/// store's own deletion does it.
async fn deleted_with_the_account(
    store: &AccountStore,
    operator: &DatabaseConnection,
    keys: &ServerKeys,
) {
    let (gone, native) = moved(store, keys, "mail-gone").await;
    let session = Session::Native(&native);
    confirm(store, session, gone, "gone@example.org").await;
    store
        .email_pending(session, "next@example.org", "en", &digest(gone))
        .await
        .unwrap();
    assert!(
        store
            .issue_mailed_reset("mail-gone", &ResetCode::generate().unwrap().digest())
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(rows(operator, "account_emails", gone).await, 1);
    assert_eq!(rows(operator, "account_email_pending", gone).await, 1);
    let (credential, _) = store.deletion_start(session).await.unwrap();
    store.delete_account(session, &credential).await.unwrap();
    assert_eq!(rows(operator, "account_emails", gone).await, 0);
    assert_eq!(rows(operator, "account_email_pending", gone).await, 0);
    assert_eq!(
        count(
            operator,
            "SELECT count(*)::bigint AS value FROM central_identity.account_emails
             WHERE address IN ('gone@example.org', 'next@example.org')"
        )
        .await
            + count(
                operator,
                "SELECT count(*)::bigint AS value FROM central_identity.account_email_pending
                 WHERE address IN ('gone@example.org', 'next@example.org')"
            )
            .await,
        0
    );
    // The runtime role reads and writes the two tables and nothing more.
    let runtime = central_postgres::connect_dev(super::config(
        "central_runtime",
        "CENTRAL_TEST_RUNTIME_PASSWORD",
    ))
    .await
    .unwrap();
    for sql in [
        "TRUNCATE central_identity.account_emails",
        "TRUNCATE central_identity.account_email_pending",
        "ALTER TABLE central_identity.account_emails DROP CONSTRAINT account_emails_pkey",
    ] {
        assert!(runtime.execute_unprepared(sql).await.is_err(), "{sql}");
    }
    runtime.close().await.unwrap();
}
