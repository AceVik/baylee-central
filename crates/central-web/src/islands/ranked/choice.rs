//! Which board the page shows, and how that stands in the address
//! (`?format=highlander&track=deck&who=ai&league=gold-2&season=<id>`): what
//! the visitor chose, nothing else. A value the page does not know is left
//! at its default, so an old or edited address still shows a board.

use central_api::ranked::ai::BoardPlayers;
use central_api::ranked::{Format, Id, League, Track};

use super::super::invite::query_param_from_search;
use crate::data::{id_from_hex, id_hex};

/// The leagues from the top down, as the page offers and searches them.
pub const TOP_DOWN: [League; 12] = [
    League::Champion,
    League::Grandmaster,
    League::Master,
    League::Diamond,
    League::Platinum,
    League::Gold,
    League::Silver,
    League::Bronze,
    League::Iron,
    League::Wood,
    League::Paper,
    League::Unranked,
];

/// How many divisions a league has (`docs/ranked.md` §"Permanent division
/// cosmetics"): Wood to Silver three, Gold to Diamond four, the rest none.
#[must_use]
pub const fn divisions(league: League) -> u8 {
    match league {
        League::Wood | League::Iron | League::Bronze | League::Silver => 3,
        League::Gold | League::Platinum | League::Diamond => 4,
        League::Unranked
        | League::Paper
        | League::Master
        | League::Grandmaster
        | League::Champion => 0,
    }
}

/// A league's name in the address.
#[must_use]
pub const fn league_key(league: League) -> &'static str {
    match league {
        League::Unranked => "unranked",
        League::Paper => "paper",
        League::Wood => "wood",
        League::Iron => "iron",
        League::Bronze => "bronze",
        League::Silver => "silver",
        League::Gold => "gold",
        League::Platinum => "platinum",
        League::Diamond => "diamond",
        League::Master => "master",
        League::Grandmaster => "grandmaster",
        League::Champion => "champion",
    }
}

/// The board a visitor looks at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Choice {
    /// The format's ladder.
    pub format: Format,
    /// Players or decks.
    pub track: Track,
    /// Everyone, people or AI players.
    pub players: BoardPlayers,
    /// The league; none leaves it to the page, which shows the highest
    /// league anybody stands in.
    pub league: Option<League>,
    /// One division of the league, or all of them.
    pub division: Option<u8>,
    /// A season by its id; none is the one running now.
    pub season: Option<Id>,
}

impl Default for Choice {
    /// Standard's players, everyone, the highest league with standings,
    /// the season running now.
    fn default() -> Self {
        Self {
            format: Format::Standard,
            track: Track::Player,
            players: BoardPlayers::Everyone,
            league: None,
            division: None,
            season: None,
        }
    }
}

/// What the page remembers its search for the highest league by: the
/// board without its league.
pub type Key = (Option<Id>, Format, Track, BoardPlayers);

impl Choice {
    /// The board without its league.
    #[must_use]
    pub const fn key(&self) -> Key {
        (self.season, self.format, self.track, self.players)
    }

    /// The league and division as one value of the league field: `""`
    /// for the highest league with standings, `gold` for all of Gold,
    /// `gold-2` for Gold II.
    #[must_use]
    pub fn league_value(&self) -> String {
        match (self.league, self.division) {
            (None, _) => String::new(),
            (Some(league), None) => league_key(league).to_owned(),
            (Some(league), Some(division)) => format!("{}-{division}", league_key(league)),
        }
    }

    /// This choice with the league field's value: an unknown value, or a
    /// division the league does not have, leaves the league to the page.
    #[must_use]
    pub fn with_league_value(self, value: &str) -> Self {
        let (league, division) = parse_league(value).unwrap_or((None, None));
        Self {
            league,
            division,
            ..self
        }
    }

    /// The query string for the address, with a leading `?`; empty when
    /// everything is at its default.
    #[must_use]
    pub fn to_search(&self) -> String {
        let mut pairs: Vec<String> = Vec::new();
        if self.format != Format::Standard {
            pairs.push(format!("format={}", self.format.as_str()));
        }
        if self.track != Track::Player {
            pairs.push(format!("track={}", self.track.as_str()));
        }
        match self.players {
            BoardPlayers::Everyone => {}
            BoardPlayers::People => pairs.push("who=people".to_owned()),
            BoardPlayers::Ai => pairs.push("who=ai".to_owned()),
        }
        if self.league.is_some() {
            pairs.push(format!("league={}", self.league_value()));
        }
        if let Some(season) = self.season {
            pairs.push(format!("season={}", id_hex(&season)));
        }
        if pairs.is_empty() {
            String::new()
        } else {
            format!("?{}", pairs.join("&"))
        }
    }

    /// The choice an address's query string names.
    #[must_use]
    pub fn from_search(search: &str) -> Self {
        let read = |name: &str| query_param_from_search(search, name);
        let format = read("format")
            .and_then(|value| value.parse::<Format>().ok())
            .unwrap_or(Format::Standard);
        let track = match read("track").as_deref() {
            Some("deck") => Track::Deck,
            _ => Track::Player,
        };
        let players = match read("who").as_deref() {
            Some("people") => BoardPlayers::People,
            Some("ai") => BoardPlayers::Ai,
            _ => BoardPlayers::Everyone,
        };
        let season = read("season").and_then(|value| id_from_hex(&value));
        Self {
            format,
            track,
            players,
            league: None,
            division: None,
            season,
        }
        .with_league_value(read("league").as_deref().unwrap_or(""))
    }
}

/// `gold`, `gold-2` or `champion` as a league and its division; `None` for
/// anything else, a division the league lacks included.
fn parse_league(value: &str) -> Option<(Option<League>, Option<u8>)> {
    let (name, division) = match value.split_once('-') {
        Some((name, number)) => {
            let number: u8 = number.parse().ok()?;
            (name, Some(number))
        }
        None => (value, None),
    };
    let league = TOP_DOWN
        .into_iter()
        .find(|league| league_key(*league) == name)?;
    if division.is_some_and(|d| d == 0 || d > divisions(league)) {
        return None;
    }
    Some((Some(league), division))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The address says only what differs from the default, and reads
    /// back as the same choice.
    #[test]
    fn a_choice_stands_in_the_address_and_reads_back() {
        assert_eq!(Choice::default().to_search(), "");
        let choice = Choice {
            format: Format::Highlander,
            track: Track::Deck,
            players: BoardPlayers::Ai,
            league: Some(League::Gold),
            division: Some(2),
            season: Some([0xab; 16]),
        };
        let search = choice.to_search();
        assert_eq!(
            search,
            format!(
                "?format=highlander&track=deck&who=ai&league=gold-2&season={}",
                "ab".repeat(16)
            )
        );
        assert_eq!(Choice::from_search(&search), choice);
        let whole = Choice {
            division: None,
            ..choice
        };
        assert_eq!(Choice::from_search(&whole.to_search()), whole);
    }

    /// Whatever an address holds, the page shows a board: an unknown
    /// value stays at its default, and a division a league lacks leaves
    /// the league to the page.
    #[test]
    fn an_unknown_value_falls_back_to_the_default() {
        let odd =
            Choice::from_search("?format=vintage&track=team&who=robots&league=gold-5&season=xyz");
        assert_eq!(odd, Choice::default());
        for value in [
            "champion-1",
            "paper-1",
            "unranked-2",
            "wood-0",
            "wood-x",
            "mud",
        ] {
            assert_eq!(
                Choice::default().with_league_value(value).league,
                None,
                "{value}"
            );
        }
        let wood = Choice::default().with_league_value("wood-3");
        assert_eq!((wood.league, wood.division), (Some(League::Wood), Some(3)));
        let upper = Choice::from_search(&format!("?season={}", "AB".repeat(16)));
        assert_eq!(upper.season, None, "one id, one address: lowercase only");
    }

    /// Every league has a key of its own that reads back, and the leagues
    /// go from the top down, divisions as Central counts them.
    #[test]
    fn the_leagues_run_from_the_top_down() {
        let key = |league| {
            central_api::ranked::rank_key(central_api::ranked::Rank {
                league,
                division: None,
                champion_place: None,
            })
        };
        for pair in TOP_DOWN.windows(2) {
            assert!(key(pair[0]) > key(pair[1]), "{pair:?}");
        }
        for league in TOP_DOWN {
            let value = Choice {
                league: Some(league),
                ..Choice::default()
            }
            .league_value();
            assert_eq!(
                Choice::default().with_league_value(&value).league,
                Some(league)
            );
            for division in 1..=divisions(league) {
                let choice = Choice::default().with_league_value(&format!("{value}-{division}"));
                assert_eq!(choice.division, Some(division));
            }
        }
        let with_divisions: Vec<_> = TOP_DOWN.into_iter().filter(|l| divisions(*l) > 0).collect();
        assert_eq!(with_divisions.len(), 7);
    }
}
