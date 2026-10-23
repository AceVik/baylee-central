//! Restart only unfinished legacy enrollment; closed accounts can never downgrade.
use super::{AccountStore, Error, Identity, Session, identity, number, statement, uuid};

/// A live session's state, as `SessionStatus` reports it.
pub struct Status {
    /// The account and epoch the session belongs to.
    pub identity: Identity,
    /// The account's canonical username.
    pub username: String,
    /// Absolute Unix-seconds expiry; never renewed.
    pub expires_at: i64,
    /// No legacy reset authority exists any more: a closed legacy migration,
    /// or a registered account, which never had one.
    pub migration_closed: bool,
}
use sea_orm::{ConnectionTrait, TransactionTrait};

impl AccountStore {
    /// Validate the exclusive live session without loading its ciphertext or renewing expiry.
    ///
    /// # Errors
    /// Rejects revoked, expired or stale-credential sessions and database failure.
    pub async fn session_status<'a>(
        &self,
        session: impl Into<Session<'a>>,
    ) -> Result<Status, Error> {
        let row = self.db.query_one_raw(statement(
            "SELECT a.id, a.vault_epoch, a.username_key, a.legacy_closed OR a.source = 'registered' AS migration_closed,
             floor(extract(epoch FROM s.expires_at))::bigint AS expires_at
             FROM central_identity.accounts a JOIN central_identity.sessions s
             ON s.account_id = a.id AND s.credential_revision = a.credential_revision
             WHERE s.digest = $1 AND s.expires_at > pg_catalog.clock_timestamp()",
            vec![session.into().digest().to_vec().into()],
        )).await.map_err(|_| Error::Unavailable)?.ok_or(Error::Rejected)?;
        Ok(Status {
            identity: identity(&row)?,
            username: row
                .try_get("", "username_key")
                .map_err(|_| Error::Unavailable)?,
            expires_at: row
                .try_get("", "expires_at")
                .map_err(|_| Error::Unavailable)?,
            migration_closed: row
                .try_get("", "migration_closed")
                .map_err(|_| Error::Unavailable)?,
        })
    }

    /// Invalidate a pending migration before changing its legacy password. Already closed
    /// migrations reject this operation, including when gateway cutover is being retried.
    /// Old ciphertext is archived before any replacement; sessions and capabilities revoke atomically.
    /// A recovery wrapper goes with the vault epoch it was bound to, although
    /// `SetRecovery` already refuses an unfinished move.
    ///
    /// # Errors
    /// Rejects finalized migration, archive capacity, exhausted counters or storage failure.
    pub async fn invalidate_legacy(&self, legacy: &str) -> Result<(), Error> {
        let transaction = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let row = transaction
            .query_one_raw(statement(
                "SELECT id, vault_epoch, legacy_closed, credential IS NOT NULL AS enrolled
             FROM central_identity.accounts WHERE legacy_id = $1 FOR UPDATE",
                vec![legacy.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        if let Some(row) = row {
            if row
                .try_get::<bool>("", "legacy_closed")
                .map_err(|_| Error::Unavailable)?
            {
                return Err(Error::Conflict);
            }
            let account = identity(&row)?.account;
            if row
                .try_get::<bool>("", "enrolled")
                .map_err(|_| Error::Unavailable)?
            {
                let archived = transaction.execute_raw(statement(
                    "INSERT INTO central_identity.migration_archives
                     (account_id, vault_epoch, credential, wrapped_master, document, document_revision)
                     SELECT id, vault_epoch, credential, wrapped_master, document, document_revision
                     FROM central_identity.accounts WHERE id = $1
                     AND (SELECT count(*) FROM central_identity.migration_archives WHERE account_id = $1) < 32",
                    vec![uuid(account).into()],
                )).await.map_err(|_| Error::Unavailable)?;
                if archived.rows_affected() != 1 {
                    return Err(Error::Busy);
                }
            }
            transaction
                .execute_raw(statement(
                    "DELETE FROM central_identity.sessions WHERE account_id = $1",
                    vec![uuid(account).into()],
                ))
                .await
                .map_err(|_| Error::Unavailable)?;
            transaction
                .execute_raw(statement(
                    "UPDATE central_identity.accounts SET credential = NULL, wrapped_master = NULL,
                 recovery_wrapped_master = NULL, document = NULL, document_revision = 0,
                 restored = false, enrollment_digest = NULL, enrollment_expires = NULL,
                 vault_epoch = vault_epoch + 1, credential_revision = credential_revision + 1
                 WHERE id = $1",
                    vec![uuid(account).into()],
                ))
                .await
                .map_err(|_| Error::Unavailable)?;
        }
        transaction.commit().await.map_err(|_| Error::Unavailable)
    }

    /// Permanently close the legacy authorization path after source validation and restore.
    /// The gateway holds its exclusive mutation barrier until local cutover commits.
    /// A lost response may be retried with the same identity; it never reopens enrollment.
    ///
    /// # Errors
    /// Rejects stale identity/epoch, unconfirmed restoration or storage failure.
    pub async fn close_legacy(&self, legacy: &str, expected: Identity) -> Result<(), Error> {
        let changed = self.db.execute_raw(statement(
            "UPDATE central_identity.accounts SET legacy_closed = true
             WHERE legacy_id = $1 AND id = $2 AND vault_epoch = $3 AND restored AND credential IS NOT NULL",
            vec![legacy.into(), uuid(expected.account).into(), number(expected.epoch)?.into()],
        )).await.map_err(|_| Error::Unavailable)?;
        if changed.rows_affected() != 1 {
            return Err(Error::Conflict);
        }
        Ok(())
    }
}
