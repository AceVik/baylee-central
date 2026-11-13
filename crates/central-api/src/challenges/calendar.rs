//! Fixed UTC windows, independent of locale, DST and client clocks.

use crate::Error;
use serde::{Deserialize, Serialize};

/// Challenge cadence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// One task per UTC date.
    Daily,
    /// One task per UTC week, beginning Monday.
    Weekly,
}

impl Kind {
    /// Length of a whole period in seconds.
    #[must_use]
    pub const fn seconds(self) -> i64 {
        match self {
            Self::Daily => 86_400,
            Self::Weekly => 604_800,
        }
    }

    /// Database and wire spelling.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Daily => "daily",
            Self::Weekly => "weekly",
        }
    }
}

/// A half-open UTC calendar slot: start is included, end is excluded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Period {
    /// Cadence.
    pub kind: Kind,
    /// Inclusive Unix seconds, aligned to UTC midnight/Monday.
    pub starts_at: i64,
    /// Exclusive Unix seconds, exactly one cadence after the start.
    pub ends_at: i64,
}

impl Period {
    /// Find the UTC slot containing `now`.
    ///
    /// # Errors
    /// A timestamp whose slot cannot be represented as signed Unix seconds.
    pub fn containing(kind: Kind, now: i64) -> Result<Self, Error> {
        // 1970-01-01 was Thursday: adding three days aligns Monday with zero.
        let offset = if kind == Kind::Weekly { 259_200 } else { 0 };
        let shifted = now.checked_add(offset).ok_or(Error::Rejected)?;
        let starts_at = shifted
            .div_euclid(kind.seconds())
            .checked_mul(kind.seconds())
            .and_then(|value| value.checked_sub(offset))
            .ok_or(Error::Rejected)?;
        let ends_at = starts_at
            .checked_add(kind.seconds())
            .ok_or(Error::Rejected)?;
        Ok(Self {
            kind,
            starts_at,
            ends_at,
        })
    }

    /// Validate an externally decoded slot, including UTC alignment and length.
    ///
    /// # Errors
    /// Misaligned, overflowing or otherwise noncanonical slot.
    pub fn validate(self) -> Result<(), Error> {
        (Self::containing(self.kind, self.starts_at)? == self)
            .then_some(())
            .ok_or(Error::Rejected)
    }

    /// Whether the Central-clock timestamp is inside this slot.
    #[must_use]
    pub const fn live_at(self, now: i64) -> bool {
        now >= self.starts_at && now < self.ends_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_midnight_and_monday_are_half_open() {
        // Monday 2026-11-09 00:00 UTC, independently calculated Unix fixture.
        let monday = 1_794_182_400;
        let week = Period::containing(Kind::Weekly, monday).unwrap();
        assert_eq!(week.starts_at, monday);
        assert_eq!(week.ends_at, monday + 7 * 86_400);
        assert!(!week.live_at(monday - 1));
        assert!(week.live_at(monday));
        assert!(week.live_at(week.ends_at - 1));
        assert!(!week.live_at(week.ends_at));
        assert_eq!(
            Period::containing(Kind::Weekly, monday - 1)
                .unwrap()
                .ends_at,
            monday
        );
        for day in 0..7 {
            assert_eq!(
                Period::containing(Kind::Weekly, monday + day * 86_400).unwrap(),
                week
            );
            let daily = Period::containing(Kind::Daily, monday + day * 86_400 + 43_200).unwrap();
            assert_eq!(daily.starts_at, monday + day * 86_400);
            assert_eq!(daily.ends_at - daily.starts_at, 86_400);
        }
    }

    #[test]
    fn arbitrary_windows_and_overflow_are_not_a_calendar() {
        let mut period = Period::containing(Kind::Daily, 1_794_182_400).unwrap();
        period.ends_at += 1;
        assert!(period.validate().is_err());
        period.starts_at += 1;
        assert!(period.validate().is_err());
        assert!(Period::containing(Kind::Weekly, i64::MAX).is_err());
        assert!(Period::containing(Kind::Daily, i64::MIN).is_err());
    }
}
