//! Deleting and exporting an account over the real listeners
//! (`docs/account-deletion.md`). A deletion's start spends one of the
//! account's eight OPAQUE evaluations a minute, so each part has an account
//! of its own: the SDK on a moved account that owns a gateway and an object,
//! an account that has not accepted Central's terms, a lost answer, crossed
//! attempts by hand, an administrator, the budget, and the website's export
//! and deletion of a registered account.
use super::{
    Http,
    browser::Browser,
    password::{OLD, browser_sign_in, close, context, moved, register},
};
use central_api::{
    Error, Token,
    export::SessionKind,
    gateway::GatewayMetadata,
    wire::{AccountSource, Identity, Request, Response},
};
use central_client::{Session, Transport};
use central_crypto::opaque::Login;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, prelude::Uuid};
use std::sync::atomic::Ordering;
use tokio::runtime::Runtime;

pub(crate) fn run(
    http: &Http,
    browser: &Browser,
    bridge: &Token,
    runtime: &Runtime,
    operator: &DatabaseConnection,
) {
    // An address of its own: registration has a per-address budget.
    let browser = &Browser {
        address: browser.address,
        client: "198.51.100.43",
    };
    sdk(http, bridge, runtime, operator);
    unaccepted(http, bridge);
    lost(http, bridge);
    crossed(http, browser, bridge, runtime, operator);
    replaced(http, bridge);
    budget(http, bridge);
    website(http, browser, runtime, operator);
}

/// A moved account, its move closed, with the session of its sign-in. One
/// unit. Each sign-in also costs an Argon2 run on the client, so the tests
/// sign in no more than they must.
fn closed(http: &Http, bridge: &Token, legacy: &str) -> (Identity, Session) {
    let (identity, session) = moved(http, bridge, legacy);
    close(http, bridge, legacy, identity);
    (identity, session)
}

/// The SDK's export and deletion: refused while the move is open and for a
/// wrong password; the export names the gateway and the object, which comes
/// back decrypted; the deletion warns of the gateway and takes it.
fn sdk(http: &Http, bridge: &Token, runtime: &Runtime, operator: &DatabaseConnection) {
    let (identity, session) = moved(http, bridge, "delete-sdk");
    assert_eq!(
        session.start_deletion(http, OLD.into()).err(),
        Some(Error::Rejected),
        "an open move"
    );
    close(http, bridge, "delete-sdk", identity);
    let origin = "http://127.0.0.1:28768";
    let gateway = session
        .register_gateway(
            http,
            origin.into(),
            GatewayMetadata {
                name: "Deleted gateway".into(),
                version: "test-1".into(),
                website: format!("{origin}/status"),
            },
        )
        .unwrap()
        .gateway;
    runtime
        .block_on(operator.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO central_identity.gateway_approvals(gateway_id) VALUES ($1)",
            vec![Uuid::from_bytes(gateway).into()],
        )))
        .unwrap();
    let object = session.create_object(http).unwrap();
    session
        .write_object(http, object.id, 0, b"synthetic deck")
        .unwrap();
    let export = session.export(http).unwrap();
    assert_eq!(
        (export.account, export.username.as_str(), export.source),
        (identity.account, "delete-sdk", AccountSource::Legacy)
    );
    assert_eq!(export.legacy_id.as_deref(), Some("delete-sdk"));
    assert_eq!(export.objects, 1);
    assert_eq!(export.sessions.len(), 1);
    assert_eq!(export.sessions[0].kind, SessionKind::Native);
    assert_eq!(export.gateways.len(), 1);
    assert_eq!(
        (export.gateways[0].id, export.gateways[0].origin.as_str()),
        (gateway, origin)
    );
    assert!(export.terms.iter().any(|t| t.subject == "central"));
    assert!(export.complete);
    let mut plaintexts = Vec::new();
    assert_eq!(
        session
            .export_objects(http, |object| {
                plaintexts.push((object.id, object.plaintext.to_vec()));
                Ok(())
            })
            .unwrap(),
        1
    );
    assert_eq!(plaintexts, vec![(object.id, b"synthetic deck".to_vec())]);
    // A wrong password deletes nothing, and the session lives on.
    assert_eq!(
        session
            .start_deletion(http, "synthetic wrong password".into())
            .err(),
        Some(Error::Rejected)
    );
    session.check_active(http).unwrap();
    let deletion = session.start_deletion(http, OLD.into()).unwrap();
    assert_eq!(deletion.gateways().len(), 1);
    assert_eq!(
        (
            deletion.gateways()[0].id,
            deletion.gateways()[0].origin.as_str()
        ),
        (gateway, origin)
    );
    session.delete_account(http, deletion).unwrap();
    assert_eq!(session.check_active(http), Err(Error::Rejected));
    assert_eq!(
        central_client::login(http, identity, OLD.into()).err(),
        Some(Error::Rejected)
    );
    assert!(
        central_client::gateway_directory(http, None)
            .unwrap()
            .iter()
            .all(|entry| entry.id != gateway)
    );
}

/// A person who has not accepted Central's terms in force may still see
/// their data and leave.
fn unaccepted(http: &Http, bridge: &Token) {
    let Response::Enrollment { identity, ticket } = http
        .exchange(
            &Request::LegacyReserve {
                legacy_id: "delete-terms".into(),
                username: "delete-terms".into(),
                display_name: None,
                tag: None,
            },
            Some(bridge),
        )
        .unwrap()
    else {
        panic!("an enrollment");
    };
    central_client::enroll(
        http,
        identity,
        ticket,
        OLD.into(),
        super::password::DOCUMENT,
    )
    .unwrap();
    let mut session = central_client::login(http, identity, OLD.into()).unwrap();
    session.confirm_restored(http).unwrap();
    close(http, bridge, "delete-terms", identity);
    // The terms are in force: an object is refused.
    assert_eq!(
        session.create_object(http).err(),
        Some(Error::TermsRequired)
    );
    let export = session.export(http).unwrap();
    assert!(export.terms.is_empty());
    let deletion = session.start_deletion(http, OLD.into()).unwrap();
    session.delete_account(http, deletion).unwrap();
    assert_eq!(
        central_client::login(http, identity, OLD.into()).err(),
        Some(Error::Rejected)
    );
}

/// The answer to a committed deletion is lost: the SDK finds its session
/// ended and says `Rejected`; the account is gone.
fn lost(http: &Http, bridge: &Token) {
    let (identity, session) = closed(http, bridge, "delete-lost");
    let deletion = session.start_deletion(http, OLD.into()).unwrap();
    http.lose_delete_reply.store(true, Ordering::Release);
    assert_eq!(
        session.delete_account(http, deletion).err(),
        Some(Error::Rejected)
    );
    assert!(!http.lose_delete_reply.load(Ordering::Acquire));
    assert_eq!(
        central_client::login(http, identity, OLD.into()).err(),
        Some(Error::Rejected)
    );
}

/// A deletion begun by hand on one listener: its attempt and proof.
fn begin(
    http: &Http,
    identity: Identity,
    send: impl Fn(&Request) -> Result<Response, Error>,
) -> (Token, Vec<u8>) {
    let (login, message) = Login::start(OLD.into(), context(http, identity)).unwrap();
    let Response::DeleteChallenge {
        attempt,
        login: ke2,
        ..
    } = send(&Request::DeleteAccountStart { login: message }).unwrap()
    else {
        panic!("a deletion challenge");
    };
    (attempt, login.finish(&ke2).unwrap().finalization)
}

/// An attempt finishes only on its listener and with its session; a refused
/// finish consumes it; a sign-in cannot finish it, nor a deletion a sign-in.
/// Then the account becomes an administrator, which cannot delete itself.
fn crossed(
    http: &Http,
    browser: &Browser,
    bridge: &Token,
    runtime: &Runtime,
    operator: &DatabaseConnection,
) {
    let (identity, session) = closed(http, bridge, "delete-crossed");
    let bearer = session_token(http, identity);
    let cookie = browser_sign_in(browser, identity, context(http, identity), OLD);
    let native = |request: &Request| http.exchange(request, Some(&bearer));
    let website = |request: &Request| browser.answer(request, Some(&cookie));
    let finish =
        |(attempt, proof): (Token, Vec<u8>)| Request::DeleteAccountFinish { attempt, proof };
    // Begun natively, finished on the website: refused and consumed.
    let attempt = finish(begin(http, identity, native));
    assert_eq!(website(&attempt).err(), Some(Error::Rejected));
    assert_eq!(native(&attempt).err(), Some(Error::Rejected));
    // Begun on the website, finished natively.
    let attempt = finish(begin(http, identity, website));
    assert_eq!(native(&attempt).err(), Some(Error::Rejected));
    // A deletion's attempt finishes no sign-in.
    let (attempt, proof) = begin(http, identity, native);
    assert_eq!(
        http.exchange(
            &Request::LoginFinish {
                attempt,
                message: proof
            },
            None
        )
        .err(),
        Some(Error::Rejected)
    );
    // Nor a sign-in's attempt a deletion.
    let (login, message) = Login::start(OLD.into(), context(http, identity)).unwrap();
    let Response::LoginChallenge { attempt, message } = http
        .exchange(&Request::LoginStart { identity, message }, None)
        .unwrap()
    else {
        panic!("a challenge");
    };
    let proof = login.finish(&message).unwrap().finalization;
    assert_eq!(
        native(&Request::DeleteAccountFinish { attempt, proof }).err(),
        Some(Error::Rejected)
    );
    // Without a session nothing is begun.
    assert_eq!(
        http.exchange(&Request::DeleteAccountStart { login: vec![1] }, None)
            .err(),
        Some(Error::Rejected)
    );
    // Nothing was deleted: both sessions live.
    assert!(matches!(
        native(&Request::SessionStatus).unwrap(),
        Response::SessionStatus { .. }
    ));
    website(&Request::SessionStatus).unwrap();
    // An administrator is refused before anything is spent; its export says so.
    runtime
        .block_on(central_postgres::appoint_admin(operator, "delete-crossed"))
        .unwrap();
    let (_, message) = Login::start(OLD.into(), context(http, identity)).unwrap();
    assert_eq!(
        native(&Request::DeleteAccountStart { login: message }).err(),
        Some(Error::Rejected)
    );
    let Response::AccountExport { export } = native(&Request::AccountExport).unwrap() else {
        panic!("an export");
    };
    assert!(export.admin);
    drop(session);
}

/// An attempt begun with one session is refused with the next one of the
/// same account and kind, which replaced it.
fn replaced(http: &Http, bridge: &Token) {
    let (identity, first) = closed(http, bridge, "delete-replaced");
    let deletion = first.start_deletion(http, OLD.into()).unwrap();
    let next = central_client::login(http, identity, OLD.into()).unwrap();
    assert_eq!(
        next.delete_account(http, deletion).err(),
        Some(Error::Rejected)
    );
    next.check_active(http).unwrap();
}

/// A fresh native sign-in by hand, for raw requests. One unit.
fn session_token(http: &Http, identity: Identity) -> Token {
    let (login, message) = Login::start(OLD.into(), context(http, identity)).unwrap();
    let Response::LoginChallenge { attempt, message } = http
        .exchange(&Request::LoginStart { identity, message }, None)
        .unwrap()
    else {
        panic!("a challenge");
    };
    let Response::Session { token, .. } = http
        .exchange(
            &Request::LoginFinish {
                attempt,
                message: login.finish(&message).unwrap().finalization,
            },
            None,
        )
        .unwrap()
    else {
        panic!("a session");
    };
    token
}

/// A start is one unit of the account's budget, whether or not a proof
/// follows.
fn budget(http: &Http, bridge: &Token) {
    // Two units: the sign-in of the move and this one.
    let (identity, session) = closed(http, bridge, "delete-budget");
    let bearer = session_token(http, identity);
    let start = || {
        let (_, message) = Login::start(OLD.into(), context(http, identity)).unwrap();
        http.exchange(
            &Request::DeleteAccountStart { login: message },
            Some(&bearer),
        )
    };
    for _ in 0..6 {
        assert!(matches!(start().unwrap(), Response::DeleteChallenge { .. }));
    }
    assert_eq!(start().err(), Some(Error::Busy));
    assert!(matches!(
        http.exchange(&Request::SessionStatus, Some(&bearer))
            .unwrap(),
        Response::SessionStatus { .. }
    ));
    drop(session);
}

/// The website exports and deletes a registered account; the deletion
/// clears the cookie (checked by [`Browser::call`]).
fn website(http: &Http, browser: &Browser, runtime: &Runtime, operator: &DatabaseConnection) {
    let registered = register("delete-web", http, browser, runtime, operator);
    let identity = registered.identity;
    let cookie = browser_sign_in(browser, identity, registered.context, OLD);
    let Response::AccountExport { export } = browser
        .answer(&Request::AccountExport, Some(&cookie))
        .unwrap()
    else {
        panic!("an export");
    };
    assert_eq!(
        (export.username.as_str(), export.source, export.recovery),
        ("delete-web", AccountSource::Registered, true)
    );
    assert_eq!(export.sessions.len(), 1);
    assert_eq!(export.sessions[0].kind, SessionKind::Browser);
    let (attempt, proof) = begin(http, identity, |request| {
        browser.answer(request, Some(&cookie))
    });
    let reply = browser
        .call(
            &Request::DeleteAccountFinish { attempt, proof },
            Some(&cookie),
        )
        .unwrap();
    assert!(matches!(reply.response, Response::Done));
    assert!(reply.session.is_none());
    assert_eq!(
        browser.answer(&Request::SessionStatus, Some(&cookie)).err(),
        Some(Error::Rejected)
    );
    assert_eq!(
        central_client::login(http, identity, OLD.into()).err(),
        Some(Error::Rejected)
    );
}
