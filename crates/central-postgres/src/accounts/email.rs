//! An account's optional e-mail address (`docs/email.md`, migration 000026):
//! the address a mailed code confirmed, and an address that waits for its
//! code. Every write locks the account's row first, so two requests of one
//! account run one after the other and each reads what the other committed.
//! Nothing here sends mail: the identity service mails a code, and stores
//! its digest only after the mailer took the mail.
use super::{AccountStore, Error, Session, statement, uuid};
use sea_orm::{ConnectionTrait, DatabaseTransaction, TransactionTrait, prelude::Uuid};

/// Minutes a mailed confirmation code lasts: enough to open a mailbox and
/// type eight symbols, and short for a code of 40 bits.
pub const MAIL_CODE_MINUTES: i32 = 15;
/// Wrong codes an address that waits takes; the last of them gives the
/// address up. Five guesses at 40 bits find a code once in 2^37 tries.
pub const MAIL_CODE_TRIES: i16 = 5;
/// Minutes after a password sign-in in which an address may be named. An
/// address can reset the login, so naming one asks nearly what a password
/// change asks: the page proves the password once more right before it
/// names the address, and a session someone took from its owner names
/// none unless it is taken within these minutes of its sign-in. Shorter
/// than the 30 minutes an administrator's change or a gateway token has.
pub const EMAIL_FRESH_MINUTES: i32 = 5;

/// An account's address as stored, for the live session that asked.
pub struct Email {
    /// The session's account.
    pub account: [u8; 16],
    /// The session's password sign-in is less than
    /// [`EMAIL_FRESH_MINUTES`] old.
    pub fresh: bool,
    /// The confirmed address, if the account has one.
    pub address: Option<String>,
    /// When it was confirmed, in Unix seconds.
    pub confirmed_at: Option<i64>,
    /// An address that waits for its code, while the code lasts.
    pub pending: Option<String>,
    /// When that code ends, in Unix seconds.
    pub pending_until: Option<i64>,
}

impl std::fmt::Debug for Email {
    /// Says which parts exist; never an address.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Email")
            .field("fresh", &self.fresh)
            .field("address", &self.address.as_ref().map(|_| "[REDACTED]"))
            .field("pending", &self.pending.as_ref().map(|_| "[REDACTED]"))
            .finish_non_exhaustive()
    }
}

/// A confirmed address to be told of a change: one that was replaced or
/// removed, or the account's own when its login changed.
pub struct Told {
    /// The address.
    pub address: String,
    /// The language its code was asked in.
    pub lang: String,
}

impl std::fmt::Debug for Told {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Told([REDACTED])")
    }
}

/// The enrolled account behind the live `session` and whether its password
/// sign-in is fresh. With `lock` the account's row stays locked until the
/// transaction ends.
async fn holder(
    db: &impl ConnectionTrait,
    session: Session<'_>,
    lock: bool,
) -> Result<(Uuid, bool), Error> {
    let lock = if lock { " FOR UPDATE OF a" } else { "" };
    let row = db
        .query_one_raw(statement(
            &format!(
                "SELECT a.id, COALESCE(s.proven_at, s.created_at) > pg_catalog.clock_timestamp()
                     - pg_catalog.make_interval(mins => $3) AS fresh
                 FROM central_identity.accounts a JOIN central_identity.sessions s
                 ON s.account_id = a.id AND s.credential_revision = a.credential_revision
                 WHERE s.digest = $1 AND s.kind = $2 AND a.credential IS NOT NULL
                 AND s.expires_at > pg_catalog.clock_timestamp(){lock}"
            ),
            vec![
                session.digest().to_vec().into(),
                session.kind().into(),
                EMAIL_FRESH_MINUTES.into(),
            ],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::Rejected)?;
    Ok((
        row.try_get("", "id").map_err(|_| Error::Unavailable)?,
        row.try_get("", "fresh").map_err(|_| Error::Unavailable)?,
    ))
}

/// What is stored for `account`. An address whose code is over is not shown.
async fn held(db: &impl ConnectionTrait, account: Uuid, fresh: bool) -> Result<Email, Error> {
    let row = db
        .query_one_raw(statement(
            "SELECT e.address, floor(extract(epoch FROM e.confirmed_at))::bigint AS confirmed_at,
                 p.address AS pending,
                 floor(extract(epoch FROM p.expires_at))::bigint AS pending_until
             FROM (SELECT $1::uuid AS id) k
             LEFT JOIN central_identity.account_emails e ON e.account_id = k.id
             LEFT JOIN central_identity.account_email_pending p ON p.account_id = k.id
                 AND p.expires_at > pg_catalog.clock_timestamp()",
            vec![account.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::Unavailable)?;
    Ok(Email {
        account: *account.as_bytes(),
        fresh,
        address: row.try_get("", "address").map_err(|_| Error::Unavailable)?,
        confirmed_at: row
            .try_get("", "confirmed_at")
            .map_err(|_| Error::Unavailable)?,
        pending: row.try_get("", "pending").map_err(|_| Error::Unavailable)?,
        pending_until: row
            .try_get("", "pending_until")
            .map_err(|_| Error::Unavailable)?,
    })
}

/// End a reset code that was mailed to the confirmed address of `account`:
/// the mailbox it went to is about to lose its place. An administrator's
/// code stays.
async fn end_mailed_reset(tx: &DatabaseTransaction, account: Uuid) -> Result<(), Error> {
    tx.execute_raw(statement(
        "UPDATE central_identity.accounts a SET reset_digest = NULL, reset_expires = NULL
         FROM central_identity.account_emails e
         WHERE a.id = $1 AND e.account_id = a.id AND a.reset_digest = e.reset_digest",
        vec![account.into()],
    ))
    .await
    .map_err(|_| Error::Unavailable)?;
    Ok(())
}

/// The confirmed address of `account` and its language, if it has one. An
/// address that only waits for its code is none. A change to the login
/// reads it in its own transaction, after the account's row is locked, as
/// every write of the address locks it: it is the address the account had
/// when the change was committed.
pub(super) async fn confirmed(
    tx: &DatabaseTransaction,
    account: Uuid,
) -> Result<Option<Told>, Error> {
    tx.query_one_raw(statement(
        "SELECT address, lang FROM central_identity.account_emails WHERE account_id = $1",
        vec![account.into()],
    ))
    .await
    .map_err(|_| Error::Unavailable)?
    .map(|row| {
        Ok(Told {
            address: row.try_get("", "address").map_err(|_| Error::Unavailable)?,
            lang: row.try_get("", "lang").map_err(|_| Error::Unavailable)?,
        })
    })
    .transpose()
}

impl AccountStore {
    /// The address of the live session's account, and whether the session's
    /// password sign-in is fresh.
    ///
    /// # Errors
    /// `Rejected` without a live session of an enrolled account; storage
    /// failure.
    pub async fn email(&self, session: Session<'_>) -> Result<Email, Error> {
        let (account, fresh) = holder(&self.db, session, false).await?;
        held(&self.db, account, fresh).await
    }

    /// Make `address` the address that waits for the session's account, with
    /// the `digest` of the code just mailed to it, for
    /// [`MAIL_CODE_MINUTES`]; an address that waited before is replaced and
    /// its code ends. The session's password sign-in must be fresh. A
    /// confirmed address stays in use.
    ///
    /// # Errors
    /// `Rejected` for a malformed address or language, without a live
    /// session and for a sign-in that is not fresh; `Conflict` when
    /// `address` is the account's confirmed address; storage failure.
    pub async fn email_pending(
        &self,
        session: Session<'_>,
        address: &str,
        lang: &str,
        digest: &[u8; 32],
    ) -> Result<Email, Error> {
        central_api::email::address(address)?;
        central_api::email::language(lang)?;
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let (account, fresh) = holder(&tx, session, true).await?;
        if !fresh {
            return Err(Error::Rejected);
        }
        if confirmed(&tx, account)
            .await?
            .is_some_and(|held| held.address == address)
        {
            return Err(Error::Conflict);
        }
        tx.execute_raw(statement(
            "INSERT INTO central_identity.account_email_pending
                 (account_id, address, lang, code_digest, expires_at)
             VALUES ($1, $2, $3, $4,
                 pg_catalog.clock_timestamp() + pg_catalog.make_interval(mins => $5))
             ON CONFLICT (account_id) DO UPDATE SET address = EXCLUDED.address,
                 lang = EXCLUDED.lang, code_digest = EXCLUDED.code_digest,
                 expires_at = EXCLUDED.expires_at, tries = 0",
            vec![
                account.into(),
                address.into(),
                lang.into(),
                digest.to_vec().into(),
                MAIL_CODE_MINUTES.into(),
            ],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        let email = held(&tx, account, fresh).await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(email)
    }

    /// Give `address`, which waits for the session's account, the `digest`
    /// of a new code just mailed to it: the earlier code ends, the new one
    /// lasts [`MAIL_CODE_MINUTES`] and its wrong tries start again.
    ///
    /// # Errors
    /// `Rejected` for a malformed language, without a live session, and
    /// when `address` does not wait any more (withdrawn, replaced or its
    /// code over); storage failure.
    pub async fn email_renew(
        &self,
        session: Session<'_>,
        address: &str,
        lang: &str,
        digest: &[u8; 32],
    ) -> Result<Email, Error> {
        central_api::email::language(lang)?;
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let (account, fresh) = holder(&tx, session, true).await?;
        let renewed = tx
            .execute_raw(statement(
                "UPDATE central_identity.account_email_pending SET lang = $3, code_digest = $4,
                     expires_at = pg_catalog.clock_timestamp()
                         + pg_catalog.make_interval(mins => $5), tries = 0
                 WHERE account_id = $1 AND address = $2
                 AND expires_at > pg_catalog.clock_timestamp()",
                vec![
                    account.into(),
                    address.into(),
                    lang.into(),
                    digest.to_vec().into(),
                    MAIL_CODE_MINUTES.into(),
                ],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        if renewed.rows_affected() != 1 {
            return Err(Error::Rejected);
        }
        let email = held(&tx, account, fresh).await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(email)
    }

    /// Confirm the address that waits for the session's account with the
    /// `digest` of the code the person typed, made for `account`
    /// (`MailCode::digest`). The address becomes the account's address in
    /// one transaction; the address it replaces is returned, to be told,
    /// and a reset code mailed to that address ends. A wrong code is
    /// counted and stays counted: the fifth ([`MAIL_CODE_TRIES`]) deletes
    /// the address that waited.
    ///
    /// # Errors
    /// `Rejected` without a live session, for another account's digest,
    /// when no address waits or its code is over, and for a wrong code;
    /// storage failure.
    pub async fn email_confirm(
        &self,
        session: Session<'_>,
        account: [u8; 16],
        digest: &[u8; 32],
    ) -> Result<(Email, Option<Told>), Error> {
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let (held_by, fresh) = holder(&tx, session, true).await?;
        if held_by != uuid(account) {
            return Err(Error::Rejected);
        }
        let row = tx
            .query_one_raw(statement(
                "SELECT address, lang, code_digest = $2 AS matches
                 FROM central_identity.account_email_pending
                 WHERE account_id = $1 AND expires_at > pg_catalog.clock_timestamp()",
                vec![held_by.into(), digest.to_vec().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        if !row
            .try_get::<bool>("", "matches")
            .map_err(|_| Error::Unavailable)?
        {
            Self::wrong_code(&tx, held_by).await?;
            tx.commit().await.map_err(|_| Error::Unavailable)?;
            return Err(Error::Rejected);
        }
        let address: String = row.try_get("", "address").map_err(|_| Error::Unavailable)?;
        let lang: String = row.try_get("", "lang").map_err(|_| Error::Unavailable)?;
        let told = confirmed(&tx, held_by)
            .await?
            .filter(|old| old.address != address);
        end_mailed_reset(&tx, held_by).await?;
        tx.execute_raw(statement(
            "INSERT INTO central_identity.account_emails (account_id, address, lang)
             VALUES ($1, $2, $3)
             ON CONFLICT (account_id) DO UPDATE SET address = EXCLUDED.address,
                 lang = EXCLUDED.lang, confirmed_at = pg_catalog.clock_timestamp(),
                 reset_digest = NULL",
            vec![held_by.into(), address.into(), lang.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        tx.execute_raw(statement(
            "DELETE FROM central_identity.account_email_pending WHERE account_id = $1",
            vec![held_by.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        let email = held(&tx, held_by, fresh).await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok((email, told))
    }

    /// Count a wrong code for the address that waits for `account`; the
    /// last one allowed deletes it.
    async fn wrong_code(tx: &DatabaseTransaction, account: Uuid) -> Result<(), Error> {
        let counted = tx
            .execute_raw(statement(
                "UPDATE central_identity.account_email_pending SET tries = tries + 1
                 WHERE account_id = $1 AND tries < $2",
                vec![account.into(), (MAIL_CODE_TRIES - 1).into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        if counted.rows_affected() == 0 {
            tx.execute_raw(statement(
                "DELETE FROM central_identity.account_email_pending WHERE account_id = $1",
                vec![account.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        }
        Ok(())
    }

    /// Give up the address that waits for the session's account, if one
    /// does; the confirmed address stays.
    ///
    /// # Errors
    /// `Rejected` without a live session; storage failure.
    pub async fn email_withdraw(&self, session: Session<'_>) -> Result<Email, Error> {
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let (account, fresh) = holder(&tx, session, true).await?;
        tx.execute_raw(statement(
            "DELETE FROM central_identity.account_email_pending WHERE account_id = $1",
            vec![account.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        let email = held(&tx, account, fresh).await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(email)
    }

    /// Remove the confirmed address of the session's account and one that
    /// waits. The removed address is returned, to be told, and a reset code
    /// mailed to it ends.
    ///
    /// # Errors
    /// `Rejected` without a live session; storage failure.
    pub async fn email_remove(&self, session: Session<'_>) -> Result<(Email, Option<Told>), Error> {
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let (account, fresh) = holder(&tx, session, true).await?;
        let told = confirmed(&tx, account).await?;
        end_mailed_reset(&tx, account).await?;
        for table in ["account_emails", "account_email_pending"] {
            tx.execute_raw(statement(
                &format!("DELETE FROM central_identity.{table} WHERE account_id = $1"),
                vec![account.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        }
        let email = held(&tx, account, fresh).await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok((email, told))
    }
}

/// The operator's sweep, in its transaction: delete the addresses whose
/// code is over, and tidy what the confirmed addresses keep for a reset by
/// mail (a digest whose code is gone, times older than a day). Returns the
/// number of addresses deleted.
pub(super) async fn sweep(tx: &DatabaseTransaction) -> Result<u64, Error> {
    let pending = tx
        .execute_raw(statement(
            "DELETE FROM central_identity.account_email_pending
             WHERE expires_at <= pg_catalog.clock_timestamp()",
            vec![],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .rows_affected();
    tx.execute_raw(statement(
        "UPDATE central_identity.account_emails e SET reset_digest = NULL
         WHERE e.reset_digest IS NOT NULL AND NOT EXISTS (
             SELECT 1 FROM central_identity.accounts a
             WHERE a.id = e.account_id AND a.reset_digest = e.reset_digest)",
        vec![],
    ))
    .await
    .map_err(|_| Error::Unavailable)?;
    tx.execute_raw(statement(
        "UPDATE central_identity.account_emails SET reset_mailed = ARRAY(
             SELECT t FROM pg_catalog.unnest(reset_mailed) t
             WHERE t > pg_catalog.clock_timestamp() - interval '24 hours' ORDER BY t)
         WHERE EXISTS (SELECT 1 FROM pg_catalog.unnest(reset_mailed) t
             WHERE t <= pg_catalog.clock_timestamp() - interval '24 hours')",
        vec![],
    ))
    .await
    .map_err(|_| Error::Unavailable)?;
    Ok(pending)
}
