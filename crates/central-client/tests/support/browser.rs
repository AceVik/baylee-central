//! The website's flow over the real browser listener, as the web pages and
//! their WebAssembly crypto run it: register with a beta key, enroll with a
//! recovery wrapper, sign in to a browser session held in the cookie, accept
//! Central's terms, sign out. Then the same account plays natively, and the
//! official gateway's bridge admits it without a legacy ID. Last, an account
//! moved from the official gateway signs in on the website, accepts the terms
//! there and stores the recovery code the page makes for it.
use super::Http;
use central_api::{
    Error, Token,
    beta_key::BetaKey,
    terms::TermsSubject,
    wire::{AccountSource, Discovery, Identity, Request, Response, TicketPurpose},
};
use central_client::Transport;
use central_crypto::{
    Context,
    opaque::{Enrollment, Login},
    vault::{MasterKey, ObjectContext, RecoveryCode, WrappedMasterKey},
};
use central_identity::BrowserReply;
use central_postgres::BetaKeyBatch;
use sea_orm::{ConnectionTrait, DatabaseConnection};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    time::Duration,
};
use tokio::runtime::Runtime;

const PASSWORD: &str = "synthetic browser password";
const GATEWAY: &str = "http://127.0.0.1:28766";
/// The initial account document of `docs/web-accounts.md`, with this fixture's gateway.
const DOCUMENT: &[u8] = br#"{"version":1,"gateways":["http://127.0.0.1:28766"],"preferences":{},"legacy":null,"chat":{"devices":[],"installations":[]}}"#;
const SET: &str = "; Path=/; Secure; HttpOnly; SameSite=Strict; Max-Age=43200";
const CLEARED: &str = "__Host-central=; Path=/; Secure; HttpOnly; SameSite=Strict; Max-Age=0";

/// The page's `fetch`, as `baylee.app`'s Caddy passes it to the listener.
pub(crate) struct Browser {
    pub(crate) address: SocketAddr,
    /// The client address the proxy appends to `X-Forwarded-For`.
    pub(crate) client: &'static str,
}

/// A raw answer: status, `Set-Cookie` and body.
struct Answer {
    status: u16,
    cookie: Option<String>,
    body: Vec<u8>,
}

impl Browser {
    fn post(&self, origin: &str, body: &[u8], cookie: Option<&Token>) -> Answer {
        let mut stream = TcpStream::connect_timeout(&self.address, Duration::from_secs(3)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        write!(
            stream,
            "POST /api/v1 HTTP/1.1\r\nHost: baylee.app\r\nOrigin: {origin}\r\nSec-Fetch-Site: same-origin\r\n\
             Content-Type: application/json\r\nX-Baylee-Central: 1\r\nX-Forwarded-For: {}\r\nContent-Length: {}\r\n",
            self.client,
            body.len()
        )
        .unwrap();
        if let Some(token) = cookie {
            write!(
                stream,
                "Cookie: theme=dark; __Host-central={}\r\n",
                token.hex().as_str()
            )
            .unwrap();
        }
        stream.write_all(b"\r\n").unwrap();
        stream.write_all(body).unwrap();
        let mut response = Vec::new();
        stream.take(1 << 20).read_to_end(&mut response).unwrap();
        let split = response.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
        let head = std::str::from_utf8(&response[..split]).unwrap().to_owned();
        assert!(head.to_lowercase().contains("cache-control: no-store"));
        assert!(!head.to_lowercase().contains("access-control-"));
        let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
        let cookies: Vec<&str> = head
            .lines()
            .filter_map(|line| line.split_once(':'))
            .filter(|(name, _)| name.eq_ignore_ascii_case("set-cookie"))
            .map(|(_, value)| value.trim())
            .collect();
        assert!(cookies.len() <= 1);
        Answer {
            status,
            cookie: cookies.first().map(|value| (*value).to_owned()),
            body: response[split + 4..].to_vec(),
        }
    }

    fn call(&self, request: &Request, cookie: Option<&Token>) -> Result<BrowserReply, Error> {
        let logout = matches!(request, Request::Logout);
        let answer = self.post(
            "https://baylee.app",
            &central_api::encode(&request)?,
            cookie,
        );
        if logout {
            assert_eq!(
                answer.cookie.as_deref(),
                Some(CLEARED),
                "signing out clears the cookie"
            );
        }
        match answer.status {
            200 => {}
            403 => return Err(Error::TermsRequired),
            409 => return Err(Error::Conflict),
            429 => return Err(Error::Busy),
            503 => return Err(Error::Unavailable),
            _ => return Err(Error::Rejected),
        }
        let response = central_api::decode(&answer.body)?;
        let session = match answer.cookie {
            Some(cookie) if !logout => {
                let value = cookie.strip_prefix("__Host-central=").unwrap();
                let value = value
                    .strip_suffix(SET)
                    .expect("the cookie's exact attributes");
                let token = Token::from_hex(value).unwrap();
                // The token is in the cookie and nowhere in the body.
                assert!(!String::from_utf8_lossy(&answer.body).contains(value));
                Some(token)
            }
            _ => None,
        };
        Ok(BrowserReply { response, session })
    }

    /// The listener's own refusal: 403 before any account operation.
    fn refused(&self, request: &Request, cookie: Option<&Token>) -> bool {
        let answer = self.post(
            "https://baylee.app",
            &central_api::encode(request).unwrap(),
            cookie,
        );
        answer.status == 403 && answer.body == b"request not permitted\n" && answer.cookie.is_none()
    }

    /// An answer that issues no session.
    fn answer(&self, request: &Request, cookie: Option<&Token>) -> Result<Response, Error> {
        let reply = self.call(request, cookie)?;
        assert!(reply.session.is_none());
        Ok(reply.response)
    }
}

fn copy(token: &Token) -> Token {
    Token::from_secret_bytes(token.to_secret_bytes())
}

pub(crate) fn run(
    browser: &Browser,
    http: &Http,
    bridge: &Token,
    runtime: &Runtime,
    operator: &DatabaseConnection,
) {
    let key = BetaKey::generate().unwrap();
    runtime
        .block_on(
            BetaKeyBatch::new(&[key.digest()], 1, None, "browser fixture")
                .unwrap()
                .issue(operator),
        )
        .unwrap();
    let register = |name: &str| Request::Register {
        username: name.into(),
        beta_key: key.display().as_str().to_lowercase(),
    };
    // Registration is the browser listener's; the native surface refuses it.
    assert_eq!(
        http.exchange(&register("web-player"), None).err(),
        Some(Error::Rejected)
    );
    assert_eq!(
        browser.answer(&register("synthetic"), None).err(),
        Some(Error::Conflict),
        "a valid key learns that the name is taken"
    );
    let Response::Enrollment {
        identity: account,
        ticket,
    } = browser.answer(&register("web-player"), None).unwrap()
    else {
        panic!("an enrollment");
    };
    let context = Context::new(http.endpoint.issuer(), account.account, account.epoch).unwrap();
    let (enrollment, message) = Enrollment::start(PASSWORD.into(), context).unwrap();
    let Response::EnrollmentChallenge { message } = browser
        .answer(
            &Request::EnrollmentStart {
                ticket: copy(&ticket),
                message,
            },
            None,
        )
        .unwrap()
    else {
        panic!("a challenge");
    };
    let result = enrollment.finish(&message).unwrap();
    let master = MasterKey::generate().unwrap();
    let wrapped = result.wrapping_key.wrap(&master).unwrap();
    let code = RecoveryCode::generate().unwrap();
    let recovery = code.wrap(context, &master).unwrap();
    let object = ObjectContext::new(context, account.account, 1).unwrap();
    let document = master.seal_object(object, DOCUMENT).unwrap();
    let finish = |recovery: Option<&WrappedMasterKey>| Request::EnrollmentFinish {
        ticket: copy(&ticket),
        record: result.upload.clone(),
        wrapped_master: wrapped.as_bytes().to_vec(),
        document: document.clone(),
        recovery: recovery.map(|r| r.as_bytes().to_vec()),
    };
    assert_eq!(
        browser.answer(&finish(None), None).err(),
        Some(Error::Rejected)
    );
    assert!(matches!(
        browser.answer(&finish(Some(&recovery)), None).unwrap(),
        Response::Done
    ));
    // Central stores the recovery wrapper as sent: the code opens it to the same key.
    let stored: Vec<u8> = runtime
        .block_on(operator.query_one_raw(sea_orm::Statement::from_string(
            sea_orm::DbBackend::Postgres,
            "SELECT recovery_wrapped_master FROM central_identity.accounts WHERE username_key = 'web-player'",
        )))
        .unwrap()
        .unwrap()
        .try_get("", "recovery_wrapped_master")
        .unwrap();
    let opened = code
        .unwrap(context, &WrappedMasterKey::from_bytes(&stored).unwrap())
        .unwrap();
    let first = ObjectContext::new(context, account.account, 1).unwrap();
    assert_eq!(&opened.open_object(first, &document).unwrap()[..], DOCUMENT);
    budgets(browser, &key);
    let cookie = sign_in(browser, http, context);
    summary(browser, http, &cookie, account, &recovery);
    session(browser, http, bridge, &cookie);
    directory(browser, http);
    // With the terms accepted, the registration's own wrapper is `Done` again
    // and another is a conflict: a registered account keeps its code.
    let set = |r: &WrappedMasterKey| Request::SetRecovery {
        recovery: r.as_bytes().to_vec(),
    };
    assert!(matches!(
        browser.answer(&set(&recovery), Some(&cookie)).unwrap(),
        Response::Done
    ));
    let other = RecoveryCode::generate()
        .unwrap()
        .wrap(context, &master)
        .unwrap();
    assert_eq!(
        browser.answer(&set(&other), Some(&cookie)).err(),
        Some(Error::Conflict)
    );
    play(browser, http, bridge, &cookie);
    moved(browser, http, bridge, runtime, operator);
}

/// The account page's summary of a registered account; storing a code needs
/// the terms accepted, checked against the cookie's browser session.
fn summary(
    browser: &Browser,
    http: &Http,
    cookie: &Token,
    account: Identity,
    recovery: &WrappedMasterKey,
) {
    let Response::AccountSummary {
        identity,
        username,
        display_name: None,
        tag: None,
        source: AccountSource::Registered,
        recovery: true,
        admin: false,
        signed_in_at,
        expires_at,
    } = browser
        .answer(&Request::AccountSummary, Some(cookie))
        .unwrap()
    else {
        panic!("a registered account's summary");
    };
    assert_eq!(identity.account, account.account);
    assert_eq!(username, "web-player");
    assert_eq!(expires_at - signed_in_at, 1800, "issued at the sign-in");
    assert_eq!(
        browser.answer(&Request::AccountSummary, None).err(),
        Some(Error::Rejected)
    );
    assert!(
        http.exchange(&Request::AccountSummary, Some(cookie))
            .is_err()
    );
    let set = Request::SetRecovery {
        recovery: recovery.as_bytes().to_vec(),
    };
    assert_eq!(
        browser.answer(&set, Some(cookie)).err(),
        Some(Error::TermsRequired),
        "the terms in force are not accepted yet"
    );
}

/// A player of the official gateway, its move closed, signs in on the
/// website: the summary says it moved and has no code; the code the page
/// makes is stored once the terms are accepted, on the website only, and
/// opens the account's master key.
fn moved(
    browser: &Browser,
    http: &Http,
    bridge: &Token,
    runtime: &Runtime,
    operator: &DatabaseConnection,
) {
    let (context, identity, master, cookie) = moved_sign_in(browser, http, bridge);
    // What the page's `recovery_create` does: the same master key under a new code.
    let code = RecoveryCode::generate().unwrap();
    let recovery = code.wrap(context, &master).unwrap();
    let set = |r: &WrappedMasterKey| Request::SetRecovery {
        recovery: r.as_bytes().to_vec(),
    };
    // The native listener serves neither, with its own session or the cookie.
    let token = native_refuses(http, context, identity, &cookie, &set(&recovery));
    assert_eq!(
        browser.answer(&set(&recovery), None).err(),
        Some(Error::Rejected)
    );
    // Not before the terms in force are accepted, then stored once; the same
    // bytes again are done, another code a conflict.
    assert_eq!(
        browser.answer(&set(&recovery), Some(&cookie)).err(),
        Some(Error::TermsRequired)
    );
    let Response::TermsStatus {
        current: Some(current),
        accepted: false,
    } = browser
        .answer(&Request::TermsStatus, Some(&cookie))
        .unwrap()
    else {
        panic!("terms to accept");
    };
    assert!(matches!(
        browser
            .answer(
                &Request::AcceptTerms {
                    subject: TermsSubject::Central,
                    version: current.version,
                    digest: current.digest,
                },
                Some(&cookie),
            )
            .unwrap(),
        Response::Done
    ));
    for _ in 0..2 {
        assert!(matches!(
            browser.answer(&set(&recovery), Some(&cookie)).unwrap(),
            Response::Done
        ));
    }
    let other = RecoveryCode::generate()
        .unwrap()
        .wrap(context, &master)
        .unwrap();
    assert_eq!(
        browser.answer(&set(&other), Some(&cookie)).err(),
        Some(Error::Conflict)
    );
    assert!(matches!(
        browser
            .answer(&Request::AccountSummary, Some(&cookie))
            .unwrap(),
        Response::AccountSummary { recovery: true, .. }
    ));
    // The stored wrapper opens with the code to the account's master key.
    let stored: Vec<u8> = runtime
        .block_on(operator.query_one_raw(sea_orm::Statement::from_string(
            sea_orm::DbBackend::Postgres,
            "SELECT recovery_wrapped_master FROM central_identity.accounts WHERE legacy_id = 'moved-legacy'",
        )))
        .unwrap()
        .unwrap()
        .try_get("", "recovery_wrapped_master")
        .unwrap();
    let opened = code
        .unwrap(context, &WrappedMasterKey::from_bytes(&stored).unwrap())
        .unwrap();
    let probe = ObjectContext::new(context, identity.account, 9).unwrap();
    let sealed = master.seal_object(probe, b"probe").unwrap();
    assert_eq!(&opened.open_object(probe, &sealed).unwrap()[..], b"probe");
    assert!(matches!(
        browser.answer(&Request::Logout, Some(&cookie)).unwrap(),
        Response::Done
    ));
    http.exchange(&Request::Logout, Some(&token)).unwrap();
}

/// Registration is budgeted per client address, five an hour; a page from
/// another origin is refused before any of that.
fn budgets(browser: &Browser, key: &BetaKey) {
    let discover = central_api::encode(&Request::Discover {
        username: "web-player".into(),
    })
    .unwrap();
    for origin in ["https://evil.example", "null", "https://central.baylee.app"] {
        assert_eq!(
            browser.post(origin, &discover, None).status,
            403,
            "{origin}"
        );
    }
    // Two registrations so far from this address; three more with a wrong key.
    let wrong = |name: &str| Request::Register {
        username: name.into(),
        beta_key: "CENTRAL-0000-0000-0000-0000".into(),
    };
    for _ in 0..3 {
        assert_eq!(
            browser.answer(&wrong("budget-probe"), None).err(),
            Some(Error::Rejected)
        );
    }
    let right = |name: &str| Request::Register {
        username: name.into(),
        beta_key: key.display().as_str().into(),
    };
    assert_eq!(
        browser.answer(&right("budget-probe"), None).err(),
        Some(Error::Busy)
    );
    let neighbour = Browser {
        address: browser.address,
        client: "198.51.100.77",
    };
    assert_eq!(
        neighbour.answer(&wrong("budget-probe"), None).err(),
        Some(Error::Rejected)
    );
}

/// Sign-in on the website: the session goes to the cookie, never into the body.
fn sign_in(browser: &Browser, http: &Http, context: Context) -> Token {
    let Response::Discovery {
        discovery: Discovery::Central { identity },
    } = browser
        .answer(
            &Request::Discover {
                username: "Web-Player".into(),
            },
            None,
        )
        .unwrap()
    else {
        panic!("a central account");
    };
    // An attempt started on the native surface does not finish on the website.
    let (login, message) = Login::start(PASSWORD.into(), context).unwrap();
    let Response::LoginChallenge { attempt, message } = http
        .exchange(&Request::LoginStart { identity, message }, None)
        .unwrap()
    else {
        panic!("a challenge");
    };
    let result = login.finish(&message).unwrap();
    let finish = Request::LoginFinish {
        attempt,
        message: result.finalization,
    };
    assert_eq!(browser.call(&finish, None).err(), Some(Error::Rejected));
    let (login, message) = Login::start(PASSWORD.into(), context).unwrap();
    let Response::LoginChallenge { attempt, message } = browser
        .answer(&Request::LoginStart { identity, message }, None)
        .unwrap()
    else {
        panic!("a challenge");
    };
    let result = login.finish(&message).unwrap();
    let reply = browser
        .call(
            &Request::LoginFinish {
                attempt,
                message: result.finalization,
            },
            None,
        )
        .unwrap();
    let Response::BrowserSession {
        identity: signed_in,
        username,
        wrapped_master,
        expires_at,
    } = reply.response
    else {
        panic!("a browser session");
    };
    assert_eq!(signed_in.account, identity.account);
    assert_eq!(
        username, "web-player",
        "the canonical name, though typed Web-Player"
    );
    assert!(expires_at > 0);
    // The page unwraps the master key only to see that the password worked.
    result
        .wrapping_key
        .unwrap(&WrappedMasterKey::from_bytes(&wrapped_master).unwrap())
        .unwrap();
    reply.session.expect("a session for the cookie")
}

fn session(browser: &Browser, http: &Http, bridge: &Token, cookie: &Token) {
    // The cookie is a browser session only: as a native bearer it is nothing.
    assert!(
        http.exchange(&Request::SessionStatus, Some(cookie))
            .is_err()
    );
    // The browser's status names the account, which the native one does not.
    let Response::BrowserStatus {
        username,
        expires_at,
        ..
    } = browser
        .answer(&Request::SessionStatus, Some(cookie))
        .unwrap()
    else {
        panic!("a browser status");
    };
    assert_eq!(username, "web-player");
    assert!(expires_at > 0);
    assert_eq!(
        browser.answer(&Request::SessionStatus, None).err(),
        Some(Error::Rejected)
    );
    // The page renews its session with the cookie, never as a native bearer.
    let Response::Renewed {
        expires_at: renewed,
    } = browser
        .answer(&Request::RenewSession, Some(cookie))
        .unwrap()
    else {
        panic!("a renewal");
    };
    assert!(
        renewed >= expires_at,
        "a renewal never shortens the session"
    );
    assert!(http.exchange(&Request::RenewSession, Some(cookie)).is_err());
    assert_eq!(
        browser.answer(&Request::RenewSession, None).err(),
        Some(Error::Rejected)
    );
    // Operations outside the browser listener's list are refused, with any token.
    for (request, token) in [
        (Request::ReadDocument, cookie),
        (Request::CreateObject, cookie),
        (
            Request::GatewayTicket {
                gateway: GATEWAY.into(),
                purpose: TicketPurpose::Admit,
            },
            cookie,
        ),
        (
            Request::AcceptTerms {
                subject: TermsSubject::Gateway {
                    origin: GATEWAY.into(),
                },
                version: "1".into(),
                digest: [0; 32],
            },
            cookie,
        ),
        (
            Request::LegacyStatus {
                legacy_id: "synthetic-legacy".into(),
            },
            bridge,
        ),
        (
            Request::GatewayHeartbeat {
                metadata: central_api::gateway::GatewayMetadata {
                    name: "Synthetic official gateway".into(),
                    version: "test-1".into(),
                    website: format!("{GATEWAY}/"),
                },
            },
            bridge,
        ),
    ] {
        assert!(browser.refused(&request, Some(token)));
    }
    // Central's terms are read and accepted on the website.
    let Response::TermsStatus {
        current: Some(current),
        accepted: false,
    } = browser.answer(&Request::TermsStatus, Some(cookie)).unwrap()
    else {
        panic!("terms to accept");
    };
    assert!(matches!(
        browser
            .answer(
                &Request::AcceptTerms {
                    subject: TermsSubject::Central,
                    version: current.version.clone(),
                    digest: current.digest,
                },
                Some(cookie),
            )
            .unwrap(),
        Response::Done
    ));
    assert!(matches!(
        browser.answer(&Request::TermsStatus, Some(cookie)).unwrap(),
        Response::TermsStatus { accepted: true, .. }
    ));
}

/// The public directory is read on the browser listener without a session,
/// as on the native route, and lists the same gateways.
fn directory(browser: &Browser, http: &Http) {
    let Response::GatewayDirectory { gateways } = browser
        .answer(&Request::GatewayDirectory { after: None }, None)
        .unwrap()
    else {
        panic!("the directory");
    };
    let native = central_client::gateway_directory(http, None).unwrap();
    assert!(!gateways.is_empty());
    assert_eq!(
        gateways.iter().map(|entry| entry.id).collect::<Vec<_>>(),
        native.iter().map(|entry| entry.id).collect::<Vec<_>>()
    );
}

/// The same account plays natively beside its browser session; signing out of
/// the website ends only the browser session.
fn play(browser: &Browser, http: &Http, bridge: &Token, cookie: &Token) {
    let Discovery::Central { identity } =
        central_client::discover(http, "web-player".into()).unwrap()
    else {
        panic!("a central account");
    };
    let mut session = central_client::login(http, identity, PASSWORD.into()).unwrap();
    assert_eq!(session.document(), DOCUMENT);
    assert!(session.migration_closed(http).unwrap());
    browser
        .answer(&Request::SessionStatus, Some(cookie))
        .unwrap();
    session.confirm_restored(http).unwrap();
    let ticket = session.gateway_ticket(http, GATEWAY.into()).unwrap();
    let Response::GatewayIdentity {
        identity: admitted,
        legacy_id,
        lease,
        ..
    } = http
        .exchange(
            &Request::RedeemGateway {
                ticket,
                purpose: TicketPurpose::Admit,
            },
            Some(bridge),
        )
        .unwrap()
    else {
        panic!("an admission");
    };
    assert_eq!((admitted.account, legacy_id), (identity.account, None));
    assert!(lease.is_some());
    assert!(matches!(
        browser.answer(&Request::Logout, Some(cookie)).unwrap(),
        Response::Done
    ));
    assert!(
        browser
            .answer(&Request::SessionStatus, Some(cookie))
            .is_err()
    );
    assert_eq!(
        browser.answer(&Request::Logout, None).err(),
        Some(Error::Rejected)
    );
    session.check_active(http).unwrap();
    session.logout(http).unwrap();
}

const MOVED: &str = "synthetic moved password";

/// A player moved from the official gateway as Baylee's client moves one
/// (`LegacyReserve` by the bridge, enrollment, restore, `LegacyClose`), then
/// its sign-in on the website, as the page runs it: its context, identity,
/// master key and cookie. The summary says it moved here and has no code.
fn moved_sign_in(
    browser: &Browser,
    http: &Http,
    bridge: &Token,
) -> (Context, Identity, MasterKey, Token) {
    let Response::Enrollment { identity, ticket } = http
        .exchange(
            &Request::LegacyReserve {
                legacy_id: "moved-legacy".into(),
                username: "moved-player".into(),
            },
            Some(bridge),
        )
        .unwrap()
    else {
        panic!("a legacy enrollment");
    };
    central_client::enroll(http, identity, ticket, MOVED.into(), DOCUMENT).unwrap();
    let mut session = central_client::login(http, identity, MOVED.into()).unwrap();
    session.confirm_restored(http).unwrap();
    http.exchange(
        &Request::LegacyClose {
            legacy_id: "moved-legacy".into(),
            identity,
        },
        Some(bridge),
    )
    .unwrap();
    session.logout(http).unwrap();
    let Response::Discovery {
        discovery: Discovery::Central { identity },
    } = browser
        .answer(
            &Request::Discover {
                username: "Moved-Player".into(),
            },
            None,
        )
        .unwrap()
    else {
        panic!("a central account");
    };
    let context = Context::new(http.endpoint.issuer(), identity.account, identity.epoch).unwrap();
    let (login, message) = Login::start(MOVED.into(), context).unwrap();
    let Response::LoginChallenge { attempt, message } = browser
        .answer(&Request::LoginStart { identity, message }, None)
        .unwrap()
    else {
        panic!("a challenge");
    };
    let result = login.finish(&message).unwrap();
    let reply = browser
        .call(
            &Request::LoginFinish {
                attempt,
                message: result.finalization,
            },
            None,
        )
        .unwrap();
    let Response::BrowserSession { wrapped_master, .. } = reply.response else {
        panic!("a browser session");
    };
    let cookie = reply.session.expect("a session for the cookie");
    let Response::AccountSummary {
        identity: summarized,
        username,
        display_name: None,
        tag: None,
        source: AccountSource::Legacy,
        recovery: false,
        admin: false,
        ..
    } = browser
        .answer(&Request::AccountSummary, Some(&cookie))
        .unwrap()
    else {
        panic!("a moved account's summary");
    };
    assert_eq!(summarized.account, identity.account);
    assert_eq!(username, "moved-player");
    let master = result
        .wrapping_key
        .unwrap(&WrappedMasterKey::from_bytes(&wrapped_master).unwrap())
        .unwrap();
    (context, identity, master, cookie)
}

/// A native session of the moved account: the native listener refuses the
/// account page's requests with it, and with the cookie. Returns the session.
fn native_refuses(
    http: &Http,
    context: Context,
    identity: Identity,
    cookie: &Token,
    set: &Request,
) -> Token {
    let (native, message) = Login::start(MOVED.into(), context).unwrap();
    let Response::LoginChallenge { attempt, message } = http
        .exchange(&Request::LoginStart { identity, message }, None)
        .unwrap()
    else {
        panic!("a challenge");
    };
    let finished = native.finish(&message).unwrap();
    let Response::Session { token, .. } = http
        .exchange(
            &Request::LoginFinish {
                attempt,
                message: finished.finalization,
            },
            None,
        )
        .unwrap()
    else {
        panic!("a native session");
    };
    for request in [&Request::AccountSummary, set] {
        for bearer in [&token, cookie] {
            assert_eq!(
                http.exchange(request, Some(bearer)).err(),
                Some(Error::Rejected)
            );
        }
    }
    token
}
