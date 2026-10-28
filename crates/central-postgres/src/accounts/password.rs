//! Password change under a live session (`docs/native-login.md` §"Password
//! change"): the identity service proves the current password with a fresh
//! OPAQUE login; this commits the new record and the same master key wrapped
//! under it, ends every session of the account and issues the caller's next.
//! The vault epoch, documents, objects and the recovery wrapper stay; a reset
//! code still outstanding is cleared, since the current password was just proven.
use super::{AccountStore, Credential, Error, Issued, Session, identity, number, statement, uuid};
use central_crypto::{opaque::server::ServerKeys, vault::WrappedMasterKey};
use sea_orm::{ConnectionTrait, TransactionTrait};

impl AccountStore {
    /// The credential of the live session's account, for the OPAQUE login
    /// that proves the current password. Only an account that has finished
    /// moving changes its password here: while a legacy move is open, the
    /// official gateway's bridge still holds its reset authority.
    ///
    /// # Errors
    /// Rejects an expired, replaced or stale session, a session of the other
    /// kind, an unfinished move, and reports storage failure.
    pub async fn password_credential(&self, session: Session<'_>) -> Result<Credential, Error> {
        let row = self
            .db
            .query_one_raw(statement(
                "SELECT a.id, a.vault_epoch, a.credential_revision, a.credential
             FROM central_identity.accounts a JOIN central_identity.sessions s
             ON s.account_id = a.id AND s.credential_revision = a.credential_revision
             WHERE s.digest = $1 AND s.kind = $2 AND s.expires_at > pg_catalog.clock_timestamp()
             AND a.credential IS NOT NULL AND (a.source = 'registered' OR a.legacy_closed)",
                vec![session.digest().to_vec().into(), session.kind().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        Ok(Credential {
            identity: identity(&row)?,
            revision: row
                .try_get("", "credential_revision")
                .map_err(|_| Error::Unavailable)?,
            record: row
                .try_get("", "credential")
                .map_err(|_| Error::Unavailable)?,
        })
    }

    /// Commit a password change whose current password the caller has just
    /// verified against `credential`: store `record` and `wrapped`, raise the
    /// credential revision, end every session of the account (with their
    /// tickets and leases) and issue `next`, of the same kind as `current`,
    /// in one transaction. `current` must still be live and `credential`
    /// still the account's, so a change racing another change or a sign-in
    /// that replaced `current` commits nothing. Central cannot see whether
    /// `wrapped` holds the same master key; the client is trusted with its
    /// own vault's continuity.
    ///
    /// # Errors
    /// Rejects a malformed record or wrapper, a `next` of another kind, a
    /// session no longer live, a credential changed since the proof's start,
    /// an unfinished move, and reports storage failure.
    pub async fn change_password(
        &self,
        current: Session<'_>,
        credential: &Credential,
        record: &[u8],
        wrapped: &[u8],
        next: Session<'_>,
    ) -> Result<Issued, Error> {
        ServerKeys::enrollment_record(record).map_err(|_| Error::Rejected)?;
        WrappedMasterKey::from_bytes(wrapped).map_err(|_| Error::Rejected)?;
        if current.kind() != next.kind() {
            return Err(Error::Rejected);
        }
        let account = uuid(credential.identity.account);
        let epoch = number(credential.identity.epoch)?;
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let lock = |sql| {
            statement(
                sql,
                vec![
                    current.digest().to_vec().into(),
                    current.kind().into(),
                    account.into(),
                    credential.revision.into(),
                    epoch.into(),
                ],
            )
        };
        tx.query_one_raw(lock(
            "SELECT a.id FROM central_identity.accounts a JOIN central_identity.sessions s
             ON s.account_id = a.id AND s.credential_revision = a.credential_revision
             WHERE s.digest = $1 AND s.kind = $2 AND s.expires_at > pg_catalog.clock_timestamp()
             AND a.id = $3 AND a.credential_revision = $4 AND a.vault_epoch = $5
             AND a.credential IS NOT NULL AND (a.source = 'registered' OR a.legacy_closed)
             FOR UPDATE OF a",
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::Rejected)?;
        // After the lock, a fresh snapshot: a sign-in that replaced `current`
        // while this waited has committed, and the lock's own recheck saw only
        // the account row again (`objects::locked_owner` does the same).
        let row = tx
            .query_one_raw(lock(
                "SELECT a.username_key FROM central_identity.sessions s JOIN central_identity.accounts a
                 ON a.id = s.account_id AND a.credential_revision = s.credential_revision
                 WHERE s.digest = $1 AND s.kind = $2 AND s.expires_at > pg_catalog.clock_timestamp()
                 AND s.account_id = $3 AND a.credential_revision = $4 AND a.vault_epoch = $5
                 FOR SHARE OF s",
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        let username: String = row
            .try_get("", "username_key")
            .map_err(|_| Error::Unavailable)?;
        let changed = tx
            .query_one_raw(statement(
                "UPDATE central_identity.accounts SET credential = $2, wrapped_master = $3,
                 credential_revision = credential_revision + 1,
                 last_sign_in_at = pg_catalog.clock_timestamp(),
                 reset_digest = NULL, reset_expires = NULL
                 WHERE id = $1 AND credential_revision = $4 AND credential IS NOT NULL
                 RETURNING credential_revision",
                vec![
                    account.into(),
                    record.to_vec().into(),
                    wrapped.to_vec().into(),
                    credential.revision.into(),
                ],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)?;
        let revision: i64 = changed
            .try_get("", "credential_revision")
            .map_err(|_| Error::Unavailable)?;
        // Every session ends, of both kinds; their tickets and leases cascade.
        tx.execute_raw(statement(
            "DELETE FROM central_identity.sessions WHERE account_id = $1",
            vec![account.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        let inserted = tx.query_one_raw(statement(
            "INSERT INTO central_identity.sessions (digest, account_id, credential_revision, kind, created_at, expires_at)
             SELECT $1, $2, $3, $4, t, t + interval '30 minutes' FROM (SELECT pg_catalog.clock_timestamp() AS t) clock
             RETURNING floor(extract(epoch FROM expires_at))::bigint AS expires_at",
            vec![next.digest().to_vec().into(), account.into(), revision.into(), next.kind().into()],
        )).await.map_err(|_| Error::Unavailable)?.ok_or(Error::Unavailable)?;
        let expires_at = inserted
            .try_get("", "expires_at")
            .map_err(|_| Error::Unavailable)?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(Issued {
            username,
            wrapped_master: wrapped.to_vec(),
            expires_at,
        })
    }
}
