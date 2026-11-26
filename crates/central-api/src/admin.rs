//! Central administration (`docs/administration.md`): what an administrator
//! asks and is answered, on the browser listener only, with a browser session
//! whose account holds the administrator role. No answer has a field that
//! could carry a wrapper, a document or any other ciphertext, and three can
//! carry a secret, each answered once to the change that made it and never
//! again: a reset code, a new report token and a new AI runner token.
use crate::Error;
use crate::Token;
use crate::ai_runner::AiRunnerTokenInfo;
use crate::gateway::{GatewayMetadata, GatewayState};
use crate::owner::TokenInfo;
use crate::report_tokens::{ReportScopes, ReportTokenInfo};
use crate::reports::{
    BulkAction, CrashGroup, DayCount, NamedToken, Neighbours, ReportAdmin, ReportChange,
    ReportDetail, ReportFacets, ReportFilter, ReportSort, ReportSummary, SavedView,
};
use crate::reset_code::ResetCode;
use crate::wire::AccountSource;
use serde::{Deserialize, Serialize};

/// Accounts and audit events per page.
pub const PAGE: usize = 50;
/// Audit events and owned gateways shown with one account.
pub const ACCOUNT_LIST: usize = 20;
/// The longest search, in bytes.
pub const MAX_QUERY: usize = 64;

/// An administrator's request, tagged by `action`.
#[derive(Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum AdminRequest {
    /// Accounts, newest first, [`PAGE`] at a time: all, or those whose
    /// username or display name contains the query (without case), or the one
    /// with a `#tag`. Answered with [`AdminAnswer::Accounts`].
    Accounts {
        /// Text or `#tag` ([`AccountQuery::parse`]); none lists all.
        query: Option<String>,
        /// The last account of the page before, for the next.
        before: Option<[u8; 16]>,
    },
    /// One account with its recovery state, the gateways it owns and its
    /// latest audit events. Answered with [`AdminAnswer::Account`].
    Account {
        /// The account's id.
        account: [u8; 16],
    },
    /// Grant or remove the administrator role: a change, so it needs a
    /// password sign-in less than 30 minutes old. Nobody removes their own
    /// role, so the last administrator stays; only an enrolled account is
    /// appointed. The same state again is `Done` and writes nothing.
    SetAdmin {
        /// The account's id.
        account: [u8; 16],
        /// Whether it holds the role afterwards.
        admin: bool,
    },
    /// Make a reset code for an enrolled account whose move is finished and
    /// which is neither an administrator's nor one's own
    /// (`docs/account-reset.md`): a change. A new code replaces the previous
    /// one; issuing ends nothing. Answered with [`AdminAnswer::ResetCode`].
    ResetLogin {
        /// The account's id.
        account: [u8; 16],
    },
    /// Audit events, newest first, [`PAGE`] at a time, of one account or all.
    /// Answered with [`AdminAnswer::Audit`].
    Audit {
        /// Only the events naming this account.
        account: Option<[u8; 16]>,
        /// The last event of the page before, for the next.
        before: Option<[u8; 16]>,
    },
    /// Registered gateways, newest first, [`PAGE`] at a time, all or those
    /// in one state. Answered with [`AdminAnswer::Gateways`].
    Gateways {
        /// Only gateways in this state.
        state: Option<GatewayState>,
        /// The last gateway of the page before, for the next.
        before: Option<[u8; 16]>,
    },
    /// One gateway with its tokens. Answered with [`AdminAnswer::Gateway`].
    Gateway {
        /// The gateway's id.
        gateway: [u8; 16],
    },
    /// Approve (or enable again) a gateway, or disable it: a change. Approval
    /// stays a person's decision after checking outside Central who runs the
    /// origin (`docs/gateway-registry.md`). Disabling hides it from the
    /// directory, stops its credentials and tokens, and ends its players'
    /// sign-ins there. The same state again is `Done` and writes nothing.
    SetGatewayApproval {
        /// The gateway's id.
        gateway: [u8; 16],
        /// Approved and enabled afterwards.
        enabled: bool,
    },
    /// Grant or withdraw an approved gateway's authority to host ranked matches.
    SetRankedVerification {
        /// The registered gateway's id.
        gateway: [u8; 16],
        /// Whether this gateway may host ranked matches.
        verified: bool,
    },
    /// Revoke one gateway token at once: a change. `Done` again for one
    /// revoked already.
    RevokeGatewayToken {
        /// The token's id.
        token: [u8; 16],
    },
    /// Give an enrolled account another tag: a change. The tag stays unique
    /// at Central (`Conflict` when another account holds it), so the handle
    /// `Name#tag` stays unique; gateways take it at the account's next
    /// sign-in there. The same tag again is `Done` and writes nothing.
    SetTag {
        /// The account's id.
        account: [u8; 16],
        /// Its tag afterwards, 1 to [`crate::profile::MAX_TAG`].
        tag: u32,
    },
    /// What is left of the legacy store (`docs/native-login.md` §"Legacy
    /// login"): its deadline and every account not moved yet, without hashes
    /// or snapshots. Answered with [`AdminAnswer::LegacyAccounts`]; refused
    /// when this Central has no legacy store.
    LegacyAccounts,
    /// The administration's first view: how many accounts and gateways
    /// there are, by kind and state. Counts only, no row. Answered with
    /// [`AdminAnswer::Overview`].
    Overview,
    /// A page of reports (`docs/reports.md` §"Triage"), metadata only,
    /// after the sweep. Answered with [`AdminAnswer::Reports`].
    Reports {
        /// Which.
        filter: ReportFilter,
        /// In what order.
        sort: ReportSort,
        /// Newest, largest or last first.
        descending: bool,
        /// Skipped rows.
        offset: u32,
        /// At most [`crate::reports::MAX_PAGE`].
        limit: u32,
    },
    /// Counts per dimension under the filter. Answered with [`AdminAnswer::ReportFacets`].
    ReportFacets {
        /// Which.
        filter: ReportFilter,
    },
    /// One report with its notes and history, without its parts' bytes.
    /// Answered with [`AdminAnswer::Report`].
    Report {
        /// Which.
        report: [u8; 16],
    },
    /// A report's place in a list. Answered with [`AdminAnswer::Neighbours`].
    ReportNeighbours {
        /// Which.
        report: [u8; 16],
        /// The list's filter.
        filter: ReportFilter,
        /// Its order.
        sort: ReportSort,
        /// Its direction.
        descending: bool,
    },
    /// Crashes grouped by signature. Answered with [`AdminAnswer::CrashGroups`].
    CrashGroups {
        /// Which reports.
        filter: ReportFilter,
    },
    /// Reports per UTC day and kind. Answered with [`AdminAnswer::ReportStats`].
    ReportStats,
    /// The administrators a report may be assigned to. Answered with [`AdminAnswer::ReportAdmins`].
    ReportAdmins,
    /// Change one report's triage; `Conflict` when it moved past
    /// `expected_revision`. Answered with [`AdminAnswer::Report`].
    SetReport {
        /// Which.
        report: [u8; 16],
        /// What.
        change: ReportChange,
        /// The revision it was read at, if the page knows it.
        expected_revision: Option<u32>,
    },
    /// Change, trash, restore or purge a selection, in one transaction.
    /// Answered with [`AdminAnswer::Changed`].
    ReportsBulk {
        /// Which, at most [`crate::reports::MAX_BULK`].
        reports: Vec<[u8; 16]>,
        /// What.
        bulk: BulkAction,
    },
    /// Into the trash. Answered with `Done`.
    TrashReport {
        /// Which.
        report: [u8; 16],
    },
    /// Out of the trash. Answered with `Done`.
    RestoreReport {
        /// Which.
        report: [u8; 16],
    },
    /// Deleted for good from the trash. Answered with `Done`.
    PurgeReport {
        /// Which.
        report: [u8; 16],
    },
    /// A note on a report. Answered with [`AdminAnswer::Report`].
    AddReportNote {
        /// Which.
        report: [u8; 16],
        /// Its words, 1 to [`crate::reports::MAX_NOTE`] characters.
        text: String,
    },
    /// A note removed by its author. Answered with [`AdminAnswer::Report`].
    RemoveReportNote {
        /// Which report.
        report: [u8; 16],
        /// Which note.
        note: [u8; 16],
    },
    /// The reading administrator's saved lists. Answered with [`AdminAnswer::ReportViews`].
    ReportViews,
    /// Save a list; a name saved again replaces its query. Answered with [`AdminAnswer::ReportViews`].
    SaveReportView {
        /// Its name.
        name: String,
        /// The list's query string.
        query: String,
    },
    /// Forget a saved list. Answered with [`AdminAnswer::ReportViews`].
    DeleteReportView {
        /// Which.
        view: [u8; 16],
    },
    /// Mark reports opened, or not, for the reading administrator. Answered with `Done`.
    SetReportsSeen {
        /// Which, at most [`crate::reports::MAX_SEEN`].
        reports: Vec<[u8; 16]>,
        /// Opened, or not.
        seen: bool,
    },
    /// The report tokens (`docs/reports.md` §"Report tokens"): the live
    /// ones, and those revoked or expired in the last 30 days. Answered with
    /// [`AdminAnswer::ReportTokens`].
    ReportTokens,
    /// Make a report token: a change that hands out a credential. At most
    /// [`crate::report_tokens::LIVE_TOKENS`] live ones and
    /// [`crate::report_tokens::TOKENS_PER_DAY`] new ones a day (`Busy`).
    /// Answered with [`AdminAnswer::ReportTokenCreated`], the only answer
    /// that carries the token.
    CreateReportToken {
        /// Its name ([`crate::owner::token_name`]).
        name: String,
        /// What it allows.
        scopes: ReportScopes,
        /// How many days it lives, 1 to [`crate::owner::MAX_TOKEN_DAYS`].
        days: u16,
    },
    /// Revoke a report token at once: a change. `Done` again for one
    /// revoked already.
    RevokeReportToken {
        /// The token's id.
        token: [u8; 16],
    },
    /// Whether registration is open (`docs/registration-abuse.md`).
    /// Answered with [`AdminAnswer::Registration`].
    Registration,
    /// Close registration during an attack, or open it again: the brake the
    /// owner kept when the keys went (11.11.2026). Audited. `Done`; the
    /// same state again changes nothing.
    SetRegistration {
        /// Open, or closed.
        open: bool,
    },
    /// Profile pictures' moderation (`docs/profile-pictures.md`).
    Picture {
        /// The action.
        request: crate::picture::PictureAdmin,
    },
    /// Every AI player (`docs/ai-players.md`), at most [`AI_PLAYERS`].
    AiPlayers,
    /// Flag an account as an AI player, change one, or clear the flag
    /// (`None`). Flagging and clearing are refused after the account's first
    /// ranked match; the kind and the profile never change. Audited.
    SetAiPlayer {
        /// Account ID.
        account: [u8; 16],
        /// The settings, or none to clear.
        settings: Option<AiPlayerSettings>,
    },
    /// Switch an AI player on or off. Off takes it out of every new match
    /// at once; its running matches end normally. Audited.
    SetAiEnabled {
        /// Account ID.
        account: [u8; 16],
        /// On or off.
        enabled: bool,
    },
    /// The settings for every AI.
    AiSettings,
    /// Change the settings for every AI. Audited.
    SetAiSettings {
        /// The new settings.
        settings: AiSettings,
    },
    /// Stop every AI at once (`global_seats` to 0). Audited.
    StopAllAi,
    /// The AI runner tokens (`docs/ai-players.md` §2): the live ones, and
    /// those revoked or expired in the last 30 days. Answered with
    /// [`AdminAnswer::AiRunnerTokens`].
    AiRunnerTokens,
    /// Make an AI runner token for a runner machine: a change that hands out
    /// a credential. Every account must be an AI player. At most
    /// [`crate::ai_runner::LIVE_TOKENS`] live ones and
    /// [`crate::ai_runner::TOKENS_PER_DAY`] new ones a day (`Busy`).
    /// Answered with [`AdminAnswer::AiRunnerTokenCreated`], the only answer
    /// that carries the token.
    CreateAiRunnerToken {
        /// Its name ([`crate::owner::token_name`]).
        name: String,
        /// The AI accounts it may play, 1 to
        /// [`crate::ai_runner::MAX_ACCOUNTS`], each once; fixed for its life.
        accounts: Vec<[u8; 16]>,
        /// How many days it lives, 1 to [`crate::owner::MAX_TOKEN_DAYS`].
        days: u16,
    },
    /// Revoke an AI runner token at once, and with it every session it
    /// minted: a change that needs no fresh sign-in, since it only takes
    /// exposure away. Audited. `Done` again for one revoked already.
    RevokeAiRunnerToken {
        /// The token's id.
        token: [u8; 16],
    },
}

/// Most AI players a listing shows.
pub const AI_PLAYERS: usize = 64;

/// What an administrator sets for an AI player.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiPlayerSettings {
    /// The kind of mind; fixed once flagged.
    pub kind: crate::ranked::ai::AiKind,
    /// The runner's key for the mind, such as `house:expert`; fixed once flagged.
    pub profile: String,
    /// The public label.
    pub label: String,
    /// Matches at once, 1 to 16.
    pub max_games: u8,
    /// Games a UTC day; required for a language model.
    pub daily_games: Option<u32>,
    /// The display name, which begins with an AI prefix ([`crate::profile::ai_name`]).
    pub display_name: String,
}

/// The settings for every AI.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiSettings {
    /// Seconds a search waits before an AI may fill it, 30 to 600.
    pub fill_in_after: u16,
    /// Minutes an AI's session must have left before its cap, 30 to 600.
    pub drain_minutes: u16,
    /// AI seats at once, 0 to 16; a match between two AIs takes two.
    pub global_seats: u8,
    /// Slots each host keeps for people, 0 to 64.
    pub host_reserve: u8,
    /// Matches between two AIs at once, 0 to 8.
    pub carousel_games: u8,
    /// Matches between two AIs a UTC day, 0 to 10,000.
    pub carousel_daily: u32,
}

impl AiSettings {
    /// Whether every value is in its range.
    #[must_use]
    pub const fn valid(&self) -> bool {
        self.fill_in_after >= 30
            && self.fill_in_after <= 600
            && self.drain_minutes >= 30
            && self.drain_minutes <= 600
            && self.global_seats <= 16
            && self.host_reserve <= 64
            && self.carousel_games <= 8
            && self.carousel_daily <= 10_000
    }
}

impl AiPlayerSettings {
    /// Whether the settings are in their bounds. The kind must be a known one.
    #[must_use]
    pub fn valid(&self) -> bool {
        self.kind.as_str().is_some()
            && crate::ranked::ai::ranked_profile(&self.profile)
            && crate::ranked::ai::valid_label(&self.label)
            && (1..=16).contains(&self.max_games)
            && self.daily_games.is_none_or(|games| games <= 10_000)
            && (self.kind != crate::ranked::ai::AiKind::Model || self.daily_games.is_some())
            && crate::profile::ai_name(&self.display_name)
    }
}

/// One AI player as the administration lists it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminAiPlayer {
    /// Account ID.
    pub account: [u8; 16],
    /// Display name.
    pub display_name: Option<String>,
    /// Tag.
    pub tag: Option<u32>,
    /// Its settings.
    pub settings: AiPlayerSettings,
    /// Switched on.
    pub enabled: bool,
    /// Matches reserved or playing now.
    pub live_matches: u32,
    /// Matches made for it today (UTC).
    pub games_today: u32,
    /// Formats it offers a deck for now.
    pub offers: Vec<crate::ranked::Format>,
    /// Its last password sign-in, Unix seconds.
    pub last_sign_in: Option<i64>,
    /// When it was flagged, Unix seconds.
    pub set_at: i64,
}

impl AdminRequest {
    /// Whether the request changes anything: a change needs a recent
    /// password sign-in and writes its audit event. Every action is named,
    /// so a new one does not compile until it is decided here. A report's
    /// triage is the administrators' daily work and writes the report's own
    /// history instead (`report_audit`); of it only what cannot be undone,
    /// a purge, asks for the password again ([`Self::fresh`]).
    #[must_use]
    pub const fn changes(&self) -> bool {
        match self {
            Self::SetAdmin { .. }
            | Self::ResetLogin { .. }
            | Self::SetGatewayApproval { .. }
            | Self::SetRankedVerification { .. }
            | Self::RevokeGatewayToken { .. }
            | Self::SetTag { .. }
            | Self::SetReport { .. }
            | Self::ReportsBulk { .. }
            | Self::TrashReport { .. }
            | Self::RestoreReport { .. }
            | Self::PurgeReport { .. }
            | Self::AddReportNote { .. }
            | Self::RemoveReportNote { .. }
            | Self::SaveReportView { .. }
            | Self::DeleteReportView { .. }
            | Self::SetReportsSeen { .. }
            | Self::CreateReportToken { .. }
            | Self::RevokeReportToken { .. }
            | Self::SetRegistration { .. }
            | Self::SetAiPlayer { .. }
            | Self::SetAiEnabled { .. }
            | Self::SetAiSettings { .. }
            | Self::StopAllAi
            | Self::CreateAiRunnerToken { .. }
            | Self::RevokeAiRunnerToken { .. } => true,
            Self::Picture { request } => request.changes(),
            Self::Accounts { .. }
            | Self::Account { .. }
            | Self::Audit { .. }
            | Self::Gateways { .. }
            | Self::Gateway { .. }
            | Self::LegacyAccounts
            | Self::Overview
            | Self::Reports { .. }
            | Self::ReportFacets { .. }
            | Self::Report { .. }
            | Self::ReportNeighbours { .. }
            | Self::CrashGroups { .. }
            | Self::ReportStats
            | Self::ReportAdmins
            | Self::ReportViews
            | Self::ReportTokens
            | Self::Registration
            | Self::AiPlayers
            | Self::AiSettings
            | Self::AiRunnerTokens => false,
        }
    }

    /// Whether the change needs a password sign-in less than 30 minutes
    /// old: every change of the administration's, and of a report's triage
    /// only a purge, which cannot be undone.
    #[must_use]
    pub const fn fresh(&self) -> bool {
        match self {
            Self::SetReport { .. }
            | Self::TrashReport { .. }
            | Self::RestoreReport { .. }
            | Self::AddReportNote { .. }
            | Self::RemoveReportNote { .. }
            | Self::SaveReportView { .. }
            | Self::DeleteReportView { .. }
            | Self::SetReportsSeen { .. }
            // Stopping an AI only takes exposure away (docs/ai-players.md,
            // open question 6): it needs no fresh password, and is audited.
            | Self::SetAiEnabled { enabled: false, .. }
            | Self::StopAllAi
            // So does revoking a runner token (docs/ai-players.md §2).
            | Self::RevokeAiRunnerToken { .. } => false,
            Self::ReportsBulk { bulk, .. } => matches!(bulk, BulkAction::Purge),
            _ => self.changes(),
        }
    }
}

/// A search over accounts.
#[derive(Debug, PartialEq, Eq)]
pub enum AccountQuery {
    /// Part of a username or display name, lowercase.
    Text(String),
    /// A tag, written `#3fa2`.
    Tag(u32),
}

impl AccountQuery {
    /// Read a search: `#` and one to eight hex digits is a tag, anything else
    /// of 1 to [`MAX_QUERY`] bytes of printable ASCII (spaces trimmed) is
    /// text, compared without case.
    ///
    /// # Errors
    /// `Rejected` for an empty, overlong, non-ASCII or control-bearing query
    /// and a malformed tag.
    pub fn parse(query: &str) -> Result<Self, Error> {
        let query = query.trim();
        if query.is_empty()
            || query.len() > MAX_QUERY
            || !query.bytes().all(|b| b.is_ascii_graphic() || b == b' ')
        {
            return Err(Error::Rejected);
        }
        if let Some(hex) = query.strip_prefix('#') {
            if hex.is_empty() || hex.len() > 8 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(Error::Rejected);
            }
            let tag = u32::from_str_radix(hex, 16).map_err(|_| Error::Rejected)?;
            return if tag == 0 || tag > crate::profile::MAX_TAG {
                Err(Error::Rejected)
            } else {
                Ok(Self::Tag(tag))
            };
        }
        Ok(Self::Text(query.to_ascii_lowercase()))
    }
}

/// How far an account got.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountState {
    /// Reserved, not enrolled yet.
    Pending,
    /// Enrolled; its first document not yet confirmed by a native client.
    Enrolled,
    /// A native client restored and confirmed its document.
    Restored,
}

/// One account as an administrator sees it: names and times, nothing encrypted.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminAccount {
    /// The account's id.
    pub id: [u8; 16],
    /// Its canonical username, the private sign-in name.
    pub username: String,
    /// Its display name, if it has one.
    pub display_name: Option<String>,
    /// Its tag, if it has one.
    pub tag: Option<u32>,
    /// Where it came from.
    pub source: AccountSource,
    /// How far it got.
    pub state: AccountState,
    /// When it was reserved, in Unix seconds.
    pub created_at: i64,
    /// Its last password sign-in, in Unix seconds, if any since Central kept it.
    pub last_sign_in_at: Option<i64>,
    /// It holds the administrator role.
    pub admin: bool,
    /// Until when an unused reset code is valid, in Unix seconds, if one is.
    pub reset_until: Option<i64>,
}

/// A gateway an account owns.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedGateway {
    /// The gateway's id.
    pub id: [u8; 16],
    /// Its origin.
    pub origin: String,
    /// An operator approved it.
    pub approved: bool,
}

/// The administrator's distinct decision about ranked hosting.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RankedHosting {
    /// Ordinary gateway registration and approval start here.
    #[default]
    Unverified,
    /// This host may accept ranked assignments.
    Verified,
}

/// Omit an additive default from admin replies for older website bundles.
fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    value == &T::default()
}

/// A registered gateway as an administrator sees it.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminGateway {
    /// The gateway's id.
    pub id: [u8; 16],
    /// Its origin.
    pub origin: String,
    /// Its public metadata.
    pub metadata: GatewayMetadata,
    /// It is the official gateway, the bridge's.
    pub official: bool,
    /// The account that registered it; none for the official gateway.
    pub owner: Option<[u8; 16]>,
    /// That account's display name, if it has one.
    pub owner_name: Option<String>,
    /// That account's tag, if it has one.
    pub owner_tag: Option<u32>,
    /// Whether it is approved.
    pub state: GatewayState,
    /// A Central administrator explicitly trusts this gateway to host ranked matches.
    #[serde(default, skip_serializing_if = "is_default")]
    pub ranked_hosting: RankedHosting,
    /// A heartbeat arrived in the last 90 seconds.
    pub online: bool,
    /// Its last heartbeat, in Unix seconds, if any.
    pub last_seen: Option<i64>,
    /// When it was registered, in Unix seconds.
    pub created_at: i64,
    /// Its live tokens.
    pub live_tokens: u32,
    /// It still has the credential pair of `RegisterGateway`.
    pub credential_pair: bool,
}

/// How many accounts and gateways there are, for the administration's first
/// view. Accounts count once enrolled; gateways are the registered ones,
/// the official gateway not among them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminOverview {
    /// Enrolled accounts.
    pub accounts: u64,
    /// Of them, created in the last seven days.
    pub accounts_week: u64,
    /// Of them, moved from the official gateway.
    pub moved: u64,
    /// Accounts holding the administrator role.
    pub admins: u64,
    /// Registered gateways awaiting approval.
    pub pending: u64,
    /// Registered gateways approved and enabled.
    pub approved: u64,
    /// Registered gateways disabled.
    pub disabled: u64,
    /// Registered gateways heard from in the last 90 seconds.
    pub online: u64,
    /// Gateway tokens neither revoked nor expired.
    pub live_tokens: u64,
}

/// An account an audit page names, as it is called today: the record keeps
/// ids only, the read adds the names (a renamed account shows its new name,
/// a deleted one none).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamedAccount {
    /// The account's id.
    pub id: [u8; 16],
    /// Its display name, if it has one.
    pub display_name: Option<String>,
    /// Its tag, if it has one.
    pub tag: Option<u32>,
}

/// A gateway an audit page names, as it is called today.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamedGateway {
    /// The gateway's id.
    pub id: [u8; 16],
    /// Its name.
    pub name: String,
    /// Its origin.
    pub origin: String,
}

/// One audit event: who, what, and the ids it touched; never a word.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditEvent {
    /// The event's id, ordered by time.
    pub id: [u8; 16],
    /// When, in Unix seconds.
    pub at: i64,
    /// The administrator who acted, or none for the operator.
    pub actor: Option<[u8; 16]>,
    /// What was done, such as `account.admin_granted`.
    pub action: String,
    /// The account it touched, if any.
    pub account: Option<[u8; 16]>,
    /// The gateway it touched, if any.
    pub gateway: Option<[u8; 16]>,
    /// The closed beta's key it touched, if any: history from before
    /// 11.11.2026, when the keys went.
    pub beta_key: Option<[u8; 16]>,
}

/// The report token an audit event made or revoked, as it is called. Beside
/// the events and not in them: the account export holds events too, and the
/// released client reads those field by field (`export/released.rs`).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TokenEvent {
    /// The event's id.
    pub event: [u8; 16],
    /// The token.
    pub token: NamedToken,
}

/// The answer to an [`AdminRequest`] that reads; a change answers `Done`.
#[derive(Serialize, Deserialize)]
#[serde(tag = "answer", rename_all = "snake_case", deny_unknown_fields)]
pub enum AdminAnswer {
    /// A page of accounts.
    Accounts {
        /// At most [`PAGE`], newest first.
        accounts: Vec<AdminAccount>,
        /// Another page follows.
        more: bool,
    },
    /// One account.
    Account {
        /// Its row.
        account: AdminAccount,
        /// It has a recovery wrapper.
        recovery: bool,
        /// The gateways it owns, at most [`ACCOUNT_LIST`].
        gateways: Vec<OwnedGateway>,
        /// Its latest audit events, at most [`ACCOUNT_LIST`], newest first.
        audit: Vec<AuditEvent>,
    },
    /// A page of audit events.
    Audit {
        /// At most [`PAGE`], newest first.
        events: Vec<AuditEvent>,
        /// Another page follows.
        more: bool,
        /// The accounts the page names (actors and subjects) that still
        /// exist, each once.
        accounts: Vec<NamedAccount>,
        /// The gateways the page names that still exist, each once.
        gateways: Vec<NamedGateway>,
        /// The report tokens the page's events made or revoked.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        report_tokens: Vec<TokenEvent>,
    },
    /// A page of gateways.
    Gateways {
        /// At most [`PAGE`], newest first.
        gateways: Vec<AdminGateway>,
        /// Another page follows.
        more: bool,
    },
    /// One gateway.
    Gateway {
        /// Its row.
        gateway: AdminGateway,
        /// Its tokens, newest first, at most [`ACCOUNT_LIST`].
        tokens: Vec<TokenInfo>,
    },
    /// A reset code, shown to the administrator once; Central keeps its digest.
    ResetCode {
        /// The code to hand over.
        code: ResetCode,
        /// Until when it is valid, in Unix seconds.
        expires_at: i64,
    },
    /// What is left of the legacy store.
    LegacyAccounts {
        /// Its deadline and its accounts.
        store: crate::legacy::LegacyStore,
    },
    /// The counts of the administration's first view.
    Overview {
        /// Accounts and gateways by kind and state.
        overview: AdminOverview,
    },
    /// Whether registration is open.
    Registration {
        /// Open: anyone may register, with the proof of work.
        open: bool,
        /// When an administrator last changed it, Unix seconds; none since
        /// it opened with the keys' removal.
        changed_at: Option<i64>,
    },
    /// A page of reports.
    Reports {
        /// At most the page asked for.
        reports: Vec<ReportSummary>,
        /// All the filter matches.
        total: u64,
    },
    /// Counts per dimension.
    ReportFacets {
        /// The counts.
        facets: ReportFacets,
    },
    /// One report.
    Report {
        /// It, boxed: the largest answer by far.
        report: Box<ReportDetail>,
    },
    /// A report's place in a list.
    Neighbours {
        /// Its place.
        neighbours: Neighbours,
    },
    /// Crashes by signature.
    CrashGroups {
        /// At most [`crate::reports::MAX_CRASH_GROUPS`], the latest first.
        groups: Vec<CrashGroup>,
    },
    /// Reports per day and kind.
    ReportStats {
        /// Over [`crate::reports::STATS_DAYS`] days, by day then kind.
        days: Vec<DayCount>,
    },
    /// The administrators.
    ReportAdmins {
        /// Each once.
        admins: Vec<ReportAdmin>,
    },
    /// How many reports a selection changed.
    Changed {
        /// How many.
        changed: u32,
    },
    /// The reading administrator's saved lists.
    ReportViews {
        /// Oldest first.
        views: Vec<SavedView>,
    },
    /// The report tokens.
    ReportTokens {
        /// At most [`crate::report_tokens::LISTED_TOKENS`], the live ones
        /// first, each group newest first.
        tokens: Vec<ReportTokenInfo>,
    },
    /// A report token, shown to the administrator once; Central keeps its
    /// digest.
    ReportTokenCreated {
        /// The token's id.
        id: [u8; 16],
        /// The token: [`crate::report_tokens::token_text`] is the form to show.
        token: Token,
        /// When it stops working, in Unix seconds.
        expires_at: i64,
    },
    /// A page of picture reports.
    PictureReports {
        /// At most [`crate::picture::REPORT_PAGE`], newest first.
        reports: Vec<crate::picture::AdminPictureReport>,
        /// Another page follows.
        more: bool,
    },
    /// One account's picture case.
    PictureCase {
        /// The account as it is called today.
        account: crate::picture::Named,
        /// Its current picture, if any.
        picture: Option<crate::picture::Picture>,
        /// The reports about its pictures, newest first, at most
        /// [`crate::picture::REPORT_PAGE`].
        reports: Vec<crate::picture::AdminPictureReport>,
        /// The last removal's notice, if any.
        notice: Option<crate::picture::PictureNotice>,
        /// An upload block in force, if any.
        block: Option<crate::picture::UploadBlock>,
    },
    /// Every AI player, oldest flag first.
    AiPlayers {
        /// At most [`AI_PLAYERS`].
        players: Vec<AdminAiPlayer>,
    },
    /// The settings for every AI.
    AiSettings {
        /// The settings.
        settings: AiSettings,
        /// When they last changed, Unix seconds.
        changed_at: Option<i64>,
    },
    /// The AI runner tokens.
    AiRunnerTokens {
        /// At most [`crate::ai_runner::LISTED_TOKENS`], the live ones first,
        /// each group newest first.
        tokens: Vec<AiRunnerTokenInfo>,
    },
    /// An AI runner token, shown to the administrator once; Central keeps
    /// its digest.
    AiRunnerTokenCreated {
        /// The token's id.
        id: [u8; 16],
        /// The token: [`crate::ai_runner::token_text`] is the form to show.
        token: Token,
        /// When it stops working, in Unix seconds.
        expires_at: i64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_query_is_text_or_a_tag_and_nothing_unbounded() {
        assert_eq!(
            AccountQuery::parse("  Alice ").unwrap(),
            AccountQuery::Text("alice".into())
        );
        assert_eq!(
            AccountQuery::parse("#3fA2").unwrap(),
            AccountQuery::Tag(0x3fa2)
        );
        assert_eq!(AccountQuery::parse("#7").unwrap(), AccountQuery::Tag(7));
        assert_eq!(
            AccountQuery::parse("50%_x").unwrap(),
            AccountQuery::Text("50%_x".into())
        );
        let long = "a".repeat(MAX_QUERY + 1);
        for bad in [
            "",
            "   ",
            "#",
            "#0",
            "#123456789",
            "#zz",
            "#80000000",
            "tab\there",
            "nul\0",
            "ä",
            long.as_str(),
        ] {
            assert_eq!(AccountQuery::parse(bad), Err(Error::Rejected), "{bad:?}");
        }
        assert!(AccountQuery::parse(&"a".repeat(MAX_QUERY)).is_ok());
    }

    #[test]
    fn the_changes_are_named_and_the_reads_are_not() {
        assert!(
            AdminRequest::SetAdmin {
                account: [0; 16],
                admin: true
            }
            .changes()
        );
        assert!(AdminRequest::ResetLogin { account: [0; 16] }.changes());
        for change in [
            AdminRequest::SetGatewayApproval {
                gateway: [0; 16],
                enabled: false,
            },
            AdminRequest::SetRankedVerification {
                gateway: [0; 16],
                verified: true,
            },
            AdminRequest::RevokeGatewayToken { token: [0; 16] },
            AdminRequest::SetTag {
                account: [0; 16],
                tag: 7,
            },
            AdminRequest::CreateReportToken {
                name: "CI".into(),
                scopes: ReportScopes::ONLY_POST,
                days: 30,
            },
            AdminRequest::RevokeReportToken { token: [0; 16] },
        ] {
            assert!(change.changes() && change.fresh());
        }
        for read in [
            AdminRequest::ReportTokens,
            AdminRequest::Gateways {
                state: Some(GatewayState::Pending),
                before: None,
            },
            AdminRequest::Gateway { gateway: [0; 16] },
            AdminRequest::Accounts {
                query: None,
                before: None,
            },
            AdminRequest::Account { account: [0; 16] },
            AdminRequest::Audit {
                account: None,
                before: None,
            },
        ] {
            assert!(!read.changes());
        }
    }

    #[test]
    fn the_requests_and_answers_have_the_designed_shapes() {
        let request: AdminRequest = crate::decode(
            br#"{"action":"set_admin","account":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],"admin":false}"#,
        )
        .unwrap();
        assert!(matches!(
            request,
            AdminRequest::SetAdmin { admin: false, .. }
        ));
        for bad in [
            br#"{"action":"set_admin","account":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]}"#.as_slice(),
            br#"{"action":"accounts","query":null,"before":null,"role":"admin"}"#,
            br#"{"action":"delete_everything"}"#,
        ] {
            assert!(crate::decode::<AdminRequest>(bad).is_err());
        }
        let answer = AdminAnswer::Audit {
            events: vec![AuditEvent {
                id: [1; 16],
                at: 1_800_000_000,
                actor: None,
                action: "account.admin_granted".into(),
                account: Some([2; 16]),
                gateway: None,
                beta_key: None,
            }],
            more: false,
            accounts: vec![NamedAccount {
                id: [2; 16],
                display_name: Some("Synthetic".into()),
                tag: Some(0x3fa2),
            }],
            gateways: Vec::new(),
            report_tokens: Vec::new(),
        };
        let text = String::from_utf8(crate::encode(&answer).unwrap()).unwrap();
        assert!(text.starts_with(r#"{"answer":"audit","events":[{"id":[1,"#));
        assert!(text.contains(r#""accounts":[{"id":[2,"#));
        assert!(crate::decode::<AdminAnswer>(text.as_bytes()).is_ok());
        // an event without a token reads as it did before there were any
        assert!(!text.contains("report_token"));
        assert!(
            crate::decode::<AdminAnswer>(
                text.replace(r#""beta_key":null"#, r#""beta_key":null,"note":"x""#)
                    .as_bytes()
            )
            .is_err()
        );
        // the overview: counts only, every field named
        let overview = AdminAnswer::Overview {
            overview: AdminOverview {
                accounts: 12,
                accounts_week: 3,
                moved: 4,
                admins: 2,
                pending: 1,
                approved: 5,
                disabled: 0,
                online: 4,
                live_tokens: 7,
            },
        };
        let text = String::from_utf8(crate::encode(&overview).unwrap()).unwrap();
        assert!(text.starts_with(r#"{"answer":"overview","overview":{"accounts":12,"#));
        assert!(
            crate::decode::<AdminAnswer>(
                text.replace(r#""online":4"#, r#""online":4,"rows":[]"#)
                    .as_bytes()
            )
            .is_err()
        );
        let request = crate::decode::<AdminRequest>(br#"{"action":"overview"}"#).unwrap();
        assert!(matches!(request, AdminRequest::Overview) && !request.changes());
    }
    /// A report token's requests, and the one answer that carries a token.
    #[test]
    fn a_report_token_is_asked_for_by_its_rights_and_shown_once() {
        let request: AdminRequest = crate::decode(
            br#"{"action":"create_report_token","name":"CI","scopes":["read","triage"],"days":90}"#,
        )
        .unwrap();
        assert!(matches!(
            request,
            AdminRequest::CreateReportToken { days: 90, scopes, .. } if scopes == ReportScopes::READ_AND_EDIT
        ));
        for bad in [
            br#"{"action":"create_report_token","name":"CI","scopes":["admin"],"days":90}"#.as_slice(),
            br#"{"action":"create_report_token","name":"CI","scopes":["read"]}"#,
            br#"{"action":"create_report_token","name":"CI","scopes":["read"],"days":90,"digest":"x"}"#,
            br#"{"action":"revoke_report_token"}"#,
        ] {
            assert!(crate::decode::<AdminRequest>(bad).is_err());
        }
        let token = Token::generate().unwrap();
        let hex = token.hex();
        let list = AdminAnswer::ReportTokens {
            tokens: vec![ReportTokenInfo {
                id: [1; 16],
                name: "CI".into(),
                scopes: ReportScopes::ONLY_POST,
                created_by: [2; 16],
                created_by_name: Some("Synthetic".into()),
                created_by_tag: Some(0x3fa2),
                suspended: false,
                created_at: 1_800_000_000,
                expires_at: 1_800_086_400,
                revoked_at: None,
                revoked_by: None,
                revoked_by_name: None,
                revoked_by_tag: None,
                last_used_at: None,
            }],
        };
        let text = String::from_utf8(crate::encode(&list).unwrap()).unwrap();
        assert!(text.starts_with(r#"{"answer":"report_tokens","tokens":[{"id":[1,"#));
        assert!(!text.contains("digest") && !text.contains(hex.as_str()));
        assert!(
            crate::decode::<AdminAnswer>(
                text.replace(r#""name":"CI""#, r#""name":"CI","token":"x""#)
                    .as_bytes()
            )
            .is_err()
        );
        let created = AdminAnswer::ReportTokenCreated {
            id: [1; 16],
            token,
            expires_at: 1_800_086_400,
        };
        let text = crate::encode(&created).unwrap();
        let AdminAnswer::ReportTokenCreated { token, .. } = crate::decode(&text).unwrap() else {
            panic!("another answer");
        };
        assert_eq!(token.hex().as_str(), hex.as_str());
        assert!(!format!("{token:?}").contains(hex.as_str()));
    }

    /// An AI runner token's requests: making one is a change that needs a
    /// fresh sign-in, revoking one is a change that needs none, and only the
    /// creation answer carries the token.
    #[test]
    fn a_runner_token_is_made_fresh_revoked_at_once_and_shown_once() {
        let request: AdminRequest = crate::decode(
            br#"{"action":"create_ai_runner_token","name":"runner-1","accounts":[[1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1]],"days":90}"#,
        )
        .unwrap();
        assert!(matches!(
            &request,
            AdminRequest::CreateAiRunnerToken { days: 90, accounts, .. } if accounts == &[[1; 16]]
        ));
        assert!(request.changes() && request.fresh());
        let revoke = AdminRequest::RevokeAiRunnerToken { token: [0; 16] };
        assert!(revoke.changes() && !revoke.fresh());
        let list = AdminRequest::AiRunnerTokens;
        assert!(!list.changes() && !list.fresh());
        for bad in [
            br#"{"action":"create_ai_runner_token","name":"runner-1","days":90}"#.as_slice(),
            br#"{"action":"create_ai_runner_token","name":"runner-1","accounts":[],"days":90,"digest":"x"}"#,
            br#"{"action":"create_ai_runner_token","name":"runner-1","accounts":[[1]],"days":90}"#,
            br#"{"action":"revoke_ai_runner_token"}"#,
        ] {
            assert!(
                crate::decode::<AdminRequest>(bad).is_err(),
                "{}",
                String::from_utf8_lossy(bad)
            );
        }
        let token = Token::generate().unwrap();
        let hex = token.hex();
        let created = AdminAnswer::AiRunnerTokenCreated {
            id: [1; 16],
            token,
            expires_at: 1_807_776_000,
        };
        let text = crate::encode(&created).unwrap();
        assert!(text.starts_with(br#"{"answer":"ai_runner_token_created","id":[1,"#));
        let AdminAnswer::AiRunnerTokenCreated { token, .. } = crate::decode(&text).unwrap() else {
            panic!("another answer");
        };
        assert_eq!(token.hex().as_str(), hex.as_str());
        assert!(!format!("{token:?}").contains(hex.as_str()));
        let listed = AdminAnswer::AiRunnerTokens { tokens: Vec::new() };
        assert_eq!(
            crate::encode(&listed).unwrap(),
            br#"{"answer":"ai_runner_tokens","tokens":[]}"#
        );
    }
}
