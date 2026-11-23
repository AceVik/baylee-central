//! Real isolated PostgreSQL fixture; invoke through `cargo xtask postgres`.
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
#[ignore = "requires the owned PostgreSQL fixture from `cargo xtask postgres --tower-awards-only`"]
async fn tower_awards() {
    let operator = connect_dev(config("central_migrator", "CENTRAL_TEST_MIGRATOR_PASSWORD"))
        .await
        .unwrap();
    central_migration::apply(&operator).await.unwrap();
    support::tower_awards(&operator).await;
    operator.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires the owned PostgreSQL container from `cargo xtask postgres --tower-only`"]
async fn tower_authority() {
    let operator = connect_dev(config("central_migrator", "CENTRAL_TEST_MIGRATOR_PASSWORD"))
        .await
        .unwrap();
    central_migration::apply(&operator).await.unwrap();
    support::tower(&operator).await;
    operator.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires the owned PostgreSQL container from `cargo xtask postgres`"]
async fn migrations_ids_roles_rollback_and_locking() {
    let operator = connect_dev(config("central_migrator", "CENTRAL_TEST_MIGRATOR_PASSWORD"))
        .await
        .unwrap();

    verify_migrations(&operator).await;
    support::upgrade(&operator).await;

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
    support::advent_provisioning::run(&operator).await;
    support::tls().await;
    store.close().await.unwrap();
    runtime.close().await.unwrap();
    operator.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires the isolated PostgreSQL staging fixture"]
async fn advent_staging_login_rolls_back_and_retries() {
    let operator = connect_dev(config("central_migrator", "CENTRAL_TEST_MIGRATOR_PASSWORD"))
        .await
        .unwrap();
    central_migration::apply(&operator).await.unwrap();
    support::advent_provisioning::run(&operator).await;
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
        46
    );

    history_as_sea_orm_left_it(operator).await;

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

/// The history as sea-orm-migration 2.0.4 left it, and a database one migration
/// behind: the runner reads the one and finishes the other.
async fn history_as_sea_orm_left_it(operator: &sea_orm::DatabaseConnection) {
    // The history has the shape sea-orm-migration 2.0.4 gave it, so a database
    // that crate migrated is read by this runner as it was written.
    assert_eq!(
        scalar(
            operator,
            "SELECT (pg_catalog.string_agg(column_name || ' ' || data_type || ' ' || is_nullable, ', '
                ORDER BY ordinal_position) = 'version character varying NO, applied_at bigint NO')
                ::integer::bigint AS value
             FROM information_schema.columns
             WHERE table_schema = 'public' AND table_name = 'seaql_migrations'"
        )
        .await,
        1
    );
    assert_eq!(
        scalar(
            operator,
            "SELECT count(*)::bigint AS value FROM pg_catalog.pg_constraint
             WHERE conrelid = 'public.seaql_migrations'::regclass AND contype = 'p'
               AND pg_catalog.pg_get_constraintdef(oid) = 'PRIMARY KEY (version)'"
        )
        .await,
        1
    );
    // Rebuild the suffix over the pre-presence schema. Older files would fail if run twice.
    operator
        .execute_unprepared(
            "ALTER TABLE central_identity.gateway_sids DROP COLUMN ranked_generation,
                 DROP COLUMN ranked_proof_until, DROP COLUMN ranked_match;
             DROP TRIGGER retire_advent_looks ON central_identity.ranked_matches;
             DROP FUNCTION central_identity.retire_advent_looks();
             DROP VIEW central_identity.cosmetic_collection;
             DROP TABLE central_identity.ranked_cosmetic_steps,central_identity.ranked_cosmetic_unlocks,central_identity.ranked_cosmetic_catalog,central_identity.cosmetic_equip_rights,
                        central_identity.advent_table_outfits, central_identity.advent_table_looks, central_identity.advent_ranked_looks, central_identity.advent_entitlements,
                        central_identity.advent_solutions, central_identity.advent_doors, central_identity.advent_campaigns, central_identity.playmat_framing, central_identity.advent_art;
             DROP TABLE central_identity.ranked_clocks, central_identity.ranked_commands,
                        central_identity.ranked_host_permissions, central_identity.ranked_host_aliases,
                        central_identity.ranked_witnesses, central_identity.ranked_replicas;
             DROP FUNCTION central_identity.ranked_seed_erasure();
             DROP INDEX central_identity.gateway_sids_one_ranked;
             DROP TABLE central_identity.ranked_badge_peaks, central_identity.ranked_champions,
                        central_identity.ranked_publications, central_identity.ranked_receipts, central_identity.ranked_deck_ratings,
                        central_identity.ranked_deck_versions, central_identity.ranked_ratings,
                        central_identity.ranked_tickets, central_identity.ranked_matches,
                        central_identity.ranked_hosts, central_identity.ranked_seasons;
             DROP TABLE central_identity.legacy_deck_transfers;
             ALTER TABLE central_identity.accounts DROP COLUMN legacy_deck_source;
             ALTER TABLE central_legacy.accounts DROP CONSTRAINT legacy_source_size,
                 ADD CONSTRAINT accounts_snapshot_check CHECK (octet_length(snapshot) BETWEEN 2 AND 196608),
                 ALTER COLUMN snapshot SET STORAGE EXTENDED;
             ALTER TABLE central_identity.gateway_sids DROP CONSTRAINT gateway_sids_purpose_check,
                 ADD CONSTRAINT gateway_sids_purpose_check CHECK (purpose IN ('admit','admin'));
             DROP FUNCTION central_identity.ranked_earned_badge();
             DROP FUNCTION central_identity.earn_ranked_cosmetics();
             ALTER TABLE central_identity.gateway_tokens DROP CONSTRAINT gateway_token_scope,
                 DROP COLUMN ranked, ADD CONSTRAINT gateway_token_scope CHECK (sso OR listing);
             DROP TRIGGER ranked_approval_changed ON central_identity.gateway_approvals;
             DROP FUNCTION central_identity.ranked_approval_changed();
             ALTER TABLE central_identity.gateway_approvals DROP COLUMN ranked_verified, DROP COLUMN ranked_epoch;
             ALTER TABLE central_identity.accounts DROP COLUMN presence;
             DELETE FROM public.seaql_migrations WHERE version >= 'm20261119_000033_presence'",
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
        46
    );
    assert_eq!(
        scalar(
            operator,
            "SELECT count(*)::bigint AS value FROM central_identity.accounts
             WHERE presence <> 'online'"
        )
        .await,
        0
    );
    assert_eq!(
        scalar(
            operator,
            "SELECT count(*)::bigint AS value FROM central_identity.donation_count"
        )
        .await,
        1
    );
}
