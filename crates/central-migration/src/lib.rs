//! Explicit versioned migrations. Never run automatically from the HTTP service.
//! The caller supplies a dedicated DDL connection, with trusted TLS outside fixtures.
//!
//! Each migration is one SQL file, applied in the order of [`MIGRATIONS`] and
//! recorded in `public.seaql_migrations` under its name. Until 18.11.2026 the
//! `sea-orm-migration` crate ran them; this runner replaced it (it brought
//! `chrono` and every default feature of `SeaORM` into the service) and keeps its
//! history table as that crate created it, so a database it migrated is migrated
//! on by this one without a change. No destructive operation exists: there is no
//! `down`, `fresh` or `reset`, and the list is append-only.
//!
//! ```compile_fail
//! use central_migration::Migrator;
//! ```
#![forbid(unsafe_code)]

use sea_orm::{
    ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, DbErr, Statement,
    TransactionTrait,
};

/// One migration: its name in the history, which is also its file's, and its SQL.
struct Migration {
    name: &'static str,
    sql: &'static str,
}

/// Declares the ordered, append-only list from the files' names alone, so that a
/// name and its SQL cannot drift apart.
macro_rules! migrations {
    ($($name:literal),+ $(,)?) => {
        [$(Migration { name: $name, sql: include_str!(concat!($name, ".sql")) }),+]
    };
}

/// Every migration, in the order they are applied. Append only: a database records
/// the names it ran, and one it ran is never run again or removed.
const MIGRATIONS: [Migration; 37] = migrations![
    "m20261019_000001_pending_accounts",
    "m20261019_000002_native_accounts",
    "m20261019_000003_ticket_purpose",
    "m20261019_000004_gateway_registry",
    "m20261019_000005_private_objects",
    "m20261020_000006_terms",
    "m20261021_000007_registration",
    "m20261022_000008_session_renewal",
    "m20261022_000009_official_gateway",
    "m20261022_000010_legacy_recovery",
    "m20261023_000011_profiles",
    "m20261028_000012_administration",
    "m20261028_000013_reset",
    "m20261030_000014_gateway_tokens",
    "m20261030_000015_legacy_store",
    "m20261030_000016_pairing",
    "m20261031_000018_reports",
    "m20261102_000019_social",
    "m20261102_000020_chat",
    "m20261102_000021_groups",
    "m20261102_000022_notifications",
    "m20261111_000023_open_registration",
    "m20261111_000024_session_tokens",
    "m20261111_000025_gateway_activity",
    "m20261112_000026_email",
    "m20261112_000027_pictures",
    "m20261113_000028_challenges",
    "m20261113_000029_tower",
    "m20261114_000030_tower_access",
    "m20261115_000031_tower_awards",
    "m20261118_000032_donations",
    "m20261119_000033_presence",
    "m20261122_000034_ranked_hosts",
    "m20261122_000035_ranked_queue",
    "m20261122_000036_ranked_scores",
    "m20261122_000037_ranked_boards",
    "m20261122_000038_ranked_token_scope",
];

/// The history table exactly as `sea-orm-migration` 2.0.4 created it
/// (`Schema::create_table_from_entity` of its `seaql_migrations` entity, read
/// 18.11.2026), qualified with the trusted schema.
const HISTORY: &str = "CREATE TABLE IF NOT EXISTS public.\"seaql_migrations\" ( \"version\" varchar NOT NULL PRIMARY KEY, \"applied_at\" bigint NOT NULL )";

/// Generic operator-facing error; database values and credentials are not exposed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MigrationError;
impl std::fmt::Display for MigrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("database migration rejected")
    }
}
impl std::error::Error for MigrationError {}

/// The migrations still to run, given the names a database recorded in their
/// order: those after the recorded ones. `None` when the record is not exactly the
/// list's beginning (a name this build does not know, one missing before another,
/// or another order), which no run of this list can leave.
fn pending<'a>(recorded: &[String], migrations: &'a [Migration]) -> Option<&'a [Migration]> {
    if recorded.len() > migrations.len()
        || recorded
            .iter()
            .zip(migrations)
            .any(|(name, migration)| name != migration.name)
    {
        return None;
    }
    Some(&migrations[recorded.len()..])
}

fn statement(sql: &str, values: Vec<sea_orm::Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, values)
}

/// Inside the outer transaction, after the gate: the history installed if it is
/// not there yet, read, checked, and every pending migration run and recorded.
async fn run(transaction: &DatabaseTransaction) -> Result<(), DbErr> {
    transaction.execute_unprepared(HISTORY).await?;
    let recorded: Vec<String> = transaction
        .query_all_raw(statement(
            "SELECT \"version\" FROM public.\"seaql_migrations\" ORDER BY \"version\" ASC",
            vec![],
        ))
        .await?
        .iter()
        .map(|row| row.try_get::<String>("", "version"))
        .collect::<Result<_, _>>()?;
    let pending = pending(&recorded, &MIGRATIONS)
        .ok_or_else(|| DbErr::Custom("unknown migration history".into()))?;
    for migration in pending {
        transaction.execute_unprepared(migration.sql).await?;
        transaction
            .execute_raw(statement(
                "INSERT INTO public.\"seaql_migrations\" (\"version\", \"applied_at\")
                 VALUES ($1, floor(extract(epoch FROM pg_catalog.clock_timestamp()))::bigint)",
                vec![migration.name.into()],
            ))
            .await?;
    }
    Ok(())
}

/// Apply all pending migrations atomically under a transaction-scoped advisory lock.
/// PostgreSQL 18+ is required. A failed attempt rolls back DDL and migration history.
/// Destructive down/fresh/reset operations do not exist.
/// History lives in the trusted `public` schema, independently of caller settings.
/// A missing or inaccessible `public` schema is rejected instead of selecting TEMP.
///
/// # Errors
///
/// Rejects unsupported backends/versions, concurrent migration, an unknown history,
/// failed DDL or commit.
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
            .ok_or_else(|| DbErr::Custom("gate".into()))?;
        if !row.try_get::<bool>("", "allowed")? {
            return Err(DbErr::Custom("gate".into()));
        }
        run(&transaction).await
    }
    .await;
    if result.is_err() {
        let _ = transaction.rollback().await;
        return Err(MigrationError);
    }
    transaction.commit().await.map_err(|_| MigrationError)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recorded(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn the_list_is_ordered_unique_and_shaped() {
        assert_eq!(MIGRATIONS.len(), 37);
        for pair in MIGRATIONS.windows(2) {
            // the history is read in this order, so the list must be in it too
            assert!(pair[0].name < pair[1].name, "{}", pair[1].name);
        }
        for migration in &MIGRATIONS {
            let bytes = migration.name.as_bytes();
            assert!(
                bytes.len() > 18
                    && bytes[0] == b'm'
                    && bytes[1..9].iter().all(u8::is_ascii_digit)
                    && bytes[9] == b'_'
                    && bytes[10..16].iter().all(u8::is_ascii_digit)
                    && bytes[16] == b'_',
                "{}",
                migration.name
            );
            assert!(!migration.sql.trim().is_empty(), "{}", migration.name);
        }
    }

    #[test]
    fn the_history_is_the_one_sea_orm_migration_created() {
        // As `sea-orm-migration` 2.0.4 wrote it (unqualified; this runner names
        // the trusted schema).
        let captured = "CREATE TABLE IF NOT EXISTS \"seaql_migrations\" ( \"version\" varchar NOT NULL PRIMARY KEY, \"applied_at\" bigint NOT NULL )";
        assert_eq!(HISTORY.replace("public.", ""), captured);
    }

    #[test]
    fn only_the_list_s_beginning_is_a_history() {
        let all: Vec<&str> = MIGRATIONS.iter().map(|m| m.name).collect();
        assert_eq!(pending(&[], &MIGRATIONS).map(<[_]>::len), Some(37));
        assert_eq!(
            pending(&recorded(&all[..31]), &MIGRATIONS).map(|rest| rest[0].name),
            Some("m20261119_000033_presence")
        );
        assert_eq!(
            pending(&recorded(&all), &MIGRATIONS).map(<[_]>::len),
            Some(0)
        );
        // a name this build does not know, after all or among them
        let mut longer = recorded(&all);
        longer.push("unexpected_future_version".into());
        assert!(pending(&longer, &MIGRATIONS).is_none());
        let mut foreign = recorded(&all[..3]);
        foreign[1] = "m20261019_000002_something_else".into();
        assert!(pending(&foreign, &MIGRATIONS).is_none());
        // a gap: a later one recorded without an earlier
        let gap = recorded(&[all[0], all[2]]);
        assert!(pending(&gap, &MIGRATIONS).is_none());
    }
}
