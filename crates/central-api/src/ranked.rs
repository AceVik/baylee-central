//! Shared competitive ladder protocol. Only Central assigns matches and publishes scores.
use serde::{Deserialize, Serialize};

/// Wire representation of a UUID.
pub type Id = [u8; 16];

/// Independent one-on-one ladders.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Format {
    /// Standard constructed.
    Standard,
    /// Baylee Highlander.
    Highlander,
    /// Two-seat Commander.
    Commander,
}

impl Format {
    /// Formats in their lobby display order.
    pub const ALL: [Self; 3] = [Self::Standard, Self::Highlander, Self::Commander];

    /// Stable protocol, catalogue and storage key.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Highlander => "highlander",
            Self::Commander => "commander",
        }
    }
}

impl std::str::FromStr for Format {
    type Err = crate::Error;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|format| format.as_str() == value)
            .ok_or(crate::Error::Rejected)
    }
}

/// Subject rated by a ladder.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Track {
    /// Player skill.
    Player,
    /// One player's deck version.
    Deck,
}

impl Track {
    /// Stable protocol and storage key.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Player => "player",
            Self::Deck => "deck",
        }
    }
}

/// Leagues in ascending order.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum League {
    /// Still completing the ten placement games for this format and track.
    Unranked,
    /// Paper league.
    Paper,
    /// Wood league.
    Wood,
    /// Iron league.
    Iron,
    /// Bronze league.
    Bronze,
    /// Silver league.
    Silver,
    /// Gold league.
    Gold,
    /// Platinum league.
    Platinum,
    /// Diamond league.
    Diamond,
    /// Master league.
    Master,
    /// Grandmaster league.
    Grandmaster,
    /// Champion league.
    Champion,
}

/// Engine-confirmed result in assignment seat order.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Outcome {
    /// Seat zero won.
    FirstWins,
    /// Seat one won.
    SecondWins,
    /// Neither seat won.
    Draw,
}

/// Season metadata.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Season {
    /// Season ID.
    pub id: Id,
    /// Public season name.
    pub name: String,
    /// Inclusive start, Unix seconds.
    pub starts: i64,
    /// Exclusive end, Unix seconds, when scheduled.
    pub ends: Option<i64>,
}

/// Published league, division and optional podium place.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rank {
    /// League title.
    pub league: League,
    /// Roman division, I = 1.
    pub division: Option<u8>,
    /// Published Champion place from 1 to 100.
    pub champion_place: Option<u8>,
}

/// Aggregate confirmed results across seasons.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    /// Completed games, including draws.
    pub games: u32,
    /// Victories.
    pub wins: u32,
    /// Defeats.
    pub losses: u32,
    /// Drawn games.
    pub draws: u32,
}

/// One format and track in a season.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Standing {
    /// Career total as of this season.
    pub career: Record,
    /// Completed placement games for this player or exact deck pool, capped at ten.
    pub placement_games: u8,
    /// Competitive format.
    pub format: Format,
    /// Player or deck ladder.
    pub track: Track,
    /// Opaque public deck-version ID on the deck ladder.
    pub deck: Option<[u8; 32]>,
    /// Rating in Elo points.
    pub elo: i32,
    /// Number of games in the season.
    pub games: u32,
    /// Number of wins in the season.
    pub wins: u32,
    /// Number of losses in the season.
    pub losses: u32,
    /// Number of draws in the season.
    pub draws: u32,
    /// Published competitive rank.
    pub rank: Rank,
}

/// Public ladder entry.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Row {
    /// Public account ID.
    pub account: Id,
    /// Public display name if set.
    pub name: Option<String>,
    /// Public discriminator if set.
    pub tag: Option<u32>,
    /// Standing within the selected ladder.
    pub standing: Standing,
}

/// Exclusive keyset cursor, stable for ties in score.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cursor {
    /// Rating in Elo points.
    pub elo: i32,
    /// Public account ID.
    pub account: Id,
    /// Opaque public deck-version ID on the deck ladder.
    pub deck: Option<[u8; 32]>,
}

/// Competitive distinction.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum AwardKind {
    /// Highest league reached.
    Peak,
    /// Final standing of a completed season.
    SeasonFinish,
}

/// A distinction earned on a published ladder.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Award {
    /// Reason for the distinction.
    pub kind: AwardKind,
    /// Competitive format.
    pub format: Format,
    /// Player or deck ladder.
    pub track: Track,
    /// Season in which the rank was achieved.
    pub season: Id,
    /// Published competitive rank.
    pub rank: Rank,
}

/// Public identity used for profile lookup.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ProfileTarget {
    /// Stable public account ID.
    Account(Id),
    /// Full public handle including discriminator.
    Handle(String),
}

/// One distinct card in the combined deck and sideboard.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PoolEntry {
    /// Domain-separated card identity digest.
    pub card: [u8; 32],
    /// Combined number of copies.
    pub copies: u16,
}

/// Check the bounded canonical multiset shared by engine attestations and owner previews.
pub fn valid_pool(entries: &[PoolEntry]) -> bool {
    !entries.is_empty()
        && entries.len() <= 250
        && entries.windows(2).all(|pair| pair[0].card < pair[1].card)
        && entries.iter().all(|row| row.copies > 0)
        && entries.iter().map(|row| u32::from(row.copies)).sum::<u32>() <= 250
}

/// Whether a deck projection restores play history or inherits a starting score.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum PreviewSource {
    /// This account has never played a deck in the selected format.
    New,
    /// This exact composition has never played; a stable reference applies.
    Inherited,
    /// Resume the exact composition's confirmed Elo and placement games.
    Recorded,
}

/// Private, read-only estimate for the selected pool as of one database snapshot.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeckPreview {
    /// Competitive format.
    pub format: Format,
    /// Origin of this version's rating.
    pub source: PreviewSource,
    /// Current or inherited deck rating before any further result.
    pub elo: i32,
    /// Points deducted from the selected stable reference for these edits.
    pub edit_loss: i32,
    /// Current published rank of this composition.
    pub rank: Rank,
    /// Confirmed placement progress, capped at ten.
    pub placement_games: u8,
    /// Unmatched copies between the proposed composition and the closest reference.
    pub changed_copies: u16,
    /// Number of copies in the larger pool for that comparison.
    pub compared_copies: u16,
}

/// One account's private seat assignment.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assignment {
    /// Central's match ID.
    pub match_id: Id,
    /// Verified destination gateway.
    pub host_id: Id,
    /// Canonical origin for the game connection.
    pub origin: String,
    /// Competitive format.
    pub format: Format,
    /// Public account ID.
    pub account: Id,
    /// Zero-based seat index.
    pub seat: u8,
    /// Commitment to the frozen playable deck.
    pub deck: [u8; 32],
    /// Bound game session after both seats have claimed.
    pub game: Option<Id>,
}

/// Private reservation available only to the selected game host.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostAssignment {
    /// Central's match ID.
    pub match_id: Id,
    /// Competitive format.
    pub format: Format,
    /// Distinct authenticated accounts in seat order.
    pub accounts: [Id; 2],
    /// Deck commitments in seat order.
    pub decks: [[u8; 32]; 2],
    /// Bound game session after both seats have claimed.
    pub game: Option<Id>,
}

/// Settlement of the played deck version, delivered with its owner's result.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeckResult {
    /// Rank of this exact composition before play.
    pub before: Rank,
    /// Rank of this exact composition after play.
    pub after: Rank,
    /// New deck rating.
    pub elo: i32,
    /// Change due to this game.
    pub delta: i32,
    /// Completed deck placement games, capped at ten.
    pub placement_games: u8,
}

/// Status of this account's most recent queue request.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum QueueState {
    /// No live entry.
    Idle,
    /// Awaiting an eligible opponent and available host.
    Waiting {
        /// Stable queue request ID, reused on retries.
        key: Id,
        /// Competitive format.
        format: Format,
        /// Admission time, Unix seconds.
        queued_at: i64,
    },
    /// A verified host reserved both seats.
    Assigned {
        /// Stable queue request ID, reused on retries.
        key: Id,
        /// Only this account's seat.
        assignment: Assignment,
    },
    /// Terminal player receipt for the completed match.
    Finished {
        /// Settlement for this account's deck in the same transaction.
        deck: DeckResult,
        /// Stable queue request ID, reused on retries.
        key: Id,
        /// Central's match ID.
        match_id: Id,
        /// Competitive format.
        format: Format,
        /// Engine result.
        outcome: Outcome,
        /// This player's seat.
        seat: u8,
        /// Rating in Elo points.
        elo: i32,
        /// Change in player Elo.
        delta: i32,
        /// Rank before this result.
        before: Rank,
        /// Rank after this result.
        after: Rank,
    },
}

/// Public directory reads and authenticated queue operations.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    /// Estimate the owner's explicitly selected competitive deck without registering it.
    PreviewDeck {
        /// Competitive format the list is intended for.
        format: Format,
        /// Sorted combined rules-identity commitments and copy counts.
        pool: Vec<PoolEntry>,
    },
    /// Enter one global format queue.
    Join {
        /// Stable queue request ID, reused on retries.
        key: Id,
        /// Competitive format.
        format: Format,
        /// Commitment to the frozen playable deck.
        deck: [u8; 32],
    },
    /// Cancel the identified queue request.
    Cancel {
        /// Stable queue request ID, reused on retries.
        key: Id,
    },
    /// Poll this account's queue and result.
    Status,
    /// Read one division of the public ladder.
    Board {
        /// Selected season. None selects the current season.
        season: Option<Id>,
        /// Competitive format.
        format: Format,
        /// Player or deck ladder.
        track: Track,
        /// Selected league.
        league: League,
        /// Selected division or the entire league.
        division: Option<u8>,
        /// Exclusive continuation.
        after: Option<Cursor>,
    },
    /// Read a public competitive profile.
    Profile {
        /// Public identity.
        target: ProfileTarget,
        /// Selected season. None selects the current season.
        season: Option<Id>,
    },
    /// List seasons newest first.
    Seasons {
        /// Exclusive continuation.
        before: Option<Id>,
    },
}

/// Ranked reply. Public views never contain queue or game credentials.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Response {
    /// Private valuation of the selected deck.
    DeckPreview {
        /// The result of the owner's read-only comparison.
        preview: DeckPreview,
    },
    /// Private queue or receipt.
    Queue {
        /// Current status.
        state: QueueState,
    },
    /// One bounded page.
    Board {
        /// Resolved season.
        season: Season,
        /// Ladder rows.
        entries: Vec<Row>,
        /// Exclusive continuation.
        next: Option<Cursor>,
    },
    /// Public seasonal profile.
    Profile {
        /// Resolved season.
        season: Season,
        /// Public account ID.
        account: Id,
        /// Public display name if set.
        name: Option<String>,
        /// Public discriminator if set.
        tag: Option<u32>,
        /// Competitive standings.
        formats: Vec<Standing>,
        /// Earned distinctions.
        awards: Vec<Award>,
    },
    /// Page of season metadata.
    Seasons {
        /// Seasons newest first.
        items: Vec<Season>,
        /// Exclusive continuation.
        next: Option<Id>,
    },
}

/// Restricted operations for a verified host credential.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum HostRequest {
    /// Renew short-lived availability.
    Ready {
        /// Total ranked game slots, including occupied ones.
        capacity: u8,
    },
    /// Confirm a locally authenticated seat and its deck.
    Claim {
        /// Central's match ID.
        match_id: Id,
        /// Public account ID.
        account: Id,
        /// Commitment to the frozen playable deck.
        deck: [u8; 32],
    },
    /// Bind both legal decks and one engine session.
    Start {
        /// Central's match ID.
        match_id: Id,
        /// UUID of the host's engine session.
        game: Id,
        /// Sorted combined pools in seat order.
        pools: [Vec<PoolEntry>; 2],
    },
    /// Deliver one authoritative terminal result.
    Finish {
        /// Central's match ID.
        match_id: Id,
        /// UUID of the host's engine session.
        game: Id,
        /// Engine result.
        outcome: Outcome,
    },
    /// Release an unplayable reservation.
    Void {
        /// Central's match ID.
        match_id: Id,
    },
}

/// Private host acknowledgement.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum HostResponse {
    /// Availability renewed.
    Ready,
    /// Authorized reservation.
    Assignment {
        /// Both authenticated seats on this host.
        assignment: HostAssignment,
    },
    /// Operation completed, including an identical retry.
    Done,
}

impl Request {
    /// Whether the request may be answered without an account session.
    #[must_use]
    pub const fn is_public(&self) -> bool {
        matches!(
            self,
            Self::Board { .. } | Self::Profile { .. } | Self::Seasons { .. }
        )
    }
}
