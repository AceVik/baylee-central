//! Actual OPAQUE reauthentication carries only the presented browser's push consent.
use super::{
    Http,
    browser::Browser,
    password::{OLD, browser_sign_in, context, moved},
};
use central_api::{
    Token,
    notifications::{NotificationRequest, Subscription},
    wire::{Request, Response},
};
use central_crypto::opaque::Login;
use central_postgres::AccountStore;
use tokio::runtime::Runtime;

pub(crate) fn run(
    http: &Http,
    browser: &Browser,
    bridge: &Token,
    store: &AccountStore,
    runtime: &Runtime,
) {
    let browser = Browser {
        address: browser.address,
        client: "198.51.100.197",
    };
    let (identity, _) = moved(http, bridge, "push-reauth-http");
    let context = context(http, identity);
    let old = browser_sign_in(&browser, identity, context, OLD);
    let endpoint = "https://fcm.googleapis.com/fcm/send/synthetic-reauth-http";
    // This fixture runs only IdentityService, without an outbound push sender.
    runtime
        .block_on(store.notifications(
            &old,
            NotificationRequest::Subscribe {
                subscription: Subscription {
                    endpoint: endpoint.into(),
                    p256dh: "A".repeat(87),
                    auth: "A".repeat(22),
                    nonce: "0123456789abcdef0123456789abcdef".into(),
                    lang: "en".into(),
                },
            },
        ))
        .unwrap();
    let (login, message) = Login::start(OLD.into(), context).unwrap();
    let Response::LoginChallenge { attempt, message } = browser
        .answer(&Request::LoginStart { identity, message }, Some(&old))
        .unwrap()
    else {
        panic!("login challenge");
    };
    let proof = login.finish(&message).unwrap();
    let reply = browser
        .call(
            &Request::LoginFinish {
                attempt,
                message: proof.finalization,
            },
            Some(&old),
        )
        .unwrap();
    let current = reply.session.expect("new browser cookie");
    browser.answer(&Request::Logout, Some(&old)).unwrap();
    let read = || NotificationRequest::Read {
        endpoint: Some(endpoint.into()),
    };
    let Response::Notifications { state } = browser
        .answer(&Request::Notifications { request: read() }, Some(&current))
        .unwrap()
    else {
        panic!("notification state");
    };
    assert!(
        state.subscribed,
        "reauthentication preserves consent beyond the old cookie"
    );
    browser.answer(&Request::Logout, Some(&current)).unwrap();
    let returned = browser_sign_in(&browser, identity, context, OLD);
    assert!(
        !runtime
            .block_on(store.notifications(&returned, read()))
            .unwrap()
            .subscribed,
        "server-side logout revokes consent even without a client unsubscribe"
    );
    browser.answer(&Request::Logout, Some(&returned)).unwrap();
}
