//! Bearers nobody issued (`docs/native-login.md` §"A bearer nobody
//! issued"): a flood of them keeps no session out. The flood is asked of
//! the service the listeners serve, in one instant and in a set order,
//! which no client over a socket can do reliably; the sessions it must
//! not keep out were signed in over the real listeners. The numbers, the
//! half hour and the table's bound are counted with a clock of their own
//! in `central-identity`'s unit tests.
//!
//! On the account listener a session's bearer is a signed token, read
//! before the budgets are: a made-up one is refused at its signature, and a
//! capability presented bare is refused for a client's operation
//! (`central_identity::credential_operation`). So the flood here is what
//! still reaches the budgets: capabilities as if their signature had
//! verified, which over a socket is the token of a session that ended.
use super::{
    Http,
    browser::Browser,
    password::{self, OLD},
};
use central_api::{
    Error, SignedToken, Token,
    gateway::GatewayMetadata,
    notifications::NotificationRequest,
    owner::GatewayScopes,
    social::SocialRequest,
    wire::{GatewayPurpose, Request, Response},
};
use central_crypto::opaque::server::ServerKeys;
use central_identity::{IdentityService, Place, Presented, Seen, credential_operation};
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

/// As the account listener hands a request on once it read the bearer: a
/// gateway's operation with a bare credential, any other with a verified
/// session's capability.
fn native<'a>(
    service: &'a IdentityService,
    request: Request,
    bearer: &'a Token,
    from: Place,
) -> Asked<'a> {
    let capability = Token::from_secret_bytes(bearer.to_secret_bytes());
    let presented = if credential_operation(&request) {
        Presented::Credential(capability)
    } else {
        Presented::Session(capability)
    };
    Box::pin(async move {
        service
            .handle_from(request, Some(&presented), Some(from))
            .await
    })
}

/// The capability inside a native session's signed token, as the account
/// listener reads it.
fn capability(service: &IdentityService, signed: &SignedToken) -> Token {
    let Ok(Presented::Session(capability)) = service.present(&signed.bearer(), None) else {
        panic!("a session's own token verifies");
    };
    capability
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
        Request::GatewayToken {
            gateway: "http://127.0.0.1:28766".into(),
            purpose: GatewayPurpose::Admit,
        },
        Request::RegisterInvalidate {
            path: "/auth/central/invalidate".into(),
        },
        Request::GatewayRevocations { after: 0 },
        Request::GatewayHeartbeat {
            metadata: GatewayMetadata {
                name: "Synthetic".into(),
                version: "1".into(),
                website: "https://gateway.invalid/".into(),
            },
        },
        Request::GatewayStatus,
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
        super::token_keys(),
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
    let (signed, _) = password::sign_in(http, account.identity, OLD);
    let bearer = capability(service, &signed);

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

    one_owner_s_tokens(service, store, &cookie, &bearer, runtime);
    after_a_restart(&restarted, &cookie, &bearer, runtime);
    replaced(service, http, &account, bearer, runtime);
    made_up_signatures(service, &restarted, http, &account, &signed, runtime);
}

/// Made-up signed tokens, read as the account listener reads a bearer (`present`): each
/// costs a signature check, of its place's quarter of the second's checks and then of all
/// places' (`limits.rs`). A token Central issued or verified is not checked again
/// (`verified.rs`), so a flood that spends every check leaves a session its requests, and a
/// session that ended is still refused by the store.
fn made_up_signatures(
    service: &IdentityService,
    restarted: &IdentityService,
    http: &Http,
    account: &password::Registered,
    first: &SignedToken,
    runtime: &Runtime,
) {
    const MADE_UP: &str = "Bearer a.b";
    let quarter = 1_024 / 4;
    let session = |presented: Result<Presented, Error>| match presented {
        Ok(Presented::Session(capability)) => capability,
        Ok(Presented::Credential(_)) => panic!("a signed token is no credential"),
        Err(error) => panic!("a session's token beside the flood: {error:?}"),
    };

    // A Central that has verified nothing yet, and one address: a quarter of the second's
    // checks, each refused at its signature, then nothing more is checked for it.
    second();
    let answers: Vec<_> = (0..1_024)
        .map(|_| restarted.present(MADE_UP, Some(place(165))).err())
        .collect();
    assert!(
        answers[..quarter]
            .iter()
            .all(|answer| *answer == Some(Error::Rejected))
    );
    assert!(
        answers[quarter..]
            .iter()
            .all(|answer| *answer == Some(Error::Busy))
    );
    // Another address's real token is still checked, and verifies.
    session(restarted.present(&first.bearer(), Some(place(166))));
    // Four addresses spend the whole second: a fifth's token is not checked, nor one
    // from no known place ...
    for last in 167..=170 {
        for _ in 0..quarter {
            let _ = restarted.present(MADE_UP, Some(place(last)));
        }
    }
    assert_eq!(
        restarted.present(MADE_UP, Some(place(171))).err(),
        Some(Error::Busy)
    );
    assert_eq!(restarted.present(MADE_UP, None).err(), Some(Error::Busy));
    // ... but the token verified a moment ago needs no check.
    session(restarted.present(&first.bearer(), Some(place(171))));

    // A token Central issued needs no check even for its first request.
    let (fresh, _) = password::sign_in(http, account.identity, OLD);
    second();
    for last in 167..=170 {
        for _ in 0..quarter {
            let _ = service.present(MADE_UP, Some(place(last)));
        }
    }
    assert_eq!(
        service.present(MADE_UP, Some(place(171))).err(),
        Some(Error::Busy)
    );
    let live = session(service.present(&fresh.bearer(), Some(place(161))));
    assert!(
        runtime
            .block_on(native(service, Request::SessionStatus, &live, place(161)))
            .is_ok()
    );
    // The first token's session was replaced long ago. Its token is still read as what it
    // says, without a check, and the store refuses the session it names.
    let ended = session(service.present(&first.bearer(), Some(place(161))));
    assert_eq!(
        runtime
            .block_on(native(service, Request::SessionStatus, &ended, place(161)))
            .err(),
        Some(Error::Rejected)
    );
}

/// One account's gateway tokens are counted together. Four tokens of two
/// gateways, each asking a bearer's whole data budget in one instant and
/// each from an address of its own, were the whole of what sessions share
/// (4 x 64). They have one budget between them now, whichever gateway
/// they are of, and the account's own session is served beside them.
fn one_owner_s_tokens(
    service: &IdentityService,
    store: &AccountStore,
    cookie: &Token,
    bearer: &Token,
    runtime: &Runtime,
) {
    let scopes = GatewayScopes {
        sso: true,
        listing: true,
    };
    let mut tokens = Vec::new();
    for origin in ["http://127.0.0.1:28771", "http://127.0.0.1:28772"] {
        let gateway = runtime
            .block_on(store.add_gateway(cookie, origin, "Counted together", true))
            .unwrap();
        for name in ["First", "Second"] {
            let token = Token::generate().unwrap();
            runtime
                .block_on(store.create_gateway_token(cookie, gateway, name, scopes, 1, &token))
                .unwrap();
            tokens.push(token);
        }
    }
    // Heard once each: the answer vouches for the token, and the store
    // says whose gateway it is of.
    for token in &tokens {
        let heard = runtime.block_on(native(service, Request::GatewayStatus, token, place(170)));
        assert!(heard.is_ok(), "a token's status: {:?}", heard.err());
    }
    second();
    let mut asked = Vec::new();
    for (token, last) in tokens.iter().zip(171..) {
        for _ in 0..64 {
            asked.push(native(service, Request::GatewayStatus, token, place(last)));
        }
    }
    asked.push(native(service, Request::ReadDocument, bearer, place(161)));
    let mut answers = runtime.block_on(in_order(asked));
    let held = answers.pop().unwrap();
    let served = answers.iter().filter(|answer| answer.is_ok()).count();
    assert_eq!(served, 64, "one owner's tokens, one bearer's budget");
    for answer in answers {
        assert!(matches!(answer, Ok(_) | Err(Error::Busy)));
    }
    assert!(
        held.is_ok(),
        "the account's session beside its gateways' tokens: {:?}",
        held.err()
    );
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
        let (signed, _) = password::sign_in(http, account.identity, OLD);
        let next = capability(service, &signed);
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
