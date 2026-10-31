//! The legacy store's operator modes (`docs/operations.md` §"The legacy
//! store"), under the migrator login: the cutover's import and the deadline.
//! The sweep is `--sweep-stdin`'s.
//!
//! `--legacy-import-stdin` reads two JSON documents from standard input, the
//! operator's `{"database": …}` first and the official gateway's export
//! (`baylee-gateway legacy export`) right after it, so the export, which holds
//! every remaining password hash, travels through a pipe and never through a
//! file. Nothing is printed but counts, a digest and refusals naming legacy
//! ids.
use super::production::Database;
use central_api::legacy::MAX_SNAPSHOT_BYTES;
use central_postgres::{LegacyRow, MAX_LEGACY_ROWS, connect_server, import_legacy};
use serde::Deserialize;
use std::{io::Read, process::ExitCode};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

/// The export's format, as the gateway names it.
const FORMAT: &str = "baylee-legacy-accounts/1";

/// Upper bound on both documents together: the gateway's thousand accounts
/// would not fit, its six fit many times over; a snapshot byte is at most
/// four JSON bytes.
const MAX_IMPORT: usize = 32 * 1024 * 1024;

#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
struct Operator {
    database: Database,
}

#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
struct Deadline {
    database: Database,
    /// `YYYY-MM-DD`, the start of that day in UTC; `null` clears it.
    deadline: Option<String>,
}

#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
struct Export {
    format: String,
    accounts: Vec<Row>,
    count: u64,
    digest: [u8; 32],
}

#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
struct Row {
    legacy_id: String,
    username: String,
    display_name: Option<String>,
    tag: Option<u32>,
    password: Password,
    terms_version: Option<String>,
    must_change_password: bool,
    ban: Option<Ban>,
    snapshot: Vec<u8>,
}

#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
struct Password {
    salt: [u8; 16],
    hash: [u8; 32],
}

#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
struct Ban {
    until: Option<i64>,
}

/// The snapshot is the gateway's `{"preferences": {…}, "decks": […]}` and
/// nothing else; the client parses it again after decryption.
fn snapshot_shaped(snapshot: &[u8]) -> bool {
    if snapshot.len() > MAX_SNAPSHOT_BYTES {
        return false;
    }
    let Ok(serde_json::Value::Object(document)) = serde_json::from_slice(snapshot) else {
        return false;
    };
    document.len() == 2
        && document
            .get("preferences")
            .is_some_and(serde_json::Value::is_object)
        && document
            .get("decks")
            .is_some_and(serde_json::Value::is_array)
}

fn row(row: &mut Row) -> Result<LegacyRow, String> {
    if !snapshot_shaped(&row.snapshot) {
        return Err(format!("{}: snapshot malformed", row.legacy_id));
    }
    Ok(LegacyRow {
        legacy_id: std::mem::take(&mut row.legacy_id),
        username: std::mem::take(&mut row.username),
        display_name: row.display_name.take(),
        tag: row.tag,
        salt: row.password.salt,
        hash: row.password.hash,
        terms_version: row.terms_version.take(),
        must_change_password: row.must_change_password,
        ban: row.ban.as_ref().map(|ban| ban.until),
        snapshot: std::mem::take(&mut row.snapshot),
    })
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    })
}

fn read_input() -> Result<Zeroizing<Vec<u8>>, String> {
    let mut bytes = Zeroizing::new(Vec::new());
    std::io::stdin()
        .lock()
        .take(MAX_IMPORT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "standard input unreadable".to_owned())?;
    if bytes.len() > MAX_IMPORT {
        return Err("input larger than 32 MiB".into());
    }
    Ok(bytes)
}

/// Import the gateway's export once, and print what was stored.
pub(super) async fn run_import() -> ExitCode {
    match import().await {
        Ok((count, digest)) => {
            eprintln!("central: legacy import: {count} accounts, digest {digest}");
            ExitCode::SUCCESS
        }
        Err(why) => {
            eprintln!("central: legacy import refused, nothing was stored: {why}");
            ExitCode::FAILURE
        }
    }
}

async fn import() -> Result<(u64, String), String> {
    let bytes = read_input()?;
    let mut documents = serde_json::Deserializer::from_slice(&bytes);
    let mut operator = Operator::deserialize(&mut documents)
        .map_err(|_| "the first document is not the operator's {\"database\": …}".to_owned())?;
    let mut export = Export::deserialize(&mut documents)
        .map_err(|_| "the second document is not the gateway's export".to_owned())?;
    documents
        .end()
        .map_err(|_| "something follows the export".to_owned())?;
    drop(bytes);
    if export.format != FORMAT {
        return Err(format!("the export's format is not {FORMAT}"));
    }
    if export.accounts.len() > MAX_LEGACY_ROWS {
        return Err(format!("more than {MAX_LEGACY_ROWS} accounts"));
    }
    let rows = export
        .accounts
        .iter_mut()
        .map(row)
        .collect::<Result<Vec<_>, _>>()?;
    let database = operator
        .database
        .server()
        .map_err(|()| "the database settings are invalid".to_owned())?;
    drop(operator);
    let db = connect_server(database)
        .await
        .map_err(|_| "the database is unavailable".to_owned())?;
    let imported = import_legacy(&db, rows, export.count, export.digest).await;
    let closed = db.close().await;
    let imported = imported.map_err(|refusal| refusal.to_string())?;
    closed.map_err(|_| "the database did not close cleanly".to_owned())?;
    Ok((imported.count, hex(&imported.digest)))
}

/// Set or clear the deadline, and print it.
pub(super) async fn run_deadline() -> ExitCode {
    match deadline().await {
        Ok(Some(at)) => {
            eprintln!("central: legacy deadline at Unix {at}");
            ExitCode::SUCCESS
        }
        Ok(None) => {
            eprintln!("central: legacy deadline cleared");
            ExitCode::SUCCESS
        }
        Err(()) => {
            eprintln!(
                "central: legacy deadline rejected configuration or date (YYYY-MM-DD), or failed"
            );
            ExitCode::FAILURE
        }
    }
}

async fn deadline() -> Result<Option<i64>, ()> {
    let bytes = super::production::read_settings()?;
    let mut settings: Deadline = serde_json::from_slice(&bytes).map_err(|_| ())?;
    drop(bytes);
    let database = settings.database.server()?;
    let db = connect_server(database).await.map_err(|_| ())?;
    let set = central_postgres::set_legacy_deadline(&db, settings.deadline.as_deref()).await;
    let closed = db.close().await;
    let set = set.map_err(|_| ())?;
    closed.map_err(|_| ())?;
    Ok(set)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_gateways_two_keys_are_a_snapshot() {
        assert!(snapshot_shaped(br#"{"preferences":{},"decks":[]}"#));
        for bad in [
            &br#"{"preferences":{},"decks":{}}"#[..],
            br#"{"preferences":[],"decks":[]}"#,
            br#"{"preferences":{},"decks":[],"extra":1}"#,
            br#"{"decks":[]}"#,
            br"[]",
            b"not json",
        ] {
            assert!(!snapshot_shaped(bad), "{}", String::from_utf8_lossy(bad));
        }
        let mut large = br#"{"preferences":{"p":""#.to_vec();
        large.extend(std::iter::repeat_n(b'x', MAX_SNAPSHOT_BYTES));
        large.extend(br#""},"decks":[]}"#);
        assert!(!snapshot_shaped(&large));
    }

    #[test]
    fn both_documents_are_read_in_order_and_nothing_after() {
        let database = r#"{"host":"127.0.0.1","port":5432,"name":"baylee_central","user":"central_migrator","password":"PRIVATE_DB_CANARY","ca_certificate":"x"}"#;
        let export = r#"{"format":"baylee-legacy-accounts/1","accounts":[],"count":0,"digest":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]}"#;
        let input = format!("{{\"database\":{database}}}\n{export}\n");
        let mut documents = serde_json::Deserializer::from_slice(input.as_bytes());
        assert!(Operator::deserialize(&mut documents).is_ok());
        let parsed = Export::deserialize(&mut documents).unwrap();
        assert_eq!(parsed.count, 0);
        assert!(documents.end().is_ok());
        let trailing = format!("{{\"database\":{database}}}{export}{{}}");
        let mut documents = serde_json::Deserializer::from_slice(trailing.as_bytes());
        Operator::deserialize(&mut documents).unwrap();
        Export::deserialize(&mut documents).unwrap();
        assert!(documents.end().is_err());
    }
}
