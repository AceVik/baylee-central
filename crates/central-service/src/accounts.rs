//! Explicit synthetic development service; secret configuration arrives over stdin, never argv.
use central_api::{Endpoint, Token};
use central_crypto::opaque::server::ServerKeys;
use central_http::DevelopmentServer;
use central_identity::IdentityService;
use central_policy::config::Config;
use central_postgres::{AccountStore, DevConfig};
use serde::Deserialize;
use std::{io::Read, process::ExitCode, sync::Arc};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
struct Settings {
    endpoint: String,
    gateway: String,
    database_port: u16,
    database_user: String,
    database_name: String,
    database_password: String,
    opaque_setup: Vec<u8>,
    bridge: Token,
}

pub(super) async fn run() -> ExitCode {
    if serve().await.is_ok() {
        ExitCode::SUCCESS
    } else {
        eprintln!("central: development account service rejected configuration or failed");
        ExitCode::FAILURE
    }
}

async fn serve() -> Result<(), ()> {
    // Bounded input. The invoking developer owns secure persistence/permissions of
    // this synthetic configuration; restarting must reuse the same OPAQUE setup.
    let mut bytes = Zeroizing::new(Vec::new());
    std::io::stdin()
        .lock()
        .take(16_385)
        .read_to_end(&mut bytes)
        .map_err(|_| ())?;
    if bytes.len() > 16_384 {
        return Err(());
    }
    let mut settings: Settings = serde_json::from_slice(&bytes).map_err(|_| ())?;
    let endpoint = Endpoint::development(&settings.endpoint).map_err(|_| ())?;
    let listen = endpoint.origin().strip_prefix("http://").ok_or(())?;
    let config = Config::from_pairs([
        ("BAYLEE_CENTRAL_LISTEN", listen),
        ("BAYLEE_CENTRAL_REQUEST_BYTES", "1056768"),
    ])
    .map_err(|_| ())?;
    let keys = ServerKeys::from_secret_bytes(&settings.opaque_setup).map_err(|_| ())?;
    let database = DevConfig::new(
        settings.database_port,
        &settings.database_user,
        std::mem::take(&mut settings.database_password),
        &settings.database_name,
    )
    .map_err(|_| ())?;
    let store = AccountStore::connect_dev(database).await.map_err(|_| ())?;
    let identity = IdentityService::new(
        store.clone(),
        &endpoint,
        keys,
        &settings.bridge,
        &settings.gateway,
    );
    drop(settings);
    let result = async {
        let identity = identity.map_err(|_| ())?;
        let server = DevelopmentServer::bind(config).await.map_err(|_| ())?;
        println!(
            "central: development account listener at http://{}",
            server.local_addr()
        );
        server
            .run_with_identity(&endpoint, Arc::new(identity), async {
                let _ = tokio::signal::ctrl_c().await;
            })
            .await
            .map_err(|_| ())
    }
    .await;
    let closed = store.close().await.map_err(|_| ());
    result.and(closed)
}
