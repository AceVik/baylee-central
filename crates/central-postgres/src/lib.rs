//! PostgreSQL 18+ adapter. Development transport is explicitly loopback-only.
//! No production TLS configuration or public account registration endpoint yet.
//! Client/policy/crypto libraries do not depend on this server-side crate.
#![forbid(unsafe_code)]

mod connection;
mod pending;

pub use connection::{DevConfig, connect_dev, supports_version};
pub use pending::{PendingAccount, PendingAccountStore};

/// Generic diagnostics: database errors and connection strings never cross this boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StoreError {
    /// Invalid development configuration.
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
