//! A prior-version upgrade: the exact migration SQL of 000001 to 000006 in a
//! scratch schema, legacy accounts and sessions written as those versions wrote
//! them, then 000007, then 000008 over a live session and its lease, then 000009
//! over registered gateways, then 000010 over legacy and registered
//! accounts, then 000011 over the same, drawing the registered accounts'
//! tags, then 000012's administration over all of it, then 000013's reset
//! codes, then 000014's gateway tokens over the registered gateways. All
//! inside one transaction that is rolled back.
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
const LEGACY_RECOVERY: &str =
    include_str!("../../../central-migration/src/m20261022_000010_legacy_recovery.sql");
const PROFILES: &str = include_str!("../../../central-migration/src/m20261023_000011_profiles.sql");
const ADMINISTRATION: &str =
    include_str!("../../../central-migration/src/m20261028_000012_administration.sql");
const RESET: &str = include_str!("../../../central-migration/src/m20261028_000013_reset.sql");
const TOKENS: &str =
    include_str!("../../../central-migration/src/m20261030_000014_gateway_tokens.sql");

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
    legacy_recovery(&tx).await;
    profiles(&tx).await;
    administration(&tx).await;
    reset(&tx).await;
    tokens(&tx).await;
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

/// 000010 over what came before: every row keeps its wrapper or its lack of
/// one; then an enrolled legacy account may take a recovery wrapper, while a
/// pending account still may not hold one and a registered account may not
/// lose its own.
async fn legacy_recovery(tx: &sea_orm::DatabaseTransaction) {
    // As the binary before it wrote one: a registered account with its wrapper.
    tx.execute_unprepared(
        "INSERT INTO upgrade_probe.accounts (source, username_key, credential, wrapped_master, document,
             document_revision, recovery_wrapped_master)
         VALUES ('registered', 'registered-player', '\\x01', decode(repeat('01', 73), 'hex'),
             decode(repeat('01', 41), 'hex'), 1, decode(repeat('02', 73), 'hex'));",
    )
    .await
    .unwrap();
    tx.execute_unprepared(&scratch(LEGACY_RECOVERY))
        .await
        .unwrap();
    for (expected, sql) in [
        (
            3,
            "SELECT count(*)::bigint AS value FROM upgrade_probe.accounts
             WHERE source = 'legacy' AND recovery_wrapped_master IS NULL",
        ),
        (
            1,
            "SELECT count(*)::bigint AS value FROM upgrade_probe.accounts
             WHERE source = 'registered' AND recovery_wrapped_master = decode(repeat('02', 73), 'hex')",
        ),
        (
            1,
            "SELECT count(*)::bigint AS value FROM pg_catalog.pg_constraint
             WHERE conrelid = 'upgrade_probe.accounts'::regclass AND conname = 'account_recovery'",
        ),
    ] {
        assert_eq!(scalar(tx, sql).await, expected, "{sql}");
    }
    // The closed, enrolled legacy account takes a wrapper now.
    tx.execute_unprepared(
        "UPDATE upgrade_probe.accounts SET recovery_wrapped_master = decode(repeat('03', 73), 'hex')
         WHERE legacy_id = 'legacy-1'",
    )
    .await
    .unwrap();
    for sql in [
        "UPDATE upgrade_probe.accounts SET recovery_wrapped_master = decode(repeat('03', 73), 'hex')
         WHERE legacy_id = 'legacy-2'",
        "UPDATE upgrade_probe.accounts SET recovery_wrapped_master = NULL WHERE source = 'registered'",
        "UPDATE upgrade_probe.accounts SET recovery_wrapped_master = decode(repeat('03', 72), 'hex')
         WHERE legacy_id = 'legacy-1'",
        // a credential's removal must take the wrapper with it
        "UPDATE upgrade_probe.accounts SET credential = NULL, wrapped_master = NULL, document = NULL,
         document_revision = 0, restored = false, legacy_closed = false WHERE legacy_id = 'legacy-1'",
    ] {
        refused(tx, sql).await;
    }
}

/// 000011 over what came before: every registered account, pending or
/// enrolled, draws a distinct tag at or above the floor, of four hex digits;
/// legacy accounts stay without name and tag; the previous binary still
/// registers (without a tag); the schema holds the name's rule, its time and
/// the tag's.
async fn profiles(tx: &sea_orm::DatabaseTransaction) {
    use central_api::profile::REGISTERED_TAG_FLOOR;
    // As the binary before it wrote one: a pending registration with its key.
    tx.execute_unprepared(
        "INSERT INTO upgrade_probe.beta_keys (digest, uses_left, note)
         VALUES (decode(repeat('21', 32), 'hex'), 1, 'upgrade');
         INSERT INTO upgrade_probe.accounts (source, username_key, enrollment_digest, enrollment_expires, beta_key_id)
         SELECT 'registered', 'pending-registered', decode(repeat('22', 32), 'hex'),
             clock_timestamp() + interval '10 minutes', id FROM upgrade_probe.beta_keys;",
    )
    .await
    .unwrap();
    tx.execute_unprepared(&scratch(PROFILES)).await.unwrap();
    for (expected, sql) in [
        (
            2,
            format!(
                "SELECT count(DISTINCT tag)::bigint AS value FROM upgrade_probe.accounts
                 WHERE source = 'registered' AND tag BETWEEN {REGISTERED_TAG_FLOOR} AND 65535
                 AND display_name IS NULL AND display_name_at IS NULL"
            ),
        ),
        (
            0,
            "SELECT count(*)::bigint AS value FROM upgrade_probe.accounts
             WHERE source = 'legacy' AND (tag IS NOT NULL OR display_name IS NOT NULL)"
                .into(),
        ),
        (
            3,
            "SELECT count(*)::bigint AS value FROM upgrade_probe.accounts WHERE source = 'legacy'"
                .into(),
        ),
    ] {
        assert_eq!(scalar(tx, &sql).await, expected, "{sql}");
    }
    // The previous binary registers without a tag; the new one draws it at the sign-in.
    tx.execute_unprepared(
        "INSERT INTO upgrade_probe.beta_keys (digest, uses_left, note)
         VALUES (decode(repeat('23', 32), 'hex'), 1, 'upgrade');
         INSERT INTO upgrade_probe.accounts (source, username_key, enrollment_digest, enrollment_expires, beta_key_id)
         SELECT 'registered', 'later-registered', decode(repeat('24', 32), 'hex'),
             clock_timestamp() + interval '10 minutes', id FROM upgrade_probe.beta_keys
             WHERE digest = decode(repeat('23', 32), 'hex');",
    )
    .await
    .unwrap();
    let account = "WHERE legacy_id = 'legacy-1'";
    for sql in [
        // the name's rule, exactly the gateway's
        format!(
            "UPDATE upgrade_probe.accounts SET display_name = '_Alice', display_name_at = now() {account}"
        ),
        format!(
            "UPDATE upgrade_probe.accounts SET display_name = 'Alice-', display_name_at = now() {account}"
        ),
        format!(
            "UPDATE upgrade_probe.accounts SET display_name = 'ab', display_name_at = now() {account}"
        ),
        format!(
            "UPDATE upgrade_probe.accounts SET display_name = 'seventeen_chars_x', display_name_at = now() {account}"
        ),
        format!(
            "UPDATE upgrade_probe.accounts SET display_name = 'Zoë', display_name_at = now() {account}"
        ),
        format!(
            "UPDATE upgrade_probe.accounts SET display_name = 'Alice#af03', display_name_at = now() {account}"
        ),
        format!(
            "UPDATE upgrade_probe.accounts SET display_name = E'Alice\n', display_name_at = now() {account}"
        ),
        // a name and its time together
        format!("UPDATE upgrade_probe.accounts SET display_name = 'Alice' {account}"),
        format!("UPDATE upgrade_probe.accounts SET display_name_at = now() {account}"),
        // a positive tag, held once
        format!("UPDATE upgrade_probe.accounts SET tag = 0 {account}"),
        format!("UPDATE upgrade_probe.accounts SET tag = -7 {account}"),
        format!(
            "UPDATE upgrade_probe.accounts SET tag = (SELECT tag FROM upgrade_probe.accounts
             WHERE username_key = 'registered-player') {account}"
        ),
    ] {
        refused(tx, &sql).await;
    }
    tx.execute_unprepared(&format!(
        "UPDATE upgrade_probe.accounts SET display_name = 'player_one-99', display_name_at = now(), tag = 13 {account};
         UPDATE upgrade_probe.accounts SET display_name = 'Sixteen_chars_yz', display_name_at = now() WHERE legacy_id = 'legacy-2';"
    ))
    .await
    .unwrap();
    widening(tx).await;
}

/// The backfill again, over a half-taken first range: with half of the
/// four-digit tags taken (by legacy rows, as any account counts), the
/// previous binary's later registration draws a tag of five digits.
async fn widening(tx: &sea_orm::DatabaseTransaction) {
    use central_api::profile::REGISTERED_TAG_FLOOR;
    tx.execute_unprepared(&format!(
        "INSERT INTO upgrade_probe.accounts (legacy_id, username_key, enrollment_digest, enrollment_expires, tag)
         SELECT 'fill-' || n, 'fill-' || n, sha256(convert_to('fill-' || n, 'UTF8')),
             clock_timestamp() + interval '10 minutes', n
         FROM generate_series({REGISTERED_TAG_FLOOR}, {REGISTERED_TAG_FLOOR} + 30720) n
         WHERE NOT EXISTS (SELECT 1 FROM upgrade_probe.accounts a WHERE a.tag = n);"
    ))
    .await
    .unwrap();
    let backfill = &PROFILES[PROFILES.find("DO $$").unwrap()..];
    tx.execute_unprepared(&scratch(backfill)).await.unwrap();
    assert_eq!(
        scalar(
            tx,
            "SELECT count(*)::bigint AS value FROM upgrade_probe.accounts
             WHERE username_key = 'later-registered' AND tag BETWEEN 65536 AND 1048575"
        )
        .await,
        1
    );
    assert_eq!(
        scalar(
            tx,
            "SELECT count(*)::bigint AS value FROM upgrade_probe.accounts WHERE source = 'registered' AND tag IS NULL"
        )
        .await,
        0
    );
}

/// 000012 over what came before: no role and no event yet, no account with a
/// sign-in time; the previous binary still signs in (without the time); the
/// schema holds the role's name, an event's actor and its action's shape.
async fn administration(tx: &sea_orm::DatabaseTransaction) {
    tx.execute_unprepared(&scratch(ADMINISTRATION))
        .await
        .unwrap();
    for (expected, sql) in [
        (
            0,
            "SELECT count(*)::bigint AS value FROM upgrade_probe.account_roles",
        ),
        (
            0,
            "SELECT count(*)::bigint AS value FROM upgrade_probe.audit_events",
        ),
        (
            0,
            "SELECT count(*)::bigint AS value FROM upgrade_probe.accounts WHERE last_sign_in_at IS NOT NULL",
        ),
    ] {
        assert_eq!(scalar(tx, sql).await, expected, "{sql}");
    }
    // As the previous binary signs in: a session without the time.
    tx.execute_unprepared(
        "DELETE FROM upgrade_probe.sessions WHERE account_id = (SELECT id FROM upgrade_probe.accounts WHERE legacy_id = 'legacy-1') AND kind = 'native';
         INSERT INTO upgrade_probe.sessions (digest, account_id, credential_revision, kind, expires_at)
         SELECT decode(repeat('31', 32), 'hex'), id, 1, 'native', clock_timestamp() + interval '30 minutes'
         FROM upgrade_probe.accounts WHERE legacy_id = 'legacy-1';",
    )
    .await
    .unwrap();
    let account = "(SELECT id FROM upgrade_probe.accounts WHERE legacy_id = 'legacy-1')";
    tx.execute_unprepared(&format!(
        "INSERT INTO upgrade_probe.account_roles (account_id, role) SELECT {account}, 'admin';
         INSERT INTO upgrade_probe.audit_events (actor_kind, action, account_id)
         SELECT 'operator', 'account.admin_granted', {account};"
    ))
    .await
    .unwrap();
    for sql in [
        format!("INSERT INTO upgrade_probe.account_roles (account_id, role) SELECT {account}, 'owner'"),
        "INSERT INTO upgrade_probe.audit_events (actor_kind, action) VALUES ('admin', 'account.admin_granted')".into(),
        format!(
            "INSERT INTO upgrade_probe.audit_events (actor_kind, actor_account, action)
             SELECT 'operator', {account}, 'account.admin_granted'"
        ),
        "INSERT INTO upgrade_probe.audit_events (actor_kind, action) VALUES ('operator', 'Granted admin to Alice')".into(),
        "INSERT INTO upgrade_probe.audit_events (actor_kind, action) VALUES ('system', 'account.admin_granted')".into(),
        "INSERT INTO upgrade_probe.audit_events (actor_kind, action, detail)
         VALUES ('operator', 'account.admin_granted', jsonb_build_object('x', repeat('a', 1100)))"
            .into(),
    ] {
        refused(tx, &sql).await;
    }
    // An account's roles go with it; its events stay.
    tx.execute_unprepared(&format!(
        "DELETE FROM upgrade_probe.gateway_tickets; DELETE FROM upgrade_probe.accounts WHERE id = {account}"
    ))
    .await
    .unwrap();
    assert_eq!(
        scalar(
            tx,
            "SELECT count(*)::bigint AS value FROM upgrade_probe.account_roles"
        )
        .await,
        0
    );
    assert_eq!(
        scalar(
            tx,
            "SELECT count(*)::bigint AS value FROM upgrade_probe.audit_events"
        )
        .await,
        1
    );
}

/// 000013 over what came before: no account has a code; a code needs its
/// expiry, 32 bytes and an enrolled account.
async fn reset(tx: &sea_orm::DatabaseTransaction) {
    tx.execute_unprepared(&scratch(RESET)).await.unwrap();
    assert_eq!(
        scalar(
            tx,
            "SELECT count(*)::bigint AS value FROM upgrade_probe.accounts WHERE reset_digest IS NOT NULL"
        )
        .await,
        0
    );
    let pending = "WHERE credential IS NULL";
    tx.execute_unprepared(
        "UPDATE upgrade_probe.accounts SET reset_digest = decode(repeat('41', 32), 'hex'),
         reset_expires = clock_timestamp() + interval '24 hours'
         WHERE id = (SELECT id FROM upgrade_probe.accounts WHERE credential IS NOT NULL ORDER BY id LIMIT 1)",
    )
    .await
    .unwrap();
    for sql in [
        format!(
            "UPDATE upgrade_probe.accounts SET reset_digest = decode(repeat('42', 32), 'hex') {pending}"
        ),
        format!(
            "UPDATE upgrade_probe.accounts SET reset_digest = decode(repeat('43', 32), 'hex'),
             reset_expires = clock_timestamp() {pending}"
        ),
        "UPDATE upgrade_probe.accounts SET reset_digest = decode(repeat('44', 31), 'hex'),
         reset_expires = clock_timestamp() WHERE credential IS NOT NULL"
            .into(),
        "UPDATE upgrade_probe.accounts SET reset_expires = NULL WHERE reset_digest IS NOT NULL"
            .into(),
    ] {
        refused(tx, &sql).await;
    }
}

/// 000014 over the registered and official gateways: every row keeps its
/// pair or its lack of one; a registered gateway may now have no pair (it
/// authenticates by tokens), never half of one, and the official row still
/// none; a token is scoped, named, digested and lives at most 366 days.
async fn tokens(tx: &sea_orm::DatabaseTransaction) {
    let before = scalar(
        tx,
        "SELECT count(*)::bigint AS value FROM upgrade_probe.gateways WHERE credential_digest IS NOT NULL",
    )
    .await;
    tx.execute_unprepared(&scratch(TOKENS)).await.unwrap();
    assert_eq!(
        scalar(
            tx,
            "SELECT count(*)::bigint AS value FROM upgrade_probe.gateways WHERE credential_digest IS NOT NULL"
        )
        .await,
        before
    );
    // any account: earlier steps leave several, not always the same ones
    let owner = "(SELECT id FROM upgrade_probe.accounts ORDER BY id LIMIT 1)";
    tx.execute_unprepared(&format!(
        "INSERT INTO upgrade_probe.gateways (owner_id, origin, name, version, website)
         VALUES ({owner}, 'https://tokens.invalid', 'Tokens', '0', 'https://tokens.invalid/')"
    ))
    .await
    .unwrap();
    let gateway = "(SELECT id FROM upgrade_probe.gateways WHERE origin = 'https://tokens.invalid')";
    tx.execute_unprepared(&format!(
        "INSERT INTO upgrade_probe.gateway_tokens (gateway_id, name, digest, sso, listing, expires_at)
         VALUES ({gateway}, 'Main', decode(repeat('51', 32), 'hex'), true, true,
             clock_timestamp() + interval '366 days')"
    ))
    .await
    .unwrap();
    for sql in [
        format!(
            "INSERT INTO upgrade_probe.gateways (owner_id, origin, name, version, website, credential_digest)
             VALUES ({owner}, 'https://half.invalid', 'Half', '0', 'https://half.invalid/',
                 decode(repeat('52', 32), 'hex'))"
        ),
        "UPDATE upgrade_probe.gateways SET credential_digest = decode(repeat('53', 32), 'hex'),
         heartbeat_digest = decode(repeat('54', 32), 'hex') WHERE official"
            .into(),
        "INSERT INTO upgrade_probe.gateways (origin, name, version, website)
         VALUES ('https://ownerless.invalid', 'Ownerless', '0', 'https://ownerless.invalid/')"
            .into(),
        format!(
            "INSERT INTO upgrade_probe.gateway_tokens (gateway_id, name, digest, sso, listing, expires_at)
             VALUES ({gateway}, 'None', decode(repeat('55', 32), 'hex'), false, false,
                 clock_timestamp() + interval '1 day')"
        ),
        format!(
            "INSERT INTO upgrade_probe.gateway_tokens (gateway_id, name, digest, sso, listing, expires_at)
             VALUES ({gateway}, 'Long', decode(repeat('56', 32), 'hex'), true, false,
                 clock_timestamp() + interval '367 days')"
        ),
        format!(
            "INSERT INTO upgrade_probe.gateway_tokens (gateway_id, name, digest, sso, listing, expires_at)
             VALUES ({gateway}, '', decode(repeat('57', 32), 'hex'), true, false,
                 clock_timestamp() + interval '1 day')"
        ),
        format!(
            "INSERT INTO upgrade_probe.gateway_tokens (gateway_id, name, digest, sso, listing, expires_at)
             VALUES ({gateway}, 'Short', decode(repeat('58', 31), 'hex'), true, false,
                 clock_timestamp() + interval '1 day')"
        ),
        format!(
            "INSERT INTO upgrade_probe.gateway_tokens (gateway_id, name, digest, sso, listing, expires_at)
             VALUES ({gateway}, 'Again', decode(repeat('51', 32), 'hex'), true, false,
                 clock_timestamp() + interval '1 day')"
        ),
        "UPDATE upgrade_probe.gateway_tokens SET revoked_at = created_at - interval '1 second'".into(),
    ] {
        refused(tx, &sql).await;
    }
}
