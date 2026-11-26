//! What a session of an AI account may ask (`docs/ai-players.md` §2): its
//! runner's operations and nothing else. One match names every operation,
//! down to the ranked, seat and AI requests and the subject of a terms
//! acceptance, so a new one does not compile until it is decided here.
//!
//! Where it is enforced:
//!
//! - An operation that needs Central's terms reads them in one statement
//!   that also says whether the session's account is an AI
//!   ([`IdentityService::require_terms`]): a person pays nothing for it.
//! - An operation that reads no terms asks in a statement of its own
//!   ([`IdentityService::refuse_ai`]), on the native listener only. No AI
//!   account holds a browser session: the store issues none for one, a
//!   browser sign-in finds no credential for it, a refresh only rotates a
//!   browser session that exists, and a pairing begins with one.
//! - The library's operations and a presence report are refused where
//!   they are served, since people make them often: the statement that
//!   finds an object's owner refuses an AI, and a report is kept only for
//!   a push socket, which an AI's session never opens.
//! - What takes no session refuses an AI in the store where it names an
//!   account: discovery, a sign-in and every reset answer an AI as an
//!   unknown name.
use super::{Error, IdentityService, Request, Surface, Token};
use central_api::ranked::{self, ai, replica::SeatRequest};
use central_api::terms::TermsSubject;
use central_postgres::Session;

/// How an operation stands for a session of an AI account.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ForAi {
    /// A runner session may make it: the session's own requests, the
    /// terms, the ranked requests its seats need, admission to a ranked
    /// match, public reads.
    Allowed,
    /// Refused for an AI's session where dispatch reads the session.
    Refused,
    /// Refused for an AI's session where it is served (the module's notes).
    RefusedWhereServed,
    /// Acts on no session's account: it takes no credential or another kind
    /// (a gateway's, a host's, Tower's, the bridge's, a reset code, a
    /// pairing's handle, the runner token itself), or it is public.
    NoSession,
}

impl ForAi {
    /// How `request` stands for a session of an AI account.
    pub(super) const fn of(request: &Request) -> Self {
        match request {
            // The session's own, the terms (Central's and a host's, which
            // the runner accepts for each match), admission to a match.
            Request::SessionStatus
            | Request::RenewSession
            | Request::Logout
            | Request::Terms
            | Request::GatewayTerms { .. }
            | Request::TermsStatus
            | Request::GatewayToken { .. }
            | Request::RankedToken { .. } => Self::Allowed,
            Request::AcceptTerms { subject, .. } => match subject {
                TermsSubject::Central | TermsSubject::Gateway { .. } => Self::Allowed,
            },
            Request::Ranked { request } => Self::ranked(request),
            // Everything an account holds or does of its own: its profile,
            // address, password and recovery, social life, presence, tasks,
            // Tower, gateways, moves, document, export and deletion.
            Request::Advent { .. }
            | Request::TowerToken {}
            | Request::TowerProgress {}
            | Request::TowerAwardExport { .. }
            | Request::Challenges { .. }
            | Request::Notifications { .. }
            | Request::Social { .. }
            | Request::RegisterGateway { .. }
            | Request::SetGatewayAdmins { .. }
            | Request::LegacyExport
            | Request::LegacySourcePage { .. }
            | Request::LegacyFinishArchive { .. }
            | Request::LegacyLink { .. }
            | Request::ReadDocument
            | Request::WriteDocument { .. }
            | Request::ConfirmRestore { .. }
            | Request::AccountSummary
            | Request::SetRecovery { .. }
            | Request::SetDisplayName { .. }
            | Request::ChangePasswordStart { .. }
            | Request::ChangePasswordFinish { .. }
            | Request::Email { .. }
            | Request::Owner { .. }
            | Request::Admin { .. }
            | Request::AccountExport
            | Request::DeleteAccountStart { .. }
            | Request::DeleteAccountFinish { .. }
            | Request::KeyCheckStatus
            | Request::SetKeyCheck { .. }
            | Request::SetPresence { .. }
            | Request::Presence
            | Request::PairStart
            | Request::PairStatus { .. }
            | Request::PairApprove { .. }
            | Request::PairCancel { .. }
            | Request::Picture { .. }
            | Request::ProfileDetails
            | Request::SetProfileDetails { .. }
            | Request::ExportProfileDetails
            | Request::OwnBadges
            | Request::SendRecoveryBackupMail { .. } => Self::Refused,
            Request::CreateObject
            | Request::ReadObject { .. }
            | Request::ListObjects { .. }
            | Request::WriteObject { .. }
            | Request::DeleteObject { .. }
            | Request::ReportPresence { .. } => Self::RefusedWhereServed,
            // Another credential: a gateway's, a host's, Tower's, the
            // bridge's, a browser's refresh token.
            Request::AdventHost { .. }
            | Request::TowerIdentity {}
            | Request::TowerAward { .. }
            | Request::RankedHost { .. }
            | Request::ChallengeHost { .. }
            | Request::RefreshSession
            | Request::LegacyInvalidate { .. }
            | Request::LegacyClose { .. }
            | Request::LegacyReserve { .. }
            | Request::LegacyProfile { .. }
            | Request::LegacyStatus { .. }
            | Request::GatewayHeartbeat { .. }
            | Request::GatewayPulse { .. }
            | Request::GatewayStatus
            | Request::RegisterInvalidate { .. }
            | Request::GatewayRevocations { .. }
            | Request::PublishGatewayTerms { .. }
            // No session yet, or one bound to a code or handle: the store
            // answers an AI's name as an unknown one, refuses every reset of
            // one, and a pairing begins with a browser session.
            | Request::Discover { .. }
            | Request::LoginStart { .. }
            | Request::LoginFinish { .. }
            | Request::AiRunnerSession { .. }
            | Request::LegacyLogin { .. }
            | Request::RegisterChallenge
            | Request::RegisterChallengeFor { .. }
            | Request::RegisterInviteChallenge { .. }
            | Request::Register { .. }
            | Request::EnrollmentStart { .. }
            | Request::EnrollmentFinish { .. }
            | Request::ResetStart { .. }
            | Request::ResetFinish { .. }
            | Request::ResetMail { .. }
            | Request::PairClaim { .. }
            | Request::PairPoll { .. }
            | Request::PairRedeem { .. }
            // Public reads.
            | Request::GatewayDirectory { .. }
            | Request::PublicFigures { .. }
            | Request::PublicFunding {}
            | Request::PublicProfile { .. }
            | Request::FriendInviteInfo { .. } => Self::NoSession,
        }
    }

    /// The ranked requests: the runner's own, the reads, and what a seat
    /// needs to play its match. A person's ticket, a deck preview, the
    /// ranked export and the replicated seat's moves are not its.
    const fn ranked(request: &ranked::Request) -> Self {
        match request {
            ranked::Request::Ai { request } => match request {
                ai::Request::Offer { .. } | ai::Request::Work => Self::Allowed,
            },
            ranked::Request::Replicate { request } => match request {
                SeatRequest::Relay { .. }
                | SeatRequest::RelayRoute { .. }
                | SeatRequest::DeclineHost { .. }
                | SeatRequest::Hosts { .. }
                | SeatRequest::ApproveHosts { .. }
                | SeatRequest::Participants { .. }
                | SeatRequest::Presence { .. }
                | SeatRequest::Status { .. } => Self::Allowed,
                // A person's search; and a replicated match's move, which
                // no AI is ever seated in.
                SeatRequest::Join { .. } | SeatRequest::Submit { .. } => Self::Refused,
            },
            ranked::Request::Status
            | ranked::Request::OwnProfile { .. }
            | ranked::Request::Board { .. }
            | ranked::Request::Profile { .. }
            | ranked::Request::Seasons { .. } => Self::Allowed,
            ranked::Request::Join { .. }
            | ranked::Request::Cancel { .. }
            | ranked::Request::PreviewDeck { .. }
            | ranked::Request::Export { .. } => Self::Refused,
        }
    }
}

impl IdentityService {
    /// Refuse an AI account's session an operation its runner does not
    /// make, where no statement reads the terms for it: one statement of its
    /// own, on the native listener only (the module's notes). A session that
    /// does not stand is the operation's own to refuse, after whatever it
    /// does first.
    pub(super) async fn refuse_ai(
        &self,
        bearer: Option<&Token>,
        surface: Surface,
    ) -> Result<(), Error> {
        let (Surface::Native, Some(token)) = (surface, bearer) else {
            return Ok(());
        };
        match self.store.session_gate(Session::Native(token), None).await {
            Ok(gate) if gate.ai => Err(Error::Rejected),
            Ok(_) | Err(Error::Rejected) => Ok(()),
            Err(error) => Err(error),
        }
    }
}
