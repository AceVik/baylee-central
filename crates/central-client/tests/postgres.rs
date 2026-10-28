//! Real HTTP + PostgreSQL + SDK fixture; exclusively synthetic, through `cargo xtask postgres`.
#![forbid(unsafe_code)]

use central_api::{
    Endpoint, Error, Token,
    wire::{Discovery, Identity, Request, Response, TicketPurpose},
};
use central_client::{Session, Transport};
use central_crypto::opaque::{Login, server::ServerKeys};
use central_http::{BrowserOrigin, DevelopmentServer};
use central_identity::IdentityService;
use central_postgres::{AccountStore, connect_dev};
use sea_orm::{ConnectionTrait, DatabaseConnection};
use std::{
    net::TcpStream,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::{runtime::Runtime, sync::oneshot};

mod support;
use support::{Http, config};

#[test]
#[ignore = "requires the owned PostgreSQL fixture from `cargo xtask postgres`"]
fn real_native_login_restore_migration_and_adversarial_paths() {
    let runtime = Runtime::new().unwrap();
    let operator = runtime.block_on(prepare());
    let store = runtime
        .block_on(AccountStore::connect_dev(config(
            "central_runtime",
            "CENTRAL_TEST_RUNTIME_PASSWORD",
        )))
        .unwrap();
    let server = runtime
        .block_on(DevelopmentServer::bind(central_policy_config()))
        .unwrap();
    let address = server.local_addr();
    let browser = runtime
        .block_on(DevelopmentServer::bind(
            central_policy::config::Config::from_pairs([
                ("BAYLEE_CENTRAL_LISTEN", "127.0.0.1:0"),
                ("BAYLEE_CENTRAL_REQUEST_BYTES", "65536"),
            ])
            .unwrap(),
        ))
        .unwrap();
    let browser_address = browser.local_addr();
    let endpoint = Endpoint::development(&format!("http://{address}")).unwrap();
    let bridge = Token::generate().unwrap();
    let keys = ServerKeys::generate();
    let restored = ServerKeys::from_secret_bytes(&keys.to_secret_bytes()).unwrap();
    let identity = Arc::new(
        IdentityService::new(
            store.clone(),
            &endpoint,
            restored,
            &bridge,
            "http://127.0.0.1:28766",
        )
        .unwrap()
        .with_terms(Some(support::terms::central_document())),
    );
    let (stop_browser, browser_stopped) = oneshot::channel();
    let browser_task =
        runtime.spawn(
            browser.run_browser(identity.clone(), BrowserOrigin::live(), async {
                let _ = browser_stopped.await;
            }),
        );
    let (stop, stopped) = oneshot::channel();
    let serving_endpoint = endpoint.clone();
    let task = runtime.spawn(async move {
        server
            .run_with_identity(&serving_endpoint, identity, async {
                let _ = stopped.await;
            })
            .await
    });
    let http = Http {
        endpoint,
        address,
        lose_write_reply: AtomicBool::new(false),
        lose_password_reply: AtomicBool::new(false),
    };
    let tested = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run(&http, &bridge, &runtime, &operator);
        let browser = support::browser::Browser {
            address: browser_address,
            client: "198.51.100.7",
        };
        support::browser::run(&browser, &http, &bridge, &runtime, &operator);
        support::password::run(&http, &browser, &bridge, &runtime, &operator);
        support::admin::run(&http, &browser, &bridge, &runtime, &operator);
        support::reset::run(&http, &browser, &bridge, &runtime, &operator);
    }));
    // Cleanup runs even if an assertion panics; listeners and pools belong to this fixture only.
    let _ = stop.send(());
    let _ = stop_browser.send(());
    runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        tokio::time::timeout(Duration::from_secs(5), browser_task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        store.close().await.unwrap();
        operator.close().await.unwrap();
    });
    assert!(TcpStream::connect_timeout(&address, Duration::from_millis(200)).is_err());
    assert!(TcpStream::connect_timeout(&browser_address, Duration::from_millis(200)).is_err());
    if let Err(panic) = tested {
        std::panic::resume_unwind(panic);
    }
}

fn central_policy_config() -> central_policy::config::Config {
    central_policy::config::Config::from_pairs([
        ("BAYLEE_CENTRAL_LISTEN", "127.0.0.1:0"),
        ("BAYLEE_CENTRAL_REQUEST_BYTES", "1056768"),
    ])
    .unwrap()
}

async fn prepare() -> DatabaseConnection {
    let db = connect_dev(config("central_migrator", "CENTRAL_TEST_MIGRATOR_PASSWORD"))
        .await
        .unwrap();
    central_migration::apply(&db).await.unwrap();
    // The runtime role's grants exactly as the deploy hook applies them.
    db.execute_unprepared(include_str!("../../../ops/grants.sql"))
        .await
        .unwrap();
    db
}

fn run(http: &Http, bridge: &Token, runtime: &Runtime, operator: &DatabaseConnection) {
    // As a bridge from before display names sends it.
    let reserve: Request = central_api::decode(
        br#"{"operation":"legacy_reserve","legacy_id":"synthetic-legacy","username":"synthetic"}"#,
    )
    .unwrap();
    assert!(http.exchange(&reserve, None).is_err());
    assert!(
        http.exchange(&reserve, Some(&Token::generate().unwrap()))
            .is_err()
    );
    let Response::Enrollment { identity, ticket } = http.exchange(&reserve, Some(bridge)).unwrap()
    else {
        panic!("enrollment expected")
    };
    assert!(
        central_client::discover(http, "synthetic".into()).is_err(),
        "dev never redirects to live migration"
    );
    let original =
        br#"{"version":1,"gateways":["http://127.0.0.1:28766"],"settings":{"synthetic":true}}"#;
    central_client::enroll(
        http,
        identity,
        ticket,
        "synthetic secure password".into(),
        original,
    )
    .unwrap();
    assert!(
        http.exchange(&reserve, Some(bridge)).is_err(),
        "legacy retries cannot replace credentials"
    );
    let Discovery::Central { identity: found } =
        central_client::discover(http, "synthetic".into()).unwrap()
    else {
        panic!("central expected")
    };
    assert_eq!(found.account, identity.account);
    assert!(central_client::login(http, identity, "wrong synthetic password".into()).is_err());
    failed_proof_is_consumed(http, identity);
    let mut revoked =
        central_client::login(http, identity, "synthetic secure password".into()).unwrap();
    let mut first =
        central_client::login(http, identity, "synthetic secure password".into()).unwrap();
    assert!(revoked.refresh(http).is_err());
    assert!(central_client::login(http, identity, "wrong password".into()).is_err());
    first.check_active(http).unwrap();
    // Renewal needs no accepted terms, and a replaced session gets none.
    assert!(first.renew(http).unwrap() > 0);
    assert!(revoked.renew(http).is_err());
    assert!(!first.migration_closed(http).unwrap());
    assert_eq!(first.document(), original);
    assert!(
        first
            .gateway_ticket(http, "http://127.0.0.1:28766".into())
            .is_err()
    );
    first.confirm_restored(http).unwrap();
    support::terms::central(http, &first, "http://127.0.0.1:28766");
    official_admission(http, &first, bridge);
    assert!(
        first
            .gateway_ticket(http, "https://attacker.invalid".into())
            .is_err()
    );
    http.exchange(
        &Request::LegacyClose {
            legacy_id: "synthetic-legacy".into(),
            identity,
        },
        Some(bridge),
    )
    .unwrap();
    support::gateways::run(http, &first, runtime, operator);
    support::gateways::official(
        http,
        &first,
        bridge,
        "http://127.0.0.1:28766",
        runtime,
        operator,
    );
    support::terms::gateway(http, &first, bridge, "http://127.0.0.1:28766");
    assert!(first.migration_closed(http).unwrap());
    object_round_trip(http, identity, &first);
    // A different device restored the chunk and replaced this session.
    let mut first =
        central_client::login(http, identity, "synthetic secure password".into()).unwrap();
    lost_responses_and_conflicts(http, &mut first);
    runtime.block_on(operator.execute_unprepared("UPDATE central_identity.accounts SET document = set_byte(document, 25, get_byte(document,25) # 1) WHERE legacy_id = 'synthetic-legacy'")).unwrap();
    let last_good = first.document().to_vec();
    assert!(first.refresh(http).is_err());
    assert_eq!(first.document(), last_good);
    assert_eq!(session_count(runtime, operator), 1);
    assert!(central_client::login(http, identity, "synthetic secure password".into()).is_err());
    assert_eq!(
        session_count(runtime, operator),
        0,
        "failed restoration revokes its newly issued session"
    );
    first.logout(http).unwrap();
    assert!(first.refresh(http).is_err());
}

/// Only the bridge redeems an official ticket, and only once.
fn official_admission(http: &Http, session: &Session, bridge: &Token) {
    let ticket = session
        .gateway_ticket(http, "http://127.0.0.1:28766".into())
        .unwrap();
    let redeem = Request::RedeemGateway {
        ticket,
        purpose: TicketPurpose::Admit,
    };
    assert!(http.exchange(&redeem, None).is_err());
    assert!(matches!(
        http.exchange(&redeem, Some(bridge)).unwrap(),
        Response::GatewayIdentity { .. }
    ));
    assert!(http.exchange(&redeem, Some(bridge)).is_err());
}

fn session_count(runtime: &Runtime, operator: &DatabaseConnection) -> i64 {
    runtime
        .block_on(operator.query_one_raw(sea_orm::Statement::from_string(
            sea_orm::DbBackend::Postgres,
            "SELECT count(*) AS count FROM central_identity.sessions",
        )))
        .unwrap()
        .unwrap()
        .try_get("", "count")
        .unwrap()
}

fn failed_proof_is_consumed(http: &Http, identity: Identity) {
    let context =
        central_crypto::Context::new(http.endpoint.issuer(), identity.account, identity.epoch)
            .unwrap();
    let (login, message) = Login::start("synthetic secure password".into(), context).unwrap();
    let Response::LoginChallenge { attempt, message } = http
        .exchange(&Request::LoginStart { identity, message }, None)
        .unwrap()
    else {
        panic!("challenge expected")
    };
    let proof = login.finish(&message).unwrap().finalization;
    let mut request = Request::LoginFinish {
        attempt,
        message: proof,
    };
    if let Request::LoginFinish { message, .. } = &mut request {
        message[0] ^= 1;
    }
    assert!(http.exchange(&request, None).is_err());
    if let Request::LoginFinish { message, .. } = &mut request {
        message[0] ^= 1;
    }
    assert!(
        http.exchange(&request, None).is_err(),
        "failed proof consumed the challenge"
    );
}

fn lost_responses_and_conflicts(http: &Http, first: &mut Session) {
    let replacement = br#"{"version":1,"gateways":[],"settings":{"changed":true}}"#;
    http.lose_write_reply.store(true, Ordering::Release);
    assert_eq!(first.save(http, replacement), Err(Error::Unavailable));
    assert_eq!(first.revision(), 1);
    assert_eq!(first.save(http, b"stale overwrite"), Err(Error::Conflict));
    first.refresh(http).unwrap();
    assert_eq!(first.revision(), 2);
    assert_eq!(first.document(), replacement);
}

fn object_round_trip(http: &Http, identity: Identity, first: &Session) {
    let chunk = first.create_object(http).unwrap();
    assert!(first.read_object(http, chunk.id, 0).is_err());
    let history = b"synthetic complete private chat history";
    assert_eq!(first.write_object(http, chunk.id, 0, history).unwrap(), 1);
    assert_eq!(
        first.write_object(http, chunk.id, 0, b"stale"),
        Err(Error::Conflict)
    );
    assert!(first.read_object(http, chunk.id, 2).is_err());
    let another_device =
        central_client::login(http, identity, "synthetic secure password".into()).unwrap();
    assert!(first.check_active(http).is_err());
    assert!(first.read_object(http, chunk.id, 1).is_err());
    let restored = another_device.read_object(http, chunk.id, 1).unwrap();
    assert_eq!(&*restored.plaintext, history);
    assert_eq!(
        another_device.list_objects(http, None).unwrap()[0].id,
        chunk.id
    );
    assert_eq!(another_device.delete_object(http, chunk.id, 1).unwrap(), 2);
    assert!(another_device.read_object(http, chunk.id, 1).is_err());
    assert_eq!(
        another_device.write_object(http, chunk.id, 2, history),
        Err(Error::Conflict)
    );
    another_device.logout(http).unwrap();
}
