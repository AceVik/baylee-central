//! Streaming Scryfall `all-cards` catalog import run by the operator
//! (`central-service --cards-ingest-stdin`). Never runs from HTTP request paths.

use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use central_api::social::{FINISH_ETCHED, FINISH_FOIL, FINISH_NONFOIL};
use central_postgres::{
    DevConfig,
    cards_import::{CardsImport, ImportPrinting, ImportSearchName},
    connect_dev, connect_server,
};
use flate2::read::GzDecoder;
use serde::Deserialize;
use zeroize::{Zeroize, ZeroizeOnDrop};

const BULK_META_URL: &str = "https://api.scryfall.com/bulk-data/all-cards";
const USER_AGENT: &str = "BayleeCentral-Catalog/0.1 (+https://baylee.app)";
const BATCH_PRINTINGS: usize = 500;
const BATCH_NAMES: usize = 500;
const UB_SETS: &[&str] = &[
    "ltr", "ltc", "who", "pip", "acr", "fin", "fic", "40k", "bot", "rex", "msh", "msc", "spe",
    "tla", "tle", "ptla", "clu", "spm", "mar",
];

#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
struct Local {
    port: u16,
    user: String,
    password: String,
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Connection {
    Tls(super::production::Database),
    LoopbackFixture(Local),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Settings {
    database: Connection,
    #[serde(default)]
    bulk_path: Option<PathBuf>,
}

#[derive(Deserialize)]
struct BulkMeta {
    jsonl_download_uri: String,
}

#[derive(Deserialize)]
struct RawFace {
    #[serde(default)]
    printed_name: Option<String>,
    #[serde(default)]
    image_uris: Option<serde::de::IgnoredAny>,
}

#[derive(Deserialize)]
struct RawCard {
    id: String,
    #[serde(default)]
    oracle_id: Option<String>,
    #[serde(default)]
    name: String,
    #[serde(default)]
    printed_name: Option<String>,
    #[serde(default)]
    lang: String,
    #[serde(default)]
    set: String,
    #[serde(default)]
    set_name: String,
    #[serde(default)]
    collector_number: String,
    #[serde(default)]
    released_at: String,
    #[serde(default)]
    finishes: Vec<String>,
    #[serde(default)]
    digital: bool,
    #[serde(default)]
    layout: String,
    #[serde(default)]
    image_status: String,
    #[serde(default)]
    security_stamp: Option<String>,
    #[serde(default)]
    promo_types: Vec<String>,
    #[serde(default)]
    image_uris: Option<serde::de::IgnoredAny>,
    #[serde(default)]
    card_faces: Vec<RawFace>,
}

fn parse_uuid(raw: &str) -> Option<[u8; 16]> {
    let s = raw.trim();
    if s.len() != 36 {
        return None;
    }
    let bytes = s.as_bytes();
    if bytes[8] != b'-' || bytes[13] != b'-' || bytes[18] != b'-' || bytes[23] != b'-' {
        return None;
    }
    let mut out = [0_u8; 16];
    let mut idx = 0_usize;
    let mut pos = 0_usize;
    while pos < bytes.len() {
        if matches!(pos, 8 | 13 | 18 | 23) {
            pos += 1;
            continue;
        }
        let hi = char::from(*bytes.get(pos)?).to_digit(16)?;
        let lo = char::from(*bytes.get(pos + 1)?).to_digit(16)?;
        out[idx] = u8::try_from((hi << 4) | lo).ok()?;
        idx += 1;
        pos += 2;
    }
    (idx == 16).then_some(out)
}

fn supported_lang(lang: &str) -> Option<&'static str> {
    match lang {
        "en" => Some("en"),
        "de" => Some("de"),
        "fr" => Some("fr"),
        "it" => Some("it"),
        "es" => Some("es"),
        "pt" => Some("pt"),
        "ru" => Some("ru"),
        "ja" => Some("ja"),
        "ko" => Some("ko"),
        "zhs" => Some("zhs"),
        "zht" => Some("zht"),
        _ => None,
    }
}

fn valid_date(date: &str) -> &str {
    let b = date.as_bytes();
    if b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b[..4].iter().all(u8::is_ascii_digit)
        && b[5..7].iter().all(u8::is_ascii_digit)
        && b[8..10].iter().all(u8::is_ascii_digit)
    {
        date
    } else {
        "1993-01-01"
    }
}

fn finish_bits(list: &[String]) -> i16 {
    let mut mask = 0_u8;
    for item in list {
        match item.as_str() {
            "nonfoil" => mask |= FINISH_NONFOIL,
            "foil" => mask |= FINISH_FOIL,
            "etched" => mask |= FINISH_ETCHED,
            _ => {}
        }
    }
    if mask == 0 {
        i16::from(FINISH_NONFOIL)
    } else {
        i16::from(mask)
    }
}

fn localized_name(card: &RawCard) -> Option<String> {
    if let Some(name) = card.printed_name.as_deref() {
        let trimmed = name.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_owned());
        }
    }
    let parts: Vec<&str> = card
        .card_faces
        .iter()
        .filter_map(|f| f.printed_name.as_deref().map(str::trim))
        .filter(|s| !s.is_empty())
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" // "))
    }
}

async fn connect(mut database: Connection) -> Result<central_postgres::DatabaseConnection, String> {
    match &mut database {
        Connection::Tls(config) => connect_server(
            config
                .server()
                .map_err(|()| "invalid TLS config".to_owned())?,
        )
        .await
        .map_err(|e| e.to_string()),
        Connection::LoopbackFixture(config) => {
            let dev = DevConfig::new(
                config.port,
                &config.user,
                std::mem::take(&mut config.password),
                &config.name,
            )
            .map_err(|_| "invalid loopback database config".to_owned())?;
            connect_dev(dev).await.map_err(|e| e.to_string())
        }
    }
}

fn open_bulk_stream(bulk_path: Option<&Path>) -> Result<Box<dyn Read>, String> {
    if let Some(path) = bulk_path {
        let file = File::open(path).map_err(|e| format!("failed to open bulk file: {e}"))?;
        let is_gz = path
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("gz"));
        if is_gz {
            return Ok(Box::new(GzDecoder::new(file)));
        }
        return Ok(Box::new(file));
    }

    let agent = central_outbound::agent(central_outbound::Limits {
        port: None,
        total: Duration::from_mins(15),
        connect: Duration::from_secs(15),
        pin: None,
    });
    let meta_res = agent
        .get(BULK_META_URL)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/json")
        .call()
        .map_err(|e| format!("Scryfall bulk metadata request failed: {e}"))?;
    if meta_res.status() != 200 {
        return Err(format!(
            "Scryfall bulk metadata returned HTTP {}",
            meta_res.status()
        ));
    }
    let meta_bytes = meta_res
        .into_body()
        .with_config()
        .limit(64 * 1024)
        .read_to_vec()
        .map_err(|e| format!("Failed reading Scryfall bulk metadata: {e}"))?;
    let meta: BulkMeta = serde_json::from_slice(&meta_bytes)
        .map_err(|e| format!("Invalid Scryfall bulk metadata JSON: {e}"))?;
    let uri = meta.jsonl_download_uri.trim().to_owned();
    let host_ok = uri
        .strip_prefix("https://")
        .and_then(|rest| rest.split('/').next())
        .is_some_and(|host| host == "data.scryfall.io" || host.ends_with(".scryfall.io"));
    if !host_ok {
        return Err("Unexpected Scryfall bulk download host".to_owned());
    }
    let data_res = agent
        .get(&uri)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/x-ndjson, application/gzip, */*")
        .call()
        .map_err(|e| format!("Scryfall bulk stream request failed: {e}"))?;
    if data_res.status() != 200 {
        return Err(format!(
            "Scryfall bulk stream returned HTTP {}",
            data_res.status()
        ));
    }
    Ok(Box::new(GzDecoder::new(data_res.into_body().into_reader())))
}

fn parse_card_line(
    line: &[u8],
    seen_names: &mut HashSet<([u8; 16], &'static str, String)>,
    printings: &mut Vec<ImportPrinting>,
    names: &mut Vec<ImportSearchName>,
) {
    let Ok(raw_text) = std::str::from_utf8(line) else {
        return;
    };
    let text = raw_text.trim();
    if !text.starts_with('{') {
        return;
    }
    let Ok(card) = serde_json::from_str::<RawCard>(text) else {
        return;
    };
    if card.digital
        || matches!(
            card.layout.as_str(),
            "token" | "double_faced_token" | "emblem" | "art_series" | "memorabilia"
        )
    {
        return;
    }
    let Some(lang) = supported_lang(&card.lang) else {
        return;
    };
    let Some(scryfall_id) = parse_uuid(&card.id) else {
        return;
    };
    let Some(oracle_id) = card.oracle_id.as_deref().and_then(parse_uuid) else {
        return;
    };
    let en_name = card.name.trim().to_owned();
    if en_name.is_empty() {
        return;
    }
    let set_code = card.set.trim().to_ascii_lowercase();
    let set_name = card.set_name.trim().to_owned();
    let collector_number = card.collector_number.trim().to_owned();
    if set_code.is_empty() || collector_number.is_empty() {
        return;
    }
    let local = (lang != "en").then(|| localized_name(&card)).flatten();
    let is_ub = card.security_stamp.as_deref() == Some("triangle")
        || UB_SETS.contains(&set_code.as_str())
        || card
            .promo_types
            .iter()
            .any(|p| p.eq_ignore_ascii_case("universesbeyond"));
    let has_image = card.image_status != "missing"
        && (card.image_uris.is_some()
            || card
                .card_faces
                .first()
                .is_some_and(|f| f.image_uris.is_some()));

    printings.push(ImportPrinting {
        scryfall_id,
        oracle_id,
        en_name: en_name.clone(),
        printed_name: local.clone(),
        lang: lang.to_owned(),
        set_code,
        set_name,
        collector_number,
        released_at: valid_date(card.released_at.trim()).to_owned(),
        finishes: finish_bits(&card.finishes),
        is_ub,
        has_image,
    });

    let search_label = if lang == "en" {
        Some(en_name.clone())
    } else {
        local
    };
    if let Some(label) = search_label {
        let lower = label.to_lowercase();
        if seen_names.insert((oracle_id, lang, lower)) {
            names.push(ImportSearchName {
                oracle_id,
                lang: lang.to_owned(),
                name: label,
                en_name,
            });
        }
    }
}

async fn ingest() -> Result<(usize, usize), String> {
    let raw = super::production::read_settings()
        .map_err(|()| "missing stdin configuration".to_owned())?;
    let Settings {
        database,
        bulk_path,
    } = serde_json::from_slice(&raw).map_err(|e| format!("invalid ingest settings: {e}"))?;

    let db = connect(database).await?;
    let stream = open_bulk_stream(bulk_path.as_deref())?;
    let mut reader = BufReader::with_capacity(256 * 1024, stream);
    let mut importer = CardsImport::begin(&db)
        .await
        .map_err(|e| format!("starting catalog transaction failed: {e}"))?;

    let mut line = Vec::with_capacity(8192);
    let mut printings = Vec::with_capacity(BATCH_PRINTINGS);
    let mut names = Vec::with_capacity(BATCH_NAMES);
    let mut seen_names: HashSet<([u8; 16], &'static str, String)> = HashSet::with_capacity(65_536);

    loop {
        line.clear();
        let n = reader
            .read_until(b'\n', &mut line)
            .map_err(|e| format!("reading Scryfall stream failed: {e}"))?;
        if n == 0 {
            break;
        }
        parse_card_line(&line, &mut seen_names, &mut printings, &mut names);

        if printings.len() >= BATCH_PRINTINGS {
            importer
                .insert_printings(&printings)
                .await
                .map_err(|e| format!("inserting printings failed: {e}"))?;
            printings.clear();
        }
        if names.len() >= BATCH_NAMES {
            importer
                .insert_search_names(&names)
                .await
                .map_err(|e| format!("inserting search names failed: {e}"))?;
            names.clear();
        }
    }

    importer
        .insert_printings(&printings)
        .await
        .map_err(|e| format!("inserting final printings failed: {e}"))?;
    importer
        .insert_search_names(&names)
        .await
        .map_err(|e| format!("inserting final search names failed: {e}"))?;

    let p_count = importer.printings;
    let n_count = importer.search_names;
    importer
        .finish()
        .await
        .map_err(|e| format!("committing catalog failed: {e}"))?;
    Ok((p_count, n_count))
}

pub(super) async fn run() -> ExitCode {
    match ingest().await {
        Ok((printings, names)) => {
            println!(
                "Card catalog imported ({printings} printings, {names} localized search entries)."
            );
            ExitCode::SUCCESS
        }
        Err(reason) => {
            eprintln!("Card catalog import failed: {reason}");
            ExitCode::FAILURE
        }
    }
}
