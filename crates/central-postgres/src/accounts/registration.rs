//! Registration with a beta key (`docs/web-accounts.md`). A key admits a
//! registration while its `uses_left` exceeds its live reservations; the
//! enrollment commit spends the use. Both lock the key's row first.
use super::{AccountStore, Error, Identity, Token, identity, statement, username};
use sea_orm::{ConnectionTrait, TransactionTrait, prelude::Uuid};

/// Abandoned registrations removed by one registration, oldest first.
const SWEEP: i64 = 16;

impl AccountStore {
    /// Reserve a registered account for `name` with the key whose digest is
    /// `key`, bound to `ticket` for 10 minutes. An expired reservation of the
    /// same name is reclaimed; a legacy account's name never is.
    ///
    /// # Errors
    /// `Rejected` for a malformed name or an unknown, revoked, expired or used-up
    /// key, before the name is looked at; `Conflict` for a taken name with a
    /// valid key; `Unavailable` for storage failure.
    pub async fn register(
        &self,
        name: &str,
        key: &[u8; 32],
        ticket: &Token,
    ) -> Result<Identity, Error> {
        let name = username(name)?;
        // A separate statement that waits for no lock: it never holds an account
        // row while waiting for a key, so it cannot deadlock with an enrollment.
        self.db
            .execute_raw(statement(
                "DELETE FROM central_identity.accounts WHERE id IN (SELECT id FROM central_identity.accounts
                 WHERE source = 'registered' AND credential IS NULL
                 AND enrollment_expires <= pg_catalog.clock_timestamp()
                 ORDER BY enrollment_expires LIMIT $1 FOR UPDATE SKIP LOCKED)",
                vec![SWEEP.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let row = tx
            .query_one_raw(statement(
                "SELECT id, uses_left FROM central_identity.beta_keys WHERE digest = $1 AND revoked_at IS NULL
                 AND (expires_at IS NULL OR expires_at > pg_catalog.clock_timestamp()) FOR UPDATE",
                vec![key.to_vec().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        let key_id: Uuid = row.try_get("", "id").map_err(|_| Error::Unavailable)?;
        let uses: i32 = row
            .try_get("", "uses_left")
            .map_err(|_| Error::Unavailable)?;
        let reserved: i64 = tx
            .query_one_raw(statement(
                "SELECT count(*) AS reserved FROM central_identity.accounts
                 WHERE beta_key_id = $1 AND enrollment_expires > pg_catalog.clock_timestamp()",
                vec![key_id.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)?
            .try_get("", "reserved")
            .map_err(|_| Error::Unavailable)?;
        if i64::from(uses) <= reserved {
            return Err(Error::Rejected);
        }
        // Only now is the name looked at.
        tx.execute_raw(statement(
            "DELETE FROM central_identity.accounts WHERE username_key = $1 AND source = 'registered'
             AND credential IS NULL AND enrollment_expires <= pg_catalog.clock_timestamp()",
            vec![name.clone().into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        let row = tx
            .query_one_raw(statement(
                "INSERT INTO central_identity.accounts (source, username_key, enrollment_digest, enrollment_expires, beta_key_id)
                 VALUES ('registered', $1, $2, pg_catalog.clock_timestamp() + interval '10 minutes', $3)
                 ON CONFLICT (username_key) DO NOTHING RETURNING id, vault_epoch",
                vec![name.into(), ticket.digest().to_vec().into(), key_id.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Conflict)?;
        let identity = identity(&row)?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(identity)
    }
}
