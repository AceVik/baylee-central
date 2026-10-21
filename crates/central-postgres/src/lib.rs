//! PostgreSQL 18+ adapter. Development transport is explicitly loopback-only;
//! deployed servers are reached only over verified TLS.
//! Client/policy/crypto libraries do not depend on this server-side crate.
#![forbid(unsafe_code)]

mod accounts;
mod beta_keys;
mod connection;
mod pending;
pub use accounts::{AccountStore, Credential, Issued, MAX_GATEWAY_VERSIONS, Session};

pub use beta_keys::{
    BetaKeyBatch, MAX_BETA_KEY_DAYS, MAX_BETA_KEY_NOTE, MAX_BETA_KEY_USES, MAX_BETA_KEYS,
};
pub use connection::{DevConfig, ServerConfig, connect_dev, connect_server, supports_version};
pub use pending::{PendingAccount, PendingAccountStore};

/// Generic diagnostics: database errors and connection strings never cross this boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StoreError {
    /// Invalid development or server configuration.
    InvalidConfig,
    /// Connection/query/constraint failure, without values or database diagnostics.
    Unavailable,
    /// PostgreSQL older than version 18.
    UnsupportedVersion,
    /// Runtime credentials have excess privileges or the schema is unavailable.
    UnsafeRole,
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidConfig => "invalid database configuration",
            Self::Unavailable => "database operation unavailable",
            Self::UnsupportedVersion => "PostgreSQL 18 or newer is required",
            Self::UnsafeRole => "database runtime role rejected",
        })
    }
}
impl std::error::Error for StoreError {}
