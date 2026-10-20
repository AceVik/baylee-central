//! The deployed connector against the fixture's real TLS listener.
use central_postgres::{
    AccountStore, PendingAccountStore, ServerConfig, StoreError, connect_server,
};

fn pem(variable: &str) -> Vec<u8> {
    std::fs::read(std::env::var(variable).unwrap()).unwrap()
}

fn server(user: &str, password_variable: &str, ca_variable: &str) -> ServerConfig {
    let port = std::env::var("CENTRAL_TEST_PORT").unwrap().parse().unwrap();
    ServerConfig::new(
        "127.0.0.1",
        port,
        user,
        std::env::var(password_variable).unwrap(),
        "central_test",
        pem(ca_variable),
    )
    .unwrap()
}

pub(crate) async fn run() {
    const RUNTIME: &str = "CENTRAL_TEST_RUNTIME_PASSWORD";
    const MIGRATOR: &str = "CENTRAL_TEST_MIGRATOR_PASSWORD";
    const CA: &str = "CENTRAL_TEST_CA_PEM";
    const WRONG_CA: &str = "CENTRAL_TEST_WRONG_CA_PEM";

    // Same role and grant contract as the development path, now over verified TLS.
    let store = AccountStore::connect(server("central_runtime", RUNTIME, CA))
        .await
        .unwrap();
    store.close().await.unwrap();
    let pending = PendingAccountStore::connect(server("central_runtime", RUNTIME, CA))
        .await
        .unwrap();
    pending.reserve().await.unwrap();
    pending.close().await.unwrap();
    assert!(matches!(
        PendingAccountStore::connect(server("central_inherited", RUNTIME, CA)).await,
        Err(StoreError::UnsafeRole)
    ));
    assert!(
        AccountStore::connect(server("central_migrator", MIGRATOR, CA))
            .await
            .is_err()
    );

    // The operator path: migrations over the same verified transport stay idempotent.
    let operator = connect_server(server("central_migrator", MIGRATOR, CA))
        .await
        .unwrap();
    central_migration::apply(&operator).await.unwrap();
    operator.close().await.unwrap();

    // A certificate from another authority is refused before any query runs.
    assert!(matches!(
        connect_server(server("central_runtime", RUNTIME, WRONG_CA)).await,
        Err(StoreError::Unavailable)
    ));
    // A wrong password over a verified session is refused too, generically.
    let wrong = ServerConfig::new(
        "127.0.0.1",
        std::env::var("CENTRAL_TEST_PORT").unwrap().parse().unwrap(),
        "central_runtime",
        "not-the-password".into(),
        "central_test",
        pem(CA),
    )
    .unwrap();
    assert!(matches!(
        connect_server(wrong).await,
        Err(StoreError::Unavailable)
    ));
}
