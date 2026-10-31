//! Pairings and the key check (`docs/web-accounts.md` §"Pairing"; migration
//! 000016): a signed-in page brings its master key to Baylee's client, sealed
//! to a key the client made, and Central hands the client a native session
//! only together with that sealed key, once.
//!
//! Central never sees the page's one-time secret or the master key: it keeps
//! the handle's digest, the client's public key, challenge and MAC, the
//! sealed key, and the account's key check, which proves the page's approval.
//! A pairing lives five minutes and is deleted when redeemed, cancelled, or
//! when its browser session ends.
use super::{AccountStore, Error, Identity, Session, Token, identity, statement, uuid};
use sea_orm::{ConnectionTrait, DatabaseTransaction, TransactionTrait, prelude::Uuid};
use subtle::ConstantTimeEq;

/// Pairings one account may have open at once.
pub const OPEN_PER_ACCOUNT: i64 = 3;
/// Pairings open in all, whoever started them.
pub const OPEN_IN_ALL: i64 = 1_000;
/// Polls one pairing answers, about one a second for its five minutes.
pub const MAX_POLLS: i32 = 300;

/// The live session `$1` of kind `$2` and its account, as every other
/// session-bound read here joins it.
const LIVE: &str = "FROM central_identity.accounts a JOIN central_identity.sessions s
     ON s.account_id = a.id AND s.credential_revision = a.credential_revision
     WHERE s.digest = $1 AND s.kind = $2 AND a.credential IS NOT NULL
     AND s.expires_at > pg_catalog.clock_timestamp()";

/// A client's claim, as the page reads it.
pub struct Claim {
    /// The client's X25519 public key.
    pub public_key: [u8; 32],
    /// SHA-256 of the client's verifier.
    pub challenge: [u8; 32],
    /// The claim's MAC under the page's one-time secret.
    pub mac: [u8; 32],
}

/// What an approval is checked against.
pub struct Approval {
    /// The account's key check.
    pub key_check: [u8; 32],
    /// The claim.
    pub claim: Claim,
}

/// A redeemed pairing: the native session issued with it.
pub struct Redeemed {
    /// The account and epoch the seal is bound to.
    pub identity: Identity,
    /// The account's canonical username.
    pub username: String,
    /// The display name other players see; none until the account chose one.
    pub display_name: Option<String>,
    /// The tag beside it; none until it was drawn or carried.
    pub tag: Option<u32>,
    /// The master key sealed to the client.
    pub sealed: Vec<u8>,
    /// Absolute Unix-seconds expiry of the new session.
    pub expires_at: i64,
}

fn bytes<const N: usize>(row: &sea_orm::QueryResult, column: &str) -> Result<[u8; N], Error> {
    let value: Vec<u8> = row.try_get("", column).map_err(|_| Error::Unavailable)?;
    value.try_into().map_err(|_| Error::Unavailable)
}

impl AccountStore {
    /// The account's key check, if it has one.
    ///
    /// # Errors
    /// Rejects a session that is not live; reports storage failure.
    pub async fn key_check<'a>(
        &self,
        session: impl Into<Session<'a>>,
    ) -> Result<Option<[u8; 32]>, Error> {
        let session = session.into();
        let row = self
            .db
            .query_one_raw(statement(
                &format!("SELECT a.key_check {LIVE}"),
                vec![session.digest().to_vec().into(), session.kind().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        let check: Option<Vec<u8>> = row
            .try_get("", "key_check")
            .map_err(|_| Error::Unavailable)?;
        check
            .map(|value| value.try_into().map_err(|_| Error::Unavailable))
            .transpose()
    }

    /// Store the account's key check: once, while none is set, and only
    /// within two minutes of the password proof the session rests on, so a
    /// stolen session (or cookie) cannot plant a check of its own.
    ///
    /// # Errors
    /// `Conflict` when a check is set; `Rejected` for a session that is not
    /// live or whose password proof is older; storage failure.
    pub async fn set_key_check<'a>(
        &self,
        session: impl Into<Session<'a>>,
        check: &[u8; 32],
    ) -> Result<(), Error> {
        let session = session.into();
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let row = tx
            .query_one_raw(statement(
                &format!(
                    "SELECT a.id, a.key_check IS NOT NULL AS set,
                     COALESCE(s.proven_at, s.created_at) > pg_catalog.clock_timestamp() - interval '2 minutes' AS fresh
                     {LIVE} FOR UPDATE OF a"
                ),
                vec![session.digest().to_vec().into(), session.kind().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        let get = |column| {
            row.try_get::<bool>("", column)
                .map_err(|_| Error::Unavailable)
        };
        if get("set")? {
            return Err(Error::Conflict);
        }
        if !get("fresh")? {
            return Err(Error::Rejected);
        }
        let account: Uuid = row.try_get("", "id").map_err(|_| Error::Unavailable)?;
        tx.execute_raw(statement(
            "UPDATE central_identity.accounts SET key_check = $2 WHERE id = $1 AND key_check IS NULL",
            vec![account.into(), check.to_vec().into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        tx.commit().await.map_err(|_| Error::Unavailable)
    }

    /// Start a pairing for the browser session's account, which must have a
    /// key check: at most [`OPEN_PER_ACCOUNT`] open for it and
    /// [`OPEN_IN_ALL`] in all, five minutes long. Pairings past their end are
    /// swept first. Returns the pairing's end, Unix seconds.
    ///
    /// # Errors
    /// `Rejected` for a session that is not live or an account without a key
    /// check; `Busy` past the caps; storage failure.
    pub async fn start_pairing(&self, browser: &Token, handle: &[u8; 32]) -> Result<i64, Error> {
        let session = Session::Browser(browser);
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        tx.execute_raw(statement(
            "DELETE FROM central_identity.pairings WHERE expires_at <= pg_catalog.clock_timestamp()",
            vec![],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        let row = tx
            .query_one_raw(statement(
                &format!(
                    "SELECT a.id, a.vault_epoch FROM central_identity.accounts a
                     WHERE a.key_check IS NOT NULL AND a.id = (SELECT a.id {LIVE})
                     FOR UPDATE OF a"
                ),
                vec![session.digest().to_vec().into(), session.kind().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        let account: Uuid = row.try_get("", "id").map_err(|_| Error::Unavailable)?;
        let counts = tx
            .query_one_raw(statement(
                "SELECT count(*) FILTER (WHERE account_id = $1) AS mine, count(*) AS every
                 FROM central_identity.pairings",
                vec![account.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)?;
        let count = |column| {
            counts
                .try_get::<i64>("", column)
                .map_err(|_| Error::Unavailable)
        };
        if count("mine")? >= OPEN_PER_ACCOUNT || count("every")? >= OPEN_IN_ALL {
            return Err(Error::Busy);
        }
        let inserted = tx
            .query_one_raw(statement(
                "INSERT INTO central_identity.pairings
                 (digest, account_id, session_digest, epoch, proven_at, created_at, expires_at)
                 SELECT $1, s.account_id, s.digest, a.vault_epoch, COALESCE(s.proven_at, s.created_at),
                        t, t + interval '5 minutes'
                 FROM central_identity.sessions s JOIN central_identity.accounts a ON a.id = s.account_id,
                      (SELECT pg_catalog.clock_timestamp() AS t) clock
                 WHERE s.digest = $2 AND s.kind = 'browser'
                 RETURNING floor(extract(epoch FROM expires_at))::bigint AS expires_at",
                vec![handle.to_vec().into(), session.digest().to_vec().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        let expires_at = inserted
            .try_get("", "expires_at")
            .map_err(|_| Error::Unavailable)?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(expires_at)
    }

    /// The pairing's claim, if a client made one; only for the browser
    /// session that started it.
    ///
    /// # Errors
    /// `Rejected` for another session, an unknown or ended pairing; storage.
    pub async fn pairing_claim(
        &self,
        browser: &Token,
        handle: &[u8; 32],
    ) -> Result<Option<Claim>, Error> {
        let session = Session::Browser(browser);
        let row = self
            .db
            .query_one_raw(statement(
                "SELECT p.public_key, p.challenge, p.mac FROM central_identity.pairings p
                 JOIN central_identity.sessions s ON s.digest = p.session_digest
                 WHERE p.digest = $1 AND s.digest = $2 AND s.kind = 'browser'
                 AND s.expires_at > pg_catalog.clock_timestamp()
                 AND p.expires_at > pg_catalog.clock_timestamp()",
                vec![handle.to_vec().into(), session.digest().to_vec().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        let key: Option<Vec<u8>> = row
            .try_get("", "public_key")
            .map_err(|_| Error::Unavailable)?;
        if key.is_none() {
            return Ok(None);
        }
        Ok(Some(Claim {
            public_key: bytes(&row, "public_key")?,
            challenge: bytes(&row, "challenge")?,
            mac: bytes(&row, "mac")?,
        }))
    }

    /// What an approval of this claimed pairing is checked against: the
    /// account's key check at the pairing's epoch, and the claim. Only for
    /// the browser session that started it, before it was approved.
    ///
    /// # Errors
    /// `Rejected` for anything else (another session, no claim, approved
    /// already, ended, an account at another epoch); storage failure.
    pub async fn pairing_for_approval(
        &self,
        browser: &Token,
        handle: &[u8; 32],
    ) -> Result<Approval, Error> {
        let session = Session::Browser(browser);
        let row = self
            .db
            .query_one_raw(statement(
                "SELECT a.key_check, p.public_key, p.challenge, p.mac
                 FROM central_identity.pairings p
                 JOIN central_identity.sessions s ON s.digest = p.session_digest
                 JOIN central_identity.accounts a ON a.id = p.account_id
                     AND a.vault_epoch = p.epoch AND a.credential_revision = s.credential_revision
                 WHERE p.digest = $1 AND s.digest = $2 AND s.kind = 'browser'
                 AND s.expires_at > pg_catalog.clock_timestamp()
                 AND p.expires_at > pg_catalog.clock_timestamp()
                 AND p.public_key IS NOT NULL AND p.sealed IS NULL
                 AND a.credential IS NOT NULL AND a.key_check IS NOT NULL",
                vec![handle.to_vec().into(), session.digest().to_vec().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        Ok(Approval {
            key_check: bytes(&row, "key_check")?,
            claim: Claim {
                public_key: bytes(&row, "public_key")?,
                challenge: bytes(&row, "challenge")?,
                mac: bytes(&row, "mac")?,
            },
        })
    }

    /// Store the sealed master key of a pairing whose approval was checked,
    /// under the same conditions as [`Self::pairing_for_approval`]; a second
    /// approval finds nothing to approve.
    ///
    /// # Errors
    /// `Rejected` when the pairing is no longer approvable; storage failure.
    pub async fn approve_pairing(
        &self,
        browser: &Token,
        handle: &[u8; 32],
        sealed: &[u8],
    ) -> Result<(), Error> {
        let session = Session::Browser(browser);
        self.db
            .query_one_raw(statement(
                "UPDATE central_identity.pairings p SET sealed = $3
                 FROM central_identity.sessions s, central_identity.accounts a
                 WHERE p.digest = $1 AND s.digest = $2 AND s.kind = 'browser'
                 AND p.session_digest = s.digest AND s.expires_at > pg_catalog.clock_timestamp()
                 AND a.id = p.account_id AND a.vault_epoch = p.epoch
                 AND a.credential_revision = s.credential_revision AND a.credential IS NOT NULL
                 AND p.public_key IS NOT NULL AND p.sealed IS NULL
                 AND p.expires_at > pg_catalog.clock_timestamp()
                 RETURNING p.digest",
                vec![
                    handle.to_vec().into(),
                    session.digest().to_vec().into(),
                    sealed.to_vec().into(),
                ],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        Ok(())
    }

    /// End a pairing at once; idempotent, and only the browser session that
    /// started it can.
    ///
    /// # Errors
    /// Storage failure.
    pub async fn cancel_pairing(&self, browser: &Token, handle: &[u8; 32]) -> Result<(), Error> {
        let session = Session::Browser(browser);
        self.db
            .execute_raw(statement(
                "DELETE FROM central_identity.pairings p USING central_identity.sessions s
                 WHERE p.digest = $1 AND s.digest = $2 AND s.kind = 'browser'
                 AND p.session_digest = s.digest",
                vec![handle.to_vec().into(), session.digest().to_vec().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        Ok(())
    }

    /// A client's claim: once per pairing, within its first two minutes.
    /// Returns the pairing's end, which the seal is bound to. The same claim
    /// again, as a client resends it after losing the answer, is answered
    /// the same.
    ///
    /// # Errors
    /// `Conflict` when the pairing was claimed with another key, challenge or
    /// MAC; `Rejected` for an unknown or ended pairing or a late claim;
    /// storage failure.
    pub async fn claim_pairing(
        &self,
        handle: &[u8; 32],
        public_key: &[u8; 32],
        challenge: &[u8; 32],
        mac: &[u8; 32],
    ) -> Result<i64, Error> {
        let row = self
            .db
            .query_one_raw(statement(
                "UPDATE central_identity.pairings SET public_key = $2, challenge = $3, mac = $4
                 WHERE digest = $1 AND public_key IS NULL
                 AND created_at + interval '2 minutes' > pg_catalog.clock_timestamp()
                 AND expires_at > pg_catalog.clock_timestamp()
                 RETURNING floor(extract(epoch FROM expires_at))::bigint AS expires_at",
                vec![
                    handle.to_vec().into(),
                    public_key.to_vec().into(),
                    challenge.to_vec().into(),
                    mac.to_vec().into(),
                ],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        if let Some(row) = row {
            return row
                .try_get("", "expires_at")
                .map_err(|_| Error::Unavailable);
        }
        let claimed = self
            .db
            .query_one_raw(statement(
                "SELECT public_key, challenge, mac,
                     floor(extract(epoch FROM expires_at))::bigint AS expires_at
                 FROM central_identity.pairings
                 WHERE digest = $1 AND public_key IS NOT NULL
                 AND expires_at > pg_catalog.clock_timestamp()",
                vec![handle.to_vec().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        // Compared in constant time: a lost answer's resend is the client's
        // own claim, anything else another application's.
        let same = bytes::<32>(&claimed, "public_key")?.ct_eq(public_key)
            & bytes::<32>(&claimed, "challenge")?.ct_eq(challenge)
            & bytes::<32>(&claimed, "mac")?.ct_eq(mac);
        if !bool::from(same) {
            return Err(Error::Conflict);
        }
        claimed
            .try_get("", "expires_at")
            .map_err(|_| Error::Unavailable)
    }

    /// Whether the claimed pairing was approved; counts one of its
    /// [`MAX_POLLS`].
    ///
    /// # Errors
    /// `Rejected` for an unknown, unclaimed or ended pairing, or one polled
    /// to its limit; storage failure.
    pub async fn poll_pairing(&self, handle: &[u8; 32]) -> Result<bool, Error> {
        let row = self
            .db
            .query_one_raw(statement(
                "UPDATE central_identity.pairings SET polls = polls + 1
                 WHERE digest = $1 AND public_key IS NOT NULL AND polls < $2
                 AND expires_at > pg_catalog.clock_timestamp()
                 RETURNING sealed IS NOT NULL AS approved",
                vec![handle.to_vec().into(), MAX_POLLS.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        row.try_get("", "approved").map_err(|_| Error::Unavailable)
    }

    /// Redeem an approved pairing: the pairing is deleted whatever the
    /// redeem finds (a wrong verifier, an account at another epoch or a proof
    /// too old each end it), and with the right verifier a native session is
    /// issued for the account at the pairing's epoch, replacing the account's
    /// native session as a sign-in does. The session's 12 hours count from
    /// the browser session's password proof, never from now. Only a storage
    /// failure leaves the pairing as it was.
    ///
    /// # Errors
    /// `Rejected` for an unknown, unapproved or ended pairing, a wrong
    /// challenge, an account at another epoch or without a credential, or a
    /// password proof too old to give a session; storage failure.
    pub async fn redeem_pairing(
        &self,
        handle: &[u8; 32],
        challenge: &[u8; 32],
        session: &Token,
    ) -> Result<Redeemed, Error> {
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let pairing = tx
            .query_one_raw(statement(
                "DELETE FROM central_identity.pairings
                 WHERE digest = $1 AND sealed IS NOT NULL
                 AND expires_at > pg_catalog.clock_timestamp()
                 RETURNING account_id, epoch, challenge, sealed, proven_at",
                vec![handle.to_vec().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        let stored: [u8; 32] = bytes(&pairing, "challenge")?;
        let redeemed = if bool::from(stored.ct_eq(challenge)) {
            // In a savepoint: a refusal undoes the session's issue, never the
            // pairing's deletion.
            let issue = tx.begin().await.map_err(|_| Error::Unavailable)?;
            match paired_session(&issue, &pairing, session).await {
                Ok(redeemed) => {
                    issue.commit().await.map_err(|_| Error::Unavailable)?;
                    Ok(redeemed)
                }
                Err(Error::Rejected) => {
                    issue.rollback().await.map_err(|_| Error::Unavailable)?;
                    Err(Error::Rejected)
                }
                Err(error) => return Err(error),
            }
        } else {
            Err(Error::Rejected)
        };
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        redeemed
    }
}

/// The native session a redeemed pairing gives, for the account at the
/// pairing's epoch, with at least a minute of its 12 hours left.
async fn paired_session(
    tx: &DatabaseTransaction,
    pairing: &sea_orm::QueryResult,
    session: &Token,
) -> Result<Redeemed, Error> {
    let account: Uuid = pairing
        .try_get("", "account_id")
        .map_err(|_| Error::Unavailable)?;
    let epoch: i64 = pairing
        .try_get("", "epoch")
        .map_err(|_| Error::Unavailable)?;
    let row = tx
        .query_one_raw(statement(
            "SELECT id, vault_epoch, username_key, display_name, tag, credential_revision
             FROM central_identity.accounts
             WHERE id = $1 AND vault_epoch = $2 AND credential IS NOT NULL FOR UPDATE",
            vec![account.into(), epoch.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::Rejected)?;
    let identity = identity(&row)?;
    let revision: i64 = row
        .try_get("", "credential_revision")
        .map_err(|_| Error::Unavailable)?;
    tx.execute_raw(statement(
        "DELETE FROM central_identity.sessions WHERE account_id = $1 AND kind = 'native'",
        vec![uuid(identity.account).into()],
    ))
    .await
    .map_err(|_| Error::Unavailable)?;
    // At least a minute of the 12 hours must be left, or no session.
    let inserted = tx
        .query_one_raw(statement(
            "INSERT INTO central_identity.sessions
             (digest, account_id, credential_revision, kind, created_at, proven_at, expires_at)
             SELECT $1, $2, $3, 'native', t, p,
                    LEAST(t + interval '30 minutes', p + interval '12 hours')
             FROM (SELECT pg_catalog.clock_timestamp() AS t, $4::timestamptz AS p) clock
             WHERE p + interval '12 hours' > t + interval '1 minute'
             RETURNING floor(extract(epoch FROM expires_at))::bigint AS expires_at",
            vec![
                session.digest().to_vec().into(),
                uuid(identity.account).into(),
                revision.into(),
                pairing
                    .try_get::<sea_orm::prelude::DateTimeWithTimeZone>("", "proven_at")
                    .map_err(|_| Error::Unavailable)?
                    .into(),
            ],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::Rejected)?;
    let (display_name, tag) = super::profiles::columns(&row)?;
    Ok(Redeemed {
        identity,
        username: row
            .try_get("", "username_key")
            .map_err(|_| Error::Unavailable)?,
        display_name,
        tag,
        sealed: pairing
            .try_get("", "sealed")
            .map_err(|_| Error::Unavailable)?,
        expires_at: inserted
            .try_get("", "expires_at")
            .map_err(|_| Error::Unavailable)?,
    })
}
