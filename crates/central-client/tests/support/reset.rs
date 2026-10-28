//! Resetting a login over the real listeners (`docs/account-reset.md`): an
//! administrator makes a code on the website; the person keeps their data
//! with the recovery code, starts a new vault, or keeps it from a device
//! still signed in. Wrong codes spend nothing of the account's; a code is
//! spent once.
use super::{
    Http,
    browser::Browser,
    password::{DOCUMENT, NEW, OLD, browser_sign_in, close, context, moved, register},
};
use central_api::{
    Error, Token,
    admin::{AdminAnswer, AdminRequest},
    reset_code::ResetCode,
    wire::{Identity, Request, ResetVault, Response},
};
use central_client::{ResetData, Transport};
use central_crypto::{opaque::Enrollment, vault::RecoveryCode};
use sea_orm::DatabaseConnection;
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
        client: "198.51.100.42",
    };
    let (admin, session) = moved(http, bridge, "reset-web-admin");
    close(http, bridge, "reset-web-admin", admin);
    drop(session);
    runtime
        .block_on(central_postgres::appoint_admin(operator, "reset-web-admin"))
        .unwrap();
    let cookie = browser_sign_in(&browser, admin, context(http, admin), OLD);
    let issue = |account: Identity| {
        let Response::Admin {
            answer: AdminAnswer::ResetCode { code, expires_at },
        } = browser
            .answer(
                &Request::Admin {
                    request: AdminRequest::ResetLogin {
                        account: account.account,
                    },
                },
                Some(&cookie),
            )
            .unwrap()
        else {
            panic!("a reset code");
        };
        assert!(expires_at > 0);
        code
    };
    recovery(http, &browser, runtime, operator, &issue);
    renewed(http, bridge, &issue);
    device(http, bridge, &issue);
    browser.answer(&Request::Logout, Some(&cookie)).unwrap();
}

/// A registered account keeps its data with its recovery code; a wrong one
/// spends nothing, and a code is spent once.
fn recovery(
    http: &Http,
    browser: &Browser,
    runtime: &Runtime,
    operator: &DatabaseConnection,
    issue: &dyn Fn(Identity) -> ResetCode,
) {
    let registered = register("reset-web-a", http, browser, runtime, operator);
    let identity = registered.identity;
    let code = issue(identity);
    let again = || ResetCode::parse(&code.display()).unwrap();
    // A wrong recovery code is refused at the client; the code stays.
    let wrong = RecoveryCode::generate().unwrap();
    assert_eq!(
        central_client::reset(
            http,
            "reset-web-a".into(),
            again(),
            NEW.into(),
            ResetData::Recovery(&wrong)
        )
        .err(),
        Some(Error::Rejected)
    );
    assert_eq!(
        central_client::reset(
            http,
            "reset-web-a".into(),
            again(),
            NEW.into(),
            ResetData::Recovery(&registered.code)
        )
        .unwrap()
        .map(|_| ()),
        None
    );
    // The same data opens under the new password; the old one is refused.
    let session = central_client::login(http, identity, NEW.into()).unwrap();
    assert_eq!(session.document(), DOCUMENT);
    session.logout(http).unwrap();
    assert_eq!(
        central_client::login(http, identity, OLD.into()).err(),
        Some(Error::Rejected)
    );
    // The spent code starts nothing.
    assert_eq!(
        central_client::reset(
            http,
            "reset-web-a".into(),
            again(),
            OLD.into(),
            ResetData::Recovery(&registered.code)
        )
        .err(),
        Some(Error::Rejected)
    );
    twice(http, browser, identity, issue);
}

/// Two starts of one code, by hand: a finish on the other listener is refused
/// and consumed, the first finish spends the code, the second is refused.
/// Then a start spends a unit of the account's budget, as a sign-in does.
fn twice(
    http: &Http,
    browser: &Browser,
    identity: Identity,
    issue: &dyn Fn(Identity) -> ResetCode,
) {
    let code = issue(identity);
    let start = || {
        let (enrollment, registration) =
            Enrollment::start(OLD.into(), context(http, identity)).unwrap();
        let Response::ResetChallenge {
            attempt,
            registration,
            identity: answered,
            recovery,
        } = http
            .exchange(
                &Request::ResetStart {
                    username: "reset-web-a".into(),
                    code: ResetCode::parse(&code.display()).unwrap(),
                    registration,
                },
                None,
            )
            .unwrap()
        else {
            panic!("a reset challenge");
        };
        assert_eq!(answered.account, identity.account);
        assert!(recovery.is_some(), "a registered account's wrapper");
        let enrolled = enrollment.finish(&registration).unwrap();
        let master = central_crypto::vault::MasterKey::generate().unwrap();
        Request::ResetFinish {
            attempt,
            record: enrolled.upload,
            wrapped_master: enrolled
                .wrapping_key
                .wrap(&master)
                .unwrap()
                .as_bytes()
                .to_vec(),
            vault: ResetVault::Keep,
        }
    };
    let (first, second) = (start(), start());
    assert_eq!(browser.answer(&second, None).err(), Some(Error::Rejected));
    assert!(matches!(
        http.exchange(&first, None).unwrap(),
        Response::Done
    ));
    assert_eq!(http.exchange(&second, None).err(), Some(Error::Rejected));
    assert_eq!(http.exchange(&first, None).err(), Some(Error::Rejected));
    // Six units so far (two resets, two sign-ins, these two starts); two
    // failed sign-ins make eight, and a start with a good code is `Busy`.
    for _ in 0..2 {
        assert_eq!(
            central_client::login(http, identity, OLD.into()).err(),
            Some(Error::Rejected)
        );
    }
    let code = issue(identity);
    assert_eq!(
        central_client::reset(
            http,
            "reset-web-a".into(),
            code,
            NEW.into(),
            ResetData::New(DOCUMENT)
        )
        .err(),
        Some(Error::Busy)
    );
}

/// A moved account starts a new vault: a new recovery code, the next epoch,
/// the new document. Wrong codes before it spent nothing of its budget.
fn renewed(http: &Http, bridge: &Token, issue: &dyn Fn(Identity) -> ResetCode) {
    let (identity, session) = moved(http, bridge, "reset-web-b");
    close(http, bridge, "reset-web-b", identity);
    session.logout(http).unwrap();
    let code = issue(identity);
    // Eight wrong codes after the sign-in's one unit: were they charged, the
    // reset below would be the tenth unit of the minute. A fresh second keeps
    // them inside the global budget for requests without a session.
    std::thread::sleep(std::time::Duration::from_secs(1));
    for _ in 0..8 {
        assert_eq!(
            central_client::reset(
                http,
                "reset-web-b".into(),
                ResetCode::generate().unwrap(),
                NEW.into(),
                ResetData::New(DOCUMENT)
            )
            .err(),
            Some(Error::Rejected)
        );
    }
    let new_code = central_client::reset(
        http,
        "reset-web-b".into(),
        code,
        NEW.into(),
        ResetData::New(br#"{"version":1,"gateways":[],"settings":{"new":true}}"#),
    )
    .unwrap()
    .expect("a new recovery code");
    assert_eq!(new_code.to_text().len(), 39);
    let next = Identity {
        account: identity.account,
        epoch: identity.epoch + 1,
    };
    let session = central_client::login(http, next, NEW.into()).unwrap();
    assert_eq!(
        session.document(),
        br#"{"version":1,"gateways":[],"settings":{"new":true}}"#
    );
    session.logout(http).unwrap();
    // The old epoch signs in to nothing.
    assert!(central_client::login(http, identity, NEW.into()).is_err());
}

/// A device still signed in keeps the data with the key it holds; its
/// session ends with the reset.
fn device(http: &Http, bridge: &Token, issue: &dyn Fn(Identity) -> ResetCode) {
    let (identity, session) = moved(http, bridge, "reset-web-c");
    close(http, bridge, "reset-web-c", identity);
    let code = issue(identity);
    // Another account's name with this code finds nothing.
    assert_eq!(
        session
            .redeem_reset(
                http,
                "reset-web-b".into(),
                ResetCode::parse(&code.display()).unwrap(),
                NEW.into()
            )
            .err(),
        Some(Error::Rejected)
    );
    session
        .redeem_reset(http, "reset-web-c".into(), code, NEW.into())
        .unwrap();
    assert_eq!(session.check_active(http), Err(Error::Rejected));
    let session = central_client::login(http, identity, NEW.into()).unwrap();
    assert_eq!(session.document(), DOCUMENT);
    session.logout(http).unwrap();
}
