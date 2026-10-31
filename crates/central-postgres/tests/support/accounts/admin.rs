//! Migration 000012's administration: the exact grants, the operator's
//! appointment and sweep, the role's rules (a stale sign-in, oneself, the
//! last administrator, two administrators removing each other at once), the
//! reads and the sign-in time.
use super::{AccountStore, Error, Identity, ServerKeys, Token, config, enroll, login};
use central_api::admin::{AccountQuery, AccountState};
use central_postgres::{AUDIT_YEARS, Appointed, Session, appoint_admin, sweep};
use sea_orm::{ConnectionTrait, DatabaseConnection, TransactionTrait};
use std::time::Duration;

async fn count(db: &impl ConnectionTrait, sql: &str) -> i64 {
    crate::scalar(db, sql).await
}

fn uuid(identity: Identity) -> String {
    sea_orm::prelude::Uuid::from_bytes(identity.account).to_string()
}

/// A browser session for `identity`, as the website's sign-in issues it.
async fn browser(store: &AccountStore, identity: Identity) -> Token {
    let credential = store.credential(identity).await.unwrap().unwrap();
    let token = Token::generate().unwrap();
    store
        .issue_session(&credential, Session::Browser(&token))
        .await
        .unwrap();
    token
}

/// Move the browser session's password sign-in back by `minutes`.
async fn age(operator: &DatabaseConnection, identity: Identity, minutes: i64) {
    operator
        .execute_unprepared(&format!(
            "UPDATE central_identity.sessions SET created_at = created_at - interval '{minutes} minutes'
             WHERE account_id = '{}' AND kind = 'browser'",
            uuid(identity)
        ))
        .await
        .unwrap();
}

pub(super) async fn run(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    grants(operator).await;
    let first = enroll(store, keys, "admin-first", "admin-first").await;
    let second = enroll(store, keys, "admin-second", "admin-second").await;
    let pending = Token::generate().unwrap();
    store
        .reserve_legacy("admin-pending", "admin-pending", None, None, &pending)
        .await
        .unwrap();
    appointment(operator, first).await;
    let cookie = browser(store, first).await;
    let summary = store.account_summary(&cookie).await.unwrap();
    assert!(summary.admin);
    let other = browser(store, second).await;
    assert!(!store.account_summary(&other).await.unwrap().admin);
    rules(store, operator, first, second, &cookie, &other).await;
    reads(store, operator, first, second, &cookie).await;
    together(store, operator, first, second).await;
    sign_in_time(store, operator, keys, first).await;
    swept(operator).await;
}

/// The runtime takes exactly its new privileges, and refuses to start with
/// any other on the two tables.
async fn grants(operator: &DatabaseConnection) {
    for (change, undo) in [
        (
            "GRANT UPDATE ON central_identity.audit_events TO central_runtime",
            "REVOKE UPDATE ON central_identity.audit_events FROM central_runtime",
        ),
        (
            "GRANT DELETE ON central_identity.audit_events TO central_runtime",
            "REVOKE DELETE ON central_identity.audit_events FROM central_runtime",
        ),
        (
            "GRANT TRUNCATE ON central_identity.audit_events TO central_runtime",
            "REVOKE TRUNCATE ON central_identity.audit_events FROM central_runtime",
        ),
        (
            "GRANT UPDATE (granted_by) ON central_identity.account_roles TO central_runtime",
            "REVOKE UPDATE (granted_by) ON central_identity.account_roles FROM central_runtime",
        ),
        (
            "REVOKE INSERT ON central_identity.audit_events FROM central_runtime",
            "GRANT INSERT ON central_identity.audit_events TO central_runtime",
        ),
        (
            "REVOKE DELETE ON central_identity.account_roles FROM central_runtime",
            "GRANT DELETE ON central_identity.account_roles TO central_runtime",
        ),
    ] {
        // Fixed synthetic statements, never user input.
        operator.execute_unprepared(change).await.unwrap();
        let refused =
            AccountStore::connect_dev(config("central_runtime", "CENTRAL_TEST_RUNTIME_PASSWORD"))
                .await
                .is_err();
        operator.execute_unprepared(undo).await.unwrap();
        assert!(refused, "{change}");
    }
    // And the runtime itself can neither rewrite nor delete an event.
    let runtime =
        central_postgres::connect_dev(config("central_runtime", "CENTRAL_TEST_RUNTIME_PASSWORD"))
            .await
            .unwrap();
    for statement in [
        "UPDATE central_identity.audit_events SET action = 'account.admin_removed'",
        "DELETE FROM central_identity.audit_events",
        "TRUNCATE central_identity.audit_events",
    ] {
        assert!(
            runtime.execute_unprepared(statement).await.is_err(),
            "{statement}"
        );
    }
    runtime.close().await.unwrap();
}

/// The operator appoints an enrolled account once; again changes nothing.
async fn appointment(operator: &DatabaseConnection, first: Identity) {
    for refused in ["nobody-here", "admin-pending", "x"] {
        assert_eq!(
            appoint_admin(operator, refused).await,
            Err(Error::Rejected),
            "{refused}"
        );
    }
    assert_eq!(
        appoint_admin(operator, "Admin-First").await,
        Ok(Appointed::Appointed)
    );
    assert_eq!(
        appoint_admin(operator, "admin-first").await,
        Ok(Appointed::Already)
    );
    assert_eq!(
        count(
            operator,
            &format!(
                "SELECT count(*)::bigint AS value FROM central_identity.audit_events
                 WHERE actor_kind = 'operator' AND actor_account IS NULL
                 AND action = 'account.admin_granted' AND account_id = '{}'",
                uuid(first)
            )
        )
        .await,
        1,
        "one event, the operator's"
    );
    // Its role has no granting administrator.
    assert_eq!(
        count(
            operator,
            "SELECT count(*)::bigint AS value FROM central_identity.account_roles WHERE granted_by IS NULL"
        )
        .await,
        1
    );
}

/// Who may change roles, and when.
async fn rules(
    store: &AccountStore,
    operator: &DatabaseConnection,
    first: Identity,
    second: Identity,
    cookie: &Token,
    other: &Token,
) {
    let events = || async {
        count(
            operator,
            "SELECT count(*)::bigint AS value FROM central_identity.audit_events",
        )
        .await
    };
    let before = events().await;
    // A non-administrator, a native bearer and no session change nothing.
    assert_eq!(
        store.set_admin(other, second.account, true).await,
        Err(Error::Rejected)
    );
    let native = Token::generate().unwrap();
    let credential = store.credential(first).await.unwrap().unwrap();
    store.issue_session(&credential, &native).await.unwrap();
    assert_eq!(
        store.set_admin(&native, second.account, true).await,
        Err(Error::Rejected)
    );
    assert_eq!(
        store.admin_accounts(&native, None, None).await.err(),
        Some(Error::Rejected)
    );
    assert_eq!(
        store
            .set_admin(&Token::generate().unwrap(), second.account, true)
            .await,
        Err(Error::Rejected)
    );
    // A pending or unknown account is no administrator to be.
    let pending: [u8; 16] = store
        .admin_accounts(
            cookie,
            Some(&AccountQuery::Text("admin-pending".into())),
            None,
        )
        .await
        .unwrap()
        .0[0]
        .id;
    for target in [pending, [9; 16]] {
        assert_eq!(
            store.set_admin(cookie, target, true).await,
            Err(Error::Rejected)
        );
    }
    // Nobody removes their own role, so the last administrator stays.
    assert_eq!(
        store.set_admin(cookie, first.account, false).await,
        Err(Error::Rejected)
    );
    assert_eq!(events().await, before, "refusals write nothing");
    // Appointing, again (nothing new), and removing.
    store.set_admin(cookie, second.account, true).await.unwrap();
    store.set_admin(cookie, second.account, true).await.unwrap();
    assert_eq!(events().await, before + 1);
    assert!(store.account_summary(other).await.unwrap().admin);
    store
        .set_admin(cookie, second.account, false)
        .await
        .unwrap();
    store
        .set_admin(cookie, second.account, false)
        .await
        .unwrap();
    assert_eq!(events().await, before + 2);
    assert_eq!(
        count(
            operator,
            &format!(
                "SELECT count(*)::bigint AS value FROM central_identity.audit_events
                 WHERE actor_kind = 'admin' AND actor_account = '{}' AND account_id = '{}'
                 AND action IN ('account.admin_granted', 'account.admin_removed')",
                uuid(first),
                uuid(second)
            )
        )
        .await,
        2
    );
    // The removed administrator's next request fails.
    assert_eq!(
        store.admin_accounts(other, None, None).await.err(),
        Some(Error::Rejected)
    );
    // A sign-in 30 minutes old or older reads, and changes nothing.
    age(operator, first, 31).await;
    assert!(store.admin_accounts(cookie, None, None).await.is_ok());
    assert_eq!(
        store.set_admin(cookie, second.account, true).await,
        Err(Error::Rejected)
    );
    // A renewal does not freshen it.
    store.renew_session(Session::Browser(cookie)).await.unwrap();
    assert_eq!(
        store.set_admin(cookie, second.account, true).await,
        Err(Error::Rejected)
    );
    age(operator, first, -31).await;
    assert_eq!(events().await, before + 2);
}

/// The lists: search by text and tag, pages, one account, the audit.
async fn reads(
    store: &AccountStore,
    operator: &DatabaseConnection,
    first: Identity,
    second: Identity,
    cookie: &Token,
) {
    let found = |text: &str| AccountQuery::Text(text.into());
    let (accounts, _) = store
        .admin_accounts(cookie, Some(&found("admin-")), None)
        .await
        .unwrap();
    let names: Vec<&str> = accounts.iter().map(|a| a.username.as_str()).collect();
    // Newest first.
    assert_eq!(names, ["admin-pending", "admin-second", "admin-first"]);
    assert_eq!(accounts[0].state, AccountState::Pending);
    assert_eq!(accounts[1].state, AccountState::Enrolled);
    assert!(accounts[2].admin && !accounts[1].admin);
    assert!(accounts[2].last_sign_in_at.is_some());
    let (page, more) = store
        .admin_accounts(cookie, Some(&found("admin-")), Some(accounts[0].id))
        .await
        .unwrap();
    assert_eq!(page.len(), 2);
    assert!(!more);
    // A display name matches without case, and a tag exactly.
    operator
        .execute_unprepared(&format!(
            "UPDATE central_identity.accounts SET display_name = 'Second_One',
             display_name_at = clock_timestamp(), tag = 77 WHERE id = '{}'",
            uuid(second)
        ))
        .await
        .unwrap();
    for query in [found("second_o"), AccountQuery::Tag(77)] {
        let (accounts, _) = store
            .admin_accounts(cookie, Some(&query), None)
            .await
            .unwrap();
        assert_eq!(accounts.len(), 1, "{query:?}");
        assert_eq!(accounts[0].display_name.as_deref(), Some("Second_One"));
        assert_eq!(accounts[0].tag, Some(77));
    }
    // Text never acts as a pattern.
    assert!(
        store
            .admin_accounts(cookie, Some(&found("%")), None)
            .await
            .unwrap()
            .0
            .is_empty()
    );
    let (account, recovery, gateways, audit) =
        store.admin_account(cookie, second.account).await.unwrap();
    assert_eq!(account.username, "admin-second");
    assert!(!recovery && gateways.is_empty());
    assert_eq!(
        audit.iter().map(|e| e.action.as_str()).collect::<Vec<_>>(),
        ["account.admin_removed", "account.admin_granted"]
    );
    assert!(audit.iter().all(|e| e.actor == Some(first.account)));
    assert_eq!(
        store.admin_account(cookie, [9; 16]).await.err(),
        Some(Error::Rejected)
    );
    let page = store
        .admin_audit(cookie, Some(first.account), None)
        .await
        .unwrap();
    assert_eq!(page.events.len(), 1);
    assert_eq!(page.events[0].actor, None, "the operator's appointment");
    assert!(!page.more);
    // the operator acted, so the page names only its subject, as it is called today
    assert_eq!(
        page.accounts.iter().map(|a| a.id).collect::<Vec<_>>(),
        [first.account]
    );
    assert!(page.gateways.is_empty());
    let all = store.admin_audit(cookie, None, None).await.unwrap();
    let after = store
        .admin_audit(cookie, None, Some(all.events[0].id))
        .await
        .unwrap();
    assert_eq!(after.events.len(), all.events.len() - 1);
    // every account an event names that still exists is named once, and
    // nothing else (an event keeps the id of an account deleted since)
    let mut named = Vec::new();
    for id in all
        .events
        .iter()
        .flat_map(|e| [e.actor, e.account])
        .flatten()
    {
        let sql = format!(
            "SELECT count(*) AS value FROM central_identity.accounts WHERE id = '{}'",
            sea_orm::prelude::Uuid::from_bytes(id)
        );
        if count(operator, &sql).await == 1 && !named.contains(&id) {
            named.push(id);
        }
    }
    named.sort_unstable();
    assert_eq!(all.accounts.iter().map(|a| a.id).collect::<Vec<_>>(), named);
    overview(store, operator, cookie).await;
}

/// The first view's counts agree with the tables, and only an administrator reads them.
async fn overview(store: &AccountStore, operator: &DatabaseConnection, cookie: &Token) {
    let seen = store.admin_overview(cookie).await.unwrap();
    let counted =
        |sql: &'static str| async move { u64::try_from(count(operator, sql).await).unwrap() };
    assert_eq!(
        seen.accounts,
        counted(
            "SELECT count(*) AS value FROM central_identity.accounts WHERE credential IS NOT NULL"
        )
        .await
    );
    assert_eq!(
        seen.admins,
        counted(
            "SELECT count(*) AS value FROM central_identity.account_roles WHERE role = 'admin'"
        )
        .await
    );
    assert_eq!(
        seen.pending + seen.approved + seen.disabled,
        counted("SELECT count(*) AS value FROM central_identity.gateways WHERE NOT official").await
    );
    assert!(seen.accounts_week <= seen.accounts && seen.moved <= seen.accounts);
    assert!(seen.admins >= 1, "the reader is one");
    assert_eq!(
        store
            .admin_overview(&Token::generate().unwrap())
            .await
            .err(),
        Some(Error::Rejected)
    );
}

/// Two administrators removing each other at once leave one.
async fn together(
    store: &AccountStore,
    operator: &DatabaseConnection,
    first: Identity,
    second: Identity,
) {
    let a = browser(store, first).await;
    let b = browser(store, second).await;
    store.set_admin(&a, second.account, true).await.unwrap();
    let observer =
        central_postgres::connect_dev(config("central_runtime", "CENTRAL_TEST_RUNTIME_PASSWORD"))
            .await
            .unwrap();
    let held = operator.begin().await.unwrap();
    held.execute_unprepared(
        "SELECT a.id FROM central_identity.accounts a JOIN central_identity.account_roles r
         ON r.account_id = a.id WHERE r.role = 'admin' ORDER BY a.id FOR UPDATE OF a",
    )
    .await
    .unwrap();
    let (one, two) = (store.clone(), store.clone());
    let removing_b = tokio::spawn(async move { one.set_admin(&a, second.account, false).await });
    let removing_a = tokio::spawn(async move { two.set_admin(&b, first.account, false).await });
    tokio::time::timeout(Duration::from_secs(5), async {
        while count(
            &observer,
            "SELECT count(*)::bigint AS value FROM pg_catalog.pg_stat_activity
             WHERE datname = current_database() AND usename = 'central_runtime'
             AND wait_event_type = 'Lock' AND query LIKE '%ORDER BY a.id FOR UPDATE OF a%'",
        )
        .await
            < 2
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("both removals wait on the held account rows");
    held.commit().await.unwrap();
    let results = [
        tokio::time::timeout(Duration::from_secs(5), removing_b)
            .await
            .unwrap()
            .unwrap(),
        tokio::time::timeout(Duration::from_secs(5), removing_a)
            .await
            .unwrap()
            .unwrap(),
    ];
    assert_eq!(
        results.iter().filter(|r| r.is_ok()).count(),
        1,
        "{results:?}"
    );
    assert!(results.contains(&Err(Error::Rejected)));
    assert_eq!(
        count(
            operator,
            "SELECT count(*)::bigint AS value FROM central_identity.account_roles WHERE role = 'admin'"
        )
        .await,
        1
    );
    observer.close().await.unwrap();
    // Leave `first` the administrator, whichever won.
    if count(
        operator,
        &format!(
            "SELECT count(*)::bigint AS value FROM central_identity.account_roles WHERE account_id = '{}'",
            uuid(first)
        ),
    )
    .await
        == 0
    {
        appoint_admin(operator, "admin-first").await.unwrap();
    }
}

/// A password sign-in of either kind sets the time; a renewal does not.
async fn sign_in_time(
    store: &AccountStore,
    operator: &DatabaseConnection,
    keys: &ServerKeys,
    first: Identity,
) {
    let at = || async {
        count(
            operator,
            &format!(
                "SELECT floor(extract(epoch FROM last_sign_in_at) * 1000000)::bigint AS value
                 FROM central_identity.accounts WHERE id = '{}'",
                uuid(first)
            ),
        )
        .await
    };
    let earlier = at().await;
    let native = login(store, keys, first).await;
    let signed = at().await;
    assert!(signed > earlier);
    store.renew_session(&native).await.unwrap();
    assert_eq!(at().await, signed, "renewal keeps it");
    browser(store, first).await;
    assert!(at().await > signed);
}

/// The sweep deletes events older than three years, and only those.
async fn swept(operator: &DatabaseConnection) {
    assert_eq!(AUDIT_YEARS, 3, "the privacy notice's three years");
    let all = "SELECT count(*)::bigint AS value FROM central_identity.audit_events";
    let kept = count(operator, all).await;
    operator
        .execute_unprepared(
            "INSERT INTO central_identity.audit_events (at, actor_kind, action)
             VALUES (clock_timestamp() - interval '3 years 1 day', 'operator', 'account.admin_granted'),
                    (clock_timestamp() - interval '3 years' + interval '1 day', 'operator', 'account.admin_granted')",
        )
        .await
        .unwrap();
    assert_eq!(sweep(operator).await.map(|s| s.events), Ok(1));
    assert_eq!(count(operator, all).await, kept + 1);
    assert_eq!(sweep(operator).await.map(|s| s.events), Ok(0));
    // The runtime's own login has no right to sweep.
    let runtime =
        central_postgres::connect_dev(config("central_runtime", "CENTRAL_TEST_RUNTIME_PASSWORD"))
            .await
            .unwrap();
    assert!(sweep(&runtime).await.is_err());
    runtime.close().await.unwrap();
}
