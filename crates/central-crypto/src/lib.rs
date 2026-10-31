//! Experimental cryptographic integration for native clients.
//!
//! No HTTP, persistence, sessions or password-reset policy is provided; the
//! one server-side password check, the legacy store's (`legacy`, behind
//! `server`), derives and leaves the comparison and the policy to its caller.
//! TLS, trusted issuer configuration and account ownership remain mandatory.
//! Server key material is behind the opt-in `server` feature. Independent protocol
//! review and published-vector validation are release gates, not completed claims.
#![forbid(unsafe_code)]

mod context;
#[cfg(feature = "server")]
pub mod legacy;
pub mod opaque;
pub mod pairing;
pub mod vault;

pub use context::Context;

use std::fmt;

/// Deliberately generic diagnostics: no password, proof or key content.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CryptoError {
    /// Input is malformed, unsupported or not authenticated.
    Invalid,
    /// Operating-system randomness was unavailable.
    RandomnessUnavailable,
}

impl fmt::Display for CryptoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Invalid => "cryptographic operation rejected",
            Self::RandomnessUnavailable => "secure randomness unavailable",
        })
    }
}

impl std::error::Error for CryptoError {}
