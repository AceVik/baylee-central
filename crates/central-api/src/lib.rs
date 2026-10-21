//! Native account protocol. Passwords and decrypted personal content have no server wire type.
#![forbid(unsafe_code)]

pub mod beta_key;
mod endpoint;
pub mod gateway;
pub mod object;
pub mod terms;
mod token;
pub mod wire;
pub use endpoint::{Endpoint, LEGACY_ORIGIN, LIVE_ORIGIN};
pub use token::Token;

/// Maximum serialized native request or response, independently enforced by both transports.
// A 256 KiB ciphertext may require four JSON bytes per byte, plus its AEAD
// overhead, OPAQUE enrollment record, wrapper, capability and field names.
pub const MAX_WIRE_BYTES: usize = 1_056_768;

/// Generic error without remote diagnostics, secrets or supplied input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    /// Malformed input, unexpected response or failed authentication.
    Rejected,
    /// The transport or database is unavailable; never authorizes a legacy fallback.
    Unavailable,
    /// A concurrent change requires a fresh read; never overwrite the newer version.
    Conflict,
    /// Bounded resource or rate capacity was reached.
    Busy,
    /// The account has not accepted the terms in force; only reading and
    /// accepting them, exporting and signing out remain (`docs/terms.md`).
    TermsRequired,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Rejected => "account operation rejected",
            Self::Unavailable => "account service unavailable",
            Self::Conflict => "account data changed; reload required",
            Self::Busy => "account service busy; retry later",
            Self::TermsRequired => "terms of use must be accepted first",
        })
    }
}
impl std::error::Error for Error {}

/// Parse a bounded response; unknown fields and enum variants fail closed at each wire type.
///
/// # Errors
/// Rejects oversized, malformed or trailing JSON.
pub fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, Error> {
    if bytes.len() > MAX_WIRE_BYTES {
        return Err(Error::Rejected);
    }
    serde_json::from_slice(bytes).map_err(|_| Error::Rejected)
}

/// Encode a bounded request or response.
///
/// # Errors
/// Rejects oversized or unserializable values.
pub fn encode<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, Error> {
    let bytes = serde_json::to_vec(value).map_err(|_| Error::Rejected)?;
    if bytes.len() > MAX_WIRE_BYTES {
        return Err(Error::Rejected);
    }
    Ok(bytes)
}
