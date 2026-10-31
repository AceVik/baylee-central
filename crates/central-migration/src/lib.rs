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
mod m20261019_000002_native_accounts;
mod m20261019_000003_ticket_purpose;
mod m20261019_000004_gateway_registry;
mod m20261019_000005_private_objects;
mod m20261020_000006_terms;
mod m20261021_000007_registration;
mod m20261022_000008_session_renewal;
mod m20261022_000009_official_gateway;
mod m20261022_000010_legacy_recovery;
mod m20261023_000011_profiles;
mod m20261028_000012_administration;
mod m20261028_000013_reset;
mod m20261030_000014_gateway_tokens;
mod m20261030_000015_legacy_store;

use sea_orm_migration::{
    MigrationTrait, MigratorTrait,
    sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, TransactionTrait},
};

// Ordered, append-only registry; never expose SeaORM's destructive convenience API.
struct Migrator;

#[sea_orm_migration::async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20261019_000001_pending_accounts::Migration),
            Box::new(m20261019_000002_native_accounts::Migration),
            Box::new(m20261019_000003_ticket_purpose::Migration),
            Box::new(m20261019_000004_gateway_registry::Migration),
            Box::new(m20261019_000005_private_objects::Migration),
            Box::new(m20261020_000006_terms::Migration),
            Box::new(m20261021_000007_registration::Migration),
            Box::new(m20261022_000008_session_renewal::Migration),
            Box::new(m20261022_000009_official_gateway::Migration),
            Box::new(m20261022_000010_legacy_recovery::Migration),
            Box::new(m20261023_000011_profiles::Migration),
            Box::new(m20261028_000012_administration::Migration),
            Box::new(m20261028_000013_reset::Migration),
            Box::new(m20261030_000014_gateway_tokens::Migration),
            Box::new(m20261030_000015_legacy_store::Migration),
        ]
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
