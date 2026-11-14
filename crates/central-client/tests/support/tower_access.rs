//! Focused real SDK/HTTP/PostgreSQL scope and revocation regression.
#![forbid(unsafe_code)]
use super::*;
use central_api::tower_access::Grant;
use sea_orm::{DbBackend, Statement, Value, prelude::Uuid};

const ORIGIN: &str = "http://127.0.0.1:29871";

#[test]
#[ignore = "owned fixture: cargo xtask postgres --tower-access-only"]
fn scoped_identity() {
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
    let identity = Arc::new(
        IdentityService::new(
            store.clone(),
            &endpoint,
            ServerKeys::generate(),
            &bridge,
            "http://127.0.0.1:28766",
            support::token_keys(),
        )
        .unwrap()
        .with_terms(Some(support::terms::central_document()))
        .with_tower(Some(ORIGIN))
        .unwrap(),
    );
    let (stop, stopped) = oneshot::channel();
    let serving = identity.clone();
    let serving_endpoint = endpoint.clone();
    let task = runtime.spawn(async move {
        server
            .run_with_identity(&serving_endpoint, serving, async {
                let _ = stopped.await;
            })
            .await
    });
    let http = Http {
        endpoint,
        address,
        identity,
        lose_write_reply: AtomicBool::new(false),
        lose_password_reply: AtomicBool::new(false),
        lose_delete_reply: AtomicBool::new(false),
    };
    let tested = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        exercise(&http, &bridge, &runtime, &operator, &store);
    }));
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
    assert!(TcpStream::connect_timeout(&address, Duration::from_millis(100)).is_err());
    if let Err(panic) = tested {
        std::panic::resume_unwind(panic);
    }
}

fn execute(runtime: &Runtime, operator: &DatabaseConnection, sql: &str, values: Vec<Value>) {
    runtime
        .block_on(operator.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            values,
        )))
        .unwrap_or_else(|_| panic!("synthetic account mutation failed"));
}

fn count(runtime: &Runtime, operator: &DatabaseConnection) -> i64 {
    runtime
        .block_on(operator.query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*)::bigint AS n FROM central_identity.tower_access",
        )))
        .unwrap()
        .unwrap()
        .try_get("", "n")
        .unwrap()
}

fn inspect(http: &Http, grant: &Grant) -> central_api::tower_access::Identity {
    central_client::tower_identity(http, &grant.token, ORIGIN).unwrap()
}

fn exercise(
    http: &Http,
    bridge: &Token,
    runtime: &Runtime,
    operator: &DatabaseConnection,
    store: &AccountStore,
) {
    let (account, session) = support::password::moved(http, bridge, "tower-access-fixture");
    support::password::close(http, bridge, "tower-access-fixture", account);
    assert!(http.exchange(&Request::TowerToken {}, None).is_err());
    assert!(
        http.exchange(&Request::TowerToken {}, Some(bridge.into()))
            .is_err()
    );
    assert!(
        http.exchange(&Request::TowerIdentity {}, Some(bridge.into()))
            .is_err()
    );
    let first = session.tower_token(http).unwrap();
    assert_eq!(first.origin, ORIGIN);
    assert!(first.expires_at > support::now() && first.expires_at <= support::now() + 300);
    assert_eq!(inspect(http, &first).account, account.account);
    assert!(
        http.written(
            r#"{"operation":"tower_identity","origin":"https://attacker.invalid"}"#,
            Some((&first.token).into())
        )
        .is_err()
    );
    assert!(
        http.exchange(&Request::TowerToken {}, Some((&first.token).into()))
            .is_err()
    );
    assert!(
        http.exchange(&Request::SessionStatus, Some((&first.token).into()))
            .is_err()
    );
    assert!(
        http.exchange(&Request::GatewayStatus, Some((&first.token).into()))
            .is_err()
    );
    let (gateway, _) = session
        .gateway_token(http, "http://127.0.0.1:28766".into(), Admit)
        .unwrap();
    assert!(
        http.exchange(&Request::TowerIdentity {}, Some((&gateway).into()))
            .is_err()
    );
    assert!(central_client::tower_identity(http, &first.token, "http://127.0.0.1:29872").is_err());
    assert!(
        runtime
            .block_on(store.tower_identity(&first.token, "http://127.0.0.1:29872", None))
            .is_err()
    );
    let grants: Vec<_> = std::thread::scope(|scope| {
        let tasks: Vec<_> = (0..4)
            .map(|_| scope.spawn(|| session.tower_token(http).unwrap()))
            .collect();
        tasks.into_iter().map(|task| task.join().unwrap()).collect()
    });
    assert_eq!(
        count(runtime, operator),
        2,
        "concurrent refreshes cannot exceed the database bound"
    );
    assert!(central_client::tower_identity(http, &first.token, ORIGIN).is_err());
    let live: Vec<_> = grants
        .into_iter()
        .filter(|grant| central_client::tower_identity(http, &grant.token, ORIGIN).is_ok())
        .collect();
    assert_eq!(live.len(), 2);
    current_identity(http, runtime, operator, account, &session, &live[0]);
    execute(
        runtime,
        operator,
        "UPDATE central_identity.tower_access SET created_at=clock_timestamp()-interval '10 minutes',expires_at=clock_timestamp()-interval '6 minutes' WHERE digest=$1",
        vec![live[1].token.digest().to_vec().into()],
    );
    assert!(central_client::tower_identity(http, &live[1].token, ORIGIN).is_err());
    session.logout(http).unwrap();
    assert!(central_client::tower_identity(http, &live[0].token, ORIGIN).is_err());
    assert_eq!(
        count(runtime, operator),
        0,
        "logout cascades scoped credentials"
    );
    replacement(http, runtime, operator, account);
}

fn current_identity(
    http: &Http,
    runtime: &Runtime,
    operator: &DatabaseConnection,
    account: Identity,
    session: &Session,
    grant: &Grant,
) {
    execute(
        runtime,
        operator,
        "UPDATE central_identity.accounts SET display_name='TowerUpdated',display_name_at=clock_timestamp(),tag=12345 WHERE id=$1",
        vec![Uuid::from_bytes(account.account).into()],
    );
    let current = inspect(http, grant);
    assert_eq!(current.display_name.as_deref(), Some("TowerUpdated"));
    assert_eq!(current.tag, Some(12345));
    execute(
        runtime,
        operator,
        "DELETE FROM central_identity.terms_acceptances WHERE account_id=$1 AND subject='central'",
        vec![Uuid::from_bytes(account.account).into()],
    );
    assert!(matches!(
        central_client::tower_identity(http, &grant.token, ORIGIN),
        Err(Error::TermsRequired)
    ));
    let terms = session.terms_status(http).unwrap().current.unwrap();
    session
        .accept_terms(http, central_api::terms::TermsSubject::Central, &terms)
        .unwrap();
    assert_eq!(inspect(http, grant).account, account.account);
}

fn replacement(http: &Http, runtime: &Runtime, operator: &DatabaseConnection, account: Identity) {
    let (native, _master) = support::password::sign_in(http, account, support::password::OLD);
    assert!(
        http.exchange(&Request::TowerIdentity {}, Some((&native).into()))
            .is_err()
    );
    let Response::TowerToken { grant } = http
        .exchange(&Request::TowerToken {}, Some((&native).into()))
        .unwrap()
    else {
        panic!("scoped Tower grant expected");
    };
    execute(
        runtime,
        operator,
        "UPDATE central_identity.accounts SET credential_revision=credential_revision+1 WHERE id=$1",
        vec![Uuid::from_bytes(account.account).into()],
    );
    assert!(central_client::tower_identity(http, &grant.token, ORIGIN).is_err());
    assert!(
        http.exchange(&Request::TowerToken {}, Some((&native).into()))
            .is_err()
    );
    execute(
        runtime,
        operator,
        "DELETE FROM central_identity.accounts WHERE id=$1",
        vec![Uuid::from_bytes(account.account).into()],
    );
    assert_eq!(count(runtime, operator), 0);
}
