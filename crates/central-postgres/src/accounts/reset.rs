//! Resetting a login (`docs/account-reset.md`, migration 000013): an
//! administrator's reset code or one mailed to the account's confirmed
//! address (`docs/email.md`, migration 000026), and its use, which keeps
//! the vault or makes a new one at the next epoch.
use super::admin::{Action, audit};
use super::email::{Told, confirmed};
use super::{
    AccountStore, Credential, Error, Token, identity, number, statement, username, uuid,
    validate_document,
};
use central_api::wire::ResetVault;
use central_crypto::{opaque::server::ServerKeys, vault::WrappedMasterKey};
use sea_orm::{ConnectionTrait, DatabaseTransaction, TransactionTrait, prelude::Uuid};

/// An account whose login may be reset: enrolled, its move finished.
const RESETTABLE: &str = "credential IS NOT NULL AND (source = 'registered' OR legacy_closed)";

/// Hours an administrator's reset code lasts: it is handed over by a person.
pub const ISSUED_RESET_HOURS: i32 = 24;
/// Hours a mailed reset code lasts: a mail is read soon or not at all, and
/// the mailbox is the weaker place to leave a code.
pub const MAILED_RESET_HOURS: i32 = 1;
/// Minutes between two reset codes mailed for one account, so that a
/// stranger who knows a sign-in name cannot fill its mailbox.
pub const RESET_MAIL_GAP_MINUTES: i32 = 10;
/// Reset codes mailed for one account in 24 hours, for the same reason.
pub const RESET_MAILS_A_DAY: i64 = 3;

/// What a mailed code needs beyond what every reset code needs, as a
/// condition on the `accounts` row: a confirmed address; no administrator's
/// code that still lasts (a code on the row was mailed exactly when the
/// address's row holds its digest too); none mailed in the last
/// [`RESET_MAIL_GAP_MINUTES`] (`$4`) and fewer than [`RESET_MAILS_A_DAY`]
/// (`$5`) in the last 24 hours.
const MAILABLE: &str = "EXISTS (SELECT 1 FROM central_identity.account_emails e
    WHERE e.account_id = accounts.id
    AND (accounts.reset_digest IS NULL
        OR accounts.reset_expires <= pg_catalog.clock_timestamp()
        OR accounts.reset_digest = e.reset_digest)
    AND NOT EXISTS (SELECT 1 FROM pg_catalog.unnest(e.reset_mailed) t
        WHERE t > pg_catalog.clock_timestamp() - pg_catalog.make_interval(mins => $4))
    AND (SELECT count(*) FROM pg_catalog.unnest(e.reset_mailed) t
        WHERE t > pg_catalog.clock_timestamp() - interval '24 hours') < $5)";

/// Who a reset code is made for.
#[derive(Clone, Copy)]
enum Issue {
    /// An administrator, who hands it over.
    Administrator,
    /// The account's confirmed address, by mail.
    Mailed,
}

/// A reset code that waits to be mailed.
pub struct MailedReset {
    /// The account the code resets.
    pub account: [u8; 16],
    /// Its sign-in name, which the mail names.
    pub username: String,
    /// Its confirmed address, which the mail goes to.
    pub address: String,
}

impl std::fmt::Debug for MailedReset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MailedReset([REDACTED])")
    }
}

/// Lock the administrators' rows, as `set_admin` locks them, so that no
/// account becomes one while a code is made for it.
async fn lock_administrators(tx: &DatabaseTransaction) -> Result<(), Error> {
    tx.query_all_raw(statement(
        "SELECT a.id FROM central_identity.accounts a JOIN central_identity.account_roles r
         ON r.account_id = a.id WHERE r.role = 'admin' ORDER BY a.id FOR UPDATE OF a",
        vec![],
    ))
    .await
    .map_err(|_| Error::Unavailable)?;
    Ok(())
}

/// The one statement that makes `digest` the reset code of `target`, for an
/// administrator's code and a mailed one alike, so both refuse the same
/// accounts: one that is not enrolled, one whose move is not finished, an
/// administrator's. A mailed code has [`MAILABLE`] to meet as well. Returns
/// the expiry in Unix seconds, or `None` when the account is refused.
async fn issue(
    tx: &DatabaseTransaction,
    target: Uuid,
    digest: &[u8; 32],
    kind: Issue,
) -> Result<Option<i64>, Error> {
    let mut values = vec![target.into(), digest.to_vec().into()];
    let also = match kind {
        Issue::Administrator => {
            values.push(ISSUED_RESET_HOURS.into());
            "TRUE"
        }
        Issue::Mailed => {
            values.extend([
                MAILED_RESET_HOURS.into(),
                RESET_MAIL_GAP_MINUTES.into(),
                RESET_MAILS_A_DAY.into(),
            ]);
            MAILABLE
        }
    };
    tx.query_one_raw(statement(
        &format!(
            "UPDATE central_identity.accounts SET reset_digest = $2,
             reset_expires = pg_catalog.clock_timestamp() + pg_catalog.make_interval(hours => $3)
             WHERE id = $1 AND {RESETTABLE} AND NOT EXISTS (
                 SELECT 1 FROM central_identity.account_roles r
                 WHERE r.account_id = $1 AND r.role = 'admin')
             AND {also}
             RETURNING floor(extract(epoch FROM reset_expires))::bigint AS expires_at"
        ),
        values,
    ))
    .await
    .map_err(|_| Error::Unavailable)?
    .map(|row| {
        row.try_get("", "expires_at")
            .map_err(|_| Error::Unavailable)
    })
    .transpose()
}

impl AccountStore {
    /// Make `digest` the reset code of `target` for the administrator behind
    /// `browser`, valid [`ISSUED_RESET_HOURS`], replacing any earlier one,
    /// and audit it. The administrator's password sign-in must be under 30
    /// minutes old; the account must be enrolled, its move finished, and
    /// neither an administrator's nor the actor's own. Returns the expiry
    /// in Unix seconds, and the account's confirmed address as it stands
    /// in this transaction, to be told that a code exists.
    ///
    /// # Errors
    /// `Rejected` without an administrator's fresh browser session and for a
    /// refused target; storage failure.
    pub async fn issue_reset(
        &self,
        browser: &Token,
        target: [u8; 16],
        digest: &[u8; 32],
    ) -> Result<(i64, Option<Told>), Error> {
        // Anyone else is refused before any lock is taken.
        Self::administrator(&self.db, browser).await?;
        let target = Uuid::from_bytes(target);
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        // The administrators' rows, so the target cannot become one
        // meanwhile; then the target's own.
        lock_administrators(&tx).await?;
        let (actor, fresh) = Self::administrator(&tx, browser).await?;
        if !fresh || actor == target {
            return Err(Error::Rejected);
        }
        let expires_at = issue(&tx, target, digest, Issue::Administrator)
            .await?
            .ok_or(Error::Rejected)?;
        audit(&tx, Some(actor), Action::ResetIssued, target).await?;
        // The statement above holds the target's row, which every write of
        // its address locks first.
        let told = confirmed(&tx, target).await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok((expires_at, told))
    }

    /// Make `digest` the reset code of the account `name`, to be mailed to
    /// its confirmed address, valid [`MAILED_RESET_HOURS`]. It is made
    /// through the statement an administrator's code is made through, and
    /// so for no account that one is refused for; and only for an account
    /// with a confirmed address, never over an administrator's code that
    /// still lasts, at most once in [`RESET_MAIL_GAP_MINUTES`] and
    /// [`RESET_MAILS_A_DAY`] times in 24 hours. An earlier mailed code is
    /// replaced. `None` when no code was made, whatever the reason: the
    /// caller tells nobody which. Nothing is audited, as nothing is for an
    /// account's own password change: no administrator acted.
    ///
    /// # Errors
    /// Storage failure.
    pub async fn issue_mailed_reset(
        &self,
        name: &str,
        digest: &[u8; 32],
    ) -> Result<Option<MailedReset>, Error> {
        let Ok(name) = username(name) else {
            return Ok(None);
        };
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        lock_administrators(&tx).await?;
        // The account's own row, as every write of its address locks it: the
        // statements below read what such a write committed.
        let Some(row) = tx
            .query_one_raw(statement(
                "SELECT id FROM central_identity.accounts WHERE username_key = $1 FOR UPDATE",
                vec![name.clone().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
        else {
            return Ok(None);
        };
        let account: Uuid = row.try_get("", "id").map_err(|_| Error::Unavailable)?;
        if issue(&tx, account, digest, Issue::Mailed).await?.is_none() {
            return Ok(None);
        }
        let row = tx
            .query_one_raw(statement(
                "UPDATE central_identity.account_emails SET reset_digest = $2,
                 reset_mailed = ARRAY(SELECT t FROM pg_catalog.unnest(reset_mailed) t
                     WHERE t > pg_catalog.clock_timestamp() - interval '24 hours' ORDER BY t)
                     || pg_catalog.clock_timestamp()
                 WHERE account_id = $1 RETURNING address",
                vec![account.into(), digest.to_vec().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)?;
        let address = row.try_get("", "address").map_err(|_| Error::Unavailable)?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(Some(MailedReset {
            account: *account.as_bytes(),
            username: name,
            address,
        }))
    }

    /// Take back the mailed reset code with `digest` of `account`, which no
    /// mail carried after all. A code made since, by anyone, stays. The
    /// time it was made stays counted: an attempt is an attempt.
    ///
    /// # Errors
    /// Storage failure.
    pub async fn withdraw_mailed_reset(
        &self,
        account: [u8; 16],
        digest: &[u8; 32],
    ) -> Result<(), Error> {
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        for sql in [
            "UPDATE central_identity.accounts SET reset_digest = NULL, reset_expires = NULL
             WHERE id = $1 AND reset_digest = $2",
            "UPDATE central_identity.account_emails SET reset_digest = NULL
             WHERE account_id = $1 AND reset_digest = $2",
        ] {
            tx.execute_raw(statement(
                sql,
                vec![uuid(account).into(), digest.to_vec().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        }
        tx.commit().await.map_err(|_| Error::Unavailable)
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
    /// migration is closed (`account_closed_enrolled`). Returns the
    /// account's confirmed address as it stands in this transaction, to be
    /// told of the reset.
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
    ) -> Result<Option<Told>, Error> {
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
                     reset_digest = NULL, reset_expires = NULL,
                     -- A new master key: the key check of the old one is
                     -- stale, and the next password sign-in sets it again.
                     key_check = NULL WHERE id = $1",
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
        // Every session ends, of both kinds; their `sid`s cascade, each one revoked.
        tx.execute_raw(statement(
            "DELETE FROM central_identity.sessions WHERE account_id = $1",
            vec![account.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        // Under the account's row lock, which every write of its address
        // takes first.
        let told = confirmed(&tx, account).await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(told)
    }
}
