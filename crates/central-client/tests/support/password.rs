//! Password change over the real listeners (`docs/native-login.md`
//! §"Password change"). A sign-in's start spends one of the account's eight
//! OPAQUE evaluations a minute and a change's start two, so each part has an
//! account of its own and counts what it spends: the SDK on a moved account
//! (refused before its move closed, a short or wrong password, a sign-in
//! racing the change), a lost answer, a change whose session was replaced,
//! the protocol's crossed and forged attempts by hand, the budget, and the
//! website's change on a registered account, whose recovery code, document
//! and other sessions are checked.
use super::{Http, browser::Browser};
use central_api::{
    Error, Token,
    beta_key::BetaKey,
    terms::TermsSubject,
    wire::{Identity, Request, Response, TicketPurpose},
};
use central_client::{Session, Transport};
use central_crypto::{
    Context,
    opaque::{Enrollment, Login},
    vault::{MasterKey, ObjectContext, RecoveryCode, WrappedMasterKey},
};
use central_postgres::BetaKeyBatch;
use sea_orm::{ConnectionTrait, DatabaseConnection};
use std::sync::atomic::Ordering;
use tokio::runtime::Runtime;

pub(crate) const OLD: &str = "synthetic first password";
const NEW: &str = "synthetic second password";
const DOCUMENT: &[u8] = br#"{"version":1,"gateways":[],"settings":{"password":true}}"#;

pub(crate) fn run(
    http: &Http,
    browser: &Browser,
    bridge: &Token,
    runtime: &Runtime,
    operator: &DatabaseConnection,
) {
    sdk(http, bridge, runtime, operator);
    lost(http, bridge);
    replaced(http, bridge, runtime, operator);
    crossed(http, bridge, runtime, operator);
    budget(http, bridge, runtime, operator);
    website(http, browser, bridge, runtime, operator);
}

/// One row of the account, as the operator reads it.
fn row(runtime: &Runtime, operator: &DatabaseConnection, sql: &str) -> sea_orm::QueryResult {
    runtime
        .block_on(operator.query_one_raw(sea_orm::Statement::from_string(
            sea_orm::DbBackend::Postgres,
            sql.to_owned(),
        )))
        .unwrap()
        .unwrap()
}

fn revision(runtime: &Runtime, operator: &DatabaseConnection, identity: Identity) -> i64 {
    let account = hex(identity.account);
    row(
        runtime,
        operator,
        &format!(
            "SELECT credential_revision FROM central_identity.accounts WHERE id = '{account}'"
        ),
    )
    .try_get("", "credential_revision")
    .unwrap()
}

fn accept(http: &Http, session: &Session) {
    let current = session.terms_status(http).unwrap().current.unwrap();
    session
        .accept_terms(http, TermsSubject::Central, &current)
        .unwrap();
}

/// A player moved from the official gateway, signed in with [`OLD`] and its
/// document restored; its move is still open. One unit.
pub(crate) fn moved(http: &Http, bridge: &Token, legacy: &str) -> (Identity, Session) {
    let Response::Enrollment { identity, ticket } = http
        .exchange(
            &Request::LegacyReserve {
                legacy_id: legacy.into(),
                username: legacy.into(),
                display_name: None,
                tag: None,
            },
            Some(bridge),
        )
        .unwrap()
    else {
        panic!("an enrollment");
    };
    central_client::enroll(http, identity, ticket, OLD.into(), DOCUMENT).unwrap();
    let mut session = central_client::login(http, identity, OLD.into()).unwrap();
    session.confirm_restored(http).unwrap();
    accept(http, &session);
    (identity, session)
}

pub(crate) fn close(http: &Http, bridge: &Token, legacy: &str, identity: Identity) {
    http.exchange(
        &Request::LegacyClose {
            legacy_id: legacy.into(),
            identity,
        },
        Some(bridge),
    )
    .unwrap();
}

/// A moved account whose move is closed, signed in by hand: its identity,
/// bearer and master key. Two units.
fn closed(http: &Http, bridge: &Token, legacy: &str) -> (Identity, Token, MasterKey) {
    let (identity, session) = moved(http, bridge, legacy);
    close(http, bridge, legacy, identity);
    drop(session);
    let (bearer, master) = sign_in(http, identity, OLD);
    (identity, bearer, master)
}

/// A raw native sign-in: its bearer, and the master key it opens, as a
/// signed-in client holds it. One unit.
fn sign_in(http: &Http, identity: Identity, password: &str) -> (Token, MasterKey) {
    let context = context(http, identity);
    let (login, message) = Login::start(password.into(), context).unwrap();
    let Response::LoginChallenge { attempt, message } = http
        .exchange(&Request::LoginStart { identity, message }, None)
        .unwrap()
    else {
        panic!("a challenge");
    };
    let result = login.finish(&message).unwrap();
    let Response::Session {
        token,
        wrapped_master,
        ..
    } = http
        .exchange(
            &Request::LoginFinish {
                attempt,
                message: result.finalization,
            },
            None,
        )
        .unwrap()
    else {
        panic!("a session");
    };
    let master = result
        .wrapping_key
        .unwrap(&WrappedMasterKey::from_bytes(&wrapped_master).unwrap())
        .unwrap();
    (token, master)
}

/// A sign-in begun with `password` and left unfinished: its attempt and
/// proof. One unit.
fn login_attempt(http: &Http, identity: Identity, password: &str) -> (Token, Vec<u8>) {
    let (login, message) = Login::start(password.into(), context(http, identity)).unwrap();
    let Response::LoginChallenge { attempt, message } = http
        .exchange(&Request::LoginStart { identity, message }, None)
        .unwrap()
    else {
        panic!("a challenge");
    };
    (attempt, login.finish(&message).unwrap().finalization)
}

pub(crate) fn context(http: &Http, identity: Identity) -> Context {
    Context::new(http.endpoint.issuer(), identity.account, identity.epoch).unwrap()
}

/// A change begun by hand: its attempt and the finish's parts, the proof
/// under `current` and the master key wrapped under `new`.
struct Begun {
    attempt: Token,
    proof: Vec<u8>,
    record: Vec<u8>,
    wrapped: Vec<u8>,
}

/// Start a change by hand: two units.
fn begin(
    send: &dyn Fn(&Request) -> Result<Response, Error>,
    context: Context,
    master: &MasterKey,
    current: &str,
    new: &str,
) -> Result<Begun, Error> {
    let (login, message) = Login::start(current.into(), context).unwrap();
    let (enrollment, registration) = Enrollment::start(new.into(), context).unwrap();
    let Response::PasswordChallenge {
        attempt,
        login: challenge,
        registration,
    } = send(&Request::ChangePasswordStart {
        login: message,
        registration,
    })?
    else {
        panic!("a password challenge");
    };
    let proof = login.finish(&challenge).unwrap().finalization;
    let enrolled = enrollment.finish(&registration).unwrap();
    Ok(Begun {
        attempt,
        proof,
        record: enrolled.upload,
        wrapped: enrolled
            .wrapping_key
            .wrap(master)
            .unwrap()
            .as_bytes()
            .to_vec(),
    })
}

impl Begun {
    fn finish(&self) -> Request {
        Request::ChangePasswordFinish {
            attempt: copy(&self.attempt),
            proof: self.proof.clone(),
            record: self.record.clone(),
            wrapped_master: self.wrapped.clone(),
        }
    }
}

/// The SDK's change on a moved account: seven units.
fn sdk(http: &Http, bridge: &Token, runtime: &Runtime, operator: &DatabaseConnection) {
    let legacy = "password-sdk";
    // Unit 1.
    let (identity, mut session) = moved(http, bridge, legacy);
    // While the move is open, the bridge's reset authority stands: refused
    // before any OPAQUE work, so nothing is spent.
    assert_eq!(
        session.change_password(http, OLD.into(), NEW.into()),
        Err(Error::Rejected)
    );
    close(http, bridge, legacy, identity);
    // Too short a new password never leaves the client.
    assert_eq!(
        session.change_password(http, OLD.into(), "seven c".into()),
        Err(Error::Rejected)
    );
    // Units 2 and 3: a wrong current password fails at the client; nothing changed.
    assert_eq!(
        session.change_password(http, "not the password".into(), NEW.into()),
        Err(Error::Rejected)
    );
    session.check_active(http).unwrap();
    let account = hex(identity.account);
    let state = |sessions: bool| {
        row(
            runtime,
            operator,
            &format!(
                "SELECT credential_revision, vault_epoch, document_revision, document,
                 (SELECT count(*) FROM central_identity.sessions WHERE account_id = '{account}') AS sessions
                 FROM central_identity.accounts WHERE id = '{account}'"
            ),
        )
        .try_get::<i64>("", if sessions { "sessions" } else { "credential_revision" })
        .unwrap()
    };
    let before = row(
        runtime,
        operator,
        &format!(
            "SELECT vault_epoch, document_revision, document FROM central_identity.accounts
             WHERE id = '{account}'"
        ),
    );
    let unchanged = state(false);
    // Unit 4: a sign-in with the old password, begun before the change.
    let (racing, proof) = login_attempt(http, identity, OLD);
    // Units 5 and 6: the change.
    session
        .change_password(http, OLD.into(), NEW.into())
        .unwrap();
    session.check_active(http).unwrap();
    session.refresh(http).unwrap();
    assert_eq!(session.document(), DOCUMENT);
    // The racing sign-in proved the old credential, which is gone.
    assert_eq!(
        http.exchange(
            &Request::LoginFinish {
                attempt: racing,
                message: proof,
            },
            None,
        )
        .err(),
        Some(Error::Rejected)
    );
    assert_eq!((state(false), state(true)), (unchanged + 1, 1));
    let after = row(
        runtime,
        operator,
        &format!(
            "SELECT vault_epoch, document_revision, document FROM central_identity.accounts
             WHERE id = '{account}'"
        ),
    );
    for column in ["vault_epoch", "document_revision"] {
        assert_eq!(
            after.try_get::<i64>("", column).unwrap(),
            before.try_get::<i64>("", column).unwrap(),
            "{column}"
        );
    }
    assert_eq!(
        after.try_get::<Vec<u8>>("", "document").unwrap(),
        before.try_get::<Vec<u8>>("", "document").unwrap()
    );
    // Unit 7: the old password is refused, at the client already.
    assert_eq!(
        central_client::login(http, identity, OLD.into()).err(),
        Some(Error::Rejected)
    );
    session.logout(http).unwrap();
}

/// A lost answer after the commit: the session it ended says so. Five units.
fn lost(http: &Http, bridge: &Token) {
    let legacy = "password-lost";
    // Unit 1.
    let (identity, mut session) = moved(http, bridge, legacy);
    close(http, bridge, legacy, identity);
    http.lose_password_reply.store(true, Ordering::Release);
    // Units 2 and 3.
    assert_eq!(
        session.change_password(http, OLD.into(), NEW.into()),
        Err(Error::Rejected)
    );
    assert_eq!(session.check_active(http), Err(Error::Rejected));
    // Unit 4: the new password is the account's; its key opens the document.
    let session = central_client::login(http, identity, NEW.into()).unwrap();
    assert_eq!(session.document(), DOCUMENT);
    // Unit 5: and the old one is refused.
    assert_eq!(
        central_client::login(http, identity, OLD.into()).err(),
        Some(Error::Rejected)
    );
    session.logout(http).unwrap();
}

/// A change begun, then its session replaced by a new sign-in: the
/// replacement cannot finish what the replaced one began. Five units.
fn replaced(http: &Http, bridge: &Token, runtime: &Runtime, operator: &DatabaseConnection) {
    // Units 1 and 2.
    let (identity, bearer, master) = closed(http, bridge, "password-replaced");
    let unchanged = revision(runtime, operator, identity);
    let context = context(http, identity);
    // Units 3 and 4.
    let begun = begin(
        &|request: &Request| http.exchange(request, Some(&bearer)),
        context,
        &master,
        OLD,
        NEW,
    )
    .unwrap();
    // Unit 5.
    let (replacement, _) = sign_in(http, identity, OLD);
    assert_eq!(
        http.exchange(&begun.finish(), Some(&replacement)).err(),
        Some(Error::Rejected)
    );
    assert_eq!(
        http.exchange(&begun.finish(), Some(&bearer)).err(),
        Some(Error::Rejected)
    );
    assert_eq!(revision(runtime, operator, identity), unchanged);
    http.exchange(&Request::Logout, Some(&replacement)).unwrap();
}

/// Attempts refused by hand, each consumed by the refusal and changing
/// nothing: no session, an unknown bearer, another bearer, a change's
/// attempt as a sign-in's and the reverse. Seven units.
fn crossed(http: &Http, bridge: &Token, runtime: &Runtime, operator: &DatabaseConnection) {
    // Units 1 and 2.
    let (identity, bearer, master) = closed(http, bridge, "password-crossed");
    let context = context(http, identity);
    let send = |request: &Request| http.exchange(request, Some(&bearer));
    let unchanged = revision(runtime, operator, identity);
    // Without a session, or with an unknown one, nothing starts or is spent.
    let start = Request::ChangePasswordStart {
        login: Login::start(OLD.into(), context).unwrap().1,
        registration: Enrollment::start(NEW.into(), context).unwrap().1,
    };
    assert_eq!(http.exchange(&start, None).err(), Some(Error::Rejected));
    assert_eq!(
        http.exchange(&start, Some(&Token::generate().unwrap()))
            .err(),
        Some(Error::Rejected)
    );
    // Units 3 and 4: finished with another bearer, then (consumed) with its own and none.
    let begun = begin(&send, context, &master, OLD, NEW).unwrap();
    assert_eq!(
        http.exchange(&begun.finish(), Some(&Token::generate().unwrap()))
            .err(),
        Some(Error::Rejected)
    );
    assert_eq!(send(&begun.finish()).err(), Some(Error::Rejected));
    assert_eq!(
        http.exchange(&begun.finish(), None).err(),
        Some(Error::Rejected)
    );
    // Units 5 and 6: a change's attempt is no sign-in, and is consumed by trying.
    let begun = begin(&send, context, &master, OLD, NEW).unwrap();
    assert_eq!(
        http.exchange(
            &Request::LoginFinish {
                attempt: copy(&begun.attempt),
                message: begun.proof.clone(),
            },
            None,
        )
        .err(),
        Some(Error::Rejected)
    );
    assert_eq!(send(&begun.finish()).err(), Some(Error::Rejected));
    // Unit 7: nor is a sign-in's attempt a change's, with a well-formed upload.
    let (attempt, proof) = login_attempt(http, identity, OLD);
    assert_eq!(
        send(&Request::ChangePasswordFinish {
            attempt,
            proof,
            record: begun.record.clone(),
            wrapped_master: begun.wrapped.clone(),
        })
        .err(),
        Some(Error::Rejected)
    );
    assert_eq!(revision(runtime, operator, identity), unchanged);
    send(&Request::Logout).unwrap();
}

/// A forged proof and a short wrapper beside a good proof, refused and
/// consumed; then the budget, where a change's start costs two sign-ins.
fn budget(http: &Http, bridge: &Token, runtime: &Runtime, operator: &DatabaseConnection) {
    // Units 1 and 2.
    let (identity, bearer, master) = closed(http, bridge, "password-budget");
    let context = context(http, identity);
    let send = |request: &Request| http.exchange(request, Some(&bearer));
    let unchanged = revision(runtime, operator, identity);
    // Units 3 and 4: a proof with one bit changed.
    let begun = begin(&send, context, &master, OLD, NEW).unwrap();
    let mut forged = begun.finish();
    if let Request::ChangePasswordFinish { proof, .. } = &mut forged {
        proof[0] ^= 1;
    }
    assert_eq!(send(&forged).err(), Some(Error::Rejected));
    assert_eq!(send(&begun.finish()).err(), Some(Error::Rejected));
    // Units 5 and 6: a wrapper one byte short, with a good proof and record.
    let begun = begin(&send, context, &master, OLD, NEW).unwrap();
    let mut short = begun.finish();
    if let Request::ChangePasswordFinish { wrapped_master, .. } = &mut short {
        wrapped_master.pop();
    }
    assert_eq!(send(&short).err(), Some(Error::Rejected));
    assert_eq!(send(&begun.finish()).err(), Some(Error::Rejected));
    // Unit 7: a sign-in's start still fits; a change's two do not, and spend nothing.
    let _ = login_attempt(http, identity, OLD);
    assert_eq!(
        begin(&send, context, &master, OLD, NEW).err(),
        Some(Error::Busy)
    );
    // Unit 8, and then the account's next sign-in is `Busy` too.
    let _ = login_attempt(http, identity, OLD);
    assert_eq!(
        http.exchange(
            &Request::LoginStart {
                identity,
                message: Login::start(OLD.into(), context).unwrap().1,
            },
            None,
        )
        .err(),
        Some(Error::Busy)
    );
    // Nothing changed: the same credential, and the bearer still lives.
    assert_eq!(revision(runtime, operator, identity), unchanged);
    assert!(matches!(
        send(&Request::SessionStatus).unwrap(),
        Response::SessionStatus { .. }
    ));
    send(&Request::Logout).unwrap();
}

fn hex(bytes: [u8; 16]) -> String {
    sea_orm::prelude::Uuid::from_bytes(bytes).to_string()
}

fn copy(token: &Token) -> Token {
    Token::from_secret_bytes(token.to_secret_bytes())
}

/// The website's sign-in, as the page runs it: the cookie's session.
pub(crate) fn browser_sign_in(
    browser: &Browser,
    identity: Identity,
    context: Context,
    password: &str,
) -> Token {
    let (login, message) = Login::start(password.into(), context).unwrap();
    let Response::LoginChallenge { attempt, message } = browser
        .answer(&Request::LoginStart { identity, message }, None)
        .unwrap()
    else {
        panic!("a challenge");
    };
    let result = login.finish(&message).unwrap();
    browser
        .call(
            &Request::LoginFinish {
                attempt,
                message: result.finalization,
            },
            None,
        )
        .unwrap()
        .session
        .expect("a session for the cookie")
}

/// A registered account, as the website's registration leaves it.
struct Registered {
    identity: Identity,
    context: Context,
    master: MasterKey,
    code: RecoveryCode,
    recovery: WrappedMasterKey,
    document: Vec<u8>,
}

/// Register `password-web` with a beta key of its own, enrolled under [`OLD`].
fn register(
    http: &Http,
    browser: &Browser,
    runtime: &Runtime,
    operator: &DatabaseConnection,
) -> Registered {
    let key = BetaKey::generate().unwrap();
    runtime
        .block_on(
            BetaKeyBatch::new(&[key.digest()], 1, None, "password fixture")
                .unwrap()
                .issue(operator),
        )
        .unwrap();
    let Response::Enrollment { identity, ticket } = browser
        .answer(
            &Request::Register {
                username: "password-web".into(),
                beta_key: key.display().as_str().into(),
                display_name: Some("Pass_Web".into()),
            },
            None,
        )
        .unwrap()
    else {
        panic!("an enrollment");
    };
    let context = context(http, identity);
    let (enrollment, message) = Enrollment::start(OLD.into(), context).unwrap();
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
    let code = RecoveryCode::generate().unwrap();
    let recovery = code.wrap(context, &master).unwrap();
    let document = master
        .seal_object(
            ObjectContext::new(context, identity.account, 1).unwrap(),
            DOCUMENT,
        )
        .unwrap();
    assert!(matches!(
        browser
            .answer(
                &Request::EnrollmentFinish {
                    ticket,
                    record: result.upload,
                    wrapped_master: result
                        .wrapping_key
                        .wrap(&master)
                        .unwrap()
                        .as_bytes()
                        .to_vec(),
                    document: document.clone(),
                    recovery: Some(recovery.as_bytes().to_vec()),
                },
                None,
            )
            .unwrap(),
        Response::Done
    ));
    Registered {
        identity,
        context,
        master,
        code,
        recovery,
        document,
    }
}

/// Unit 2: the native client plays; the official gateway holds a lease,
/// whose check is answered.
fn playing(http: &Http, bridge: &Token, identity: Identity) -> (Session, Request) {
    let mut native = central_client::login(http, identity, OLD.into()).unwrap();
    native.confirm_restored(http).unwrap();
    let ticket = native
        .gateway_ticket(http, "http://127.0.0.1:28766".into())
        .unwrap();
    let Response::GatewayIdentity {
        lease: Some(lease), ..
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
        panic!("an admission with a lease");
    };
    let lease = Request::GatewaySession {
        lease,
        purpose: TicketPurpose::Admit,
    };
    assert!(matches!(
        http.exchange(&lease, Some(bridge)).unwrap(),
        Response::GatewaySession { .. }
    ));
    (native, lease)
}

/// The website's change on a registered account, while the native client
/// plays at the official gateway: the cookie and the native session end,
/// with the gateway's lease; a new cookie is set; the recovery code and the
/// document stay. Eight units.
fn website(
    http: &Http,
    browser: &Browser,
    bridge: &Token,
    runtime: &Runtime,
    operator: &DatabaseConnection,
) {
    // An address of its own: registration is budgeted per address.
    let browser = Browser {
        address: browser.address,
        client: "198.51.100.40",
    };
    let registered = register(http, &browser, runtime, operator);
    let (identity, context, master) = (registered.identity, registered.context, &registered.master);
    // Unit 1: the website's sign-in, and the terms.
    let cookie = browser_sign_in(&browser, identity, context, OLD);
    // Before the terms, no change starts, and nothing is spent.
    let over_browser = |request: &Request| browser.answer(request, Some(&cookie));
    assert_eq!(
        begin(&over_browser, context, master, OLD, NEW).err(),
        Some(Error::TermsRequired)
    );
    super::browser::accept_terms(&browser, &cookie);
    let (native, lease) = playing(http, bridge, identity);
    // The cookie is no native bearer: nothing starts, nothing is spent.
    let over_native = |request: &Request| http.exchange(request, Some(&cookie));
    assert_eq!(
        begin(&over_native, context, master, OLD, NEW).err(),
        Some(Error::Rejected)
    );
    // Units 3 and 4: begun on the website, finished on the native listener with
    // the same token: refused, and consumed.
    let begun = begin(&over_browser, context, master, OLD, NEW).unwrap();
    assert_eq!(
        http.exchange(&begun.finish(), Some(&cookie)).err(),
        Some(Error::Rejected)
    );
    assert_eq!(
        browser.call(&begun.finish(), Some(&cookie)).err(),
        Some(Error::Rejected)
    );
    // Units 5 and 6: the change; its answer is a browser session in a new cookie.
    let begun = begin(&over_browser, context, master, OLD, NEW).unwrap();
    let reply = browser.call(&begun.finish(), Some(&cookie)).unwrap();
    let Response::BrowserSession {
        identity: changed,
        username,
        wrapped_master,
        expires_at,
    } = reply.response
    else {
        panic!("a browser session");
    };
    assert_eq!(
        (changed.account, changed.epoch),
        (identity.account, identity.epoch)
    );
    assert_eq!(username, "password-web");
    assert_eq!(wrapped_master, begun.wrapped);
    assert!(expires_at > 0);
    let renewed = reply.session.expect("a new cookie");
    assert_eq!(
        browser.answer(&Request::SessionStatus, Some(&cookie)).err(),
        Some(Error::Rejected)
    );
    let Response::AccountSummary {
        recovery: has_code, ..
    } = browser
        .answer(&Request::AccountSummary, Some(&renewed))
        .unwrap()
    else {
        panic!("a summary");
    };
    assert!(has_code);
    // The native session ended with it, and the gateway's lease.
    assert_eq!(native.check_active(http), Err(Error::Rejected));
    assert_eq!(
        http.exchange(&lease, Some(bridge)).err(),
        Some(Error::Rejected)
    );
    afterwards(http, &browser, runtime, operator, &registered, &renewed);
}

/// After the website's change: the recovery code, the document and the new
/// password; units 7 and 8.
fn afterwards(
    http: &Http,
    browser: &Browser,
    runtime: &Runtime,
    operator: &DatabaseConnection,
    registered: &Registered,
    renewed: &Token,
) {
    let (identity, context) = (registered.identity, registered.context);
    // The recovery wrapper is the registration's, and still opens the document.
    let stored = row(
        runtime,
        operator,
        "SELECT recovery_wrapped_master, vault_epoch FROM central_identity.accounts
         WHERE username_key = 'password-web'",
    );
    let stored_recovery: Vec<u8> = stored.try_get("", "recovery_wrapped_master").unwrap();
    assert_eq!(stored_recovery, registered.recovery.as_bytes());
    assert_eq!(stored.try_get::<i64>("", "vault_epoch").unwrap(), 1);
    let opened = registered
        .code
        .unwrap(
            context,
            &WrappedMasterKey::from_bytes(&stored_recovery).unwrap(),
        )
        .unwrap();
    assert_eq!(
        &opened
            .open_object(
                ObjectContext::new(context, identity.account, 1).unwrap(),
                &registered.document
            )
            .unwrap()[..],
        DOCUMENT
    );
    // Unit 7: the new password signs the client in, and its key opens the
    // document; a native sign-in leaves the website's session alone.
    let native = central_client::login(http, identity, NEW.into()).unwrap();
    assert_eq!(native.document(), DOCUMENT);
    assert!(matches!(
        browser
            .answer(&Request::SessionStatus, Some(renewed))
            .unwrap(),
        Response::BrowserStatus { .. }
    ));
    // Unit 8: the old password is refused.
    assert_eq!(
        central_client::login(http, identity, OLD.into()).err(),
        Some(Error::Rejected)
    );
    native.logout(http).unwrap();
    browser.answer(&Request::Logout, Some(renewed)).unwrap();
}
