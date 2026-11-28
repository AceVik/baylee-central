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
const MIGRATIONS: [Migration; 56] = migrations![
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
    "m20261122_000039_legacy_deck_transfers",
    "m20261122_000040_ranked_seat_purpose",
    "m20261123_000041_ranked_replication",
    "m20261123_000042_ranked_clocks",
    "m20261123_000043_advent_entitlements",
    "m20261123_000044_advent_casual_looks",
    "m20261123_000045_advent_casual_recovery",
    "m20261123_000046_playmat_framing",
    "m20261123_000047_ranked_cosmetics",
    "m20261123_000048_web_ux_sessions_profile_public",
    "m20261124_000049_group_profiles_and_admins",
    "m20261124_000050_card_catalog",
    "m20261124_000051_friend_invites",
    "m20261125_000052_report_tokens",
    "m20261126_000053_ai_players",
    "m20261126_000054_ai_runner_tokens",
    "m20261127_000055_ai_operator_fill_in",
    "m20261128_000056_ranked_season_finish_awards",
    "m20261128_000057_ai_global_seats",
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
        assert_eq!(MIGRATIONS.len(), 56);
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
        assert_eq!(pending(&[], &MIGRATIONS).map(<[_]>::len), Some(56));
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

    #[test]
    fn friend_invites_migration_enforces_hashing_uuidv7_and_revokes_public() {
        let m = MIGRATIONS
            .iter()
            .find(|m| m.name == "m20261124_000051_friend_invites")
            .expect("friend invites migration is present");
        assert!(
            m.sql
                .contains("CREATE TABLE central_identity.friend_invites")
        );
        assert!(m.sql.contains("pg_catalog.uuidv7()"));
        assert!(m.sql.contains("pg_catalog.uuid_extract_version(id) = 7"));
        assert!(m.sql.contains("code_hash"));
        assert!(m.sql.contains("redeemed_by <> inviter_id"));
        assert!(
            m.sql
                .contains("REVOKE ALL ON TABLE central_identity.friend_invites FROM PUBLIC")
        );
    }
    /// A report token is stored by its digest alone, its rights' needs are
    /// the table's, and what it files or writes says so.
    #[test]
    fn report_tokens_migration_keeps_digests_and_marks_what_a_token_did() {
        let m = MIGRATIONS
            .iter()
            .find(|m| m.name == "m20261125_000052_report_tokens")
            .expect("report tokens migration is present");
        for said in [
            "CREATE TABLE central_identity.report_tokens",
            "digest bytea NOT NULL UNIQUE CHECK (octet_length(digest) = 32)",
            "(read OR NOT triage) AND (triage OR NOT purge)",
            "expires_at <= created_at + interval '366 days'",
            "REVOKE ALL ON central_identity.report_tokens FROM PUBLIC",
            "ADD COLUMN filed_by text NOT NULL DEFAULT 'gateway'",
            "record_origin IS DISTINCT FROM 'gateway'",
            "ADD COLUMN report_token_id uuid",
        ] {
            assert!(m.sql.contains(said), "{said}");
        }
    }

    /// AI players live in tables of their own; of the existing tables only
    /// `gateway_sids` changes, and only for rows marked as an AI seat.
    #[test]
    fn ai_players_migration_keeps_people_s_sid_rules() {
        let m = MIGRATIONS
            .iter()
            .find(|m| m.name == "m20261126_000053_ai_players")
            .expect("AI players migration is present");
        for said in [
            "CREATE TABLE central_identity.ai_players",
            "CONSTRAINT ai_model_capped CHECK (kind <> 'model' OR daily_games IS NOT NULL)",
            "CONSTRAINT ai_fair_only CHECK (profile <> 'house:seer')",
            "max_games smallint NOT NULL DEFAULT 1 CHECK (max_games BETWEEN 1 AND 16)",
            "global_seats smallint NOT NULL DEFAULT 8 CHECK (global_seats BETWEEN 0 AND 16)",
            "INSERT INTO central_identity.ai_settings (singleton) VALUES (true)",
            "REFERENCES central_identity.sessions(digest) ON DELETE CASCADE",
            "REFERENCES central_identity.ranked_tickets(account_id, request_id) ON DELETE CASCADE",
            "ADD COLUMN ai_seat boolean NOT NULL DEFAULT false",
            "CHECK (NOT ai_seat OR purpose = 'ranked')",
            "WHERE purpose = 'ranked' AND NOT ai_seat",
            "(session_digest, ranked_match)\n    WHERE ai_seat",
        ] {
            assert!(m.sql.contains(said), "{said}");
        }
        for table in [
            "ai_players",
            "ai_settings",
            "ai_offers",
            "ranked_ticket_ai",
            "ai_matches",
        ] {
            let revoke = format!("REVOKE ALL ON central_identity.{table} FROM PUBLIC");
            assert!(m.sql.contains(&revoke), "{table}");
        }
        // No existing table gains or loses a grant here.
        assert!(!m.sql.contains("GRANT"));
    }

    /// A runner token is stored by its digest alone; a session it minted
    /// names it and goes with it; a person's sessions, and every row a binary
    /// from before writes, keep the one-session rule.
    #[test]
    fn ai_runner_tokens_migration_keeps_people_s_one_session_rule() {
        let m = MIGRATIONS
            .iter()
            .find(|m| m.name == "m20261126_000054_ai_runner_tokens")
            .expect("runner tokens migration is present");
        for said in [
            "CREATE TABLE central_identity.ai_runner_tokens",
            "pg_catalog.uuid_extract_version(id) IS NOT DISTINCT FROM 7",
            "digest bytea NOT NULL UNIQUE CHECK (octet_length(digest) = 32)",
            "name text NOT NULL CHECK (octet_length(name) BETWEEN 1 AND 64)",
            "expires_at <= created_at + interval '366 days'",
            "(revoked_by IS NULL OR revoked_at IS NOT NULL)",
            "REFERENCES central_identity.ai_players(account_id) ON DELETE CASCADE",
            "PRIMARY KEY (token_id, account_id)",
            "ADD COLUMN runner_token uuid",
            "CHECK (runner_token IS NULL OR kind = 'native')",
            "FOREIGN KEY (runner_token, account_id)\n        REFERENCES central_identity.ai_runner_accounts(token_id, account_id) ON DELETE CASCADE",
            "DROP INDEX central_identity.sessions_one_per_account",
            "sessions(account_id, kind)\n    WHERE runner_token IS NULL",
            "ADD COLUMN ai_runner_token_id uuid",
        ] {
            assert!(m.sql.contains(said), "{said}");
        }
        for table in [
            "ai_runner_tokens",
            "ai_runner_accounts",
            "ai_runner_signins",
        ] {
            let revoke = format!("REVOKE ALL ON central_identity.{table} FROM PUBLIC");
            assert!(m.sql.contains(&revoke), "{table}");
        }
        // No existing table gains or loses a grant here: sessions and the
        // audit have table-wide grants that cover a new column.
        assert!(!m.sql.contains("GRANT"));
    }

    /// After 000053 and 000054 ran on test stacks, what the operator's command
    /// and calibration need comes as a migration of its own: an operator's
    /// token without a maker, and fill-in's switch, on by default.
    #[test]
    fn ai_operator_migration_only_relaxes_the_maker_and_adds_the_switch() {
        let m = MIGRATIONS
            .iter()
            .find(|m| m.name == "m20261127_000055_ai_operator_fill_in")
            .expect("AI operator migration is present");
        for said in [
            "ALTER TABLE central_identity.ai_runner_tokens ALTER COLUMN created_by DROP NOT NULL",
            "ALTER TABLE central_identity.ai_settings ADD COLUMN fill_in boolean NOT NULL DEFAULT true",
        ] {
            assert!(m.sql.contains(said), "{said}");
        }
        // The column's grant is ops/grants.sql's, as every grant is.
        assert!(!m.sql.contains("GRANT"));
        assert!(!m.sql.contains("DROP TABLE"));
    }

    /// A season may close without finish awards (the pre-season before
    /// Season 0): one column, on unless an operator sets it off, and
    /// nothing else.
    #[test]
    fn season_finish_awards_migration_only_adds_the_switch() {
        let m = MIGRATIONS
            .iter()
            .find(|m| m.name == "m20261128_000056_ranked_season_finish_awards")
            .expect("season finish awards migration is present");
        assert!(m.sql.contains(
            "ALTER TABLE central_identity.ranked_seasons\n    ADD COLUMN finish_awards boolean NOT NULL DEFAULT true"
        ));
        assert!(!m.sql.contains("GRANT"));
        assert!(!m.sql.contains("DROP"));
        assert!(!m.sql.contains("UPDATE"));
    }

    /// More AI seats at once: the two CHECKs of the settings are widened, to
    /// 32 seats and 16 matches between two AIs, under the names 000053 gave
    /// them, and nothing else changes: no row, no grant, no table.
    #[test]
    fn ai_global_seats_migration_only_widens_the_two_checks() {
        let m = MIGRATIONS.last().expect("AI seats migration is present");
        assert_eq!(m.name, "m20261128_000057_ai_global_seats");
        for said in [
            "DROP CONSTRAINT ai_settings_global_seats_check",
            "ADD CONSTRAINT ai_settings_global_seats_check CHECK (global_seats BETWEEN 0 AND 32)",
            "DROP CONSTRAINT ai_settings_carousel_games_check",
            "ADD CONSTRAINT ai_settings_carousel_games_check CHECK (carousel_games BETWEEN 0 AND 16)",
        ] {
            assert!(m.sql.contains(said), "{said}");
        }
        let statements = m
            .sql
            .lines()
            .filter(|line| !line.trim_start().starts_with("--"))
            .collect::<String>();
        assert_eq!(statements.matches(';').count(), 1, "one statement");
        assert!(statements.starts_with("ALTER TABLE central_identity.ai_settings"));
        for absent in [
            "GRANT",
            "UPDATE",
            "INSERT",
            "DELETE",
            "DROP TABLE",
            "DEFAULT",
        ] {
            assert!(!statements.contains(absent), "{absent}");
        }
    }
}
