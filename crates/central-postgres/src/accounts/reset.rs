//! Resetting a login (`docs/account-reset.md`, migration 000013): an
//! administrator's reset code, and its use, which keeps the vault or makes a
//! new one at the next epoch.
use super::admin::{Action, audit};
use super::{
    AccountStore, Credential, Error, Token, identity, number, statement, username, uuid,
    validate_document,
};
use central_api::wire::ResetVault;
use central_crypto::{opaque::server::ServerKeys, vault::WrappedMasterKey};
use sea_orm::{ConnectionTrait, TransactionTrait, prelude::Uuid};

/// An account whose login may be reset: enrolled, its move finished.
const RESETTABLE: &str = "credential IS NOT NULL AND (source = 'registered' OR legacy_closed)";

impl AccountStore {
    /// Make `digest` the reset code of `target` for the administrator behind
    /// `browser`, valid 24 hours, replacing any earlier one, and audit it.
    /// The administrator's password sign-in must be under 30 minutes old;
    /// the account must be enrolled, its move finished, and neither an
    /// administrator's nor the actor's own. Returns the expiry in Unix seconds.
    ///
    /// # Errors
    /// `Rejected` without an administrator's fresh browser session and for a
    /// refused target; storage failure.
    pub async fn issue_reset(
        &self,
        browser: &Token,
        target: [u8; 16],
        digest: &[u8; 32],
    ) -> Result<i64, Error> {
        // Anyone else is refused before any lock is taken.
        Self::administrator(&self.db, browser).await?;
        let target = Uuid::from_bytes(target);
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        // The administrators' rows, as `set_admin` locks them, so the target
        // cannot become one meanwhile; then the target's own.
        tx.query_all_raw(statement(
            "SELECT a.id FROM central_identity.accounts a JOIN central_identity.account_roles r
             ON r.account_id = a.id WHERE r.role = 'admin' ORDER BY a.id FOR UPDATE OF a",
            vec![],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        let (actor, fresh) = Self::administrator(&tx, browser).await?;
        if !fresh || actor == target {
            return Err(Error::Rejected);
        }
        let row = tx
            .query_one_raw(statement(
                &format!(
                    "UPDATE central_identity.accounts SET reset_digest = $2,
                     reset_expires = pg_catalog.clock_timestamp() + interval '24 hours'
                     WHERE id = $1 AND {RESETTABLE} AND NOT EXISTS (
                         SELECT 1 FROM central_identity.account_roles r
                         WHERE r.account_id = $1 AND r.role = 'admin')
                     RETURNING floor(extract(epoch FROM reset_expires))::bigint AS expires_at"
                ),
                vec![target.into(), digest.to_vec().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        let expires_at = row
            .try_get("", "expires_at")
            .map_err(|_| Error::Unavailable)?;
        audit(&tx, Some(actor), Action::ResetIssued, target).await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(expires_at)
    }

    /// The credential, and the recovery wrapper if any, of the account `name`
    /// whose live reset code has `digest`.
    ///
    /// # Errors
    /// `Rejected` for a malformed name, a wrong or expired code, an account
    /// whose move is not finished; storage failure.
    pub async fn reset_credential(
        &self,
        name: &str,
        digest: &[u8; 32],
    ) -> Result<(Credential, Option<Vec<u8>>), Error> {
        let row = self
            .db
            .query_one_raw(statement(
                &format!(
                    "SELECT id, vault_epoch, credential_revision, credential, recovery_wrapped_master
                     FROM central_identity.accounts WHERE username_key = $1 AND reset_digest = $2
                     AND reset_expires > pg_catalog.clock_timestamp() AND {RESETTABLE}"
                ),
                vec![username(name)?.into(), digest.to_vec().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        Ok((
            Credential {
                identity: identity(&row)?,
                revision: row
                    .try_get("", "credential_revision")
                    .map_err(|_| Error::Unavailable)?,
                record: row
                    .try_get("", "credential")
                    .map_err(|_| Error::Unavailable)?,
            },
            row.try_get("", "recovery_wrapped_master")
                .map_err(|_| Error::Unavailable)?,
        ))
    }

    /// Use the reset code with `digest` of the account behind `credential`:
    /// store `record` and `wrapped`, spend the code, raise the credential
    /// revision and end every session, in one transaction under the account's
    /// lock, with the code, revision and epoch checked again there. With
    /// [`ResetVault::New`] the epoch is raised too, the document replaced at
    /// revision 1, the recovery wrapper replaced (required for a registered
    /// account), the object quota emptied and every private object deleted,
    /// and a registered account's `restored` cleared until a native client
    /// confirms its new document; a moved account keeps it, since its
    /// migration is closed (`account_closed_enrolled`).
    ///
    /// # Errors
    /// `Rejected` for a malformed record, wrapper or document, a registered
    /// account's new vault without a recovery wrapper, and a code spent,
    /// replaced or expired meanwhile; storage failure.
    pub async fn finish_reset(
        &self,
        credential: &Credential,
        digest: &[u8; 32],
        record: &[u8],
        wrapped: &[u8],
        vault: &ResetVault,
    ) -> Result<(), Error> {
        ServerKeys::enrollment_record(record).map_err(|_| Error::Rejected)?;
        WrappedMasterKey::from_bytes(wrapped).map_err(|_| Error::Rejected)?;
        if let ResetVault::New { document, recovery } = vault {
            validate_document(document)?;
            if let Some(recovery) = recovery {
                WrappedMasterKey::from_bytes(recovery).map_err(|_| Error::Rejected)?;
            }
        }
        let account = uuid(credential.identity.account);
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        // One table, so the lock's recheck sees the row's latest version.
        let row = tx
            .query_one_raw(statement(
                &format!(
                    "SELECT source FROM central_identity.accounts WHERE id = $1 AND reset_digest = $2
                     AND reset_expires > pg_catalog.clock_timestamp() AND credential_revision = $3
                     AND vault_epoch = $4 AND {RESETTABLE} FOR UPDATE"
                ),
                vec![
                    account.into(),
                    digest.to_vec().into(),
                    credential.revision.into(),
                    number(credential.identity.epoch)?.into(),
                ],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        let registered = row
            .try_get::<String>("", "source")
            .map_err(|_| Error::Unavailable)?
            == "registered";
        let changed = match vault {
            ResetVault::Keep => tx
                .execute_raw(statement(
                    "UPDATE central_identity.accounts SET credential = $2, wrapped_master = $3,
                     credential_revision = credential_revision + 1,
                     reset_digest = NULL, reset_expires = NULL WHERE id = $1",
                    vec![
                        account.into(),
                        record.to_vec().into(),
                        wrapped.to_vec().into(),
                    ],
                ))
                .await
                .map_err(|_| Error::Unavailable)?,
            ResetVault::New { document, recovery } => {
                // The contract's answer, before the schema's own rule would refuse it.
                if registered && recovery.is_none() {
                    return Err(Error::Rejected);
                }
                tx.execute_raw(statement(
                    "DELETE FROM central_identity.private_objects WHERE account_id = $1",
                    vec![account.into()],
                ))
                .await
                .map_err(|_| Error::Unavailable)?;
                tx.execute_raw(statement(
                    "UPDATE central_identity.accounts SET credential = $2, wrapped_master = $3,
                     credential_revision = credential_revision + 1, vault_epoch = vault_epoch + 1,
                     document = $4, document_revision = 1, recovery_wrapped_master = $5,
                     object_bytes = 0, restored = restored AND source = 'legacy',
                     reset_digest = NULL, reset_expires = NULL WHERE id = $1",
                    vec![
                        account.into(),
                        record.to_vec().into(),
                        wrapped.to_vec().into(),
                        document.clone().into(),
                        recovery.clone().into(),
                    ],
                ))
                .await
                .map_err(|_| Error::Unavailable)?
            }
        };
        if changed.rows_affected() != 1 {
            return Err(Error::Unavailable);
        }
        // Every session ends, of both kinds; their tickets and leases cascade.
        tx.execute_raw(statement(
            "DELETE FROM central_identity.sessions WHERE account_id = $1",
            vec![account.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        tx.commit().await.map_err(|_| Error::Unavailable)
    }
}
