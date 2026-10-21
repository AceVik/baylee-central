//! A prior-version upgrade: the exact migration SQL of 000001 to 000006 in a
//! scratch schema, legacy accounts and sessions written as those versions wrote
//! them, then 000007. All inside one transaction that is rolled back.
use super::scalar;
use sea_orm::{ConnectionTrait, DatabaseConnection, TransactionTrait};

const PRIOR: [&str; 6] = [
    include_str!("../../../central-migration/src/m20261019_000001_pending_accounts.sql"),
    include_str!("../../../central-migration/src/m20261019_000002_native_accounts.sql"),
    // 000003 keeps its SQL in Rust; this is that statement, unchanged.
    "ALTER TABLE central_identity.gateway_tickets ADD COLUMN purpose text NOT NULL DEFAULT 'admit'
     CHECK (purpose IN ('admit', 'export', 'link'));
     ALTER TABLE central_identity.gateway_tickets ALTER COLUMN purpose DROP DEFAULT;",
    include_str!("../../../central-migration/src/m20261019_000004_gateway_registry.sql"),
    include_str!("../../../central-migration/src/m20261019_000005_private_objects.sql"),
    include_str!("../../../central-migration/src/m20261020_000006_terms.sql"),
];
const REGISTRATION: &str =
    include_str!("../../../central-migration/src/m20261021_000007_registration.sql");

fn scratch(sql: &str) -> String {
    sql.replace("central_identity", "upgrade_probe")
}

pub(crate) async fn run(operator: &DatabaseConnection) {
    let tx = operator.begin().await.unwrap();
    for sql in PRIOR {
        tx.execute_unprepared(&scratch(sql)).await.unwrap();
    }
    // As the prior binary wrote them: an enrolled, closed migration with a live
    // session and a ticket, and an unfinished one.
    tx.execute_unprepared(
        "INSERT INTO upgrade_probe.accounts (legacy_id, username_key, credential, wrapped_master, document,
             document_revision, restored, legacy_closed)
         VALUES ('legacy-1', 'closed-player', '\\x01', decode(repeat('01', 73), 'hex'),
             decode(repeat('01', 41), 'hex'), 1, true, true);
         INSERT INTO upgrade_probe.accounts (legacy_id, username_key, enrollment_digest, enrollment_expires)
         VALUES ('legacy-2', 'pending-player', decode(repeat('02', 32), 'hex'), clock_timestamp() + interval '10 minutes');
         INSERT INTO upgrade_probe.sessions (digest, account_id, credential_revision, expires_at)
         SELECT decode(repeat('03', 32), 'hex'), id, 1, clock_timestamp() + interval '30 minutes'
         FROM upgrade_probe.accounts WHERE legacy_id = 'legacy-1';
         INSERT INTO upgrade_probe.gateway_tickets (digest, session_digest, audience, purpose, expires_at)
         VALUES (decode(repeat('04', 32), 'hex'), decode(repeat('03', 32), 'hex'), 'official', 'admit',
             clock_timestamp() + interval '20 seconds');",
    )
    .await
    .unwrap();
    tx.execute_unprepared(&scratch(REGISTRATION)).await.unwrap();
    for (expected, sql) in [
        (
            2,
            "SELECT count(*)::bigint AS value FROM upgrade_probe.accounts
             WHERE source = 'legacy' AND legacy_id IS NOT NULL AND recovery_wrapped_master IS NULL
             AND beta_key_id IS NULL",
        ),
        (
            1,
            "SELECT count(*)::bigint AS value FROM upgrade_probe.accounts WHERE legacy_closed AND restored",
        ),
        (
            1,
            "SELECT count(*)::bigint AS value FROM upgrade_probe.sessions WHERE kind = 'native'",
        ),
        (
            1,
            "SELECT count(*)::bigint AS value FROM upgrade_probe.gateway_tickets",
        ),
        (
            0,
            "SELECT count(*)::bigint AS value FROM upgrade_probe.beta_keys",
        ),
    ] {
        assert_eq!(scalar(&tx, sql).await, expected, "{sql}");
    }
    // The prior binary's statements still work: a legacy reserve without a source,
    // and a session without a kind, which is native.
    tx.execute_unprepared(
        "INSERT INTO upgrade_probe.accounts (legacy_id, username_key, enrollment_digest, enrollment_expires)
         VALUES ('legacy-3', 'later-player', decode(repeat('05', 32), 'hex'), clock_timestamp() + interval '10 minutes');
         DELETE FROM upgrade_probe.sessions WHERE account_id = (SELECT id FROM upgrade_probe.accounts WHERE legacy_id = 'legacy-1');
         INSERT INTO upgrade_probe.sessions (digest, account_id, credential_revision, expires_at)
         SELECT decode(repeat('06', 32), 'hex'), id, 1, clock_timestamp() + interval '30 minutes'
         FROM upgrade_probe.accounts WHERE legacy_id = 'legacy-1';
         INSERT INTO upgrade_probe.sessions (digest, account_id, credential_revision, kind, expires_at)
         SELECT decode(repeat('07', 32), 'hex'), id, 1, 'browser', clock_timestamp() + interval '30 minutes'
         FROM upgrade_probe.accounts WHERE legacy_id = 'legacy-1';",
    )
    .await
    .unwrap();
    assert_eq!(
        scalar(
            &tx,
            "SELECT count(*)::bigint AS value FROM upgrade_probe.sessions s
             JOIN upgrade_probe.accounts a ON a.id = s.account_id WHERE a.legacy_id = 'legacy-1'"
        )
        .await,
        2
    );
    tx.rollback().await.unwrap();
    assert_eq!(
        scalar(
            operator,
            "SELECT count(*)::bigint AS value FROM pg_catalog.pg_namespace WHERE nspname = 'upgrade_probe'"
        )
        .await,
        0
    );
}
