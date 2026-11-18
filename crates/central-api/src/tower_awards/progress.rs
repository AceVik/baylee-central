use super::{Fact, Outcome};
use crate::{Error, challenges::Tier};
use serde::{Deserialize, Serialize};

/// Account-owned permanent totals, updated only with a newly committed receipt.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Progress {
    /// Verified completed duels, regardless of outcome.
    pub duels: u64,
    /// Verified victories.
    pub victories: u64,
    /// Distinct expeditions completed with eight victories.
    pub clears: u64,
    /// Such expeditions completed without a defeat or rules draw.
    pub flawless: u64,
}

impl Progress {
    /// Reject corrupted projections, including arithmetic overflow.
    /// # Errors
    /// Counters cannot be explained by distinct valid accomplishment facts.
    pub fn validate(self) -> Result<(), Error> {
        if self.victories > self.duels
            || self.flawless > self.clears
            || self
                .clears
                .checked_mul(8)
                .is_none_or(|wins| wins > self.victories)
        {
            return Err(Error::Rejected);
        }
        Ok(())
    }

    /// Apply a newly accepted fact once; the caller must atomically deduplicate first.
    /// Must process a run's facts in ordinal order when constructing its projection.
    /// # Errors
    /// Invalid input or overflow leaves the original projection completely unchanged.
    pub fn apply(&mut self, fact: &Fact) -> Result<(), Error> {
        self.validate()?;
        fact.validate()?;
        let victory = fact.outcome == Outcome::Victory;
        let clear = victory && fact.wins == 8;
        let add = |value: u64, increment: u64| value.checked_add(increment).ok_or(Error::Busy);
        let next = Self {
            duels: add(self.duels, 1)?,
            victories: add(self.victories, u64::from(victory))?,
            clears: add(self.clears, u64::from(clear))?,
            flawless: add(self.flawless, u64::from(clear && fact.defeats == 0))?,
        };
        next.validate()?;
        *self = next;
        Ok(())
    }

    /// Distinct permanent tracks, in consistent website/native display order.
    #[must_use]
    pub fn badges(self) -> [Badge; 3] {
        [
            BadgeKind::Victories.progress(self.victories),
            BadgeKind::Ascents.progress(self.clears),
            BadgeKind::Unbroken.progress(self.flawless),
        ]
    }
}

/// Independent cosmetics for v2; legacy five-floor accomplishments stay separate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BadgeKind {
    /// Deduplicated duel victories.
    Victories,
    /// Completed eight-win expeditions.
    Ascents,
    /// Eight-win expeditions without spending a life.
    Unbroken,
}

/// One permanent reward track. No currency or competitive advantage is represented.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Badge {
    /// Track with its own presentation motif.
    pub kind: BadgeKind,
    /// Authoritative cumulative count; never a streak that resets.
    pub count: u64,
    /// Highest earned material, absent before the first accomplishment.
    pub tier: Option<Tier>,
    /// Next cumulative target, absent at the highest tier.
    pub next: Option<u64>,
}

impl BadgeKind {
    /// Cumulative bronze through obsidian targets.
    #[must_use]
    pub const fn thresholds(self) -> [u64; 5] {
        match self {
            Self::Victories => [1, 10, 50, 200, 1000],
            Self::Ascents => [1, 3, 10, 30, 100],
            Self::Unbroken => [1, 3, 7, 15, 30],
        }
    }

    /// Project a deduplicated permanent count for either UI, with no clock dependency.
    #[must_use]
    pub fn progress(self, count: u64) -> Badge {
        let targets = self.thresholds();
        let tiers = [
            Tier::Bronze,
            Tier::Silver,
            Tier::Gold,
            Tier::Platinum,
            Tier::Obsidian,
        ];
        let reached = targets
            .iter()
            .take_while(|&&target| count >= target)
            .count();
        Badge {
            kind: self,
            count,
            tier: reached.checked_sub(1).map(|index| tiers[index]),
            next: targets.get(reached).copied(),
        }
    }
}
