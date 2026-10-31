//! Administration over the real listeners (`docs/administration.md`): the
//! operator appoints the first administrator; the website's session then
//! lists and reads accounts, appoints another and reads the audit; the
//! native listener refuses it all; a stale sign-in reads and changes nothing.
use super::{
    Http,
    browser::Browser,
    password::{OLD, browser_sign_in, close, context, moved},
};
use central_api::{
    Error, Token,
    admin::{AdminAnswer, AdminRequest},
    wire::{Identity, Request, Response},
};
use central_client::Transport;
use central_postgres::{Appointed, appoint_admin};
use sea_orm::{ConnectionTrait, DatabaseConnection};
use tokio::runtime::Runtime;

pub(crate) fn run(
    http: &Http,
    browser: &Browser,
    bridge: &Token,
    runtime: &Runtime,
    operator: &DatabaseConnection,
) {
    let browser = Browser {
        address: browser.address,
        client: "198.51.100.41",
    };
    let (identity, session) = moved(http, bridge, "admin-web");
    close(http, bridge, "admin-web", identity);
    drop(session);
    let (other, session) = moved(http, bridge, "admin-other");
    close(http, bridge, "admin-other", other);
    let cookie = browser_sign_in(&browser, identity, context(http, identity), OLD);
    let admin = |request: AdminRequest| Request::Admin { request };
    let accounts = || {
        admin(AdminRequest::Accounts {
            query: Some("admin-".into()),
            before: None,
        })
    };
    let summary = |cookie: &Token| {
        let Response::AccountSummary { admin, .. } = browser
            .answer(&Request::AccountSummary, Some(cookie))
            .unwrap()
        else {
            panic!("a summary");
        };
        admin
    };
    // Not an administrator yet: refused.
    assert!(!summary(&cookie));
    assert_eq!(
        browser.answer(&accounts(), Some(&cookie)).err(),
        Some(Error::Rejected)
    );
    assert_eq!(
        runtime.block_on(appoint_admin(operator, "admin-web")),
        Ok(Appointed::Appointed)
    );
    assert!(summary(&cookie));
    // The native listener serves none of it, with either token.
    for token in [&cookie, &Token::generate().unwrap()] {
        assert_eq!(
            http.exchange(&accounts(), Some(token)).err(),
            Some(Error::Rejected)
        );
    }
    drop(session);
    acting(&browser, &cookie, identity, other, runtime, operator);
}

/// The administrator's reads and a change, then a stale sign-in.
fn acting(
    browser: &Browser,
    cookie: &Token,
    identity: Identity,
    other: Identity,
    runtime: &Runtime,
    operator: &DatabaseConnection,
) {
    let admin = |request: AdminRequest| Request::Admin { request };
    let accounts = || {
        admin(AdminRequest::Accounts {
            query: Some("admin-".into()),
            before: None,
        })
    };
    let Response::Admin {
        answer: AdminAnswer::Accounts {
            accounts: found,
            more,
        },
    } = browser.answer(&accounts(), Some(cookie)).unwrap()
    else {
        panic!("accounts");
    };
    assert!(!more);
    assert_eq!(
        found
            .iter()
            .map(|a| a.username.as_str())
            .collect::<Vec<_>>(),
        ["admin-other", "admin-web"]
    );
    assert!(found[1].admin && !found[0].admin);
    let Response::Admin {
        answer: AdminAnswer::Account {
            account, recovery, ..
        },
    } = browser
        .answer(
            &admin(AdminRequest::Account {
                account: other.account,
            }),
            Some(cookie),
        )
        .unwrap()
    else {
        panic!("an account");
    };
    assert_eq!(
        (account.username.as_str(), recovery),
        ("admin-other", false)
    );
    // A change, then the audit names both ids and nothing else.
    assert!(matches!(
        browser
            .answer(
                &admin(AdminRequest::SetAdmin {
                    account: other.account,
                    admin: true,
                }),
                Some(cookie),
            )
            .unwrap(),
        Response::Done
    ));
    let Response::Admin {
        answer: AdminAnswer::Audit {
            events, accounts, ..
        },
    } = browser
        .answer(
            &admin(AdminRequest::Audit {
                account: Some(other.account),
                before: None,
            }),
            Some(cookie),
        )
        .unwrap()
    else {
        panic!("the audit");
    };
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].action, "account.admin_granted");
    assert_eq!(events[0].actor, Some(identity.account));
    // the page names its actor and subject as they are called today
    for id in [identity.account, other.account] {
        assert!(
            accounts.iter().any(|named| named.id == id),
            "a named account"
        );
    }
    stale(browser, cookie, identity, other, runtime, operator);
}

/// Thirty minutes after the sign-in a change needs the password again; reads
/// go on. A malformed search is refused.
fn stale(
    browser: &Browser,
    cookie: &Token,
    identity: Identity,
    other: Identity,
    runtime: &Runtime,
    operator: &DatabaseConnection,
) {
    let admin = |request: AdminRequest| Request::Admin { request };
    runtime
        .block_on(operator.execute_unprepared(&format!(
            "UPDATE central_identity.sessions SET created_at = created_at - interval '31 minutes'
             WHERE account_id = '{}' AND kind = 'browser'",
            sea_orm::prelude::Uuid::from_bytes(identity.account)
        )))
        .unwrap();
    assert_eq!(
        browser
            .answer(
                &admin(AdminRequest::SetAdmin {
                    account: other.account,
                    admin: false,
                }),
                Some(cookie),
            )
            .err(),
        Some(Error::Rejected)
    );
    assert!(
        browser
            .answer(
                &admin(AdminRequest::Accounts {
                    query: None,
                    before: None,
                }),
                Some(cookie),
            )
            .is_ok()
    );
    assert_eq!(
        browser
            .answer(
                &admin(AdminRequest::Accounts {
                    query: Some("#zz".into()),
                    before: None,
                }),
                Some(cookie),
            )
            .err(),
        Some(Error::Rejected)
    );
    browser.answer(&Request::Logout, Some(cookie)).unwrap();
}
