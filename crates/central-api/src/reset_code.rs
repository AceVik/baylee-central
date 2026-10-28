//! Reset codes: an administrator's one-use permission to set a new password
//! (`docs/account-reset.md`). Central keeps only the digest.
use crate::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

/// Crockford's base32 alphabet: no I, L, O or U.
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
/// The printed code's leading word, required when read back, so that a
/// recovery code (the same alphabet and length) is never taken for one.
const PREFIX: &[u8; 5] = b"RESET";
/// Symbols of the code itself: 32 of 5 bits are its 160 bits.
const SYMBOLS: usize = 32;
/// Longest text read as a code; longer input is refused before decoding.
pub const MAX_RESET_CODE_TEXT: usize = 128;

/// 160 random bits, shown once as `RESET-` and eight groups of four symbols.
/// Neither Clone nor Display; Debug redacts it. It travels as its printed form
/// only in the administrator's answer and is read back from what the person
/// types.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct ResetCode([u8; 20]);

impl std::fmt::Debug for ResetCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ResetCode([REDACTED])")
    }
}

impl ResetCode {
    /// A new code from operating-system randomness.
    ///
    /// # Errors
    /// Fails closed when operating-system randomness is unavailable.
    pub fn generate() -> Result<Self, Error> {
        let mut code = Self([0; 20]);
        getrandom::fill(&mut code.0).map_err(|_| Error::Unavailable)?;
        Ok(code)
    }

    /// Read a code as a person typed it: case-insensitive, with `O` read as
    /// `0` and `I` or `L` as `1`, hyphens and ASCII white space anywhere, and
    /// the leading `RESET` required.
    ///
    /// # Errors
    /// Rejects longer input than [`MAX_RESET_CODE_TEXT`], a missing prefix, a
    /// wrong number of symbols and any symbol outside the alphabet.
    pub fn parse(text: &str) -> Result<Self, Error> {
        if text.len() > MAX_RESET_CODE_TEXT {
            return Err(Error::Rejected);
        }
        let mut symbols = Zeroizing::new([0_u8; PREFIX.len() + SYMBOLS]);
        let mut count = 0;
        for byte in text.bytes() {
            if byte == b'-' || byte.is_ascii_whitespace() {
                continue;
            }
            *symbols.get_mut(count).ok_or(Error::Rejected)? = byte.to_ascii_uppercase();
            count += 1;
        }
        if count != symbols.len() || !symbols.starts_with(PREFIX) {
            return Err(Error::Rejected);
        }
        let mut code = Self([0; 20]);
        for (index, symbol) in symbols[PREFIX.len()..].iter().enumerate() {
            let symbol = match symbol {
                b'O' => b'0',
                b'I' | b'L' => b'1',
                other => *other,
            };
            let value = ALPHABET
                .iter()
                .position(|a| *a == symbol)
                .ok_or(Error::Rejected)?;
            for bit in 0..5 {
                if value >> (4 - bit) & 1 == 1 {
                    let position = index * 5 + bit;
                    code.0[position / 8] |= 0x80 >> (position % 8);
                }
            }
        }
        Ok(code)
    }

    /// The printed form, `RESET-` and eight groups of four symbols.
    #[must_use]
    pub fn display(&self) -> Zeroizing<String> {
        let mut text = Zeroizing::new(String::with_capacity(45));
        text.push_str("RESET");
        for index in 0..SYMBOLS {
            if index % 4 == 0 {
                text.push('-');
            }
            let mut value = 0;
            for bit in 0..5 {
                let position = index * 5 + bit;
                value = value << 1 | usize::from(self.0[position / 8] >> (7 - position % 8) & 1);
            }
            text.push(char::from(ALPHABET[value]));
        }
        text
    }

    /// The SHA-256 digest of the code's 160 bits, which is all Central stores.
    #[must_use]
    pub fn digest(&self) -> [u8; 32] {
        Sha256::digest(self.0).into()
    }
}

impl Serialize for ResetCode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.display())
    }
}

impl<'de> Deserialize<'de> for ResetCode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = Zeroizing::new(String::deserialize(deserializer)?);
        Self::parse(&text).map_err(|_| serde::de::Error::custom("invalid reset code"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_print_and_read_back_and_a_recovery_code_is_none() {
        // 0x00..=0x13 as 160 bits in groups of five (computed apart, Python).
        let code = ResetCode(core::array::from_fn(|i| u8::try_from(i).unwrap()));
        let printed = code.display();
        assert_eq!(
            printed.as_str(),
            "RESET-000G-40R4-0M30-E209-185G-R38E-1W81-24GK"
        );
        assert_eq!(ResetCode::parse(&printed).unwrap().0, code.0);
        assert_eq!(
            ResetCode::parse(" reset 000g 40r4 0m3o e209 i85g r38e iw8i 24gk ")
                .unwrap()
                .0,
            code.0
        );
        assert_eq!(code.digest(), <[u8; 32]>::from(Sha256::digest(code.0)));
        // A recovery code has the same symbols, but no prefix.
        for bad in [
            "000G-40R4-0M30-E209-185G-R38E-1W81-24GK",
            "RESET-000G-40R4-0M30-E209-185G-R38E-1W81-24G",
            "RESET-000G-40R4-0M30-E209-185G-R38E-1W81-24GKK",
            "RESET-000G-40R4-0M30-E209-185G-R38E-1W81-24GU",
            "CENTRAL-000G-40R4-0M30-E209-185G-R38E-1W81-24GK",
            "",
        ] {
            assert!(ResetCode::parse(bad).is_err(), "{bad}");
        }
        assert!(ResetCode::parse(&format!("{}{}", printed.as_str(), " ".repeat(100))).is_err());
        assert_eq!(format!("{code:?}"), "ResetCode([REDACTED])");
    }

    #[test]
    fn a_code_travels_as_its_printed_form_and_differs_each_time() {
        let code = ResetCode::generate().unwrap();
        let json = crate::encode(&code).unwrap();
        assert_eq!(
            String::from_utf8(json.clone()).unwrap(),
            format!("\"{}\"", code.display().as_str())
        );
        assert_eq!(crate::decode::<ResetCode>(&json).unwrap().0, code.0);
        assert!(crate::decode::<ResetCode>(b"\"RESET-0\"").is_err());
        assert_ne!(ResetCode::generate().unwrap().0, code.0);
    }
}
