//! The real native/browser boundaries restrict generator/result authority to the bridge.
use super::{
    Http,
    browser::Browser,
    password::{OLD, browser_sign_in, context, moved},
};
use central_api::{
    Token, challenges as api,
    wire::{Request, Response},
};
use central_client::Transport;
use std::time::{SystemTime, UNIX_EPOCH};

fn host() -> Request {
    let now = i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    )
    .unwrap();
    Request::ChallengeHost {
        request: api::HostRequest::Inventory {
            from: now,
            until: now + 86400,
        },
    }
}

pub(crate) fn refuse_gateway(http: &Http, credential: &Token, heartbeat: &Token) {
    for token in [credential, heartbeat] {
        assert!(
            http.exchange(&host(), Some(token.into())).is_err(),
            "approved ordinary gates are not challenge authorities"
        );
    }
}

pub(crate) fn run(http: &Http, browser: &Browser, bridge: &Token) {
    let browser = Browser {
        address: browser.address,
        client: "198.51.100.198",
    };
    let (identity, session) = moved(http, bridge, "challenge-http");
    assert!(matches!(
        central_client::challenges::read(http, api::Request::Catalogue { before: None }).unwrap(),
        api::Response::Catalogue { .. }
    ));
    assert!(matches!(
        session.challenges(http, api::Request::Progress).unwrap(),
        api::Response::Progress { .. }
    ));
    assert!(
        http.exchange(
            &Request::Challenges {
                request: api::Request::Progress
            },
            None
        )
        .is_err()
    );
    assert!(
        http.exchange(
            &Request::Challenges {
                request: api::Request::Progress
            },
            Some(bridge.into())
        )
        .is_err()
    );
    assert!(http.exchange(&host(), None).is_err());
    assert!(
        http.exchange(&host(), Some((&Token::generate().unwrap()).into()))
            .is_err()
    );
    assert!(matches!(
        http.exchange(&host(), Some(bridge.into())).unwrap(),
        Response::ChallengeHost {
            response: api::HostResponse::Inventory { .. }
        }
    ));
    let cookie = browser_sign_in(&browser, identity, context(http, identity), OLD);
    let stale = Token::generate().unwrap();
    assert!(
        browser
            .answer(
                &Request::Challenges {
                    request: api::Request::Catalogue { before: None }
                },
                Some(&stale)
            )
            .is_ok(),
        "an expired cookie cannot hide public tasks"
    );
    assert!(matches!(
        browser
            .answer(
                &Request::Challenges {
                    request: api::Request::Catalogue { before: None }
                },
                None
            )
            .unwrap(),
        Response::Challenges {
            response: api::Response::Catalogue { .. }
        }
    ));
    assert!(matches!(
        browser
            .answer(
                &Request::Challenges {
                    request: api::Request::Progress
                },
                Some(&cookie)
            )
            .unwrap(),
        Response::Challenges {
            response: api::Response::Progress { .. }
        }
    ));
    for bearer in [None, Some(&cookie), Some(bridge)] {
        assert!(
            browser.answer(&host(), bearer).is_err(),
            "no browser cookie can reach host operations"
        );
    }
    // A raw cookie capability on the native route is not a signed session.
    assert!(http.exchange(&host(), Some((&cookie).into())).is_err());
    browser.answer(&Request::Logout, Some(&cookie)).unwrap();
}
