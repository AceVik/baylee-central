//! The legacy store's sign-in and move over real HTTP (`docs/native-login.md`
//! §"Legacy login"), on a second listener whose service holds the store: the
//! good path from discovery to the closed move and the gateway's admission;
//! and the attacks: a wrong password, the per-name budget and the lockout, a
//! replayed enrollment ticket, a move that closed already, a Central-only
//! account on the legacy path, the bridge's own legacy operations, and the
//! bridge's admission of a move that has not closed.
use super::{Http, browser::Browser, config, password, terms};
use central_api::{
    Endpoint, Error, Token,
    admin::{AdminAnswer, AdminRequest},
    legacy::LegacyProgress,
    terms::TermsSubject,
    wire::{Discovery, Request, Response, TicketPurpose},
};
use central_client::{Session, Transport};
use central_crypto::opaque::{Login, server::ServerKeys};
use central_http::DevelopmentServer;
use central_identity::IdentityService;
use central_postgres::{
    AccountStore, Appointed, LegacyRow, LegacyStore, appoint_admin, import_legacy, legacy_digest,
};
use sea_orm::{ConnectionTrait, DatabaseConnection};
use std::{
    net::TcpStream,
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};
use tokio::{runtime::Runtime, sync::oneshot};

/// The official gateway's own Argon2id output for this synthetic password
/// (argon2 0.6's defaults, as the gateway hashed): what the import carries.
const PASSWORD: &str = "synthetic legacy password";
const SALT: [u8; 16] = [
    194, 43, 109, 59, 0, 114, 233, 156, 106, 255, 45, 3, 26, 54, 90, 176,
];
const HASH: [u8; 32] = [
    124, 25, 84, 73, 172, 183, 102, 66, 91, 229, 176, 233, 179, 208, 221, 25, 117, 160, 174, 33,
    215, 245, 200, 205, 98, 122, 120, 101, 123, 60, 210, 10,
];
const GATEWAY: &str = "http://127.0.0.1:28766";
const SNAPSHOT: &[u8] =
    br#"{"preferences":{"synthetic":true},"decks":[{"id":"deck","history":[]}]}"#;
const DOCUMENT: &[u8] = br#"{"version":1,"gateways":[],"settings":{"legacy":true}}"#;

fn row(name: &str) -> LegacyRow {
    LegacyRow {
        legacy_id: format!("{name}-id"),
        username: name.into(),
        display_name: Some("Moved".into()),
        tag: None,
        salt: SALT,
        hash: HASH,
        terms_version: None,
        must_change_password: false,
        ban: None,
        snapshot: SNAPSHOT.to_vec(),
    }
}

fn copy(token: &Token) -> Token {
    Token::from_secret_bytes(token.to_secret_bytes())
}

fn count(runtime: &Runtime, operator: &DatabaseConnection, sql: &str) -> i64 {
    runtime
        .block_on(operator.query_one_raw(sea_orm::Statement::from_string(
            sea_orm::DbBackend::Postgres,
            sql,
        )))
        .unwrap()
        .unwrap()
        .try_get("", "count")
        .unwrap()
}

fn accept(http: &Http, session: &Session) {
    let current = session.terms_status(http).unwrap().current.unwrap();
    session
        .accept_terms(http, TermsSubject::Central, &current)
        .unwrap();
}

/// The bridge's admission of the session's account at the official gateway:
/// the legacy id it learns, or the refusal.
fn admission(http: &Http, session: &Session, bridge: &Token) -> Result<Option<String>, Error> {
    let ticket = session.gateway_ticket(http, GATEWAY.into())?;
    match http.exchange(
        &Request::RedeemGateway {
            ticket,
            purpose: TicketPurpose::Admit,
        },
        Some(bridge),
    )? {
        Response::GatewayIdentity { legacy_id, .. } => Ok(legacy_id),
        _ => Err(Error::Rejected),
    }
}

/// Signed in and restored through the legacy store, terms accepted; the move
/// still open.
fn moved(http: &Http, name: &str) -> Session {
    let enrollment = central_client::legacy_login(http, name.into(), PASSWORD).unwrap();
    central_client::enroll(
        http,
        enrollment.identity,
        enrollment.ticket,
        PASSWORD.into(),
        DOCUMENT,
    )
    .unwrap();
    let mut session = central_client::login(http, enrollment.identity, PASSWORD.into()).unwrap();
    session.confirm_restored(http).unwrap();
    accept(http, &session);
    session
}

pub(crate) fn run(
    original: &Http,
    browser: &Browser,
    original_bridge: &Token,
    store: &AccountStore,
    runtime: &Runtime,
    operator: &DatabaseConnection,
) {
    // The listing's administrator and a registered account, made on the
    // service without the store, which still serves the bridge; from an
    // address of their own, for the registration budget.
    let browser = &Browser {
        address: browser.address,
        client: "198.51.100.61",
    };
    let central_only = password::register("legacy-central", original, browser, runtime, operator);
    let mut rows: Vec<LegacyRow> = ["mover", "early", "limited", "locked", "banned", "forced"]
        .into_iter()
        .map(row)
        .collect();
    rows[4].ban = Some(None);
    rows[5].must_change_password = true;
    rows.sort_by(|a, b| a.legacy_id.as_bytes().cmp(b.legacy_id.as_bytes()));
    let digest = legacy_digest(&rows);
    runtime
        .block_on(import_legacy(operator, rows, 6, digest))
        .unwrap();
    let legacy = runtime
        .block_on(LegacyStore::connect_dev(config(
            "central_legacy",
            "CENTRAL_TEST_LEGACY_PASSWORD",
        )))
        .unwrap();
    let server = runtime
        .block_on(DevelopmentServer::bind(crate::central_policy_config()))
        .unwrap();
    let address = server.local_addr();
    let endpoint = Endpoint::development(&format!("http://{address}")).unwrap();
    let bridge = Token::generate().unwrap();
    let service = Arc::new(
        IdentityService::new(
            store.clone(),
            &endpoint,
            ServerKeys::generate(),
            &bridge,
            GATEWAY,
        )
        .unwrap()
        .with_terms(Some(terms::central_document()))
        .with_legacy(legacy.clone()),
    );
    let (stop, stopped) = oneshot::channel();
    let serving = endpoint.clone();
    let handler = Arc::clone(&service);
    let task = runtime.spawn(async move {
        server
            .run_with_identity(&serving, handler, async {
                let _ = stopped.await;
            })
            .await
    });
    let http = Http {
        endpoint,
        address,
        lose_write_reply: AtomicBool::new(false),
        lose_password_reply: AtomicBool::new(false),
        lose_delete_reply: AtomicBool::new(false),
    };
    let tested = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        good_path(&http, &bridge, runtime, operator);
        open_move_is_not_admitted(&http, &bridge);
        budget_and_lockout(&http, runtime, operator);
        refusals(&http, original, &central_only);
        listing(
            &service,
            original,
            browser,
            original_bridge,
            runtime,
            operator,
        );
    }));
    let _ = stop.send(());
    runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        legacy.close().await.unwrap();
    });
    assert!(TcpStream::connect_timeout(&address, Duration::from_millis(200)).is_err());
    if let Err(panic) = tested {
        std::panic::resume_unwind(panic);
    }
}

fn good_path(http: &Http, bridge: &Token, runtime: &Runtime, operator: &DatabaseConnection) {
    // Discovery says legacy for the unmoved name, as for any unknown one.
    assert!(matches!(
        central_client::discover(http, "Mover".into()).unwrap(),
        Discovery::Legacy
    ));
    assert_eq!(
        central_client::legacy_login(http, "mover".into(), "wrong synthetic password").err(),
        Some(Error::Rejected)
    );
    let enrollment = central_client::legacy_login(http, "MOVER".into(), PASSWORD).unwrap();
    assert_eq!(enrollment.gateway, GATEWAY);
    assert_eq!(enrollment.snapshot.legacy_id, "mover-id");
    assert_eq!(enrollment.snapshot.document, SNAPSHOT);
    let digest = enrollment.snapshot.digest;
    let identity = enrollment.identity;
    let replay = copy(&enrollment.ticket);
    central_client::enroll(http, identity, enrollment.ticket, PASSWORD.into(), DOCUMENT).unwrap();
    // A replayed enrollment ticket and a second legacy sign-in replace nothing.
    assert!(central_client::enroll(http, identity, replay, PASSWORD.into(), DOCUMENT).is_err());
    assert_eq!(
        central_client::legacy_login(http, "mover".into(), PASSWORD).err(),
        Some(Error::Conflict),
        "an enrolled account signs in through OPAQUE"
    );
    let Discovery::Central { identity: found } =
        central_client::discover(http, "mover".into()).unwrap()
    else {
        panic!("enrolled now");
    };
    assert_eq!(found.account, identity.account);
    let mut session = central_client::login(http, identity, PASSWORD.into()).unwrap();
    assert_eq!(session.document(), DOCUMENT);
    assert_eq!(
        session.legacy_link(http, digest).err(),
        Some(Error::TermsRequired),
        "the close waits for Central's terms"
    );
    accept(http, &session);
    assert_eq!(
        session.legacy_link(http, digest).err(),
        Some(Error::Rejected),
        "nothing closes before the restore is confirmed"
    );
    session.confirm_restored(http).unwrap();
    assert_eq!(session.legacy_export(http).unwrap().digest, digest);
    let mut wrong = digest;
    wrong[0] ^= 1;
    assert_eq!(
        session.legacy_link(http, wrong).err(),
        Some(Error::Conflict)
    );
    assert!(!session.migration_closed(http).unwrap());
    session.legacy_link(http, digest).unwrap();
    assert!(session.migration_closed(http).unwrap());
    assert_eq!(
        count(
            runtime,
            operator,
            "SELECT count(*) AS count FROM central_legacy.accounts WHERE legacy_id = 'mover-id'"
        ),
        0,
        "the row is deleted with the close, not marked"
    );
    // Again after the close: done, and nothing to export or sign in to.
    session.legacy_link(http, digest).unwrap();
    assert_eq!(session.legacy_export(http).err(), Some(Error::Rejected));
    assert_eq!(
        central_client::legacy_login(http, "mover".into(), PASSWORD).err(),
        Some(Error::Rejected),
        "a closed move has no legacy sign-in"
    );
    // The official gateway learns the link with the admission.
    assert_eq!(
        admission(http, &session, bridge),
        Ok(Some("mover-id".into()))
    );
    // The bridge's own legacy operations are Central's now.
    for request in [
        Request::LegacyReserve {
            legacy_id: "limited-id".into(),
            username: "limited".into(),
            display_name: None,
            tag: None,
        },
        Request::LegacyStatus {
            legacy_id: "mover-id".into(),
        },
        Request::LegacyInvalidate {
            legacy_id: "limited-id".into(),
        },
        Request::LegacyClose {
            legacy_id: "mover-id".into(),
            identity,
        },
    ] {
        assert_eq!(
            http.exchange(&request, Some(bridge)).err(),
            Some(Error::Rejected)
        );
    }
    session.logout(http).unwrap();
}

/// The bridge learns a legacy id only for a closed move.
fn open_move_is_not_admitted(http: &Http, bridge: &Token) {
    let session = moved(http, "early");
    assert_eq!(admission(http, &session, bridge), Err(Error::Rejected));
    let snapshot = session.legacy_export(http).unwrap();
    session.legacy_link(http, snapshot.digest).unwrap();
    assert_eq!(
        admission(http, &session, bridge),
        Ok(Some("early-id".into()))
    );
    session.logout(http).unwrap();
}

/// Five sign-ins a minute for one name, whoever's; ten failures in a row
/// lock the account out, and then its right password is refused too.
fn budget_and_lockout(http: &Http, runtime: &Runtime, operator: &DatabaseConnection) {
    for _ in 0..5 {
        assert_eq!(
            central_client::legacy_login(http, "limited".into(), "wrong synthetic password").err(),
            Some(Error::Rejected)
        );
    }
    assert_eq!(
        central_client::legacy_login(http, "limited".into(), PASSWORD).err(),
        Some(Error::Busy),
        "the budget holds even for the right password"
    );
    // Nine failures counted already, as over two earlier minutes.
    runtime
        .block_on(operator.execute_unprepared(
            "UPDATE central_legacy.accounts SET failures = 9 WHERE legacy_id = 'locked-id'",
        ))
        .unwrap();
    assert_eq!(
        central_client::legacy_login(http, "locked".into(), "wrong synthetic password").err(),
        Some(Error::Rejected)
    );
    assert_eq!(
        central_client::legacy_login(http, "locked".into(), PASSWORD).err(),
        Some(Error::Rejected),
        "locked out"
    );
    assert_eq!(
        count(
            runtime,
            operator,
            "SELECT count(*) AS count FROM central_legacy.accounts
             WHERE legacy_id = 'locked-id' AND locked_until > now()"
        ),
        1
    );
}

/// A native session of a registered account, its terms accepted, as a bare
/// bearer.
fn native_session(original: &Http, account: &password::Registered) -> Token {
    let (login, message) = Login::start(password::OLD.into(), account.context).unwrap();
    let Response::LoginChallenge { attempt, message } = original
        .exchange(
            &Request::LoginStart {
                identity: account.identity,
                message,
            },
            None,
        )
        .unwrap()
    else {
        panic!("a challenge");
    };
    let result = login.finish(&message).unwrap();
    let Response::Session { token, .. } = original
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
    if let Response::TermsStatus {
        current: Some(current),
        accepted: false,
    } = original
        .exchange(&Request::TermsStatus, Some(&token))
        .unwrap()
    {
        original
            .exchange(
                &Request::AcceptTerms {
                    subject: TermsSubject::Central,
                    version: current.version,
                    digest: current.digest,
                },
                Some(&token),
            )
            .unwrap();
    }
    token
}

/// Refusals with the right password, and names that are not the store's.
fn refusals(http: &Http, original: &Http, central_only: &password::Registered) {
    for name in ["banned", "forced"] {
        assert_eq!(
            central_client::legacy_login(http, name.into(), PASSWORD).err(),
            Some(Error::Rejected),
            "{name}"
        );
    }
    // A Central-only account has no legacy sign-in, its own password
    // included, and its session neither exports nor closes a move.
    assert_eq!(
        central_client::legacy_login(http, "legacy-central".into(), password::OLD).err(),
        Some(Error::Rejected)
    );
    assert_eq!(
        central_client::legacy_login(http, "nobody-at-all".into(), PASSWORD).err(),
        Some(Error::Rejected)
    );
    // Its session comes from the listener it enrolled at (another issuer and
    // OPAQUE setup); sessions are the database's, so the bearer works here.
    let token = native_session(original, central_only);
    for request in [
        Request::LegacyExport,
        Request::LegacyLink { digest: [0; 32] },
    ] {
        assert_eq!(
            http.exchange(&request, Some(&token)).err(),
            Some(Error::Rejected)
        );
    }
    original.exchange(&Request::Logout, Some(&token)).unwrap();
    // An empty password is refused before anything is looked up.
    let request = Request::LegacyLogin {
        username: "limited".into(),
        password: central_api::legacy::LegacyPassword::new(String::new()),
    };
    assert_eq!(http.exchange(&request, None).err(), Some(Error::Rejected));
}

/// The administrator's listing: what is left and how far each move got,
/// for an administrator alone.
fn listing(
    service: &IdentityService,
    original: &Http,
    browser: &Browser,
    bridge: &Token,
    runtime: &Runtime,
    operator: &DatabaseConnection,
) {
    // An administrator moved through the bridge of the service without the
    // store; its terms are accepted with the move.
    let (identity, session) = password::moved(original, bridge, "legacy-admin");
    password::close(original, bridge, "legacy-admin", identity);
    drop(session);
    let cookie = password::browser_sign_in(
        browser,
        identity,
        password::context(original, identity),
        password::OLD,
    );
    let request = || Request::Admin {
        request: AdminRequest::LegacyAccounts,
    };
    assert_eq!(
        runtime
            .block_on(service.handle_browser(request(), Some(&cookie)))
            .err(),
        Some(Error::Rejected),
        "not an administrator"
    );
    assert_eq!(
        runtime.block_on(appoint_admin(operator, "legacy-admin")),
        Ok(Appointed::Appointed)
    );
    let Response::Admin {
        answer: AdminAnswer::LegacyAccounts { store },
    } = runtime
        .block_on(service.handle_browser(request(), Some(&cookie)))
        .unwrap()
        .response
    else {
        panic!("the listing");
    };
    assert_eq!(store.deadline, None);
    let names: Vec<&str> = store
        .accounts
        .iter()
        .map(|account| account.username.as_str())
        .collect();
    assert_eq!(names, ["banned", "forced", "limited", "locked"]);
    assert!(store.accounts[0].banned && store.accounts[1].must_change_password);
    assert!(store.accounts[3].locked_until.is_some());
    assert!(
        store
            .accounts
            .iter()
            .all(|account| account.progress == LegacyProgress::Unstarted)
    );
}
