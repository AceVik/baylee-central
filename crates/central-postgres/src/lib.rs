//! PostgreSQL 18+ adapter. Development transport is explicitly loopback-only;
//! deployed servers are reached only over verified TLS.
//! Client/policy/crypto libraries do not depend on this server-side crate.
#![forbid(unsafe_code)]

/// Connected database handle shared by the restricted operator import paths.
pub use sea_orm::DatabaseConnection;

mod accounts;
/// Trusted private seasonal campaign provisioning.
pub mod advent_import;
/// Operator-run Scryfall card catalog provisioning.
pub mod cards_import;
mod connection;
mod legacy;
mod pending;
pub use accounts::{
    ALL_STORED, AUDIT_YEARS, AccountStore, Action, Appointed, Approval, AuditPage,
    CHAT_SENDER_SHARE, CLOSED_TICKET_HOURS, Claim, Concerns, Credential, EMAIL_FRESH_MINUTES,
    ERASE_BATCH, Email, Erased, Filer, FilerKind, GATEWAY_STORED, GatewayGrant, GatewayPush,
    GatewayPushResult, ISSUED_RESET_HOURS, ImportedHistory, ImportedNote, ImportedReport, Issued,
    LIVE_CHECK, LegacyMove, MAIL_CODE_MINUTES, MAIL_CODE_TRIES, MAILED_RESET_HOURS,
    MAX_GATEWAY_VERSIONS, MAX_POLLS, MAX_REVOCATION, MAX_SIDS_PER_SESSION, MailedReset,
    OFFICIAL_STORED, OPEN_IN_ALL, OPEN_PER_ACCOUNT, PUSH_ATTEMPTS, PushJob, PushResult,
    REPORT_BYTES, RESET_MAIL_GAP_MINUTES, RESET_MAILS_A_DAY, RESTORE_STEP, REVOCATION_PAGE,
    RankedHostCredential, RecoveryBackupPrep, Redeemed, RefreshOutcome, ReportCaller, ReportImport,
    ReportImportRefusal, ReportToken, RevocationPage, RunnerSession, Session, Status, Summary, Swept, TAG_DRAWS,
    TagDraw, Told, administrators, appoint_admin, erase_account, provision_tower_award_issuer,
    step_revocations, sweep,
};
/// Donations (`docs/operations.md` §"Donations", migration 000032).
pub mod donations {
    pub use super::accounts::{
        CONVERT_DAYS, DONATION_DAYS, Donation, DonationKind, DonationSums, GITHUB_WINDOW_DAYS,
        KOFI_WINDOW_DAYS, MOST_AMOUNT, MOST_EURO_CENTS, Platform, Recorded,
    };
}
/// Profile pictures (`docs/profile-pictures.md`).
pub mod pictures {
    pub use super::accounts::{
        Admission, DECIDED_REPORT_DAYS, Encoding, OwnPicture, PictureCase, PictureFiles,
        REPORTS_OPEN, REPORTS_PER_HOUR, UPLOADS_PER_DAY, UPLOADS_PER_HOUR,
    };
}

pub use connection::{DevConfig, ServerConfig, connect_dev, connect_server, supports_version};
pub use legacy::{
    Candidate, ImportRefusal, Imported, LOCKOUT_FAILURES, LOCKOUT_MINUTES, LegacyRow, LegacyStore,
    LegacySwept, MAX_LEGACY_ROWS, import_legacy, legacy_digest, set_legacy_deadline, sweep_legacy,
};
pub use pending::{PendingAccount, PendingAccountStore};

/// Generic diagnostics: database errors and connection strings never cross this boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StoreError {
    /// Invalid development or server configuration.
    InvalidConfig,
    /// Connection/query/constraint failure, without values or database diagnostics.
    Unavailable,
    /// PostgreSQL older than version 18.
    UnsupportedVersion,
    /// Runtime credentials have excess privileges or the schema is unavailable.
    UnsafeRole,
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidConfig => "invalid database configuration",
            Self::Unavailable => "database operation unavailable",
            Self::UnsupportedVersion => "PostgreSQL 18 or newer is required",
            Self::UnsafeRole => "database runtime role rejected",
        })
    }
}
impl std::error::Error for StoreError {}
