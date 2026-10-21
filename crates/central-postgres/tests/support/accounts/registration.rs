//! Migration 000007: beta keys, the account's source, and the legacy rules
//! that apply to legacy accounts only.
use super::{AccountStore, ServerKeys, TicketPurpose, Token, config, context, enroll, login};
use central_api::beta_key::BetaKey;
use central_crypto::{
    opaque::Enrollment,
    vault::{MasterKey, ObjectContext},
};
use central_postgres::{BetaKeyBatch, connect_dev};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, prelude::Uuid};

pub(super) async fn run(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    issuing(operator).await;
    privileges(operator).await;
    constraints(store, operator, keys).await;
    registered(store, operator, keys).await;
}

async fn count(db: &impl ConnectionTrait, sql: &str) -> i64 {
    super::super::scalar(db, sql).await
}

/// A key of the first run, for rows that need one.
const KEY: &str =
    "(SELECT id FROM central_identity.beta_keys WHERE note = 'fixture run' ORDER BY id LIMIT 1)";

async fn issuing(operator: &DatabaseConnection) {
    let keys: Vec<BetaKey> = (0..3).map(|_| BetaKey::generate().unwrap()).collect();
    let digests: Vec<[u8; 32]> = keys.iter().map(BetaKey::digest).collect();
    BetaKeyBatch::new(&digests[..2], 5, None, "fixture run")
        .unwrap()
        .issue(operator)
        .await
        .unwrap();
    BetaKeyBatch::new(&digests[2..], 1, Some(30), "")
        .unwrap()
        .issue(operator)
        .await
        .unwrap();
    assert_eq!(
        count(
            operator,
            "SELECT count(*)::bigint AS value FROM central_identity.beta_keys
             WHERE uses_left = 5 AND note = 'fixture run' AND expires_at IS NULL AND revoked_at IS NULL"
        )
        .await,
        2
    );
    assert_eq!(
        count(
            operator,
            "SELECT count(*)::bigint AS value FROM central_identity.beta_keys WHERE uses_left = 1
             AND expires_at - created_at = interval '30 days'"
        )
        .await,
        1
    );
    // Each key is stored as its digest, and nothing else of it is.
    for key in &keys {
        let row = operator
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT count(*)::bigint AS value FROM central_identity.beta_keys
                 WHERE digest = $1 AND position($2 in note) = 0",
                [key.digest().to_vec().into(), key.display().as_str().into()],
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.try_get::<i64>("", "value").unwrap(), 1);
    }
    // A run repeating a stored digest writes nothing at all.
    let repeated = [BetaKey::generate().unwrap().digest(), digests[0]];
    assert!(
        BetaKeyBatch::new(&repeated, 1, None, "")
            .unwrap()
            .issue(operator)
            .await
            .is_err()
    );
    assert_eq!(
        count(
            operator,
            "SELECT count(*)::bigint AS value FROM central_identity.beta_keys"
        )
        .await,
        3
    );
}

async fn privileges(operator: &DatabaseConnection) {
    let runtime = connect_dev(config("central_runtime", "CENTRAL_TEST_RUNTIME_PASSWORD"))
        .await
        .unwrap();
    // The runtime spends a use; it cannot issue, revoke, re-date, re-note or delete a key.
    runtime
        .execute_unprepared(
            "UPDATE central_identity.beta_keys SET uses_left = uses_left - 1 WHERE note = 'fixture run'",
        )
        .await
        .unwrap();
    for sql in [
        "INSERT INTO central_identity.beta_keys (digest, uses_left, note) VALUES (decode(repeat('ab', 32), 'hex'), 1, '')",
        "UPDATE central_identity.beta_keys SET revoked_at = NULL",
        "UPDATE central_identity.beta_keys SET expires_at = NULL",
        "UPDATE central_identity.beta_keys SET note = 'changed'",
        "UPDATE central_identity.beta_keys SET digest = digest",
        "DELETE FROM central_identity.beta_keys",
        "TRUNCATE central_identity.beta_keys",
        "UPDATE central_identity.beta_keys SET uses_left = -1",
    ] {
        assert!(runtime.execute_unprepared(sql).await.is_err(), "{sql}");
    }
    runtime.close().await.unwrap();
    // The store's startup check refuses a wider grant on beta keys, and a missing one.
    let table = "central_identity.beta_keys";
    let restore_update = format!(
        "REVOKE UPDATE ON {table} FROM central_runtime; GRANT UPDATE (uses_left) ON {table} TO central_runtime"
    );
    for (change, undo) in [
        (
            format!("GRANT UPDATE (note) ON {table} TO central_runtime"),
            format!("REVOKE UPDATE (note) ON {table} FROM central_runtime"),
        ),
        (
            format!("GRANT UPDATE (revoked_at) ON {table} TO central_runtime"),
            format!("REVOKE UPDATE (revoked_at) ON {table} FROM central_runtime"),
        ),
        (
            format!("GRANT UPDATE ON {table} TO central_runtime"),
            restore_update.clone(),
        ),
        (
            format!("GRANT INSERT ON {table} TO central_runtime"),
            format!("REVOKE INSERT ON {table} FROM central_runtime"),
        ),
        (
            format!("GRANT INSERT (note) ON {table} TO central_runtime"),
            format!("REVOKE INSERT (note) ON {table} FROM central_runtime"),
        ),
        (
            format!("GRANT DELETE ON {table} TO central_runtime"),
            format!("REVOKE DELETE ON {table} FROM central_runtime"),
        ),
        (
            format!("GRANT UPDATE (uses_left) ON {table} TO central_runtime WITH GRANT OPTION"),
            restore_update.clone(),
        ),
        (
            format!("REVOKE UPDATE (uses_left) ON {table} FROM central_runtime"),
            restore_update.clone(),
        ),
        (
            format!("REVOKE SELECT ON {table} FROM central_runtime"),
            format!("GRANT SELECT ON {table} TO central_runtime"),
        ),
    ] {
        // Fixed synthetic statements, never user input.
        operator.execute_unprepared(&change).await.unwrap();
        let refused =
            AccountStore::connect_dev(config("central_runtime", "CENTRAL_TEST_RUNTIME_PASSWORD"))
                .await
                .is_err();
        operator.execute_unprepared(&undo).await.unwrap();
        assert!(refused, "{change}");
    }
    AccountStore::connect_dev(config("central_runtime", "CENTRAL_TEST_RUNTIME_PASSWORD"))
        .await
        .unwrap()
        .close()
        .await
        .unwrap();
}

async fn constraints(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    enroll(store, keys, "source-legacy", "source-legacy").await;
    assert_eq!(
        count(
            operator,
            "SELECT count(*)::bigint AS value FROM central_identity.accounts WHERE legacy_id = 'source-legacy'
             AND source = 'legacy' AND recovery_wrapped_master IS NULL AND beta_key_id IS NULL"
        )
        .await,
        1
    );
    let pending = format!(
        "INSERT INTO central_identity.accounts (source, username_key, enrollment_digest, enrollment_expires, beta_key_id)
         VALUES ('registered', 'constraint-probe', decode(repeat('cd', 32), 'hex'), clock_timestamp() + interval '10 minutes', {KEY})"
    );
    operator.execute_unprepared(&pending).await.unwrap();
    let enrolled = "credential = '\\x01', wrapped_master = decode(repeat('01', 73), 'hex'),
        document = decode(repeat('01', 41), 'hex'), document_revision = 1,
        enrollment_digest = NULL, enrollment_expires = NULL";
    let recovery = "recovery_wrapped_master = decode(repeat('01', 73), 'hex')";
    let probe = "WHERE username_key = 'constraint-probe'";
    for sql in [
        format!("INSERT INTO central_identity.accounts (source, legacy_id, username_key, enrollment_digest, enrollment_expires, beta_key_id)
            VALUES ('registered', 'probe-two', 'probe-two', decode(repeat('ce', 32), 'hex'), clock_timestamp() + interval '1 minute', {KEY})"),
        "INSERT INTO central_identity.accounts (username_key) VALUES ('probe-three')".into(),
        "INSERT INTO central_identity.accounts (source, username_key) VALUES ('registered', 'probe-four')".into(),
        "INSERT INTO central_identity.accounts (source, username_key, enrollment_digest, enrollment_expires)
            VALUES ('registered', 'probe-five', decode(repeat('cf', 32), 'hex'), clock_timestamp() + interval '1 minute')".into(),
        format!("INSERT INTO central_identity.accounts (legacy_id, username_key, beta_key_id) VALUES ('probe-six', 'probe-six', {KEY})"),
        "INSERT INTO central_identity.accounts (source, legacy_id, username_key) VALUES ('other', 'probe-seven', 'probe-seven')".into(),
        format!("UPDATE central_identity.accounts SET legacy_closed = true {probe}"),
        format!("UPDATE central_identity.accounts SET {recovery} {probe}"),
        format!("UPDATE central_identity.accounts SET {recovery} WHERE legacy_id = 'source-legacy'"),
        format!("UPDATE central_identity.accounts SET {enrolled}, beta_key_id = NULL {probe}"),
        format!("UPDATE central_identity.accounts SET {enrolled}, {recovery} {probe}"),
        format!("UPDATE central_identity.accounts SET {enrolled}, beta_key_id = NULL,
            recovery_wrapped_master = decode(repeat('01', 72), 'hex') {probe}"),
    ] {
        assert!(operator.execute_unprepared(&sql).await.is_err(), "{sql}");
    }
    operator
        .execute_unprepared(&format!(
            "UPDATE central_identity.accounts SET {enrolled}, {recovery}, beta_key_id = NULL {probe}"
        ))
        .await
        .unwrap();
    operator
        .execute_unprepared(&format!("DELETE FROM central_identity.accounts {probe}"))
        .await
        .unwrap();
}

/// A registered account, enrolled the way the next change's `Register` and
/// enrollment will write it: legacy rules do not hold it back.
async fn registered(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let ticket = Token::generate().unwrap();
    operator
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            format!(
                "INSERT INTO central_identity.accounts (source, username_key, enrollment_digest, enrollment_expires, beta_key_id)
                 VALUES ('registered', 'registered-one', $1, clock_timestamp() + interval '10 minutes', {KEY})"
            ),
            [ticket.digest().to_vec().into()],
        ))
        .await
        .unwrap();
    let identity = store.enrollment(&ticket).await.unwrap();
    let context = context(identity);
    let (enrollment, request) =
        Enrollment::start("synthetic strong password".into(), context).unwrap();
    let response = keys.enrollment_response(context, &request).unwrap();
    let enrolled = enrollment.finish(&response).unwrap();
    let master = MasterKey::generate().unwrap();
    let wrapped = enrolled.wrapping_key.wrap(&master).unwrap();
    let object = ObjectContext::new(context, identity.account, 1).unwrap();
    let document = master.seal_object(object, b"registered").unwrap();
    // Enrollment without a recovery wrapper, keeping the key link, is refused by the schema.
    assert!(
        store
            .enroll(&ticket, &enrolled.upload, wrapped.as_bytes(), &document)
            .await
            .is_err()
    );
    assert!(store.credential(identity).await.unwrap().is_none());
    operator
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE central_identity.accounts SET credential = $2, wrapped_master = $3,
             recovery_wrapped_master = $3, document = $4, document_revision = 1,
             enrollment_digest = NULL, enrollment_expires = NULL, beta_key_id = NULL WHERE id = $1",
            [
                Uuid::from_bytes(identity.account).into(),
                enrolled.upload.clone().into(),
                wrapped.as_bytes().to_vec().into(),
                document.into(),
            ],
        ))
        .await
        .unwrap();
    assert_eq!(
        store
            .discover("Registered-One")
            .await
            .unwrap()
            .unwrap()
            .account,
        identity.account
    );
    let session = login(store, keys, identity).await;
    let (_, _, closed) = store.session_status(&session).await.unwrap();
    assert!(closed, "a registered account has no legacy reset authority");
    // Private objects wait for the restore confirmation, not for a legacy close.
    assert!(store.create_object(&session).await.is_err());
    store.confirm_restore(&session, 1).await.unwrap();
    let object = store.create_object(&session).await.unwrap();
    store.read_object(&session, object.id).await.unwrap();
    store.list_objects(&session, None).await.unwrap();
    // The bridge learns that the account has no legacy ID, instead of failing.
    let ticket = Token::generate().unwrap();
    store
        .gateway_ticket(&session, "official", &ticket, TicketPurpose::Admit)
        .await
        .unwrap();
    let (redeemed, legacy) = store
        .redeem_gateway(&ticket, "official", TicketPurpose::Admit)
        .await
        .unwrap();
    assert_eq!(redeemed.account, identity.account);
    assert_eq!(legacy, None);
    let ticket = Token::generate().unwrap();
    store
        .gateway_ticket(&session, "official", &ticket, TicketPurpose::Admit)
        .await
        .unwrap();
    let lease = Token::generate().unwrap();
    let (redeemed, legacy, expires) = store
        .redeem_lease(
            &ticket,
            "official",
            TicketPurpose::Admit,
            None,
            Some(&lease),
        )
        .await
        .unwrap();
    assert_eq!((redeemed.account, legacy), (identity.account, None));
    assert!(expires.is_some());
    // The bridge's legacy operations cannot reach it: it has no legacy ID to name.
    assert!(store.legacy_status("registered-one").await.is_err());
    store.logout(&session).await.unwrap();
}
