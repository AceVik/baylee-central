//! Real HTTP + PostgreSQL + SDK fixture; exclusively synthetic, through scripts/test-postgres.ps1.
#![forbid(unsafe_code)]

use central_api::{
    Endpoint, Error, Token,
    wire::{Discovery, Identity, Request, Response},
};
use central_client::{Session, Transport};
use central_crypto::opaque::{Login, server::ServerKeys};
use central_http::DevelopmentServer;
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
#[ignore = "requires owned PostgreSQL fixture from scripts/test-postgres.ps1"]
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
        .unwrap(),
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
    };
    let tested = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run(&http, &bridge, &runtime, &operator);
    }));
    // Cleanup runs even if an assertion panics; listeners and pools belong to this fixture only.
    let _ = stop.send(());
    runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        store.close().await.unwrap();
        operator.close().await.unwrap();
    });
    assert!(TcpStream::connect_timeout(&address, Duration::from_millis(200)).is_err());
    if let Err(panic) = tested {
        std::panic::resume_unwind(panic);
    }
}

fn central_policy_config() -> central_policy::config::Config {
    central_policy::config::Config::from_pairs([
        ("BAYLEE_CENTRAL_LISTEN", "127.0.0.1:0"),
        ("BAYLEE_CENTRAL_REQUEST_BYTES", "1048576"),
    ])
    .unwrap()
}

async fn prepare() -> DatabaseConnection {
    let db = connect_dev(config("central_migrator", "CENTRAL_TEST_MIGRATOR_PASSWORD"))
        .await
        .unwrap();
    central_migration::apply(&db).await.unwrap();
    db.execute_unprepared("GRANT USAGE ON SCHEMA central_identity TO central_runtime;
        GRANT SELECT, INSERT ON central_identity.pending_accounts TO central_runtime;
        GRANT SELECT, INSERT, UPDATE, DELETE ON central_identity.accounts, central_identity.sessions,
        central_identity.gateway_tickets TO central_runtime").await.unwrap();
    db
}

fn run(http: &Http, bridge: &Token, runtime: &Runtime, operator: &DatabaseConnection) {
    let reserve = Request::LegacyReserve {
        legacy_id: "synthetic-legacy".into(),
        username: "synthetic".into(),
    };
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
    let mut first =
        central_client::login(http, identity, "synthetic secure password".into()).unwrap();
    let mut second =
        central_client::login(http, identity, "synthetic secure password".into()).unwrap();
    assert_eq!(first.document(), original);
    assert_eq!(second.document(), original);
    assert!(
        first
            .gateway_ticket(http, "http://127.0.0.1:28766".into())
            .is_err()
    );
    first.confirm_restored(http).unwrap();
    let ticket = first
        .gateway_ticket(http, "http://127.0.0.1:28766".into())
        .unwrap();
    let redeem = Request::RedeemGateway { ticket };
    assert!(http.exchange(&redeem, None).is_err());
    assert!(matches!(
        http.exchange(&redeem, Some(bridge)).unwrap(),
        Response::GatewayIdentity { .. }
    ));
    assert!(http.exchange(&redeem, Some(bridge)).is_err());
    assert!(
        first
            .gateway_ticket(http, "https://attacker.invalid".into())
            .is_err()
    );
    lost_responses_and_conflicts(http, &mut first, &mut second);
    runtime.block_on(operator.execute_unprepared("UPDATE central_identity.accounts SET document = set_byte(document, 25, get_byte(document,25) # 1) WHERE legacy_id = 'synthetic-legacy'")).unwrap();
    let last_good = first.document().to_vec();
    assert!(first.refresh(http).is_err());
    assert_eq!(first.document(), last_good);
    assert!(central_client::login(http, identity, "synthetic secure password".into()).is_err());
    first.logout(http).unwrap();
    assert!(first.refresh(http).is_err());
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

fn lost_responses_and_conflicts(http: &Http, first: &mut Session, second: &mut Session) {
    let replacement = br#"{"version":1,"gateways":[],"settings":{"changed":true}}"#;
    http.lose_write_reply.store(true, Ordering::Release);
    assert_eq!(first.save(http, replacement), Err(Error::Unavailable));
    assert_eq!(first.revision(), 1);
    first.refresh(http).unwrap();
    assert_eq!(first.revision(), 2);
    assert_eq!(first.document(), replacement);
    assert_eq!(second.save(http, b"stale overwrite"), Err(Error::Conflict));
    second.refresh(http).unwrap();
    assert_eq!(second.document(), replacement);
}
