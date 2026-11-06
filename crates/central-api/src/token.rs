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
    /// Import a capability from a privileged transport or secret store.
    /// This does not validate authority; the receiving service still checks its digest and scope.
    #[must_use]
    pub fn from_secret_bytes(bytes: Zeroizing<[u8; 32]>) -> Self {
        let token = Self(*bytes);
        drop(bytes);
        token
    }

    /// Export to a privileged secret store in a zeroizing buffer. Never log or expose this value.
    #[must_use]
    pub fn to_secret_bytes(&self) -> Zeroizing<[u8; 32]> {
        Zeroizing::new(self.0)
    }

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
        let mut out = Zeroizing::new(String::with_capacity(71));
        out.push_str("Bearer ");
        out.push_str(&self.hex());
        out
    }

    /// The 64 lowercase hexadecimal digits, as a cookie carries them, in a
    /// zeroizing buffer; never log it.
    #[must_use]
    pub fn hex(&self) -> Zeroizing<String> {
        use std::fmt::Write;
        let mut out = Zeroizing::new(String::with_capacity(64));
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
        Self::from_hex(value.strip_prefix("Bearer ").ok_or(Error::Rejected)?)
    }

    /// Decode exactly 64 lowercase hexadecimal digits, as [`Token::hex`] writes them.
    ///
    /// # Errors
    /// Rejects any other length, case or character.
    pub fn from_hex(text: &str) -> Result<Self, Error> {
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

/// A signed token as it travels: a compact JWS. To a client it is an opaque string, which it
/// stores and presents and never parses. Deliberately neither Clone nor Display; Debug
/// always redacts it, and it is wiped when dropped.
#[derive(Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(transparent)]
pub struct SignedToken(String);

impl std::fmt::Debug for SignedToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SignedToken([REDACTED])")
    }
}

impl SignedToken {
    /// The longest token accepted from a peer.
    pub const MAX_BYTES: usize = 16 * 1024;

    /// Take a token just signed, or read from a privileged secret store.
    ///
    /// # Errors
    /// Rejects text too long, or outside the compact form's alphabet.
    pub fn new(mut token: Zeroizing<String>) -> Result<Self, Error> {
        let well_formed = token.len() <= Self::MAX_BYTES
            && !token.is_empty()
            && token
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'));
        if !well_formed {
            return Err(Error::Rejected);
        }
        Ok(Self(std::mem::take(&mut *token)))
    }

    /// The token's text, for the one peer it is meant for. Never log it.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// Encode the bearer header into a zeroizing buffer; never log it.
    #[must_use]
    pub fn bearer(&self) -> Zeroizing<String> {
        let mut out = Zeroizing::new(String::with_capacity(self.0.len() + 7));
        out.push_str("Bearer ");
        out.push_str(&self.0);
        out
    }
}

/// What a request presents: a gateway's random credential or a client's signed session.
#[derive(Clone, Copy)]
pub enum Bearer<'a> {
    /// A gateway credential, a heartbeat credential or the bridge's.
    Capability(&'a Token),
    /// The client's Central token.
    Signed(&'a SignedToken),
}

impl Bearer<'_> {
    /// Encode the bearer header into a zeroizing buffer; never log it.
    #[must_use]
    pub fn header(self) -> Zeroizing<String> {
        match self {
            Self::Capability(token) => token.bearer(),
            Self::Signed(token) => token.bearer(),
        }
    }
}

impl<'a> From<&'a Token> for Bearer<'a> {
    fn from(token: &'a Token) -> Self {
        Self::Capability(token)
    }
}

impl<'a> From<&'a SignedToken> for Bearer<'a> {
    fn from(token: &'a SignedToken) -> Self {
        Self::Signed(token)
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
        assert_eq!(
            Token::from_secret_bytes(token.to_secret_bytes()).digest(),
            token.digest()
        );
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
        let hex = token.hex();
        assert_eq!(hex.len(), 64);
        assert_eq!(Token::from_hex(&hex).unwrap().digest(), token.digest());
        assert_eq!(encoded.as_str(), format!("Bearer {}", hex.as_str()));
        for text in [
            &hex[1..],
            &format!("{}0", hex.as_str()),
            &hex.to_uppercase(),
            &format!(" {}", &hex[1..]),
            &format!("{}g", &hex[1..]),
        ] {
            assert!(Token::from_hex(text).is_err());
        }
    }

    fn signed(text: &str) -> Result<SignedToken, Error> {
        SignedToken::new(Zeroizing::new(text.to_owned()))
    }

    #[test]
    fn a_signed_token_prints_nothing_and_keeps_to_its_alphabet() {
        let token = signed("aA0-_.bB1.cC2").unwrap();
        assert_eq!(format!("{token:?}"), "SignedToken([REDACTED])");
        assert_eq!(token.bearer().as_str(), "Bearer aA0-_.bB1.cC2");
        assert_eq!(
            Bearer::from(&token).header().as_str(),
            "Bearer aA0-_.bB1.cC2"
        );
        assert_eq!(serde_json::to_string(&token).unwrap(), "\"aA0-_.bB1.cC2\"");
        for text in ["", "a b", "a\nb", "a=b", "a/b", "a+b", "\u{e9}"] {
            assert_eq!(signed(text).err(), Some(Error::Rejected), "{text:?}");
        }
        assert!(signed(&"a".repeat(SignedToken::MAX_BYTES)).is_ok());
        assert!(signed(&"a".repeat(SignedToken::MAX_BYTES + 1)).is_err());
    }

    #[test]
    fn a_capability_presents_its_own_bearer() {
        let token = Token::generate().unwrap();
        assert_eq!(
            Bearer::from(&token).header().as_str(),
            token.bearer().as_str()
        );
    }
}
