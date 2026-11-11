//! Bearers nobody issued (`docs/native-login.md` §"A bearer nobody
//! issued"): a flood of them keeps no session out. The flood is asked of
//! the service the listeners serve, in one instant and in a set order,
//! which no client over a socket can do reliably; the sessions it must
//! not keep out were signed in over the real listeners. The numbers, the
//! half hour and the table's bound are counted with a clock of their own
//! in `central-identity`'s unit tests.
use super::{
    Http,
    browser::Browser,
    password::{self, OLD},
};
use central_api::{
    Error, Token,
    gateway::GatewayMetadata,
    notifications::NotificationRequest,
    social::SocialRequest,
    wire::{Request, Response, TicketPurpose},
};
use central_crypto::opaque::server::ServerKeys;
use central_identity::{IdentityService, Place, Seen};
use central_postgres::AccountStore;
use std::{future::Future, pin::Pin, task::Poll, time::Duration};
use tokio::runtime::Runtime;

type Asked<'a> = Pin<Box<dyn Future<Output = Result<Response, Error>> + 'a>>;

/// Every request begun in the order given, each as far as its first wait,
/// so the budgets have counted them in that order; then all answered.
async fn in_order(asked: Vec<Asked<'_>>) -> Vec<Result<Response, Error>> {
    let mut asked: Vec<_> = asked.into_iter().map(|request| (request, None)).collect();
    std::future::poll_fn(|context| {
        let mut waiting = false;
        for (request, answer) in &mut asked {
            if answer.is_none() {
                match request.as_mut().poll(context) {
                    Poll::Ready(ready) => *answer = Some(ready),
                    Poll::Pending => waiting = true,
                }
            }
        }
        if waiting {
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    })
    .await;
    asked
        .into_iter()
        .map(|(_, answer)| answer.expect("answered"))
        .collect()
}

/// As the account listener hands a request on.
fn native<'a>(
    service: &'a IdentityService,
    request: Request,
    bearer: &'a Token,
    from: Place,
) -> Asked<'a> {
    Box::pin(async move { service.handle_from(request, Some(bearer), Some(from)).await })
}

/// As the website's listener hands a request on.
fn web<'a>(
    service: &'a IdentityService,
    request: Request,
    cookie: &'a Token,
    from: Place,
) -> Asked<'a> {
    Box::pin(async move {
        service
            .handle_browser_from(request, Some(cookie), Some(from), Seen::default())
            .await
            .map(|reply| reply.response)
    })
}

fn place(last: u8) -> Place {
    Place::V4([203, 0, 113, last])
}

fn invented(count: usize) -> Vec<Token> {
    (0..count).map(|_| Token::generate().unwrap()).collect()
}

/// More requests with made-up bearers than all sessions together have in
/// a second, of both kinds and on both listeners, each from the place
/// `from` gives it.
fn flood<'a>(
    service: &'a IdentityService,
    tokens: &'a [Token],
    from: impl Fn(usize) -> Place,
) -> Vec<Asked<'a>> {
    tokens
        .iter()
        .enumerate()
        .map(|(index, token)| match index % 3 {
            0 => web(service, Request::SessionStatus, token, from(index)),
            1 => native(service, Request::SessionStatus, token, from(index)),
            _ => native(service, Request::ReadDocument, token, from(index)),
        })
        .collect()
}

/// The requests whose answer vouches for a bearer, as far as the account
/// listener serves them. (A pulse is a heartbeat with numbers, on the
/// heartbeat's path.)
fn vouching() -> Vec<Request> {
    let purpose = TicketPurpose::Admit;
    vec![
        Request::SessionStatus,
        Request::RenewSession,
        Request::TermsStatus,
        Request::Social {
            request: SocialRequest::Contacts,
        },
        Request::CreateObject,
        Request::ReadObject { id: [7; 16] },
        Request::ListObjects { after: None },
        Request::WriteObject {
            id: [7; 16],
            expected_revision: 1,
            document: vec![7; 64],
        },
        Request::DeleteObject {
            id: [7; 16],
            expected_revision: 1,
        },
        Request::ReadDocument,
        Request::WriteDocument {
            expected_revision: 1,
            document: vec![7; 64],
        },
        Request::GatewayTicket {
            gateway: "http://127.0.0.1:28766".into(),
            purpose,
        },
        Request::GatewaySession {
            lease: Token::generate().unwrap(),
            purpose,
        },
        Request::GatewayProfile {
            lease: Token::generate().unwrap(),
            purpose,
        },
        Request::RedeemGateway {
            ticket: Token::generate().unwrap(),
            purpose,
        },
        Request::GatewayHeartbeat {
            metadata: GatewayMetadata {
                name: "Synthetic".into(),
                version: "1".into(),
                website: "https://gateway.invalid/".into(),
            },
        },
        Request::GatewayStatus,
        Request::CheckGatewayAdmin { account: [7; 16] },
    ]
}

/// And as far as the website's listener serves them.
fn vouching_on_the_website() -> Vec<Request> {
    vec![
        Request::SessionStatus,
        Request::RenewSession,
        Request::TermsStatus,
        Request::AccountSummary,
        Request::Social {
            request: SocialRequest::Contacts,
        },
        Request::Notifications {
            request: NotificationRequest::Read { endpoint: None },
        },
    ]
}

/// The four requests of the account's two sessions that the flood must
/// not keep out: a control and a data request on each listener.
fn holders<'a>(
    service: &'a IdentityService,
    cookie: &'a Token,
    bearer: &'a Token,
    from: Place,
) -> Vec<Asked<'a>> {
    vec![
        web(service, Request::SessionStatus, cookie, from),
        web(service, Request::KeyCheckStatus, cookie, from),
        native(service, Request::SessionStatus, bearer, from),
        native(service, Request::ReadDocument, bearer, from),
    ]
}

fn second() {
    std::thread::sleep(Duration::from_millis(1_100));
}

pub(crate) fn run(
    service: &IdentityService,
    store: &AccountStore,
    keys: &ServerKeys,
    bridge: &Token,
    http: &Http,
    browser: &Browser,
    runtime: &Runtime,
) {
    // Central after a restart: a second service on the same store, with
    // the same keys, that has heard nobody yet.
    let restarted = IdentityService::new(
        store.clone(),
        &http.endpoint,
        ServerKeys::from_secret_bytes(&keys.to_secret_bytes()).unwrap(),
        bridge,
        "http://127.0.0.1:28766",
    )
    .unwrap()
    .with_terms(Some(super::terms::central_document()));
    let home = Browser {
        address: browser.address,
        client: "203.0.113.161",
    };
    let account = password::register("among-strangers", http, &home);
    // Both sessions over the real listeners: nothing but the sign-in
    // itself has vouched for them when the flood begins.
    let cookie = password::browser_sign_in(&home, account.identity, account.context, OLD);
    let (bearer, _) = password::sign_in(http, account.identity, OLD);

    // One address, 900 made-up bearers in one instant: 600 control
    // requests where all sessions share 512 a second, 300 data requests
    // where they share 256. The holder's four requests come last.
    second();
    let tokens = invented(900);
    let mut asked = flood(service, &tokens, |_| place(162));
    asked.extend(holders(service, &cookie, &bearer, place(161)));
    let mut answers = runtime.block_on(in_order(asked));
    let held = answers.split_off(tokens.len());
    for answer in answers {
        assert!(
            matches!(answer, Err(Error::Rejected | Error::Busy)),
            "a bearer nobody issued is answered nothing"
        );
    }
    for (index, answer) in held.iter().enumerate() {
        assert!(
            answer.is_ok(),
            "the holder's request {index} beside the flood: {:?}",
            answer.as_ref().err()
        );
    }
    // Nor did the flood take anything from those who are signing in.
    assert!(
        runtime
            .block_on(service.handle_from(Request::Terms, None, Some(place(163))))
            .is_ok()
    );

    // No request whose answer vouches for a bearer answers one that
    // nobody issued: each is refused by the store, not by a budget.
    second();
    let nobody = Token::generate().unwrap();
    let mut asked: Vec<_> = vouching()
        .into_iter()
        .map(|request| native(service, request, &nobody, place(164)))
        .collect();
    asked.extend(
        vouching_on_the_website()
            .into_iter()
            .map(|request| web(service, request, &nobody, place(164))),
    );
    for (index, answer) in runtime.block_on(in_order(asked)).into_iter().enumerate() {
        assert_eq!(answer.err(), Some(Error::Rejected), "request {index}");
    }

    after_a_restart(&restarted, &cookie, &bearer, runtime);
    replaced(service, http, &account, bearer, runtime);
}

/// A Central that has just started knows no session: `restarted` is a
/// second service on the same store. A session's first answered request
/// vouches for it again; until then it waits with the strangers.
fn after_a_restart(restarted: &IdentityService, cookie: &Token, bearer: &Token, runtime: &Runtime) {
    assert!(
        runtime
            .block_on(web(restarted, Request::SessionStatus, cookie, place(161)))
            .is_ok(),
        "served, and vouched for from here on"
    );
    // Sixteen addresses now, so no place's share ends the flood early.
    second();
    let tokens = invented(900);
    let mut asked = flood(restarted, &tokens, |index| {
        place(200 + u8::try_from(index % 16).unwrap())
    });
    asked.push(web(restarted, Request::SessionStatus, cookie, place(161)));
    asked.push(web(restarted, Request::KeyCheckStatus, cookie, place(161)));
    // The native session has asked nothing since the restart.
    asked.push(native(
        restarted,
        Request::SessionStatus,
        bearer,
        place(161),
    ));
    let mut answers = runtime.block_on(in_order(asked));
    let unheard = answers.pop().unwrap();
    let data = answers.pop().unwrap();
    let control = answers.pop().unwrap();
    assert!(
        control.is_ok() && data.is_ok(),
        "the session that was heard"
    );
    // What stays: a session not yet heard since the restart shares the
    // strangers' second, and waits while they fill it.
    assert_eq!(unheard.err(), Some(Error::Busy));
    second();
    assert!(
        runtime
            .block_on(native(
                restarted,
                Request::SessionStatus,
                bearer,
                place(161)
            ))
            .is_ok()
    );
}

/// A session that a later sign-in replaced was vouched for when it was
/// issued. Four refusals later it is a stranger's token again: four such
/// tokens, each asking for its whole budget, leave the live session's
/// second alone.
fn replaced(
    service: &IdentityService,
    http: &Http,
    account: &password::Registered,
    first: Token,
    runtime: &Runtime,
) {
    let mut ended = vec![first];
    let live = loop {
        let (next, _) = password::sign_in(http, account.identity, OLD);
        if ended.len() == 4 {
            break next;
        }
        ended.push(next);
    };
    for token in &ended {
        for _ in 0..4 {
            assert_eq!(
                runtime
                    .block_on(native(service, Request::SessionStatus, token, place(161)))
                    .err(),
                Some(Error::Rejected)
            );
        }
    }
    second();
    let mut asked = Vec::new();
    for token in &ended {
        for _ in 0..128 {
            asked.push(native(service, Request::SessionStatus, token, place(165)));
        }
    }
    asked.push(native(service, Request::SessionStatus, &live, place(161)));
    let answer = runtime.block_on(in_order(asked)).pop().unwrap();
    assert!(
        answer.is_ok(),
        "the live session beside four ended ones: {:?}",
        answer.err()
    );
}
