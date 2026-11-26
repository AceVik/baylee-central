//! The session's own ranked data, in bounded pages for the account download
//! (`docs/ranked.md` §"Publication and storage"). Another account appears
//! only as the public account ID of a settled receipt's opponent: never its
//! deck, commitment, rating, seat or consent. No host evidence, journal,
//! seed, alias, token or credential has a field here.
use super::{Format, Id, PoolEntry, Rank, Record, Season, Track};
use crate::{Error, terms::TermsVersion};
use serde::{Deserialize, Serialize};

/// Most rows a website download takes of one dataset; past it the dataset
/// says `complete: false`, as the other exports do.
pub const DOWNLOAD_ROWS: usize = 7680;
/// Most badge peaks one season can hold under the ledger's key: per track
/// and format, ten leagues of five stored divisions, and Champion's five
/// divisions in four podium designs.
pub const MAX_SEASON_PEAKS: usize = 2 * 3 * (10 * 5 + 5 * 4);
/// Most operator decisions exported with one match (a primary, a standby
/// and every replacement both players approved). A match that holds more
/// fails the page instead of losing some silently.
pub const MAX_MATCH_OPERATORS: usize = 32;

/// The account's ranked datasets, each in its own stable key order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Table {
    /// Per season: player ratings, badge peaks and the Champion place.
    Seasons,
    /// Deck versions with their cards and per-season deck ratings.
    Decks,
    /// Settled results, one per match.
    Receipts,
    /// Matches the account was seated in, reduced to its own seat.
    Matches,
    /// Queue requests that Central still keeps.
    Tickets,
    /// Division sleeves and playmats unlocked for the whole account.
    Cosmetics,
}

impl Table {
    /// Every dataset, in the download's order.
    pub const ALL: [Self; 6] = [
        Self::Seasons,
        Self::Decks,
        Self::Receipts,
        Self::Matches,
        Self::Tickets,
        Self::Cosmetics,
    ];

    /// Stable name in the download.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Seasons => "seasons",
            Self::Decks => "decks",
            Self::Receipts => "receipts",
            Self::Matches => "matches",
            Self::Tickets => "tickets",
            Self::Cosmetics => "cosmetics",
        }
    }

    /// Rows in one page. A deck with 250 cards and a season with every
    /// badge are large, so a page of them stays well under the wire limit.
    #[must_use]
    pub const fn page_size(self) -> usize {
        match self {
            Self::Seasons => 8,
            Self::Decks => 6,
            Self::Receipts | Self::Matches | Self::Tickets | Self::Cosmetics => 30,
        }
    }
}

/// Exclusive, strictly ascending page key; each dataset uses one variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Key {
    /// A season, match or queue request, each a `UUIDv7`.
    Id(Id),
    /// A deck version's public identifier.
    Deck([u8; 32]),
    /// A placed ladder step: league index and division.
    Step {
        /// Stored league index, 0 (Paper) to 10 (Champion).
        league: u8,
        /// Stored division, 0 where a league has none.
        division: u8,
    },
}

/// The result from this account's side.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// This account won.
    Won,
    /// This account lost.
    Lost,
    /// Nobody won.
    Drawn,
}

/// One format's player rating as stored for a season.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rating {
    /// Competitive format.
    pub format: Format,
    /// Career totals through this season (each season's row carries them on).
    pub career: Record,
    /// Stored rank: Unranked before ten games, never the board's inactivity clamp.
    pub rank: Rank,
    /// Published Elo.
    pub elo: i32,
    /// Elo the queue pairs with.
    pub queue_elo: i32,
    /// The rating model's underlying score, in thousandths of a point.
    pub score_thousandths: i64,
    /// The rating model's placement estimate, in thousandths of a point.
    pub placement_thousandths: i64,
    /// The rating model's remaining upset allowance, in thousandths of a point.
    pub allowance_thousandths: i64,
    /// First settled game in this row, Unix seconds.
    pub first_at: i64,
    /// Last settled game in this row, Unix seconds.
    pub last_at: i64,
}

/// One earned badge design.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Peak {
    /// Player or deck ladder.
    pub track: Track,
    /// Competitive format.
    pub format: Format,
    /// League, division and, for Champion, the place.
    pub rank: Rank,
}

/// The season's published Champion place.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Champion {
    /// Place among the season's 100, from 1.
    pub place: u8,
    /// The player ladder it was earned on, if any.
    pub player_format: Option<Format>,
    /// The deck ladder it was earned on, if any.
    pub deck_format: Option<Format>,
    /// That deck version's public identifier.
    pub deck: Option<[u8; 32]>,
}

/// Everything per season that is not a deck's.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeasonEntry {
    /// The season.
    pub season: Season,
    /// Player ratings, one per format played.
    pub ratings: Vec<Rating>,
    /// Badge designs reached in this season.
    pub peaks: Vec<Peak>,
    /// The Champion place, if published.
    pub champion: Option<Champion>,
}

/// A deck version's rating in one season.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeckRating {
    /// Season ID.
    pub season: Id,
    /// Career totals of this version through this season.
    pub career: Record,
    /// Games of this version in this season.
    pub season_games: u32,
    /// Stored rank: Unranked before ten games.
    pub rank: Rank,
    /// Deck Elo.
    pub elo: i32,
    /// First settled game in this row, Unix seconds.
    pub first_at: i64,
    /// Last settled game in this row, Unix seconds.
    pub last_at: i64,
}

/// One rated composition of the account's.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Deck {
    /// Public deck-version identifier, as on the deck boards.
    pub deck: [u8; 32],
    /// Competitive format.
    pub format: Format,
    /// Card identity digests and combined copies, ascending.
    pub cards: Vec<PoolEntry>,
    /// Total copies.
    pub copies: u32,
    /// Current deck Elo.
    pub elo: i32,
    /// Elo a later edit inherits from, when this version is a reference.
    pub anchor: Option<i32>,
    /// The format's first reference of this account.
    pub initial: bool,
    /// Season of its first game.
    pub first_season: Id,
    /// Per-season ratings, oldest season first.
    pub ratings: Vec<DeckRating>,
}

/// One settled game from this account's side.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    /// Central's match ID.
    pub match_id: Id,
    /// Competitive format.
    pub format: Format,
    /// Settlement, Unix seconds.
    pub settled_at: i64,
    /// Result.
    pub verdict: Verdict,
    /// The opponent's public account ID, as on the ladder (nothing else of it).
    pub opponent: Id,
    /// Player Elo after the game.
    pub elo: i32,
    /// Change in player Elo.
    pub delta: i32,
    /// Player rank before.
    pub before: Rank,
    /// Player rank after.
    pub after: Rank,
    /// The played deck version's settlement.
    pub deck: super::DeckResult,
}

/// A match's lifecycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchState {
    /// Seats reserved on a host.
    Reserved,
    /// The engine game is bound.
    Playing,
    /// Settled.
    Finished,
    /// Ended without a rating.
    Void,
}

/// This account's decision about a named operator of the match.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Operator {
    /// Gateway ID.
    pub host_id: Id,
    /// Origin shown when deciding.
    pub origin: String,
    /// Ranked verification epoch shown.
    pub epoch: u64,
    /// Approved (true) or declined.
    pub allowed: bool,
    /// The operator's terms accepted with an approval.
    pub terms: Option<TermsVersion>,
}

/// A match, reduced to this account's seat.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Match {
    /// Central's match ID.
    pub match_id: Id,
    /// Season ID.
    pub season: Id,
    /// Competitive format.
    pub format: Format,
    /// This account's seat, 0 or 1.
    pub seat: u8,
    /// This account's deck commitment.
    pub deck: [u8; 32],
    /// This account's seat was claimed at the host.
    pub claimed: bool,
    /// Assigned host gateway.
    pub host_id: Id,
    /// Its origin.
    pub origin: String,
    /// Lifecycle state.
    pub state: MatchState,
    /// Engine game session, once bound.
    pub game: Option<Id>,
    /// Result from this side, once finished.
    pub verdict: Option<Verdict>,
    /// Reservation, Unix seconds.
    pub created_at: i64,
    /// End, Unix seconds.
    pub finished_at: Option<i64>,
    /// This account's operator decisions, by gateway ID.
    pub operators: Vec<Operator>,
}

/// A queue request's state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TicketState {
    /// In the queue.
    Waiting,
    /// Seated in a match.
    Assigned,
    /// Done.
    Closed,
}

/// One queue request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ticket {
    /// The client's request key.
    pub key: Id,
    /// Season ID.
    pub season: Id,
    /// Competitive format.
    pub format: Format,
    /// This account's deck commitment.
    pub deck: [u8; 32],
    /// Elo it queued with.
    pub queue_elo: i32,
    /// Asked for two-host protection.
    pub replicable: bool,
    /// State.
    pub state: TicketState,
    /// Its match, if one was reserved.
    pub match_id: Option<Id>,
    /// Admission, Unix seconds.
    pub created_at: i64,
    /// End of waiting, Unix seconds.
    pub expires_at: i64,
}

/// A ladder step's sleeve and playmat, unlocked for the account.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unlock {
    /// League and division.
    pub rank: Rank,
    /// Unix seconds.
    pub earned_at: i64,
}

/// A page holds entries of its requested dataset only.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Entry {
    /// [`Table::Seasons`].
    Season {
        /// The season's rows.
        season: SeasonEntry,
    },
    /// [`Table::Decks`].
    Deck {
        /// The version.
        deck: Deck,
    },
    /// [`Table::Receipts`].
    Receipt {
        /// The receipt.
        receipt: Receipt,
    },
    /// [`Table::Matches`].
    Match {
        /// The seat's view.
        seat: Match,
    },
    /// [`Table::Tickets`].
    Ticket {
        /// The request.
        ticket: Ticket,
    },
    /// [`Table::Cosmetics`].
    Cosmetic {
        /// The unlock.
        unlock: Unlock,
    },
}

impl Entry {
    /// Dataset and page key of this row.
    #[must_use]
    pub const fn key(&self) -> (Table, Key) {
        match self {
            Self::Season { season } => (Table::Seasons, Key::Id(season.season.id)),
            Self::Deck { deck } => (Table::Decks, Key::Deck(deck.deck)),
            Self::Receipt { receipt } => (Table::Receipts, Key::Id(receipt.match_id)),
            Self::Match { seat } => (Table::Matches, Key::Id(seat.match_id)),
            Self::Ticket { ticket } => (Table::Tickets, Key::Id(ticket.key)),
            Self::Cosmetic { unlock } => (
                Table::Cosmetics,
                Key::Step {
                    // The stored index: Paper is 0, Unranked has none (refused).
                    league: (unlock.rank.league as u8).saturating_sub(1),
                    division: match unlock.rank.division {
                        Some(division) => division,
                        None => 0,
                    },
                },
            ),
        }
    }

    fn valid(&self) -> bool {
        match self {
            Self::Season { season } => {
                season.ratings.len() <= Format::ALL.len() && season.peaks.len() <= MAX_SEASON_PEAKS
            }
            Self::Deck { deck } => {
                super::valid_pool(&deck.cards)
                    && deck.copies == deck.cards.iter().map(|c| u32::from(c.copies)).sum::<u32>()
                    && deck.ratings.iter().all(|rating| v7(&rating.season))
                    && v7(&deck.first_season)
            }
            Self::Match { seat } => seat.seat < 2 && seat.operators.len() <= MAX_MATCH_OPERATORS,
            Self::Cosmetic { unlock } => {
                unlock.rank.league != super::League::Unranked
                    && unlock.rank.champion_place.is_none()
            }
            Self::Receipt { .. } | Self::Ticket { .. } => true,
        }
    }
}

const fn v7(id: &Id) -> bool {
    id[6] >> 4 == 7 && id[8] >> 6 == 2
}

/// Whether `key` can continue `table`: its variant, and a `UUIDv7` for IDs.
#[must_use]
pub const fn valid_key(table: Table, key: &Key) -> bool {
    match (table, key) {
        (Table::Seasons | Table::Receipts | Table::Matches | Table::Tickets, Key::Id(id)) => v7(id),
        (Table::Decks, Key::Deck(_)) => true,
        (Table::Cosmetics, Key::Step { league, division }) => *league <= 10 && *division <= 4,
        _ => false,
    }
}

/// Validate an export page before following its cursor or keeping its rows.
///
/// # Errors
/// Another dataset's or an invalid row, keys out of order, an oversized page
/// or a cursor that is not the last key of a full page.
pub fn validate_page(
    table: Table,
    previous: Option<Key>,
    entries: &[Entry],
    next: Option<Key>,
) -> Result<(), Error> {
    if entries.len() > table.page_size() || previous.is_some_and(|key| !valid_key(table, &key)) {
        return Err(Error::Rejected);
    }
    let mut last = previous;
    for entry in entries {
        let (kind, key) = entry.key();
        if kind != table
            || !valid_key(table, &key)
            || !entry.valid()
            || last.is_some_and(|last| key <= last)
        {
            return Err(Error::Rejected);
        }
        last = Some(key);
    }
    if next.is_some() && (entries.len() != table.page_size() || next != last) {
        return Err(Error::Rejected);
    }
    Ok(())
}

#[cfg(test)]
#[path = "export_tests.rs"]
mod tests;
