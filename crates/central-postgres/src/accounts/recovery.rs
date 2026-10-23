//! The account page's reads and writes (`docs/web-accounts.md`): the browser
//! session's account, and a recovery wrapper made after a sign-in for an
//! account that has none, which is every account moved from the official
//! gateway (migration 000010).
use super::{AccountStore, Error, Identity, Session, Token, identity, statement};
use central_api::wire::AccountSource;
use central_crypto::vault::WrappedMasterKey;
use sea_orm::{ConnectionTrait, TransactionTrait, prelude::Uuid};

/// A live session's account, as `AccountSummary` reports it.
pub struct Summary {
    /// The account and epoch the session belongs to.
    pub identity: Identity,
    /// The account's canonical username.
    pub username: String,
    /// Where the account came from.
    pub source: AccountSource,
    /// The account has a recovery wrapper.
    pub recovery: bool,
    /// Absolute Unix seconds of the session's password sign-in; renewal never
    /// moves it.
    pub signed_in_at: i64,
    /// Absolute Unix-seconds expiry; never renewed here.
    pub expires_at: i64,
}

fn source(value: &str) -> Result<AccountSource, Error> {
    match value {
        "legacy" => Ok(AccountSource::Legacy),
        "registered" => Ok(AccountSource::Registered),
        _ => Err(Error::Unavailable),
    }
}

impl AccountStore {
    /// The account of the live browser session `browser` (the website's
    /// cookie), without renewing it. A native bearer is no browser session.
    ///
    /// # Errors
    /// Rejects revoked, expired or stale-credential sessions and a native
    /// bearer, and reports storage failure.
    pub async fn account_summary(&self, browser: &Token) -> Result<Summary, Error> {
        let session = Session::Browser(browser);
        let row = self
            .db
            .query_one_raw(statement(
                "SELECT a.id, a.vault_epoch, a.username_key, a.source,
             a.recovery_wrapped_master IS NOT NULL AS recovery,
             floor(extract(epoch FROM s.created_at))::bigint AS signed_in_at,
             floor(extract(epoch FROM s.expires_at))::bigint AS expires_at
             FROM central_identity.accounts a JOIN central_identity.sessions s
             ON s.account_id = a.id AND s.credential_revision = a.credential_revision
             WHERE s.digest = $1 AND s.kind = $2 AND a.credential IS NOT NULL
             AND s.expires_at > pg_catalog.clock_timestamp()",
                vec![session.digest().to_vec().into(), session.kind().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        let get = |column| {
            row.try_get::<i64>("", column)
                .map_err(|_| Error::Unavailable)
        };
        Ok(Summary {
            identity: identity(&row)?,
            username: row
                .try_get("", "username_key")
                .map_err(|_| Error::Unavailable)?,
            source: source(
                &row.try_get::<String>("", "source")
                    .map_err(|_| Error::Unavailable)?,
            )?,
            recovery: row
                .try_get("", "recovery")
                .map_err(|_| Error::Unavailable)?,
            signed_in_at: get("signed_in_at")?,
            expires_at: get("expires_at")?,
        })
    }

    /// Store `recovery`, the master key wrapped under a recovery code made
    /// after a sign-in, for the account of the live browser session `browser`
    /// (the website's cookie; a native bearer is none). It is stored when the
    /// account is enrolled, has finished moving (its legacy migration closed,
    /// or registered), has no wrapper yet, and the session's password sign-in
    /// is less than 10 minutes old, so a stolen cookie cannot put a wrapper on
    /// an account long after its owner signed in. The same bytes again
    /// succeed at any time, so a retry after a lost answer does; replacing a
    /// wrapper comes with password reset (M1).
    ///
    /// # Errors
    /// `Rejected` for a malformed wrapper, no live browser session, an
    /// unfinished move or a sign-in 10 minutes old or older; `Conflict` for
    /// other bytes while a wrapper exists; `Unavailable` for storage failure.
    pub async fn set_recovery(&self, browser: &Token, recovery: &[u8]) -> Result<(), Error> {
        WrappedMasterKey::from_bytes(recovery).map_err(|_| Error::Rejected)?;
        let session = Session::Browser(browser);
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let row = tx.query_one_raw(statement(
            "SELECT a.id, a.recovery_wrapped_master, a.source = 'registered' OR a.legacy_closed AS moved,
             s.created_at > pg_catalog.clock_timestamp() - interval '10 minutes' AS fresh
             FROM central_identity.accounts a JOIN central_identity.sessions s
             ON s.account_id = a.id AND s.credential_revision = a.credential_revision
             WHERE s.digest = $1 AND s.kind = $2 AND a.credential IS NOT NULL
             AND s.expires_at > pg_catalog.clock_timestamp() FOR UPDATE OF a",
            vec![session.digest().to_vec().into(), session.kind().into()],
        )).await.map_err(|_| Error::Unavailable)?.ok_or(Error::Rejected)?;
        let account: Uuid = row.try_get("", "id").map_err(|_| Error::Unavailable)?;
        let stored: Option<Vec<u8>> = row
            .try_get("", "recovery_wrapped_master")
            .map_err(|_| Error::Unavailable)?;
        if let Some(stored) = stored {
            // A retry of the request that stored it, or another code.
            return if stored == recovery {
                Ok(())
            } else {
                Err(Error::Conflict)
            };
        }
        let moved: bool = row.try_get("", "moved").map_err(|_| Error::Unavailable)?;
        let fresh: bool = row.try_get("", "fresh").map_err(|_| Error::Unavailable)?;
        if !moved || !fresh {
            return Err(Error::Rejected);
        }
        let stored = tx
            .execute_raw(statement(
                "UPDATE central_identity.accounts SET recovery_wrapped_master = $2
                 WHERE id = $1 AND recovery_wrapped_master IS NULL AND credential IS NOT NULL",
                vec![account.into(), recovery.to_vec().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        if stored.rows_affected() != 1 {
            return Err(Error::Unavailable);
        }
        tx.commit().await.map_err(|_| Error::Unavailable)
    }
}
