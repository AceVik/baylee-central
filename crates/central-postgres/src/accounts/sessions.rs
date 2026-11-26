use super::{AccountStore, Credential, Error, Token, number, statement, uuid, validate_document};
use sea_orm::{ConnectionTrait, TransactionTrait};

/// A session token and the surface it belongs to. An account holds at most one
/// session of each kind. A browser session is stored under a digest in its own
/// domain, so a cookie value presented as a native bearer finds no session, and
/// a native bearer is no browser session. A plain `&Token` is a native session.
#[derive(Clone, Copy)]
pub enum Session<'a> {
    /// The native client's bearer on `/v1/native`.
    Native(&'a Token),
    /// The website's `__Host-` cookie on the browser listener.
    Browser(&'a Token),
}

impl<'a> From<&'a Token> for Session<'a> {
    fn from(token: &'a Token) -> Self {
        Self::Native(token)
    }
}

impl Session<'_> {
    /// Prefix of a browser session's digest input. With it the input is longer
    /// than any token, so no bearer hashes to a browser session's digest.
    const BROWSER_DOMAIN: &'static [u8] = b"baylee-central/browser-session/v1\0";
    /// Domain separator for rotating browser refresh tokens stored beside the
    /// access session digest.
    const REFRESH_DOMAIN: &'static [u8] = b"baylee-central/browser-refresh/v1\0";

    pub(super) const fn kind(self) -> &'static str {
        match self {
            Self::Native(_) => "native",
            Self::Browser(_) => "browser",
        }
    }

    /// The digest the session is stored under (`token.digest()` for native,
    /// domain-separated SHA-256 for browser).
    #[must_use]
    pub fn digest(self) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        match self {
            Self::Native(token) => token.digest(),
            Self::Browser(token) => Sha256::new()
                .chain_update(Self::BROWSER_DOMAIN)
                .chain_update(token.to_secret_bytes().as_slice())
                .finalize()
                .into(),
        }
    }

    /// The digest of a sliding browser refresh token.
    pub(super) fn refresh_digest(token: &Token) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        Sha256::new()
            .chain_update(Self::REFRESH_DOMAIN)
            .chain_update(token.to_secret_bytes().as_slice())
            .finalize()
            .into()
    }
}

/// Outcome of exchanging a sliding browser refresh token (`__Host-central-refresh`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefreshOutcome {
    /// Rotated cleanly; `expires_at` is the short-lived access cookie expiry.
    Refreshed {
        /// New access token expiry in Unix seconds.
        expires_at: i64,
        /// Previous access session digest (`sessions.digest`) replaced by rotation.
        previous_digest: [u8; 32],
    },
    /// Unknown, expired, or credential-stale refresh token.
    Rejected {
        /// Deleted session digest when a matched row was expired or stale.
        revoked_digest: Option<[u8; 32]>,
    },
    /// Previously rotated refresh token replayed: the session was revoked.
    Reused {
        /// Deleted session digest of the compromised session.
        revoked_digest: [u8; 32],
    },
}

/// A session just issued.
pub struct Issued {
    /// The account's canonical username.
    pub username: String,
    /// The account's stored client-encrypted master key.
    pub wrapped_master: Vec<u8>,
    /// Absolute Unix-seconds expiry, 30 minutes after issue; renewed by
    /// [`AccountStore::renew_session`] up to 12 hours after it.
    pub expires_at: i64,
}

impl AccountStore {
    /// Issue after OPAQUE verification, conditional on the unchanged credential revision.
    /// Atomically replace the account's earlier session of the same kind, which revokes
    /// its tokens at gateways; a session of the other kind stays. A registered
    /// account without a tag (one the binary before migration 000011
    /// registered) draws it now; when none can be drawn, the sign-in goes on
    /// without one and the next one tries again.
    ///
    /// # Errors
    /// Rejects stale credentials or database failure without revoking the current session.
    pub async fn issue_session<'a>(
        &self,
        credential: &Credential,
        session: impl Into<Session<'a>>,
    ) -> Result<Issued, Error> {
        self.issue_session_inner(credential, session.into(), None, None)
            .await
    }

    /// Issue a verified browser login, preserving this browser's explicit push consent.
    /// Only subscriptions owned by this account and bound to the presented previous
    /// cookie move to the new session. Other devices/accounts are left untouched.
    /// An expired cookie may identify consent, but never authorizes this login.
    ///
    /// # Errors
    /// Rejects stale verified credentials or database failure, atomically.
    pub async fn issue_browser_session(
        &self,
        credential: &Credential,
        token: &Token,
        previous: Option<&Token>,
    ) -> Result<Issued, Error> {
        self.issue_session_inner(credential, Session::Browser(token), previous, None)
            .await
    }

    /// Issue a verified browser login paired with a sliding 3-day rotating refresh
    /// token capped at 30 days from sign-in (`expires_at` = +15 minutes).
    ///
    /// # Errors
    /// Rejects stale verified credentials or database failure, atomically.
    pub async fn issue_browser_session_with_refresh(
        &self,
        credential: &Credential,
        token: &Token,
        refresh: &Token,
        previous: Option<&Token>,
    ) -> Result<Issued, Error> {
        self.issue_session_inner(credential, Session::Browser(token), previous, Some(refresh))
            .await
    }

    async fn issue_session_inner(
        &self,
        credential: &Credential,
        session: Session<'_>,
        previous_browser: Option<&Token>,
        refresh: Option<&Token>,
    ) -> Result<Issued, Error> {
        let transaction = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let row = transaction.query_one_raw(statement(
            &format!("SELECT wrapped_master, username_key, source = 'registered' AND tag IS NULL AS untagged
             FROM central_identity.accounts WHERE id = $1
             AND vault_epoch = $2 AND credential_revision = $3 AND credential IS NOT NULL
             -- an AI player signs in through its runner's token alone (docs/ai-players.md §2)
             AND {} FOR UPDATE", super::ai::NO_AI),
            vec![uuid(credential.identity.account).into(), number(credential.identity.epoch)?.into(), credential.revision.into()],
        )).await.map_err(|_| Error::Unavailable)?.ok_or(Error::Rejected)?;
        if row
            .try_get::<bool>("", "untagged")
            .map_err(|_| Error::Unavailable)?
        {
            let account = uuid(credential.identity.account);
            // Each draw runs in a savepoint: a failed one leaves the sign-in intact.
            let _ = self
                .first_free_tag(
                    &transaction,
                    Some(account),
                    None,
                    Error::Unavailable,
                    |tag| {
                        statement(
                        "UPDATE central_identity.accounts SET tag = $2 WHERE id = $1 AND tag IS NULL
                         RETURNING id",
                        vec![account.into(), tag.into()],
                    )
                    },
                )
                .await;
        }
        // The password sign-in's time, for administering accounts; renewal never moves it.
        transaction
            .execute_raw(statement(
                "UPDATE central_identity.accounts SET last_sign_in_at = pg_catalog.clock_timestamp()
                 WHERE id = $1",
                vec![uuid(credential.identity.account).into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        transaction
            .execute_raw(statement(
                "DELETE FROM central_identity.sessions WHERE account_id = $1 AND kind = $2",
                vec![
                    uuid(credential.identity.account).into(),
                    session.kind().into(),
                ],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        let inserted = insert_issued_session_tx(&transaction, credential, session, refresh).await?;
        let issued = Issued {
            username: row
                .try_get("", "username_key")
                .map_err(|_| Error::Unavailable)?,
            wrapped_master: row
                .try_get("", "wrapped_master")
                .map_err(|_| Error::Unavailable)?,
            expires_at: inserted,
        };
        if let Some(previous) = previous_browser {
            transaction
                .execute_raw(statement(
                    "UPDATE central_identity.push_subscriptions SET session_digest=$1
                 WHERE account_id=$2 AND credential_revision=$3 AND session_digest=$4",
                    vec![
                        session.digest().to_vec().into(),
                        uuid(credential.identity.account).into(),
                        credential.revision.into(),
                        Session::Browser(previous).digest().to_vec().into(),
                    ],
                ))
                .await
                .map_err(|_| Error::Unavailable)?;
        }
        transaction.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(issued)
    }

    /// Read only the authenticated account's ciphertext and current revision.
    ///
    /// # Errors
    /// Expired/revoked sessions are rejected, never treated as empty accounts.
    pub async fn document(&self, session: &Token) -> Result<(u64, Vec<u8>), Error> {
        let row = self.db.query_one_raw(statement(
            "SELECT a.document_revision, a.document FROM central_identity.accounts a
             JOIN central_identity.sessions s ON s.account_id = a.id AND s.credential_revision = a.credential_revision
             WHERE s.digest = $1 AND s.expires_at > pg_catalog.clock_timestamp()",
            vec![session.digest().to_vec().into()],
        )).await.map_err(|_| Error::Unavailable)?.ok_or(Error::Rejected)?;
        let revision: i64 = row
            .try_get("", "document_revision")
            .map_err(|_| Error::Unavailable)?;
        Ok((
            u64::try_from(revision).map_err(|_| Error::Unavailable)?,
            row.try_get("", "document")
                .map_err(|_| Error::Unavailable)?,
        ))
    }

    /// Store ciphertext with a compare-and-swap; the caller cannot supply a target account.
    ///
    /// # Errors
    /// Rejects invalid sessions, malformed size, stale revision or unavailable storage.
    pub async fn write_document(
        &self,
        session: &Token,
        expected: u64,
        document: &[u8],
    ) -> Result<(), Error> {
        validate_document(document)?;
        let expected = number(expected)?;
        if expected < 1 || expected == i64::MAX {
            return Err(Error::Rejected);
        }
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        super::objects::locked_owner(&tx, session, false).await?;
        let result = tx.execute_raw(statement(
            "UPDATE central_identity.accounts a SET document = $2, document_revision = document_revision + 1
             FROM central_identity.sessions s WHERE s.account_id = a.id AND s.credential_revision = a.credential_revision
             AND s.digest = $1 AND s.expires_at > pg_catalog.clock_timestamp() AND a.document_revision = $3",
            vec![session.digest().to_vec().into(), document.to_vec().into(), expected.into()],
        )).await.map_err(|_| Error::Unavailable)?;
        if result.rows_affected() != 1 {
            return Err(Error::Conflict);
        }
        tx.commit().await.map_err(|_| Error::Unavailable)
    }

    /// Record successful authenticated restore of the currently stored revision.
    ///
    /// # Errors
    /// Rejects invalid sessions, a changed revision or database failure.
    pub async fn confirm_restore(&self, session: &Token, revision: u64) -> Result<(), Error> {
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        super::objects::locked_owner(&tx, session, false).await?;
        let result = tx.execute_raw(statement(
            "UPDATE central_identity.accounts a SET restored = true FROM central_identity.sessions s
             WHERE s.account_id = a.id AND s.credential_revision = a.credential_revision AND s.digest = $1
             AND s.expires_at > pg_catalog.clock_timestamp() AND a.document_revision = $2",
            vec![session.digest().to_vec().into(), number(revision)?.into()],
        )).await.map_err(|_| Error::Unavailable)?;
        if result.rows_affected() != 1 {
            return Err(Error::Conflict);
        }
        tx.commit().await.map_err(|_| Error::Unavailable)
    }

    /// Renew a live session: it expires 30 minutes from now, never later than
    /// 12 hours after the password sign-in that issued it. An expiry
    /// only moves forward: each renewal is later than the one before, up to the
    /// cap. Returns the new absolute Unix-seconds expiry.
    ///
    /// # Errors
    /// Rejects an expired, replaced or revoked session (also after a credential
    /// change) without touching anything, and reports unavailable storage.
    pub async fn renew_session<'a>(&self, session: impl Into<Session<'a>>) -> Result<i64, Error> {
        let session = session.into();
        let digest = session.digest().to_vec();
        let row = self
            .db
            .query_one_raw(statement(
                "UPDATE central_identity.sessions s SET expires_at = CASE
                 WHEN s.kind = 'browser' THEN LEAST(
                     pg_catalog.clock_timestamp()
                         + CASE WHEN s.refresh_digest IS NOT NULL THEN interval '15 minutes' ELSE interval '30 minutes' END,
                     COALESCE(s.absolute_expires_at, s.created_at + interval '30 days'))
                 ELSE LEAST(
                     pg_catalog.clock_timestamp() + interval '30 minutes',
                     COALESCE(s.proven_at, s.created_at) + interval '12 hours')
                 END
             FROM central_identity.accounts a
             WHERE s.digest = $1 AND s.kind = $2 AND a.id = s.account_id
             AND a.credential_revision = s.credential_revision AND a.credential IS NOT NULL
             AND s.expires_at > pg_catalog.clock_timestamp()
             RETURNING floor(extract(epoch FROM s.expires_at))::bigint AS expires_at",
                vec![digest.into(), session.kind().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        let expires_at: i64 = row
            .try_get("", "expires_at")
            .map_err(|_| Error::Unavailable)?;
        Ok(expires_at)
    }

    /// Exchange a single-use sliding browser refresh token for a fresh access
    /// and refresh token pair (+15 min access, +3 day sliding refresh capped
    /// at +30 days from sign-in; `proven_at`/`created_at` never moves).
    /// Replay of a previously rotated refresh token revokes the entire browser session.
    ///
    /// # Errors
    /// Reports storage failure without assuming validity.
    pub async fn refresh_browser_session(
        &self,
        refresh: &Token,
        next_access: &Token,
        next_refresh: &Token,
    ) -> Result<RefreshOutcome, Error> {
        let presented = Session::refresh_digest(refresh).to_vec();
        let next_access_digest = Session::Browser(next_access).digest().to_vec();
        let next_refresh_digest = Session::refresh_digest(next_refresh).to_vec();
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        if let Some(outcome) =
            rotate_browser_session_tx(&tx, &presented, next_access_digest, next_refresh_digest)
                .await?
        {
            tx.commit().await.map_err(|_| Error::Unavailable)?;
            return Ok(outcome);
        }
        if let Some(revoked_digest) = check_reused_refresh_tx(&tx, &presented).await? {
            tx.commit().await.map_err(|_| Error::Unavailable)?;
            return Ok(RefreshOutcome::Reused { revoked_digest });
        }
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(RefreshOutcome::Rejected {
            revoked_digest: None,
        })
    }

    /// Revoke the central session and its tokens at gateways; idempotent after expiry.
    ///
    /// # Errors
    /// Reports unavailable storage; does not claim success until the deletion commits.
    pub async fn logout<'a>(&self, session: impl Into<Session<'a>>) -> Result<(), Error> {
        let digest = session.into().digest().to_vec();
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        tx.query_all_raw(statement(
            "SELECT digest FROM central_identity.sessions WHERE digest=$1 FOR UPDATE",
            vec![digest.clone().into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        delete_session_tx(&tx, &digest).await?;
        tx.commit().await.map_err(|_| Error::Unavailable)
    }
}

async fn rotate_browser_session_tx(
    tx: &impl sea_orm::ConnectionTrait,
    presented: &[u8],
    next_access_digest: Vec<u8>,
    next_refresh_digest: Vec<u8>,
) -> Result<Option<RefreshOutcome>, Error> {
    let Some(row) = tx
        .query_one_raw(statement(
            "SELECT s.digest,
                 (a.credential IS NOT NULL
                  AND a.credential_revision = s.credential_revision
                  AND s.refresh_expires_at > pg_catalog.clock_timestamp()
                  AND COALESCE(s.absolute_expires_at, s.created_at + interval '30 days')
                      > pg_catalog.clock_timestamp()) AS valid
             FROM central_identity.sessions s
             JOIN central_identity.accounts a ON a.id = s.account_id
             WHERE s.refresh_digest = $1 AND s.kind = 'browser'
             FOR UPDATE OF s",
            vec![presented.to_vec().into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
    else {
        return Ok(None);
    };
    let old_digest: Vec<u8> = row.try_get("", "digest").map_err(|_| Error::Unavailable)?;
    let previous_digest = slice_digest(&old_digest)?;
    let valid: bool = row.try_get("", "valid").map_err(|_| Error::Unavailable)?;
    if !valid {
        delete_session_tx(tx, &old_digest).await?;
        return Ok(Some(RefreshOutcome::Rejected {
            revoked_digest: Some(previous_digest),
        }));
    }
    tx.execute_raw(statement(
        "DELETE FROM central_identity.pairings
         WHERE session_digest = $1 AND expires_at <= pg_catalog.clock_timestamp()",
        vec![old_digest.clone().into()],
    ))
    .await
    .map_err(|_| Error::Unavailable)?;
    let updated = tx
        .query_one_raw(statement(
            "UPDATE central_identity.sessions
             SET digest = $2,
                 previous_refresh_digest = $1,
                 refresh_digest = $3,
                 expires_at = LEAST(
                     pg_catalog.clock_timestamp() + interval '15 minutes',
                     COALESCE(absolute_expires_at, created_at + interval '30 days')),
                 refresh_expires_at = LEAST(
                     pg_catalog.clock_timestamp() + interval '3 days',
                     COALESCE(absolute_expires_at, created_at + interval '30 days'))
             WHERE digest = $4 AND kind = 'browser'
             RETURNING floor(extract(epoch FROM expires_at))::bigint AS expires_at",
            vec![
                presented.to_vec().into(),
                next_access_digest.clone().into(),
                next_refresh_digest.into(),
                old_digest.clone().into(),
            ],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::Unavailable)?;
    tx.execute_raw(statement(
        "UPDATE central_identity.push_subscriptions SET session_digest = $1
         WHERE session_digest = $2",
        vec![next_access_digest.into(), old_digest.into()],
    ))
    .await
    .map_err(|_| Error::Unavailable)?;
    let expires_at: i64 = updated
        .try_get("", "expires_at")
        .map_err(|_| Error::Unavailable)?;
    Ok(Some(RefreshOutcome::Refreshed {
        expires_at,
        previous_digest,
    }))
}

async fn check_reused_refresh_tx(
    tx: &impl sea_orm::ConnectionTrait,
    presented: &[u8],
) -> Result<Option<[u8; 32]>, Error> {
    let Some(replayed) = tx
        .query_one_raw(statement(
            "SELECT digest FROM central_identity.sessions
             WHERE previous_refresh_digest = $1 AND kind = 'browser'
             FOR UPDATE",
            vec![presented.to_vec().into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
    else {
        return Ok(None);
    };
    let compromised: Vec<u8> = replayed
        .try_get("", "digest")
        .map_err(|_| Error::Unavailable)?;
    let revoked_digest = slice_digest(&compromised)?;
    delete_session_tx(tx, &compromised).await?;
    Ok(Some(revoked_digest))
}

async fn insert_issued_session_tx(
    tx: &impl sea_orm::ConnectionTrait,
    credential: &Credential,
    session: Session<'_>,
    refresh: Option<&Token>,
) -> Result<i64, Error> {
    let access_mins: i32 = if matches!(session, Session::Browser(_)) && refresh.is_some() {
        15
    } else {
        30
    };
    let refresh_digest: Option<Vec<u8>> = if matches!(session, Session::Browser(_)) {
        refresh.map(|token| Session::refresh_digest(token).to_vec())
    } else {
        None
    };
    let inserted = tx
        .query_one_raw(statement(
            "INSERT INTO central_identity.sessions
             (digest, account_id, credential_revision, kind, created_at, expires_at,
              refresh_digest, refresh_expires_at, absolute_expires_at)
         SELECT $1, $2, $3, $4, t, t + pg_catalog.make_interval(mins => $5), $6,
                CASE WHEN $6::bytea IS NULL THEN NULL ELSE t + interval '3 days' END,
                CASE WHEN $6::bytea IS NULL THEN NULL ELSE t + interval '30 days' END
         FROM (SELECT pg_catalog.clock_timestamp() AS t) clock
         RETURNING floor(extract(epoch FROM expires_at))::bigint AS expires_at",
            vec![
                session.digest().to_vec().into(),
                uuid(credential.identity.account).into(),
                credential.revision.into(),
                session.kind().into(),
                access_mins.into(),
                refresh_digest.into(),
            ],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::Busy)?;
    inserted
        .try_get("", "expires_at")
        .map_err(|_| Error::Unavailable)
}

fn slice_digest(bytes: &[u8]) -> Result<[u8; 32], Error> {
    bytes.try_into().map_err(|_| Error::Unavailable)
}

pub(super) async fn delete_session_tx(
    tx: &impl sea_orm::ConnectionTrait,
    digest: &[u8],
) -> Result<(), Error> {
    tx.execute_raw(statement(
        "DELETE FROM central_identity.push_subscriptions WHERE session_digest = $1",
        vec![digest.to_vec().into()],
    ))
    .await
    .map_err(|_| Error::Unavailable)?;
    tx.execute_raw(statement(
        "DELETE FROM central_identity.sessions WHERE digest = $1",
        vec![digest.to_vec().into()],
    ))
    .await
    .map_err(|_| Error::Unavailable)?;
    Ok(())
}
