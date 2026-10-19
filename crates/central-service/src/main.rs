//! Local-development launcher. No production mode, password endpoints or storage.
#![forbid(unsafe_code)]

use central_http::DevelopmentServer;
use central_policy::config::{Config, is_reserved_setting};
use std::process::ExitCode;

mod accounts;

#[tokio::main]
async fn main() -> ExitCode {
    // Do not let a future dependency panic print potentially sensitive payloads.
    std::panic::set_hook(Box::new(|_| eprintln!("central: internal failure")));
    let mut args = std::env::args_os().skip(1);
    let command = args.next();
    if command.as_deref() == Some(std::ffi::OsStr::new("--dev-accounts-stdin"))
        && args.next().is_none()
    {
        return accounts::run().await;
    }
    if command.as_deref() != Some(std::ffi::OsStr::new("--dev")) || args.next().is_some() {
        eprintln!("usage: central-service --dev (local foundation only)");
        return ExitCode::FAILURE;
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
