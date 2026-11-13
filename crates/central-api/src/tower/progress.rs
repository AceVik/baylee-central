//! Pure, bounded expedition transitions and permanent reward thresholds.

use super::{FLOORS, Track, coordinates};
use crate::{
    Error,
    challenges::{AttemptState, Tier},
};
use serde::{Deserialize, Serialize};

/// Durable expedition state; a disconnected transport is not a state transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// A floor can be reserved.
    Ready,
    /// A reservation or duel is active.
    Playing,
    /// A victory earned one deck improvement.
    Choosing,
    /// All five floors were cleared.
    Conquered,
    /// All three lives were spent.
    Exhausted,
    /// The player deliberately retired the expedition.
    Abandoned,
}

impl Phase {
    /// Whether this run still occupies the account's expedition slot.
    #[must_use]
    pub const fn active(self) -> bool {
        matches!(self, Self::Ready | Self::Playing | Self::Choosing)
    }
}

/// Server-owned progress. Clients receive it but cannot submit a replacement.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Progress {
    /// Current lifecycle phase.
    pub phase: Phase,
    /// Number of won duels, at most five.
    pub cleared: u8,
    /// Remaining lives, initially three.
    pub lives: u8,
    /// Chronological improvement choices, at most four.
    pub route: Vec<Track>,
    /// Sum of winning game turns.
    pub turns: u32,
    /// Sum of measured human time for victories; none after an interrupted victory.
    pub milliseconds: Option<u64>,
    /// Every played result in this run so far was admitted and finished in its live week.
    pub competitive: bool,
}

impl Progress {
    /// An unopened run reveals neither a shuffled deck nor a private seed.
    #[must_use]
    pub const fn new(competitive: bool) -> Self {
        Self {
            phase: Phase::Ready,
            cleared: 0,
            lives: 3,
            route: Vec::new(),
            turns: 0,
            milliseconds: Some(0),
            competitive,
        }
    }

    /// Accumulated upgrades, independent of the order in which they were chosen.
    #[must_use]
    pub fn counts(&self) -> [u8; 3] {
        let mut counts = [0_u8; 3];
        for track in self.route.iter().take(4) {
            counts[track.index()] += 1;
        }
        counts
    }

    /// The immutable encounter selected by the route.
    ///
    /// # Errors
    /// A malformed stored route is never used as an unchecked index.
    pub fn node(&self) -> Result<u8, Error> {
        self.validate()?;
        coordinates()
            .iter()
            .position(|counts| *counts == self.counts())
            .and_then(|index| u8::try_from(index).ok())
            .ok_or(Error::Rejected)
    }

    /// Check state shape after decoding durable metadata.
    ///
    /// # Errors
    /// Impossible route, phase, life count or score.
    pub fn validate(&self) -> Result<(), Error> {
        let choices = usize::from(self.cleared.min(FLOORS - 1));
        let expected = if self.phase == Phase::Choosing {
            choices.saturating_sub(1)
        } else {
            choices
        };
        if self.cleared > FLOORS
            || self.lives > 3
            || self.route.len() > 4
            || self.turns > 50_000
            || self.milliseconds.is_some_and(|ms| ms > 3_024_000_000)
            || (self.phase != Phase::Abandoned && self.route.len() != expected)
            || (self.phase.active() && (self.lives == 0 || self.cleared == FLOORS))
            || (self.phase == Phase::Choosing && self.cleared == 0)
            || (self.phase == Phase::Conquered && (self.cleared != FLOORS || self.lives == 0))
            || (self.phase == Phase::Exhausted && (self.lives != 0 || self.cleared == FLOORS))
        {
            return Err(Error::Rejected);
        }
        Ok(())
    }

    /// Reserve a floor without changing its deterministic encounter.
    ///
    /// # Errors
    /// Only a ready run can own a new reservation.
    pub fn start(&mut self) -> Result<(), Error> {
        self.validate()?;
        if self.phase != Phase::Ready {
            return Err(Error::Conflict);
        }
        self.phase = Phase::Playing;
        Ok(())
    }

    /// Commit one offered upgrade after a victory.
    ///
    /// # Errors
    /// No choice is available in another phase.
    pub fn choose(&mut self, track: Track) -> Result<(), Error> {
        self.validate()?;
        if self.phase != Phase::Choosing {
            return Err(Error::Conflict);
        }
        self.route.push(track);
        self.phase = Phase::Ready;
        Ok(())
    }

    /// Apply a newly accepted official result once; durable deduplication belongs to Central.
    ///
    /// # Errors
    /// Invalid phase, nonterminal outcome or out-of-bounds score.
    pub fn finish(
        &mut self,
        state: AttemptState,
        turn: u32,
        milliseconds: Option<u64>,
        live: bool,
    ) -> Result<(), Error> {
        self.validate()?;
        if self.phase != Phase::Playing {
            return Err(Error::Conflict);
        }
        if turn > 10_000 || milliseconds.is_some_and(|ms| ms == 0 || ms > 604_800_000) {
            return Err(Error::Rejected);
        }
        let mut next = self.clone();
        match state {
            AttemptState::Void => {
                next.phase = Phase::Ready;
            }
            AttemptState::Won if turn > 0 => {
                next.cleared += 1;
                next.turns += turn;
                next.milliseconds = next.milliseconds.zip(milliseconds).map(|(a, b)| a + b);
                next.competitive &= live;
                next.phase = if next.cleared == FLOORS {
                    Phase::Conquered
                } else {
                    Phase::Choosing
                };
            }
            AttemptState::Lost | AttemptState::Abandoned => {
                next.lives -= 1;
                next.competitive &= live;
                next.phase = if next.lives == 0 {
                    Phase::Exhausted
                } else {
                    Phase::Ready
                };
            }
            _ => return Err(Error::Rejected),
        }
        next.validate()?;
        *self = next;
        Ok(())
    }
}

/// Permanent tower accomplishments, independent from Daily/Weekly badges.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BadgeKind {
    /// Distinct season/floor victories, including practice.
    Pathfinder,
    /// Distinct seasons whose summit was conquered.
    Summit,
    /// Distinct seasons conquered without spending a life.
    Unbroken,
}

/// Deduplicated progress on a tower reward track.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Badge {
    /// Track identity, with its own visual motif.
    pub kind: BadgeKind,
    /// Distinct accomplishments.
    pub count: u32,
    /// Highest earned material.
    pub tier: Option<Tier>,
    /// Next cumulative target; no streak reset.
    pub next: Option<u32>,
}

impl BadgeKind {
    /// Display order.
    pub const ALL: [Self; 3] = [Self::Pathfinder, Self::Summit, Self::Unbroken];

    /// Cumulative permanent targets from bronze through obsidian.
    #[must_use]
    pub const fn thresholds(self) -> [u32; 5] {
        match self {
            Self::Pathfinder => [1, 15, 50, 130, 260],
            Self::Summit => [1, 4, 12, 26, 52],
            Self::Unbroken => [1, 3, 8, 20, 40],
        }
    }

    /// Project an authoritative deduplicated count for either interface.
    #[must_use]
    pub fn progress(self, count: u32) -> Badge {
        let tiers = [
            Tier::Bronze,
            Tier::Silver,
            Tier::Gold,
            Tier::Platinum,
            Tier::Obsidian,
        ];
        let targets = self.thresholds();
        let reached = targets
            .iter()
            .take_while(|&&target| count >= target)
            .count();
        Badge {
            kind: self,
            count,
            tier: reached.checked_sub(1).map(|i| tiers[i]),
            next: targets.get(reached).copied(),
        }
    }
}
