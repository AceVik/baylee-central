//! Compact JWS for Baylee Central's three tokens (RFC 7515, RFC 7519, RFC 8037):
//! the Central token, the gateway token and the revocation message. One
//! implementation for Central and for gateways, so that neither writes a second
//! verifier.
//!
//! The format is fixed, not negotiated. Only `EdDSA` over Ed25519 exists here: a
//! token's header is compared with the bytes this module would write, so no field
//! of it selects an algorithm, and a key comes from the caller's [`KeySet`], never
//! from the token. `iss`, `aud`, `exp` and `nbf` are required and always checked,
//! and a verifier is called for one [`Claims`] type and refuses the other kinds.
//! The caller supplies the time, so this module reads no clock.
//!
//! Signing needs the `server` feature as well as `jwt`.

mod claims;
mod keys;
mod replay;
#[cfg(test)]
mod tests;

pub use claims::{CentralClaims, Claims, GatewayClaims, Purpose, RevocationClaims, TermsClaim};
#[cfg(feature = "server")]
pub use keys::SigningKey;
pub use keys::{KeySet, MAX_JWKS_BYTES, MAX_KEYS, PublicKey};
pub use replay::ReplayCache;

use base64ct::{Base64UrlUnpadded, Encoding as _};
use ed25519_dalek::Signature;
use std::fmt;

/// Seconds of clock difference tolerated on `exp` and `nbf`.
pub const LEEWAY_SECONDS: i64 = 30;
/// A gateway token's longest life.
pub const GATEWAY_TOKEN_SECONDS: i64 = 3600;
/// A Central token's longest life: the session's cap.
pub const CENTRAL_TOKEN_SECONDS: i64 = 12 * 3600;
/// A revocation message's life.
pub const REVOCATION_SECONDS: i64 = 60;
/// Most session identifiers one revocation message names.
pub const MAX_SIDS: usize = 256;

const HEADER_START: &[u8] = br#"{"alg":"EdDSA","typ":""#;
const HEADER_KID: &[u8] = br#"","kid":""#;
const HEADER_END: &[u8] = br#""}"#;
const SIGNATURE_BYTES: usize = 64;
/// The length of a base64url SHA-256 thumbprint, which every `kid` is.
const KID_LEN: usize = 43;

/// The three tokens. Each has its own `typ`, so that one is never read as another.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    /// The client's session at Central; only Central ever reads it.
    Central,
    /// A client's admission at exactly one gateway.
    Gateway,
    /// Central's message to one gateway, naming sessions that ended there.
    Revocation,
}

impl Kind {
    /// The token's `typ` header.
    #[must_use]
    pub const fn typ(self) -> &'static str {
        match self {
            Self::Central => "baylee-central+jwt",
            Self::Gateway => "baylee-gateway+jwt",
            Self::Revocation => "baylee-revocation+jwt",
        }
    }

    /// The longest token of this kind, in bytes, refused before any decoding.
    #[must_use]
    pub const fn max_bytes(self) -> usize {
        match self {
            Self::Central | Self::Gateway => 2 * 1024,
            Self::Revocation => 16 * 1024,
        }
    }

    /// The longest `exp - iat` of this kind.
    #[must_use]
    pub const fn max_seconds(self) -> i64 {
        match self {
            Self::Central => CENTRAL_TOKEN_SECONDS,
            Self::Gateway => GATEWAY_TOKEN_SECONDS,
            Self::Revocation => REVOCATION_SECONDS,
        }
    }
}

/// Why a token was refused. The caller answers its peer with one fixed refusal; the
/// reasons exist for tests and for [`JwtError::UnknownKey`], which asks a gateway to
/// refresh its keys. None carries any part of a token.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JwtError {
    /// Too long, not three parts, not canonical base64url, or a malformed header.
    Malformed,
    /// The header names another algorithm.
    Algorithm,
    /// The header names another kind of token.
    Kind,
    /// No key of the set has this `kid`.
    UnknownKey,
    /// The signature does not verify.
    Signature,
    /// `iss` is not the expected issuer.
    Issuer,
    /// `aud` is not the expected audience.
    Audience,
    /// `exp` has passed, tolerance included.
    Expired,
    /// `nbf` has not come, tolerance included.
    NotYetValid,
    /// A claim is missing, unknown or out of its range.
    Claims,
    /// The `jti` was seen already.
    Replayed,
    /// The replay cache is full of live entries; try again later.
    Full,
    /// Operating-system randomness was unavailable.
    Randomness,
}

impl fmt::Display for JwtError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("token rejected")
    }
}

impl std::error::Error for JwtError {}

/// What a token must say for this verifier: the issuer and audience it is configured
/// with, never values read from the token, and the current Unix time.
#[derive(Clone, Copy)]
pub struct Expected<'a> {
    /// Central's origin.
    pub issuer: &'a str,
    /// The verifier's own origin.
    pub audience: &'a str,
    /// Unix seconds.
    pub now: i64,
}

fn encode(bytes: &[u8]) -> String {
    Base64UrlUnpadded::encode_string(bytes)
}

fn header(kind: Kind, key_id: &str) -> String {
    let mut text = Vec::with_capacity(96);
    text.extend_from_slice(HEADER_START);
    text.extend_from_slice(kind.typ().as_bytes());
    text.extend_from_slice(HEADER_KID);
    text.extend_from_slice(key_id.as_bytes());
    text.extend_from_slice(HEADER_END);
    encode(&text)
}

/// The `kid` of a header that is exactly what [`header`] writes for `kind`.
fn header_kid(bytes: &[u8], kind: Kind) -> Result<&str, JwtError> {
    let rest = bytes
        .strip_prefix(HEADER_START)
        .ok_or(JwtError::Algorithm)?;
    let rest = rest
        .strip_prefix(kind.typ().as_bytes())
        .ok_or(JwtError::Kind)?;
    let key_id = rest
        .strip_prefix(HEADER_KID)
        .and_then(|rest| rest.strip_suffix(HEADER_END))
        .ok_or(JwtError::Malformed)?;
    if key_id.len() != KID_LEN
        || !key_id
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        return Err(JwtError::Malformed);
    }
    std::str::from_utf8(key_id).map_err(|_| JwtError::Malformed)
}

/// Check everything but the claims' own meaning, and return the signed payload.
fn verified_payload(token: &str, kind: Kind, keys: &KeySet) -> Result<Vec<u8>, JwtError> {
    if token.len() > kind.max_bytes() {
        return Err(JwtError::Malformed);
    }
    let mut parts = token.split('.');
    let (Some(head), Some(body), Some(signature), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(JwtError::Malformed);
    };
    if head.is_empty() || body.is_empty() {
        return Err(JwtError::Malformed);
    }
    let header_bytes = Base64UrlUnpadded::decode_vec(head).map_err(|_| JwtError::Malformed)?;
    let key = keys
        .find(header_kid(&header_bytes, kind)?)
        .ok_or(JwtError::UnknownKey)?;
    let signature: [u8; SIGNATURE_BYTES] = Base64UrlUnpadded::decode_vec(signature)
        .map_err(|_| JwtError::Malformed)?
        .try_into()
        .map_err(|_| JwtError::Malformed)?;
    let signed = &token[..head.len() + 1 + body.len()];
    key.verifying()
        .verify_strict(signed.as_bytes(), &Signature::from_bytes(&signature))
        .map_err(|_| JwtError::Signature)?;
    Base64UrlUnpadded::decode_vec(body).map_err(|_| JwtError::Malformed)
}

/// Verify a token of `C`'s kind: size, header, key, signature, then the claims.
///
/// # Errors
/// [`JwtError`] says which rule refused it. Answer the peer with one fixed refusal.
pub fn verify<C: Claims>(
    token: &str,
    keys: &KeySet,
    expected: Expected<'_>,
) -> Result<C, JwtError> {
    let payload = verified_payload(token, C::KIND, keys)?;
    let claims: C = serde_json::from_slice(&payload).map_err(|_| JwtError::Claims)?;
    let time = claims.registered();
    if time.iss != expected.issuer {
        return Err(JwtError::Issuer);
    }
    if time.aud != expected.audience {
        return Err(JwtError::Audience);
    }
    if time.iat > time.nbf
        || time.nbf >= time.exp
        || time.exp.saturating_sub(time.iat) > C::KIND.max_seconds()
    {
        return Err(JwtError::Claims);
    }
    if expected.now >= time.exp.saturating_add(LEEWAY_SECONDS) {
        return Err(JwtError::Expired);
    }
    if time.nbf.saturating_sub(LEEWAY_SECONDS) > expected.now {
        return Err(JwtError::NotYetValid);
    }
    if !claims.well_formed() {
        return Err(JwtError::Claims);
    }
    Ok(claims)
}
