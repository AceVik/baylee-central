use crate::Error;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

/// Random opaque bearer capability. Only its SHA-256 digest is persisted server-side.
/// Deliberately neither Clone nor Display; Debug always redacts it.
#[derive(Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(transparent)]
pub struct Token([u8; 32]);

impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Token([REDACTED])")
    }
}

impl Token {
    /// Generate an independent 256-bit operating-system random capability.
    ///
    /// # Errors
    /// Fails closed when operating-system randomness is unavailable.
    pub fn generate() -> Result<Self, Error> {
        let mut bytes = Zeroizing::new([0; 32]);
        getrandom::fill(bytes.as_mut()).map_err(|_| Error::Unavailable)?;
        Ok(Self(*bytes))
    }

    /// Digest for lookup in the capability's own table/namespace.
    #[must_use]
    pub fn digest(&self) -> [u8; 32] {
        Sha256::digest(self.0).into()
    }

    /// Encode the bearer header into a zeroizing buffer; never log it.
    #[must_use]
    pub fn bearer(&self) -> Zeroizing<String> {
        use std::fmt::Write;
        let mut out = Zeroizing::new(String::with_capacity(71));
        out.push_str("Bearer ");
        for byte in self.0 {
            let _ = write!(out, "{byte:02x}");
        }
        out
    }

    /// Decode exactly the canonical bearer spelling; no whitespace or alternate encoding.
    ///
    /// # Errors
    /// Rejects noncanonical, malformed or wrong-sized credentials.
    pub fn from_bearer(value: &str) -> Result<Self, Error> {
        let text = value.strip_prefix("Bearer ").ok_or(Error::Rejected)?;
        if text.len() != 64
            || !text
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(Error::Rejected);
        }
        let mut bytes = Zeroizing::new([0; 32]);
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16)
                .map_err(|_| Error::Rejected)?;
        }
        Ok(Self(*bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tokens_round_trip_but_never_print_their_value() {
        let token = Token::generate().unwrap();
        let encoded = token.bearer();
        assert_eq!(
            Token::from_bearer(&encoded).unwrap().digest(),
            token.digest()
        );
        assert_ne!(Token::generate().unwrap().digest(), token.digest());
        assert_eq!(format!("{token:?}"), "Token([REDACTED])");
        for text in [
            "bearer ",
            "Bearer ",
            "Bearer xyz",
            encoded.trim_end_matches('0'),
        ] {
            if text.len() != 71 {
                assert!(Token::from_bearer(text).is_err());
            }
        }
        assert!(Token::from_bearer(&format!("{} ", encoded.as_str())).is_err());
        assert!(Token::from_bearer(&encoded.to_uppercase()).is_err());
    }
}
