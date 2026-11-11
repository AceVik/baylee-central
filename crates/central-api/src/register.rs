//! Open registration's proof of work (`docs/registration-abuse.md`): no
//! service and no tracker, only time. Central issues a random nonce and a
//! difficulty; for each part `p` of the challenge the registering side finds
//! the smallest counter `c` whose SHA-256 over the ASCII text
//! `nonce ":" p ":" c` starts with `difficulty` zero bits, the puzzle the
//! website's Impressum already solves in its Worker. Several parts keep the
//! time close to its average, whatever luck a single part has.
//!
//! This is no cryptography: the work only costs time, and every answer is
//! checked with a reviewed SHA-256.

use sha2::{Digest, Sha256};

/// Parts in every challenge.
pub const PARTS: u8 = 8;
/// The nonce's length: 32 random bytes in lowercase hex.
pub const NONCE_LEN: usize = 64;
/// The highest difficulty Central asks for or a client accepts.
pub const MAX_DIFFICULTY: u8 = 30;

fn zero_bits(hash: &[u8]) -> u32 {
    let mut bits = 0;
    for byte in hash {
        if *byte == 0 {
            bits += 8;
        } else {
            return bits + byte.leading_zeros();
        }
    }
    bits
}

fn hash(nonce: &str, part: u8, counter: u64) -> [u8; 32] {
    Sha256::digest(format!("{nonce}:{part}:{counter}").as_bytes()).into()
}

/// A nonce of the issued shape: `NONCE_LEN` lowercase hex digits.
#[must_use]
pub fn valid_nonce(nonce: &str) -> bool {
    nonce.len() == NONCE_LEN
        && nonce
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Whether `counters` solve the challenge: one per part, each with at least
/// `difficulty` leading zero bits.
#[must_use]
pub fn solved(nonce: &str, difficulty: u8, counters: &[u64]) -> bool {
    valid_nonce(nonce)
        && difficulty <= MAX_DIFFICULTY
        && counters.len() == usize::from(PARTS)
        && counters
            .iter()
            .zip(0..PARTS)
            .all(|(counter, part)| zero_bits(&hash(nonce, part, *counter)) >= u32::from(difficulty))
}

/// Finds the counters: for a client that registers without a browser and
/// for tests. `None` for a challenge no client should accept.
#[must_use]
pub fn solve(nonce: &str, parts: u8, difficulty: u8) -> Option<Vec<u64>> {
    if !valid_nonce(nonce) || parts != PARTS || difficulty > MAX_DIFFICULTY {
        return None;
    }
    (0..parts)
        .map(|part| {
            (0..u64::MAX).find(|c| zero_bits(&hash(nonce, part, *c)) >= u32::from(difficulty))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONCE: &str = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

    #[test]
    fn solves_and_checks() {
        let counters = solve(NONCE, PARTS, 8).expect("a challenge");
        assert_eq!(counters.len(), usize::from(PARTS));
        assert!(solved(NONCE, 8, &counters));
        assert!(solved(NONCE, 0, &counters));
        let mut wrong = counters.clone();
        wrong[3] = (0..u64::MAX)
            .find(|c| zero_bits(&hash(NONCE, 3, *c)) < 8)
            .expect("a miss");
        assert!(!solved(NONCE, 8, &wrong));
        assert!(!solved(NONCE, 8, &counters[..7]), "every part");
    }

    #[test]
    fn refuses_what_was_not_issued() {
        assert!(!valid_nonce("short"));
        assert!(!valid_nonce(&"A".repeat(NONCE_LEN)));
        assert!(solve(NONCE, PARTS, MAX_DIFFICULTY + 1).is_none());
        assert!(solve(NONCE, 4, 4).is_none());
        assert!(!solved(NONCE, MAX_DIFFICULTY + 1, &[0; 8]));
    }

    #[test]
    fn the_same_puzzle_as_the_impressum() {
        // SHA-256("n:0:0"): its first byte decides the zero bits
        let first = hash("n", 0, 0);
        let expected: [u8; 32] = Sha256::digest(b"n:0:0").into();
        assert_eq!(first, expected);
        assert_eq!(zero_bits(&[0, 0, 0b0001_0000]), 19);
        assert_eq!(zero_bits(&[0x80]), 0);
    }
}
