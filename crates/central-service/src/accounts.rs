//! Explicit synthetic development service; secret configuration arrives over stdin, never argv.
use super::listeners::{stopped, together};
use central_api::{Endpoint, Token};
use central_crypto::opaque::server::ServerKeys;
use central_http::{BROWSER_REQUEST_BYTES, BrowserOrigin, DevelopmentServer};
use central_identity::IdentityService;
use central_policy::config::Config;
use central_postgres::{AccountStore, DevConfig, LegacyStore};
use serde::Deserialize;
use std::{io::Read, process::ExitCode, sync::Arc};
use tokio::sync::watch;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
struct Settings {
    endpoint: String,
    gateway: String,
    /// Further numeric-loopback origins of the official gateway, as a live
    /// service has its compiled aliases; absent means none.
    #[serde(default)]
    gateway_aliases: Vec<String>,
    database_port: u16,
    database_user: String,
    database_name: String,
    database_password: String,
    opaque_setup: Vec<u8>,
    bridge: Token,
    /// A browser listener for a page served locally; absent means off.
    #[serde(default)]
    browser: Option<Browser>,
    /// The legacy store's own login on the same loopback database
    /// (`docs/native-login.md` §"Legacy login"); absent means no legacy
    /// sign-in.
    #[serde(default)]
    legacy: Option<Legacy>,
    /// A mailer on this host (`docs/email.md`), its site a numeric loopback
    /// origin; absent means no mail.
    #[serde(default)]
    mail: Option<super::mail::Mail>,
    /// The picture worker's binary (`docs/profile-pictures.md`), an
    /// absolute path, started as a child for each upload; absent means the
    /// `central-picture-worker` beside this program, when there is one.
    #[serde(default)]
    picture_worker: Option<String>,
}

/// The development picture worker: the configured binary, or the one built
/// beside this program; none when neither is a file.
fn picture_worker(configured: Option<&str>) -> Result<Option<central_picture::Worker>, ()> {
    if let Some(path) = configured {
        let path = std::path::PathBuf::from(path);
        if !path.is_absolute() || !path.is_file() {
            return Err(());
        }
        return Ok(Some(central_picture::Worker::program(path)));
    }
    let name = format!("central-picture-worker{}", std::env::consts::EXE_SUFFIX);
    Ok(std::env::current_exe()
        .ok()
        .map(|exe| exe.with_file_name(name))
        .filter(|path| path.is_file())
        .map(central_picture::Worker::program))
}

/// The development legacy store's role and password.
#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
struct Legacy {
    database_user: String,
    database_password: String,
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

/// The legacy store's login on the same loopback database, if configured.
fn legacy_config(settings: &mut Settings) -> Result<Option<DevConfig>, ()> {
    let port = settings.database_port;
    let name = settings.database_name.clone();
    settings
        .legacy
        .as_mut()
        .map(|legacy| {
            DevConfig::new(
                port,
                &legacy.database_user,
                std::mem::take(&mut legacy.database_password),
                &name,
            )
            .map_err(|_| ())
        })
        .transpose()
}

/// The legacy store, if configured; one that will not open closes the
/// account store too.
async fn legacy_store(
    store: &AccountStore,
    config: Option<DevConfig>,
) -> Result<Option<LegacyStore>, ()> {
    let Some(config) = config else {
        return Ok(None);
    };
    if let Ok(legacy) = LegacyStore::connect_dev(config).await {
        Ok(Some(legacy))
    } else {
        let _ = store.clone().close().await;
        Err(())
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
    let mailer = settings
        .mail
        .as_mut()
        .map(|mail| mail.client(true))
        .transpose()?;
    let legacy = legacy_config(&mut settings)?;
    let pictures = picture_worker(settings.picture_worker.as_deref())?;
    let store = AccountStore::connect_dev(database).await.map_err(|_| ())?;
    let legacy = legacy_store(&store, legacy).await?;
    let identity = IdentityService::new(
        store.clone(),
        &endpoint,
        keys,
        &settings.bridge,
        &settings.gateway,
    )
    .and_then(|identity| identity.with_aliases(settings.gateway_aliases.clone()))
    .map(|identity| match &legacy {
        Some(legacy) => identity.with_legacy(legacy.clone()),
        None => identity,
    })
    .map(|identity| match mailer {
        Some(mailer) => identity.with_mailer(mailer),
        None => identity,
    })
    .map(|identity| match pictures {
        Some(worker) => identity.with_pictures(worker),
        None => identity,
    });
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
        let mail = super::mail::run(identity.clone(), stop.subscribe());
        let native = server.run_with_identity(&endpoint, identity.clone(), stopped(&stop));
        let browser =
            browser.map(|(browser, site)| browser.run_browser(identity, site, stopped(&stop)));
        let interrupted = async {
            let _ = tokio::signal::ctrl_c().await;
        };
        let (result, ()) = tokio::join!(together(&stop, native, browser, interrupted), mail);
        result.map_err(|_| ())
    }
    .await;
    let closed = store.close().await.map_err(|_| ());
    let legacy_closed = match legacy {
        Some(legacy) => legacy.close().await.map_err(|_| ()),
        None => Ok(()),
    };
    result.and(closed).and(legacy_closed)
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

    #[test]
    fn the_development_mailer_is_optional_and_on_loopback_only() {
        assert!(
            parse(&settings("")).unwrap().mail.is_none(),
            "absent means no mail"
        );
        let token = "01234567".repeat(8);
        let with = |endpoint: &str, site: &str| {
            parse(&settings(&format!(
                r#","mail":{{"endpoint":"{endpoint}","site":"{site}","token":"{token}"}}"#
            )))
            .unwrap()
        };
        let mut parsed = with("http://127.0.0.1:28795", "http://127.0.0.1:5173");
        assert!(parsed.mail.as_mut().unwrap().client(true).is_ok());
        for (endpoint, site) in [
            ("http://192.168.1.2:28795", "http://127.0.0.1:5173"),
            ("https://127.0.0.1:28795", "http://127.0.0.1:5173"),
            ("http://127.0.0.1:28795", "https://baylee.app"),
            ("http://127.0.0.1:28795", "http://localhost:5173"),
        ] {
            let mut parsed = with(endpoint, site);
            assert!(
                parsed.mail.as_mut().unwrap().client(true).is_err(),
                "{endpoint} {site}"
            );
        }
        for extra in [
            r#","mail":{"endpoint":"http://127.0.0.1:28795"}"#,
            r#","mail":{"endpoint":"http://127.0.0.1:28795","site":"http://127.0.0.1:5173","token":"x","from":"central@example.org"}"#,
        ] {
            assert!(parse(&settings(extra)).is_err(), "{extra}");
        }
    }
}
