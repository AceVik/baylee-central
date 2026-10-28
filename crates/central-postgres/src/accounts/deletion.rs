//! Deleting an account (`docs/account-deletion.md`, GDPR Art. 17): the
//! identity service proves the password with a fresh OPAQUE login; this
//! deletes the account row, and the foreign keys take every row naming it
//! with it: sessions with their tickets and leases, objects, archives, the
//! gateways it owns, its places on other gateways' lists, its terms and
//! roles. No grant beyond `DELETE` on accounts is needed for that. The
//! audit stays: it names ids only.
use super::{AccountStore, Credential, Error, Session, admin, number, statement, uuid};
use central_api::admin::OwnedGateway;
use sea_orm::{ConnectionTrait, TransactionTrait};

/// The live session `$1` of kind `$2` of an account that can be deleted, as
/// a condition on `accounts a` joined to `sessions s`: enrolled, its move
/// finished, no administrator.
const DELETABLE: &str =
    "s.digest = $1 AND s.kind = $2 AND s.expires_at > pg_catalog.clock_timestamp()
    AND a.credential IS NOT NULL AND (a.source = 'registered' OR a.legacy_closed)
    AND NOT EXISTS (SELECT 1 FROM central_identity.account_roles r
        WHERE r.account_id = a.id AND r.role = 'admin')";

/// The most gateways one account owns (`docs/gateway-registry.md`).
const OWNED: i64 = 8;

impl AccountStore {
    /// The credential of the live session's account, for the OPAQUE login
    /// that proves its password before a deletion, and the gateways the
    /// deletion takes with it.
    ///
    /// # Errors
    /// Rejects an expired, replaced or stale session, a session of the other
    /// kind, an unfinished move and an administrator; reports storage failure.
    pub async fn deletion_start(
        &self,
        session: Session<'_>,
    ) -> Result<(Credential, Vec<OwnedGateway>), Error> {
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let row = tx
            .query_one_raw(statement(
                &format!(
                    "SELECT a.id, a.vault_epoch, a.credential_revision, a.credential
                     FROM central_identity.accounts a JOIN central_identity.sessions s
                     ON s.account_id = a.id AND s.credential_revision = a.credential_revision
                     WHERE {DELETABLE}"
                ),
                vec![session.digest().to_vec().into(), session.kind().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        let credential = Credential {
            identity: super::identity(&row)?,
            revision: row
                .try_get("", "credential_revision")
                .map_err(|_| Error::Unavailable)?,
            record: row
                .try_get("", "credential")
                .map_err(|_| Error::Unavailable)?,
        };
        let gateways = tx
            .query_all_raw(statement(
                "SELECT g.id, g.origin, EXISTS (SELECT 1 FROM central_identity.gateway_approvals p
                     WHERE p.gateway_id = g.id) AS approved
                 FROM central_identity.gateways g WHERE g.owner_id = $1 ORDER BY g.id LIMIT $2",
                vec![uuid(credential.identity.account).into(), OWNED.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .iter()
            .map(admin::owned_gateway)
            .collect::<Result<Vec<_>, Error>>()?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok((credential, gateways))
    }

    /// Delete the account whose password the caller has just verified against
    /// `credential`, with everything naming it, in one transaction. `current`
    /// must still be live and `credential` still the account's, so a deletion
    /// racing a password change, a reset or a sign-in that replaced `current`
    /// deletes nothing; an account that became an administrator meanwhile is
    /// refused. The gateways the account administers move their list's
    /// revision, so their owners reread the list.
    ///
    /// # Errors
    /// Rejects a session no longer live, a credential changed since the
    /// proof's start, an unfinished move and an administrator; reports
    /// storage failure.
    pub async fn delete_account(
        &self,
        current: Session<'_>,
        credential: &Credential,
    ) -> Result<(), Error> {
        let account = uuid(credential.identity.account);
        let epoch = number(credential.identity.epoch)?;
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let values = || {
            vec![
                current.digest().to_vec().into(),
                current.kind().into(),
                account.into(),
                credential.revision.into(),
                epoch.into(),
            ]
        };
        tx.query_one_raw(statement(
            "SELECT a.id FROM central_identity.accounts a JOIN central_identity.sessions s
             ON s.account_id = a.id AND s.credential_revision = a.credential_revision
             WHERE s.digest = $1 AND s.kind = $2 AND s.expires_at > pg_catalog.clock_timestamp()
             AND a.id = $3 AND a.credential_revision = $4 AND a.vault_epoch = $5
             AND a.credential IS NOT NULL AND (a.source = 'registered' OR a.legacy_closed)
             FOR UPDATE OF a",
            values(),
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::Rejected)?;
        // After the lock, a fresh snapshot: a sign-in that replaced `current`
        // or an appointment that committed while this waited is seen; the
        // lock's own recheck saw only the account row again.
        tx.query_one_raw(statement(
            &format!(
                "SELECT a.id FROM central_identity.sessions s JOIN central_identity.accounts a
                 ON a.id = s.account_id AND a.credential_revision = s.credential_revision
                 WHERE {DELETABLE} AND a.id = $3 AND a.credential_revision = $4 AND a.vault_epoch = $5
                 FOR SHARE OF s"
            ),
            values(),
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::Rejected)?;
        tx.execute_raw(statement(
            "UPDATE central_identity.gateways SET admin_revision = admin_revision + 1
             WHERE owner_id <> $1 AND id IN (SELECT gateway_id FROM central_identity.gateway_admins
                 WHERE account_id = $1)",
            vec![account.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        let deleted = tx
            .execute_raw(statement(
                "DELETE FROM central_identity.accounts WHERE id = $1 AND credential_revision = $2",
                vec![account.into(), credential.revision.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .rows_affected();
        if deleted != 1 {
            return Err(Error::Unavailable);
        }
        tx.commit().await.map_err(|_| Error::Unavailable)
    }
}
