//! A prior-version upgrade: the exact migration SQL of 000001 to 000006 in a
//! scratch schema, legacy accounts and sessions written as those versions wrote
//! them, then 000007, then 000008 over a live session and its lease, then 000009
//! over registered gateways. All inside one transaction that is rolled back.
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
const RENEWAL: &str =
    include_str!("../../../central-migration/src/m20261022_000008_session_renewal.sql");
const OFFICIAL: &str =
    include_str!("../../../central-migration/src/m20261022_000009_official_gateway.sql");

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
    renewal(&tx).await;
    official(&tx).await;
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

/// 000008 over what 000007 left: the live session and its lease stay, both may
/// now live up to 12 hours after they were made, and no further.
async fn renewal(tx: &sea_orm::DatabaseTransaction) {
    tx.execute_unprepared(
        "INSERT INTO upgrade_probe.gateway_leases (digest, session_digest, audience, purpose, expires_at)
         VALUES (decode(repeat('08', 32), 'hex'), decode(repeat('06', 32), 'hex'), 'official', 'admit',
             clock_timestamp() + interval '29 minutes');",
    )
    .await
    .unwrap();
    tx.execute_unprepared(&scratch(RENEWAL)).await.unwrap();
    for (expected, sql) in [
        (
            2,
            "SELECT count(*)::bigint AS value FROM upgrade_probe.sessions",
        ),
        (
            1,
            "SELECT count(*)::bigint AS value FROM upgrade_probe.gateway_leases",
        ),
        (
            0,
            "SELECT count(*)::bigint AS value FROM pg_catalog.pg_constraint
             WHERE conrelid IN ('upgrade_probe.sessions'::regclass, 'upgrade_probe.gateway_leases'::regclass)
             AND pg_catalog.pg_get_constraintdef(oid) LIKE '%00:30:00%'",
        ),
    ] {
        assert_eq!(scalar(tx, sql).await, expected, "{sql}");
    }
    for table in ["sessions", "gateway_leases"] {
        tx.execute_unprepared(&format!(
            "UPDATE upgrade_probe.{table} SET expires_at = created_at + interval '11 hours 59 minutes'"
        ))
        .await
        .unwrap();
        tx.execute_unprepared("SAVEPOINT past_the_cap")
            .await
            .unwrap();
        assert!(
            tx.execute_unprepared(&format!(
                "UPDATE upgrade_probe.{table} SET expires_at = created_at + interval '12 hours 1 second'"
            ))
            .await
            .is_err(),
            "{table} outlives its 12 hours"
        );
        tx.execute_unprepared("ROLLBACK TO SAVEPOINT past_the_cap")
            .await
            .unwrap();
    }
}

/// `sql` fails, and the transaction goes on as before it.
async fn refused(tx: &sea_orm::DatabaseTransaction, sql: &str) {
    tx.execute_unprepared("SAVEPOINT refused").await.unwrap();
    assert!(tx.execute_unprepared(sql).await.is_err(), "{sql}");
    tx.execute_unprepared("ROLLBACK TO SAVEPOINT refused")
        .await
        .unwrap();
}

/// 000009 over what 000008 left: registered gateways, one approved and seen,
/// keep their owner and credentials and are not official; the prior binary's
/// registration still works; one official row at most, with no owner and no
/// credential, and no other row without them.
async fn official(tx: &sea_orm::DatabaseTransaction) {
    let owner = "(SELECT id FROM upgrade_probe.accounts WHERE legacy_id = 'legacy-1')";
    tx.execute_unprepared(&format!(
        "INSERT INTO upgrade_probe.gateways (owner_id, origin, name, version, website,
             credential_digest, heartbeat_digest, last_seen)
         VALUES ({owner}, 'https://approved.invalid', 'Approved', '1', 'https://approved.invalid/',
             decode(repeat('11', 32), 'hex'), decode(repeat('12', 32), 'hex'), clock_timestamp()),
             ({owner}, 'https://pending.invalid', 'Pending', '1', 'https://pending.invalid/',
             decode(repeat('13', 32), 'hex'), decode(repeat('14', 32), 'hex'), NULL);
         INSERT INTO upgrade_probe.gateway_approvals (gateway_id)
         SELECT id FROM upgrade_probe.gateways WHERE origin = 'https://approved.invalid';"
    ))
    .await
    .unwrap();
    tx.execute_unprepared(&scratch(OFFICIAL)).await.unwrap();
    for (expected, sql) in [
        (
            2,
            "SELECT count(*)::bigint AS value FROM upgrade_probe.gateways WHERE NOT official
             AND owner_id IS NOT NULL AND credential_digest IS NOT NULL AND heartbeat_digest IS NOT NULL",
        ),
        (
            1,
            "SELECT count(*)::bigint AS value FROM upgrade_probe.gateway_approvals p
             JOIN upgrade_probe.gateways g ON g.id = p.gateway_id
             WHERE p.enabled AND g.origin = 'https://approved.invalid' AND g.last_seen IS NOT NULL",
        ),
    ] {
        assert_eq!(scalar(tx, sql).await, expected, "{sql}");
    }
    // The prior binary registers without naming the column; the row is not official.
    tx.execute_unprepared(&format!(
        "INSERT INTO upgrade_probe.gateways (owner_id, origin, name, version, website,
             credential_digest, heartbeat_digest)
         VALUES ({owner}, 'https://later.invalid', 'Later', '1', 'https://later.invalid/',
             decode(repeat('15', 32), 'hex'), decode(repeat('16', 32), 'hex'))"
    ))
    .await
    .unwrap();
    // The operator's row (docs/operations.md §"The official gateway"), and its approval.
    let listed = "INSERT INTO upgrade_probe.gateways (official, origin, name, version, website)
        VALUES (true, 'https://baylee.acevik.de', 'Baylee', '0', 'https://baylee.acevik.de/')";
    tx.execute_unprepared(listed).await.unwrap();
    tx.execute_unprepared(
        "INSERT INTO upgrade_probe.gateway_approvals (gateway_id)
         SELECT id FROM upgrade_probe.gateways WHERE official",
    )
    .await
    .unwrap();
    refused(
        tx,
        "INSERT INTO upgrade_probe.gateways (official, origin, name, version, website)
         VALUES (true, 'https://second.invalid', 'Second', '0', 'https://second.invalid/')",
    )
    .await;
    for (column, value) in [
        ("owner_id", owner.to_owned()),
        (
            "credential_digest",
            "decode(repeat('17', 32), 'hex')".into(),
        ),
        ("heartbeat_digest", "decode(repeat('18', 32), 'hex')".into()),
    ] {
        refused(
            tx,
            &format!("UPDATE upgrade_probe.gateways SET {column} = {value} WHERE official"),
        )
        .await;
        refused(
            tx,
            &format!(
                "UPDATE upgrade_probe.gateways SET {column} = NULL WHERE origin = 'https://later.invalid'"
            ),
        )
        .await;
    }
    refused(
        tx,
        "UPDATE upgrade_probe.gateways SET official = true WHERE origin = 'https://later.invalid'",
    )
    .await;
    assert_eq!(
        scalar(
            tx,
            "SELECT count(*)::bigint AS value FROM upgrade_probe.gateways g
             JOIN upgrade_probe.gateway_approvals p ON p.gateway_id = g.id
             WHERE g.official AND g.owner_id IS NULL AND g.credential_digest IS NULL
             AND g.heartbeat_digest IS NULL AND g.last_seen IS NULL"
        )
        .await,
        1
    );
}
