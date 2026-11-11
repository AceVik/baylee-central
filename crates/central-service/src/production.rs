//! Deployed operation: the live account service, explicit migrations,
//! appointing an administrator, the audit's sweep and secret generation. Configuration arrives over stdin, never argv
//! or the environment.
//!
//! The endpoint is always [`Endpoint::live`] and the database is reached only over
//! verified TLS ([`ServerConfig`]). A supervisor such as systemd opens the
//! operator's root-owned configuration file and hands it over as standard input,
//! so the service identity itself never needs read access to that file.
use super::listeners::{stopped, together};
use central_api::{Endpoint, MAX_WIRE_BYTES, OFFICIAL_ORIGIN, Token};
use central_crypto::opaque::server::ServerKeys;
use central_http::{BROWSER_REQUEST_BYTES, BrowserOrigin, DevelopmentServer};
use central_identity::IdentityService;
use central_policy::config::Config;
use central_postgres::{
    AccountStore, Appointed, LegacyStore, ServerConfig, appoint_admin, connect_server, sweep,
    sweep_legacy,
};
use serde::Deserialize;
use std::{
    io::{IsTerminal, Read, Write},
    process::ExitCode,
    sync::Arc,
};
use tokio::sync::watch;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

/// Upper bound on one configuration document; the CA bundle is most of it.
const MAX_SETTINGS: usize = 131_072;

/// The browser listener's connections at once: the policy's most. Each holds
/// at most a 64 KiB body, one address at most four of them
/// (`central_http`'s places in flight), so a stalled body takes no more than
/// its own address's share.
const BROWSER_CONNECTIONS: &str = "128";

/// The account listener's connections at once: the policy's most too. A
/// quarter of them (32) are the places of bodies that are late or larger
/// than 64 KiB, four an address (`central_http`'s boundary), so bodies that
/// are withheld leave 96 connections to requests whose small body is there,
/// and no more bodies of up to 1 MiB are in memory at once than the 32 the
/// listener's connections allowed before.
const ACCOUNT_CONNECTIONS: &str = "128";

/// One deployed database login. The address must be a loopback or private IP
/// literal and the certificate authority a PEM bundle (see [`ServerConfig`]).
#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
pub(super) struct Database {
    host: String,
    port: u16,
    name: String,
    user: String,
    password: String,
    ca_certificate: String,
}

impl Database {
    pub(super) fn server(&mut self) -> Result<ServerConfig, ()> {
        ServerConfig::new(
            &self.host,
            self.port,
            &self.user,
            std::mem::take(&mut self.password),
            &self.name,
            std::mem::take(&mut self.ca_certificate).into_bytes(),
        )
        .map_err(|_| ())
    }
}

/// The live account service. There is no endpoint field: production is the
/// compiled live endpoint, and its official gateway (`OFFICIAL_ORIGIN` and
/// its aliases) is compiled in too.
#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
struct Service {
    listen: String,
    /// The website's browser listener; absent means off.
    #[serde(default)]
    browser: Option<Browser>,
    database: Database,
    opaque_setup: Vec<u8>,
    bridge: Token,
    /// The legacy store's own login, from the cutover until its deadline
    /// (`docs/operations.md` §"The legacy store"); absent means no legacy
    /// sign-in.
    #[serde(default)]
    legacy: Option<Legacy>,
    #[serde(default)]
    push: Option<Push>,
}

/// Server-only persistent VAPID key, read with the other private configuration.
#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
struct Push {
    vapid_key: Vec<u8>,
}

/// The legacy store's login: the `central_legacy` role, never the runtime's.
#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
struct Legacy {
    database: Database,
}

/// The browser listener for `baylee.app`, a numeric loopback address behind
/// the site's proxy (`docs/web-accounts.md`).
#[derive(Deserialize, Zeroize)]
#[serde(deny_unknown_fields)]
struct Browser {
    listen: String,
}

/// An explicit migration run under the separate DDL login.
#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
struct Migration {
    database: Database,
}

/// An administrator to appoint under the migrator login
/// (`docs/administration.md`): an enrolled account's username, which arrives
/// at run time so the repository names nobody.
#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
struct Appoint {
    database: Database,
    username: String,
}

pub(super) fn read_settings() -> Result<Zeroizing<Vec<u8>>, ()> {
    let mut bytes = Zeroizing::new(Vec::new());
    std::io::stdin()
        .lock()
        .take(MAX_SETTINGS as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ())?;
    if bytes.len() > MAX_SETTINGS {
        return Err(());
    }
    Ok(bytes)
}

fn parse<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T, ()> {
    serde_json::from_slice(bytes).map_err(|_| ())
}

/// Serve the live account protocol until SIGTERM/SIGINT.
pub(super) async fn run_service() -> ExitCode {
    if serve().await.is_ok() {
        ExitCode::SUCCESS
    } else {
        eprintln!("central: account service rejected configuration or failed");
        ExitCode::FAILURE
    }
}

fn listener_configs(settings: &Service) -> Result<(Config, Option<Config>), ()> {
    let request_bytes = MAX_WIRE_BYTES.to_string();
    let config = Config::from_pairs([
        ("BAYLEE_CENTRAL_LISTEN", settings.listen.as_str()),
        ("BAYLEE_CENTRAL_REQUEST_BYTES", request_bytes.as_str()),
        ("BAYLEE_CENTRAL_CONCURRENT_REQUESTS", ACCOUNT_CONNECTIONS),
    ])
    .map_err(|_| ())?;
    let browser_bytes = BROWSER_REQUEST_BYTES.to_string();
    let browser = settings
        .browser
        .as_ref()
        .map(|browser| {
            Config::from_pairs([
                ("BAYLEE_CENTRAL_LISTEN", browser.listen.as_str()),
                ("BAYLEE_CENTRAL_REQUEST_BYTES", browser_bytes.as_str()),
                ("BAYLEE_CENTRAL_CONCURRENT_REQUESTS", BROWSER_CONNECTIONS),
            ])
        })
        .transpose()
        .map_err(|_| ())?;
    Ok((config, browser))
}

async fn serve() -> Result<(), ()> {
    let bytes = read_settings()?;
    let mut settings: Service = parse(&bytes)?;
    drop(bytes);
    let push = settings
        .push
        .as_ref()
        .map(|push| {
            central_push::Sender::new(&push.vapid_key, "https://baylee.app/contact".into())
                .map(Arc::new)
        })
        .transpose()
        .map_err(|_| ())?;
    let endpoint = Endpoint::live();
    let (config, browser) = listener_configs(&settings)?;
    let keys = ServerKeys::from_secret_bytes(&settings.opaque_setup).map_err(|_| ())?;
    let database = settings.database.server()?;
    let legacy = settings
        .legacy
        .as_mut()
        .map(|legacy| legacy.database.server())
        .transpose()?;
    // Bind before connecting: a taken port fails fast without touching the database.
    let server = DevelopmentServer::bind(config).await.map_err(|_| ())?;
    let browser = match browser {
        Some(config) => Some(DevelopmentServer::bind(config).await.map_err(|_| ())?),
        None => None,
    };
    let store = AccountStore::connect(database).await.map_err(|_| ())?;
    let legacy = match legacy {
        Some(config) => {
            if let Ok(legacy) = LegacyStore::connect(config).await {
                Some(legacy)
            } else {
                let _ = store.close().await;
                return Err(());
            }
        }
        None => None,
    };
    let identity = IdentityService::new(
        store.clone(),
        &endpoint,
        keys,
        &settings.bridge,
        OFFICIAL_ORIGIN,
    )
    .map(|identity| match &legacy {
        Some(legacy) => identity.with_legacy(legacy.clone()),
        None => identity,
    });
    let identity = identity.map(|identity| match &push {
        Some(push) => identity.with_push_key(push.public_key().into()),
        None => identity,
    });
    drop(settings);
    let result = async {
        let identity = Arc::new(identity.map_err(|_| ())?);
        println!(
            "central: account listener at http://{} for {}",
            server.local_addr(),
            endpoint.origin()
        );
        if let Some(browser) = &browser {
            println!(
                "central: browser listener at http://{} for https://baylee.app",
                browser.local_addr()
            );
        }
        if legacy.is_some() {
            println!("central: legacy sign-in from the legacy store");
        }
        let (stop, _) = watch::channel(false);
        let native = server.run_with_identity(&endpoint, identity.clone(), stopped(&stop));
        let browser = browser
            .map(|browser| browser.run_browser(identity, BrowserOrigin::live(), stopped(&stop)));
        if push.is_some() {
            store
                .announce_release(env!("CARGO_PKG_VERSION"))
                .await
                .map_err(|_| ())?;
        }
        let (result, ()) = tokio::join!(
            together(&stop, native, browser, shutdown()),
            super::push::run(store.clone(), push, stop.subscribe())
        );
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

/// Apply pending migrations once, under the migration login, then exit.
pub(super) async fn run_migration() -> ExitCode {
    if migrate().await.is_ok() {
        println!("central: migrations applied");
        ExitCode::SUCCESS
    } else {
        eprintln!("central: migration rejected configuration or failed");
        ExitCode::FAILURE
    }
}

async fn migrate() -> Result<(), ()> {
    let bytes = read_settings()?;
    let mut settings: Migration = parse(&bytes)?;
    drop(bytes);
    let database = settings.database.server()?;
    drop(settings);
    let db = connect_server(database).await.map_err(|_| ())?;
    let applied = central_migration::apply(&db).await.map_err(|_| ());
    let closed = db.close().await.map_err(|_| ());
    applied.and(closed)
}

/// Appoint an enrolled account as an administrator: the first one, and the
/// way back in when none can sign in. Again is no error.
pub(super) async fn run_appoint_admin() -> ExitCode {
    match appoint().await {
        Ok(Appointed::Appointed) => {
            eprintln!("central: administrator appointed");
            ExitCode::SUCCESS
        }
        Ok(Appointed::Already) => {
            eprintln!("central: already an administrator; nothing changed");
            ExitCode::SUCCESS
        }
        Err(()) => {
            eprintln!(
                "central: appointment rejected configuration, found no enrolled account of that name, or failed"
            );
            ExitCode::FAILURE
        }
    }
}

async fn appoint() -> Result<Appointed, ()> {
    let bytes = read_settings()?;
    let mut settings: Appoint = parse(&bytes)?;
    drop(bytes);
    let database = settings.database.server()?;
    let db = connect_server(database).await.map_err(|_| ())?;
    let appointed = appoint_admin(&db, &settings.username).await.map_err(|_| ());
    let closed = db.close().await.map_err(|_| ());
    let appointed = appointed?;
    closed?;
    Ok(appointed)
}

/// Delete audit events older than three years, expired reset codes, expired
/// pairings, reports past their retention and what the legacy store no
/// longer needs under the migrator
/// login, which a daily timer runs (`docs/operations.md`).
pub(super) async fn run_sweep() -> ExitCode {
    if let Ok((swept, legacy)) = swept().await {
        eprintln!(
            "central: {} audit events older than three years, {} expired reset codes, {} expired pairings and {} reports past their retention deleted",
            swept.events, swept.codes, swept.pairings, swept.reports
        );
        eprintln!(
            "central: {} moved and {} expired legacy accounts and {} unused legacy reservations deleted",
            legacy.moved, legacy.expired, legacy.reservations
        );
        ExitCode::SUCCESS
    } else {
        eprintln!("central: sweep rejected configuration or failed");
        ExitCode::FAILURE
    }
}

async fn swept() -> Result<(central_postgres::Swept, central_postgres::LegacySwept), ()> {
    let bytes = read_settings()?;
    let mut settings: Migration = parse(&bytes)?;
    drop(bytes);
    let database = settings.database.server()?;
    drop(settings);
    let db = connect_server(database).await.map_err(|_| ())?;
    let swept = sweep(&db).await.map_err(|_| ());
    let legacy = match swept {
        Ok(_) => sweep_legacy(&db).await.map_err(|_| ()),
        Err(()) => Err(()),
    };
    let closed = db.close().await.map_err(|_| ());
    let swept = swept?;
    let legacy = legacy?;
    closed?;
    Ok((swept, legacy))
}

/// Write a fresh OPAQUE setup and bridge credential as one JSON document.
/// Refuses a terminal: the secrets belong in a file the operator redirects to.
pub(super) fn generate_secrets() -> ExitCode {
    let mut stdout = std::io::stdout().lock();
    if stdout.is_terminal() {
        eprintln!("central: refusing to print secrets to a terminal; redirect to a 0600 file");
        return ExitCode::FAILURE;
    }
    let Ok(document) = secrets_document() else {
        eprintln!("central: secret generation failed");
        return ExitCode::FAILURE;
    };
    if stdout
        .write_all(document.as_bytes())
        .and_then(|()| stdout.flush())
        .is_err()
    {
        eprintln!("central: secret output failed");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn secrets_document() -> Result<Zeroizing<String>, ()> {
    let setup = ServerKeys::generate().to_secret_bytes();
    let bridge = Token::generate().map_err(|_| ())?.to_secret_bytes();
    let mut text = Zeroizing::new(String::with_capacity(1024));
    text.push_str("{\"opaque_setup\":");
    push_bytes(&mut text, &setup);
    text.push_str(",\"bridge\":");
    push_bytes(&mut text, bridge.as_slice());
    text.push_str(",\"push\":{\"vapid_key\":");
    push_bytes(
        &mut text,
        &central_push::Sender::generate_key().map_err(|_| ())?,
    );
    text.push_str("}}\n");
    Ok(text)
}

/// A JSON byte array, written without intermediate unzeroized buffers.
fn push_bytes(text: &mut String, bytes: &[u8]) {
    use std::fmt::Write as _;
    text.push('[');
    for (index, byte) in bytes.iter().enumerate() {
        if index > 0 {
            text.push(',');
        }
        let _ = write!(text, "{byte}");
    }
    text.push(']');
}

/// Resolves on SIGTERM (a supervisor's stop) or Ctrl-C. Signal registration
/// failure resolves at once, closing the listener rather than hanging.
async fn shutdown() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let Ok(mut terminate) = signal(SignalKind::terminate()) else {
            return;
        };
        tokio::select! {
            _ = terminate.recv() => {}
            _ = tokio::signal::ctrl_c() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CA: &str = "-----BEGIN CERTIFICATE-----\\nMIIB\\n-----END CERTIFICATE-----\\n";

    fn database() -> String {
        format!(
            r#"{{"host":"127.0.0.1","port":5432,"name":"baylee_central","user":"central_runtime","password":"PRIVATE_DB_CANARY","ca_certificate":"{CA}"}}"#
        )
    }

    fn service(extra: &str) -> String {
        let setup = ServerKeys::generate().to_secret_bytes();
        let setup = serde_json::to_string(setup.as_slice()).unwrap();
        let bridge = serde_json::to_string(&[7_u8; 32]).unwrap();
        format!(
            r#"{{"listen":"127.0.0.1:28790","database":{},"opaque_setup":{setup},"bridge":{bridge}{extra}}}"#,
            database()
        )
    }

    #[test]
    fn a_service_document_parses_and_yields_a_verified_tls_database() {
        let mut settings: Service = parse(service("").as_bytes()).unwrap();
        assert_eq!(settings.listen, "127.0.0.1:28790");
        assert!(ServerKeys::from_secret_bytes(&settings.opaque_setup).is_ok());
        assert!(settings.database.server().is_ok());
        // The secrets are moved out, not copied, into the connector.
        assert_eq!(settings.database.password, "");
        assert_eq!(settings.database.ca_certificate, "");
    }

    #[test]
    fn development_shaped_or_extended_documents_are_rejected() {
        // The development launcher's fields name a selectable endpoint and a
        // loopback-only database; neither exists in production.
        for extra in [
            r#","endpoint":"http://127.0.0.1:9000""#,
            r#","gateway":"https://gateway.baylee.app""#,
            r#","gateway_aliases":["https://gateway.baylee.app"]"#,
            r#","database_port":5432"#,
            r#","sslmode":"disable""#,
        ] {
            assert!(
                parse::<Service>(service(extra).as_bytes()).is_err(),
                "{extra}"
            );
        }
        let missing = service("").replace(r#""listen":"127.0.0.1:28790","#, "");
        assert!(parse::<Service>(missing.as_bytes()).is_err());
        let plaintext = database().replace(&format!(r#","ca_certificate":"{CA}""#), "");
        assert_ne!(plaintext, database());
        assert!(parse::<Database>(plaintext.as_bytes()).is_err());
    }

    #[test]
    fn a_migration_document_takes_only_its_database() {
        let document = format!(r#"{{"database":{}}}"#, database());
        let mut settings: Migration = parse(document.as_bytes()).unwrap();
        assert!(settings.database.server().is_ok());
        let extended = format!(r#"{{"database":{},"opaque_setup":[1]}}"#, database());
        assert!(parse::<Migration>(extended.as_bytes()).is_err());
    }

    #[test]
    fn the_browser_listener_is_optional_and_loopback_only() {
        let settings: Service = parse(service("").as_bytes()).unwrap();
        assert!(settings.browser.is_none(), "absent means off");
        let settings: Service =
            parse(service(r#","browser":{"listen":"127.0.0.1:28791"}"#).as_bytes()).unwrap();
        let listen = settings.browser.as_ref().unwrap().listen.clone();
        let config = |listen: &str| {
            Config::from_pairs([
                ("BAYLEE_CENTRAL_LISTEN", listen),
                ("BAYLEE_CENTRAL_REQUEST_BYTES", "65536"),
            ])
        };
        assert_eq!(config(&listen).unwrap().listen().port(), 28_791);
        for listen in ["0.0.0.0:28791", "192.168.1.2:28791", "baylee.app:443"] {
            assert!(config(listen).is_err(), "{listen}");
        }
        // The deployed listeners both take the policy's most connections.
        let (native, browser) = listener_configs(&settings).unwrap();
        let browser = browser.unwrap();
        assert_eq!(browser.concurrent_requests().get(), 128);
        assert_eq!(browser.request_bytes().get(), BROWSER_REQUEST_BYTES);
        assert_eq!(native.concurrent_requests().get(), 128);
        assert_eq!(native.request_bytes().get(), MAX_WIRE_BYTES);
        for extra in [
            r#","browser":{"listen":"127.0.0.1:28791","origin":"https://evil.example"}"#,
            r#","browser":{}"#,
            r#","browser":"127.0.0.1:28791""#,
        ] {
            assert!(
                parse::<Service>(service(extra).as_bytes()).is_err(),
                "{extra}"
            );
        }
    }

    #[test]
    fn an_appointment_takes_a_username_and_its_database_only() {
        let appoint =
            |extra: &str| format!(r#"{{"database":{},"username":"Owner"{extra}}}"#, database());
        let mut settings: Appoint = parse(appoint("").as_bytes()).unwrap();
        assert_eq!(settings.username, "Owner");
        assert!(settings.database.server().is_ok());
        for extra in [
            r#","admin":true"#,
            r#","role":"admin""#,
            r#","account":[1]"#,
        ] {
            assert!(
                parse::<Appoint>(appoint(extra).as_bytes()).is_err(),
                "{extra}"
            );
        }
        let without = appoint("").replace(r#","username":"Owner""#, "");
        assert!(parse::<Appoint>(without.as_bytes()).is_err());
        // The sweep takes the migration document, nothing else.
        let sweep = format!(r#"{{"database":{}}}"#, database());
        assert!(parse::<Migration>(sweep.as_bytes()).is_ok());
        let longer = format!(r#"{{"database":{},"years":1}}"#, database());
        assert!(parse::<Migration>(longer.as_bytes()).is_err());
    }

    #[test]
    fn database_values_are_validated_by_the_connector() {
        for (from, to) in [
            ("127.0.0.1", "db.example.org"),
            ("127.0.0.1", "8.8.8.8"),
            ("central_runtime", "central runtime"),
            ("-----BEGIN CERTIFICATE-----", "-----BEGIN PRIVATE KEY-----"),
        ] {
            let mut settings: Database = parse(database().replace(from, to).as_bytes()).unwrap();
            assert!(settings.server().is_err(), "{to}");
        }
    }

    #[test]
    fn generated_secrets_restore_and_differ_each_time() {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Secrets {
            opaque_setup: Vec<u8>,
            bridge: Token,
            push: Push,
        }
        let first: Secrets = parse(secrets_document().unwrap().as_bytes()).unwrap();
        let second: Secrets = parse(secrets_document().unwrap().as_bytes()).unwrap();
        assert!(ServerKeys::from_secret_bytes(&first.opaque_setup).is_ok());
        assert_ne!(first.opaque_setup, second.opaque_setup);
        assert_ne!(first.bridge.digest(), second.bridge.digest());
        assert_ne!(first.push.vapid_key, second.push.vapid_key);
        assert!(
            central_push::Sender::new(&first.push.vapid_key, "https://baylee.app/contact".into())
                .is_ok()
        );
    }

    #[test]
    fn the_settings_bound_admits_a_certificate_bundle_and_nothing_unbounded() {
        let bundle = "A".repeat(60_000);
        let document = service("").replace("MIIB", &bundle);
        assert!(document.len() < MAX_SETTINGS);
        assert!(parse::<Service>(document.as_bytes()).is_ok());
    }
}
