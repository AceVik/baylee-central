//! The one-time copy of the feedback service's open reports (Baylee
//! `docs/feedback.md` §"The one-time copy to Central", `docs/reports.md`
//! §"Order"), under the migrator login, in one transaction: each report
//! checked by the intake's own rules and stored with its parts, its notes
//! and its history. A report whose id is here already is skipped, so a
//! second run copies nothing twice; anything wrong stores nothing.
//!
//! What has no account here keeps its name in words: a note is filed under
//! the administrator who runs the copy, its first author named in its text;
//! a line of the old history keeps its time, its actor named in its detail;
//! an assignee is named in the report's own first line of history.
use super::super::{statement, uuid};
use central_api::reports::{
    Announced, AnnouncedRecord, Build, Channel, Crash, GatewaySays, MAX_LABELS, MAX_NOTE, Priority,
    RecordOrigin, ReportIntake, ReportKind, ReportStatus, valid_label,
};
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DatabaseTransaction, TransactionTrait, prelude::Uuid,
};
use sha2::{Digest, Sha256};

/// The most a line of history may say (the table's own bound).
const MAX_DETAIL: usize = 500;

/// A note of the service's, by the name of the admin who wrote it there.
#[derive(Clone, Debug)]
pub struct ImportedNote {
    /// Unix seconds.
    pub at: i64,
    /// The admin's name at the service.
    pub author: String,
    /// Its words.
    pub text: String,
}

/// A line of the service's triage history.
#[derive(Clone, Debug)]
pub struct ImportedHistory {
    /// Unix seconds.
    pub at: i64,
    /// An admin's name, or `token`.
    pub actor: String,
    /// What was done.
    pub action: String,
    /// To what.
    pub detail: Option<String>,
}

/// One report of the copy, as its line holds it, its parts decoded.
#[derive(Clone, Debug)]
pub struct ImportedReport {
    /// The service's id, a UUID v7, kept.
    pub id: [u8; 16],
    /// Unix seconds.
    pub created_at: i64,
    /// Unix seconds.
    pub updated_at: i64,
    /// The name the service filed it under.
    pub gateway_label: String,
    /// What the gateway said of itself.
    pub gateway_says: GatewaySays,
    /// How it came.
    pub channel: Channel,
    /// The pseudonym.
    pub reporter: String,
    /// What it is about.
    pub kind: ReportKind,
    /// `new`, `triaged` or `in_progress`.
    pub status: ReportStatus,
    /// The player's words.
    pub text: String,
    /// The gateway's game.
    pub game_id: Option<String>,
    /// The client's build.
    pub build: Option<Build>,
    /// A crash's message and place.
    pub crash: Option<Crash>,
    /// The issue it is filed as.
    pub issue: Option<u32>,
    /// Its priority.
    pub priority: Option<Priority>,
    /// Its labels.
    pub labels: Vec<String>,
    /// Whom the service assigned it to, by name.
    pub assignee: Option<String>,
    /// The client object, gzip.
    pub client: Option<Vec<u8>>,
    /// The record: whether complete, who wrote it, gzip.
    pub record: Option<(bool, RecordOrigin, Vec<u8>)>,
    /// Its notes.
    pub notes: Vec<ImportedNote>,
    /// Its history.
    pub history: Vec<ImportedHistory>,
}

/// Why a copy stored nothing.
#[derive(Debug, PartialEq, Eq)]
pub enum ReportImportRefusal {
    /// The database did not answer.
    Unavailable,
    /// No official gateway is listed to file the copied reports under.
    NoOfficialGateway,
    /// The account named to file the notes under is no administrator.
    NotAnAdministrator,
    /// The last line's count is not the number of reports before it.
    CountMismatch,
    /// The last line's SHA-256 is not that of the lines before it.
    DigestMismatch,
    /// A report outside the intake's rules: its id and what.
    Malformed {
        /// The report, 32 hex digits.
        id: String,
        /// What is wrong.
        field: &'static str,
    },
}

impl std::fmt::Display for ReportImportRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable => f.write_str("the database is unavailable"),
            Self::NoOfficialGateway => f.write_str("no official gateway is listed"),
            Self::NotAnAdministrator => f.write_str("the author is no administrator"),
            Self::CountMismatch => f.write_str("the count is not the number of reports"),
            Self::DigestMismatch => f.write_str("the SHA-256 is not that of the reports"),
            Self::Malformed { id, field } => write!(f, "report {id}: {field}"),
        }
    }
}

fn announced(gzip: &[u8]) -> Option<Announced> {
    Some(Announced {
        bytes: u32::try_from(gzip.len()).ok()?,
        sha256: Sha256::digest(gzip).into(),
    })
}

fn chars(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

/// `report` checked by the intake's rules and the triage's; the field that is not.
fn check(report: &ImportedReport) -> Result<(), &'static str> {
    if !super::super::gateways::valid_id(report.id) {
        return Err("id");
    }
    if report.created_at <= 0 || report.updated_at < report.created_at {
        return Err("times");
    }
    if report.gateway_label.is_empty()
        || report.gateway_label.len() > 256
        || report.gateway_label.chars().any(char::is_control)
    {
        return Err("gateway");
    }
    if !matches!(
        report.status,
        ReportStatus::New | ReportStatus::Triaged | ReportStatus::InProgress
    ) {
        return Err("status");
    }
    let intake = ReportIntake {
        gateway: report.gateway_says.clone(),
        channel: report.channel,
        reporter: report.reporter.clone(),
        kind: report.kind,
        text: report.text.clone(),
        game_id: report.game_id.clone(),
        build: report.build.clone(),
        crash: report.crash.clone(),
        client: report
            .client
            .as_deref()
            .map(announced)
            .map(|a| a.ok_or("client"))
            .transpose()?,
        record: report
            .record
            .as_ref()
            .map(|(complete, origin, gzip)| {
                let a = announced(gzip).ok_or("record")?;
                Ok::<_, &'static str>(AnnouncedRecord {
                    complete: *complete,
                    origin: *origin,
                    bytes: a.bytes,
                    sha256: a.sha256,
                })
            })
            .transpose()?,
    };
    if intake.validate().is_err() {
        return Err("metadata");
    }
    let gzip =
        |bytes: &Option<Vec<u8>>| bytes.as_ref().is_none_or(|b| b.starts_with(&[0x1f, 0x8b]));
    if !gzip(&report.client)
        || !report
            .record
            .as_ref()
            .is_none_or(|(_, _, b)| b.starts_with(&[0x1f, 0x8b]))
    {
        return Err("parts");
    }
    if report.issue.is_some_and(|n| n == 0 || n > 10_000_000) {
        return Err("issue");
    }
    let mut labels = report.labels.clone();
    labels.sort_unstable();
    labels.dedup();
    if labels.len() != report.labels.len()
        || labels.len() > MAX_LABELS
        || !labels.iter().all(|l| valid_label(l))
    {
        return Err("labels");
    }
    if report
        .notes
        .iter()
        .any(|n| n.text.trim().is_empty() || n.author.is_empty())
    {
        return Err("notes");
    }
    Ok(())
}

/// `report`'s history as lines of Central's: each old line at its time,
/// its action as Central writes one (`imported` when nothing is left of
/// it), its actor named in its detail; then a line saying where it came
/// from and whom it was assigned to there.
fn history_lines(report: &ImportedReport) -> Vec<(Option<i64>, String, String)> {
    let mut lines: Vec<_> = report
        .history
        .iter()
        .map(|line| {
            let action: String = line
                .action
                .chars()
                .filter(|c| c.is_ascii_lowercase() || *c == '_')
                .take(32)
                .collect();
            let action = if action.is_empty() {
                "imported".to_owned()
            } else {
                action
            };
            let actor = chars(&line.actor, 64);
            let detail = line
                .detail
                .as_ref()
                .map_or_else(|| actor.clone(), |detail| format!("{actor}: {detail}"));
            (Some(line.at), action, chars(&detail, MAX_DETAIL))
        })
        .collect();
    let assignee = report
        .assignee
        .as_deref()
        .map(|a| format!(", assigned there to {}", chars(a, 64)))
        .unwrap_or_default();
    lines.push((
        None,
        "imported".to_owned(),
        chars(&format!("from the feedback service{assignee}"), MAX_DETAIL),
    ));
    lines
}

/// An import on its way: one transaction, the official gateway the reports
/// are filed under, and the administrator their notes are.
pub struct ReportImport {
    tx: DatabaseTransaction,
    gateway: Uuid,
    author: Uuid,
    imported: u64,
    skipped: u64,
    /// Every line taken, its line break included, for the last line's check.
    digest: Sha256,
    lines: u64,
}

impl ReportImport {
    /// Opens the transaction, with `author`, who must be an administrator.
    ///
    /// # Errors
    /// [`ReportImportRefusal`]; nothing is stored then.
    pub async fn begin(
        db: &DatabaseConnection,
        author: [u8; 16],
    ) -> Result<Self, ReportImportRefusal> {
        let tx = db
            .begin()
            .await
            .map_err(|_| ReportImportRefusal::Unavailable)?;
        let gateway: Uuid = tx
            .query_one_raw(statement(
                "SELECT id FROM central_identity.gateways WHERE official",
                vec![],
            ))
            .await
            .map_err(|_| ReportImportRefusal::Unavailable)?
            .ok_or(ReportImportRefusal::NoOfficialGateway)?
            .try_get("", "id")
            .map_err(|_| ReportImportRefusal::Unavailable)?;
        let admin = tx
            .query_one_raw(statement(
                "SELECT 1 AS yes FROM central_identity.account_roles WHERE account_id = $1 AND role = 'admin'",
                vec![uuid(author).into()],
            ))
            .await
            .map_err(|_| ReportImportRefusal::Unavailable)?;
        if admin.is_none() {
            return Err(ReportImportRefusal::NotAnAdministrator);
        }
        Ok(Self {
            tx,
            gateway,
            author: uuid(author),
            imported: 0,
            skipped: 0,
            digest: Sha256::new(),
            lines: 0,
        })
    }

    /// Stores `report`, read from `line` (its line break included), or
    /// skips it when its id is here already.
    ///
    /// # Errors
    /// [`ReportImportRefusal::Malformed`] for a report outside the rules,
    /// `Unavailable` for the database; the copy stores nothing then.
    pub async fn add(
        &mut self,
        line: &[u8],
        report: &ImportedReport,
    ) -> Result<(), ReportImportRefusal> {
        self.digest.update(line);
        self.lines += 1;
        let named = || central_api::reports::hex(&report.id);
        check(report).map_err(|field| ReportImportRefusal::Malformed { id: named(), field })?;
        let id = uuid(report.id);
        let record = report.record.as_ref();
        let stored = self
            .tx
            .query_one_raw(statement(
                "INSERT INTO central_identity.reports (id, created_at, updated_at, gateway_id, gateway_label,
                     gateway_says, gateway_url, gateway_version, channel, reporter, kind, status, text, game_id,
                     build_version, build_commit, crash_message, crash_location, signature, client_bytes,
                     client_sha256, record_bytes, record_sha256, record_complete, record_origin, issue_number,
                     priority, labels)
                 VALUES ($1, pg_catalog.to_timestamp($2), pg_catalog.to_timestamp($3), $4, $5, $6, $7, $8, $9, $10,
                     $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27,
                     COALESCE(pg_catalog.string_to_array(NULLIF($28::text, ''), ','), '{}'))
                 ON CONFLICT (id) DO NOTHING RETURNING id",
                vec![
                    id.into(),
                    report.created_at.into(),
                    report.updated_at.into(),
                    self.gateway.into(),
                    report.gateway_label.clone().into(),
                    report.gateway_says.name.clone().into(),
                    report.gateway_says.url.clone().into(),
                    report.gateway_says.version.clone().into(),
                    report.channel.as_str().into(),
                    report.reporter.clone().into(),
                    report.kind.as_str().into(),
                    report.status.as_str().into(),
                    report.text.clone().into(),
                    report.game_id.clone().into(),
                    report.build.as_ref().map(|b| b.version.clone()).into(),
                    report.build.as_ref().and_then(|b| b.commit.clone()).into(),
                    report.crash.as_ref().map(|c| c.message.clone()).into(),
                    report.crash.as_ref().map(|c| c.location.clone()).into(),
                    report.crash.as_ref().map(|c| central_api::reports::crash_signature(&c.message, &c.location)).into(),
                    report.client.as_ref().map(|b| i32::try_from(b.len()).unwrap_or(i32::MAX)).into(),
                    report.client.as_ref().map(|b| Sha256::digest(b).to_vec()).into(),
                    record.map(|(_, _, b)| i32::try_from(b.len()).unwrap_or(i32::MAX)).into(),
                    record.map(|(_, _, b)| Sha256::digest(b).to_vec()).into(),
                    record.map(|(complete, _, _)| *complete).into(),
                    record.map(|(_, origin, _)| origin.as_str()).into(),
                    report.issue.map(|n| i32::try_from(n).unwrap_or(i32::MAX)).into(),
                    report.priority.map(Priority::as_str).into(),
                    report.labels.join(",").into(),
                ],
            ))
            .await
            .map_err(|_| ReportImportRefusal::Unavailable)?;
        if stored.is_none() {
            self.skipped += 1;
            return Ok(());
        }
        self.extras(id, report).await?;
        self.imported += 1;
        Ok(())
    }

    /// `report`'s parts, notes and history, after its row.
    async fn extras(&self, id: Uuid, report: &ImportedReport) -> Result<(), ReportImportRefusal> {
        let down = |_| ReportImportRefusal::Unavailable;
        let record = report.record.as_ref().map(|(_, _, b)| b);
        for (part, gzip) in [("client", report.client.as_ref()), ("record", record)] {
            let Some(gzip) = gzip else { continue };
            self.tx
                .execute_raw(statement(
                    "INSERT INTO central_identity.report_parts (report_id, part, gzip) VALUES ($1, $2, $3)",
                    vec![id.into(), part.into(), gzip.clone().into()],
                ))
                .await
                .map_err(down)?;
        }
        for note in &report.notes {
            let text = chars(
                &format!(
                    "{} (feedback service): {}",
                    chars(&note.author, 64),
                    note.text
                ),
                MAX_NOTE,
            );
            self.tx
                .execute_raw(statement(
                    "INSERT INTO central_identity.report_notes (report_id, at, author, text)
                     VALUES ($1, pg_catalog.to_timestamp($2), $3, $4)",
                    vec![id.into(), note.at.into(), self.author.into(), text.into()],
                ))
                .await
                .map_err(down)?;
        }
        for (at, action, detail) in history_lines(report) {
            self.tx
                .execute_raw(statement(
                    "INSERT INTO central_identity.report_audit (report_id, at, actor, action, detail)
                     VALUES ($1, COALESCE(pg_catalog.to_timestamp($2), pg_catalog.clock_timestamp()), NULL, $3, $4)",
                    vec![id.into(), at.into(), format!("report.{action}").into(), detail.into()],
                ))
                .await
                .map_err(down)?;
        }
        Ok(())
    }

    /// Stores the copy when the last line's `count` and `sha256` (64 hex
    /// digits) are those of the lines taken; how many reports were stored
    /// and how many skipped.
    ///
    /// # Errors
    /// `CountMismatch`, `DigestMismatch`, `Unavailable`; nothing is stored then.
    pub async fn finish(self, count: u64, sha256: &str) -> Result<(u64, u64), ReportImportRefusal> {
        if count != self.lines {
            return Err(ReportImportRefusal::CountMismatch);
        }
        if central_api::reports::hex(&self.digest.clone().finalize()) != sha256 {
            return Err(ReportImportRefusal::DigestMismatch);
        }
        self.tx
            .commit()
            .await
            .map_err(|_| ReportImportRefusal::Unavailable)?;
        Ok((self.imported, self.skipped))
    }
}
