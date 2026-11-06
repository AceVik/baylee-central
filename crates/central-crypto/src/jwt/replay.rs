//! The `jti`s a gateway has taken, so that a captured message is taken once.
use super::{Claims, JwtError, LEEWAY_SECONDS};
use std::collections::BTreeMap;

/// A bounded set of used `jti`s, each held until its token could no longer verify, so
/// that forgetting one never lets it verify twice. It refuses rather than forgets when
/// full of live entries.
pub struct ReplayCache {
    seen: BTreeMap<String, i64>,
    capacity: usize,
}

impl ReplayCache {
    /// Hold at most `capacity` entries.
    #[must_use]
    pub const fn new(capacity: usize) -> Self {
        Self {
            seen: BTreeMap::new(),
            capacity,
        }
    }

    /// Take verified claims once.
    ///
    /// # Errors
    /// [`JwtError::Replayed`] for a `jti` already taken and [`JwtError::Full`] when
    /// nothing can be forgotten yet; the sender may try again later.
    pub fn take<C: Claims>(&mut self, claims: &C, now: i64) -> Result<(), JwtError> {
        let jti = claims.jti();
        if self.seen.contains_key(jti) {
            return Err(JwtError::Replayed);
        }
        if self.seen.len() >= self.capacity {
            self.seen.retain(|_, until| *until > now);
        }
        if self.seen.len() >= self.capacity {
            return Err(JwtError::Full);
        }
        let until = claims.registered().exp.saturating_add(LEEWAY_SECONDS);
        self.seen.insert(jti.to_owned(), until);
        Ok(())
    }
}
