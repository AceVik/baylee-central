//! Beta keys: invitations to register on the website (`docs/web-accounts.md`).
use crate::Error;
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

/// Crockford's base32 alphabet: no I, L, O or U.
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
/// The printed key's leading word, optional when read back.
const PREFIX: &[u8; 7] = b"CENTRAL";
/// Symbols of the key itself: 16 of 5 bits are its 80 bits.
const SYMBOLS: usize = 16;

/// Longest text read as a key; longer input is refused before decoding.
pub const MAX_BETA_KEY_TEXT: usize = 64;

/// 80 random bits, printed once as `CENTRAL-XXXX-XXXX-XXXX-XXXX`. Central keeps
/// only [`BetaKey::digest`]. Neither Clone nor Display; Debug redacts it.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct BetaKey([u8; 10]);

impl std::fmt::Debug for BetaKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BetaKey([REDACTED])")
    }
}

impl BetaKey {
    /// A new key from operating-system randomness.
    ///
    /// # Errors
    /// Fails closed when operating-system randomness is unavailable.
    pub fn generate() -> Result<Self, Error> {
        let mut key = Self([0; 10]);
        getrandom::fill(&mut key.0).map_err(|_| Error::Unavailable)?;
        Ok(key)
    }

    /// Read a key as a person typed it: case-insensitive, with `O` read as `0`
    /// and `I` or `L` as `1`, hyphens and spaces anywhere, surrounding white
    /// space ignored, and the leading `CENTRAL` optional.
    ///
    /// # Errors
    /// Rejects longer input than [`MAX_BETA_KEY_TEXT`], a wrong number of
    /// symbols and any symbol outside the alphabet.
    pub fn parse(text: &str) -> Result<Self, Error> {
        if text.len() > MAX_BETA_KEY_TEXT {
            return Err(Error::Rejected);
        }
        let mut symbols = Zeroizing::new([0_u8; PREFIX.len() + SYMBOLS]);
        let mut count = 0;
        for byte in text.trim_ascii().bytes() {
            if byte == b'-' || byte == b' ' {
                continue;
            }
            *symbols.get_mut(count).ok_or(Error::Rejected)? = byte.to_ascii_uppercase();
            count += 1;
        }
        let body = match count {
            SYMBOLS => &symbols[..SYMBOLS],
            _ if count == symbols.len() && symbols.starts_with(PREFIX) => &symbols[PREFIX.len()..],
            _ => return Err(Error::Rejected),
        };
        let mut key = Self([0; 10]);
        for (index, symbol) in body.iter().enumerate() {
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
                    key.0[position / 8] |= 0x80 >> (position % 8);
                }
            }
        }
        Ok(key)
    }

    /// The printed form, `CENTRAL-` and four groups of four symbols.
    #[must_use]
    pub fn display(&self) -> Zeroizing<String> {
        let mut text = Zeroizing::new(String::with_capacity(27));
        text.push_str("CENTRAL");
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

    /// The SHA-256 digest of the key's 80 bits, which is all Central stores.
    #[must_use]
    pub fn digest(&self) -> [u8; 32] {
        Sha256::digest(self.0).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(text: &str) -> [u8; 10] {
        BetaKey::parse(text).unwrap().0
    }

    #[test]
    fn known_keys_print_and_read_back() {
        // Computed independently (Python): 0x00..=0x09 as 80 bits in groups of five.
        let key = BetaKey([0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        assert_eq!(key.display().as_str(), "CENTRAL-000G-40R4-0M30-E209");
        assert_eq!(bytes("CENTRAL-000G-40R4-0M30-E209"), key.0);
        assert_eq!(
            BetaKey([0; 10]).display().as_str(),
            "CENTRAL-0000-0000-0000-0000"
        );
        assert_eq!(
            BetaKey([255; 10]).display().as_str(),
            "CENTRAL-ZZZZ-ZZZZ-ZZZZ-ZZZZ"
        );
        assert_eq!(
            key.digest(),
            <[u8; 32]>::from(Sha256::digest([0, 1, 2, 3, 4, 5, 6, 7, 8, 9]))
        );
    }

    #[test]
    fn generated_keys_round_trip_and_differ() {
        let key = BetaKey::generate().unwrap();
        let printed = key.display();
        assert_eq!(printed.len(), 27);
        assert_eq!(BetaKey::parse(&printed).unwrap().digest(), key.digest());
        assert_ne!(BetaKey::generate().unwrap().digest(), key.digest());
        assert_eq!(format!("{key:?}"), "BetaKey([REDACTED])");
    }

    #[test]
    fn a_typed_key_is_read_like_the_recovery_code() {
        let expected = bytes("CENTRAL-000G-40R4-0M30-E209");
        for typed in [
            "central-000g-40r4-0m30-e209",
            "000G-40R4-0M30-E209",
            "000G40R40M30E209",
            "  CENTRAL 000G 40R4 0M30 E209\n",
            "CENTRAL-OOOG-4OR4-OM3O-E2O9",
            "CENTRAL000G-40R4-0M30-E209",
        ] {
            assert_eq!(bytes(typed), expected, "{typed}");
        }
        assert_eq!(bytes("IIII-LLLL-iiii-llll"), bytes("1111-1111-1111-1111"));
    }

    #[test]
    fn malformed_keys_are_refused() {
        for typed in [
            "",
            "CENTRAL",
            "CENTRAL-0000-8100-G40M-30W",
            "CENTRAL-0000-8100-G40M-30W99",
            "0000-8100-G40M-30W",
            "0000-8100-G40M-30WU",
            "0000-8100-G40M-30W*",
            "BAYLEE-0000-8100-G40M-30W9",
            "CENTRAL_0000_8100_G40M_30W9",
            "0000-8100-G40M-30W9\t1",
            "CENTRAL-0000-8100-G40M-30W9-CENTRAL",
            "０000-8100-G40M-30W9",
        ] {
            assert!(BetaKey::parse(typed).is_err(), "{typed}");
        }
        let padded = format!("{}0000-8100-G40M-30W9", "-".repeat(MAX_BETA_KEY_TEXT));
        assert!(BetaKey::parse(&padded).is_err());
    }
}
