//! What a report token's holder sends and is answered on the native
//! listener's report routes (`docs/reports.md` §"Report tokens"): a contract
//! of its own, apart from the administration's types (`admin`, `reports`),
//! which change with the website. Ids are 32 lowercase hex digits, a body's
//! unknown field is refused, and what a body asks for is turned into the
//! administration's typed and bounded forms before anything acts on it.
use crate::reports::{
    BulkAction, Channel, CrashGroup, DayCount, PAGE, PartState, Priority, RecordOrigin,
    ReportAdmin, ReportAuditRow, ReportChange, ReportDetail, ReportFacets, ReportFilter,
    ReportKind, ReportNote, ReportSort, ReportStatus, ReportSummary, hex, present,
};
use crate::{Error, reports};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A request's body, at most.
pub const MAX_BODY_BYTES: usize = reports::MAX_INTAKE_BYTES;
/// An answer, at most: two hundred reports whose every bounded part is at
/// its largest fit, and so does one report with its notes
/// ([`reports::MAX_NOTES_BYTES`]) and its history ([`reports::HISTORY`]).
pub const MAX_ANSWER_BYTES: usize = 8 * 1024 * 1024;

/// Encode an answer. Apart from the wire's bound, which is a page's and a
/// client's.
///
/// # Errors
/// `Unavailable` for an answer out of its bound: nothing stored is.
pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, Error> {
    let bytes = serde_json::to_vec(value).map_err(|_| Error::Unavailable)?;
    if bytes.len() > MAX_ANSWER_BYTES {
        return Err(Error::Unavailable);
    }
    Ok(bytes)
}

/// An id: 32 lowercase hex digits, as the routes' addresses write it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id(pub [u8; 16]);

impl Id {
    /// The id `text` writes, and only its one spelling.
    ///
    /// # Errors
    /// `Rejected` for any other length, case or character.
    pub fn parse(text: &str) -> Result<Self, Error> {
        crate::token::lower_hex(text).map(|id| Self(*id))
    }
}

impl Serialize for Id {
    fn serialize<S: Serializer>(&self, to: S) -> Result<S::Ok, S::Error> {
        to.serialize_str(&hex(&self.0))
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D: Deserializer<'de>>(from: D) -> Result<Self, D::Error> {
        let text = String::deserialize(from)?;
        Self::parse(&text).map_err(|_| serde::de::Error::custom("an id is 32 lowercase hex digits"))
    }
}

fn ids(id: Option<[u8; 16]>) -> Option<Id> {
    id.map(Id)
}

// ---- What is sent ------------------------------------------------------------

/// Which reports: every part optional, several values of one dimension are
/// alternatives.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Filter {
    /// Any of these statuses.
    pub statuses: Vec<ReportStatus>,
    /// Any of these kinds.
    pub kinds: Vec<ReportKind>,
    /// Any of these priorities; `null` is no priority.
    pub priorities: Vec<Option<Priority>>,
    /// Words of the text or of a crash's message, or a piece of either or of the id.
    pub q: Option<String>,
    /// From this UTC day, `YYYY-MM-DD`, inclusive.
    pub from: Option<String>,
    /// To this UTC day, inclusive.
    pub to: Option<String>,
    /// With a record, or without.
    pub has_record: Option<bool>,
    /// Through a gateway, direct, or from a report token.
    pub channel: Option<Channel>,
    /// Filed by the gateway of this name.
    pub gateway: Option<String>,
    /// Filed by this report token.
    pub token: Option<Id>,
    /// Looked after by this administrator.
    pub assignee: Option<Id>,
    /// Looked after by nobody.
    pub unassigned: bool,
    /// With this label.
    pub label: Option<String>,
    /// From this reporter.
    pub reporter: Option<String>,
    /// In this crash group.
    pub signature: Option<String>,
    /// Duplicates of this report.
    pub duplicate_of: Option<Id>,
    /// In the trash (only there), or out of it.
    pub trash: bool,
}

impl Filter {
    /// The administration's filter, its bounds checked. A token opens
    /// nothing, so nothing is asked of what was opened.
    ///
    /// # Errors
    /// `Rejected` for a part out of its bounds.
    pub fn typed(self) -> Result<ReportFilter, Error> {
        let filter = ReportFilter {
            statuses: self.statuses,
            kinds: self.kinds,
            priorities: self.priorities,
            q: self.q,
            from: self.from,
            to: self.to,
            has_record: self.has_record,
            channel: self.channel,
            gateway: self.gateway,
            token: self.token.map(|id| id.0),
            assignee: self.assignee.map(|id| id.0),
            unassigned: self.unassigned,
            label: self.label,
            reporter: self.reporter,
            signature: self.signature,
            duplicate_of: self.duplicate_of.map(|id| id.0),
            trash: self.trash,
            unseen: false,
        };
        filter.validate()?;
        Ok(filter)
    }
}

/// `POST /v1/reports/search`: a page of the reports a filter holds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Search {
    /// Which.
    pub filter: Filter,
    /// In what order.
    pub sort: ReportSort,
    /// Newest, largest or last first.
    pub descending: bool,
    /// Skipped reports.
    pub offset: u32,
    /// At most [`reports::MAX_PAGE`].
    pub limit: u32,
}

impl Default for Search {
    /// The newest reports first, a page of them.
    fn default() -> Self {
        Self {
            filter: Filter::default(),
            sort: ReportSort::Received,
            descending: true,
            offset: 0,
            limit: PAGE,
        }
    }
}

/// `POST /v1/reports/facets` and `/v1/reports/crashes`: over which reports.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Over {
    /// Which.
    pub filter: Filter,
}

/// A change to a report's triage: a part left out stays, `null` clears it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Change {
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
    pub assignee: Option<Option<Id>>,
    /// Its priority.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub priority: Option<Option<Priority>>,
    /// Its labels, the whole set (one report's change only).
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
    pub duplicate_of: Option<Option<Id>>,
}

impl Change {
    /// The administration's change, its bounds checked as one report's or,
    /// with `bulk`, as a selection's.
    ///
    /// # Errors
    /// `Rejected` for a part out of its bounds or a change that changes nothing.
    pub fn typed(self, bulk: bool) -> Result<ReportChange, Error> {
        let change = ReportChange {
            status: self.status,
            issue: self.issue,
            assignee: self.assignee.map(|id| id.map(|id| id.0)),
            priority: self.priority,
            labels: self.labels,
            labels_add: self.labels_add,
            labels_remove: self.labels_remove,
            duplicate_of: self.duplicate_of.map(|id| id.map(|id| id.0)),
        };
        change.validate(bulk)?;
        Ok(change)
    }
}

/// `PATCH /v1/reports/{id}`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Patch {
    /// What changes.
    pub change: Change,
    /// The revision it was read at: a report that moved on is a conflict.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_revision: Option<u32>,
}

/// `POST /v1/reports/{id}/notes`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewNote {
    /// Its words, 1 to [`reports::MAX_NOTE`] characters.
    pub text: String,
}

impl NewNote {
    /// Its words, if a person can read them as they stand: within their
    /// bound, and none of the characters a report's own text may not hold.
    ///
    /// # Errors
    /// `Rejected` for an empty or overlong note, or one with a control or
    /// invisible character.
    pub fn typed(self) -> Result<String, Error> {
        if self.text.trim().is_empty() || !reports::shown(&self.text, reports::MAX_NOTE, true) {
            return Err(Error::Rejected);
        }
        Ok(self.text)
    }
}

/// What a selection's request does; its purge is a route of its own, since
/// it is another right.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "do", rename_all = "snake_case", deny_unknown_fields)]
pub enum BulkDo {
    /// A change to each, as one report's (no revision).
    Change {
        /// The change.
        change: Change,
    },
    /// Into the trash.
    Trash,
    /// Out of the trash.
    Restore,
}

/// `POST /v1/reports/bulk`: all of a selection or none.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bulk {
    /// Which, at most [`reports::MAX_BULK`].
    pub reports: Vec<Id>,
    /// What.
    pub bulk: BulkDo,
}

impl Bulk {
    /// The administration's selection and action, the change's bounds
    /// checked; never a purge.
    ///
    /// # Errors
    /// `Rejected` for a change out of its bounds.
    pub fn typed(self) -> Result<(Vec<[u8; 16]>, BulkAction), Error> {
        let action = match self.bulk {
            BulkDo::Change { change } => BulkAction::Change {
                change: change.typed(true)?,
            },
            BulkDo::Trash => BulkAction::Trash,
            BulkDo::Restore => BulkAction::Restore,
        };
        Ok((self.reports.into_iter().map(|id| id.0).collect(), action))
    }
}

/// `POST /v1/reports/purge`: deleted for good from the trash, all or none.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    /// Which, at most [`reports::MAX_BULK`].
    pub reports: Vec<Id>,
}

// ---- What is answered --------------------------------------------------------

/// The record a report announced.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    /// The game's end is in it.
    pub complete: bool,
    /// Who wrote it: the gateway, or a client (unverified).
    pub origin: RecordOrigin,
    /// Its gzip, in bytes.
    pub bytes: Option<u32>,
}

/// One report as a list shows it: metadata and an excerpt, no part.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    /// Its id, ordered by arrival.
    pub id: Id,
    /// When it came, in Unix seconds.
    pub created_at: i64,
    /// When its triage last changed.
    pub updated_at: i64,
    /// How it came.
    pub channel: Channel,
    /// The authenticated gateway's name in Central, or the report token's, as it was then.
    pub source: String,
    /// The report token that filed it; none for a gateway's.
    pub token: Option<Id>,
    /// What the gateway called itself.
    pub gateway_says: Option<String>,
    /// The gateway's build as it said.
    pub gateway_version: Option<String>,
    /// The reporter's pseudonym.
    pub reporter: String,
    /// What it is about.
    pub kind: ReportKind,
    /// Where it stands.
    pub status: ReportStatus,
    /// The first [`reports::EXCERPT`] characters of its text.
    pub excerpt: String,
    /// The client's build.
    pub build: Option<String>,
    /// A crash's message.
    pub crash: Option<String>,
    /// The record it announced.
    pub record: Option<Record>,
    /// Its GitHub issue.
    pub issue: Option<u32>,
    /// The administrator who looks after it.
    pub assignee: Option<Id>,
    /// How urgent.
    pub priority: Option<Priority>,
    /// Its labels, sorted.
    pub labels: Vec<String>,
    /// The report it duplicates.
    pub duplicate_of: Option<Id>,
    /// Its crash group.
    pub signature: Option<String>,
    /// Moves on with every change.
    pub revision: u32,
    /// When it was done.
    pub done_at: Option<i64>,
    /// When it went into the trash.
    pub deleted_at: Option<i64>,
    /// How many notes it has.
    pub notes: u32,
}

impl From<ReportSummary> for Report {
    fn from(r: ReportSummary) -> Self {
        Self {
            id: Id(r.id),
            created_at: r.created_at,
            updated_at: r.updated_at,
            channel: r.channel,
            source: r.gateway,
            token: ids(r.token),
            gateway_says: r.gateway_says,
            gateway_version: r.gateway_version,
            reporter: r.reporter,
            kind: r.kind,
            status: r.status,
            excerpt: r.excerpt,
            build: r.build,
            crash: r.crash,
            record: r.record_origin.map(|origin| Record {
                complete: r.record_complete,
                origin,
                bytes: r.record_bytes,
            }),
            issue: r.issue,
            assignee: ids(r.assignee),
            priority: r.priority,
            labels: r.labels,
            duplicate_of: ids(r.duplicate_of),
            signature: r.signature,
            revision: r.revision,
            done_at: r.done_at,
            deleted_at: r.deleted_at,
            notes: r.notes,
        }
    }
}

/// The answer to a [`Search`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page {
    /// At most the limit asked for.
    pub reports: Vec<Report>,
    /// All the filter matches.
    pub total: u64,
}

/// Who wrote a note or changed a report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum By {
    /// An administrator.
    Admin {
        /// Their account.
        id: Id,
    },
    /// A report token.
    Token {
        /// The token.
        id: Id,
        /// Its name then.
        name: String,
    },
}

impl By {
    fn of(id: [u8; 16], token: Option<String>) -> Self {
        match token {
            Some(name) => Self::Token { id: Id(id), name },
            None => Self::Admin { id: Id(id) },
        }
    }
}

/// A note on a report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    /// Its id.
    pub id: Id,
    /// When.
    pub at: i64,
    /// Its author.
    pub by: By,
    /// Its words.
    pub text: String,
}

impl From<ReportNote> for Note {
    fn from(note: ReportNote) -> Self {
        Self {
            id: Id(note.id),
            at: note.at,
            by: By::of(note.author, note.token),
            text: note.text,
        }
    }
}

/// A line of a report's history.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryRow {
    /// When.
    pub at: i64,
    /// Who; none for the gateway that filed it or the sweep.
    pub by: Option<By>,
    /// What, such as `report.status`.
    pub action: String,
    /// The change in short, never the report's words.
    pub detail: String,
}

impl From<ReportAuditRow> for HistoryRow {
    fn from(row: ReportAuditRow) -> Self {
        Self {
            at: row.at,
            by: row.actor.map(|id| By::of(id, row.token)),
            action: row.action,
            detail: row.detail,
        }
    }
}

/// `GET /v1/reports/{id}` and the answer to a change: one report with all
/// but its parts' bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Detail {
    /// What a list shows.
    pub report: Report,
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
    /// The client's details, and whether they arrived.
    pub client: Option<PartState>,
    /// The record, and whether it arrived.
    pub record: Option<PartState>,
    /// Its notes, oldest first.
    pub notes: Vec<Note>,
    /// Its history, oldest first: the newest [`reports::HISTORY`] lines.
    pub history: Vec<HistoryRow>,
    /// Its history has older lines.
    pub older: bool,
}

impl From<ReportDetail> for Detail {
    fn from(d: ReportDetail) -> Self {
        Self {
            report: d.summary.into(),
            text: d.text,
            game_id: d.game_id,
            gateway_url: d.gateway_url,
            commit: d.commit,
            crash_location: d.crash_location,
            client: d.client,
            record: d.record,
            notes: d.notes.into_iter().map(Note::from).collect(),
            history: d.audit.into_iter().map(HistoryRow::from).collect(),
            older: d.older,
        }
    }
}

/// How many reports have one value of a dimension.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Counted<T> {
    /// The value; `null` for none (no priority, nobody assigned).
    pub value: T,
    /// How many.
    pub count: u64,
}

/// A report token by its id and its name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenName {
    /// Its id.
    pub id: Id,
    /// Its name.
    pub name: String,
}

/// The answer to `POST /v1/reports/facets`: counts per dimension, each under
/// the filter but its own.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Facets {
    /// By status.
    pub statuses: Vec<Counted<ReportStatus>>,
    /// By kind.
    pub kinds: Vec<Counted<ReportKind>>,
    /// By priority.
    pub priorities: Vec<Counted<Option<Priority>>>,
    /// By assignee.
    pub assignees: Vec<Counted<Option<Id>>>,
    /// By label.
    pub labels: Vec<Counted<String>>,
    /// By gateway.
    pub gateways: Vec<Counted<String>>,
    /// By report token.
    pub tokens: Vec<Counted<TokenName>>,
    /// By channel.
    pub channels: Vec<Counted<Channel>>,
    /// The busiest reporters, at most [`reports::FACET_REPORTERS`].
    pub reporters: Vec<Counted<String>>,
    /// All the filter matches.
    pub total: u64,
}

fn counted<T, U>(rows: Vec<reports::Count<T>>, read: impl Fn(T) -> U) -> Vec<Counted<U>> {
    rows.into_iter()
        .map(|row| Counted {
            value: read(row.value),
            count: row.count,
        })
        .collect()
}

impl From<ReportFacets> for Facets {
    fn from(f: ReportFacets) -> Self {
        Self {
            statuses: counted(f.statuses, |v| v),
            kinds: counted(f.kinds, |v| v),
            priorities: counted(f.priorities, |v| v),
            assignees: counted(f.assignees, ids),
            labels: counted(f.labels, |v| v),
            gateways: counted(f.gateways, |v| v),
            tokens: counted(f.tokens, |v| TokenName {
                id: Id(v.id),
                name: v.name,
            }),
            channels: counted(f.channels, |v| v),
            reporters: counted(f.reporters, |v| v),
            total: f.total,
        }
    }
}

/// The crashes of one group.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Crashes {
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
    pub latest: Id,
    /// Its message.
    pub message: String,
    /// Its place.
    pub location: String,
}

impl From<CrashGroup> for Crashes {
    fn from(g: CrashGroup) -> Self {
        Self {
            signature: g.signature,
            count: g.count,
            open: g.open,
            first: g.first,
            last: g.last,
            latest: Id(g.latest),
            message: g.message,
            location: g.location,
        }
    }
}

/// The answer to `POST /v1/reports/crashes`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrashGroups {
    /// At most [`reports::MAX_CRASH_GROUPS`], the latest first.
    pub groups: Vec<Crashes>,
}

/// The answer to `GET /v1/reports/stats`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stats {
    /// Over [`reports::STATS_DAYS`] days, by day then kind.
    pub days: Vec<DayCount>,
}

/// An administrator a report may be assigned to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Assignee {
    /// The account.
    pub id: Id,
    /// Its display name, if it has one.
    pub display_name: Option<String>,
    /// Its tag, if it has one.
    pub tag: Option<u32>,
}

impl From<ReportAdmin> for Assignee {
    fn from(admin: ReportAdmin) -> Self {
        Self {
            id: Id(admin.id),
            display_name: admin.display_name,
            tag: admin.tag,
        }
    }
}

/// The answer to `GET /v1/reports/admins`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Assignees {
    /// Each once.
    pub admins: Vec<Assignee>,
}

/// The answer to a selection's change or purge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Changed {
    /// How many reports changed.
    pub changed: u32,
}

/// The answer to `POST /v1/reports`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Filed {
    /// The report's id.
    pub report_id: Id,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reports::{Count, NamedToken};

    const HEX: &str = "0192d0c57b3e7a1f8c2d3e4f5a6b7c8d";

    fn id() -> [u8; 16] {
        Id::parse(HEX).unwrap().0
    }

    #[test]
    fn an_id_has_one_spelling() {
        assert_eq!(
            serde_json::to_string(&Id(id())).unwrap(),
            format!("\"{HEX}\"")
        );
        assert_eq!(
            serde_json::from_str::<Id>(&format!("\"{HEX}\"")).unwrap(),
            Id(id())
        );
        for bad in [
            HEX.to_uppercase(),
            HEX[1..].to_owned(),
            format!("{HEX}0"),
            "0192d0c5-7b3e-7a1f-8c2d-3e4f5a6b7c8d".to_owned(),
            format!(" {HEX}"),
            String::new(),
        ] {
            assert!(Id::parse(&bad).is_err(), "{bad:?}");
            assert!(serde_json::from_str::<Id>(&format!("\"{bad}\"")).is_err());
        }
        assert!(serde_json::from_str::<Id>("[1,2,3]").is_err());
    }

    #[test]
    fn a_search_is_typed_bounded_and_the_newest_by_default() {
        let search: Search = serde_json::from_str("{}").unwrap();
        assert_eq!(search, Search::default());
        assert_eq!(
            (search.sort, search.descending, search.offset, search.limit),
            (ReportSort::Received, true, 0, PAGE)
        );
        let search: Search = serde_json::from_str(&format!(
            r#"{{"filter":{{"statuses":["new"],"priorities":[null,"high"],"channel":"token","token":"{HEX}",
                "assignee":"{HEX}","duplicate_of":"{HEX}","q":"stack","trash":true}},"sort":"changed","descending":false,"limit":10}}"#
        ))
        .unwrap();
        let filter = search.filter.typed().unwrap();
        assert_eq!(
            (filter.token, filter.assignee, filter.duplicate_of),
            (Some(id()), Some(id()), Some(id()))
        );
        assert_eq!(filter.channel, Some(Channel::Token));
        assert!(filter.trash && !filter.unseen);
        for bad in [
            r#"{"filter":{"unseen":true}}"#,
            r#"{"filter":{"sql":"1=1"}}"#,
            r#"{"page":1}"#,
            r#"{"limit":-1}"#,
            r#"{"filter":{"assignee":[1,2]}}"#,
        ] {
            assert!(serde_json::from_str::<Search>(bad).is_err(), "{bad}");
        }
        for out_of_bounds in [
            r#"{"q":"  "}"#,
            r#"{"from":"2026-13-01"}"#,
            r#"{"label":"Not A Label"}"#,
            r#"{"reporter":"%"}"#,
        ] {
            let filter: Filter = serde_json::from_str(out_of_bounds).unwrap();
            assert_eq!(
                filter.typed().err(),
                Some(Error::Rejected),
                "{out_of_bounds}"
            );
        }
        assert!(serde_json::from_str::<Over>(r#"{"limit":5}"#).is_err());
    }

    #[test]
    fn a_change_tells_leave_from_clear_and_a_selection_never_purges() {
        let patch: Patch = serde_json::from_str(&format!(
            r#"{{"change":{{"assignee":null,"duplicate_of":"{HEX}"}},"expected_revision":3}}"#
        ))
        .unwrap();
        assert_eq!(patch.expected_revision, Some(3));
        let change = patch.change.typed(false).unwrap();
        assert_eq!(change.assignee, Some(None));
        assert_eq!(change.duplicate_of, Some(Some(id())));
        assert_eq!(change.status, None);
        for bad in [
            r#"{"change":{"text":"rewritten"}}"#,
            r#"{"change":{"deleted_at":1}}"#,
            r#"{"change":{"status":"gone"}}"#,
            r#"{"change":{"status":"new"},"force":true}"#,
            r#"{"status":"new"}"#,
        ] {
            assert!(serde_json::from_str::<Patch>(bad).is_err(), "{bad}");
        }
        assert_eq!(
            Change::default().typed(false).err(),
            Some(Error::Rejected),
            "nothing to change"
        );
        let bulk: Bulk = serde_json::from_str(&format!(
            r#"{{"reports":["{HEX}"],"bulk":{{"do":"change","change":{{"labels_add":["ui"]}}}}}}"#
        ))
        .unwrap();
        let (reports, action) = bulk.typed().unwrap();
        assert_eq!(reports, [id()]);
        assert!(matches!(action, BulkAction::Change { .. }));
        for (text, expected) in [
            (r#"{"do":"trash"}"#, BulkAction::Trash),
            (r#"{"do":"restore"}"#, BulkAction::Restore),
        ] {
            let bulk = Bulk {
                reports: vec![Id(id())],
                bulk: serde_json::from_str(text).unwrap(),
            };
            assert_eq!(bulk.typed().unwrap().1, expected);
        }
        assert!(serde_json::from_str::<BulkDo>(r#"{"do":"purge"}"#).is_err());
        // a selection's change takes no whole set of labels
        let whole = Bulk {
            reports: vec![Id(id())],
            bulk: serde_json::from_str(r#"{"do":"change","change":{"labels":["ui"]}}"#).unwrap(),
        };
        assert_eq!(whole.typed().err(), Some(Error::Rejected));
    }

    #[test]
    fn a_note_is_words_a_person_reads_as_they_stand() {
        let note = |text: &str| NewNote { text: text.into() }.typed();
        assert_eq!(
            note("a line\n\tand another").as_deref(),
            Ok("a line\n\tand another")
        );
        assert!(note(&"ä".repeat(reports::MAX_NOTE)).is_ok());
        for bad in [
            String::new(),
            " \n".into(),
            "ä".repeat(reports::MAX_NOTE + 1),
            "\u{1b}[2J".into(),
            "over\u{202e}ride".into(),
        ] {
            assert_eq!(note(&bad), Err(Error::Rejected), "{bad:?}");
        }
        assert!(serde_json::from_str::<NewNote>(r#"{"text":"n","author":"x"}"#).is_err());
    }

    fn summary() -> ReportSummary {
        ReportSummary {
            id: id(),
            created_at: 1,
            updated_at: 2,
            gateway: "CI".into(),
            token: Some(id()),
            gateway_says: None,
            gateway_version: None,
            channel: Channel::Token,
            reporter: "5bdc".into(),
            kind: ReportKind::Bug,
            status: ReportStatus::New,
            excerpt: "e".into(),
            build: None,
            crash: None,
            has_record: true,
            record_complete: true,
            record_origin: Some(RecordOrigin::Client),
            record_bytes: Some(7),
            issue: None,
            assignee: Some(id()),
            priority: None,
            labels: vec!["ui".into()],
            duplicate_of: None,
            signature: None,
            revision: 0,
            done_at: None,
            deleted_at: None,
            seen: false,
            notes: 1,
        }
    }

    /// The largest page fits its bound: two hundred reports whose every
    /// bounded part is at its longest, in signs that travel as two each.
    #[test]
    fn the_largest_page_fits_its_answer() {
        let quotes = |n: usize| "\"".repeat(n);
        let wide = |n: usize| "\u{10348}".repeat(n);
        let largest = Report {
            source: quotes(256),
            gateway_says: Some(quotes(reports::MAX_SAID)),
            gateway_version: Some(quotes(reports::MAX_SAID)),
            reporter: "a".repeat(reports::MAX_REPORTER),
            excerpt: wide(reports::EXCERPT),
            build: Some(quotes(reports::MAX_SAID)),
            crash: Some(wide(reports::MAX_CRASH_MESSAGE)),
            labels: vec!["l".repeat(reports::MAX_LABEL); reports::MAX_LABELS],
            signature: Some("a".repeat(32)),
            duplicate_of: Some(Id(id())),
            ..Report::from(summary())
        };
        let page = Page {
            reports: vec![largest; reports::MAX_PAGE as usize],
            total: u64::MAX,
        };
        assert!(encode(&page).unwrap().len() <= MAX_ANSWER_BYTES);
    }

    /// The contract's answers, each field by name: a change here is a change
    /// of what scripts read.
    #[test]
    fn the_answers_have_the_designed_shapes() {
        let report = format!(
            r#"{{"id":"{HEX}","created_at":1,"updated_at":2,"channel":"token","source":"CI","token":"{HEX}","gateway_says":null,"gateway_version":null,"reporter":"5bdc","kind":"bug","status":"new","excerpt":"e","build":null,"crash":null,"record":{{"complete":true,"origin":"client","bytes":7}},"issue":null,"assignee":"{HEX}","priority":null,"labels":["ui"],"duplicate_of":null,"signature":null,"revision":0,"done_at":null,"deleted_at":null,"notes":1}}"#
        );
        let page = Page {
            reports: vec![summary().into()],
            total: 1,
        };
        assert_eq!(
            serde_json::to_string(&page).unwrap(),
            format!(r#"{{"reports":[{report}],"total":1}}"#)
        );
        let detail: Detail = ReportDetail {
            summary: summary(),
            text: "t".into(),
            game_id: None,
            gateway_url: None,
            commit: None,
            crash_location: None,
            client: None,
            record: Some(PartState {
                bytes: 7,
                received: false,
            }),
            notes: vec![
                ReportNote {
                    id: id(),
                    at: 1,
                    author: id(),
                    token: None,
                    text: "by a person".into(),
                },
                ReportNote {
                    id: id(),
                    at: 2,
                    author: id(),
                    token: Some("CI".into()),
                    text: "by a token".into(),
                },
            ],
            audit: vec![
                ReportAuditRow {
                    at: 1,
                    actor: None,
                    token: None,
                    action: "report.swept".into(),
                    detail: String::new(),
                },
                ReportAuditRow {
                    at: 2,
                    actor: Some(id()),
                    token: Some("CI".into()),
                    action: "report.status".into(),
                    detail: "new → triaged".into(),
                },
            ],
            older: true,
        }
        .into();
        assert_eq!(
            serde_json::to_string(&detail).unwrap(),
            format!(
                r#"{{"report":{report},"text":"t","game_id":null,"gateway_url":null,"commit":null,"crash_location":null,"client":null,"record":{{"bytes":7,"received":false}},"notes":[{{"id":"{HEX}","at":1,"by":{{"kind":"admin","id":"{HEX}"}},"text":"by a person"}},{{"id":"{HEX}","at":2,"by":{{"kind":"token","id":"{HEX}","name":"CI"}},"text":"by a token"}}],"history":[{{"at":1,"by":null,"action":"report.swept","detail":""}},{{"at":2,"by":{{"kind":"token","id":"{HEX}","name":"CI"}},"action":"report.status","detail":"new → triaged"}}],"older":true}}"#
            )
        );
    }

    #[test]
    fn the_counts_have_the_designed_shape() {
        let facets: Facets = ReportFacets {
            statuses: vec![Count {
                value: ReportStatus::New,
                count: 1,
            }],
            kinds: Vec::new(),
            priorities: Vec::new(),
            assignees: vec![Count {
                value: None,
                count: 1,
            }],
            labels: Vec::new(),
            gateways: Vec::new(),
            tokens: vec![Count {
                value: NamedToken {
                    id: id(),
                    name: "CI".into(),
                },
                count: 1,
            }],
            channels: Vec::new(),
            reporters: Vec::new(),
            total: 1,
        }
        .into();
        assert_eq!(
            serde_json::to_string(&facets).unwrap(),
            format!(
                r#"{{"statuses":[{{"value":"new","count":1}}],"kinds":[],"priorities":[],"assignees":[{{"value":null,"count":1}}],"labels":[],"gateways":[],"tokens":[{{"value":{{"id":"{HEX}","name":"CI"}},"count":1}}],"channels":[],"reporters":[],"total":1}}"#
            )
        );
        assert_eq!(
            serde_json::to_string(&Filed {
                report_id: Id(id())
            })
            .unwrap(),
            format!(r#"{{"report_id":"{HEX}"}}"#)
        );
    }
}
