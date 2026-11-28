//! Launcher: explicit local development modes and the deployed account service.
//! Every mode is named by one argument; secrets arrive over stdin, never argv.
#![forbid(unsafe_code)]

use central_http::DevelopmentServer;
use central_policy::config::{Config, is_reserved_setting};
use std::process::ExitCode;

mod accounts;
mod advent_import;
mod ai_players;
mod cards_ingest;
mod funding;
mod gateway_push;
mod legacy;
mod listeners;
mod mail;
mod production;
mod push;
mod reports_import;
mod restore;

/// Every mode, in the order the usage names them; `--modes` prints them.
const MODES: [&str; 20] = [
    "--dev",
    "--dev-accounts-stdin",
    "--accounts-stdin",
    "--check-accounts-stdin",
    "--migrate-stdin",
    "--appoint-admin-stdin",
    "--ai-players-stdin",
    "--sweep-stdin",
    "--rerank-stdin",
    "--after-restore-stdin",
    "--erase-accounts-stdin",
    "--legacy-import-stdin",
    "--legacy-deadline-stdin",
    "--import-reports-stdin",
    "--advent-import-stdin",
    "--ranked-cosmetics-import-stdin",
    "--cards-ingest-stdin",
    "--generate-secrets",
    "--generate-token-key",
    "--modes",
];

/// A mode that is not one, or an argument after it: the usage, and this
/// status, which no mode ends with.
const USAGE_STATUS: u8 = 2;

#[tokio::main]
async fn main() -> ExitCode {
    // Do not let a future dependency panic print potentially sensitive payloads.
    std::panic::set_hook(Box::new(|_| eprintln!("central: internal failure")));
    let mut args = std::env::args_os().skip(1);
    let command = args.next();
    let command = if args.next().is_none() {
        command.as_deref().and_then(std::ffi::OsStr::to_str)
    } else {
        None
    };
    match command {
        Some("--dev") => {}
        Some("--dev-accounts-stdin") => return accounts::run().await,
        Some("--accounts-stdin") => return production::run_service().await,
        Some("--migrate-stdin") => return production::run_migration().await,
        Some("--appoint-admin-stdin") => return production::run_appoint_admin().await,
        Some("--ai-players-stdin") => return ai_players::run().await,
        Some("--sweep-stdin") => return production::run_sweep().await,
        Some("--rerank-stdin") => return production::run_rerank().await,
        Some("--after-restore-stdin") => return restore::run_after_restore().await,
        Some("--erase-accounts-stdin") => return restore::run_erase().await,
        Some("--legacy-import-stdin") => return legacy::run_import().await,
        Some("--legacy-deadline-stdin") => return legacy::run_deadline().await,
        Some("--import-reports-stdin") => return reports_import::run().await,
        Some("--advent-import-stdin") => return advent_import::run().await,
        Some("--ranked-cosmetics-import-stdin") => return advent_import::ranked::run().await,
        Some("--cards-ingest-stdin") => return cards_ingest::run().await,
        Some("--check-accounts-stdin") => return production::run_check(),
        Some("--generate-secrets") => return production::generate_secrets(),
        Some("--generate-token-key") => return production::generate_token_key(),
        Some("--modes") => return modes(),
        _ => {
            eprintln!("usage: central-service {}", MODES.join(" | "));
            return ExitCode::from(USAGE_STATUS);
        }
    }
    let mut values = Vec::new();
    for (key, value) in std::env::vars_os() {
        if is_reserved_setting(&key.to_string_lossy()) {
            let (Some(key), Some(value)) = (key.to_str(), value.to_str()) else {
                eprintln!("central: invalid configuration");
                return ExitCode::FAILURE;
            };
            values.push((key.to_owned(), value.to_owned()));
        }
    }
    let Ok(config) = Config::from_pairs(
        values
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str())),
    ) else {
        eprintln!("central: invalid configuration");
        return ExitCode::FAILURE;
    };
    let Ok(server) = DevelopmentServer::bind(config).await else {
        eprintln!("central: local listener unavailable");
        return ExitCode::FAILURE;
    };
    println!(
        "central: development listener at http://{}",
        server.local_addr()
    );
    let result = server
        .run(async {
            // Signal registration failure closes the listener instead of hanging.
            let _ = tokio::signal::ctrl_c().await;
        })
        .await;
    if result.is_err() {
        eprintln!("central: listener stopped after an internal failure");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

/// The modes this build knows, as one JSON line, so that a tool can ask
/// before it relies on one (an older build ends `--modes` with its usage).
fn modes() -> ExitCode {
    match serde_json::to_string(&serde_json::json!({ "modes": MODES })) {
        Ok(line) => {
            println!("{line}");
            ExitCode::SUCCESS
        }
        Err(_) => ExitCode::FAILURE,
    }
}
