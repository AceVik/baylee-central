//! Explicit versioned migrations. Never run automatically from the HTTP service.
//! The caller supplies a dedicated DDL connection, with trusted TLS outside fixtures.
//! The raw `SeaORM` migrator is private: callers cannot bypass [`apply`] or invoke
//! destructive convenience operations through this library.
//!
//! ```compile_fail
//! use central_migration::Migrator;
//! ```
#![forbid(unsafe_code)]

mod m20261019_000001_pending_accounts;

use sea_orm_migration::{
    MigrationTrait, MigratorTrait,
    sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, TransactionTrait},
};

// Ordered, append-only registry; never expose SeaORM's destructive convenience API.
struct Migrator;

#[sea_orm_migration::async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20261019_000001_pending_accounts::Migration)]
    }
}

/// Generic operator-facing error; database values and credentials are not exposed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MigrationError;
impl std::fmt::Display for MigrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("database migration rejected")
    }
}
impl std::error::Error for MigrationError {}

/// Apply all pending migrations atomically under a transaction-scoped advisory lock.
/// PostgreSQL 18+ is required. A failed attempt rolls back DDL and migration history.
/// Destructive down/fresh/reset operations are not exposed by this entry point.
/// History lives in the trusted `public` schema, independently of caller settings.
/// A missing or inaccessible `public` schema is rejected instead of selecting TEMP.
///
/// # Errors
///
/// Rejects unsupported backends/versions, concurrent migration, failed DDL or commit.
pub async fn apply(db: &DatabaseConnection) -> Result<(), MigrationError> {
    if db.get_database_backend() != DbBackend::Postgres {
        return Err(MigrationError);
    }
    let transaction = db.begin().await.map_err(|_| MigrationError)?;
    let result = async {
        transaction
            .execute_unprepared(
                // Explicit pg_temp placement prevents its implicit precedence;
                // leaving pg_catalog implicit keeps system functions first.
                "SET LOCAL search_path = public, pg_temp;
             SET LOCAL statement_timeout = '30s';
             SET LOCAL lock_timeout = '3s';
             SET LOCAL idle_in_transaction_session_timeout = '15s';",
            )
            .await?;
        let row = transaction
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT pg_catalog.current_setting('server_version_num')::integer >= 180000
             AND (pg_catalog.current_schema() IS NOT DISTINCT FROM 'public')
             AND pg_catalog.pg_try_advisory_xact_lock(1862384219, 1) AS allowed",
            ))
            .await?
            .ok_or_else(|| sea_orm_migration::DbErr::Custom("gate".into()))?;
        if !row.try_get::<bool>("", "allowed")? {
            return Err(sea_orm_migration::DbErr::Custom("gate".into()));
        }
        Migrator::up(&transaction, None).await
    }
    .await;
    if result.is_err() {
        let _ = transaction.rollback().await;
        return Err(MigrationError);
    }
    transaction.commit().await.map_err(|_| MigrationError)
}
