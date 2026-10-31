//! `--import-reports-stdin`: the one-time copy of the feedback service's open
//! reports (`docs/reports.md` §"Order", Baylee `docs/feedback.md` §"The
//! one-time copy to Central"), under the migrator login.
//!
//! Standard input holds the migration document first (`{"database": …}`,
//! as `--legacy-import-stdin` takes it), then `{"author": "<32 hex
//! digits>"}` (the administrator the copied notes are filed under) on a line
//! of its own, then `baylee-feedback export-reports`'s lines, the last of
//! them its count and SHA-256. One transaction: a line out of the rules, a
//! count or digest that is not the lines', anything after the last line or a
//! copy cut short stores nothing. Nothing is printed but counts and refusals
//! naming report ids.
use super::production::Database;
use central_api::reports::{
    Build, Channel, Crash, GatewaySays, MAX_CLIENT_GZIP, MAX_CLIENT_RECORD_GZIP, MAX_RECORD_GZIP,
    Priority, RecordOrigin, ReportKind, ReportStatus,
};
use central_postgres::{
    ImportedHistory, ImportedNote, ImportedReport, ReportImport, connect_server,
};
use serde::Deserialize;
use std::{io::BufRead, process::ExitCode};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// The copy's format, as its last line names it.
const FORMAT: &str = "baylee-feedback-reports/1";

/// The longest line: a record of 64 MiB of gzip as hex, and room around it.
const MAX_LINE: usize = 2 * MAX_RECORD_GZIP as usize + 1024 * 1024;

/// The most reports a copy holds.
const MAX_REPORTS: u64 = 100_000;

#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
struct Operator {
    database: Database,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Author {
    author: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordLine {
    complete: bool,
    origin: RecordOrigin,
    gzip: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NoteLine {
    at: i64,
    author: String,
    text: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AuditLine {
    at: i64,
    actor: String,
    action: String,
    detail: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Line {
    id: String,
    created_at: i64,
    updated_at: i64,
    gateway: String,
    gateway_says: GatewaySays,
    channel: Channel,
    reporter: String,
    kind: ReportKind,
    status: ReportStatus,
    text: String,
    game_id: Option<String>,
    build: Option<Build>,
    crash: Option<Crash>,
    issue: Option<u32>,
    priority: Option<Priority>,
    labels: Vec<String>,
    assignee: Option<String>,
    client_gzip: Option<String>,
    record: Option<RecordLine>,
    notes: Vec<NoteLine>,
    audit: Vec<AuditLine>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Trailer {
    format: String,
    count: u64,
    sha256: String,
}

/// Lowercase hex as bytes, at most `max` of them.
fn unhex(text: &str, max: usize) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) || text.len() / 2 > max {
        return None;
    }
    let digit = |b: u8| match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        _ => None,
    };
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|[high, low]| Some(digit(*high)? << 4 | digit(*low)?))
        .collect()
}

/// `line` as a report to store; or what of it cannot be read.
fn report(line: Line) -> Result<ImportedReport, (String, &'static str)> {
    let id: [u8; 16] = unhex(&line.id, 16)
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(|| (line.id.clone(), "id"))?;
    let client = line
        .client_gzip
        .as_deref()
        .map(|gzip| unhex(gzip, MAX_CLIENT_GZIP as usize).ok_or((line.id.clone(), "client")))
        .transpose()?;
    let record = line
        .record
        .map(|r| {
            let max = match r.origin {
                RecordOrigin::Gateway => MAX_RECORD_GZIP,
                RecordOrigin::Client => MAX_CLIENT_RECORD_GZIP,
            };
            unhex(&r.gzip, max as usize)
                .map(|gzip| (r.complete, r.origin, gzip))
                .ok_or((line.id.clone(), "record"))
        })
        .transpose()?;
    Ok(ImportedReport {
        id,
        created_at: line.created_at,
        updated_at: line.updated_at,
        gateway_label: line.gateway,
        gateway_says: line.gateway_says,
        channel: line.channel,
        reporter: line.reporter,
        kind: line.kind,
        status: line.status,
        text: line.text,
        game_id: line.game_id,
        build: line.build,
        crash: line.crash,
        issue: line.issue,
        priority: line.priority,
        labels: line.labels,
        assignee: line.assignee,
        client,
        record,
        notes: line
            .notes
            .into_iter()
            .map(|n| ImportedNote {
                at: n.at,
                author: n.author,
                text: n.text,
            })
            .collect(),
        history: line
            .audit
            .into_iter()
            .map(|a| ImportedHistory {
                at: a.at,
                actor: a.actor,
                action: a.action,
                detail: a.detail,
            })
            .collect(),
    })
}

/// The next line into `buf`, its line break kept; `false` at the end.
fn next_line(input: &mut impl BufRead, buf: &mut Vec<u8>) -> Result<bool, String> {
    buf.clear();
    let read = std::io::Read::take(&mut *input, MAX_LINE as u64 + 1)
        .read_until(b'\n', buf)
        .map_err(|_| "standard input unreadable".to_owned())?;
    if buf.len() > MAX_LINE {
        return Err("a line longer than a report can be".to_owned());
    }
    Ok(read > 0)
}

/// Import the copy once, and print what was stored.
pub(super) async fn run() -> ExitCode {
    match import(&mut std::io::stdin().lock()).await {
        Ok((imported, skipped)) => {
            eprintln!("central: reports import: {imported} stored, {skipped} already here");
            ExitCode::SUCCESS
        }
        Err(why) => {
            eprintln!("central: reports import refused, nothing was stored: {why}");
            ExitCode::FAILURE
        }
    }
}

/// The migration document and the author's, read no further than the line
/// the author's ends: the copy's lines follow untouched.
fn documents(input: &mut impl BufRead) -> Result<(Operator, [u8; 16]), String> {
    let mut documents = serde_json::Deserializer::from_reader(&mut *input);
    let operator = Operator::deserialize(&mut documents)
        .map_err(|_| "the first document is not the operator's {\"database\": …}".to_owned())?;
    let author = Author::deserialize(&mut documents)
        .map_err(|_| "the second document is not {\"author\": …}".to_owned())?;
    drop(documents);
    let author: [u8; 16] = unhex(&author.author, 16)
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or("the author is not an account id (32 hex digits)")?;
    let mut rest = Vec::new();
    if !next_line(input, &mut rest)? || !rest.iter().all(u8::is_ascii_whitespace) {
        return Err("the author's document is not on a line of its own".to_owned());
    }
    Ok((operator, author))
}

async fn import(input: &mut impl BufRead) -> Result<(u64, u64), String> {
    let (mut operator, author) = documents(input)?;
    let mut buf = Vec::new();
    let database = operator
        .database
        .server()
        .map_err(|()| "the database settings are invalid".to_owned())?;
    drop(operator);
    let db = connect_server(database)
        .await
        .map_err(|_| "the database is unavailable".to_owned())?;
    let copied = match ReportImport::begin(&db, author).await {
        Ok(import) => copy(import, input, &mut buf).await,
        Err(why) => Err(why.to_string()),
    };
    let closed = db.close().await;
    let copied = copied?;
    closed.map_err(|_| "the database did not close cleanly".to_owned())?;
    Ok(copied)
}

/// Every report line into one transaction, then the last line's check.
async fn copy(
    mut import: ReportImport,
    input: &mut impl BufRead,
    buf: &mut Vec<u8>,
) -> Result<(u64, u64), String> {
    let mut reports = 0_u64;
    loop {
        if !next_line(input, buf)? {
            return Err("the copy ends before its last line".to_owned());
        }
        if let Ok(line) = serde_json::from_slice::<Line>(buf) {
            reports += 1;
            if reports > MAX_REPORTS {
                return Err(format!("more than {MAX_REPORTS} reports"));
            }
            let report = report(line).map_err(|(id, field)| format!("report {id}: {field}"))?;
            import
                .add(buf, &report)
                .await
                .map_err(|why| why.to_string())?;
            continue;
        }
        let trailer: Trailer = serde_json::from_slice(buf)
            .map_err(|_| format!("line {} is neither a report nor the last line", reports + 2))?;
        if trailer.format != FORMAT {
            return Err(format!("the copy's format is not {FORMAT}"));
        }
        if next_line(input, buf)? {
            return Err("something follows the last line".to_owned());
        }
        return import
            .finish(trailer.count, &trailer.sha256)
            .await
            .map_err(|why| why.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_reads_only_lowercase_pairs_within_bounds() {
        assert_eq!(unhex("1f8b00", 3), Some(vec![0x1f, 0x8b, 0]));
        assert_eq!(unhex("1F8B", 2), None);
        assert_eq!(unhex("1f8", 2), None);
        assert_eq!(unhex("1f8b00", 2), None);
        assert_eq!(unhex("zz", 1), None);
    }

    #[test]
    fn the_operators_documents_come_first_and_the_lines_follow_untouched() {
        let input = concat!(
            "{\n  \"database\": {\"host\": \"127.0.0.1\", \"port\": 5432, \"name\": \"central\",\n",
            "    \"user\": \"central_migrator\", \"password\": \"x\", \"ca_certificate\": \"pem\"}\n}\n",
            "{\"author\": \"0192d0c57b507a1f8c2d3e4f5a6b7c01\"}\n",
            "{\"a\":1}\n",
        );
        let mut reader = std::io::BufReader::new(input.as_bytes());
        let (_, author) = documents(&mut reader).unwrap();
        assert_eq!(author[0], 0x01);
        let mut buf = Vec::new();
        assert!(next_line(&mut reader, &mut buf).unwrap());
        assert_eq!(buf, b"{\"a\":1}\n");
        let same_line = "{\"database\": {\"host\": \"h\", \"port\": 1, \"name\": \"n\", \"user\": \"u\", \"password\": \"p\", \"ca_certificate\": \"c\"}}\n{\"author\": \"0192d0c57b507a1f8c2d3e4f5a6b7c01\"} {\"a\":1}\n";
        let mut reader = std::io::BufReader::new(same_line.as_bytes());
        assert!(
            documents(&mut reader).is_err(),
            "a report on the author's line"
        );
    }

    #[test]
    fn a_line_is_read_whole_or_refused() {
        let text = b"one\ntwo";
        let mut input = std::io::BufReader::new(&text[..]);
        let mut buf = Vec::new();
        assert!(next_line(&mut input, &mut buf).unwrap());
        assert_eq!(buf, b"one\n");
        assert!(next_line(&mut input, &mut buf).unwrap());
        assert_eq!(buf, b"two");
        assert!(!next_line(&mut input, &mut buf).unwrap());
    }

    #[test]
    fn a_line_reads_as_a_report_with_its_parts() {
        let line = serde_json::json!({
            "id": "0192d0c57b507a1f8c2d3e4f5a6b7c01", "created_at": 1_761_900_000, "updated_at": 1_761_900_000,
            "gateway": "official", "gateway_says": {"name": "Baylee", "url": null, "version": "0.1.1"},
            "channel": "gateway", "reporter": "ab", "kind": "crash", "status": "triaged", "text": "boom",
            "game_id": null, "build": {"version": "0.1.1", "commit": null}, "crash": {"message": "boom", "location": "a.rs:1"},
            "issue": 311, "priority": "high", "labels": ["ui"], "assignee": "viktor",
            "client_gzip": "1f8b00", "record": {"complete": true, "origin": "client", "gzip": "1f8b01"},
            "notes": [{"at": 1, "author": "viktor", "text": "seen"}],
            "audit": [{"at": 1, "actor": "token", "action": "status", "detail": null}],
        });
        let line: Line = serde_json::from_value(line).unwrap();
        let report = report(line).unwrap();
        assert_eq!(report.client, Some(vec![0x1f, 0x8b, 0]));
        assert_eq!(
            report.record,
            Some((true, RecordOrigin::Client, vec![0x1f, 0x8b, 1]))
        );
        assert_eq!((report.notes.len(), report.history.len()), (1, 1));
        let unknown = serde_json::json!({"format": FORMAT, "count": 0, "sha256": "", "more": 1});
        assert!(
            serde_json::from_value::<Trailer>(unknown).is_err(),
            "nothing unknown is taken"
        );
    }
}
