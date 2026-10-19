//! Real isolated PostgreSQL fixture; invoke through scripts/test-postgres.ps1.
#![forbid(unsafe_code)]

use central_postgres::{DevConfig, PendingAccountStore, StoreError, connect_dev};
use sea_orm::{ConnectionTrait, DbBackend, Statement, TransactionTrait};
use std::collections::HashSet;

mod support;

fn config(user: &str, password_variable: &str) -> DevConfig {
    assert_eq!(
        std::env::var("CENTRAL_TEST_FIXTURE").unwrap(),
        "isolated-v1"
    );
    let port = std::env::var("CENTRAL_TEST_PORT").unwrap().parse().unwrap();
    DevConfig::new(
        port,
        user,
        std::env::var(password_variable).unwrap(),
        "central_test",
    )
    .unwrap()
}

async fn scalar(db: &impl ConnectionTrait, query: &str) -> i64 {
    db.query_one_raw(Statement::from_string(DbBackend::Postgres, query))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "value")
        .unwrap()
}

#[tokio::test]
#[ignore = "requires the owned PostgreSQL container from scripts/test-postgres.ps1"]
async fn migrations_ids_roles_rollback_and_locking() {
    let operator = connect_dev(config("central_migrator", "CENTRAL_TEST_MIGRATOR_PASSWORD"))
        .await
        .unwrap();

    verify_migrations(&operator).await;

    assert!(matches!(
        PendingAccountStore::connect_dev(config(
            "central_inherited",
            "CENTRAL_TEST_RUNTIME_PASSWORD"
        ))
        .await,
        Err(StoreError::UnsafeRole)
    ));

    let runtime = connect_dev(config("central_runtime", "CENTRAL_TEST_RUNTIME_PASSWORD"))
        .await
        .unwrap();
    assert!(
        runtime
            .execute_unprepared("CREATE TABLE central_identity.forbidden (id integer)")
            .await
            .is_err()
    );
    assert!(
        runtime
            .execute_unprepared("DROP TABLE central_identity.pending_accounts")
            .await
            .is_err()
    );
    assert!(central_migration::apply(&runtime).await.is_err());
    verify_constraints_and_rollback(&runtime).await;
    support::privileges(&operator, &runtime).await;
    support::cancellation(&runtime).await;

    let store = PendingAccountStore::connect_dev(config(
        "central_runtime",
        "CENTRAL_TEST_RUNTIME_PASSWORD",
    ))
    .await
    .unwrap();
    let mut ids = HashSet::new();
    for _ in 0..32 {
        let (first, second) = tokio::join!(store.reserve(), store.reserve());
        for reservation in [first.unwrap(), second.unwrap()] {
            let id = reservation.id();
            assert_eq!(id[6] >> 4, 7);
            assert_eq!(id[8] >> 6, 2);
            assert!(ids.insert(id));
            assert!(reservation.expires_at() > 0);
        }
    }
    assert_eq!(ids.len(), 64);
    assert_eq!(
        scalar(
            &runtime,
            "SELECT count(*)::bigint AS value FROM central_identity.pending_accounts"
        )
        .await,
        64
    );
    assert_eq!(
        scalar(
            &runtime,
            "SELECT count(*)::bigint AS value FROM central_identity.pending_accounts
         WHERE expires_at - created_at = interval '15 minutes'"
        )
        .await,
        64
    );
    let rows = runtime
        .query_all_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT pg_get_expr(adbin, adrelid) AS expression FROM pg_attrdef d
         JOIN pg_attribute a ON a.attrelid = d.adrelid AND a.attnum = d.adnum
         WHERE a.attrelid = 'central_identity.pending_accounts'::regclass AND a.attname = 'id'",
        ))
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert!(
        rows[0]
            .try_get::<String>("", "expression")
            .unwrap()
            .contains("uuidv7()")
    );

    support::accounts(&operator).await;
    store.close().await.unwrap();
    runtime.close().await.unwrap();
    operator.close().await.unwrap();
}

async fn verify_migrations(operator: &sea_orm::DatabaseConnection) {
    // Collision forces failure after migration history installation. The outer
    // transaction must remove that partial history as well as any migration DDL.
    operator
        .execute_unprepared("CREATE SCHEMA central_identity")
        .await
        .unwrap();
    assert!(central_migration::apply(operator).await.is_err());
    assert_eq!(
        scalar(
            operator,
            "SELECT count(*)::bigint AS value FROM pg_tables
         WHERE schemaname = 'public' AND tablename = 'seaql_migrations'"
        )
        .await,
        0
    );
    operator
        .execute_unprepared("DROP SCHEMA central_identity")
        .await
        .unwrap();

    support::migration_search_path(operator).await;
    central_migration::apply(operator).await.unwrap();
    assert_eq!(
        scalar(
            operator,
            "SELECT count(*)::bigint AS value FROM seaql_migrations"
        )
        .await,
        2
    );

    operator.execute_unprepared(
        "INSERT INTO seaql_migrations (version, applied_at) VALUES ('unexpected_future_version', 0)"
    ).await.unwrap();
    assert!(central_migration::apply(operator).await.is_err());
    operator
        .execute_unprepared(
            "DELETE FROM seaql_migrations WHERE version = 'unexpected_future_version'",
        )
        .await
        .unwrap();

    let lock = operator.begin().await.unwrap();
    lock.execute_unprepared("SELECT pg_advisory_xact_lock(1862384219, 1)")
        .await
        .unwrap();
    assert!(central_migration::apply(operator).await.is_err());
    lock.rollback().await.unwrap();
    central_migration::apply(operator).await.unwrap();

    operator
        .execute_unprepared(
            "GRANT USAGE ON SCHEMA central_identity TO central_runtime;
         GRANT INSERT, SELECT ON central_identity.pending_accounts TO central_runtime;",
        )
        .await
        .unwrap();

    assert!(matches!(
        PendingAccountStore::connect_dev(config(
            "central_migrator",
            "CENTRAL_TEST_MIGRATOR_PASSWORD"
        ))
        .await,
        Err(StoreError::UnsafeRole)
    ));
}

async fn verify_constraints_and_rollback(runtime: &sea_orm::DatabaseConnection) {
    for invalid_id in [
        "00000000-0000-4000-8000-000000000000",
        "00000000-0000-7000-0000-000000000000",
    ] {
        let statement = Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO central_identity.pending_accounts (id) VALUES ($1::uuid)",
            [invalid_id.into()],
        );
        assert!(runtime.execute_raw(statement).await.is_err());
    }

    let transaction = runtime.begin().await.unwrap();
    transaction
        .execute_unprepared("INSERT INTO central_identity.pending_accounts DEFAULT VALUES")
        .await
        .unwrap();
    transaction.rollback().await.unwrap();
    assert_eq!(
        scalar(
            runtime,
            "SELECT count(*)::bigint AS value FROM central_identity.pending_accounts"
        )
        .await,
        0
    );

    // Drop queues rollback in SQLx. Reusing the pool must not retain the transaction.
    {
        let abandoned = runtime.begin().await.unwrap();
        abandoned
            .execute_unprepared("INSERT INTO central_identity.pending_accounts DEFAULT VALUES")
            .await
            .unwrap();
    }
    assert_eq!(
        scalar(
            runtime,
            "SELECT count(*)::bigint AS value FROM central_identity.pending_accounts"
        )
        .await,
        0
    );
}
