//! Safe adapters to maintained RFC 8291 and VAPID implementations. This crate
//! never receives chat plaintext or E2E keys. Dependency internals include C/assembly;
//! see docs/dependency-proposal/push.md. All errors are deliberately opaque.
#![forbid(unsafe_code)]
use base64ct::{Base64UrlUnpadded, Encoding as _};
use central_api::{
    Error,
    notifications::{Category, Subscription, endpoint},
};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use openssl::{
    bn::BigNumContext,
    ec::{EcGroup, EcKey, EcPoint, PointConversionForm},
    nid::Nid,
    pkey::PKey,
};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};
use zeroize::Zeroizing;
mod transport;

/// Provider outcome; no response body, endpoint or token is exposed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    /// Accepted by the push provider; device delivery remains best effort.
    Delivered,
    /// Endpoint no longer exists, and may be removed.
    Gone,
    /// Temporary or indeterminate failure; retry with a bounded backoff.
    Retry,
}
/// Persistent deployment key plus a bounded HTTPS agent. No secret Debug impl.
pub struct Sender {
    key: EncodingKey,
    public: String,
    contact: String,
    http: ureq::Agent,
}
#[derive(Serialize)]
struct Claims<'a> {
    aud: &'a str,
    exp: u64,
    sub: &'a str,
}
#[derive(Serialize)]
struct Hint<'a> {
    v: u8,
    kind: Category,
    lang: &'a str,
    nonce: &'a str,
}
fn rejected(_: impl Sized) -> Error {
    Error::Rejected
}
impl Sender {
    /// Restore a persistent P-256 PKCS#8 key from private stdin configuration.
    /// # Errors
    /// Rejects another curve, invalid key, contact or conflicting crypto provider.
    pub fn new(der: &[u8], contact: String) -> Result<Self, Error> {
        if der.len() > 1024
            || contact.len() > 200
            || !contact.starts_with("https://")
            || contact.chars().any(char::is_control)
        {
            return Err(Error::Rejected);
        }
        if contact != "https://baylee.app/contact" {
            return Err(Error::Rejected);
        }
        if let Err(provider) = jsonwebtoken_openssl::install_default()
            && !std::ptr::eq(provider, &raw const jsonwebtoken_openssl::DEFAULT_PROVIDER)
        {
            return Err(Error::Rejected);
        }
        let key = PKey::private_key_from_der(der)
            .map_err(rejected)?
            .ec_key()
            .map_err(rejected)?;
        if key.group().curve_name() != Some(Nid::X9_62_PRIME256V1) {
            return Err(Error::Rejected);
        }
        key.check_key().map_err(rejected)?;
        let mut context = BigNumContext::new().map_err(rejected)?;
        let public = key
            .public_key()
            .to_bytes(key.group(), PointConversionForm::UNCOMPRESSED, &mut context)
            .map_err(rejected)?;
        Ok(Self {
            key: EncodingKey::from_ec_der(der),
            public: Base64UrlUnpadded::encode_string(&public),
            contact,
            http: transport::agent(),
        })
    }
    /// Public application server key supplied to PushManager.subscribe.
    #[must_use]
    pub fn public_key(&self) -> &str {
        &self.public
    }
    /// Generate a persistent key for private operator configuration, never logging it.
    /// # Errors
    /// Operating-system randomness or crypto-provider failure.
    pub fn generate_key() -> Result<Zeroizing<Vec<u8>>, Error> {
        let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).map_err(rejected)?;
        let key = EcKey::generate(&group).map_err(rejected)?;
        Ok(Zeroizing::new(
            PKey::from_ec_key(key)
                .map_err(rejected)?
                .private_key_to_pkcs8()
                .map_err(rejected)?,
        ))
    }
    fn prepare(&self, sub: &Subscription, category: Category) -> Result<(String, Vec<u8>), Error> {
        sub.validate()?;
        let url = endpoint(&sub.endpoint)?;
        let public = Base64UrlUnpadded::decode_vec(&sub.p256dh).map_err(rejected)?;
        let auth = Zeroizing::new(Base64UrlUnpadded::decode_vec(&sub.auth).map_err(rejected)?);
        if public.len() != 65 || public[0] != 4 || auth.len() != 16 {
            return Err(Error::Rejected);
        }
        let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).map_err(rejected)?;
        let mut context = BigNumContext::new().map_err(rejected)?;
        let point = EcPoint::from_bytes(&group, &public, &mut context).map_err(rejected)?;
        EcKey::from_public_key(&group, &point)
            .map_err(rejected)?
            .check_key()
            .map_err(rejected)?;
        let payload = serde_json::to_vec(&Hint {
            v: 1,
            kind: category,
            lang: &sub.lang,
            nonce: &sub.nonce,
        })
        .map_err(rejected)?;
        let encrypted = ece::encrypt(&public, &auth, &payload).map_err(rejected)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(rejected)?
            .as_secs();
        let audience = url.origin().ascii_serialization();
        let jwt = jsonwebtoken::encode(
            &Header::new(Algorithm::ES256),
            &Claims {
                aud: &audience,
                exp: now + 3600,
                sub: &self.contact,
            },
            &self.key,
        )
        .map_err(rejected)?;
        Ok((format!("vapid t={jwt}, k={}", self.public), encrypted))
    }
    /// Send one generic category hint over verified TLS, with no redirects or proxies.
    /// # Errors
    /// Invalid subscription; network/provider failures are an opaque retry outcome.
    pub fn send(&self, sub: &Subscription, category: Category) -> Result<Outcome, Error> {
        let (authorization, payload) = self.prepare(sub, category)?;
        let result = self
            .http
            .post(&sub.endpoint)
            .header("Authorization", &authorization)
            .header("TTL", "86400")
            .header("Content-Encoding", "aes128gcm")
            .header("Content-Type", "application/octet-stream")
            .header("Urgency", "normal")
            .send(payload);
        Ok(match result {
            Ok(response) => match response.status().as_u16() {
                200..=299 => Outcome::Delivered,
                404 | 410 => Outcome::Gone,
                _ => Outcome::Retry,
            },
            Err(_) => Outcome::Retry,
        })
    }
}
#[cfg(test)]
mod tests;
