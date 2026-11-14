//! Durable personal records, including the ranked prefix before practice continued.
use super::{Period, Request, Run};
use serde::{Deserialize, Serialize};

/// Independently bounded personal datasets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportTable {
    /// Expeditions and their last qualifying ranked prefix.
    Runs,
    /// Exact accepted idempotent begin, start and upgrade requests.
    Actions,
    /// Season consent, including seasons with no expedition, and earned floors.
    Participation,
}

/// Persisted qualifying result, which can differ from subsequent practice progress.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RankedScore {
    /// Cleared qualifying floors.
    pub floors: u8,
    /// Lives lost before that result.
    pub losses: u8,
    /// Winning turns in that prefix.
    pub turns: u32,
    /// Timed duration, absent after an interruption.
    pub milliseconds: Option<u64>,
}

/// A distinct earned floor that contributes to permanent relic progress.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Accomplishment {
    /// One-based floor.
    pub floor: u8,
    /// Whether this floor was ever reached without losing a life in its run.
    pub flawless: bool,
}

/// Private export record; never includes execution packages or authentication capabilities.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "record", rename_all = "snake_case", deny_unknown_fields)]
pub enum ExportEntry {
    /// Complete durable expedition metadata.
    Run {
        /// Current state, route and consent.
        run: Box<Run>,
        /// Original begin request identity.
        request_id: [u8; 16],
        /// Exact persisted creation timestamp, microseconds since Unix epoch.
        created_at_micros: i64,
        /// Frozen qualifying prefix, if the run has one.
        ranked: Option<RankedScore>,
    },
    /// Accepted retry-safe operation.
    Action {
        /// Stable request identity, also the pagination cursor.
        request_id: [u8; 16],
        /// Owned expedition.
        run: [u8; 16],
        /// Exact original request.
        request: Box<Request>,
        /// Exact persisted creation timestamp, microseconds since Unix epoch.
        created_at_micros: i64,
    },
    /// Consent and earned floors for a season, independently of runs.
    Participation {
        /// Season identity, also the pagination cursor.
        season: [u8; 16],
        /// Canonical UTC week.
        period: Period,
        /// Current public-name consent.
        listed: bool,
        /// At most five exact earned-floor records.
        accomplishments: Vec<Accomplishment>,
    },
}

impl ExportEntry {
    /// Dataset and exclusive cursor identity of this record.
    #[must_use]
    pub fn identity(&self) -> (ExportTable, [u8; 16]) {
        match self {
            Self::Run { run, .. } => (ExportTable::Runs, run.id),
            Self::Action { request_id, .. } => (ExportTable::Actions, *request_id),
            Self::Participation { season, .. } => (ExportTable::Participation, *season),
        }
    }
}
