//! Launcher: explicit local development modes and the deployed account service.
//! Every mode is named by one argument; secrets arrive over stdin, never argv.
#![forbid(unsafe_code)]

use central_http::DevelopmentServer;
use central_policy::config::{Config, is_reserved_setting};
use std::process::ExitCode;

mod accounts;
mod legacy;
mod listeners;
mod production;

const USAGE: &str = "usage: central-service --dev | --dev-accounts-stdin | --accounts-stdin | --migrate-stdin | --issue-beta-keys-stdin | --appoint-admin-stdin | --sweep-stdin | --legacy-import-stdin | --legacy-deadline-stdin | --generate-secrets";

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
        Some("--issue-beta-keys-stdin") => return production::run_issue_beta_keys().await,
        Some("--appoint-admin-stdin") => return production::run_appoint_admin().await,
        Some("--sweep-stdin") => return production::run_sweep().await,
        Some("--legacy-import-stdin") => return legacy::run_import().await,
        Some("--legacy-deadline-stdin") => return legacy::run_deadline().await,
        Some("--generate-secrets") => return production::generate_secrets(),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::FAILURE;
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
