//! Migration 000007: the account's source and the legacy rules that apply
//! to legacy accounts only; and 000023: the closed beta's keys gone, the
//! administrators' registration switch in their place.
use super::{AccountStore, ServerKeys, TicketPurpose, Token, config, enroll, login};
use central_postgres::connect_dev;
use sea_orm::{ConnectionTrait, DatabaseConnection};

pub(super) async fn run(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    switch(operator).await;
    privileges(operator).await;
    constraints(store, operator, keys).await;
    registered(store, keys).await;
}

async fn count(db: &impl ConnectionTrait, sql: &str) -> i64 {
    super::super::scalar(db, sql).await
}

async fn switch(operator: &DatabaseConnection) {
    // Nothing of the keys is left: their table, and an account's link to one.
    for (expected, sql) in [
        (0, "SELECT count(*)::bigint AS value FROM pg_catalog.pg_class
             WHERE oid = pg_catalog.to_regclass('central_identity.beta_keys')"),
        (0, "SELECT count(*)::bigint AS value FROM information_schema.columns
             WHERE table_schema = 'central_identity' AND table_name = 'accounts' AND column_name = 'beta_key_id'"),
        // The audit keeps the key ids its old events name, as history.
        (1, "SELECT count(*)::bigint AS value FROM information_schema.columns
             WHERE table_schema = 'central_identity' AND table_name = 'audit_events' AND column_name = 'beta_key_id'"),
        // One row, open.
        (1, "SELECT count(*)::bigint AS value FROM central_identity.registration WHERE open AND changed_by IS NULL"),
    ] {
        assert_eq!(count(operator, sql).await, expected, "{sql}");
    }
    for sql in [
        "INSERT INTO central_identity.registration (singleton, open) VALUES (true, false)",
        "INSERT INTO central_identity.registration (singleton, open) VALUES (false, false)",
        "UPDATE central_identity.registration SET open = NULL",
    ] {
        assert!(operator.execute_unprepared(sql).await.is_err(), "{sql}");
    }
}

async fn privileges(operator: &DatabaseConnection) {
    let runtime = connect_dev(config("central_runtime", "CENTRAL_TEST_RUNTIME_PASSWORD"))
        .await
        .unwrap();
    // The runtime opens and closes registration; the row itself stays.
    runtime
        .execute_unprepared(
            "UPDATE central_identity.registration SET open = true, changed_at = NULL, changed_by = NULL",
        )
        .await
        .unwrap();
    for sql in [
        "INSERT INTO central_identity.registration (singleton, open) VALUES (true, true)",
        "UPDATE central_identity.registration SET singleton = true",
        "DELETE FROM central_identity.registration",
        "TRUNCATE central_identity.registration",
    ] {
        assert!(runtime.execute_unprepared(sql).await.is_err(), "{sql}");
    }
    runtime.close().await.unwrap();
    // The store's startup check refuses a wider grant on the switch, and a missing one.
    let table = "central_identity.registration";
    let restore_update = format!(
        "REVOKE UPDATE ON {table} FROM central_runtime;
         GRANT UPDATE (open, changed_at, changed_by) ON {table} TO central_runtime"
    );
    for (change, undo) in [
        (
            format!("GRANT UPDATE (singleton) ON {table} TO central_runtime"),
            format!("REVOKE UPDATE (singleton) ON {table} FROM central_runtime"),
        ),
        (
            format!("GRANT INSERT ON {table} TO central_runtime"),
            format!("REVOKE INSERT ON {table} FROM central_runtime"),
        ),
        (
            format!("GRANT DELETE ON {table} TO central_runtime"),
            format!("REVOKE DELETE ON {table} FROM central_runtime"),
        ),
        (
            format!("REVOKE UPDATE (open) ON {table} FROM central_runtime"),
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
             AND source = 'legacy' AND recovery_wrapped_master IS NULL"
        )
        .await,
        1
    );
    // Since 000010 an enrolled legacy account may hold a recovery wrapper
    // (one made after a sign-in); a pending one still may not.
    operator
        .execute_unprepared(
            "UPDATE central_identity.accounts SET recovery_wrapped_master = decode(repeat('01', 73), 'hex')
             WHERE legacy_id = 'source-legacy';
             UPDATE central_identity.accounts SET recovery_wrapped_master = NULL WHERE legacy_id = 'source-legacy';",
        )
        .await
        .unwrap();
    store
        .reserve_legacy(
            "pending-legacy",
            "pending-legacy",
            None,
            None,
            &Token::generate().unwrap(),
        )
        .await
        .unwrap();
    let pending = "INSERT INTO central_identity.accounts (source, username_key, enrollment_digest, enrollment_expires)
         VALUES ('registered', 'constraint-probe', decode(repeat('cd', 32), 'hex'), clock_timestamp() + interval '10 minutes')";
    operator.execute_unprepared(pending).await.unwrap();
    let enrolled = "credential = '\\x01', wrapped_master = decode(repeat('01', 73), 'hex'),
        document = decode(repeat('01', 41), 'hex'), document_revision = 1,
        enrollment_digest = NULL, enrollment_expires = NULL";
    let recovery = "recovery_wrapped_master = decode(repeat('01', 73), 'hex')";
    let probe = "WHERE username_key = 'constraint-probe'";
    for sql in [
        "INSERT INTO central_identity.accounts (source, legacy_id, username_key, enrollment_digest, enrollment_expires)
            VALUES ('registered', 'probe-two', 'probe-two', decode(repeat('ce', 32), 'hex'), clock_timestamp() + interval '1 minute')".into(),
        "INSERT INTO central_identity.accounts (username_key) VALUES ('probe-three')".into(),
        "INSERT INTO central_identity.accounts (source, username_key) VALUES ('registered', 'probe-four')".into(),
        "INSERT INTO central_identity.accounts (source, legacy_id, username_key) VALUES ('other', 'probe-seven', 'probe-seven')".into(),
        format!("UPDATE central_identity.accounts SET legacy_closed = true {probe}"),
        format!("UPDATE central_identity.accounts SET {recovery} {probe}"),
        format!("UPDATE central_identity.accounts SET {recovery} WHERE legacy_id = 'pending-legacy'"),
        format!("UPDATE central_identity.accounts SET {enrolled} {probe}"),
        format!("UPDATE central_identity.accounts SET {enrolled},
            recovery_wrapped_master = decode(repeat('01', 72), 'hex') {probe}"),
    ] {
        assert!(operator.execute_unprepared(&sql).await.is_err(), "{sql}");
    }
    operator
        .execute_unprepared(&format!(
            "UPDATE central_identity.accounts SET {enrolled}, {recovery} {probe}"
        ))
        .await
        .unwrap();
    operator
        .execute_unprepared(&format!("DELETE FROM central_identity.accounts {probe}"))
        .await
        .unwrap();
}

/// A registered account, through registration and enrollment: legacy rules
/// do not hold it back.
async fn registered(store: &AccountStore, keys: &ServerKeys) {
    let ticket = Token::generate().unwrap();
    let identity = store
        .register("Registered-One", None, &ticket)
        .await
        .unwrap();
    assert_eq!(
        store.enrollment(&ticket).await.unwrap().account,
        identity.account
    );
    let enrolled = super::register::credentials(keys, identity);
    store
        .enroll(
            &ticket,
            &enrolled.upload,
            &enrolled.wrapped,
            &enrolled.document,
            Some(&enrolled.recovery),
        )
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
    let status = store.session_status(&session).await.unwrap();
    assert!(
        status.migration_closed,
        "a registered account has no legacy reset authority"
    );
    assert_eq!(status.username, "registered-one", "the canonical name");
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
            &["official"],
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
