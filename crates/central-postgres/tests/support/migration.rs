use super::{config, scalar};
use sea_orm::{ConnectionTrait, DatabaseConnection, TransactionTrait};
use std::time::Duration;

pub(crate) async fn search_path(operator: &DatabaseConnection) {
    // Keep three connections busy so session settings and apply use the same
    // fourth backend. A caller's search_path must not choose migration history.
    let mut held = Vec::new();
    for _ in 0..3 {
        held.push(operator.begin().await.unwrap());
    }
    missing_public(operator).await;
    inaccessible_public(operator).await;
    shadow_histories(operator).await;
    central_migration::apply(operator).await.unwrap();
    assert_eq!(
        scalar(operator,
            "SELECT (pg_catalog.to_regclass('central_identity.pending_accounts') IS NOT NULL)::integer::bigint AS value"
        ).await,
        1,
        "a shadow history must not silently skip schema installation"
    );
    assert_eq!(
        scalar(operator,
            "SELECT (pg_catalog.current_setting('search_path') = 'migration_shadow, public')::integer::bigint AS value"
        ).await,
        1,
        "the caller's session settings must be restored after success"
    );

    operator
        .execute_unprepared(
            "DELETE FROM pg_temp.seaql_migrations;
             INSERT INTO pg_temp.seaql_migrations VALUES ('unknown_shadow_version', 0);
             SET search_path = pg_temp, migration_shadow",
        )
        .await
        .unwrap();
    central_migration::apply(operator).await.unwrap();
    assert_eq!(
        scalar(
            operator,
            "SELECT count(*)::bigint AS value FROM public.seaql_migrations"
        )
        .await,
        6
    );
    operator
        .execute_unprepared(
            "INSERT INTO public.seaql_migrations VALUES ('unknown_public_version', 0)",
        )
        .await
        .unwrap();
    assert!(central_migration::apply(operator).await.is_err());
    assert_eq!(
        scalar(operator,
            "SELECT (pg_catalog.current_setting('search_path') = 'pg_temp, migration_shadow')::integer::bigint AS value"
        ).await,
        1,
        "the caller's session settings must also be restored after rollback"
    );
    operator
        .execute_unprepared(
            "DELETE FROM public.seaql_migrations WHERE version = 'unknown_public_version';
             DROP TABLE pg_temp.seaql_migrations;
             SET search_path = public;
             DROP SCHEMA migration_shadow CASCADE",
        )
        .await
        .unwrap();
    for transaction in held {
        transaction.rollback().await.unwrap();
    }
    cancellation(operator).await;
}

async fn cancellation(operator: &DatabaseConnection) {
    let mut held = Vec::new();
    for _ in 0..3 {
        held.push(operator.begin().await.unwrap());
    }
    held[0]
        .execute_unprepared(
            "SAVEPOINT migration_lock_fixture;
             LOCK TABLE public.seaql_migrations IN ACCESS EXCLUSIVE MODE",
        )
        .await
        .unwrap();
    let backend = scalar(
        operator,
        "SELECT pg_catalog.pg_backend_pid()::bigint AS value",
    )
    .await;
    let connection = operator.clone();
    let task = tokio::spawn(async move { central_migration::apply(&connection).await });
    // Wait until the migration owns its advisory lock and is blocked on history;
    // no timing assumption about how quickly the spawned task acquires a socket.
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if scalar(
                &held[1],
                "SELECT count(*)::bigint AS value FROM pg_catalog.pg_locks
                 WHERE relation = 'public.seaql_migrations'::regclass AND NOT granted",
            )
            .await
                == 1
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    // Release the relation lock without opening another available pool slot.
    held[0]
        .execute_unprepared("ROLLBACK TO SAVEPOINT migration_lock_fixture")
        .await
        .unwrap();
    let reused = operator.begin().await.unwrap();
    assert_eq!(
        scalar(
            &reused,
            "SELECT pg_catalog.pg_backend_pid()::bigint AS value"
        )
        .await,
        backend
    );
    assert_eq!(scalar(&reused,
        "SELECT (pg_catalog.current_setting('search_path') = 'public')::integer::bigint AS value"
    ).await, 1);
    assert_eq!(
        scalar(
            &held[1],
            "SELECT pg_catalog.pg_try_advisory_xact_lock(1862384219, 1)::integer::bigint AS value"
        )
        .await,
        1
    );
    reused.rollback().await.unwrap();
    for transaction in held {
        transaction.rollback().await.unwrap();
    }
    central_migration::apply(operator).await.unwrap();
}

async fn shadow_histories(operator: &DatabaseConnection) {
    // Both shadows exist before the real history has ever been installed.
    operator
        .execute_unprepared(
            "CREATE SCHEMA migration_shadow;
             CREATE TABLE migration_shadow.seaql_migrations (
               version varchar PRIMARY KEY, applied_at bigint NOT NULL
             );
             INSERT INTO migration_shadow.seaql_migrations VALUES
               ('m20261019_000001_pending_accounts', 0);
             CREATE TEMP TABLE seaql_migrations (
               version varchar PRIMARY KEY, applied_at bigint NOT NULL
             );
             INSERT INTO pg_temp.seaql_migrations VALUES
               ('m20261019_000001_pending_accounts', 0);
             SET search_path = migration_shadow, public",
        )
        .await
        .unwrap();
}

async fn missing_public(operator: &DatabaseConnection) {
    // PostgreSQL silently skips nonexistent schemas in search_path. Without a
    // trusted-schema gate, SeaORM's CREATE IF NOT EXISTS can accept this TEMP
    // history and report success while installing none of the real schema.
    operator
        .execute_unprepared(
            "ALTER SCHEMA public RENAME TO migration_public_saved;
             CREATE TEMP TABLE seaql_migrations (
               version varchar PRIMARY KEY, applied_at bigint NOT NULL
             );
             INSERT INTO pg_temp.seaql_migrations VALUES
               ('m20261019_000001_pending_accounts', 0)",
        )
        .await
        .unwrap();
    let result = central_migration::apply(operator).await;
    operator
        .execute_unprepared(
            "ALTER SCHEMA migration_public_saved RENAME TO public;
             DROP TABLE pg_temp.seaql_migrations",
        )
        .await
        .unwrap();
    assert!(
        result.is_err(),
        "missing public must not select TEMP history"
    );
}

async fn inaccessible_public(operator: &DatabaseConnection) {
    operator
        .execute_unprepared(
            "GRANT TEMP ON DATABASE central_test TO central_runtime;
             REVOKE USAGE ON SCHEMA public FROM PUBLIC",
        )
        .await
        .unwrap();
    let runtime =
        central_postgres::connect_dev(config("central_runtime", "CENTRAL_TEST_RUNTIME_PASSWORD"))
            .await
            .unwrap();
    let mut held = Vec::new();
    for _ in 0..3 {
        held.push(runtime.begin().await.unwrap());
    }
    runtime
        .execute_unprepared(
            "CREATE TEMP TABLE seaql_migrations (
               version varchar PRIMARY KEY, applied_at bigint NOT NULL
             );
             INSERT INTO pg_temp.seaql_migrations VALUES
               ('m20261019_000001_pending_accounts', 0);
             SET search_path = public, pg_temp",
        )
        .await
        .unwrap();
    assert_eq!(
        scalar(
            &runtime,
            "SELECT (pg_catalog.current_schema() LIKE 'pg_temp_%')::integer::bigint AS value"
        )
        .await,
        1
    );
    let result = central_migration::apply(&runtime).await;
    for transaction in held {
        transaction.rollback().await.unwrap();
    }
    runtime.close().await.unwrap();
    operator
        .execute_unprepared(
            "GRANT USAGE ON SCHEMA public TO PUBLIC;
             REVOKE TEMP ON DATABASE central_test FROM central_runtime",
        )
        .await
        .unwrap();
    assert!(
        result.is_err(),
        "inaccessible public must not select TEMP history"
    );
}
