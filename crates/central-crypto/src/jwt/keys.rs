//! Ed25519 keys, their `kid` (the RFC 7638 thumbprint) and the JWKS document.
use super::{JwtError, encode};
use base64ct::{Base64UrlUnpadded, Encoding as _};
use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

/// Most keys a set holds.
pub const MAX_KEYS: usize = 8;
/// The longest JWKS document, refused before parsing.
pub const MAX_JWKS_BYTES: usize = 16 * 1024;

/// A public key and the `kid` computed from it.
#[derive(Clone)]
pub struct PublicKey {
    kid: String,
    key: VerifyingKey,
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "PublicKey({})", self.kid)
    }
}

fn thumbprint(x: &str) -> String {
    // RFC 7638: the required members of an OKP key, in lexicographic order.
    let document = format!(r#"{{"crv":"Ed25519","kty":"OKP","x":"{x}"}}"#);
    encode(&Sha256::digest(document.as_bytes()))
}

impl PublicKey {
    /// Accept the 32 bytes of a key.
    ///
    /// # Errors
    /// Refuses bytes that are no curve point, and a key of small order.
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self, JwtError> {
        let key = VerifyingKey::from_bytes(bytes).map_err(|_| JwtError::Malformed)?;
        if key.is_weak() {
            return Err(JwtError::Malformed);
        }
        Ok(Self::from_verifying(key))
    }

    fn from_verifying(key: VerifyingKey) -> Self {
        Self {
            kid: thumbprint(&encode(key.as_bytes())),
            key,
        }
    }

    /// The key's identifier.
    #[must_use]
    pub fn kid(&self) -> &str {
        &self.kid
    }

    /// The key's 32 bytes.
    #[must_use]
    pub fn to_bytes(&self) -> [u8; 32] {
        self.key.to_bytes()
    }

    pub(super) const fn verifying(&self) -> &VerifyingKey {
        &self.key
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Jwk {
    kty: String,
    crv: String,
    alg: String,
    #[serde(rename = "use")]
    usage: String,
    kid: String,
    x: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Jwks {
    keys: Vec<Jwk>,
}

/// The keys tokens may verify under: 1 to [`MAX_KEYS`], each `kid` once.
#[derive(Clone, Debug)]
pub struct KeySet {
    keys: Vec<PublicKey>,
}

impl KeySet {
    /// Hold these keys.
    ///
    /// # Errors
    /// Refuses no key, too many, and one key twice.
    pub fn new(keys: Vec<PublicKey>) -> Result<Self, JwtError> {
        let distinct = keys
            .iter()
            .enumerate()
            .all(|(index, key)| keys[..index].iter().all(|other| other.kid != key.kid));
        if keys.is_empty() || keys.len() > MAX_KEYS || !distinct {
            return Err(JwtError::Malformed);
        }
        Ok(Self { keys })
    }

    /// Read a JWKS document. A key that is not Ed25519 for signing, or whose `kid` is
    /// not its thumbprint, refuses the whole document: it is not what Central writes.
    ///
    /// # Errors
    /// [`JwtError::Malformed`] for any departure from that form.
    pub fn from_jwks(document: &[u8]) -> Result<Self, JwtError> {
        if document.len() > MAX_JWKS_BYTES {
            return Err(JwtError::Malformed);
        }
        let parsed: Jwks = serde_json::from_slice(document).map_err(|_| JwtError::Malformed)?;
        if parsed.keys.len() > MAX_KEYS {
            return Err(JwtError::Malformed);
        }
        let mut keys = Vec::with_capacity(parsed.keys.len());
        for jwk in &parsed.keys {
            if jwk.kty != "OKP" || jwk.crv != "Ed25519" || jwk.alg != "EdDSA" || jwk.usage != "sig"
            {
                return Err(JwtError::Malformed);
            }
            let bytes: [u8; 32] = Base64UrlUnpadded::decode_vec(&jwk.x)
                .map_err(|_| JwtError::Malformed)?
                .try_into()
                .map_err(|_| JwtError::Malformed)?;
            let key = PublicKey::from_bytes(&bytes)?;
            if key.kid != jwk.kid {
                return Err(JwtError::Malformed);
            }
            keys.push(key);
        }
        Self::new(keys)
    }

    /// The JWKS document of these keys.
    ///
    /// # Errors
    /// Only if serialization fails.
    pub fn to_jwks(&self) -> Result<Vec<u8>, JwtError> {
        serde_json::to_vec(&Jwks {
            keys: self
                .keys
                .iter()
                .map(|key| Jwk {
                    kty: "OKP".into(),
                    crv: "Ed25519".into(),
                    alg: "EdDSA".into(),
                    usage: "sig".into(),
                    kid: key.kid.clone(),
                    x: encode(key.key.as_bytes()),
                })
                .collect(),
        })
        .map_err(|_| JwtError::Malformed)
    }

    /// The key with this `kid`.
    #[must_use]
    pub fn find(&self, kid: &str) -> Option<&PublicKey> {
        self.keys.iter().find(|key| key.kid == kid)
    }

    /// The keys held.
    #[must_use]
    pub fn keys(&self) -> &[PublicKey] {
        &self.keys
    }
}

#[cfg(feature = "server")]
mod signing {
    use super::super::{Claims, JwtError, encode, header};
    use super::PublicKey;
    use ed25519_dalek::Signer as _;
    use zeroize::Zeroizing;

    /// Central's Ed25519 signing key. Wiped when dropped; it prints only its `kid`.
    pub struct SigningKey {
        key: ed25519_dalek::SigningKey,
        public: PublicKey,
    }

    impl std::fmt::Debug for SigningKey {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(formatter, "SigningKey({})", self.public.kid())
        }
    }

    impl SigningKey {
        /// Restore a key from its 32-byte seed, from private operator configuration.
        #[must_use]
        pub fn from_seed(seed: &Zeroizing<[u8; 32]>) -> Self {
            let key = ed25519_dalek::SigningKey::from_bytes(seed);
            let public = PublicKey::from_verifying(key.verifying_key());
            Self { key, public }
        }

        /// A fresh seed for operator configuration.
        ///
        /// # Errors
        /// [`JwtError::Randomness`] when operating-system randomness is unavailable.
        pub fn generate_seed() -> Result<Zeroizing<[u8; 32]>, JwtError> {
            let mut seed = Zeroizing::new([0; 32]);
            getrandom::fill(seed.as_mut()).map_err(|_| JwtError::Randomness)?;
            Ok(seed)
        }

        #[cfg(test)]
        pub(in crate::jwt) const fn raw(&self) -> &ed25519_dalek::SigningKey {
            &self.key
        }

        /// This key's public half.
        #[must_use]
        pub const fn public(&self) -> &PublicKey {
            &self.public
        }

        /// Sign claims into a compact token. Claims that a verifier would refuse are
        /// refused here, so that Central never hands out a token nobody accepts.
        ///
        /// # Errors
        /// [`JwtError::Claims`] for claims out of range or a token over its size.
        pub fn sign<C: Claims>(&self, claims: &C) -> Result<Zeroizing<String>, JwtError> {
            if !claims.well_formed() {
                return Err(JwtError::Claims);
            }
            let payload = Zeroizing::new(serde_json::to_vec(claims).map_err(|_| JwtError::Claims)?);
            let mut token = Zeroizing::new(header(C::KIND, self.public.kid()));
            token.push('.');
            token.push_str(&Zeroizing::new(encode(&payload)));
            let signature = self.key.sign(token.as_bytes());
            token.push('.');
            token.push_str(&encode(&signature.to_bytes()));
            if token.len() > C::KIND.max_bytes() {
                return Err(JwtError::Claims);
            }
            Ok(token)
        }
    }
}

#[cfg(feature = "server")]
pub use signing::SigningKey;
