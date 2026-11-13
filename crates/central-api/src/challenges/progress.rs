//! Permanent, non-streak badge thresholds shared by website and native client.

use serde::{Deserialize, Serialize};

/// Independent permanent achievement tracks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BadgeKind {
    /// Distinct daily victories, including the archive.
    Daily,
    /// Distinct weekly victories, including the archive.
    Weekly,
    /// Distinct tasks won within the frozen turn target.
    Tactician,
    /// Distinct live first-attempt wins without internal hints.
    FirstTry,
    /// Nonconsecutive weeks with a weekly or three daily live completions.
    Regular,
}

/// Earned material tier; absence means the first threshold is still ahead.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    /// First accomplishment.
    Bronze,
    /// Established experience.
    Silver,
    /// Sustained experience.
    Gold,
    /// Long-term accomplishment.
    Platinum,
    /// Highest permanent tier.
    Obsidian,
}

/// One account's durable, deduplicated progress on one track.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Badge {
    /// Track.
    pub kind: BadgeKind,
    /// Distinct qualifying accomplishments, never repeated wins on one task.
    pub count: u32,
    /// Highest threshold reached.
    pub tier: Option<Tier>,
    /// Next cumulative target, or none at the highest tier.
    pub next: Option<u32>,
}

impl BadgeKind {
    /// All tracks in display order.
    pub const ALL: [Self; 5] = [
        Self::Daily,
        Self::Weekly,
        Self::Tactician,
        Self::FirstTry,
        Self::Regular,
    ];

    /// Cumulative thresholds in tier order.
    #[must_use]
    pub const fn thresholds(self) -> [u32; 5] {
        match self {
            Self::Daily => [1, 10, 30, 100, 365],
            Self::Weekly | Self::Regular => [1, 4, 12, 26, 52],
            Self::Tactician | Self::FirstTry => [1, 5, 20, 75, 200],
        }
    }

    /// Describe a validated server-side count; no clock or streak reset is involved.
    #[must_use]
    pub fn progress(self, count: u32) -> Badge {
        let tiers = [
            Tier::Bronze,
            Tier::Silver,
            Tier::Gold,
            Tier::Platinum,
            Tier::Obsidian,
        ];
        let thresholds = self.thresholds();
        let reached = thresholds
            .iter()
            .take_while(|&&target| count >= target)
            .count();
        Badge {
            kind: self,
            count,
            tier: reached.checked_sub(1).map(|index| tiers[index]),
            next: thresholds.get(reached).copied(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tier_begins_at_its_threshold_and_never_wraps() {
        let tiers = [
            Tier::Bronze,
            Tier::Silver,
            Tier::Gold,
            Tier::Platinum,
            Tier::Obsidian,
        ];
        for kind in BadgeKind::ALL {
            assert_eq!(kind.progress(0).tier, None);
            for (index, threshold) in kind.thresholds().into_iter().enumerate() {
                let before = kind.progress(threshold - 1);
                assert_eq!(before.next, Some(threshold));
                assert_ne!(before.tier, Some(tiers[index]));
                assert_eq!(kind.progress(threshold).tier, Some(tiers[index]));
            }
            assert_eq!(kind.progress(u32::MAX).tier, Some(Tier::Obsidian));
            assert_eq!(kind.progress(u32::MAX).next, None);
        }
    }
}
