//! Central tokens this process issued or verified, so that a session's later requests cost
//! no signature check (`docs/native-login.md` §"A bearer nobody issued").
//!
//! A made-up token never enters: only a token Central signed itself, as it signs it, or one
//! that verified. So a flood of made-up tokens, from however many places, spends the
//! signature checks (`limits.rs`) and takes nothing from a session whose token is here.
//!
//! What it may and may not decide:
//! - A token is found by the SHA-256 of its whole text. A token that differs in one byte is
//!   another token, and is verified.
//! - It answers only while the token's `exp` is in the future, with no tolerance; past that
//!   the token is verified, with the tolerance verification allows.
//! - It says only what verification said: whose session's capability the token carries.
//!   Whether that session is live is the store's to say on every request, so a session that
//!   ended is refused as before.
//! - The key set is read once, at the start, and never changes while the process runs (a
//!   key is rotated or dropped by a restart, `docs/operations.md` §"Rotating the signing
//!   key"). The memory goes with the process, so no token outlives the key that signed it.
//! - Nothing here is logged or printed: it holds capabilities, and has no `Debug`.

use central_api::Token;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

/// Tokens remembered at once: as many as the bearers vouched for (`limits.rs` `PROVEN`).
/// One more takes the place of the one used longest ago. About two megabytes.
pub(super) const VERIFIED: usize = 16_384;
const MINUTE: Duration = Duration::from_secs(60);

/// The SHA-256 of a token's whole text, which it is remembered by.
pub(super) fn key(text: &str) -> [u8; 32] {
    Sha256::digest(text.as_bytes()).into()
}

struct Entry {
    capability: Token,
    /// The token's `exp`, in seconds since the epoch.
    expires: i64,
    /// When it was last issued, verified or used here.
    used: Instant,
}

#[derive(Default)]
pub(super) struct Verified {
    known: BTreeMap<[u8; 32], Entry>,
    swept: Option<Instant>,
}

impl Verified {
    /// The capability of the token remembered by `key`, while its `exp` is after `time`;
    /// it counts as used now.
    pub(super) fn find(&mut self, key: &[u8; 32], time: i64, now: Instant) -> Option<Token> {
        let entry = self.known.get_mut(key)?;
        if time >= entry.expires {
            self.known.remove(key);
            return None;
        }
        entry.used = now;
        Some(Token::from_secret_bytes(entry.capability.to_secret_bytes()))
    }

    /// Remember a token Central issued or verified: its capability until `expires`.
    pub(super) fn remember(
        &mut self,
        key: [u8; 32],
        capability: &Token,
        expires: i64,
        time: i64,
        now: Instant,
    ) {
        if time >= expires {
            return;
        }
        let due = self
            .swept
            .is_none_or(|swept| now.saturating_duration_since(swept) >= MINUTE);
        if due {
            self.known.retain(|_, entry| time < entry.expires);
            self.swept = Some(now);
        }
        if self.known.len() >= VERIFIED && !self.known.contains_key(&key) {
            let oldest = self
                .known
                .iter()
                .min_by_key(|(_, entry)| entry.used)
                .map(|(key, _)| *key);
            if let Some(oldest) = oldest {
                self.known.remove(&oldest);
            }
        }
        self.known.insert(
            key,
            Entry {
                capability: Token::from_secret_bytes(capability.to_secret_bytes()),
                expires,
                used: now,
            },
        );
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.known.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TIME: i64 = 1_800_000_000;

    fn same(token: Option<Token>, capability: &Token) -> bool {
        token.is_some_and(|found| found.digest() == capability.digest())
    }

    #[test]
    fn a_remembered_token_answers_with_its_capability_until_its_exp() {
        let now = Instant::now();
        let mut verified = Verified::default();
        let capability = Token::generate().unwrap();
        let text = "header.payload.signature";
        verified.remember(key(text), &capability, TIME + 60, TIME, now);
        assert!(same(verified.find(&key(text), TIME, now), &capability));
        assert!(same(verified.find(&key(text), TIME + 59, now), &capability));
        // at its exp and after it, never: verification decides then
        assert!(verified.find(&key(text), TIME + 60, now).is_none());
        // and it is gone, not only refused
        assert!(verified.find(&key(text), TIME, now).is_none());
        assert_eq!(verified.len(), 0);
    }

    #[test]
    fn a_token_that_differs_in_one_byte_is_another_token() {
        let now = Instant::now();
        let mut verified = Verified::default();
        let capability = Token::generate().unwrap();
        verified.remember(key("aaaa.bbbb.cccc"), &capability, TIME + 60, TIME, now);
        for other in [
            "aaaa.bbbb.cccd",
            "aaaa.bbbb.ccc",
            "aaaa.bbbb.cccc ",
            "Aaaa.bbbb.cccc",
        ] {
            assert!(verified.find(&key(other), TIME, now).is_none(), "{other}");
        }
    }

    #[test]
    fn an_expired_token_is_not_remembered() {
        let now = Instant::now();
        let mut verified = Verified::default();
        let capability = Token::generate().unwrap();
        verified.remember(key("a.b.c"), &capability, TIME, TIME, now);
        assert_eq!(verified.len(), 0);
    }

    #[test]
    fn the_table_is_bounded_and_gives_up_the_token_used_longest_ago() {
        let start = Instant::now();
        let mut verified = Verified::default();
        let first = Token::generate().unwrap();
        let second = Token::generate().unwrap();
        verified.remember(key("first"), &first, TIME + 600, TIME, start);
        verified.remember(key("second"), &second, TIME + 600, TIME, start);
        for index in 2..VERIFIED {
            let at = start + Duration::from_millis(u64::try_from(index).unwrap());
            verified.remember(
                key(&format!("t{index}")),
                &Token::generate().unwrap(),
                TIME + 600,
                TIME,
                at,
            );
        }
        assert_eq!(verified.len(), VERIFIED);
        // the first is used again: the second is now the one used longest ago
        let later = start + Duration::from_secs(30);
        assert!(same(verified.find(&key("first"), TIME, later), &first));
        verified.remember(
            key("one more"),
            &Token::generate().unwrap(),
            TIME + 600,
            TIME,
            later,
        );
        assert_eq!(verified.len(), VERIFIED);
        assert!(same(verified.find(&key("first"), TIME, later), &first));
        assert!(verified.find(&key("second"), TIME, later).is_none());
    }

    #[test]
    fn expired_tokens_are_swept_once_a_minute() {
        let start = Instant::now();
        let mut verified = Verified::default();
        for index in 0..10 {
            verified.remember(
                key(&format!("t{index}")),
                &Token::generate().unwrap(),
                TIME + 10,
                TIME,
                start,
            );
        }
        assert_eq!(verified.len(), 10);
        let later = start + MINUTE;
        verified.remember(
            key("new"),
            &Token::generate().unwrap(),
            TIME + 600,
            TIME + 20,
            later,
        );
        assert_eq!(verified.len(), 1);
    }
}
