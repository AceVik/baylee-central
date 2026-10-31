//! The account side of a legacy move (`docs/native-login.md` §"Legacy
//! login"): what a session's account is in its move, how far each move in the
//! legacy store got, and the runtime role's half of the legacy store's
//! separation. The store itself is the legacy role's (`crate::legacy`).
use super::{AccountStore, Error, Identity, Session, Token, identity, statement};
use central_api::legacy::LegacyProgress;
use sea_orm::ConnectionTrait;

/// A live session's account, as a legacy move sees it.
pub struct LegacyMove {
    /// The account and its epoch.
    pub identity: Identity,
    /// Its id at the official gateway; `None` for a registered account.
    pub legacy_id: Option<String>,
    /// Its move closed already.
    pub closed: bool,
    /// A client confirmed its restored document.
    pub restored: bool,
}

impl AccountStore {
    /// The session's account as a move sees it.
    ///
    /// # Errors
    /// Rejects an expired, replaced or unknown session, and reports
    /// unavailable storage.
    pub async fn legacy_move<'a>(
        &self,
        session: impl Into<Session<'a>>,
    ) -> Result<LegacyMove, Error> {
        let row = self
            .db
            .query_one_raw(statement(
                "SELECT a.id, a.vault_epoch, a.legacy_id, a.legacy_closed, a.restored
                 FROM central_identity.accounts a JOIN central_identity.sessions s
                 ON s.account_id = a.id AND s.credential_revision = a.credential_revision
                 WHERE s.digest = $1 AND s.expires_at > pg_catalog.clock_timestamp()
                     AND a.credential IS NOT NULL",
                vec![session.into().digest().to_vec().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        Ok(LegacyMove {
            identity: identity(&row)?,
            legacy_id: row
                .try_get("", "legacy_id")
                .map_err(|_| Error::Unavailable)?,
            closed: row
                .try_get("", "legacy_closed")
                .map_err(|_| Error::Unavailable)?,
            restored: row
                .try_get("", "restored")
                .map_err(|_| Error::Unavailable)?,
        })
    }

    /// Refuse anything but an administrator's live browser session, before
    /// the legacy store is read for the administrator's listing.
    ///
    /// # Errors
    /// `Rejected` without an administrator's live browser session; storage
    /// failure.
    pub async fn require_administrator(&self, browser: &Token) -> Result<(), Error> {
        Self::administrator(&self.db, browser).await.map(|_| ())
    }

    /// How far each of these legacy accounts' moves got; an id without a
    /// reservation (or with one the import made and no sign-in used) is
    /// [`LegacyProgress::Unstarted`].
    ///
    /// # Errors
    /// Reports unavailable storage.
    pub async fn legacy_progress(
        &self,
        legacy_ids: &[String],
    ) -> Result<Vec<LegacyProgress>, Error> {
        let rows = self
            .db
            .query_all_raw(statement(
                "SELECT l.id AS legacy_id,
                     CASE WHEN a.restored THEN 'restored'
                          WHEN a.credential IS NOT NULL THEN 'enrolled'
                          WHEN a.enrollment_digest IS NOT NULL THEN 'reserved'
                          ELSE 'unstarted' END AS progress
                 FROM unnest($1::text[]) AS l(id)
                 LEFT JOIN central_identity.accounts a ON a.legacy_id = l.id",
                vec![legacy_ids.to_vec().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        legacy_ids
            .iter()
            .map(|id| {
                let row = rows
                    .iter()
                    .find(|row| row.try_get::<String>("", "legacy_id").ok().as_ref() == Some(id))
                    .ok_or(Error::Unavailable)?;
                Ok(
                    match row
                        .try_get::<String>("", "progress")
                        .map_err(|_| Error::Unavailable)?
                        .as_str()
                    {
                        "restored" => LegacyProgress::Restored,
                        "enrolled" => LegacyProgress::Enrolled,
                        "reserved" => LegacyProgress::Reserved,
                        _ => LegacyProgress::Unstarted,
                    },
                )
            })
            .collect()
    }

    /// The runtime role's half of the legacy store's separation: no usage of
    /// its schema and no privilege on anything in it. A database without the
    /// schema (before migration 000015) has nothing to keep apart.
    pub(super) async fn legacy_store_closed(&self) -> bool {
        self.db
            .query_one_raw(statement(
                "SELECT NOT EXISTS (
                     SELECT 1 FROM pg_catalog.pg_namespace n WHERE n.nspname = 'central_legacy'
                     AND (pg_catalog.has_schema_privilege(current_user, n.oid, 'USAGE,CREATE')
                         OR EXISTS (SELECT 1 FROM pg_catalog.pg_class c WHERE c.relnamespace = n.oid
                             AND (pg_catalog.has_any_column_privilege(current_user, c.oid,
                                     'SELECT,INSERT,UPDATE,REFERENCES')
                                 OR pg_catalog.has_table_privilege(current_user, c.oid,
                                     'DELETE,TRUNCATE,TRIGGER,MAINTAIN'))))
                 ) AS closed",
                vec![],
            ))
            .await
            .ok()
            .flatten()
            .and_then(|row| row.try_get::<bool>("", "closed").ok())
            == Some(true)
    }
}
