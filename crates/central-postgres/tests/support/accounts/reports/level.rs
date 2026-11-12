//! What reports may take in the store (the store's `reports/level.rs`): a
//! registered gateway's bound to the byte, the official gateway's, and
//! everyone's; what never came stops counting after its hour and what is
//! stored does not; a report's going frees its room; one filing is weighed
//! at a time.
use super::{AccountStore, DatabaseConnection, Error, Uuid, fresh_id, gzip, intake, sql};
use central_api::reports::{Part, ReportIntake, ReportKind};
use central_postgres::{ALL_STORED, Filer, GATEWAY_STORED, OFFICIAL_STORED, REPORT_BYTES};
use sea_orm::{ConnectionTrait, DbBackend, Statement, TransactionTrait};

/// The parts at their largest: 2 MiB of `client`, 64 MiB of record.
const CLIENT: u64 = 2 * 1024 * 1024;
const RECORD: u64 = 64 * 1024 * 1024;
/// What a report announcing both counts as.
const LARGEST: u64 = REPORT_BYTES + CLIENT + RECORD;

/// A gateway nobody registered: a report holds no reference to its
/// gateway, and the store weighs a filer's reports by its id.
async fn gateway(operator: &DatabaseConnection, official: bool) -> Filer {
    Filer {
        gateway: *fresh_id(operator).await.as_bytes(),
        label: "Weighed".into(),
        official,
    }
}

/// A report announcing `client` and `record` bytes (none for 0), of which
/// nothing is sent here.
fn announcing(client: u64, record: u64) -> ReportIntake {
    let mut report = intake(ReportKind::Other, "Weighed", &gzip(1, 2), Some(&gzip(2, 2)));
    match report.client.as_mut() {
        Some(part) if client > 0 => part.bytes = u32::try_from(client).unwrap(),
        _ => report.client = None,
    }
    match report.record.as_mut() {
        Some(part) if record > 0 => part.bytes = u32::try_from(record).unwrap(),
        _ => report.record = None,
    }
    report
}

/// `many` reports of `gateway`, each announcing both parts at their
/// largest and sending neither, written by the operator: no bound is asked.
async fn pile(operator: &DatabaseConnection, gateway: [u8; 16], many: u64) {
    sql(
        operator,
        "INSERT INTO central_identity.reports (gateway_id, gateway_label, channel, reporter, kind, text,
             client_bytes, client_sha256, record_bytes, record_sha256, record_complete, record_origin)
         SELECT $1, 'Weighed', 'gateway', 'ab', 'other', 'announced and never sent', $2,
             pg_catalog.sha256(''::bytea), $3, pg_catalog.sha256(''::bytea), false, 'gateway'
         FROM pg_catalog.generate_series(1, $4::bigint)",
        vec![
            Uuid::from_bytes(gateway).into(),
            i32::try_from(CLIENT).unwrap().into(),
            i32::try_from(RECORD).unwrap().into(),
            i64::try_from(many).unwrap().into(),
        ],
    )
    .await;
}

/// The reports of `gateway` were filed more than an hour ago.
async fn age(operator: &DatabaseConnection, gateway: [u8; 16]) {
    sql(
        operator,
        "UPDATE central_identity.reports SET created_at = created_at - interval '3601 seconds'
         WHERE gateway_id = $1",
        vec![Uuid::from_bytes(gateway).into()],
    )
    .await;
}

async fn theirs(store: &AccountStore, filer: &Filer) -> u64 {
    store.report_levels(filer.gateway).await.unwrap().0
}

async fn everyone(store: &AccountStore, filer: &Filer) -> u64 {
    store.report_levels(filer.gateway).await.unwrap().1
}

pub(super) async fn run(store: &AccountStore, operator: &DatabaseConnection) {
    let one = registered(store, operator).await;
    stored_and_awaited(store, operator, &one).await;
    official(store, operator).await;
    runbook(store, operator, &one).await;
    all(store, operator, &one).await;
    one_at_a_time(store, operator).await;
    sql(
        operator,
        "DELETE FROM central_identity.reports WHERE gateway_label = 'Weighed'",
        vec![],
    )
    .await;
}

/// The runbook's query (`ops/report-levels.sql`) counts as a filing is
/// weighed: a gateway's line, and the line without a gateway for all.
async fn runbook(store: &AccountStore, operator: &DatabaseConnection, filer: &Filer) {
    let lines = operator
        .query_all_raw(Statement::from_string(
            DbBackend::Postgres,
            include_str!("../../../../../../ops/report-levels.sql"),
        ))
        .await
        .unwrap();
    let counted = |gateway: Option<Uuid>| {
        lines
            .iter()
            .find(|line| line.try_get::<Option<Uuid>>("", "gateway_id").unwrap() == gateway)
            .map(|line| line.try_get::<i64>("", "bytes").unwrap())
            .and_then(|bytes| u64::try_from(bytes).ok())
    };
    let (theirs, everyone) = store.report_levels(filer.gateway).await.unwrap();
    assert!(theirs > 0 && everyone > theirs);
    assert_eq!(counted(Some(Uuid::from_bytes(filer.gateway))), Some(theirs));
    assert_eq!(counted(None), Some(everyone));
}

/// A registered gateway's bound, to the byte; another gateway is not
/// refused for it.
async fn registered(store: &AccountStore, operator: &DatabaseConnection) -> Filer {
    let one = gateway(operator, false).await;
    let before = everyone(store, &one).await;
    assert_eq!(theirs(store, &one).await, 0);
    pile(operator, one.gateway, 31).await;
    assert_eq!(theirs(store, &one).await, 31 * LARGEST);
    assert_eq!(everyone(store, &one).await, before + 31 * LARGEST);
    // exactly a report without parts is left of its 2 GiB
    assert_eq!(GATEWAY_STORED - 31 * LARGEST, REPORT_BYTES);
    assert_eq!(
        store.file_report(&one, &announcing(1, 0)).await.err(),
        Some(Error::Busy),
        "one byte past a gateway's bound"
    );
    assert_eq!(theirs(store, &one).await, 31 * LARGEST, "refused whole");
    store.file_report(&one, &announcing(0, 0)).await.unwrap();
    assert_eq!(theirs(store, &one).await, GATEWAY_STORED);
    assert_eq!(
        store.file_report(&one, &announcing(0, 0)).await.err(),
        Some(Error::Busy)
    );
    let other = gateway(operator, false).await;
    store
        .file_report(&other, &announcing(CLIENT, RECORD))
        .await
        .unwrap();
    assert_eq!(theirs(store, &other).await, LARGEST);
    one
}

/// What was announced and never came stops counting when it can no longer
/// be stored; what is stored counts however old; a report's going frees
/// its room.
async fn stored_and_awaited(store: &AccountStore, operator: &DatabaseConnection, one: &Filer) {
    age(operator, one.gateway).await;
    assert_eq!(theirs(store, one).await, 32 * REPORT_BYTES);
    store
        .file_report(one, &announcing(CLIENT, RECORD))
        .await
        .unwrap();
    assert_eq!(theirs(store, one).await, 32 * REPORT_BYTES + LARGEST);
    let sender = gateway(operator, false).await;
    let record = gzip(7, 20_000);
    let report = store
        .file_report(
            &sender,
            &intake(ReportKind::Other, "Sent", &gzip(3, 4_000), Some(&record)),
        )
        .await
        .unwrap();
    store
        .file_report_part(&sender, report, Part::Record, &record)
        .await
        .unwrap();
    assert_eq!(theirs(store, &sender).await, REPORT_BYTES + 4_000 + 20_000);
    age(operator, sender.gateway).await;
    assert_eq!(
        theirs(store, &sender).await,
        REPORT_BYTES + 20_000,
        "the record is stored; the client part never came"
    );
    sql(
        operator,
        "DELETE FROM central_identity.reports WHERE id = $1",
        vec![Uuid::from_bytes(report).into()],
    )
    .await;
    assert_eq!(theirs(store, &sender).await, 0);
}

/// The official gateway's bound is four times a registered one's.
async fn official(store: &AccountStore, operator: &DatabaseConnection) {
    let official = gateway(operator, true).await;
    let fit = OFFICIAL_STORED / LARGEST;
    assert!(fit > 4 * 30);
    pile(operator, official.gateway, fit - 1).await;
    store
        .file_report(&official, &announcing(CLIENT, RECORD))
        .await
        .unwrap();
    assert_eq!(
        store
            .file_report(&official, &announcing(CLIENT, RECORD))
            .await
            .err(),
        Some(Error::Busy)
    );
    assert_eq!(theirs(store, &official).await, fit * LARGEST);
}

/// Everyone's bound refuses a gateway whose own reports take little, and
/// no longer when room was made.
async fn all(store: &AccountStore, operator: &DatabaseConnection, one: &Filer) {
    let room = ALL_STORED - everyone(store, one).await;
    let crowd = gateway(operator, false).await;
    pile(operator, crowd.gateway, room / LARGEST - 1).await;
    let late = gateway(operator, false).await;
    store
        .file_report(&late, &announcing(CLIENT, RECORD))
        .await
        .unwrap();
    assert_eq!(
        store
            .file_report(&late, &announcing(CLIENT, RECORD))
            .await
            .err(),
        Some(Error::Busy),
        "everyone's bound"
    );
    assert_eq!(theirs(store, &late).await, LARGEST);
    let level = everyone(store, &late).await;
    assert!(level <= ALL_STORED && level + LARGEST > ALL_STORED);
    sql(
        operator,
        "DELETE FROM central_identity.reports WHERE gateway_id = $1",
        vec![Uuid::from_bytes(crowd.gateway).into()],
    )
    .await;
    store
        .file_report(&late, &announcing(CLIENT, RECORD))
        .await
        .unwrap();
}

/// A filing that finds another being weighed is `Busy`, and of several at
/// once for the last room exactly one is filed.
async fn one_at_a_time(store: &AccountStore, operator: &DatabaseConnection) {
    let racer = gateway(operator, false).await;
    let held = operator.begin().await.unwrap();
    held.execute_raw(Statement::from_string(
        DbBackend::Postgres,
        "SELECT pg_catalog.pg_advisory_xact_lock(1862384219, 2)",
    ))
    .await
    .unwrap();
    assert_eq!(
        store.file_report(&racer, &announcing(0, 0)).await.err(),
        Some(Error::Busy),
        "another filing is being weighed"
    );
    held.rollback().await.unwrap();
    assert_eq!(theirs(store, &racer).await, 0);
    // room for exactly one report without parts, asked for four times at once
    pile(operator, racer.gateway, 31).await;
    let report = announcing(0, 0);
    let filed = tokio::join!(
        store.file_report(&racer, &report),
        store.file_report(&racer, &report),
        store.file_report(&racer, &report),
        store.file_report(&racer, &report),
    );
    let filed = [filed.0, filed.1, filed.2, filed.3];
    assert_eq!(filed.iter().filter(|answer| answer.is_ok()).count(), 1);
    assert!(
        filed
            .iter()
            .all(|answer| answer.is_ok() || answer == &Err(Error::Busy))
    );
    assert_eq!(theirs(store, &racer).await, GATEWAY_STORED);
}
