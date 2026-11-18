//! The donation figures Central publishes ([`crate::wire::Request::PublicFunding`]):
//! sums and counts in euro cents, never a single donation, its time or anything about
//! who gave. `docs/operations.md` §"Donations" says where each figure comes from.
use crate::Error;
use serde::{Deserialize, Serialize};

/// The most a month's figure may say, in euro cents: ten million euros. A figure
/// beyond it is no figure any platform could report and is refused.
pub const MOST_MONTH: u64 = 1_000_000_000;
/// The most supporters or donations a figure may count.
pub const MOST_COUNT: u64 = 1_000_000;
/// The most a sum of donations given once may say, in euro cents: a hundred million
/// euros.
pub const MOST_TOTAL: u64 = 10_000_000_000;

/// What one platform's supporters give regularly: the amount a month in euro cents
/// (another currency converted at the latest reference rate) and by how many.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Monthly {
    /// What is given a month, in euro cents.
    pub month: u64,
    /// How many people give it, as the platform counts them.
    pub supporters: u64,
}

/// The donations given once, added up: each in euro cents at the reference rate of
/// the day it came, refunds taken off.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Once {
    /// Their sum, in euro cents.
    pub total: u64,
    /// How many there were.
    pub count: u64,
    /// The day the count began, `YYYY-MM-DD`: the day Central's store began it,
    /// never the day of a donation.
    pub since: String,
}

/// The donation figures as Central knows them. A platform Central does not collect,
/// or has no figure from yet, is absent and never zero.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Funding {
    /// GitHub Sponsors: the platform's own monthly estimate.
    pub github: Option<Monthly>,
    /// Liberapay: what is received a week, as a month's amount.
    pub liberapay: Option<Monthly>,
    /// Ko-fi: the recurring payments of the last 31 days.
    pub kofi: Option<Monthly>,
    /// What was given once, on GitHub Sponsors and Ko-fi together.
    pub once: Option<Once>,
}

/// Whether `text` is a date of the shape `YYYY-MM-DD`, its month 1 to 12 and its day
/// 1 to 31.
#[must_use]
pub fn is_day(text: &str) -> bool {
    let bytes = text.as_bytes();
    let digits = |range: std::ops::Range<usize>| bytes[range].iter().all(u8::is_ascii_digit);
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !digits(0..4)
        || !digits(5..7)
        || !digits(8..10)
    {
        return false;
    }
    let part = |range: std::ops::Range<usize>| text[range].parse::<u8>().ok();
    part(5..7).is_some_and(|month| (1..=12).contains(&month))
        && part(8..10).is_some_and(|day| (1..=31).contains(&day))
}

impl Monthly {
    fn validate(self) -> Result<Self, Error> {
        if self.month > MOST_MONTH || self.supporters > MOST_COUNT {
            return Err(Error::Rejected);
        }
        Ok(self)
    }
}

impl Funding {
    /// Checks every figure against its bound and the day's shape, as a reader of the
    /// answer must: a figure out of bounds refuses the whole answer.
    ///
    /// # Errors
    /// [`Error::Rejected`] for a figure beyond its bound or a day of another shape.
    pub fn validate(&self) -> Result<(), Error> {
        for monthly in [self.github, self.liberapay, self.kofi]
            .into_iter()
            .flatten()
        {
            monthly.validate()?;
        }
        if let Some(once) = &self.once
            && (once.total > MOST_TOTAL || once.count > MOST_COUNT || !is_day(&once.since))
        {
            return Err(Error::Rejected);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_day_is_a_date_of_one_shape() {
        for day in ["2026-11-18", "2026-01-01", "2026-12-31"] {
            assert!(is_day(day), "{day}");
        }
        for text in [
            "",
            "2026-11-18T00:00:00Z",
            "2026-13-01",
            "2026-00-10",
            "2026-11-00",
            "2026-11-32",
            "20261118",
            "2026/11/18",
            "２０２６-11-18",
            "<b>2026</b>",
        ] {
            assert!(!is_day(text), "{text}");
        }
    }

    #[test]
    fn a_figure_out_of_bounds_refuses_the_answer() {
        let fine = Funding {
            github: Some(Monthly {
                month: 4_600,
                supporters: 2,
            }),
            liberapay: None,
            kofi: Some(Monthly {
                month: 0,
                supporters: 0,
            }),
            once: Some(Once {
                total: 23_500,
                count: 12,
                since: "2026-11-18".into(),
            }),
        };
        assert_eq!(fine.validate(), Ok(()));
        assert_eq!(Funding::default().validate(), Ok(()));
        let mut beyond = fine.clone();
        beyond.liberapay = Some(Monthly {
            month: MOST_MONTH + 1,
            supporters: 1,
        });
        assert_eq!(beyond.validate(), Err(Error::Rejected));
        let mut many = fine.clone();
        many.github = Some(Monthly {
            month: 1,
            supporters: MOST_COUNT + 1,
        });
        assert_eq!(many.validate(), Err(Error::Rejected));
        let mut dated = fine.clone();
        dated.once = Some(Once {
            total: 1,
            count: 1,
            since: "yesterday".into(),
        });
        assert_eq!(dated.validate(), Err(Error::Rejected));
        let mut summed = fine;
        summed.once = Some(Once {
            total: MOST_TOTAL + 1,
            count: 1,
            since: "2026-11-18".into(),
        });
        assert_eq!(summed.validate(), Err(Error::Rejected));
    }

    #[test]
    fn the_wire_shape_is_closed() {
        let text =
            r#"{"github":null,"liberapay":{"month":10010,"supporters":3},"kofi":null,"once":null}"#;
        let funding: Funding = serde_json::from_str(text).unwrap();
        assert_eq!(funding.liberapay.map(|m| m.month), Some(10_010));
        assert_eq!(serde_json::to_string(&funding).unwrap(), text);
        for text in [
            r#"{"github":null,"liberapay":null,"kofi":null,"once":null,"donors":[]}"#,
            r#"{"github":{"month":1,"supporters":1,"name":"x"},"liberapay":null,"kofi":null,"once":null}"#,
            r#"{"github":null,"liberapay":null,"kofi":null,"once":{"total":1,"count":1,"since":"2026-11-18","last":1}}"#,
        ] {
            assert!(serde_json::from_str::<Funding>(text).is_err(), "{text}");
        }
    }
}
