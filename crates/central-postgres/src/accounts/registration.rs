//! Open registration (`docs/web-accounts.md`, `docs/registration-abuse.md`):
//! while an administrator has not closed it, anyone may reserve a name; the
//! identity service checks the proof of work before this is reached. The
//! reservation draws the account's tag and holds its display name, so an
//! abandoned registration frees both with its name.
use super::{AccountStore, Error, Identity, Token, identity, statement, username};
use central_api::profile;
use sea_orm::{ConnectionTrait, TransactionTrait};

/// Abandoned registrations removed by one registration, oldest first.
const SWEEP: i64 = 16;

impl AccountStore {
    /// Reserve a registered account for `name`, bound to `ticket` for 10
    /// minutes, with `display_name` (or none, chosen later) and a tag drawn
    /// for it, while registration is open. An expired reservation of the
    /// same name is reclaimed; a legacy account's name never is.
    ///
    /// # Errors
    /// `Rejected` for a malformed name and while registration is closed,
    /// before the name is looked at, and then for a display name a person
    /// may not choose; `Conflict` for a taken name; `Busy` when no tag could
    /// be drawn; `Unavailable` for storage failure.
    pub async fn register(
        &self,
        name: &str,
        display_name: Option<&str>,
        ticket: &Token,
    ) -> Result<Identity, Error> {
        let name = username(name)?;
        // A separate statement that waits for no lock: it never holds an account
        // row while another registration waits, so it cannot deadlock with one.
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
        // Shared with every registration, exclusive with the brake: a
        // registration either sees it open and finishes before it closes,
        // or sees it closed.
        let open: bool = tx
            .query_one_raw(statement(
                "SELECT open FROM central_identity.registration FOR SHARE",
                vec![],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)?
            .try_get("", "open")
            .map_err(|_| Error::Unavailable)?;
        if !open {
            return Err(Error::Rejected);
        }
        // Only now are the names looked at.
        if display_name.is_some_and(|n| !profile::choosable(n)) {
            return Err(Error::Rejected);
        }
        tx.execute_raw(statement(
            "DELETE FROM central_identity.accounts WHERE username_key = $1 AND source = 'registered'
             AND credential IS NULL AND enrollment_expires <= pg_catalog.clock_timestamp()",
            vec![name.clone().into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        let row = self
            .first_free_tag(&tx, None, None, Error::Unavailable, |tag| {
                statement(
                    "INSERT INTO central_identity.accounts (source, username_key, enrollment_digest,
                        enrollment_expires, display_name, display_name_at, tag)
                     SELECT 'registered', $1, $2, now + interval '10 minutes', $3,
                        CASE WHEN $3::text IS NULL THEN NULL ELSE now END, $4
                     FROM (SELECT pg_catalog.clock_timestamp() AS now) clock
                     ON CONFLICT (username_key) DO NOTHING RETURNING id, vault_epoch",
                    vec![
                        name.clone().into(),
                        ticket.digest().to_vec().into(),
                        display_name.into(),
                        tag.into(),
                    ],
                )
            })
            .await?
            .ok_or(Error::Conflict)?;
        let identity = identity(&row)?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(identity)
    }
}

impl AccountStore {
    /// Whether registration is open now.
    ///
    /// # Errors
    /// Storage failure.
    pub async fn registration_open(&self) -> Result<bool, Error> {
        self.db
            .query_one_raw(statement(
                "SELECT open FROM central_identity.registration",
                vec![],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)?
            .try_get("", "open")
            .map_err(|_| Error::Unavailable)
    }
}
