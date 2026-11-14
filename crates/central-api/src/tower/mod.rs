//! Weekly, manually played expeditions. Private packages have no player response type.

mod export;
mod progress;
#[cfg(test)]
mod tests;
mod validation;

use crate::challenges::{Attempt, Challenge, Period, Publication, Words};
pub use export::{Accomplishment, ExportEntry, ExportTable, RankedScore};
pub use progress::{Badge, BadgeKind, Phase, Progress};
use serde::{Deserialize, Serialize};

/// Five duels and four intervening choices.
pub const FLOORS: u8 = 5;
/// All triples of upgrade counts whose sum is at most four.
pub const NODES: usize = 35;

/// Independent, cumulative deck improvement tracks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Track {
    /// Stronger attacking creatures.
    Assault,
    /// More efficient interaction.
    Precision,
    /// Resilient board development.
    Resolve,
}

impl Track {
    /// Stable display and storage order.
    pub const ALL: [Self; 3] = [Self::Assault, Self::Precision, Self::Resolve];

    /// Index into the three upgrade counts.
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Assault => 0,
            Self::Precision => 1,
            Self::Resolve => 2,
        }
    }
}

/// A precise replacement offered between duels; canonical card names are untranslated.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Upgrade {
    /// Name of this improvement.
    pub title: Words,
    /// Short explanation of the strategic choice.
    pub summary: Words,
    /// Number of copies replaced.
    pub copies: u8,
    /// Card removed from the supplied deck.
    pub remove: String,
    /// Card added to the supplied deck.
    pub add: String,
}

/// Immutable public design of an expedition, without execution seeds or openings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Theme {
    /// A canonical UTC week.
    pub period: Period,
    /// Expedition title.
    pub title: Words,
    /// Floor names, with the sentinel third and guardian fifth.
    pub floors: [Words; 5],
    /// Four cumulative improvements for each of the three tracks.
    pub upgrades: [[Upgrade; 4]; 3],
}

/// A published expedition. Unfinished drafts are never returned here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Season {
    /// Database-generated identity.
    pub id: [u8; 16],
    /// Immutable public rules and rewards.
    pub theme: Box<Theme>,
    /// No new starts or qualifying results are allowed.
    pub withdrawn: bool,
}

/// Private publisher manifest. Each package is staged separately within the wire bound.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// Public design.
    pub theme: Theme,
    /// Package digests in [`coordinates`] order, exactly [`NODES`].
    pub nodes: Vec<[u8; 32]>,
}

/// Publisher-only operations, carried by the existing official bridge authority.
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum HostRequest {
    /// Read staged/published slots in a bounded 32-day window.
    Inventory {
        /// Inclusive Unix seconds.
        from: i64,
        /// Exclusive Unix seconds.
        until: i64,
    },
    /// Idempotently stage one certified encounter against the immutable manifest.
    Stage {
        /// Content binding for the whole expedition.
        manifest: Box<Manifest>,
        /// Index in [`coordinates`] order.
        node: u8,
        /// Certified private execution package for this node.
        publication: Box<Publication>,
    },
    /// Atomically make a complete, validated set visible.
    Activate {
        /// Staged season identity.
        season: [u8; 16],
    },
    /// Withdraw a season and all its encounters without deleting earned medals.
    Withdraw {
        /// Season identity.
        season: [u8; 16],
    },
}

/// Publisher status, without repeating the potentially large private packages.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stock {
    /// Season identity.
    pub id: [u8; 16],
    /// Calendar slot.
    pub period: Period,
    /// Node indexes already durably staged.
    pub staged: Vec<u8>,
    /// Complete and visible once its UTC week starts.
    pub published: bool,
    /// Administratively withdrawn.
    pub withdrawn: bool,
}

/// Separate comparable score dimensions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Board {
    /// Floors descending, losses ascending, winning turns ascending.
    Ascent,
    /// Measured active time of flawless, uninterrupted summit runs only.
    Sprint,
}

/// One public name and score; no account/run identities are disclosed.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Standing {
    /// Equal scores share this rank.
    pub rank: u32,
    /// Public display name.
    pub name: String,
    /// Optional public discriminator.
    pub tag: Option<u32>,
    /// Belongs to the authenticated reader.
    pub own: bool,
    /// Cleared floors, at most five.
    pub floors: u8,
    /// Defeats before this score was reached.
    pub losses: u8,
    /// Total turns of the winning duels.
    pub turns: u32,
    /// Total measured human time, absent after an interruption.
    pub milliseconds: Option<u64>,
}

/// Account-owned expedition, with exactly the current encounter's public information.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Run {
    /// Database-generated run identity.
    pub id: [u8; 16],
    /// Immutable season, including archived runs.
    pub season: Season,
    /// Optimistic concurrency revision for decisions.
    pub revision: u32,
    /// Durable progress and route.
    pub progress: Progress,
    /// Current floor's supplied deck and objective.
    pub encounter: Challenge,
    /// Active reservation or duel, resumable across gateways.
    pub attempt: Option<Attempt>,
    /// Account's current consent for this season.
    pub listed: bool,
}

/// Public reads and account-owned mutations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    /// Current season (or a named past season), scores and optional personal progress.
    Read {
        /// None selects the latest published season that has opened.
        season: Option<[u8; 16]>,
        /// Independent ranking.
        board: Board,
        /// Bounded to 3,000.
        offset: u32,
    },
    /// Read an owned run, including historical routes.
    Run {
        /// Owned identity.
        run: [u8; 16],
    },
    /// Restore the expedition context of an owned game after a client restart.
    Locate {
        /// Official game identity, matched through this account's reserved attempt.
        game: [u8; 16],
    },
    /// Start a new expedition with three lives; does not reveal an opening.
    Begin {
        /// Public season identity.
        season: [u8; 16],
        /// Stable `UUIDv7` across ambiguous retries.
        request_id: [u8; 16],
        /// Explicit public name consent for the season.
        listed: bool,
    },
    /// Reserve the next duel. The server selects the node, host and package.
    Start {
        /// Owned run.
        run: [u8; 16],
        /// Expected run revision.
        revision: u32,
        /// Stable `UUIDv7` across retries.
        request_id: [u8; 16],
    },
    /// Confirm one offered, persistent improvement.
    Choose {
        /// Owned run.
        run: [u8; 16],
        /// Expected run revision.
        revision: u32,
        /// One of the three offered tracks.
        track: Track,
        /// Stable `UUIDv7` across retries.
        request_id: [u8; 16],
    },
    /// Explicitly retire this expedition and cancel its active reservation/duel.
    Abandon {
        /// Owned run.
        run: [u8; 16],
        /// Expected revision, preventing stale cancellation.
        revision: u32,
    },
    /// Change publication consent for every run in this season.
    Visibility {
        /// Season identity.
        season: [u8; 16],
        /// Explicit consent; false also works before accepting changed terms.
        listed: bool,
    },
    /// Page through owned durable expedition records.
    Export {
        /// Dataset to export independently.
        table: ExportTable,
        /// Exclusive `UUIDv7` cursor; never another account's identity.
        after: Option<[u8; 16]>,
    },
}

/// Public metadata is distinct from private packages; personal state requires a session.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "answer", rename_all = "snake_case", deny_unknown_fields)]
pub enum Response {
    /// Landing page and ranking, also usable without signing in.
    Read {
        /// Central clock, in Unix seconds.
        now: i64,
        /// Current/past publication, or none before any publication exists.
        season: Option<Box<Season>>,
        /// At most thirty scores.
        standings: Vec<Standing>,
        /// Current account's latest run in this season, if signed in.
        run: Option<Box<Run>>,
        /// An active expedition from any week, so rollover never strands a reserved run.
        active: Option<Box<Run>>,
        /// All three permanent reward tracks when signed in.
        badges: Vec<Badge>,
    },
    /// Owned state after a read or mutation.
    Run {
        /// No seeds, proof records or seat capabilities.
        run: Box<Run>,
    },
    /// At most ten owned records, ascending by dataset identity.
    Export {
        /// Records, never another account's.
        entries: Vec<ExportEntry>,
        /// Next exclusive cursor, absent at the end.
        after: Option<[u8; 16]>,
    },
    /// Consent changed without starting a new run.
    Updated,
}

/// Canonical acyclic graph: counts sum to floor minus one; every choice increases one count.
#[must_use]
pub fn coordinates() -> Vec<[u8; 3]> {
    let mut nodes = Vec::with_capacity(NODES);
    for sum in 0..FLOORS {
        for assault in 0..=sum {
            for precision in 0..=sum - assault {
                nodes.push([assault, precision, sum - assault - precision]);
            }
        }
    }
    nodes
}
