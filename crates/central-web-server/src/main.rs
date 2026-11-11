//! `central-web`: baylee.app's web server.
//!
//! ```text
//! central-web --config-stdin            deployed: the settings as JSON on standard input
//! central-web --dev <site> [<fallback>] development: 127.0.0.1:28792, no GitHub
//! ```
#![forbid(unsafe_code)]

use std::io::Read as _;
use std::path::PathBuf;
use std::process::ExitCode;

use central_web_server::config::Config;
use central_web_server::{app, fetch, routes, views};

fn settings() -> Result<Config, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let config = match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["--config-stdin"] => {
            let mut text = String::new();
            std::io::stdin()
                .take(64 * 1024)
                .read_to_string(&mut text)
                .map_err(|_| "the settings could not be read".to_owned())?;
            serde_json::from_str(&text).map_err(|_| "the settings are malformed".to_owned())?
        }
        ["--dev", site] => Config::development(PathBuf::from(site), None),
        ["--dev", site, fallback] => {
            Config::development(PathBuf::from(site), Some(PathBuf::from(fallback)))
        }
        _ => {
            return Err("usage: central-web --config-stdin | --dev <site> [<fallback>]".to_owned());
        }
    };
    config.validate().map_err(|invalid| invalid.0.to_owned())?;
    Ok(config)
}

async fn shutdown() {
    let interrupt = tokio::signal::ctrl_c();
    #[cfg(unix)]
    {
        let Ok(mut terminate) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        else {
            return;
        };
        tokio::select! {
            _ = interrupt => {}
            _ = terminate.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = interrupt.await;
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> ExitCode {
    let config = match settings() {
        Ok(config) => config,
        Err(message) => {
            eprintln!("central-web: {message}");
            return ExitCode::FAILURE;
        }
    };
    let listen = config.listen;
    let app = match app(config) {
        Ok(app) => app,
        Err(broken) => {
            eprintln!("central-web: {}", broken.0);
            return ExitCode::FAILURE;
        }
    };
    fetch::spawn(&app.config, &app.data);
    let counted = app.views.clone();
    if let Some(counted) = &counted {
        views::spawn(counted, &app.data);
    }
    let Ok(listener) = tokio::net::TcpListener::bind(listen).await else {
        eprintln!("central-web: cannot listen on {listen}");
        return ExitCode::FAILURE;
    };
    println!("central-web: listening on {listen}");
    let served = axum::serve(listener, routes::router(app))
        .with_graceful_shutdown(shutdown())
        .await;
    // what was counted since the last minute
    if let Some(counted) = counted {
        counted.save();
    }
    match served {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}
