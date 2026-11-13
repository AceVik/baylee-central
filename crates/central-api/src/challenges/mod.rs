//! Global daily/weekly challenges. Public types never contain an execution package.
//! Only the separately authenticated official host uses [`HostRequest`].

mod calendar;
mod personal;
mod progress;
#[cfg(test)]
mod tests;
mod validation;

pub use calendar::{Kind, Period};
pub use personal::{ExportEntry, ExportTable, ExportedAttempt, TaskProgress};
pub use progress::{Badge, BadgeKind, Tier};
use serde::{Deserialize, Serialize};

/// Largest private engine package admitted by the challenge API.
pub const MAX_PACKAGE_BYTES: usize = 196_608;
/// Number of challenge or leaderboard rows in one page.
pub const PAGE_SIZE: usize = 30;

/// The two supported interface languages; card names retain their canonical spelling.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Words {
    /// English.
    pub en: String,
    /// German.
    pub de: String,
}

/// Public description fixed before a challenge opens.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Description {
    /// Calendar slot, in UTC.
    pub period: Period,
    /// Short theme title.
    pub title: Words,
    /// Objective and rules, without solution spoilers.
    pub instructions: Words,
    /// Canonical text list of the supplied player's deck, without its order.
    pub deck: String,
    /// The tested executor build, never a client-supplied version claim.
    pub executor: String,
    /// Positive turn target for the tactician badge; not a claimed optimum.
    pub target_turn: u32,
    /// Suggested active minutes, calibrated separately from bot execution time.
    pub expected_minutes: u16,
}

/// One publicly available challenge.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Challenge {
    /// Central's immutable database-generated `UUIDv7`.
    pub id: [u8; 16],
    /// Frozen description.
    pub description: Description,
    /// Digest binding the private package and its description.
    pub digest: [u8; 32],
    /// A withdrawn task cannot start or accept competitive results.
    pub withdrawn: bool,
}

/// What is known about an account's attempt, without a seat capability.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Attempt {
    /// Database-generated `UUIDv7`, independent of game randomness.
    pub id: [u8; 16],
    /// Challenge identity.
    pub challenge: [u8; 16],
    /// Current lifecycle state.
    pub state: AttemptState,
    /// One-based count of attempts that actually started, including this one if started.
    pub number: u32,
    /// Canonical official host origin. Never a caller-supplied redirect.
    pub host: String,
    /// Game identity once the host has claimed the attempt.
    pub game: Option<[u8; 16]>,
    /// At least one internal hint was seen before this attempt completed.
    pub assisted: bool,
    /// The attempt was admitted during the live calendar window.
    pub live: bool,
    /// Winning turn, when a verified win exists.
    pub turn: Option<u32>,
}

/// Durable attempt states; losing connectivity alone changes none of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptState {
    /// Central reserved the attempt, before any opening hand was revealed.
    Reserved,
    /// The official host claimed the attempt.
    Playing,
    /// Verified player victory.
    Won,
    /// Verified defeat or draw.
    Lost,
    /// The player deliberately gave up.
    Abandoned,
    /// The host confirmed a technical failure; no failed-attempt penalty.
    Void,
}

impl AttemptState {
    /// Whether the attempt still owns an active slot.
    #[must_use]
    pub const fn active(self) -> bool {
        matches!(self, Self::Reserved | Self::Playing)
    }

    /// Stable storage spelling.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Reserved => "reserved",
            Self::Playing => "playing",
            Self::Won => "won",
            Self::Lost => "lost",
            Self::Abandoned => "abandoned",
            Self::Void => "void",
        }
    }
}

/// Independent rankings: no opaque weighted combination of seconds and turns.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Board {
    /// Failures before the first unassisted live victory, then its turn.
    FirstWin,
    /// Fewest game turns in an unassisted live victory.
    BestTurns,
    /// Least measured human time in a continuous unassisted live victory.
    Sprint,
}

/// Public leaderboard row. Account IDs and private attempt IDs stay server-side.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Standing {
    /// Equal scores share a rank.
    pub rank: u32,
    /// The account's public display name.
    pub name: String,
    /// Public discriminator, where assigned.
    pub tag: Option<u32>,
    /// This is the authenticated reader's own row.
    pub own: bool,
    /// Failures before the first victory.
    pub failures: u32,
    /// Winning game turn.
    pub turn: u32,
    /// Authoritative human time; absent for interrupted victories.
    pub milliseconds: Option<u64>,
}

/// Public or account-authorized challenge operations, on native and browser surfaces.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    /// Current tasks and an archive page; future packages are never returned.
    Catalogue {
        /// Strictly older than this UTC start; omitted for the latest page.
        before: Option<i64>,
    },
    /// One task and one bounded page of its live leaderboard.
    Detail {
        /// Task identity.
        challenge: [u8; 16],
        /// Score to compare.
        board: Board,
        /// Zero-based row offset, bounded to 3,000.
        offset: u32,
    },
    /// Personal badge progress and active attempts.
    Progress,
    /// Read this account's attempts, assistance and accomplishments for one task.
    Personal {
        /// Public challenge identity.
        challenge: [u8; 16],
    },
    /// Export account-owned challenge metadata without changing the older account export.
    Export {
        /// Dataset to page through.
        table: ExportTable,
        /// Exclusive UUID cursor returned by the previous page.
        after: Option<[u8; 16]>,
    },
    /// Reserve once before the host reveals private information.
    Begin {
        /// Task identity.
        challenge: [u8; 16],
        /// Client-generated `UUIDv7` for idempotency, never authorization.
        request_id: [u8; 16],
        /// Explicit consent to appear by public name in this task's leaderboard.
        listed: bool,
    },
    /// Read a private attempt owned by the current account.
    Attempt {
        /// Attempt identity.
        attempt: [u8; 16],
    },
    /// End an owned attempt deliberately; the host must stop the associated game.
    Abandon {
        /// Attempt identity.
        attempt: [u8; 16],
    },
    /// Mark assistance account-wide before returning a spoiler-bearing hint.
    Hint {
        /// Task identity.
        challenge: [u8; 16],
        /// First or second hint, 1 or 2.
        level: u8,
    },
    /// Change leaderboard consent without starting another attempt.
    Visibility {
        /// Task identity.
        challenge: [u8; 16],
        /// Whether its completed scores may appear by public name.
        listed: bool,
    },
}

/// Challenge responses. Private execution material is in [`HostResponse`] only.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "answer", rename_all = "snake_case", deny_unknown_fields)]
pub enum Response {
    /// Public calendar and archive.
    Catalogue {
        /// Central's current Unix seconds.
        now: i64,
        /// At most [`PAGE_SIZE`] public tasks.
        challenges: Vec<Challenge>,
        /// Cursor for strictly older tasks.
        before: Option<i64>,
    },
    /// Public task and ranking.
    Detail {
        /// Central's current Unix seconds.
        now: i64,
        /// The task.
        challenge: Challenge,
        /// A bounded, stably ordered leaderboard page.
        standings: Vec<Standing>,
    },
    /// Account-owned state.
    Progress {
        /// All five permanent badge tracks.
        badges: Vec<Badge>,
        /// At most two active attempts, one per challenge kind.
        active: Vec<Attempt>,
    },
    /// Task-specific account progress; never another account's private state.
    Personal {
        /// Durable counters and the current personal best.
        progress: TaskProgress,
    },
    /// At most [`PAGE_SIZE`] account-owned metadata records.
    Export {
        /// Stable ascending identity order.
        entries: Vec<ExportEntry>,
        /// Next exclusive cursor; none after the last page.
        after: Option<[u8; 16]>,
    },
    /// An owned attempt, freshly read after the operation.
    Attempt {
        /// The attempt.
        attempt: Attempt,
    },
    /// Assistance has been durably marked before these words leave Central.
    Hint {
        /// Requested hint.
        text: Words,
    },
    /// A preference was changed; no new attempt was created.
    Updated,
}

/// Generator evidence admitted only from the official publishing authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Certificate {
    /// Version of the conservative template-specific classifier.
    pub validator: String,
    /// At least three freshly verified and semantically distinct winning traces.
    pub witnesses: Vec<Witness>,
}

/// A conservatively recognised cause of victory, derived from the executed game.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SolutionFamily {
    /// Combat damage establishes victory.
    Combat,
    /// A spell or noncombat ability establishes victory.
    DirectDamage,
    /// An explicit alternative victory condition establishes victory.
    AlternativeVictory,
}

/// One positive proof. Hashes bind records kept by the trusted generator.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Witness {
    /// Full verified record digest.
    pub record_digest: [u8; 32],
    /// Template-specific semantic signature, excluding mana order and identical copies.
    pub strategy_digest: [u8; 32],
    /// Family established by this specific trace.
    pub family: SolutionFamily,
}

/// Frozen private publication; never included in public catalogue responses.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Publication {
    /// Public description.
    pub description: Description,
    /// SHA-256 over the canonical publication content, checked by the publisher/host.
    pub digest: [u8; 32],
    /// Opaque engine package, bounded by [`MAX_PACKAGE_BYTES`].
    pub package: Vec<u8>,
    /// Positive evidence for several solutions.
    pub certificate: Certificate,
    /// Two graduated hints, unavailable before assistance is recorded.
    pub hints: [Words; 2],
}

/// Official-host operations. Registered gateway authority is insufficient.
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum HostRequest {
    /// List occupied slots in a bounded future window, without engine packages.
    Inventory {
        /// Inclusive UTC Unix seconds.
        from: i64,
        /// Exclusive UTC Unix seconds, at most 32 days later.
        until: i64,
    },
    /// Insert a certified immutable slot, idempotently by period and digest.
    Publish {
        /// Verified generator output.
        publication: Box<Publication>,
    },
    /// Claim an already-reserved attempt for its verified Central account.
    Claim {
        /// Attempt identity.
        attempt: [u8; 16],
        /// Account from the gateway's verified Central session, never client JSON.
        account: [u8; 16],
        /// Gateway-generated `UUIDv7`; existing claims retain their game identity.
        game: [u8; 16],
        /// Build the controlled host will actually execute.
        executor: String,
    },
    /// Re-read host-owned state, including cancellation, without private content.
    Status {
        /// Attempt identity.
        attempt: [u8; 16],
    },
    /// Record an engine result once. Exact retries succeed; conflicting results fail.
    Finish {
        /// Host's verified result.
        result: Completion,
    },
    /// Withdraw a broken task without deleting earned completion progress.
    Withdraw {
        /// Task identity.
        challenge: [u8; 16],
    },
}

/// Trusted engine outcome, bound to a claimed attempt and game.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Completion {
    /// Attempt identity.
    pub attempt: [u8; 16],
    /// Claimed game identity.
    pub game: [u8; 16],
    /// Won, lost, abandoned or void; never a nonterminal state.
    pub state: AttemptState,
    /// Final game turn; zero only for a technical abort before play.
    pub turn: u32,
    /// Official host's Unix-second end time, retained across delivery retries.
    /// Central checks it against the claim and its clock; no player supplies it.
    pub finished_at: i64,
    /// Server-measured human time; absent after any pause/disconnect.
    pub milliseconds: Option<u64>,
    /// Digest of the engine's kept record for auditing.
    pub record_digest: [u8; 32],
}

/// Responses restricted to the official bridge.
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "answer", rename_all = "snake_case", deny_unknown_fields)]
pub enum HostResponse {
    /// Occupied slots, including future ones, at most 64.
    Inventory {
        /// Known tasks.
        challenges: Vec<Challenge>,
    },
    /// Published or withdrawn task.
    Challenge {
        /// Public metadata.
        challenge: Challenge,
    },
    /// Private execution package, only after an account-bound claim.
    Claimed {
        /// Claimed state.
        attempt: Attempt,
        /// The immutable private publication.
        publication: Box<Publication>,
    },
    /// Current or finalized attempt.
    Attempt {
        /// Attempt state.
        attempt: Attempt,
    },
}
