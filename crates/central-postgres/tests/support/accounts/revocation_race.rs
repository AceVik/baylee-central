use super::{AccountStore, DatabaseConnection, ServerKeys, Token, config, enroll, login};
use central_api::{Error, gateway::GatewayMetadata};
use sea_orm::{ConnectionTrait, DbBackend, Statement, TransactionTrait, prelude::Uuid};
use std::time::Duration;

fn metadata(origin: &str) -> GatewayMetadata {
    GatewayMetadata {
        name: "Revocation fixture".into(),
        version: "1.0".into(),
        website: format!("{origin}/"),
    }
}

pub(super) async fn run(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    // A separate runtime observer may see its own role's query text without
    // granting pg_read_all_stats or weakening the actual application role.
    let observer =
        central_postgres::connect_dev(config("central_runtime", "CENTRAL_TEST_RUNTIME_PASSWORD"))
            .await
            .unwrap();
    let identity = enroll(store, keys, "revocation-race", "revocation-race").await;
    let initial = login(store, keys, identity).await;
    store.confirm_restore(&initial, 1).await.unwrap();
    let gateway = store
        .register_gateway(
            &initial,
            "https://race-owned.invalid",
            &metadata("https://race-owned.invalid"),
            &Token::generate().unwrap(),
            &Token::generate().unwrap(),
            false,
        )
        .await
        .unwrap();
    let credential = store.credential(identity).await.unwrap().unwrap();
    for action in 0..4 {
        let session = Token::generate().unwrap();
        store.issue_session(&credential, &session).await.unwrap();
        let tx = operator.begin().await.unwrap();
        tx.query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT id FROM central_identity.accounts WHERE id=$1 FOR UPDATE",
            vec![Uuid::from_bytes(identity.account).into()],
        ))
        .await
        .unwrap()
        .unwrap();
        let waiting_store = store.clone();
        let waiter =
            tokio::spawn(async move { mutation(&waiting_store, &session, gateway, action).await });
        wait_for_account_lock(&observer).await;
        tx.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "DELETE FROM central_identity.sessions WHERE account_id=$1",
            vec![Uuid::from_bytes(identity.account).into()],
        ))
        .await
        .unwrap();
        tx.commit().await.unwrap();
        assert!(
            tokio::time::timeout(Duration::from_secs(5), waiter)
                .await
                .unwrap()
                .unwrap()
                .is_err(),
            "revoked action {action}"
        );
    }
    let row = operator.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT document_revision FROM central_identity.accounts WHERE legacy_id='revocation-race'"
    )).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "document_revision").unwrap(), 1);
    let row = operator.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT count(*)::bigint AS count,max(admin_revision) AS revision FROM central_identity.gateways
         WHERE origin IN ('https://race-owned.invalid','https://race-forbidden.invalid')"
    )).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "count").unwrap(), 1);
    assert_eq!(row.try_get::<i64>("", "revision").unwrap(), 1);
    observer.close().await.unwrap();
}

async fn mutation(
    store: &AccountStore,
    session: &Token,
    gateway: [u8; 16],
    action: u8,
) -> Result<(), Error> {
    match action {
        0 => store.write_document(session, 1, &[1; 41]).await,
        1 => store.confirm_restore(session, 1).await,
        2 => store
            .register_gateway(
                session,
                "https://race-forbidden.invalid",
                &metadata("https://race-forbidden.invalid"),
                &Token::generate().unwrap(),
                &Token::generate().unwrap(),
                false,
            )
            .await
            .map(|_| ()),
        _ => store
            .set_gateway_admins(session, gateway, 1, &[])
            .await
            .map(|_| ()),
    }
}

pub(super) async fn wait_for_account_lock(operator: &DatabaseConnection) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let row = operator.query_one_raw(Statement::from_string(DbBackend::Postgres,
                "SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_stat_activity WHERE datname=current_database()
                 AND usename='central_runtime' AND wait_event_type='Lock'
                 AND query LIKE '%SELECT a.id FROM central_identity.accounts a JOIN central_identity.sessions s%') AS waiting"
            )).await.unwrap().unwrap();
            if row.try_get::<bool>("", "waiting").unwrap() { break; }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }).await.expect("runtime mutation must actually block on the held account lock");
}
