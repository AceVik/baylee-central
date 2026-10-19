use crate::StoreError;
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement,
};
use secrecy::{ExposeSecret, SecretString};
use std::time::Duration;
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
    let mut options = ConnectOptions::new(config.url()?.to_string());
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
    let db = Database::connect(options)
        .await
        .map_err(|_| StoreError::Unavailable)?;
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
        let _ = db.close().await;
        return Err(StoreError::UnsupportedVersion);
    }
    Ok(db)
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
