use crate::StoreError;
use sea_orm::sqlx::postgres::PgSslMode;
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement,
};
use secrecy::{ExposeSecret, SecretString};
use std::{net::IpAddr, time::Duration};
use url::Url;

/// Explicit local-only connection settings. No Debug or Clone of credentials.
/// This type cannot connect to a remote host or consume ambient `DATABASE_URL`.
pub struct DevConfig {
    port: u16,
    user: String,
    password: SecretString,
    database: String,
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 63
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

impl DevConfig {
    /// Configure an IPv4 loopback fixture with no TLS; never for real user data.
    ///
    /// # Errors
    ///
    /// Rejects invalid ports, identifiers or empty/oversized credentials.
    pub fn new(
        port: u16,
        user: &str,
        password: String,
        database: &str,
    ) -> Result<Self, StoreError> {
        let password = SecretString::from(password);
        if port == 0
            || !identifier(user)
            || !identifier(database)
            || password.expose_secret().is_empty()
            || password.expose_secret().len() > 1024
        {
            return Err(StoreError::InvalidConfig);
        }
        Ok(Self {
            port,
            user: user.into(),
            password,
            database: database.into(),
        })
    }

    fn url(&self) -> Result<Url, StoreError> {
        let mut url =
            Url::parse("postgresql://127.0.0.1").map_err(|_| StoreError::InvalidConfig)?;
        url.set_port(Some(self.port))
            .map_err(|()| StoreError::InvalidConfig)?;
        url.set_username(&self.user)
            .map_err(|()| StoreError::InvalidConfig)?;
        url.set_password(Some(self.password.expose_secret()))
            .map_err(|()| StoreError::InvalidConfig)?;
        url.set_path(&self.database);
        url.query_pairs_mut().append_pair("sslmode", "disable");
        Ok(url)
    }
}

/// Upper bound on the configured certificate authority bundle.
const MAX_CA_PEM: usize = 65_536;

/// Verified-TLS settings for a deployed PostgreSQL server. No Debug or Clone of
/// credentials, and no ambient `DATABASE_URL`.
///
/// The address is a loopback or private IP literal. The TLS stack trusts the
/// configured authority alongside the public web roots, and public authorities do
/// not certify loopback or private addresses, so only the configured authority
/// can vouch for such a server. A DNS name would let any public authority do so.
pub struct ServerConfig {
    host: IpAddr,
    port: u16,
    user: String,
    password: SecretString,
    database: String,
    ca_pem: Vec<u8>,
}

fn private_address(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => ip.is_loopback() || ip.is_private(),
        IpAddr::V6(ip) => ip.is_loopback() || (ip.segments()[0] & 0xfe00) == 0xfc00,
    }
}

fn certificate_bundle(pem: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(pem) else {
        return false;
    };
    !pem.is_empty()
        && pem.len() <= MAX_CA_PEM
        && text.contains("-----BEGIN CERTIFICATE-----")
        && text.contains("-----END CERTIFICATE-----")
        // A pasted private key is a provisioning mistake that must not travel further.
        && !text.contains("PRIVATE KEY")
}

impl ServerConfig {
    /// Configure a deployed server reached over verified TLS.
    ///
    /// # Errors
    ///
    /// Rejects a non-IP, public or unspecified address, a zero port, invalid
    /// identifiers, empty/oversized credentials and a missing or oversized
    /// PEM certificate bundle (or one that carries a private key).
    pub fn new(
        host: &str,
        port: u16,
        user: &str,
        password: String,
        database: &str,
        ca_pem: Vec<u8>,
    ) -> Result<Self, StoreError> {
        let password = SecretString::from(password);
        let host: IpAddr = host.parse().map_err(|_| StoreError::InvalidConfig)?;
        if !private_address(host)
            || port == 0
            || !identifier(user)
            || !identifier(database)
            || password.expose_secret().is_empty()
            || password.expose_secret().len() > 1024
            || !certificate_bundle(&ca_pem)
        {
            return Err(StoreError::InvalidConfig);
        }
        Ok(Self {
            host,
            port,
            user: user.into(),
            password,
            database: database.into(),
            ca_pem,
        })
    }

    /// The connection URL without any credential: the password and certificate
    /// authority reach the driver through its typed options only.
    fn url(&self) -> Result<Url, StoreError> {
        let mut url =
            Url::parse("postgresql://127.0.0.1").map_err(|_| StoreError::InvalidConfig)?;
        url.set_ip_host(self.host)
            .map_err(|()| StoreError::InvalidConfig)?;
        url.set_port(Some(self.port))
            .map_err(|()| StoreError::InvalidConfig)?;
        url.set_username(&self.user)
            .map_err(|()| StoreError::InvalidConfig)?;
        url.set_path(&self.database);
        url.query_pairs_mut().append_pair("sslmode", "verify-full");
        Ok(url)
    }
}

/// PostgreSQL's `server_version_num` compatibility gate; no extension fallback.
#[must_use]
pub const fn supports_version(version: i32) -> bool {
    version >= 180_000
}

/// Open a bounded local fixture pool for explicit migrations or server-side adapters.
/// SQL logging is disabled. Caller must close the pool. This low-level function
/// does not grant authority; normal runtime use goes through `PendingAccountStore`.
///
/// # Errors
///
/// Rejects unsupported PostgreSQL versions, invalid configuration and failed I/O.
pub async fn connect_dev(config: DevConfig) -> Result<DatabaseConnection, StoreError> {
    let options = pool_options(&config.url()?);
    open(options, false).await
}

/// Open a bounded pool to a deployed server over verified TLS, for explicit
/// migrations or server-side adapters. The server certificate must chain to the
/// configured authority and name the configured address; a server that declines
/// TLS fails the connection instead of falling back to plaintext. SQL logging is
/// disabled and the password never enters the connection URL. This low-level
/// function does not grant authority; runtime use goes through the stores, whose
/// role checks run on this connection exactly as on the development one.
///
/// # Errors
///
/// Rejects unsupported PostgreSQL versions, an unverified or plaintext session,
/// invalid configuration and failed I/O.
pub async fn connect_server(config: ServerConfig) -> Result<DatabaseConnection, StoreError> {
    let mut options = pool_options(&config.url()?);
    let ServerConfig {
        password, ca_pem, ..
    } = config;
    options.map_sqlx_postgres_opts(move |options| {
        options
            .ssl_mode(PgSslMode::VerifyFull)
            .ssl_root_cert_from_pem(ca_pem.clone())
            .password(password.expose_secret())
            .application_name("baylee-central")
    });
    open(options, true).await
}

fn pool_options(url: &Url) -> ConnectOptions {
    let mut options = ConnectOptions::new(url.to_string());
    options
        .max_connections(4)
        .min_connections(0)
        .connect_timeout(Duration::from_secs(5))
        .acquire_timeout(Duration::from_secs(3))
        .idle_timeout(Duration::from_secs(30))
        .max_lifetime(Duration::from_secs(300))
        .statement_timeout(Duration::from_secs(5))
        .set_schema_search_path("public")
        .sqlx_logging(false);
    options
}

async fn open(options: ConnectOptions, tls: bool) -> Result<DatabaseConnection, StoreError> {
    let db = Database::connect(options)
        .await
        .map_err(|_| StoreError::Unavailable)?;
    let checked = async {
        verify_version(&db).await?;
        if tls {
            verify_tls(&db).await?;
        }
        Ok(())
    }
    .await;
    if let Err(error) = checked {
        let _ = db.close().await;
        return Err(error);
    }
    Ok(db)
}

/// Defense in depth: the session the pool handed out must itself be encrypted.
async fn verify_tls(db: &DatabaseConnection) -> Result<(), StoreError> {
    let encrypted = db
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT ssl FROM pg_catalog.pg_stat_ssl WHERE pid = pg_catalog.pg_backend_pid()",
        ))
        .await
        .map_err(|_| StoreError::Unavailable)?
        .ok_or(StoreError::Unavailable)?
        .try_get::<bool>("", "ssl")
        .map_err(|_| StoreError::Unavailable)?;
    if encrypted {
        Ok(())
    } else {
        Err(StoreError::Unavailable)
    }
}

async fn verify_version(db: &DatabaseConnection) -> Result<(), StoreError> {
    let version = db
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT current_setting('server_version_num')::integer AS version",
        ))
        .await
        .map_err(|_| StoreError::Unavailable)?
        .ok_or(StoreError::Unavailable)?
        .try_get::<i32>("", "version")
        .map_err(|_| StoreError::Unavailable)?;
    if !supports_version(version) {
        return Err(StoreError::UnsupportedVersion);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configuration_cannot_change_host_or_inject_options() {
        for user in ["", "a".repeat(64).as_str(), "x?sslmode=disable", "u@remote"] {
            assert!(DevConfig::new(5432, user, "synthetic".into(), "central").is_err());
        }
        assert!(DevConfig::new(0, "u", "synthetic".into(), "db").is_err());
        let config = DevConfig::new(5432, "u", "synthetic@host/?#".into(), "db").unwrap();
        let url = config.url().unwrap();
        assert_eq!(url.host_str(), Some("127.0.0.1"));
        assert_eq!(url.port(), Some(5432));
        assert_eq!(url.path(), "/db");
        assert_eq!(url.query(), Some("sslmode=disable"));
        assert!(!format!("{}", StoreError::Unavailable).contains("synthetic"));
    }

    const CA: &str = "-----BEGIN CERTIFICATE-----\nMIIB\n-----END CERTIFICATE-----\n";

    fn server(
        host: &str,
        user: &str,
        database: &str,
        ca: &str,
    ) -> Result<ServerConfig, StoreError> {
        ServerConfig::new(
            host,
            5432,
            user,
            "synthetic@host/?#".into(),
            database,
            ca.into(),
        )
    }

    #[test]
    fn server_url_carries_no_credential_and_demands_verified_tls() {
        for host in [
            "127.0.0.1",
            "10.1.2.3",
            "172.16.0.9",
            "192.168.4.5",
            "::1",
            "fd00::5",
        ] {
            let url = server(host, "central_runtime", "baylee_central", CA)
                .unwrap()
                .url()
                .unwrap();
            assert_eq!(url.password(), None, "{host}");
            assert!(!url.as_str().contains("synthetic"), "{host}");
            assert!(!url.as_str().contains("CERTIFICATE"), "{host}");
            assert_eq!(url.query(), Some("sslmode=verify-full"), "{host}");
            assert_eq!(url.username(), "central_runtime");
            assert_eq!(url.path(), "/baylee_central");
            assert_eq!(url.port(), Some(5432));
        }
        let url = server("::1", "u", "db", CA).unwrap().url().unwrap();
        assert_eq!(url.host_str(), Some("[::1]"));
    }

    #[test]
    fn server_configuration_rejects_names_public_addresses_and_injection() {
        for host in [
            "localhost",
            "db.example.org",
            "8.8.8.8",
            "2001:4860::8888",
            "0.0.0.0",
            "::",
            "127.0.0.1:5432",
            "127.0.0.1/db",
            "",
        ] {
            assert!(server(host, "u", "db", CA).is_err(), "{host}");
        }
        for user in ["", "a".repeat(64).as_str(), "x?sslmode=disable", "u@remote"] {
            assert!(server("127.0.0.1", user, "db", CA).is_err(), "{user}");
        }
        assert!(server("127.0.0.1", "u", "db?sslmode=disable", CA).is_err());
        assert!(
            ServerConfig::new("127.0.0.1", 0, "u", "synthetic".into(), "db", CA.into()).is_err()
        );
        assert!(ServerConfig::new("127.0.0.1", 5432, "u", String::new(), "db", CA.into()).is_err());
        assert!(
            ServerConfig::new("127.0.0.1", 5432, "u", "p".repeat(1025), "db", CA.into()).is_err()
        );
    }

    #[test]
    fn server_configuration_requires_a_bounded_certificate_bundle_without_keys() {
        let key = format!("{CA}-----BEGIN PRIVATE KEY-----\nMIIE\n-----END PRIVATE KEY-----\n");
        let oversized = format!("{CA}{}", "A".repeat(MAX_CA_PEM));
        for ca in [
            "",
            "MIIB",
            "-----BEGIN CERTIFICATE-----",
            key.as_str(),
            oversized.as_str(),
        ] {
            assert!(server("127.0.0.1", "u", "db", ca).is_err());
        }
        assert!(
            ServerConfig::new("127.0.0.1", 5432, "u", "p".into(), "db", vec![0xff, 0xfe]).is_err()
        );
        assert!(!format!("{}", StoreError::InvalidConfig).contains("synthetic"));
    }

    #[test]
    fn minimum_server_version_is_enforced() {
        for version in [0, 170_000, 179_999] {
            assert!(!supports_version(version));
        }
        assert!(supports_version(180_000));
        assert!(supports_version(180_005));
        assert!(supports_version(190_000));
    }
}
