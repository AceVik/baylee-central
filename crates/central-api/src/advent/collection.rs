//! The owner's permanent sleeves and playmats from all supported sources.
use super::{AssetMeta, Id, Kind, Reward, Text};
use crate::ranked::League;
use serde::{Deserialize, Serialize};

/// Presentation-only acquisition label. Possession is enforced independently.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Source {
    /// First earned in a calendar campaign.
    Advent {
        #[doc = "Campaign year."]
        year: u16,
        #[doc = "Door within its campaign."]
        door: u8,
    },
    /// Earned through a confirmed ranked achievement.
    Ranked {
        #[doc = "Reward's league."]
        league: League,
        #[doc = "I=1; undivided leagues have no numeral."]
        division: Option<u8>,
    },
    /// A future acquisition label; the awarded image stays usable.
    #[serde(other)]
    Unknown,
}

/// An individually authorized item in the native account's complete collection.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Item {
    /// Opaque asset identity.
    pub id: Id,
    /// Equipment slot.
    pub kind: Kind,
    /// Localized title.
    pub title: Text,
    /// Authenticated immutable media descriptor.
    pub asset: AssetMeta,
    /// First acquisition recorded by Central, Unix seconds.
    pub earned_at: i64,
    /// Player-facing origin.
    pub source: Source,
}

impl From<Reward> for Item {
    fn from(r: Reward) -> Self {
        Self {
            id: r.id,
            kind: r.kind,
            title: r.title,
            asset: r.asset,
            earned_at: r.earned_at,
            source: Source::Advent {
                year: r.year,
                door: r.door,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unrecognized_presentation_source_preserves_the_item() {
        let encoded = br#"{"type":"future_award","attribute":{"value":4}}"#;
        assert_eq!(crate::decode::<Source>(encoded), Ok(Source::Unknown));
    }
}
