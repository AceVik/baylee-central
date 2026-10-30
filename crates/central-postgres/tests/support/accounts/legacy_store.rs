//! The legacy store (`docs/native-login.md` §"Legacy login") under the real
//! restricted roles: its role's contract and the runtime's, both ways; the
//! import and each of its refusals; the lockout; the deadline and the sweep.
use super::{AccountStore, ServerKeys, TicketPurpose, Token, config, enroll, login};
use central_api::legacy::LegacyProgress;
use central_postgres::{
    ImportRefusal, LOCKOUT_FAILURES, LegacyRow, LegacyStore, StoreError, import_legacy,
    legacy_digest, set_legacy_deadline, sweep_legacy,
};
use sea_orm::{ConnectionTrait, DatabaseConnection};

fn legacy_role() -> central_postgres::DevConfig {
    config("central_legacy", "CENTRAL_TEST_LEGACY_PASSWORD")
}

fn row(legacy_id: &str, username: &str) -> LegacyRow {
    LegacyRow {
        legacy_id: legacy_id.into(),
        username: username.into(),
        display_name: Some("Moved".into()),
        tag: Some(7),
        salt: [3; 16],
        hash: [4; 32],
        terms_version: Some("2026-10-01".into()),
        must_change_password: false,
        ban: None,
        snapshot: br#"{"preferences":{"synthetic":true},"decks":[]}"#.to_vec(),
    }
}

/// The rows, the count and the digest the gateway would declare for them.
fn declared(mut rows: Vec<LegacyRow>) -> (Vec<LegacyRow>, u64, [u8; 32]) {
    rows.sort_by(|a, b| a.legacy_id.as_bytes().cmp(b.legacy_id.as_bytes()));
    let digest = legacy_digest(&rows);
    let count = rows.len() as u64;
    (rows, count, digest)
}

async fn count(db: &DatabaseConnection, sql: &str) -> i64 {
    super::super::scalar(db, sql).await
}

pub(super) async fn run(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    roles(operator).await;
    let legacy = LegacyStore::connect_dev(legacy_role()).await.unwrap();
    refusals(operator).await;
    let (rows, total, digest) = declared(vec![row("legacy-one", "legacy-one"), {
        let mut banned = row("legacy-two", "legacy-two");
        banned.display_name = None;
        banned.tag = Some(8);
        banned.ban = Some(None);
        banned
    }]);
    let imported = import_legacy(operator, rows, total, digest).await.unwrap();
    assert_eq!((imported.count, imported.digest), (2, digest));
    let (again, total, digest) = declared(vec![row("legacy-three", "legacy-three")]);
    assert_eq!(
        import_legacy(operator, again, total, digest).await,
        Err(ImportRefusal::StoreNotEmpty),
        "the import runs once"
    );
    // Each name is reserved at Central, so nobody registers it meanwhile.
    assert_eq!(
        count(
            operator,
            "SELECT count(*)::bigint AS value FROM central_identity.accounts
             WHERE source = 'legacy' AND credential IS NULL
             AND legacy_id IN ('legacy-one', 'legacy-two') AND username_key = legacy_id"
        )
        .await,
        2
    );
    lockout(&legacy, operator).await;
    let one = legacy.candidate("Legacy-One").await.unwrap().unwrap();
    assert_eq!((one.salt, *one.hash), ([3; 16], [4; 32]));
    assert!(!one.banned && !one.must_change_password);
    assert!(
        legacy
            .candidate("legacy-two")
            .await
            .unwrap()
            .unwrap()
            .banned
    );
    assert!(legacy.candidate("nobody").await.unwrap().is_none());
    assert!(legacy.candidate("not a name!").await.unwrap().is_none());
    let snapshot = legacy.snapshot("legacy-one").await.unwrap().unwrap();
    assert!(snapshot.verified());
    assert_eq!(snapshot.username, "legacy-one");
    let (deadline, listed) = legacy.listing().await.unwrap();
    assert_eq!(
        deadline,
        Some(1_814_392_800),
        "the owner's date, the end of 30.06.2027 in Berlin"
    );
    assert_eq!(listed.len(), 2);
    assert!(
        listed
            .iter()
            .all(|account| account.progress == LegacyProgress::Unstarted)
    );
    assert!(listed[1].banned && listed[1].display_name.is_none());
    closed_move(store, &legacy, operator, keys).await;
    deadline_and_sweep(&legacy, operator).await;
    legacy.close().await.unwrap();
}

/// A move through the store's account: its progress, its session's view,
/// the bridge's admission only once it is closed, and the sweep of a row
/// whose deletion was lost.
async fn closed_move(
    store: &AccountStore,
    legacy: &LegacyStore,
    operator: &DatabaseConnection,
    keys: &ServerKeys,
) {
    let identity = enroll(store, keys, "legacy-one", "legacy-one").await;
    let session = login(store, keys, identity).await;
    assert_eq!(
        store
            .legacy_progress(&["legacy-one".into(), "legacy-two".into()])
            .await
            .unwrap(),
        [LegacyProgress::Enrolled, LegacyProgress::Unstarted]
    );
    store.confirm_restore(&session, 1).await.unwrap();
    let moving = store.legacy_move(&session).await.unwrap();
    assert_eq!(moving.legacy_id.as_deref(), Some("legacy-one"));
    assert!(moving.restored && !moving.closed);
    // The bridge learns the legacy id only once the move is closed; the
    // refused redemption spends nothing.
    let ticket = Token::generate().unwrap();
    store
        .gateway_ticket(&session, "official", &ticket, TicketPurpose::Admit)
        .await
        .unwrap();
    let lease = Token::generate().unwrap();
    assert!(
        store
            .redeem_lease(
                &ticket,
                "official",
                TicketPurpose::Admit,
                None,
                Some(&lease)
            )
            .await
            .is_err()
    );
    store.close_legacy("legacy-one", identity).await.unwrap();
    let (_, legacy_id, _) = store
        .redeem_lease(
            &ticket,
            "official",
            TicketPurpose::Admit,
            None,
            Some(&lease),
        )
        .await
        .unwrap();
    assert_eq!(legacy_id.as_deref(), Some("legacy-one"));
    // The close's row deletion was lost here: the sweep catches it.
    let swept = sweep_legacy(operator).await.unwrap();
    assert_eq!((swept.moved, swept.expired, swept.reservations), (1, 0, 0));
    assert!(legacy.snapshot("legacy-one").await.unwrap().is_none());
}

/// The legacy role holds exactly its grants, and the runtime role nothing in
/// the legacy schema: either side with one privilege more is refused at
/// startup, and the statements themselves fail under the real roles.
async fn roles(operator: &DatabaseConnection) {
    LegacyStore::connect_dev(legacy_role())
        .await
        .unwrap()
        .close()
        .await
        .unwrap();
    for (grant, revoke) in [
        (
            "GRANT INSERT ON central_legacy.accounts TO central_legacy",
            "REVOKE INSERT ON central_legacy.accounts FROM central_legacy",
        ),
        (
            "GRANT UPDATE (snapshot) ON central_legacy.accounts TO central_legacy",
            "REVOKE UPDATE (snapshot) ON central_legacy.accounts FROM central_legacy",
        ),
        (
            "GRANT UPDATE ON central_legacy.accounts TO central_legacy",
            "REVOKE UPDATE ON central_legacy.accounts FROM central_legacy;
             GRANT UPDATE (failures, locked_until) ON central_legacy.accounts TO central_legacy",
        ),
        (
            "GRANT UPDATE ON central_legacy.settings TO central_legacy",
            "REVOKE UPDATE ON central_legacy.settings FROM central_legacy",
        ),
        (
            "GRANT TRUNCATE ON central_legacy.accounts TO central_legacy",
            "REVOKE TRUNCATE ON central_legacy.accounts FROM central_legacy",
        ),
        (
            "GRANT USAGE ON SCHEMA central_identity TO central_legacy",
            "REVOKE USAGE ON SCHEMA central_identity FROM central_legacy",
        ),
        (
            "REVOKE DELETE ON central_legacy.accounts FROM central_legacy",
            "GRANT DELETE ON central_legacy.accounts TO central_legacy",
        ),
    ] {
        operator.execute_unprepared(grant).await.unwrap();
        let refused = LegacyStore::connect_dev(legacy_role()).await;
        operator.execute_unprepared(revoke).await.unwrap();
        assert!(matches!(refused, Err(StoreError::UnsafeRole)), "{grant}");
    }
    for (grant, revoke) in [
        (
            "GRANT USAGE ON SCHEMA central_legacy TO central_runtime",
            "REVOKE USAGE ON SCHEMA central_legacy FROM central_runtime",
        ),
        (
            "GRANT SELECT ON central_legacy.accounts TO central_runtime",
            "REVOKE SELECT ON central_legacy.accounts FROM central_runtime",
        ),
        (
            "GRANT UPDATE (deadline) ON central_legacy.settings TO central_runtime",
            "REVOKE UPDATE (deadline) ON central_legacy.settings FROM central_runtime",
        ),
    ] {
        operator.execute_unprepared(grant).await.unwrap();
        let refused =
            AccountStore::connect_dev(config("central_runtime", "CENTRAL_TEST_RUNTIME_PASSWORD"))
                .await;
        operator.execute_unprepared(revoke).await.unwrap();
        assert!(refused.is_err(), "{grant}");
    }
    let runtime =
        central_postgres::connect_dev(config("central_runtime", "CENTRAL_TEST_RUNTIME_PASSWORD"))
            .await
            .unwrap();
    for sql in [
        "SELECT legacy_id FROM central_legacy.accounts",
        "SELECT deadline FROM central_legacy.settings",
        "DELETE FROM central_legacy.accounts",
    ] {
        assert!(runtime.execute_unprepared(sql).await.is_err(), "{sql}");
    }
    runtime.close().await.unwrap();
    let raw = central_postgres::connect_dev(legacy_role()).await.unwrap();
    for sql in [
        "SELECT id FROM central_identity.accounts",
        "INSERT INTO central_legacy.accounts (legacy_id) VALUES ('x')",
        "UPDATE central_legacy.accounts SET snapshot = '\\x7b7d'",
        "UPDATE central_legacy.settings SET deadline = NULL",
        "TRUNCATE central_legacy.accounts",
    ] {
        assert!(raw.execute_unprepared(sql).await.is_err(), "{sql}");
    }
    raw.close().await.unwrap();
}

/// Each refusal stores nothing: no row, no reservation.
async fn refusals(operator: &DatabaseConnection) {
    let empty = |operator| async move {
        count(
            operator,
            "SELECT count(*)::bigint AS value FROM central_legacy.accounts",
        )
        .await
            + count(
                operator,
                "SELECT count(*)::bigint AS value FROM central_identity.accounts
                 WHERE legacy_id LIKE 'refused-%'",
            )
            .await
    };
    let malformed = |field: &'static str, change: fn(&mut LegacyRow)| {
        let mut bad = row("refused-a", "refused-a");
        change(&mut bad);
        (vec![bad, row("refused-b", "refused-b")], field)
    };
    for (rows, field) in [
        malformed("username", |row| row.username = "Refused-A".into()),
        malformed("display_name", |row| row.display_name = Some("x".into())),
        malformed("tag", |row| row.tag = Some(0)),
        malformed("legacy_id", |row| row.legacy_id = "refused a".into()),
        malformed("terms_version", |row| {
            row.terms_version = Some(String::new());
        }),
        malformed("snapshot", |row| row.snapshot = vec![0xff, 0xfe]),
    ] {
        let (rows, total, digest) = declared(rows);
        let refusal = import_legacy(operator, rows, total, digest)
            .await
            .unwrap_err();
        assert!(
            matches!(&refusal, ImportRefusal::Malformed { field: named, .. } if *named == field),
            "{refusal}"
        );
    }
    let (rows, total, digest) = declared(vec![row("refused-a", "refused-a")]);
    assert_eq!(
        import_legacy(operator, rows, total + 1, digest).await,
        Err(ImportRefusal::CountMismatch)
    );
    let (rows, total, mut digest) = declared(vec![row("refused-a", "refused-a")]);
    digest[0] ^= 1;
    assert_eq!(
        import_legacy(operator, rows, total, digest).await,
        Err(ImportRefusal::DigestMismatch)
    );
    let (rows, total, digest) = declared(vec![
        row("refused-a", "refused-x"),
        row("refused-b", "refused-x"),
    ]);
    assert!(matches!(
        import_legacy(operator, rows, total, digest).await,
        Err(ImportRefusal::Duplicate { .. })
    ));
    // `alice` is an enrolled Central account by now; `restart-legacy`'s
    // move closed in `migration::run`.
    let (rows, total, digest) = declared(vec![
        row("refused-a", "refused-a"),
        row("refused-b", "alice"),
    ]);
    assert_eq!(
        import_legacy(operator, rows, total, digest).await,
        Err(ImportRefusal::NameTaken {
            legacy_id: "refused-b".into()
        })
    );
    let (rows, total, digest) = declared(vec![
        row("refused-a", "refused-a"),
        row("restart-legacy", "refused-z"),
    ]);
    assert_eq!(
        import_legacy(operator, rows, total, digest).await,
        Err(ImportRefusal::AlreadyMoved {
            legacy_id: "restart-legacy".into()
        })
    );
    assert_eq!(empty(operator).await, 0, "a refused import stores nothing");
}

/// Ten failures in a row lock an account out for the hour; while locked it
/// reads as unknown and counts nothing more.
async fn lockout(legacy: &LegacyStore, operator: &DatabaseConnection) {
    let failures = |operator| async move {
        count(
            operator,
            "SELECT failures::bigint AS value FROM central_legacy.accounts WHERE legacy_id = 'legacy-one'",
        )
        .await
    };
    for _ in 1..LOCKOUT_FAILURES {
        legacy.failed("LEGACY-ONE").await.unwrap();
    }
    assert_eq!(failures(operator).await, i64::from(LOCKOUT_FAILURES - 1));
    assert!(legacy.candidate("legacy-one").await.unwrap().is_some());
    legacy.failed("legacy-one").await.unwrap();
    assert_eq!(failures(operator).await, 0);
    assert!(
        legacy.candidate("legacy-one").await.unwrap().is_none(),
        "locked out"
    );
    legacy.failed("legacy-one").await.unwrap();
    assert_eq!(
        failures(operator).await,
        0,
        "a locked account counts nothing"
    );
    legacy.failed("nobody").await.unwrap();
    // The hour passes.
    operator
        .execute_unprepared(
            "UPDATE central_legacy.accounts SET locked_until = now() - interval '1 second'
             WHERE legacy_id = 'legacy-one'",
        )
        .await
        .unwrap();
    assert!(legacy.candidate("legacy-one").await.unwrap().is_some());
    legacy.failed("legacy-one").await.unwrap();
    legacy.succeeded("legacy-one").await.unwrap();
    assert_eq!(failures(operator).await, 0);
}

/// After the deadline nothing is handed out, and the sweep deletes every row
/// and every legacy reservation that never enrolled.
async fn deadline_and_sweep(legacy: &LegacyStore, operator: &DatabaseConnection) {
    for bad in [
        "2026-13-01",
        "2026-1-01",
        "tomorrow",
        "2026-10-30T00:00:00Z",
    ] {
        assert!(
            set_legacy_deadline(operator, Some(bad)).await.is_err(),
            "{bad}"
        );
    }
    // The date is the last day: its end in Berlin (CET in January).
    assert_eq!(
        set_legacy_deadline(operator, Some("2099-01-01"))
            .await
            .unwrap(),
        Some(4_070_991_600)
    );
    assert_eq!(sweep_legacy(operator).await.unwrap().expired, 0);
    assert!(legacy.candidate("legacy-two").await.unwrap().is_some());
    set_legacy_deadline(operator, Some("2000-01-01"))
        .await
        .unwrap();
    assert!(legacy.candidate("legacy-two").await.unwrap().is_none());
    assert!(legacy.snapshot("legacy-two").await.unwrap().is_none());
    let unenrolled = count(
        operator,
        "SELECT count(*)::bigint AS value FROM central_identity.accounts
         WHERE source = 'legacy' AND credential IS NULL AND NOT legacy_closed",
    )
    .await;
    assert!(unenrolled >= 1);
    let swept = sweep_legacy(operator).await.unwrap();
    assert_eq!(swept.expired, 1);
    assert_eq!(swept.reservations, u64::try_from(unenrolled).unwrap());
    assert_eq!(
        count(
            operator,
            "SELECT count(*)::bigint AS value FROM central_legacy.accounts"
        )
        .await,
        0
    );
    assert_eq!(set_legacy_deadline(operator, None).await.unwrap(), None);
}
