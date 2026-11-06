//! Native account protocol. Decrypted personal content has no server wire type,
//! and neither have passwords but one: [`legacy::LegacyPassword`], the legacy
//! store's sign-in, which ends with the store's deadline.
#![forbid(unsafe_code)]

pub mod admin;
pub mod beta_key;
pub mod chat;
mod endpoint;
pub mod export;
pub mod gateway;
pub mod groups;
pub mod legacy;
pub mod notifications;
pub mod object;
pub mod owner;
pub mod profile;
pub mod reports;
pub mod reset_code;
pub mod social;
pub mod terms;
mod token;
pub mod wire;
pub use endpoint::{
    Endpoint, LEGACY_ORIGIN, LIVE_ORIGIN, OFFICIAL_ALIASES, OFFICIAL_ORIGIN, is_official_origin,
};
pub use token::{Bearer, SignedToken, Token};

/// The header every native request carries, naming the protocol it speaks.
pub const PROTOCOL_HEADER: &str = "Baylee-Central-Protocol";
/// The protocol this build speaks. Central answers any other with
/// [`UPDATE_REQUIRED_STATUS`] before it reads the request.
pub const PROTOCOL_VERSION: &str = "2";
/// The status of a client or gateway too old to be served: 426 Upgrade Required.
pub const UPDATE_REQUIRED_STATUS: u16 = 426;
/// That answer's body, in the refusal form clients read. It names no version: Central
/// does not know client releases, and the update itself needs no sign-in.
pub const UPDATE_REQUIRED_BODY: &str = r#"{"refusal":{"code":"update_required"}}"#;

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
    /// This build is too old to be served; the update needs no sign-in.
    UpdateRequired,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Rejected => "account operation rejected",
            Self::Unavailable => "account service unavailable",
            Self::Conflict => "account data changed; reload required",
            Self::Busy => "account service busy; retry later",
            Self::TermsRequired => "terms of use must be accepted first",
            Self::UpdateRequired => "client update required",
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
