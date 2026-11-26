//! Atomic native account persistence. SQL identifiers are fixed; every supplied value is bound.
use crate::{DevConfig, PendingAccountStore, ServerConfig};
use central_api::{
    Error, Token,
    wire::{Identity, MigrationState},
};
use central_crypto::{opaque::server::ServerKeys, vault::WrappedMasterKey};
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DbBackend, QueryResult, Statement, TransactionTrait,
    Value, prelude::Uuid,
};

mod admin;
mod advent;
pub(crate) mod ai;
mod ai_runner;
pub use ai_runner::RunnerSession;
mod challenges;
mod deletion;
mod donations;
mod email;
mod erasure;
mod export;
mod gateways;
mod grants;
mod legacy;
mod live;
pub use live::{Concerns, LIVE_CHECK};
mod migration;
mod notifications;
mod objects;
mod overview;
pub use notifications::{PushJob, PushResult};
mod pairing;
mod password;
mod pictures;
mod profiles;
mod ranked;
pub use ranked::{CLOSED_TICKET_HOURS, RankedHostCredential};
mod recovery;
mod registration;
mod report_tokens;
mod reports;
mod reset;
mod sessions;
mod social;
mod streams;
pub use social::CHAT_SENDER_SHARE;
mod terms;
mod tokens;
mod tower_access;
mod tower_awards;
pub use admin::{AUDIT_YEARS, Action, Appointed, AuditPage, Swept, appoint_admin, sweep};
pub use donations::{
    CONVERT_DAYS, DONATION_DAYS, Donation, DonationKind, DonationSums, GITHUB_WINDOW_DAYS,
    KOFI_WINDOW_DAYS, MOST_AMOUNT, MOST_EURO_CENTS, Platform, Recorded,
};
pub use email::{
    EMAIL_FRESH_MINUTES, Email, MAIL_CODE_MINUTES, MAIL_CODE_TRIES, RecoveryBackupPrep, Told,
};
pub use erasure::{
    ERASE_BATCH, Erased, MAX_REVOCATION, RESTORE_STEP, administrators, erase_account,
    step_revocations,
};
pub use legacy::LegacyMove;
pub use migration::Status;
pub use pairing::{Approval, Claim, MAX_POLLS, OPEN_IN_ALL, OPEN_PER_ACCOUNT, Redeemed};
pub use pictures::{
    Admission, DECIDED_REPORT_DAYS, Encoding, OwnPicture, PictureCase, PictureFiles, REPORTS_OPEN,
    REPORTS_PER_HOUR, UPLOADS_PER_DAY, UPLOADS_PER_HOUR,
};
pub use profiles::{TAG_DRAWS, TagDraw};
pub use recovery::Summary;
pub use report_tokens::ReportToken;
pub use reports::{
    ALL_STORED, Filer, FilerKind, GATEWAY_STORED, ImportedHistory, ImportedNote, ImportedReport,
    OFFICIAL_STORED, REPORT_BYTES, ReportCaller, ReportImport, ReportImportRefusal,
};
pub use reset::{
    ISSUED_RESET_HOURS, MAILED_RESET_HOURS, MailedReset, RESET_MAIL_GAP_MINUTES, RESET_MAILS_A_DAY,
};
pub use sessions::{Issued, RefreshOutcome, Session};
pub use streams::{
    GatewayGrant, GatewayPush, GatewayPushResult, MAX_SIDS_PER_SESSION, PUSH_ATTEMPTS,
    REVOCATION_PAGE, RevocationPage,
};
pub use terms::{MAX_GATEWAY_VERSIONS, SessionGate};
pub use tower_awards::provision_tower_award_issuer;

/// An OPAQUE credential snapshot. It cannot issue sessions without proof verification.
pub struct Credential {
    /// Immutable context, checked against the client's expected account.
    pub identity: Identity,
    /// Credential revision used to prevent a stale login racing a password change.
    pub revision: i64,
    /// Canonical OPAQUE registration upload; never a plaintext password.
    pub record: Vec<u8>,
}

/// Server-only account store. Separate from the DDL/migration entry point.
#[derive(Clone)]
pub struct AccountStore {
    db: DatabaseConnection,
    tags: profiles::Tags,
}

fn statement(sql: &str, values: Vec<Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, values)
}
/// The origins a gateway answers for as one parameter, read back by
/// [`any_origin`]: a validated origin holds no space, so a space divides them.
fn origins<S: AsRef<str>>(set: &[S]) -> Result<Value, Error> {
    let set: Vec<&str> = set.iter().map(AsRef::as_ref).collect();
    if set.is_empty() || set.iter().any(|o| o.is_empty() || o.contains(' ')) {
        return Err(Error::Rejected);
    }
    Ok(set.join(" ").into())
}
/// `ANY(...)` over the [`origins`] parameter number `n`.
fn any_origin(n: u8) -> String {
    format!("ANY(pg_catalog.string_to_array(${n}::text,' '))")
}
/// A registered gateway `g` an administrator has not disabled and not soft-deleted:
/// approved or still pending. A pending gateway signs players in by token
/// (`GatewayState`); only the directory waits for the approval.
const NOT_DISABLED: &str =
    "g.deleted_at IS NULL AND NOT EXISTS(SELECT 1 FROM central_identity.gateway_approvals d
    WHERE d.gateway_id=g.id AND NOT d.enabled)";
fn uuid(id: [u8; 16]) -> Uuid {
    Uuid::from_bytes(id)
}
fn number(value: u64) -> Result<i64, Error> {
    i64::try_from(value).map_err(|_| Error::Rejected)
}
fn identity(row: &QueryResult) -> Result<Identity, Error> {
    let id: Uuid = row.try_get("", "id").map_err(|_| Error::Unavailable)?;
    let epoch: i64 = row
        .try_get("", "vault_epoch")
        .map_err(|_| Error::Unavailable)?;
    let account = *id.as_bytes();
    let epoch = u64::try_from(epoch).map_err(|_| Error::Unavailable)?;
    if account[6] >> 4 != 7 || account[8] >> 6 != 2 || epoch == 0 {
        return Err(Error::Unavailable);
    }
    Ok(Identity { account, epoch })
}

/// Canonical native username. Clients normalize existing NFKC names before this boundary.
pub(crate) fn username(value: &str) -> Result<String, Error> {
    let name = value.to_ascii_lowercase();
    if !(3..=24).contains(&name.len())
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
        || !name.as_bytes()[0].is_ascii_alphanumeric()
        || !name.as_bytes()[name.len() - 1].is_ascii_alphanumeric()
        || name
            .as_bytes()
            .windows(2)
            .any(|p| p.iter().all(|b| !b.is_ascii_alphanumeric()))
    {
        return Err(Error::Rejected);
    }
    Ok(name)
}

impl AccountStore {
    /// Open a restricted development pool; the role must additionally hold native-table CRUD.
    ///
    /// # Errors
    /// Rejects an unsafe role, wrong schema/version or inaccessible database.
    pub async fn connect_dev(config: DevConfig) -> Result<Self, Error> {
        Self::checked(PendingAccountStore::connect_dev(config).await).await
    }

    /// Open a restricted pool to a deployed server over verified TLS, with the same
    /// role and grant contract as [`Self::connect_dev`].
    ///
    /// # Errors
    /// Rejects an unsafe role, wrong schema/version, an unverified session or an
    /// inaccessible database.
    pub async fn connect(config: ServerConfig) -> Result<Self, Error> {
        Self::checked(PendingAccountStore::connect(config).await).await
    }

    async fn checked(
        pending: Result<PendingAccountStore, crate::StoreError>,
    ) -> Result<Self, Error> {
        let db = pending.map_err(|_| Error::Unavailable)?.into_connection();
        let store = Self {
            db,
            tags: profiles::Tags::default(),
        };
        if !store.valid_grants().await || !store.legacy_store_closed().await {
            let _ = store.db.close().await;
            return Err(Error::Unavailable);
        }
        Ok(store)
    }

    /// Find an enrolled account by its canonical private login name. An AI
    /// player's is none, as an unknown name's (`docs/ai-players.md` §2).
    ///
    /// # Errors
    /// Returns generic input or database failures.
    pub async fn discover(&self, name: &str) -> Result<Option<Identity>, Error> {
        self.db.query_one_raw(statement(
            &format!("SELECT id, vault_epoch FROM central_identity.accounts WHERE username_key = $1 AND credential IS NOT NULL
             AND {}", ai::NO_AI),
            vec![username(name)?.into()],
        )).await.map_err(|_| Error::Unavailable)?.as_ref().map(identity).transpose()
    }

    /// Reserve after the bridge has freshly verified legacy ownership. Existing credentials stay intact.
    /// An abandoned registration holding the name gives it up; a live one keeps it.
    /// The bridge's display name and tag are stored with it, and a renewed
    /// reservation of a pending account refreshes them: the name it carries
    /// replaces the one held, and a tag, once held, never changes. A carried
    /// tag another account holds already is replaced by a drawn one.
    ///
    /// # Errors
    /// Rejects conflicting links/names, committed accounts, a malformed
    /// carried name or tag, or inaccessible storage; `Busy` when a taken
    /// tag cannot be replaced.
    pub async fn reserve_legacy(
        &self,
        legacy: &str,
        name: &str,
        display_name: Option<&str>,
        tag: Option<u32>,
        ticket: &Token,
    ) -> Result<Identity, Error> {
        if legacy.is_empty()
            || legacy.len() > 64
            || !legacy
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return Err(Error::Rejected);
        }
        let name = username(name)?;
        let display_name = profiles::carried_name(display_name)?;
        let carried = profiles::carried_tag(tag)?;
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        tx.execute_raw(statement(
            "DELETE FROM central_identity.accounts WHERE username_key = $1 AND source = 'registered'
             AND credential IS NULL AND enrollment_expires <= pg_catalog.clock_timestamp()",
            vec![name.clone().into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        let reserve = |tag: Option<i32>| {
            statement(
                "INSERT INTO central_identity.accounts (source, legacy_id, username_key, enrollment_digest,
                    enrollment_expires, display_name, display_name_at, tag)
                 SELECT 'legacy', $1, $2, $3, now + interval '10 minutes', $4,
                    CASE WHEN $4::text IS NULL THEN NULL ELSE now END, $5
                 FROM (SELECT pg_catalog.clock_timestamp() AS now) clock
                 ON CONFLICT (legacy_id) DO UPDATE SET enrollment_digest = EXCLUDED.enrollment_digest,
                    enrollment_expires = EXCLUDED.enrollment_expires, username_key = EXCLUDED.username_key,
                    display_name = COALESCE(EXCLUDED.display_name, central_identity.accounts.display_name),
                    display_name_at = CASE
                        WHEN EXCLUDED.display_name IS NULL THEN central_identity.accounts.display_name_at
                        WHEN EXCLUDED.display_name = central_identity.accounts.display_name
                            THEN central_identity.accounts.display_name_at
                        ELSE EXCLUDED.display_name_at END,
                    tag = COALESCE(central_identity.accounts.tag, EXCLUDED.tag)
                 WHERE central_identity.accounts.credential IS NULL AND NOT central_identity.accounts.legacy_closed
                 RETURNING id, vault_epoch",
                vec![
                    legacy.into(),
                    name.clone().into(),
                    ticket.digest().to_vec().into(),
                    display_name.into(),
                    tag.into(),
                ],
            )
        };
        let row = match carried {
            // The row being refreshed is this legacy account's, whose own tag is no collision.
            Some(_) => {
                let own: Option<Uuid> = tx
                    .query_one_raw(statement(
                        "SELECT id FROM central_identity.accounts WHERE legacy_id = $1",
                        vec![legacy.into()],
                    ))
                    .await
                    .map_err(|_| Error::Unavailable)?
                    .map(|row| row.try_get("", "id"))
                    .transpose()
                    .map_err(|_| Error::Unavailable)?;
                self.first_free_tag(&tx, own, carried, Error::Conflict, |tag| reserve(Some(tag)))
                    .await?
            }
            // A bridge from before: no tag, and none drawn; it waits for the bridge.
            None => tx
                .query_one_raw(reserve(None))
                .await
                .map_err(|_| Error::Conflict)?,
        }
        .ok_or(Error::Conflict)?;
        let identity = identity(&row)?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(identity)
    }

    /// Read the account bound to a still-valid enrollment capability.
    ///
    /// # Errors
    /// Expired, consumed, missing and malformed tickets all reject generically.
    pub async fn enrollment(&self, ticket: &Token) -> Result<Identity, Error> {
        let row = self
            .db
            .query_one_raw(statement(
                "SELECT id, vault_epoch FROM central_identity.accounts WHERE enrollment_digest = $1
             AND enrollment_expires > pg_catalog.clock_timestamp() AND credential IS NULL",
                vec![ticket.digest().to_vec().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        identity(&row)
    }

    /// Commit credential, wrapper and document together, consuming a valid ticket atomically.
    /// A registered account also commits its recovery wrapper and spends its key's
    /// reserved use; a legacy account takes no recovery wrapper.
    ///
    /// # Errors
    /// Rejects malformed cryptography, a recovery wrapper missing for a registered
    /// or present for a legacy account, invalid/expired capabilities, a key revoked
    /// since the reservation, or concurrent completion.
    pub async fn enroll(
        &self,
        ticket: &Token,
        record: &[u8],
        wrapped: &[u8],
        document: &[u8],
        recovery: Option<&[u8]>,
    ) -> Result<(), Error> {
        self.enroll_concerning(ticket, record, wrapped, document, recovery)
            .await
            .map(|_| ())
    }

    /// Atomically commit first enrollment (`credential`, `wrapped_master`,
    /// encrypted document) and consume any reserved friend invite, returning
    /// live push/presence notifications for any new mutual friendship.
    ///
    /// # Errors
    /// Rejects invalid/expired tickets, malformed cryptographic fields, or blocked/full friend limits.
    pub async fn enroll_concerning(
        &self,
        ticket: &Token,
        record: &[u8],
        wrapped: &[u8],
        document: &[u8],
        recovery: Option<&[u8]>,
    ) -> Result<Concerns, Error> {
        ServerKeys::enrollment_record(record).map_err(|_| Error::Rejected)?;
        WrappedMasterKey::from_bytes(wrapped).map_err(|_| Error::Rejected)?;
        if let Some(recovery) = recovery {
            WrappedMasterKey::from_bytes(recovery).map_err(|_| Error::Rejected)?;
        }
        validate_document(document)?;
        let digest = ticket.digest().to_vec();
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let pending = tx
            .query_one_raw(statement(
                "SELECT source FROM central_identity.accounts WHERE enrollment_digest = $1
                 AND enrollment_expires > pg_catalog.clock_timestamp() AND credential IS NULL",
                vec![digest.clone().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        let source: String = pending
            .try_get("", "source")
            .map_err(|_| Error::Unavailable)?;
        // A registered account enrols with its recovery wrapper, a legacy one
        // without (`docs/web-accounts.md`).
        match (source.as_str(), recovery) {
            ("registered", Some(_)) | ("legacy", None) => {}
            _ => return Err(Error::Rejected),
        }
        let enrolled_row = tx
            .query_one_raw(statement(
                "UPDATE central_identity.accounts SET credential = $2, wrapped_master = $3, document = $4,
                 recovery_wrapped_master = $5, document_revision = 1, enrollment_digest = NULL,
                 enrollment_expires = NULL
                 WHERE enrollment_digest = $1 AND enrollment_expires > pg_catalog.clock_timestamp() AND credential IS NULL
                 AND source = $6
                 RETURNING id, tag",
                vec![
                    digest.into(),
                    record.to_vec().into(),
                    wrapped.to_vec().into(),
                    document.to_vec().into(),
                    recovery.map(<[u8]>::to_vec).into(),
                    source.into(),
                ],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        let account_id: sea_orm::prelude::Uuid = enrolled_row
            .try_get("", "id")
            .map_err(|_| Error::Unavailable)?;
        let account_tag: Option<i32> = enrolled_row
            .try_get("", "tag")
            .map_err(|_| Error::Unavailable)?;
        let concerns = social::invites::finalize_on_enroll(&tx, account_id, account_tag).await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(concerns)
    }

    /// Read a credential for an OPAQUE exchange; the context is checked
    /// independently. An AI player has none to read: a sign-in on one runs
    /// as one on an unknown account does (`docs/ai-players.md` §2).
    ///
    /// # Errors
    /// Rejects stale account/epoch mappings and database failure.
    pub async fn credential(&self, expected: Identity) -> Result<Option<Credential>, Error> {
        let row = self.db.query_one_raw(statement(
            &format!("SELECT id, vault_epoch, credential_revision, credential FROM central_identity.accounts
             WHERE id = $1 AND vault_epoch = $2 AND credential IS NOT NULL AND {}", ai::NO_AI),
            vec![uuid(expected.account).into(), number(expected.epoch)?.into()],
        )).await.map_err(|_| Error::Unavailable)?;
        row.as_ref()
            .map(|row| {
                Ok(Credential {
                    identity: identity(row)?,
                    revision: row
                        .try_get("", "credential_revision")
                        .map_err(|_| Error::Unavailable)?,
                    record: row
                        .try_get("", "credential")
                        .map_err(|_| Error::Unavailable)?,
                })
            })
            .transpose()
    }

    /// Bridge-only durable progress; never trusts a client-supplied account identifier.
    ///
    /// # Errors
    /// Rejects unknown legacy links or storage failure.
    pub async fn legacy_status(&self, legacy: &str) -> Result<(Identity, MigrationState), Error> {
        let row = self.db.query_one_raw(statement(
            "SELECT id, vault_epoch, credential IS NOT NULL AS enrolled, restored FROM central_identity.accounts WHERE legacy_id = $1",
            vec![legacy.into()],
        )).await.map_err(|_| Error::Unavailable)?.ok_or(Error::Rejected)?;
        let enrolled: bool = row
            .try_get("", "enrolled")
            .map_err(|_| Error::Unavailable)?;
        let restored: bool = row
            .try_get("", "restored")
            .map_err(|_| Error::Unavailable)?;
        let state = if restored {
            MigrationState::Restored
        } else if enrolled {
            MigrationState::Enrolled
        } else {
            MigrationState::Pending
        };
        Ok((identity(&row)?, state))
    }

    /// Close this pool explicitly during owned-service cleanup.
    ///
    /// # Errors
    /// Reports generic connection cleanup failure.
    pub async fn close(self) -> Result<(), Error> {
        self.db.close().await.map_err(|_| Error::Unavailable)
    }
}

fn validate_document(document: &[u8]) -> Result<(), Error> {
    if !(41..=central_crypto::vault::MAX_OBJECT_BYTES + 41).contains(&document.len())
        || document[0] != 1
    {
        return Err(Error::Rejected);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_username_has_one_spelling_and_no_look_alikes() {
        for (typed, stored) in [
            ("abc", "abc"),
            ("ABC", "abc"),
            ("a.b-c_d", "a.b-c_d"),
            ("007", "007"),
            ("a1b2c3d4e5f6g7h8i9j0k1l2", "a1b2c3d4e5f6g7h8i9j0k1l2"),
        ] {
            assert_eq!(username(typed).unwrap(), stored, "{typed}");
            // The stored spelling is its own.
            assert_eq!(username(stored).unwrap(), stored, "{typed}");
        }
        for refused in [
            "",
            "ab",
            "a1b2c3d4e5f6g7h8i9j0k1l2m",
            ".abc",
            "abc.",
            "-abc",
            "abc_",
            "a..b",
            "a._b",
            "a-.b",
            "a b",
            " abc",
            "abc ",
            "abc\n",
            "abc\0",
            "a@b",
            "a/b",
            "a%b",
            "abc'--",
            // Letters that look like, or fold to, Latin ones.
            "аbc",
            "ａｂｃ",
            "ſbc",
            "\u{212a}abc",
            "straße",
            "ab\u{301}c",
            "abc\u{200b}",
            "\u{202e}abc",
        ] {
            assert!(username(refused).is_err(), "{refused:?}");
        }
    }

    #[test]
    fn a_document_is_a_version_one_sealed_body_within_its_bound() {
        let most = central_crypto::vault::MAX_OBJECT_BYTES + 41;
        let document = |length: usize, version: u8| {
            let mut bytes = vec![7; length];
            if let Some(first) = bytes.first_mut() {
                *first = version;
            }
            validate_document(&bytes).is_ok()
        };
        for (length, accepted) in [
            (0, false),
            (1, false),
            (40, false),
            (41, true),
            (most, true),
            (most + 1, false),
        ] {
            assert_eq!(document(length, 1), accepted, "{length}");
        }
        for version in [0, 2, 255] {
            assert!(!document(41, version), "{version}");
        }
        assert_eq!(number(i64::MAX.unsigned_abs()).unwrap(), i64::MAX);
        assert!(number(i64::MAX.unsigned_abs() + 1).is_err());
    }
}
