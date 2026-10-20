//! Randomness from the operating system (`getrandom`), for fixture
//! passwords, temporary names and scanner canaries. Never a clock or a
//! counter: these values must not be guessable by another local user.

use crate::fail::{OrFail, Result};

/// `bytes` random bytes, as lowercase hexadecimal.
///
/// # Errors
/// When the operating system has no randomness to give.
pub fn hex(bytes: usize) -> Result<String> {
    let mut buffer = vec![0_u8; bytes];
    getrandom::fill(&mut buffer).or_fail("Operating-system randomness unavailable")?;
    Ok(to_hex(&buffer))
}

/// `bytes` as lowercase hexadecimal.
#[must_use]
pub fn to_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(char::from(DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    text
}
