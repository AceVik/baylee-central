//! Registration's proof of work with a cost in memory
//! (`docs/registration-abuse.md` §"A proof that costs memory"): for each
//! part of a challenge, the smallest counter whose Argon2id output starts
//! with `difficulty` zero bits.
//!
//! One function for both sides. [`Proof::holds`] is the only place a
//! counter is judged: the page's Worker searches with it (this crate built
//! to WebAssembly) and Central verifies with it (built natively), so a
//! solution cannot be right on one side and wrong on the other, and a
//! program that is not a page gains little over the page. There is no
//! second implementation, in JavaScript or anywhere.
//!
//! This is no cryptography of ours. Argon2id (RFC 9106, version 0x13) is
//! used as specified through the `argon2` crate's safe API, with one lane
//! and a 32-byte output; nothing stands around it but the encoding of its
//! input, which holds no secret:
//!
//! - salt: the fixed label [`LABEL`], which keeps this use apart from the
//!   password stretch and the legacy store's check;
//! - password: the nonce's 64 ASCII hex digits, the part's index (one
//!   byte) and the counter (eight bytes, big-endian), 73 bytes in all.
//!
//! Central sets the price ([`Work`]) and keeps it with the nonce; a side
//! that is handed a price above the ceilings here refuses it before any
//! work.

use crate::CryptoError;
use argon2::{Algorithm, Argon2, Block, Params, Version};

/// The salt of every evaluation: this use's own domain.
pub const LABEL: &[u8] = b"baylee-central/registration-proof/argon2id/v1";
/// The nonce's length: 32 random bytes in lowercase hex, as Central issues it.
pub const NONCE_LEN: usize = 64;
/// The least memory of one evaluation, in KiB: Argon2's own floor for one lane.
pub const MIN_MEMORY: u32 = 8;
/// The most memory of one evaluation, in KiB: 64 MiB, what the password
/// stretch already takes in the same Worker, so a phone that can register
/// at all has it.
pub const MAX_MEMORY: u32 = 65_536;
/// The most passes over the memory.
pub const MAX_PASSES: u32 = 4;
/// The most parts of one challenge.
pub const MAX_PARTS: u8 = 16;
/// The most leading zero bits a part may be asked for.
pub const MAX_DIFFICULTY: u8 = 16;

const OUTPUT: usize = 32;
const INPUT: usize = NONCE_LEN + 1 + 8;

/// What one challenge costs: set by Central, kept with the nonce, and the
/// only price a solution is checked against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Work {
    /// Memory of one evaluation, in KiB.
    pub memory: u32,
    /// Passes over the memory.
    pub passes: u32,
    /// Parts, each with a counter of its own.
    pub parts: u8,
    /// Leading zero bits each part's output needs.
    pub difficulty: u8,
}

impl Work {
    /// Whether every number is within this module's ceilings.
    #[must_use]
    pub const fn acceptable(&self) -> bool {
        self.memory >= MIN_MEMORY
            && self.memory <= MAX_MEMORY
            && self.passes >= 1
            && self.passes <= MAX_PASSES
            && self.parts >= 1
            && self.parts <= MAX_PARTS
            && self.difficulty <= MAX_DIFFICULTY
    }
}

fn zero_bits(output: &[u8; OUTPUT]) -> u32 {
    let mut bits = 0;
    for byte in output {
        if *byte == 0 {
            bits += 8;
        } else {
            return bits + byte.leading_zeros();
        }
    }
    bits
}

/// One challenge at its price, with the memory its evaluations work in:
/// allocated once, here, and given back when this is dropped.
pub struct Proof {
    algorithm: Argon2<'static>,
    blocks: Vec<Block>,
    input: [u8; INPUT],
    work: Work,
}

impl Proof {
    /// The challenge `nonce` at the price `work`.
    ///
    /// # Errors
    /// `Invalid` for a nonce that is not of the issued shape or a price
    /// above the ceilings: nothing is allocated and nothing computed.
    pub fn new(nonce: &str, work: Work) -> Result<Proof, CryptoError> {
        let shaped = nonce.len() == NONCE_LEN
            && nonce
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        if !shaped || !work.acceptable() {
            return Err(CryptoError::Invalid);
        }
        let parameters = Params::new(work.memory, work.passes, 1, Some(OUTPUT))
            .map_err(|_| CryptoError::Invalid)?;
        let blocks = vec![Block::default(); parameters.block_count()];
        let mut input = [0; INPUT];
        input[..NONCE_LEN].copy_from_slice(nonce.as_bytes());
        Ok(Proof {
            algorithm: Argon2::new(Algorithm::Argon2id, Version::V0x13, parameters),
            blocks,
            input,
            work,
        })
    }

    /// The price this was made with.
    #[must_use]
    pub fn work(&self) -> Work {
        self.work
    }

    /// The Argon2id output for `counter` in `part`: one evaluation.
    ///
    /// # Errors
    /// `Invalid` for a part the challenge does not have.
    pub fn output(&mut self, part: u8, counter: u64) -> Result<[u8; OUTPUT], CryptoError> {
        if part >= self.work.parts {
            return Err(CryptoError::Invalid);
        }
        self.input[NONCE_LEN] = part;
        self.input[NONCE_LEN + 1..].copy_from_slice(&counter.to_be_bytes());
        let mut output = [0; OUTPUT];
        self.algorithm
            .hash_password_into_with_memory(&self.input, LABEL, &mut output, &mut self.blocks[..])
            .map_err(|_| CryptoError::Invalid)?;
        Ok(output)
    }

    /// Whether `counter` solves `part`: the one judgement both sides share.
    /// A part the challenge does not have is never solved.
    pub fn holds(&mut self, part: u8, counter: u64) -> bool {
        let difficulty = u32::from(self.work.difficulty);
        self.output(part, counter)
            .is_ok_and(|output| zero_bits(&output) >= difficulty)
    }

    /// Looks for `part`'s counter among the `tries` counters from `from`
    /// on: the first that holds, or `None` when none of them does. The
    /// page's Worker calls this in turns, so it can say how far it is.
    ///
    /// # Errors
    /// `Invalid` for a part the challenge does not have.
    pub fn search(&mut self, part: u8, from: u64, tries: u32) -> Result<Option<u64>, CryptoError> {
        if part >= self.work.parts {
            return Err(CryptoError::Invalid);
        }
        for step in 0..u64::from(tries) {
            let Some(counter) = from.checked_add(step) else {
                break;
            };
            if self.holds(part, counter) {
                return Ok(Some(counter));
            }
        }
        Ok(None)
    }

    /// Whether `counters` solve the challenge: one for each part, in order.
    /// It stops at the first part that does not hold, so a wrong answer
    /// costs its checker one evaluation and a right one as many as there
    /// are parts.
    pub fn solved(&mut self, counters: &[u64]) -> bool {
        counters.len() == usize::from(self.work.parts)
            && (0..self.work.parts)
                .zip(counters)
                .all(|(part, counter)| self.holds(part, *counter))
    }
}

/// Whether `counters` solve the challenge `nonce` at the price `work`
/// ([`Proof::solved`]). Central's check: the memory is allocated for this
/// one call.
#[must_use]
pub fn solved(nonce: &str, work: Work, counters: &[u64]) -> bool {
    // The length first: no memory for an answer that cannot be right.
    counters.len() == usize::from(work.parts)
        && Proof::new(nonce, work).is_ok_and(|mut proof| proof.solved(counters))
}

/// Finds every part's smallest counter: for a client that registers
/// without a browser, and for tests.
///
/// # Errors
/// `Invalid` as [`Proof::new`].
pub fn solve(nonce: &str, work: Work) -> Result<Vec<u64>, CryptoError> {
    let mut proof = Proof::new(nonce, work)?;
    (0..work.parts)
        .map(|part| {
            let mut from = 0;
            loop {
                if let Some(counter) = proof.search(part, from, 1_024)? {
                    return Ok(counter);
                }
                from = from.checked_add(1_024).ok_or(CryptoError::Invalid)?;
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONCE: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
    const SMALL: Work = Work {
        memory: 64,
        passes: 1,
        parts: 4,
        difficulty: 3,
    };

    /// The pinned vector: this nonce at this price has these smallest
    /// counters and this output for part 0's. The page's Worker is held to
    /// the same counters in a browser (`crates/central-web/browser/e2e-site/account.spec.ts`), so
    /// neither build can drift from the other unseen. The output is also
    /// what OpenSSL 4.1.0's Argon2id gives for the same bytes
    /// (`openssl kdf -keylen 32 -kdfopt hexpass:… -kdfopt hexsalt:…
    /// -kdfopt iter:1 -kdfopt memcost:64 -kdfopt lanes:1 ARGON2ID`, with
    /// the nonce's 64 digits, `00` and `0000000000000007` as the password
    /// and [`LABEL`] as the salt), an implementation that shares no code
    /// with this one.
    #[test]
    fn the_pinned_vector_holds() {
        let counters = solve(NONCE, SMALL).unwrap();
        assert_eq!(counters, PINNED_COUNTERS);
        let mut proof = Proof::new(NONCE, SMALL).unwrap();
        let output = proof.output(0, counters[0]).unwrap();
        assert_eq!(output, PINNED_OUTPUT);
        assert!(zero_bits(&output) >= 3);
        assert!(solved(NONCE, SMALL, &counters));
    }

    const PINNED_COUNTERS: [u64; 4] = [7, 25, 2, 7];
    const PINNED_OUTPUT: [u8; 32] = [
        18, 98, 121, 8, 118, 64, 121, 55, 223, 9, 106, 88, 121, 69, 251, 59, 44, 88, 112, 19, 77,
        108, 35, 160, 148, 171, 58, 209, 85, 126, 113, 64,
    ];

    /// The same input through the Argon2 that `opaque-ke` brings, another
    /// release of the implementation: the encoding above is all that is
    /// ours, and the function under it is Argon2id as specified.
    #[test]
    fn it_is_argon2id_of_the_label_the_nonce_the_part_and_the_counter() {
        let mut proof = Proof::new(NONCE, SMALL).unwrap();
        let ours = proof.output(2, 0x0102_0304_0506_0708).unwrap();
        let mut input = NONCE.as_bytes().to_vec();
        input.push(2);
        input.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        let other = opaque_ke::argon2::Argon2::new(
            opaque_ke::argon2::Algorithm::Argon2id,
            opaque_ke::argon2::Version::V0x13,
            opaque_ke::argon2::Params::new(64, 1, 1, Some(32)).unwrap(),
        );
        let mut theirs = [0; 32];
        other
            .hash_password_into(&input, LABEL, &mut theirs)
            .unwrap();
        assert_eq!(ours, theirs);
    }

    #[test]
    fn every_part_must_hold_and_in_its_own_place() {
        let counters = solve(NONCE, SMALL).unwrap();
        let mut proof = Proof::new(NONCE, SMALL).unwrap();
        assert!(proof.solved(&counters));
        // too few, too many
        assert!(!proof.solved(&counters[..3]));
        let mut more = counters.clone();
        more.push(0);
        assert!(!proof.solved(&more));
        // each part's counter is the smallest that holds
        for (part, counter) in (0..SMALL.parts).zip(&counters) {
            assert!(
                (0..*counter).all(|below| !proof.holds(part, below)),
                "{part}"
            );
        }
        // a part the challenge does not have
        assert!(!proof.holds(SMALL.parts, 0));
        assert_eq!(proof.search(SMALL.parts, 0, 1), Err(CryptoError::Invalid));
        // another nonce's answer
        let other = NONCE.replace('0', "f");
        assert!(!solved(&other, SMALL, &counters));
    }

    #[test]
    fn a_solution_for_a_lower_price_does_not_hold_at_the_price_asked() {
        // Found at three bits, checked at twelve: 2^-9 a part by chance.
        let easy = solve(NONCE, SMALL).unwrap();
        let asked = Work {
            difficulty: 12,
            ..SMALL
        };
        assert!(!solved(NONCE, asked, &easy));
        // other memory, other passes: another function altogether
        for other in [
            Work {
                memory: 72,
                ..SMALL
            },
            Work { passes: 2, ..SMALL },
        ] {
            let mut proof = Proof::new(NONCE, other).unwrap();
            let mut plain = Proof::new(NONCE, SMALL).unwrap();
            assert_ne!(proof.output(0, 0).unwrap(), plain.output(0, 0).unwrap());
        }
    }

    #[test]
    fn a_search_in_turns_finds_what_one_search_finds() {
        let counters = solve(NONCE, SMALL).unwrap();
        let mut proof = Proof::new(NONCE, SMALL).unwrap();
        for (part, counter) in (0..SMALL.parts).zip(&counters) {
            let mut from = 0;
            let found = loop {
                if let Some(found) = proof.search(part, from, 2).unwrap() {
                    break found;
                }
                from += 2;
            };
            assert_eq!(found, *counter);
        }
        assert_eq!(proof.search(0, u64::MAX, 4), Ok(None));
    }

    #[test]
    fn a_price_above_the_ceilings_or_a_nonce_of_another_shape_is_refused_before_any_work() {
        let refused =
            |work: Work| Proof::new(NONCE, work).is_err() && !solved(NONCE, work, &[0; 4]);
        assert!(refused(Work {
            memory: MIN_MEMORY - 1,
            ..SMALL
        }));
        assert!(refused(Work {
            memory: MAX_MEMORY + 1,
            ..SMALL
        }));
        assert!(refused(Work { passes: 0, ..SMALL }));
        assert!(refused(Work {
            passes: MAX_PASSES + 1,
            ..SMALL
        }));
        assert!(refused(Work { parts: 0, ..SMALL }));
        assert!(
            Proof::new(
                NONCE,
                Work {
                    parts: MAX_PARTS + 1,
                    ..SMALL
                }
            )
            .is_err()
        );
        assert!(refused(Work {
            difficulty: MAX_DIFFICULTY + 1,
            ..SMALL
        }));
        for nonce in [
            "",
            "n",
            &NONCE[1..],
            &NONCE.to_uppercase(),
            &format!("{NONCE}0"),
        ] {
            assert!(Proof::new(nonce, SMALL).is_err(), "{nonce}");
            assert!(!solved(nonce, SMALL, &[0; 4]));
        }
        // the ceilings themselves are a price
        assert!(
            Work {
                memory: MAX_MEMORY,
                passes: MAX_PASSES,
                parts: MAX_PARTS,
                difficulty: MAX_DIFFICULTY
            }
            .acceptable()
        );
    }
}
