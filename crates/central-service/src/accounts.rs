//! Explicit synthetic development service; secret configuration arrives over stdin, never argv.
use super::listeners::{stopped, together};
use central_api::{Endpoint, Token};
use central_crypto::opaque::server::ServerKeys;
use central_http::{BROWSER_REQUEST_BYTES, BrowserOrigin, DevelopmentServer};
use central_identity::IdentityService;
use central_policy::config::Config;
use central_postgres::{AccountStore, DevConfig};
use serde::Deserialize;
use std::{io::Read, process::ExitCode, sync::Arc};
use tokio::sync::watch;
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
    /// A browser listener for a page served locally; absent means off.
    #[serde(default)]
    browser: Option<Browser>,
}

/// The development browser listener: a numeric loopback address, and the
/// page's numeric loopback `http` origin, which replaces `baylee.app`.
#[derive(Deserialize, Zeroize)]
#[serde(deny_unknown_fields)]
struct Browser {
    listen: String,
    origin: String,
}

impl Browser {
    fn config(&self) -> Result<(Config, BrowserOrigin), ()> {
        let bytes = BROWSER_REQUEST_BYTES.to_string();
        let config = Config::from_pairs([
            ("BAYLEE_CENTRAL_LISTEN", self.listen.as_str()),
            ("BAYLEE_CENTRAL_REQUEST_BYTES", bytes.as_str()),
        ])
        .map_err(|_| ())?;
        Ok((
            config,
            BrowserOrigin::development(&self.origin).map_err(|_| ())?,
        ))
    }
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
    let browser = settings.browser.as_ref().map(Browser::config).transpose()?;
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
        let identity = Arc::new(identity.map_err(|_| ())?);
        let server = DevelopmentServer::bind(config).await.map_err(|_| ())?;
        let browser = match browser {
            Some((config, site)) => {
                Some((DevelopmentServer::bind(config).await.map_err(|_| ())?, site))
            }
            None => None,
        };
        println!(
            "central: development account listener at http://{}",
            server.local_addr()
        );
        if let Some((browser, site)) = &browser {
            println!(
                "central: development browser listener at http://{} for {}",
                browser.local_addr(),
                site.origin()
            );
        }
        let (stop, _) = watch::channel(false);
        let native = server.run_with_identity(&endpoint, identity.clone(), stopped(&stop));
        let browser =
            browser.map(|(browser, site)| browser.run_browser(identity, site, stopped(&stop)));
        let interrupted = async {
            let _ = tokio::signal::ctrl_c().await;
        };
        together(&stop, native, browser, interrupted)
            .await
            .map_err(|_| ())
    }
    .await;
    let closed = store.close().await.map_err(|_| ());
    result.and(closed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(extra: &str) -> String {
        let setup =
            serde_json::to_string(ServerKeys::generate().to_secret_bytes().as_slice()).unwrap();
        let bridge = serde_json::to_string(&[7_u8; 32]).unwrap();
        format!(
            r#"{{"endpoint":"http://127.0.0.1:28780","gateway":"http://127.0.0.1:28766","database_port":5432,"database_user":"central_runtime","database_name":"central_dev","database_password":"PRIVATE_DB_CANARY","opaque_setup":{setup},"bridge":{bridge}{extra}}}"#
        )
    }

    fn parse(text: &str) -> Result<Settings, serde_json::Error> {
        serde_json::from_str(text)
    }

    #[test]
    fn the_development_browser_listener_is_optional_and_loopback_only() {
        assert!(
            parse(&settings("")).unwrap().browser.is_none(),
            "absent means off"
        );
        let with = |listen: &str, origin: &str| {
            parse(&settings(&format!(
                r#","browser":{{"listen":"{listen}","origin":"{origin}"}}"#
            )))
            .unwrap()
        };
        let parsed = with("127.0.0.1:28791", "http://127.0.0.1:5173");
        let (config, site) = parsed.browser.as_ref().unwrap().config().unwrap();
        assert_eq!(config.listen().port(), 28_791);
        assert_eq!(site.origin(), "http://127.0.0.1:5173");
        let ipv6 = with("[::1]:28791", "http://[::1]:5173");
        assert_eq!(
            ipv6.browser.as_ref().unwrap().config().unwrap().1.origin(),
            "http://[::1]:5173"
        );
        for (listen, origin) in [
            ("0.0.0.0:28791", "http://127.0.0.1:5173"),
            ("127.0.0.1:28791", "https://baylee.app"),
            ("127.0.0.1:28791", "http://localhost:5173"),
            ("127.0.0.1:28791", "https://127.0.0.1:5173"),
            ("127.0.0.1:28791", "http://127.0.0.1"),
            ("127.0.0.1:28791", "http://192.168.1.2:5173"),
            ("127.0.0.1:28791", "http://127.0.0.1:5173/app"),
            ("127.0.0.1:28791", "http://user@127.0.0.1:5173"),
        ] {
            let parsed = with(listen, origin);
            assert!(
                parsed.browser.as_ref().unwrap().config().is_err(),
                "{listen} {origin}"
            );
        }
        for extra in [
            r#","browser":{"listen":"127.0.0.1:28791"}"#,
            r#","browser":{"listen":"127.0.0.1:28791","origin":"http://127.0.0.1:5173","host":"baylee.app"}"#,
        ] {
            assert!(parse(&settings(extra)).is_err(), "{extra}");
        }
    }
}
