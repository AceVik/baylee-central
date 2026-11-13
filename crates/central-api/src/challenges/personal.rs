//! Account-owned task progress and paginated metadata export.
use super::AttemptState;
use serde::{Deserialize, Serialize};

/// Permanent task progress. Failed later retries never erase accomplishments.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskProgress {
    /// Task identity.
    pub challenge: [u8; 16],
    /// Attempts whose execution package was claimed by the official host.
    pub starts: u32,
    /// Lost and deliberately abandoned started attempts, excluding technical aborts.
    pub failures: u32,
    /// Highest hint revealed, zero to two.
    pub hint_level: u8,
    /// Current consent to appear on this task's public leaderboards.
    pub listed: bool,
    /// First competitive live victory, if any.
    pub first_win: Option<[u8; 16]>,
    /// First completion timestamp, in Unix seconds.
    pub completed_at: Option<i64>,
    /// UTC week credited to the regular-participation badge, if any.
    pub live_week: Option<i64>,
    /// Ever reached this task's turn target.
    pub tactician: bool,
    /// Earned a live, unassisted win before any failure.
    pub first_try: bool,
    /// Lowest turn of any victory, including assisted or archive play.
    pub best_turn: Option<u32>,
}

/// The two account-owned datasets; completion fields are included with participation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportTable {
    /// Preferences, hints, counters and durable accomplishment fields, by task.
    Participation,
    /// Every attempt, including technical aborts and abandoned reservations.
    Attempts,
}

/// Execution activity, separate from the two calendar cadences.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptKind {
    /// A daily task.
    Daily,
    /// A weekly task.
    Weekly,
    /// One manually played expedition floor.
    Tower,
}

/// One attempt's personal metadata. No token, private package or record is exported.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportedAttempt {
    /// Attempt identity and pagination cursor.
    pub id: [u8; 16],
    /// Task identity.
    pub challenge: [u8; 16],
    /// Kind, retained with the attempt.
    pub kind: AttemptKind,
    /// Original client idempotency identity, not a capability.
    pub request_id: [u8; 16],
    /// Claimed game identity.
    pub game: Option<[u8; 16]>,
    /// Current state.
    pub state: AttemptState,
    /// Started attempt number, or zero before the first claim.
    pub number: u32,
    /// Failures at claim time.
    pub failures: u32,
    /// Whether assistance applied to this attempt.
    pub assisted: bool,
    /// Admitted while the challenge was live.
    pub live: bool,
    /// Final engine turn, if finished.
    pub turn: Option<u32>,
    /// Server-measured continuous human time, if eligible.
    pub milliseconds: Option<u64>,
    /// Reservation timestamp in Unix seconds.
    pub created_at: i64,
    /// Claim timestamp in Unix seconds.
    pub started_at: Option<i64>,
    /// Actual finish timestamp in Unix seconds, or Central's time on a user cancellation.
    pub finished_at: Option<i64>,
    /// When Central accepted an official result, in Unix seconds.
    pub received_at: Option<i64>,
}

/// A page contains entries only from its requested dataset.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ExportEntry {
    /// Account-owned progress for a task.
    Participation {
        /// Hint consent and completion fields.
        progress: TaskProgress,
    },
    /// Account-owned attempt metadata.
    Attempt {
        /// Attempt fields, including timestamps.
        attempt: ExportedAttempt,
    },
}

impl ExportEntry {
    /// Dataset and stable cursor of this row.
    #[must_use]
    pub const fn key(&self) -> (ExportTable, [u8; 16]) {
        match self {
            Self::Participation { progress } => (ExportTable::Participation, progress.challenge),
            Self::Attempt { attempt } => (ExportTable::Attempts, attempt.id),
        }
    }
}

/// Validate an export page before following its cursor or handing rows to a caller.
///
/// # Errors
/// Wrong dataset, invalid or nonascending identities, oversized pages or inconsistent cursor.
pub fn validate_page(
    table: ExportTable,
    previous: Option<[u8; 16]>,
    entries: &[ExportEntry],
    next: Option<[u8; 16]>,
) -> Result<(), crate::Error> {
    if entries.len() > super::PAGE_SIZE {
        return Err(crate::Error::Rejected);
    }
    let mut last = previous;
    for entry in entries {
        let (kind, id) = entry.key();
        if kind != table || !super::validation::uuid_v7(&id) || last.is_some_and(|last| id <= last)
        {
            return Err(crate::Error::Rejected);
        }
        last = Some(id);
    }
    if next.is_some() && (entries.len() != super::PAGE_SIZE || next != last) {
        return Err(crate::Error::Rejected);
    }
    Ok(())
}
