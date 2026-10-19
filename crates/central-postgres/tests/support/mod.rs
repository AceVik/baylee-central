use super::{config, scalar};
use central_postgres::{PendingAccountStore, StoreError};
use sea_orm::{ConnectionTrait, DatabaseConnection, TransactionTrait};
use std::time::Duration;

pub(super) async fn privileges(operator: &DatabaseConnection, runtime: &DatabaseConnection) {
    for sql in [
        "CREATE SCHEMA forbidden",
        "CREATE TEMP TABLE forbidden (id integer)",
        "DELETE FROM central_identity.pending_accounts",
        "TRUNCATE central_identity.pending_accounts",
        "UPDATE central_identity.pending_accounts SET expires_at = now()",
    ] {
        assert!(runtime.execute_unprepared(sql).await.is_err());
    }
    operator
        .execute_unprepared("CREATE SCHEMA central_extra")
        .await
        .unwrap();
    for (object, privilege) in [
        ("DATABASE central_test", "CREATE"),
        ("DATABASE central_test", "TEMP"),
        ("SCHEMA central_extra", "CREATE"),
        ("central_identity.pending_accounts", "UPDATE"),
        ("central_identity.pending_accounts", "UPDATE (expires_at)"),
        ("central_identity.pending_accounts", "REFERENCES (id)"),
        ("central_identity.pending_accounts", "DELETE"),
        ("central_identity.pending_accounts", "TRUNCATE"),
        ("central_identity.pending_accounts", "TRIGGER"),
        ("central_identity.pending_accounts", "MAINTAIN"),
    ] {
        // These are fixed synthetic fixture identifiers, never user-provided SQL.
        operator
            .execute_unprepared(&format!("GRANT {privilege} ON {object} TO central_runtime"))
            .await
            .unwrap();
        let result = PendingAccountStore::connect_dev(config(
            "central_runtime",
            "CENTRAL_TEST_RUNTIME_PASSWORD",
        ))
        .await;
        operator
            .execute_unprepared(&format!(
                "REVOKE {privilege} ON {object} FROM central_runtime"
            ))
            .await
            .unwrap();
        assert!(matches!(result, Err(StoreError::UnsafeRole)), "{privilege}");
    }
    operator.execute_unprepared(
        "GRANT INSERT ON central_identity.pending_accounts TO central_runtime WITH GRANT OPTION"
    ).await.unwrap();
    assert!(matches!(
        PendingAccountStore::connect_dev(config(
            "central_runtime",
            "CENTRAL_TEST_RUNTIME_PASSWORD"
        ))
        .await,
        Err(StoreError::UnsafeRole)
    ));
    operator.execute_unprepared(
        "REVOKE GRANT OPTION FOR INSERT ON central_identity.pending_accounts FROM central_runtime;
         REVOKE SELECT ON central_identity.pending_accounts FROM central_runtime"
    ).await.unwrap();
    assert!(matches!(
        PendingAccountStore::connect_dev(config(
            "central_runtime",
            "CENTRAL_TEST_RUNTIME_PASSWORD"
        ))
        .await,
        Err(StoreError::UnsafeRole)
    ));
    operator
        .execute_unprepared(
            "GRANT SELECT ON central_identity.pending_accounts TO central_runtime;
         DROP SCHEMA central_extra",
        )
        .await
        .unwrap();
}

pub(super) async fn cancellation(runtime: &DatabaseConnection) {
    // Occupy three of four pool connections: reuse must use the cancelled fourth,
    // rather than hiding a lingering transaction by checking a different socket.
    let mut held = Vec::new();
    for _ in 0..3 {
        held.push(runtime.begin().await.unwrap());
    }
    let (ready, received) = tokio::sync::oneshot::channel();
    let pool = runtime.clone();
    let task = tokio::spawn(async move {
        let transaction = pool.begin().await.unwrap();
        transaction
            .execute_unprepared(
                "INSERT INTO central_identity.pending_accounts DEFAULT VALUES;
             SELECT pg_advisory_xact_lock(1862384219, 2);
             SET LOCAL application_name = 'cancel-canary'",
            )
            .await
            .unwrap();
        ready
            .send(scalar(&transaction, "SELECT pg_backend_pid()::bigint AS value").await)
            .unwrap();
        std::future::pending::<()>().await;
        transaction.rollback().await.unwrap();
    });
    let backend = tokio::time::timeout(Duration::from_secs(5), received)
        .await
        .unwrap()
        .unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());

    let reused = runtime.begin().await.unwrap();
    assert_eq!(
        scalar(&reused, "SELECT pg_backend_pid()::bigint AS value").await,
        backend
    );
    assert_eq!(
        scalar(
            &reused,
            "SELECT count(*)::bigint AS value FROM central_identity.pending_accounts"
        )
        .await,
        0
    );
    assert_eq!(scalar(&reused,
        "SELECT (current_setting('application_name') <> 'cancel-canary')::integer::bigint AS value"
    ).await, 1);
    // A separate occupied connection must acquire the cancelled transaction's lock.
    assert_eq!(
        scalar(
            &held[0],
            "SELECT pg_try_advisory_xact_lock(1862384219, 2)::integer::bigint AS value"
        )
        .await,
        1
    );
    reused.rollback().await.unwrap();
    for transaction in held {
        transaction.rollback().await.unwrap();
    }
}
