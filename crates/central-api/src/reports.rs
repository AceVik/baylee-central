//! Bug and crash reports (`docs/reports.md`): what a gateway files and what
//! an administrator reads and decides. What is heavy (the client's details
//! with the window's picture, a game's record) travels and rests as gzip
//! that Central never unpacks; it checks the announced length and SHA-256.
//! What a list filters and searches by is plain, bounded metadata here.
use crate::Error;
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};

/// The metadata a gateway files, at most.
pub const MAX_INTAKE_BYTES: usize = 65_536;
/// A report's text, in characters.
pub const MAX_TEXT: usize = 20_000;
/// The list's excerpt of the text, in characters.
pub const EXCERPT: usize = 300;
/// A game id, in characters.
pub const MAX_GAME_ID: usize = 128;
/// A reporter pseudonym, in hex digits.
pub const MAX_REPORTER: usize = 128;
/// What a gateway says of itself and a build, each in bytes.
pub const MAX_SAID: usize = 256;
/// A crash's message and place, in characters.
pub const MAX_CRASH_MESSAGE: usize = 2_000;
/// A crash's place, in characters.
pub const MAX_CRASH_LOCATION: usize = 512;
/// The client's details as gzip.
pub const MAX_CLIENT_GZIP: u32 = 2 * 1024 * 1024;
/// A record a gateway kept, as gzip.
pub const MAX_RECORD_GZIP: u32 = 64 * 1024 * 1024;
/// A record a client wrote itself, as gzip.
pub const MAX_CLIENT_RECORD_GZIP: u32 = 4 * 1024 * 1024;
/// A part may follow its report this long, in seconds.
pub const PART_SECONDS: i64 = 3_600;
/// Reports one gateway files in an hour.
pub const PER_GATEWAY_HOUR: u32 = 120;
/// A list's page, and the most one asks for.
pub const PAGE: u32 = 50;
/// The largest page.
pub const MAX_PAGE: u32 = 200;
/// A search, in characters.
pub const MAX_QUERY: usize = 200;
/// A note, in characters.
pub const MAX_NOTE: usize = 4_000;
/// Labels on one report, and one label's length.
pub const MAX_LABELS: usize = 16;
/// One label's length.
pub const MAX_LABEL: usize = 32;
/// Saved views per administrator.
pub const MAX_VIEWS: usize = 50;
/// A saved view's name, in characters.
pub const MAX_VIEW_NAME: usize = 64;
/// A saved view's query, in bytes.
pub const MAX_VIEW_QUERY: usize = 2_048;
/// Reports changed in one request.
pub const MAX_BULK: usize = 200;
/// Reports marked opened in one request.
pub const MAX_SEEN: usize = 500;
/// The busiest reporters a facet lists.
pub const FACET_REPORTERS: usize = 50;
/// Crash groups a list shows.
pub const MAX_CRASH_GROUPS: usize = 100;
/// Days in the trash before the sweep purges a report.
pub const TRASH_DAYS: u32 = 30;
/// Days after a report was done before the sweep deletes it.
pub const DONE_DAYS: u32 = 90;
/// Days after which the sweep deletes any report.
pub const KEEP_DAYS: u32 = 1_095;
/// Days the statistics cover.
pub const STATS_DAYS: u32 = 30;

macro_rules! words {
    ($(#[$meta:meta])* $name:ident { $($(#[$v:meta])* $variant:ident = $text:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($(#[$v])* #[doc = concat!("`", $text, "`")] $variant),+ }
        impl $name {
            /// Every value, in order.
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];
            /// The stored word.
            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $text),+ }
            }
            /// The value a stored word names.
            ///
            /// # Errors
            /// `Unavailable` for a word no value has: the store holds only these.
            pub fn from_stored(word: &str) -> Result<Self, Error> {
                match word { $($text => Ok(Self::$variant),)+ _ => Err(Error::Unavailable) }
            }
        }
    };
}

words!(
    /// What a report is about. A reported chat message is the gateway's
    /// moderation and never filed here.
    ReportKind { Bug = "bug", Improvement = "improvement", Feedback = "feedback", Crash = "crash", Other = "other" }
);
words!(
    /// Where a report stands.
    ReportStatus { New = "new", Triaged = "triaged", InProgress = "in_progress", Resolved = "resolved", WontFix = "wont_fix", Duplicate = "duplicate" }
);
words!(
    /// How urgent an administrator finds it.
    Priority { Critical = "critical", High = "high", Normal = "normal", Low = "low" }
);
words!(
    /// How it came: through a gateway's player, or from a client signed in
    /// nowhere through the official gateway.
    Channel { Gateway = "gateway", Direct = "direct" }
);
words!(
    /// Who wrote the record: the gateway, or the client (unverified).
    RecordOrigin { Gateway = "gateway", Client = "client" }
);
words!(
    /// A part that follows the metadata as gzip.
    Part { Client = "client", Record = "record" }
);
words!(
    /// A list's order.
    ReportSort { Received = "received", Changed = "changed", Priority = "priority", Status = "status", Kind = "kind", Reporter = "reporter", Gateway = "gateway" }
);

impl ReportStatus {
    /// Done: resolved, won't fix, or a duplicate.
    #[must_use]
    pub const fn done(self) -> bool {
        matches!(self, Self::Resolved | Self::WontFix | Self::Duplicate)
    }
}

/// A part announced with the report: its gzip's length and SHA-256.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Announced {
    /// Bytes of gzip.
    pub bytes: u32,
    /// Their SHA-256.
    pub sha256: [u8; 32],
}

/// The record announced with a report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnnouncedRecord {
    /// The game's end is in it.
    pub complete: bool,
    /// Who wrote it.
    pub origin: RecordOrigin,
    /// Bytes of gzip.
    pub bytes: u32,
    /// Their SHA-256.
    pub sha256: [u8; 32],
}

/// What a gateway says of itself; stored beside the authenticated gateway, never instead of it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GatewaySays {
    /// Its name.
    pub name: Option<String>,
    /// Its address.
    pub url: Option<String>,
    /// Its build.
    pub version: Option<String>,
}

/// The client's build.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Build {
    /// Its version.
    pub version: String,
    /// Its commit.
    pub commit: Option<String>,
}

/// A crash's message and place, for grouping and search.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Crash {
    /// The panic's message.
    pub message: String,
    /// Its file, line and column.
    pub location: String,
}

/// A report as a gateway files it (`POST /v1/reports`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportIntake {
    /// What the gateway says of itself.
    pub gateway: GatewaySays,
    /// How it came; `direct` only from the official gateway.
    pub channel: Channel,
    /// The gateway's pseudonym for its player, or a device's: hex.
    pub reporter: String,
    /// What it is about.
    pub kind: ReportKind,
    /// The player's words.
    pub text: String,
    /// The game it is about.
    pub game_id: Option<String>,
    /// The client's build.
    pub build: Option<Build>,
    /// A crash's message and place.
    pub crash: Option<Crash>,
    /// The client's details, to follow as gzip.
    pub client: Option<Announced>,
    /// The record, to follow as gzip.
    pub record: Option<AnnouncedRecord>,
}

fn chars(text: &str) -> usize {
    text.chars().count()
}

/// The invisible characters that change how text around them reads
/// (direction marks, embeddings, overrides, isolates, invisible separators
/// and spaces, the byte order mark); the zero-width joiners stay.
const fn invisible(c: char) -> bool {
    matches!(
        c,
        '\u{061C}'
            | '\u{200B}'
            | '\u{200E}'
            | '\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{2069}'
            | '\u{FEFF}'
    )
}

/// Text another person reads: within `max` characters, no control
/// character (a tab and a newline aside where `lines`), and none of the
/// [`invisible`] ones.
fn shown(text: &str, max: usize, lines: bool) -> bool {
    chars(text) <= max
        && text
            .chars()
            .all(|c| (lines && (c == '\n' || c == '\t')) || (!c.is_control() && !invisible(c)))
}

fn said(text: Option<&str>) -> bool {
    text.is_none_or(|t| !t.is_empty() && t.len() <= MAX_SAID && shown(t, MAX_SAID, false))
}

impl ReportIntake {
    /// Check every bound before anything is stored.
    ///
    /// # Errors
    /// `Rejected` for a field out of its bounds or shape.
    pub fn validate(&self) -> Result<(), Error> {
        let reporter = !self.reporter.is_empty()
            && self.reporter.len() <= MAX_REPORTER
            && self.reporter.bytes().all(|b| b.is_ascii_hexdigit());
        let text = !self.text.trim().is_empty() && shown(&self.text, MAX_TEXT, true);
        let game = self.game_id.as_deref().is_none_or(|g| {
            !g.is_empty() && chars(g) <= MAX_GAME_ID && g.chars().all(|c| c.is_ascii_graphic())
        });
        let build = self.build.as_ref().is_none_or(|b| {
            !b.version.is_empty()
                && b.version.len() <= MAX_SAID
                && shown(&b.version, MAX_SAID, false)
                && b.commit.as_deref().is_none_or(|c| {
                    !c.is_empty() && c.len() <= 64 && c.bytes().all(|b| b.is_ascii_hexdigit())
                })
        });
        let crash = self.crash.as_ref().is_none_or(|c| {
            shown(&c.message, MAX_CRASH_MESSAGE, true)
                && shown(&c.location, MAX_CRASH_LOCATION, false)
        });
        let client = self
            .client
            .as_ref()
            .is_none_or(|p| p.bytes > 0 && p.bytes <= MAX_CLIENT_GZIP);
        let record = self.record.as_ref().is_none_or(|r| {
            r.bytes > 0
                && r.bytes
                    <= match r.origin {
                        RecordOrigin::Gateway => MAX_RECORD_GZIP,
                        RecordOrigin::Client => MAX_CLIENT_RECORD_GZIP,
                    }
        });
        // a direct report carries no game of a gateway's, and a crash names its place only for a crash
        let consistent = (self.channel == Channel::Gateway || self.game_id.is_none())
            && (self.crash.is_none() || self.kind == ReportKind::Crash);
        if reporter
            && text
            && game
            && build
            && crash
            && client
            && record
            && consistent
            && said(self.gateway.name.as_deref())
            && said(self.gateway.url.as_deref())
            && said(self.gateway.version.as_deref())
        {
            Ok(())
        } else {
            Err(Error::Rejected)
        }
    }

    /// The crash group this report falls into, if it is a crash with a place.
    #[must_use]
    pub fn signature(&self) -> Option<String> {
        self.crash
            .as_ref()
            .map(|c| crash_signature(&c.message, &c.location))
    }
}

/// A crash's group: its message with the numbers taken out and its file
/// without line and column, hashed. A grouping key, nothing secret: the
/// first 16 bytes of SHA-256, in hex.
#[must_use]
pub fn crash_signature(message: &str, location: &str) -> String {
    let message: String = message.chars().filter(|c| !c.is_ascii_digit()).collect();
    let file = location.split(':').next().unwrap_or_default();
    let digest = Sha256::digest(format!("{message}\0{file}").as_bytes());
    hex(&digest[..16])
}

/// Lowercase hex of `bytes`.
#[must_use]
pub fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// A label: lowercase letters, digits and dashes, 1 to [`MAX_LABEL`].
#[must_use]
pub fn valid_label(label: &str) -> bool {
    !label.is_empty()
        && label.len() <= MAX_LABEL
        && label
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn day(text: &str) -> bool {
    let b = text.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
        && (1..=12).contains(&text[5..7].parse::<u8>().unwrap_or(0))
        && (1..=31).contains(&text[8..10].parse::<u8>().unwrap_or(0))
}

/// Which reports a list holds (`docs/reports.md` §"Triage"). Every part is
/// optional; several values of one dimension are alternatives.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ReportFilter {
    /// Any of these statuses.
    pub statuses: Vec<ReportStatus>,
    /// Any of these kinds.
    pub kinds: Vec<ReportKind>,
    /// Any of these priorities; `None` is no priority.
    pub priorities: Vec<Option<Priority>>,
    /// Words of the text or of a crash's message, or a piece of either or of the id.
    pub q: Option<String>,
    /// From this UTC day, `YYYY-MM-DD`, inclusive.
    pub from: Option<String>,
    /// To this UTC day, inclusive.
    pub to: Option<String>,
    /// With a record, or without.
    pub has_record: Option<bool>,
    /// Through a gateway, or direct.
    pub channel: Option<Channel>,
    /// Filed by the gateway of this name.
    pub gateway: Option<String>,
    /// Looked after by this administrator.
    pub assignee: Option<[u8; 16]>,
    /// Looked after by nobody.
    pub unassigned: bool,
    /// With this label.
    pub label: Option<String>,
    /// From this reporter.
    pub reporter: Option<String>,
    /// In this crash group.
    pub signature: Option<String>,
    /// Duplicates of this report.
    pub duplicate_of: Option<[u8; 16]>,
    /// In the trash (only there), or out of it.
    pub trash: bool,
    /// Not opened by the reading administrator.
    pub unseen: bool,
}

impl ReportFilter {
    /// Check every part's bounds.
    ///
    /// # Errors
    /// `Rejected` for a part out of its bounds.
    pub fn validate(&self) -> Result<(), Error> {
        let small = self.statuses.len() <= ReportStatus::ALL.len()
            && self.kinds.len() <= ReportKind::ALL.len()
            && self.priorities.len() <= Priority::ALL.len() + 1;
        let q = self
            .q
            .as_deref()
            .is_none_or(|q| !q.trim().is_empty() && shown(q, MAX_QUERY, false));
        let days = self.from.as_deref().is_none_or(day) && self.to.as_deref().is_none_or(day);
        let gateway = self
            .gateway
            .as_deref()
            .is_none_or(|g| !g.is_empty() && g.len() <= MAX_SAID && shown(g, MAX_SAID, false));
        let label = self.label.as_deref().is_none_or(valid_label);
        let reporter = self.reporter.as_deref().is_none_or(|r| {
            !r.is_empty() && r.len() <= MAX_REPORTER && r.bytes().all(|b| b.is_ascii_hexdigit())
        });
        let signature = self.signature.as_deref().is_none_or(|s| {
            s.len() == 32 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
        });
        if small
            && q
            && days
            && gateway
            && label
            && reporter
            && signature
            && !(self.unassigned && self.assignee.is_some())
        {
            Ok(())
        } else {
            Err(Error::Rejected)
        }
    }
}

/// Reads a present field, `null` included, as `Some`: absent stays `None`
/// (the field's `default`), so a change tells "leave it" from "clear it".
fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(d: D) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}

/// A change to a report's triage: a part left out stays, `null` clears it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportChange {
    /// Its status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<ReportStatus>,
    /// The GitHub issue of Baylee it is linked to.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub issue: Option<Option<u32>>,
    /// The administrator who looks after it.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub assignee: Option<Option<[u8; 16]>>,
    /// Its priority.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub priority: Option<Option<Priority>>,
    /// Its labels, the whole set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
    /// Labels added (a selection's change only).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels_add: Vec<String>,
    /// Labels taken off (a selection's change only).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels_remove: Vec<String>,
    /// The report it duplicates.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub duplicate_of: Option<Option<[u8; 16]>>,
}

impl ReportChange {
    /// Check its parts; `bulk` allows the label additions and removals,
    /// and one report's change the whole set.
    ///
    /// # Errors
    /// `Rejected` for a part out of its bounds, or a change that changes nothing.
    pub fn validate(&self, bulk: bool) -> Result<(), Error> {
        let labels = |set: &[String]| set.len() <= MAX_LABELS && set.iter().all(|l| valid_label(l));
        let shape = if bulk {
            self.labels.is_none() && labels(&self.labels_add) && labels(&self.labels_remove)
        } else {
            self.labels_add.is_empty()
                && self.labels_remove.is_empty()
                && self.labels.as_deref().is_none_or(labels)
        };
        let issue = self
            .issue
            .is_none_or(|i| i.is_none_or(|n| (1..=10_000_000).contains(&n)));
        let something = self.status.is_some()
            || self.issue.is_some()
            || self.assignee.is_some()
            || self.priority.is_some()
            || self.labels.is_some()
            || !self.labels_add.is_empty()
            || !self.labels_remove.is_empty()
            || self.duplicate_of.is_some();
        if shape && issue && something {
            Ok(())
        } else {
            Err(Error::Rejected)
        }
    }
}

/// What a selection's request does.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "do", rename_all = "snake_case", deny_unknown_fields)]
pub enum BulkAction {
    /// A change to each, as one report's (no revision).
    Change {
        /// The change.
        change: ReportChange,
    },
    /// Into the trash.
    Trash,
    /// Out of the trash.
    Restore,
    /// Deleted for good from the trash.
    Purge,
}

/// One report as a list shows it: metadata and an excerpt, never `client` or a record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportSummary {
    /// Its id, ordered by arrival.
    pub id: [u8; 16],
    /// When it came, in Unix seconds.
    pub created_at: i64,
    /// When its triage last changed.
    pub updated_at: i64,
    /// The authenticated gateway's name in Central (the official one, or a registered one's).
    pub gateway: String,
    /// What the gateway called itself.
    pub gateway_says: Option<String>,
    /// The gateway's build as it said.
    pub gateway_version: Option<String>,
    /// How it came.
    pub channel: Channel,
    /// The reporter's pseudonym.
    pub reporter: String,
    /// What it is about.
    pub kind: ReportKind,
    /// Where it stands.
    pub status: ReportStatus,
    /// The first [`EXCERPT`] characters of its text.
    pub excerpt: String,
    /// The client's build.
    pub build: Option<String>,
    /// A crash's message.
    pub crash: Option<String>,
    /// It carries a record.
    pub has_record: bool,
    /// The record's game ended in it.
    pub record_complete: bool,
    /// Who wrote the record.
    pub record_origin: Option<RecordOrigin>,
    /// The record's gzip, in bytes.
    pub record_bytes: Option<u32>,
    /// Its GitHub issue.
    pub issue: Option<u32>,
    /// Who looks after it.
    pub assignee: Option<[u8; 16]>,
    /// How urgent.
    pub priority: Option<Priority>,
    /// Its labels, sorted.
    pub labels: Vec<String>,
    /// The report it duplicates.
    pub duplicate_of: Option<[u8; 16]>,
    /// Its crash group.
    pub signature: Option<String>,
    /// Moves on with every change.
    pub revision: u32,
    /// When it was done.
    pub done_at: Option<i64>,
    /// When it went into the trash.
    pub deleted_at: Option<i64>,
    /// The reading administrator opened it.
    pub seen: bool,
    /// Its notes.
    pub notes: u32,
}

/// A note on a report: an administrator's, internal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportNote {
    /// Its id.
    pub id: [u8; 16],
    /// When.
    pub at: i64,
    /// Its author.
    pub author: [u8; 16],
    /// Its words.
    pub text: String,
}

/// A line of a report's history.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportAuditRow {
    /// When.
    pub at: i64,
    /// The administrator; none for the gateway that filed it or the sweep.
    pub actor: Option<[u8; 16]>,
    /// What, such as `report.status`.
    pub action: String,
    /// The change in short, never the report's words.
    pub detail: String,
}

/// A part's state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartState {
    /// Its gzip, in bytes, as announced.
    pub bytes: u32,
    /// It has arrived.
    pub received: bool,
}

/// One report with all but its parts' bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportDetail {
    /// What a list shows.
    pub summary: ReportSummary,
    /// The whole text.
    pub text: String,
    /// The game it is about.
    pub game_id: Option<String>,
    /// The gateway's address as it said.
    pub gateway_url: Option<String>,
    /// The client's commit.
    pub commit: Option<String>,
    /// A crash's place.
    pub crash_location: Option<String>,
    /// The client's details.
    pub client: Option<PartState>,
    /// The record.
    pub record: Option<PartState>,
    /// Its notes, oldest first.
    pub notes: Vec<ReportNote>,
    /// Its history, oldest first.
    pub audit: Vec<ReportAuditRow>,
}

/// How many reports have one value of a dimension.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Count<T> {
    /// The value; `None` for none (no priority, nobody assigned).
    pub value: T,
    /// How many.
    pub count: u64,
}

/// Counts per dimension, each under the filter but its own dimension.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportFacets {
    /// By status.
    pub statuses: Vec<Count<ReportStatus>>,
    /// By kind.
    pub kinds: Vec<Count<ReportKind>>,
    /// By priority.
    pub priorities: Vec<Count<Option<Priority>>>,
    /// By assignee.
    pub assignees: Vec<Count<Option<[u8; 16]>>>,
    /// By label.
    pub labels: Vec<Count<String>>,
    /// By gateway.
    pub gateways: Vec<Count<String>>,
    /// By channel.
    pub channels: Vec<Count<Channel>>,
    /// The busiest reporters, at most [`FACET_REPORTERS`].
    pub reporters: Vec<Count<String>>,
    /// All the filter matches.
    pub total: u64,
}

/// The crashes of one group.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrashGroup {
    /// The group.
    pub signature: String,
    /// Its reports.
    pub count: u64,
    /// Of them, not done.
    pub open: u64,
    /// The first came then.
    pub first: i64,
    /// The last came then.
    pub last: i64,
    /// The newest report.
    pub latest: [u8; 16],
    /// Its message.
    pub message: String,
    /// Its place.
    pub location: String,
}

/// Reports of one kind on one UTC day.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DayCount {
    /// The day, `YYYY-MM-DD`.
    pub day: String,
    /// The kind.
    pub kind: ReportKind,
    /// How many.
    pub count: u64,
}

/// An administrator's saved list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedView {
    /// Its id.
    pub id: [u8; 16],
    /// Its name.
    pub name: String,
    /// The list's query string, as the website reads it.
    pub query: String,
}

/// A saved view's name and query: within their bounds, the query a query string.
#[must_use]
pub fn valid_view(name: &str, query: &str) -> bool {
    !name.trim().is_empty()
        && shown(name, MAX_VIEW_NAME, false)
        && query.len() <= MAX_VIEW_QUERY
        && query.bytes().all(|b| b.is_ascii_graphic())
}

/// An administrator, for assigning a report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportAdmin {
    /// The account.
    pub id: [u8; 16],
    /// Its display name, if it has one.
    pub display_name: Option<String>,
    /// Its tag, if it has one.
    pub tag: Option<u32>,
}

/// A report's place in a list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Neighbours {
    /// The one before it.
    pub previous: Option<[u8; 16]>,
    /// The one after it.
    pub next: Option<[u8; 16]>,
    /// Its place, from 1; none when the list does not hold it.
    pub position: Option<u64>,
    /// The list's length.
    pub total: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn intake() -> ReportIntake {
        ReportIntake {
            gateway: GatewaySays {
                name: Some("Synthetic".into()),
                url: Some("https://gateway.invalid".into()),
                version: Some("0.1.1-beta.9b".into()),
            },
            channel: Channel::Gateway,
            reporter: "5bdc".into(),
            kind: ReportKind::Bug,
            text: "The stack\nwent wrong".into(),
            game_id: Some("0192d0c5-7b3e-7a1f".into()),
            build: Some(Build {
                version: "0.1.1-beta.9b".into(),
                commit: Some("3f9a1c7e21".into()),
            }),
            crash: None,
            client: Some(Announced {
                bytes: 2_000,
                sha256: [1; 32],
            }),
            record: Some(AnnouncedRecord {
                complete: false,
                origin: RecordOrigin::Gateway,
                bytes: MAX_RECORD_GZIP,
                sha256: [2; 32],
            }),
        }
    }

    #[test]
    fn a_report_keeps_to_its_bounds() {
        intake().validate().unwrap();
        let refused: [fn(&mut ReportIntake); 12] = [
            |r| r.reporter = "not hex".into(),
            |r| r.reporter = "a".repeat(MAX_REPORTER + 1),
            |r| r.text = "  ".into(),
            |r| r.text = "x".repeat(MAX_TEXT + 1),
            |r| r.text = "overrides \u{202E}".into(),
            |r| r.game_id = Some("with space".into()),
            |r| {
                r.crash = Some(Crash {
                    message: "m".into(),
                    location: "f".into(),
                });
            },
            |r| {
                r.client = Some(Announced {
                    bytes: MAX_CLIENT_GZIP + 1,
                    sha256: [0; 32],
                });
            },
            |r| {
                r.record = Some(AnnouncedRecord {
                    complete: true,
                    origin: RecordOrigin::Client,
                    bytes: MAX_CLIENT_RECORD_GZIP + 1,
                    sha256: [0; 32],
                });
            },
            |r| r.channel = Channel::Direct,
            |r| r.gateway.name = Some(String::new()),
            |r| {
                r.build = Some(Build {
                    version: "1".into(),
                    commit: Some("xyz".into()),
                });
            },
        ];
        for refuse in refused {
            let mut report = intake();
            refuse(&mut report);
            assert_eq!(report.validate(), Err(Error::Rejected), "{report:?}");
        }
        // a chat message is the gateway's moderation, and an unknown field is no report
        let mut text = serde_json::to_value(intake()).unwrap();
        text["kind"] = "chat".into();
        assert!(serde_json::from_value::<ReportIntake>(text.clone()).is_err());
        text["kind"] = "bug".into();
        text["sender"] = "x".into();
        assert!(serde_json::from_value::<ReportIntake>(text).is_err());
    }

    #[test]
    fn crashes_group_by_message_without_numbers_and_file_without_line() {
        let a = crash_signature(
            "index 7 out of range for 3",
            "crates/engine/src/stack.rs:120:9",
        );
        let b = crash_signature(
            "index 12 out of range for 40",
            "crates/engine/src/stack.rs:131:5",
        );
        let c = crash_signature(
            "index 7 out of range for 3",
            "crates/engine/src/zone.rs:120:9",
        );
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.len(), 32);
    }

    #[test]
    fn a_change_tells_leave_from_clear() {
        let change: ReportChange =
            serde_json::from_str(r#"{"assignee":null,"status":"triaged"}"#).unwrap();
        assert_eq!(change.assignee, Some(None));
        assert_eq!(change.priority, None);
        change.validate(false).unwrap();
        assert_eq!(
            ReportChange::default().validate(false),
            Err(Error::Rejected),
            "nothing to change"
        );
        let bulk = ReportChange {
            labels_add: vec!["ui".into()],
            ..ReportChange::default()
        };
        bulk.validate(true).unwrap();
        assert_eq!(bulk.validate(false), Err(Error::Rejected));
        let bad = ReportChange {
            labels: Some(vec!["Not Ok".into()]),
            ..ReportChange::default()
        };
        assert_eq!(bad.validate(false), Err(Error::Rejected));
        // round trip: a cleared part stays cleared
        let text = serde_json::to_string(&change).unwrap();
        assert_eq!(serde_json::from_str::<ReportChange>(&text).unwrap(), change);
    }

    #[test]
    fn a_filter_is_typed_and_bounded() {
        let filter: ReportFilter = serde_json::from_str(
            r#"{"statuses":["new","triaged"],"priorities":[null,"high"],"from":"2026-10-01","q":"stack"}"#,
        )
        .unwrap();
        filter.validate().unwrap();
        for bad in [
            r#"{"from":"2026-13-01"}"#,
            r#"{"signature":"ABC"}"#,
            r#"{"label":"Bad Label"}"#,
            r#"{"q":"   "}"#,
            r#"{"unassigned":true,"assignee":[1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1]}"#,
        ] {
            let filter: ReportFilter = serde_json::from_str(bad).unwrap();
            assert_eq!(filter.validate(), Err(Error::Rejected), "{bad}");
        }
        assert!(serde_json::from_str::<ReportFilter>(r#"{"sql":"1=1"}"#).is_err());
    }

    #[test]
    fn stored_words_read_back_and_nothing_else() {
        for status in ReportStatus::ALL {
            assert_eq!(ReportStatus::from_stored(status.as_str()), Ok(*status));
        }
        assert_eq!(ReportKind::from_stored("chat"), Err(Error::Unavailable));
        assert!(ReportStatus::Duplicate.done() && !ReportStatus::InProgress.done());
    }

    /// A report as Baylee's gateway writes it (its `baylee_account::reports`
    /// mirrors this type, which its pinned central-api predates): this must
    /// keep decoding, or every report from a gateway is refused.
    #[test]
    fn a_report_as_baylees_gateway_writes_it() {
        let sha = |b: u8| serde_json::Value::from(vec![b; 32]);
        let json = serde_json::json!({
            "gateway": {"name": "Synthetic gateway", "url": "https://gateway.example", "version": "0.1.1-beta.9b"},
            "channel": "gateway",
            "reporter": "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843",
            "kind": "crash",
            "text": "It crashed\nwhile I was choosing attackers",
            "game_id": "0192d0c5-7b50-7a1f-8c2d-3e4f5a6b7c01",
            "build": {"version": "0.1.1-beta.9b", "commit": "3f9a1c7e21"},
            "crash": {"message": "index out of bounds", "location": "crates/baylee-client/src/table.rs:812:17"},
            "client": {"bytes": 182, "sha256": sha(7)},
            "record": {"complete": false, "origin": "gateway", "bytes": 4_000, "sha256": sha(9)},
        });
        let report: ReportIntake = serde_json::from_value(json).unwrap();
        report.validate().unwrap();
        let bare = serde_json::json!({
            "gateway": {"name": null, "url": null, "version": "0.1.1"},
            "channel": "gateway", "reporter": "ab", "kind": "bug", "text": "t", "game_id": null,
            "build": null, "crash": null, "client": null, "record": null,
        });
        serde_json::from_value::<ReportIntake>(bare)
            .unwrap()
            .validate()
            .unwrap();
    }
}
