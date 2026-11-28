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
    verify_friend_invites(&operator).await;
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
        55
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

async fn assert_seaql_table_shape(operator: &sea_orm::DatabaseConnection) {
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
}

/// Migrations 000055 and 000054 undone, the newest first, for [`history_as_sea_orm_left_it`]
/// (000055 changes only tables this drops):
/// the runner tokens' tables and columns, and the one-session rule as 000007 left it.
async fn undo_runner_tokens(operator: &sea_orm::DatabaseConnection) {
    operator
        .execute_unprepared(
            "ALTER TABLE central_identity.sessions DROP COLUMN runner_token;
             DROP INDEX IF EXISTS central_identity.sessions_one_per_account;
             CREATE UNIQUE INDEX sessions_one_per_account ON central_identity.sessions(account_id, kind);
             DROP TABLE central_identity.ai_runner_signins, central_identity.ai_runner_accounts,
                        central_identity.ai_runner_tokens;
             ALTER TABLE central_identity.audit_events DROP COLUMN ai_runner_token_id;",
        )
        .await
        .unwrap();
}

/// The history as sea-orm-migration 2.0.4 left it, and a database one migration
/// behind: the runner reads the one and finishes the other.
async fn history_as_sea_orm_left_it(operator: &sea_orm::DatabaseConnection) {
    // The history has the shape sea-orm-migration 2.0.4 gave it, so a database
    // that crate migrated is read by this runner as it was written.
    assert_seaql_table_shape(operator).await;
    // Rebuild the suffix over the pre-presence schema. Older files would fail if run twice.
    undo_runner_tokens(operator).await;
    operator
        .execute_unprepared(
            "DROP TABLE central_identity.ai_matches, central_identity.ranked_ticket_ai,
                        central_identity.ai_offers, central_identity.ai_settings, central_identity.ai_players;
             DROP INDEX central_identity.gateway_sids_ai_match, central_identity.gateway_sids_one_each,
                        central_identity.gateway_sids_one_ranked;
             ALTER TABLE central_identity.gateway_sids DROP COLUMN ai_seat,
                 ADD UNIQUE (session_digest, stream, purpose);
             CREATE UNIQUE INDEX gateway_sids_one_ranked ON central_identity.gateway_sids(session_digest)
                 WHERE purpose='ranked';
             DROP TABLE central_identity.report_tokens;
             ALTER TABLE central_identity.reports DROP COLUMN filed_by;
             ALTER TABLE central_identity.report_notes DROP COLUMN token_name;
             ALTER TABLE central_identity.report_audit DROP COLUMN token_name;
             ALTER TABLE central_identity.audit_events DROP COLUMN report_token_id;
             DROP TABLE central_identity.friend_invites, central_identity.card_search_names, central_identity.card_printings;
             DROP TABLE central_identity.chat_group_pictures;
             ALTER TABLE central_identity.chat_members DROP COLUMN admin;
             ALTER TABLE central_identity.chat_groups DROP COLUMN description;
             DROP INDEX central_identity.gateways_origin_active;
             ALTER TABLE central_identity.gateways DROP COLUMN deleted_at, ADD UNIQUE (origin);
             ALTER TABLE central_identity.accounts DROP COLUMN first_name, DROP COLUMN last_name,
                 DROP COLUMN birth_date, DROP COLUMN public_profile;
             ALTER TABLE central_identity.sessions DROP CONSTRAINT session_lifetime,
                 DROP COLUMN refresh_digest, DROP COLUMN previous_refresh_digest,
                 DROP COLUMN refresh_expires_at, DROP COLUMN absolute_expires_at,
                 ADD CONSTRAINT session_lifetime CHECK (
                     expires_at > created_at
                     AND (proven_at IS NULL OR proven_at <= created_at)
                     AND expires_at <= COALESCE(proven_at, created_at) + interval '12 hours');
             ALTER TABLE central_identity.gateway_sids DROP COLUMN ranked_generation,
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
        55
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

async fn enroll_registered_and_login(
    store: &central_postgres::AccountStore,
    keys: &central_crypto::opaque::server::ServerKeys,
    name: &str,
    display_name: &str,
    invite_code: Option<&str>,
) -> (
    central_api::wire::Identity,
    central_api::Token,
    central_postgres::Concerns,
) {
    use central_api::{Endpoint, Token};
    use central_crypto::{
        Context,
        opaque::{Enrollment, Login},
        vault::{MasterKey, ObjectContext},
    };
    let ticket = Token::generate().unwrap();
    let identity = store
        .register_with_invite(name, Some(display_name), &ticket, invite_code)
        .await
        .unwrap();
    let issuer = Endpoint::development("http://127.0.0.1:28780")
        .unwrap()
        .issuer();
    let ctx = Context::new(issuer, identity.account, identity.epoch).unwrap();
    let (enrollment, request) = Enrollment::start("synthetic strong pass".into(), ctx).unwrap();
    let response = keys.enrollment_response(ctx, &request).unwrap();
    let enrolled = enrollment.finish(&response).unwrap();
    let master = MasterKey::generate().unwrap();
    let wrapped = enrolled.wrapping_key.wrap(&master).unwrap();
    let obj = ObjectContext::new(ctx, identity.account, 1).unwrap();
    let document = master.seal_object(obj, name.as_bytes()).unwrap();
    let concerns = store
        .enroll_concerning(
            &ticket,
            &enrolled.upload,
            wrapped.as_bytes(),
            &document,
            Some(wrapped.as_bytes()),
        )
        .await
        .unwrap();

    let cred = store.credential(identity).await.unwrap().unwrap();
    let record =
        central_crypto::opaque::server::ServerKeys::enrollment_record(&cred.record).unwrap();
    let (login, lreq) = Login::start("synthetic strong pass".into(), ctx).unwrap();
    let (attempt, lresp) = keys.start_login(ctx, Some(&record), &lreq).unwrap();
    let finished = login.finish(&lresp).unwrap();
    attempt.finish(&finished.finalization).unwrap();
    let tok = Token::generate().unwrap();
    store.issue_session(&cred, &tok).await.unwrap();
    (identity, tok, concerns)
}

struct InviteOnlyFixture<'a> {
    operator: &'a sea_orm::DatabaseConnection,
    store: &'a central_postgres::AccountStore,
    keys: &'a central_crypto::opaque::server::ServerKeys,
    inviter: central_api::wire::Identity,
    inviter_tok: &'a central_api::Token,
    other_tok: &'a central_api::Token,
}

async fn verify_invite_only_enrollment(
    fx: &InviteOnlyFixture<'_>,
    used_code: &str,
    active_code: &str,
) {
    use central_api::{
        Endpoint, Error, Token,
        social::{Relation, SocialRequest, SocialResponse},
    };
    use central_crypto::{
        Context,
        opaque::Enrollment,
        vault::{MasterKey, ObjectContext},
    };
    use central_postgres::Session;

    fx.operator
        .execute_unprepared("UPDATE central_identity.registration SET open = false")
        .await
        .unwrap();
    let closed_preview = fx.store.friend_invite_info(active_code).await.unwrap();
    assert!(!closed_preview.registration_open);
    assert!(closed_preview.invite_only);
    assert!(!fx.store.invite_can_register(used_code).await.unwrap());
    assert!(fx.store.invite_can_register(active_code).await.unwrap());

    let denied_ticket = Token::generate().unwrap();
    let denied = fx
        .store
        .register_with_invite(
            "invite_test_blocked",
            Some("InviteBlocked"),
            &denied_ticket,
            None,
        )
        .await;
    assert!(matches!(denied, Err(Error::Rejected)));

    let reg_ticket = Token::generate().unwrap();
    let newbie_id = fx
        .store
        .register_with_invite(
            "invite_test_newbie",
            Some("InviteNewbie"),
            &reg_ticket,
            Some(active_code),
        )
        .await
        .unwrap();
    // Pending registration reservation must not preview as Active or redeem twice.
    assert_eq!(
        fx.store.friend_invite_info(active_code).await.unwrap_err(),
        Error::Rejected
    );
    assert_eq!(
        fx.store
            .redeem_friend_invite(Session::Native(fx.other_tok), active_code, false, None)
            .await
            .unwrap_err(),
        Error::Rejected
    );

    let issuer = Endpoint::development("http://127.0.0.1:28780")
        .unwrap()
        .issuer();
    let ctx = Context::new(issuer, newbie_id.account, newbie_id.epoch).unwrap();
    let (enrollment, req) = Enrollment::start("synthetic strong pass".into(), ctx).unwrap();
    let resp = fx.keys.enrollment_response(ctx, &req).unwrap();
    let enrolled = enrollment.finish(&resp).unwrap();
    let master = MasterKey::generate().unwrap();
    let wrapped = enrolled.wrapping_key.wrap(&master).unwrap();
    let obj = ObjectContext::new(ctx, newbie_id.account, 1).unwrap();
    let doc = master.seal_object(obj, b"newbie").unwrap();
    let enroll_concerns = fx
        .store
        .enroll_concerning(
            &reg_ticket,
            &enrolled.upload,
            wrapped.as_bytes(),
            &doc,
            Some(wrapped.as_bytes()),
        )
        .await
        .unwrap();
    assert_eq!(
        enroll_concerns.pair,
        Some((fx.inviter.account, newbie_id.account))
    );

    let SocialResponse::Contacts {
        contacts: one_friends,
    } = fx
        .store
        .social(Session::Native(fx.inviter_tok), SocialRequest::Contacts)
        .await
        .unwrap()
    else {
        panic!("contacts expected");
    };
    assert!(
        one_friends
            .iter()
            .any(|c| c.person.display_name == "InviteNewbie" && c.relation == Relation::Friend)
    );

    fx.operator
        .execute_unprepared("UPDATE central_identity.registration SET open = true")
        .await
        .unwrap();
}

async fn verify_invite_limits_and_expiry(
    operator: &sea_orm::DatabaseConnection,
    store: &central_postgres::AccountStore,
    tok_one: &central_api::Token,
    tok_two: &central_api::Token,
) -> (Vec<[u8; 16]>, Vec<String>) {
    use central_api::{Error, social::FriendInviteStatus};
    use central_postgres::Session;

    let mut created_codes = Vec::new();
    let mut created_ids = Vec::new();
    for idx in 0..10 {
        let (summary, code, _) = store
            .create_friend_invite(
                Session::Native(tok_one),
                Some(&format!("Invite {idx}")),
                Some("Hello friend"),
                None,
                Some("en"),
                Some(7),
            )
            .await
            .unwrap();
        assert_eq!(summary.status, FriendInviteStatus::Active);
        created_ids.push(summary.id);
        created_codes.push(code.as_str().to_owned());
    }
    assert_eq!(
        store
            .create_friend_invite(
                Session::Native(tok_one),
                Some("Extra"),
                None,
                None,
                Some("en"),
                Some(7),
            )
            .await
            .unwrap_err(),
        Error::Busy
    );

    let raw_code = created_codes[0].as_str();
    let leaked_count = operator
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT count(*)::bigint AS value FROM central_identity.friend_invites WHERE encode(code_hash, 'escape') = $1",
            vec![raw_code.into()],
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<i64>("", "value")
        .unwrap();
    assert_eq!(leaked_count, 0);

    assert_eq!(
        store
            .list_friend_invites(Session::Native(tok_one))
            .await
            .unwrap()
            .len(),
        10
    );
    store
        .revoke_friend_invite(Session::Native(tok_one), created_ids[0])
        .await
        .unwrap();
    let revoked_info = store.friend_invite_info(raw_code).await.unwrap();
    assert_eq!(revoked_info.status, FriendInviteStatus::Revoked);
    assert!(revoked_info.note.is_none());
    assert!(revoked_info.invited_email.is_none());
    assert_eq!(
        store
            .revoke_friend_invite(Session::Native(tok_one), created_ids[0])
            .await
            .unwrap_err(),
        Error::Rejected
    );
    assert_eq!(
        store
            .redeem_friend_invite(
                Session::Native(tok_one),
                created_codes[1].as_str(),
                false,
                None,
            )
            .await
            .unwrap_err(),
        Error::Rejected
    );

    // Expired invite transitions cleanly to Expired and does not count as active.
    verify_invite_expiry(operator, store, tok_two, created_ids[9], &created_codes[9]).await;
    (created_ids, created_codes)
}

async fn verify_invite_expiry(
    operator: &sea_orm::DatabaseConnection,
    store: &central_postgres::AccountStore,
    tok_two: &central_api::Token,
    expired_id: [u8; 16],
    expired_code: &str,
) {
    use central_api::{Error, social::FriendInviteStatus};
    use central_postgres::Session;

    operator
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE central_identity.friend_invites
             SET created_at = pg_catalog.clock_timestamp() - interval '2 hours',
                 expires_at = pg_catalog.clock_timestamp() - interval '1 minute'
             WHERE id = $1",
            vec![sea_orm::prelude::Uuid::from_bytes(expired_id).into()],
        ))
        .await
        .unwrap();
    let expired_info = store.friend_invite_info(expired_code).await.unwrap();
    assert_eq!(expired_info.status, FriendInviteStatus::Expired);
    assert!(expired_info.note.is_none());
    assert!(expired_info.invited_email.is_none());
    assert_eq!(
        store
            .redeem_friend_invite(Session::Native(tok_two), expired_code, false, None)
            .await
            .unwrap_err(),
        Error::Rejected
    );
}

async fn verify_stale_session_invite_link_refused(
    operator: &sea_orm::DatabaseConnection,
    store: &central_postgres::AccountStore,
    keys: &central_crypto::opaque::server::ServerKeys,
    tok_one: &central_api::Token,
) {
    use central_postgres::Session;

    let (id_stale, tok_stale, _) =
        enroll_registered_and_login(store, keys, "invite_test_stale", "InviteStale", None).await;
    let (_stale_summary, stale_code, _) = store
        .create_friend_invite(
            Session::Native(tok_one),
            Some("Stale session test"),
            None,
            Some("stale-target@example.org"),
            Some("en"),
            Some(5),
        )
        .await
        .unwrap();
    operator
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE central_identity.sessions
             SET created_at = created_at - interval '6 minutes',
                 expires_at = expires_at - interval '6 minutes',
                 proven_at = NULL
             WHERE account_id = $1",
            vec![sea_orm::prelude::Uuid::from_bytes(id_stale.account).into()],
        ))
        .await
        .unwrap();
    let (_, stale_pending, _) = store
        .redeem_friend_invite(
            Session::Native(&tok_stale),
            stale_code.as_str(),
            true,
            Some("en"),
        )
        .await
        .unwrap();
    assert!(stale_pending.is_none());
    assert!(
        !store
            .save_invite_email_pending(
                Session::Native(&tok_stale),
                "stale-target@example.org",
                "en",
                [7; 32],
            )
            .await
            .unwrap()
    );
}

async fn verify_invite_email_linking(
    operator: &sea_orm::DatabaseConnection,
    store: &central_postgres::AccountStore,
    keys: &central_crypto::opaque::server::ServerKeys,
    tok_one: &central_api::Token,
    tok_two: &central_api::Token,
) -> (String, central_api::Token) {
    use central_api::social::{FriendInviteStatus, Relation, SocialRequest, SocialResponse};
    use central_postgres::Session;

    let (_, person_one) = store.social_caller(Session::Native(tok_one)).await.unwrap();
    let (email_summary, email_code, _) = store
        .create_friend_invite(
            Session::Native(tok_one),
            Some("With email"),
            Some("Join me"),
            Some("invited-two@example.org"),
            Some("de"),
            Some(5),
        )
        .await
        .unwrap();
    assert_eq!(email_summary.status, FriendInviteStatus::Active);
    // Unauthenticated preview must never expose invited_email PII.
    let active_info = store.friend_invite_info(email_code.as_str()).await.unwrap();
    assert_eq!(active_info.status, FriendInviteStatus::Active);
    assert!(active_info.invited_email.is_none());

    // A stale session (>5m since proven_at/created_at) may still redeem friendship
    // cleanly, but must never link an email onto the account (link_email=true ignored).
    verify_stale_session_invite_link_refused(operator, store, keys, tok_one).await;

    store
        .social(
            Session::Native(tok_two),
            SocialRequest::Contact {
                tag: person_one.tag,
                change: central_api::social::ContactAction::Request,
            },
        )
        .await
        .unwrap();
    let (redeemed_inviter, pending_email, concerns) = store
        .redeem_friend_invite(Session::Native(tok_two), email_code.as_str(), true, None)
        .await
        .unwrap();
    assert_eq!(redeemed_inviter.tag, person_one.tag);
    assert!(concerns.pair.is_some());
    assert_eq!(
        pending_email,
        Some(["invited-two@example.org".into(), "de".into()])
    );
    assert!(
        store
            .save_invite_email_pending(
                Session::Native(tok_two),
                "invited-two@example.org",
                "de",
                [9; 32],
            )
            .await
            .unwrap()
    );
    let SocialResponse::Contacts { contacts } = store
        .social(Session::Native(tok_two), SocialRequest::Contacts)
        .await
        .unwrap()
    else {
        panic!("contacts expected");
    };
    assert!(
        contacts
            .iter()
            .any(|c| c.person.tag == person_one.tag && c.relation == Relation::Friend)
    );
    let redeemed_info = store.friend_invite_info(email_code.as_str()).await.unwrap();
    assert_eq!(redeemed_info.status, FriendInviteStatus::Redeemed);
    assert!(redeemed_info.redeemed_by.is_none());
    assert!(redeemed_info.invited_email.is_none());
    let inviter_list = store
        .list_friend_invites(Session::Native(tok_one))
        .await
        .unwrap();
    let owner_row = inviter_list
        .iter()
        .find(|item| item.id == email_summary.id)
        .unwrap();
    assert!(owner_row.redeemed_by.is_some());
    assert_eq!(
        owner_row.invited_email.as_deref(),
        Some("invited-two@example.org")
    );

    let tok_three = verify_invite_preserves_existing_email(operator, store, keys, tok_one).await;
    (email_code.as_str().to_owned(), tok_three)
}

async fn verify_invite_preserves_existing_email(
    operator: &sea_orm::DatabaseConnection,
    store: &central_postgres::AccountStore,
    keys: &central_crypto::opaque::server::ServerKeys,
    tok_one: &central_api::Token,
) -> central_api::Token {
    use central_postgres::Session;

    // Confirmed email on an account must never be overwritten on link_email=true.
    let (id_three, tok_three, _) =
        enroll_registered_and_login(store, keys, "invite_test_three", "InviteThree", None).await;
    operator
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO central_identity.account_emails (account_id, address, lang) VALUES ($1, 'keep@example.org', 'en')",
            vec![sea_orm::prelude::Uuid::from_bytes(id_three.account).into()],
        ))
        .await
        .unwrap();
    let (_, code_three, _) = store
        .create_friend_invite(
            Session::Native(tok_one),
            None,
            None,
            Some("overwrite@example.org"),
            Some("en"),
            None,
        )
        .await
        .unwrap();
    let (_, pending_three, _) = store
        .redeem_friend_invite(
            Session::Native(&tok_three),
            code_three.as_str(),
            true,
            Some("en"),
        )
        .await
        .unwrap();
    assert!(pending_three.is_none());
    assert!(
        !store
            .save_invite_email_pending(
                Session::Native(&tok_three),
                "overwrite@example.org",
                "en",
                [8; 32],
            )
            .await
            .unwrap()
    );
    tok_three
}

async fn verify_friend_invites(operator: &sea_orm::DatabaseConnection) {
    use central_crypto::opaque::server::ServerKeys;
    use central_postgres::AccountStore;

    let store =
        AccountStore::connect_dev(config("central_runtime", "CENTRAL_TEST_RUNTIME_PASSWORD"))
            .await
            .unwrap();
    let keys = ServerKeys::generate();

    let (id_one, tok_one, c_one) =
        enroll_registered_and_login(&store, &keys, "invite_test_one", "InviteOne", None).await;
    assert!(c_one.pair.is_none());
    let (_id_two, tok_two, _) =
        enroll_registered_and_login(&store, &keys, "invite_test_two", "InviteTwo", None).await;

    let (_created_ids, created_codes) =
        verify_invite_limits_and_expiry(operator, &store, &tok_one, &tok_two).await;
    let (email_code, tok_three) =
        verify_invite_email_linking(operator, &store, &keys, &tok_one, &tok_two).await;

    verify_invite_only_enrollment(
        &InviteOnlyFixture {
            operator,
            store: &store,
            keys: &keys,
            inviter: id_one,
            inviter_tok: &tok_one,
            other_tok: &tok_three,
        },
        email_code.as_str(),
        created_codes[1].as_str(),
    )
    .await;
    store.close().await.unwrap();
}
