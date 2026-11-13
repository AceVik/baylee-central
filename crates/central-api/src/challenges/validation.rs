//! Bounds at both sides of the challenge transport; authorization is separate.

use super::{AttemptState, Completion, Description, MAX_PACKAGE_BYTES, Publication, Words};
use crate::Error;
use sha2::{Digest, Sha256};

fn text(value: &str, maximum: usize) -> bool {
    !value.trim().is_empty()
        && value.len() <= maximum
        && value
            .chars()
            .all(|c| crate::gateway::shown_char(c) || c == '\n')
}

impl Words {
    /// Bound both translations and reject control/directional spoofing characters.
    ///
    /// # Errors
    /// Missing, oversized or unsafe text.
    pub fn validate(&self, maximum: usize) -> Result<(), Error> {
        (text(&self.en, maximum) && text(&self.de, maximum))
            .then_some(())
            .ok_or(Error::Rejected)
    }
}

impl Description {
    /// Validate public metadata before storing or rendering it.
    ///
    /// # Errors
    /// Noncanonical period, invalid text, target or resource bounds.
    pub fn validate(&self) -> Result<(), Error> {
        self.period.validate()?;
        self.title.validate(100)?;
        self.instructions.validate(2_000)?;
        if !text(&self.deck, 16_000)
            || !text(&self.executor, 160)
            || self.executor.contains('\n')
            || !(1..=500).contains(&self.target_turn)
            || !(1..=180).contains(&self.expected_minutes)
        {
            return Err(Error::Rejected);
        }
        Ok(())
    }
}

impl Publication {
    /// Validate admission bounds and minimum positive evidence.
    /// The trusted generator, not Central, verifies Magic and strategic diversity.
    ///
    /// # Errors
    /// Invalid metadata/package size, missing distinct witnesses or families.
    pub fn validate(&self) -> Result<(), Error> {
        self.description.validate()?;
        for hint in &self.hints {
            hint.validate(2_000)?;
        }
        let certificate = &self.certificate;
        if !(3..=12).contains(&certificate.witnesses.len()) {
            return Err(Error::Rejected);
        }
        let mut witnesses: Vec<_> = certificate
            .witnesses
            .iter()
            .map(|proof| proof.strategy_digest)
            .collect();
        witnesses.sort_unstable();
        witnesses.dedup();
        let mut records: Vec<_> = certificate
            .witnesses
            .iter()
            .map(|proof| proof.record_digest)
            .collect();
        records.sort_unstable();
        records.dedup();
        let mut families: Vec<_> = certificate
            .witnesses
            .iter()
            .map(|proof| proof.family)
            .collect();
        families.sort_unstable();
        families.dedup();
        if self.package.is_empty()
            || self.package.len() > MAX_PACKAGE_BYTES
            || !text(&certificate.validator, 100)
            || witnesses.len() != certificate.witnesses.len()
            || witnesses.contains(&[0; 32])
            || records.len() != certificate.witnesses.len()
            || records.contains(&[0; 32])
            || families.len() < 2
        {
            return Err(Error::Rejected);
        }
        if self.digest != self.calculated_digest()? {
            return Err(Error::Rejected);
        }
        Ok(())
    }

    /// Hash all immutable publication content with SHA-256 and a fixed domain.
    /// This is content integrity, not a signature or substitute for host authentication.
    ///
    /// # Errors
    /// The bounded content could not be encoded.
    pub fn calculated_digest(&self) -> Result<[u8; 32], Error> {
        let content = crate::encode(&(
            &self.description,
            &self.package,
            &self.certificate,
            &self.hints,
        ))?;
        Ok(Sha256::new()
            .chain_update(b"baylee/challenge/publication/v1\0")
            .chain_update(content)
            .finalize()
            .into())
    }
}

impl Completion {
    /// Reject nonterminal or implausibly large results before database work.
    ///
    /// # Errors
    /// Invalid IDs, nonterminal state, missing turn/record or oversized clock.
    pub fn validate(&self) -> Result<(), Error> {
        let terminal = matches!(
            self.state,
            AttemptState::Won | AttemptState::Lost | AttemptState::Abandoned | AttemptState::Void
        );
        if !terminal
            || !uuid_v7(&self.attempt)
            || !uuid_v7(&self.game)
            || self.turn > 10_000
            || !(1..=253_402_300_799).contains(&self.finished_at)
            || (self.state != AttemptState::Void && self.turn == 0)
            || (self.state == AttemptState::Won && self.record_digest == [0; 32])
            || self
                .milliseconds
                .is_some_and(|value| value == 0 || value > 604_800_000)
        {
            return Err(Error::Rejected);
        }
        Ok(())
    }
}

/// Whether an externally supplied identifier has the database's `UUIDv7` shape.
pub(super) const fn uuid_v7(value: &[u8; 16]) -> bool {
    value[6] >> 4 == 7 && value[8] >> 6 == 2
}
