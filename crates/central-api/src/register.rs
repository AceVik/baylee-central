//! Open registration's proof of work (`docs/registration-abuse.md`): no
//! service and no tracker, only work. Central issues a random nonce and a
//! price; for each part of the challenge the registering side finds the
//! smallest counter whose hash starts with `difficulty` zero bits. Several
//! parts keep the time close to its average, whatever luck a single part
//! has.
//!
//! Two hashes. The proof Central asks for and takes costs memory: Argon2id
//! over the nonce, the part and the counter, at a memory and a number of
//! passes Central sets ([`crate::Response::RegisterWork`]). It is computed
//! by `central_crypto::proof`, one function for the page's Worker and for
//! Central; this crate has no Argon2 and carries only what both agree on
//! before any work, the ceilings below. The older proof is SHA-256 over the
//! ASCII text `nonce ":" p ":" c` ([`solved`], [`solve`]), the puzzle the
//! website's Impressum solves in its Worker: a Central of this build
//! neither issues nor takes it for a registration, and it stays here for a
//! page that meets a Central from before.
//!
//! This is no cryptography: the work only costs its solver, and every
//! answer is checked with a reviewed hash.

use sha2::{Digest, Sha256};

/// Parts in every challenge of the older, SHA-256 proof.
pub const PARTS: u8 = 8;
/// The nonce's length: 32 random bytes in lowercase hex.
pub const NONCE_LEN: usize = 64;
/// The highest difficulty of the older, SHA-256 proof a client accepts:
/// what a Central from before asked at its most, sixteen bits when
/// registrations were few and six more. A page takes none above it, so it
/// never works on one it could not hand in.
pub const MAX_DIFFICULTY: u8 = 22;

/// What a page says of its own browser when it asks for a challenge
/// (`Request::RegisterChallengeFor`), one bit each, the bits the Impressum's
/// page sends its service: a browser a program drives (`navigator.webdriver`).
pub const SIGNAL_WEBDRIVER: u8 = 1;
/// A browser without languages (`navigator.languages` is empty).
pub const SIGNAL_NO_LANGUAGES: u8 = 2;
/// Every bit Central reads; it ignores the others, so a later page may
/// send more.
pub const SIGNALS: u8 = SIGNAL_WEBDRIVER | SIGNAL_NO_LANGUAGES;

/// The least memory of one evaluation of the memory-hard proof, in KiB.
pub const WORK_MIN_MEMORY: u32 = 8;
/// The most memory of one evaluation, in KiB: 64 MiB, what the password
/// stretch already takes in the page's Worker.
pub const WORK_MAX_MEMORY: u32 = 65_536;
/// The most passes over the memory.
pub const WORK_MAX_PASSES: u32 = 4;
/// The most parts of one memory-hard challenge.
pub const WORK_MAX_PARTS: u8 = 16;
/// The most leading zero bits a part of it may be asked for.
pub const WORK_MAX_DIFFICULTY: u8 = 16;
/// The most a whole memory-hard challenge may cost ([`work_cost`]): what
/// Central asks at its ceiling, and not one unit more. Each number above
/// bounds one field; this bounds their product, so a page never starts on
/// a challenge it could not hand in within its lifetime. The identity
/// service's `challenge` asserts that its ceiling is exactly this.
pub const WORK_MAX_COST: u64 = work_cost(19_456, 1, 8, 10);

/// What a memory-hard challenge costs its solver on average, in KiB of
/// memory filled: every evaluation fills `memory` KiB `passes` times, and
/// a part takes `2^difficulty` evaluations on average. Argon2's time
/// follows this product, so two prices of the same cost take a solver
/// about the same time.
#[must_use]
pub const fn work_cost(memory: u32, passes: u32, parts: u8, difficulty: u8) -> u64 {
    let evaluations = if difficulty < 32 {
        1_u64 << difficulty
    } else {
        1_u64 << 32
    };
    (memory as u64)
        .saturating_mul(passes as u64)
        .saturating_mul(parts as u64)
        .saturating_mul(evaluations)
}

/// Whether a side takes the price of a memory-hard challenge: every field
/// within its ceiling and the whole within [`WORK_MAX_COST`]. The page
/// asks this before it starts, as it holds the older proof to
/// [`MAX_DIFFICULTY`].
#[must_use]
pub const fn work_acceptable(memory: u32, passes: u32, parts: u8, difficulty: u8) -> bool {
    memory >= WORK_MIN_MEMORY
        && memory <= WORK_MAX_MEMORY
        && passes >= 1
        && passes <= WORK_MAX_PASSES
        && parts >= 1
        && parts <= WORK_MAX_PARTS
        && difficulty <= WORK_MAX_DIFFICULTY
        && work_cost(memory, passes, parts, difficulty) <= WORK_MAX_COST
}

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
    fn the_ceiling_is_sixty_four_times_the_base_work() {
        // 16 bits and 6 more: what a Central from before asked at its most
        assert_eq!(MAX_DIFFICULTY, 22);
        assert_eq!(SIGNALS, 3);
        assert_eq!(SIGNAL_WEBDRIVER & SIGNAL_NO_LANGUAGES, 0);
    }

    #[test]
    fn a_price_is_taken_up_to_each_ceiling_and_up_to_the_whole() {
        assert!(work_acceptable(19_456, 1, 8, 10));
        assert_eq!(work_cost(19_456, 1, 8, 10), WORK_MAX_COST);
        // the same cost in another shape is taken, one bit more is not
        assert!(work_acceptable(9_728, 2, 8, 10));
        assert!(work_acceptable(19_456, 1, 4, 11));
        assert!(!work_acceptable(19_456, 1, 8, 11));
        assert!(!work_acceptable(19_456, 2, 8, 10));
        assert!(!work_acceptable(19_457, 1, 8, 10));
        // each field's own ceiling, however cheap the rest
        assert!(work_acceptable(WORK_MIN_MEMORY, 1, 1, 0));
        assert!(!work_acceptable(WORK_MIN_MEMORY - 1, 1, 1, 0));
        assert!(work_acceptable(WORK_MAX_MEMORY, 1, 1, 0));
        assert!(!work_acceptable(WORK_MAX_MEMORY + 1, 1, 1, 0));
        assert!(!work_acceptable(64, 0, 1, 0));
        assert!(work_acceptable(64, WORK_MAX_PASSES, 1, 0));
        assert!(!work_acceptable(64, WORK_MAX_PASSES + 1, 1, 0));
        assert!(!work_acceptable(64, 1, 0, 0));
        assert!(work_acceptable(64, 1, WORK_MAX_PARTS, 0));
        assert!(!work_acceptable(64, 1, WORK_MAX_PARTS + 1, 0));
        assert!(work_acceptable(8, 1, 1, WORK_MAX_DIFFICULTY));
        assert!(!work_acceptable(8, 1, 1, WORK_MAX_DIFFICULTY + 1));
        // no number a hostile answer could carry wraps the product
        assert_eq!(work_cost(u32::MAX, u32::MAX, u8::MAX, u8::MAX), u64::MAX);
        assert!(!work_acceptable(u32::MAX, u32::MAX, u8::MAX, u8::MAX));
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
