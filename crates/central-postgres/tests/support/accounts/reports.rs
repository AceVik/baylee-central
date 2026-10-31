//! Reports (migration 000018, `docs/reports.md`): filed only by an approved
//! gateway's `sso` token or the bridge, parts stored only as announced and
//! once, an administrator's lists, triage by its rules (revision, one
//! duplicate level, assignees who are administrators), notes by their
//! author, the trash and a purge with a fresh password, saved views, the
//! unread marker, the sweep, and the runtime's exact grants.
use super::{AccountStore, DatabaseConnection, Error, ServerKeys, Token, config, enroll, login};
use central_api::owner::GatewayScopes;
use central_api::reports::{
    Announced, AnnouncedRecord, Build, BulkAction, Channel, Crash, GatewaySays, Part, Priority,
    RecordOrigin, ReportChange, ReportFilter, ReportIntake, ReportKind, ReportSort, ReportStatus,
};
use central_api::wire::Identity;
use central_postgres::{Session, appoint_admin};
use sea_orm::{ConnectionTrait, DbBackend, Statement, prelude::Uuid};
use sha2::{Digest, Sha256};

const ORIGIN: &str = "https://reports.invalid";

/// An account whose move is finished, with a browser session.
async fn signed_in(store: &AccountStore, keys: &ServerKeys, legacy: &str) -> (Identity, Token) {
    let identity = enroll(store, keys, legacy, legacy).await;
    let native = login(store, keys, identity).await;
    store.confirm_restore(&native, 1).await.unwrap();
    store.close_legacy(legacy, identity).await.unwrap();
    let credential = store.credential(identity).await.unwrap().unwrap();
    let browser = Token::generate().unwrap();
    store
        .issue_session(&credential, Session::Browser(&browser))
        .await
        .unwrap();
    (identity, browser)
}

async fn sql(operator: &DatabaseConnection, text: &str, values: Vec<sea_orm::Value>) {
    operator
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            text,
            values,
        ))
        .await
        .unwrap();
}

async fn count(operator: &DatabaseConnection, text: &str, values: Vec<sea_orm::Value>) -> i64 {
    operator
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            text,
            values,
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<i64>("", "n")
        .unwrap()
}

/// Synthetic gzip bytes (the magic, then anything: Central never unpacks them).
fn gzip(fill: u8, length: usize) -> Vec<u8> {
    let mut bytes = vec![fill; length];
    bytes[0] = 0x1f;
    bytes[1] = 0x8b;
    bytes
}

fn announce(bytes: &[u8]) -> Announced {
    Announced {
        bytes: u32::try_from(bytes.len()).unwrap(),
        sha256: Sha256::digest(bytes).into(),
    }
}

fn intake(kind: ReportKind, text: &str, client: &[u8], record: Option<&[u8]>) -> ReportIntake {
    ReportIntake {
        gateway: GatewaySays {
            name: Some("Synthetic reports".into()),
            url: Some(ORIGIN.into()),
            version: Some("1.0-test".into()),
        },
        channel: Channel::Gateway,
        reporter: "5bdc11".into(),
        kind,
        text: text.into(),
        game_id: None,
        build: Some(Build {
            version: "0.1.1-test".into(),
            commit: Some("3f9a1c7e21".into()),
        }),
        crash: (kind == ReportKind::Crash).then(|| Crash {
            message: "index 7 out of range for 3".into(),
            location: "crates/engine/src/stack.rs:120:9".into(),
        }),
        client: Some(announce(client)),
        record: record.map(|r| AnnouncedRecord {
            complete: false,
            origin: RecordOrigin::Gateway,
            bytes: u32::try_from(r.len()).unwrap(),
            sha256: Sha256::digest(r).into(),
        }),
    }
}

fn all() -> ReportFilter {
    ReportFilter::default()
}

pub(super) async fn run(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    grants(operator).await;
    let (owner, owner_browser) = signed_in(store, keys, "report-owner").await;
    let (admin, cookie) = signed_in(store, keys, "report-admin").await;
    let (_, stranger) = signed_in(store, keys, "report-stranger").await;
    appoint_admin(operator, "report-admin").await.unwrap();
    let gateway = store
        .add_gateway(&owner_browser, ORIGIN, "Synthetic reports", false)
        .await
        .unwrap();
    let token = Token::generate().unwrap();
    let scopes = GatewayScopes {
        sso: true,
        listing: false,
    };
    store
        .create_gateway_token(&owner_browser, gateway, "reports", scopes, 30, &token)
        .await
        .unwrap();
    // pending: the token signs players in, but files nothing
    assert_eq!(
        store.report_filer(&token).await.err(),
        Some(Error::Rejected)
    );
    sql(
        operator,
        "INSERT INTO central_identity.gateway_approvals (gateway_id, enabled) VALUES ($1, true)",
        vec![Uuid::from_bytes(gateway).into()],
    )
    .await;
    let filer = store.report_filer(&token).await.unwrap();
    assert_eq!((filer.gateway, filer.official), (gateway, false));
    assert_eq!(
        store.report_filer(&Token::generate().unwrap()).await.err(),
        Some(Error::Rejected)
    );
    // a second administrator, whose marks and notes are their own
    let (_, other) = signed_in(store, keys, "report-admin-two").await;
    appoint_admin(operator, "report-admin-two").await.unwrap();
    let (first, second, crash) = filing(store, &filer).await;
    listing(store, &cookie, &stranger, first, crash).await;
    reading(store, &cookie, &stranger, first, crash).await;
    triage(store, &cookie, admin, owner, first).await;
    duplicates(store, &cookie, [first, second, crash]).await;
    notes(store, &cookie, &other, admin, first).await;
    binning(
        store,
        operator,
        &cookie,
        &other,
        admin,
        [first, second, crash],
    )
    .await;
    marking(store, &cookie, &other, admin, first).await;
    sweeping(store, operator, &cookie, first).await;
}

/// Filing: bounds, direct only from the bridge, parts only as announced and once.
async fn filing(
    store: &AccountStore,
    filer: &central_postgres::Filer,
) -> ([u8; 16], [u8; 16], [u8; 16]) {
    let client = gzip(3, 4_000);
    let record = gzip(5, 20_000);
    let first = store
        .file_report(
            filer,
            &intake(
                ReportKind::Bug,
                "The stack went wrong",
                &client,
                Some(&record),
            ),
        )
        .await
        .unwrap();
    let mut direct = intake(ReportKind::Bug, "Direct", &client, None);
    direct.channel = Channel::Direct;
    assert_eq!(
        store.file_report(filer, &direct).await.err(),
        Some(Error::Rejected),
        "direct only through the bridge"
    );
    let official = store.official_filer().await.unwrap();
    assert!(official.official);
    store.file_report(&official, &direct).await.unwrap();
    // the announced length, then the bytes: wrong length, wrong hash, not gzip
    assert_eq!(
        store
            .report_part_expected(filer, first, Part::Record)
            .await
            .unwrap(),
        20_000
    );
    let mut forged = record.clone();
    forged[100] ^= 1;
    for bytes in [
        record[..19_999].to_vec(),
        forged,
        gzip(5, 20_000).iter().map(|b| b ^ 0xff).collect(),
    ] {
        assert_eq!(
            store
                .file_report_part(filer, first, Part::Record, &bytes)
                .await
                .err(),
            Some(Error::Rejected)
        );
    }
    store
        .file_report_part(filer, first, Part::Record, &record)
        .await
        .unwrap();
    assert_eq!(
        store
            .file_report_part(filer, first, Part::Record, &record)
            .await
            .err(),
        Some(Error::Conflict)
    );
    assert_eq!(
        store
            .report_part_expected(filer, first, Part::Record)
            .await
            .err(),
        Some(Error::Conflict)
    );
    store
        .file_report_part(filer, first, Part::Client, &client)
        .await
        .unwrap();
    // another gateway's report and a part never announced
    assert_eq!(
        store
            .report_part_expected(&official, first, Part::Client)
            .await
            .err(),
        Some(Error::Rejected)
    );
    let second = store
        .file_report(
            filer,
            &intake(ReportKind::Improvement, "Another idea", &client, None),
        )
        .await
        .unwrap();
    assert_eq!(
        store
            .report_part_expected(filer, second, Part::Record)
            .await
            .err(),
        Some(Error::Rejected)
    );
    let crash = store
        .file_report(
            filer,
            &intake(ReportKind::Crash, "It crashed", &client, None),
        )
        .await
        .unwrap();
    (first, second, crash)
}

/// Listing: administrators only, the list's order, search and filters.
async fn listing(
    store: &AccountStore,
    cookie: &Token,
    stranger: &Token,
    first: [u8; 16],
    crash: [u8; 16],
) {
    assert_eq!(
        store
            .reports(stranger, &all(), ReportSort::Received, true, 0, 50)
            .await
            .err(),
        Some(Error::Rejected),
        "no administrator"
    );
    let (page, total) = store
        .reports(cookie, &all(), ReportSort::Received, true, 0, 50)
        .await
        .unwrap();
    assert!(total >= 4 && u64::try_from(page.len()).unwrap() == total.min(50));
    assert!(page.windows(2).all(|w| w[0].id > w[1].id), "newest first");
    let found = |q: &str| ReportFilter {
        q: Some(q.into()),
        ..all()
    };
    let (hits, _) = store
        .reports(
            cookie,
            &found("STACK WENT"),
            ReportSort::Received,
            true,
            0,
            50,
        )
        .await
        .unwrap();
    assert_eq!(hits.iter().map(|r| r.id).collect::<Vec<_>>(), [first]);
    let (hits, _) = store
        .reports(
            cookie,
            &found("out of range"),
            ReportSort::Received,
            true,
            0,
            50,
        )
        .await
        .unwrap();
    assert_eq!(
        hits.iter().map(|r| r.id).collect::<Vec<_>>(),
        [crash],
        "a crash's message is searched"
    );
    let crashes = ReportFilter {
        kinds: vec![ReportKind::Crash],
        ..all()
    };
    let (hits, _) = store
        .reports(cookie, &crashes, ReportSort::Received, true, 0, 50)
        .await
        .unwrap();
    assert!(hits.iter().all(|r| r.kind == ReportKind::Crash) && hits.iter().any(|r| r.id == crash));
    assert_eq!(
        store
            .reports(cookie, &all(), ReportSort::Received, true, 0, 201)
            .await
            .err(),
        Some(Error::Rejected)
    );
}

/// Reading one report: its detail, a part as stored, crash groups, facets,
/// the month's numbers and its place in a list.
async fn reading(
    store: &AccountStore,
    cookie: &Token,
    stranger: &Token,
    first: [u8; 16],
    crash: [u8; 16],
) {
    let detail = store.report_detail(cookie, first).await.unwrap();
    assert_eq!(detail.text, "The stack went wrong");
    assert!(
        detail.client.as_ref().is_some_and(|c| c.received)
            && detail.record.as_ref().is_some_and(|r| r.received)
    );
    assert_eq!(detail.audit[0].action, "report.filed");
    let (bytes, origin) = store
        .report_part(cookie, first, Part::Record)
        .await
        .unwrap();
    assert_eq!(
        (bytes, origin),
        (gzip(5, 20_000), Some(RecordOrigin::Gateway))
    );
    assert_eq!(
        store.report_part(stranger, first, Part::Record).await.err(),
        Some(Error::Rejected)
    );
    let groups = store.crash_groups(cookie, &all()).await.unwrap();
    assert!(
        groups
            .iter()
            .any(|g| g.latest == crash && g.count >= 1 && g.open >= 1)
    );
    let crashes = ReportFilter {
        kinds: vec![ReportKind::Crash],
        ..all()
    };
    let facets = store.report_facets(cookie, &crashes).await.unwrap();
    assert!(facets.kinds.len() > 1, "its own dimension is not filtered");
    assert!(facets.statuses.iter().all(|c| c.value == ReportStatus::New));
    assert!(
        store
            .report_stats(cookie)
            .await
            .unwrap()
            .iter()
            .any(|d| d.kind == ReportKind::Bug)
    );
    let place = store
        .report_neighbours(cookie, first, &all(), ReportSort::Received, true)
        .await
        .unwrap();
    assert!(place.position.is_some() && place.total >= 4);
}

/// Triage: the revision, the assignee, labels, priority and issue.
async fn triage(
    store: &AccountStore,
    cookie: &Token,
    admin: Identity,
    owner: Identity,
    first: [u8; 16],
) {
    let change = |f: fn(&mut ReportChange)| {
        let mut c = ReportChange::default();
        f(&mut c);
        c
    };
    let triaged = change(|c| c.status = Some(ReportStatus::Triaged));
    let now = store
        .set_report(cookie, first, &triaged, Some(0))
        .await
        .unwrap();
    assert_eq!(
        (now.summary.status, now.summary.revision),
        (ReportStatus::Triaged, 1)
    );
    assert_eq!(
        store
            .set_report(cookie, first, &triaged, Some(0))
            .await
            .err(),
        Some(Error::Conflict),
        "an older revision"
    );
    assert_eq!(
        store
            .set_report(cookie, first, &triaged, Some(1))
            .await
            .unwrap()
            .summary
            .revision,
        1,
        "nothing changed, nothing written"
    );
    let mut assign = ReportChange {
        assignee: Some(Some(owner.account)),
        ..ReportChange::default()
    };
    assert_eq!(
        store.set_report(cookie, first, &assign, None).await.err(),
        Some(Error::Rejected),
        "only an administrator"
    );
    assign.assignee = Some(Some(admin.account));
    assign.labels = Some(vec!["ui".into(), "lands".into(), "ui".into()]);
    assign.priority = Some(Some(Priority::High));
    assign.issue = Some(Some(311));
    let now = store
        .set_report(cookie, first, &assign, None)
        .await
        .unwrap();
    assert_eq!(now.summary.labels, ["lands", "ui"]);
    assert_eq!(
        (now.summary.assignee, now.summary.issue),
        (Some(admin.account), Some(311))
    );
}

/// One duplicate level; a report that becomes a duplicate hands its own on,
/// and unmarking one opens it again.
async fn duplicates(store: &AccountStore, cookie: &Token, [first, second, crash]: [[u8; 16]; 3]) {
    let dup = |of: [u8; 16]| ReportChange {
        duplicate_of: Some(Some(of)),
        ..ReportChange::default()
    };
    let now = store
        .set_report(cookie, second, &dup(first), None)
        .await
        .unwrap();
    assert_eq!(now.summary.status, ReportStatus::Duplicate);
    assert!(now.summary.done_at.is_some());
    assert_eq!(
        store
            .set_report(cookie, crash, &dup(second), None)
            .await
            .err(),
        Some(Error::Rejected),
        "the original is no duplicate"
    );
    store
        .set_report(cookie, first, &dup(crash), None)
        .await
        .unwrap();
    assert_eq!(
        store
            .report_detail(cookie, second)
            .await
            .unwrap()
            .summary
            .duplicate_of,
        Some(crash),
        "handed on"
    );
    let undone = store
        .set_report(
            cookie,
            first,
            &ReportChange {
                duplicate_of: Some(None),
                ..ReportChange::default()
            },
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        (undone.summary.status, undone.summary.done_at),
        (ReportStatus::Triaged, None)
    );
}

/// Notes: added, removed only by their author.
async fn notes(
    store: &AccountStore,
    cookie: &Token,
    other: &Token,
    admin: Identity,
    first: [u8; 16],
) {
    // notes: added, removed only by their author
    let noted = store
        .add_report_note(cookie, first, "Looks like the stack")
        .await
        .unwrap();
    let note = noted.notes[0].id;
    assert_eq!(noted.notes[0].author, admin.account);
    assert_eq!(
        store.remove_report_note(other, first, note).await.err(),
        Some(Error::Rejected)
    );
    let left = store.remove_report_note(cookie, first, note).await.unwrap();
    assert_eq!(left.notes.len(), 0);
}

/// A selection changed, the trash, and a purge only from it with a fresh password.
async fn binning(
    store: &AccountStore,
    operator: &DatabaseConnection,
    cookie: &Token,
    other: &Token,
    admin: Identity,
    [first, second, crash]: [[u8; 16]; 3],
) {
    let add = ReportChange {
        labels_add: vec!["batch".into()],
        ..ReportChange::default()
    };
    assert_eq!(
        store
            .reports_bulk(cookie, &[first, crash], &BulkAction::Change { change: add })
            .await
            .unwrap(),
        2
    );
    assert_eq!(
        store
            .reports_bulk(cookie, &[crash], &BulkAction::Trash)
            .await
            .unwrap(),
        1
    );
    let trash = ReportFilter {
        trash: true,
        ..all()
    };
    let (bin, _) = store
        .reports(cookie, &trash, ReportSort::Received, true, 0, 50)
        .await
        .unwrap();
    assert_eq!(bin.iter().map(|r| r.id).collect::<Vec<_>>(), [crash]);
    assert_eq!(
        store
            .bin_report(cookie, first, &BulkAction::Purge)
            .await
            .err(),
        Some(Error::Rejected),
        "not in the trash"
    );
    sql(
        operator,
        "UPDATE central_identity.sessions SET created_at = created_at - interval '31 minutes' WHERE account_id = $1 AND kind = 'browser'",
        vec![Uuid::from_bytes(admin.account).into()],
    )
    .await;
    assert_eq!(
        store
            .bin_report(cookie, crash, &BulkAction::Purge)
            .await
            .err(),
        Some(Error::Rejected),
        "a stale sign-in"
    );
    store
        .bin_report(cookie, crash, &BulkAction::Restore)
        .await
        .unwrap();
    store
        .bin_report(other, crash, &BulkAction::Trash)
        .await
        .unwrap();
    store
        .bin_report(other, crash, &BulkAction::Purge)
        .await
        .unwrap();
    assert_eq!(
        store.report_detail(cookie, crash).await.err(),
        Some(Error::Rejected)
    );
    // its duplicate keeps no original: opened again
    let freed = store.report_detail(cookie, second).await.unwrap().summary;
    assert_eq!(
        (freed.status, freed.duplicate_of, freed.done_at),
        (ReportStatus::Triaged, None, None)
    );
    assert!(
        count(operator, "SELECT count(*) AS n FROM central_identity.report_audit WHERE report_id = $1 AND action = 'report.purged'", vec![Uuid::from_bytes(crash).into()]).await == 1,
        "the history stays"
    );
}

/// Saved views, the unread marker (each administrator's own) and who may be assigned.
async fn marking(
    store: &AccountStore,
    cookie: &Token,
    other: &Token,
    admin: Identity,
    first: [u8; 16],
) {
    // views: saved, replaced by name, forgotten
    store
        .save_report_view(cookie, "Mine", "assignee=me")
        .await
        .unwrap();
    let views = store
        .save_report_view(cookie, "Mine", "assignee=me&status=new")
        .await
        .unwrap();
    assert_eq!(
        (views.len(), views[0].query.as_str()),
        (1, "assignee=me&status=new")
    );
    let left = store.delete_report_view(cookie, views[0].id).await.unwrap();
    assert_eq!(left.len(), 0);
    assert_eq!(
        store.save_report_view(cookie, " ", "x").await.err(),
        Some(Error::Rejected)
    );
    // the unread marker is each administrator's own
    store
        .set_reports_seen(cookie, &[first], true)
        .await
        .unwrap();
    let unseen = ReportFilter {
        unseen: true,
        ..all()
    };
    let (mine, _) = store
        .reports(cookie, &unseen, ReportSort::Received, true, 0, 50)
        .await
        .unwrap();
    assert!(!mine.iter().any(|r| r.id == first));
    let (theirs, _) = store
        .reports(other, &unseen, ReportSort::Received, true, 0, 50)
        .await
        .unwrap();
    assert!(theirs.iter().any(|r| r.id == first));
    assert!(
        store
            .report_admins(cookie)
            .await
            .unwrap()
            .iter()
            .any(|a| a.id == admin.account)
    );
}

/// The sweep: done for 90 days goes, its history stays.
async fn sweeping(
    store: &AccountStore,
    operator: &DatabaseConnection,
    cookie: &Token,
    done: [u8; 16],
) {
    let resolved = ReportChange {
        status: Some(ReportStatus::Resolved),
        ..ReportChange::default()
    };
    store
        .set_report(cookie, done, &resolved, None)
        .await
        .unwrap();
    sql(
        operator,
        "UPDATE central_identity.reports SET done_at = done_at - interval '91 days' WHERE id = $1",
        vec![Uuid::from_bytes(done).into()],
    )
    .await;
    store
        .reports(cookie, &all(), ReportSort::Received, true, 0, 50)
        .await
        .unwrap();
    assert_eq!(
        store.report_detail(cookie, done).await.err(),
        Some(Error::Rejected)
    );
    assert_eq!(
        count(operator, "SELECT count(*) AS n FROM central_identity.report_audit WHERE report_id = $1 AND action = 'report.swept'", vec![Uuid::from_bytes(done).into()]).await,
        1
    );
}

/// The runtime holds exactly its grants on the report tables: each wider and
/// each missing privilege refuses startup; it cannot rewrite a report's words
/// or a part, nor the history.
async fn grants(operator: &DatabaseConnection) {
    for (change, undo) in [
        (
            "GRANT UPDATE (text) ON central_identity.reports TO central_runtime",
            "REVOKE UPDATE (text) ON central_identity.reports FROM central_runtime",
        ),
        (
            "GRANT UPDATE ON central_identity.report_parts TO central_runtime",
            "REVOKE UPDATE ON central_identity.report_parts FROM central_runtime",
        ),
        (
            "GRANT DELETE ON central_identity.report_audit TO central_runtime",
            "REVOKE DELETE ON central_identity.report_audit FROM central_runtime",
        ),
        (
            "REVOKE DELETE ON central_identity.report_seen FROM central_runtime",
            "GRANT DELETE ON central_identity.report_seen TO central_runtime",
        ),
        (
            "REVOKE UPDATE (revision) ON central_identity.reports FROM central_runtime",
            "GRANT UPDATE (revision) ON central_identity.reports TO central_runtime",
        ),
    ] {
        operator.execute_unprepared(change).await.unwrap();
        let refused =
            AccountStore::connect_dev(config("central_runtime", "CENTRAL_TEST_RUNTIME_PASSWORD"))
                .await
                .is_err();
        operator.execute_unprepared(undo).await.unwrap();
        assert!(refused, "{change}");
    }
    let runtime =
        central_postgres::connect_dev(config("central_runtime", "CENTRAL_TEST_RUNTIME_PASSWORD"))
            .await
            .unwrap();
    for statement in [
        "UPDATE central_identity.reports SET text = 'rewritten'",
        "UPDATE central_identity.report_parts SET gzip = '\\x1f8b00'",
        "DELETE FROM central_identity.report_audit",
        "UPDATE central_identity.report_audit SET detail = ''",
    ] {
        assert!(
            runtime.execute_unprepared(statement).await.is_err(),
            "{statement}"
        );
    }
    runtime.close().await.unwrap();
}
