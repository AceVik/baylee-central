use super::{
    AccountStore, Credential, Error, Identity, Token, identity, number, statement, uuid,
    validate_document,
};
use central_api::wire::TicketPurpose;
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

    pub(super) const fn kind(self) -> &'static str {
        match self {
            Self::Native(_) => "native",
            Self::Browser(_) => "browser",
        }
    }

    /// The digest the session is stored under.
    pub(super) fn digest(self) -> [u8; 32] {
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
    /// Atomically replace the account's earlier session of the same kind, with its
    /// tickets and leases; a session of the other kind stays. A registered
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
        let session = session.into();
        let transaction = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let row = transaction.query_one_raw(statement(
            "SELECT wrapped_master, username_key, source = 'registered' AND tag IS NULL AS untagged
             FROM central_identity.accounts WHERE id = $1
             AND vault_epoch = $2 AND credential_revision = $3 AND credential IS NOT NULL FOR UPDATE",
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
        let inserted = transaction.query_one_raw(statement(
            "INSERT INTO central_identity.sessions (digest, account_id, credential_revision, kind, created_at, expires_at)
             SELECT $1, $2, $3, $4, t, t + interval '30 minutes' FROM (SELECT pg_catalog.clock_timestamp() AS t) clock
             RETURNING floor(extract(epoch FROM expires_at))::bigint AS expires_at",
            vec![session.digest().to_vec().into(), uuid(credential.identity.account).into(),
                credential.revision.into(), session.kind().into()],
        )).await.map_err(|_| Error::Unavailable)?.ok_or(Error::Busy)?;
        let issued = Issued {
            username: row
                .try_get("", "username_key")
                .map_err(|_| Error::Unavailable)?,
            wrapped_master: row
                .try_get("", "wrapped_master")
                .map_err(|_| Error::Unavailable)?,
            expires_at: inserted
                .try_get("", "expires_at")
                .map_err(|_| Error::Unavailable)?,
        };
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

    /// Issue one short-lived capability for an already-authorized gateway audience.
    ///
    /// # Errors
    /// Rejects unconfirmed migration, expired/revoked sessions and ticket capacity exhaustion.
    pub async fn gateway_ticket(
        &self,
        session: &Token,
        audience: &str,
        ticket: &Token,
        purpose: TicketPurpose,
    ) -> Result<(), Error> {
        let transaction = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let valid = transaction.query_one_raw(statement(
            "SELECT a.id FROM central_identity.accounts a JOIN central_identity.sessions s
             ON s.account_id = a.id AND s.credential_revision = a.credential_revision WHERE s.digest = $1
             AND s.expires_at > pg_catalog.clock_timestamp() AND a.restored FOR UPDATE OF s",
            vec![session.digest().to_vec().into()],
        )).await.map_err(|_| Error::Unavailable)?.is_some();
        if !valid {
            return Err(Error::Rejected);
        }
        transaction.execute_raw(statement(
            "DELETE FROM central_identity.gateway_tickets WHERE session_digest = $1 AND expires_at <= pg_catalog.clock_timestamp()",
            vec![session.digest().to_vec().into()],
        )).await.map_err(|_| Error::Unavailable)?;
        let issued = transaction.execute_raw(statement(
            "INSERT INTO central_identity.gateway_tickets (digest, session_digest, audience, purpose, created_at, expires_at)
             SELECT $1, $2, $3, $4, t, t + interval '30 seconds' FROM (SELECT pg_catalog.clock_timestamp() AS t) clock
             WHERE (SELECT count(*) FROM central_identity.gateway_tickets WHERE session_digest = $2) < 8",
            vec![ticket.digest().to_vec().into(), session.digest().to_vec().into(), audience.into(), purpose.as_str().into()],
        )).await.map_err(|_| Error::Unavailable)?;
        if issued.rows_affected() != 1 {
            return Err(Error::Busy);
        }
        transaction.commit().await.map_err(|_| Error::Unavailable)
    }

    /// Redeem once for the bridge's configured audience, with fresh session/revision checks.
    /// A registered account has no legacy ID.
    ///
    /// # Errors
    /// Rejects expiry, replay, wrong audience, revocation or storage failure.
    pub async fn redeem_gateway(
        &self,
        ticket: &Token,
        audience: &str,
        purpose: TicketPurpose,
    ) -> Result<(Identity, Option<String>), Error> {
        let row = self.db.query_one_raw(statement(
            "WITH spent AS (DELETE FROM central_identity.gateway_tickets WHERE digest = $1 AND audience = $2 AND purpose = $3
             AND expires_at > pg_catalog.clock_timestamp() RETURNING session_digest)
             SELECT a.id, a.vault_epoch, a.legacy_id FROM spent
             JOIN central_identity.sessions s ON s.digest = spent.session_digest
             JOIN central_identity.accounts a ON a.id = s.account_id AND a.credential_revision = s.credential_revision
             WHERE s.expires_at > pg_catalog.clock_timestamp() AND a.restored",
            vec![ticket.digest().to_vec().into(), audience.into(), purpose.as_str().into()],
        )).await.map_err(|_| Error::Unavailable)?.ok_or(Error::Rejected)?;
        Ok((
            identity(&row)?,
            row.try_get("", "legacy_id")
                .map_err(|_| Error::Unavailable)?,
        ))
    }

    /// Renew a live session: it expires 30 minutes from now, never later than
    /// 12 hours after the password sign-in that issued it, and the gateway
    /// leases delegated from it follow, in the same transaction. An expiry
    /// only moves forward: each renewal is later than the one before, up to the
    /// cap. Returns the new absolute Unix-seconds expiry.
    ///
    /// # Errors
    /// Rejects an expired, replaced or revoked session (also after a credential
    /// change) without touching anything, and reports unavailable storage.
    pub async fn renew_session<'a>(&self, session: impl Into<Session<'a>>) -> Result<i64, Error> {
        let session = session.into();
        let digest = session.digest().to_vec();
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let row = tx.query_one_raw(statement(
            "UPDATE central_identity.sessions s SET expires_at = LEAST(
                 pg_catalog.clock_timestamp() + interval '30 minutes', s.created_at + interval '12 hours')
             FROM central_identity.accounts a
             WHERE s.digest = $1 AND s.kind = $2 AND a.id = s.account_id
             AND a.credential_revision = s.credential_revision AND a.credential IS NOT NULL
             AND s.expires_at > pg_catalog.clock_timestamp()
             RETURNING floor(extract(epoch FROM s.expires_at))::bigint AS expires_at",
            vec![digest.clone().into(), session.kind().into()],
        )).await.map_err(|_| Error::Unavailable)?.ok_or(Error::Rejected)?;
        let expires_at: i64 = row
            .try_get("", "expires_at")
            .map_err(|_| Error::Unavailable)?;
        // A lease never outlives its session, and now lives as long as it.
        tx.execute_raw(statement(
            "UPDATE central_identity.gateway_leases l SET expires_at = s.expires_at
             FROM central_identity.sessions s WHERE s.digest = $1 AND l.session_digest = s.digest
             AND l.expires_at > pg_catalog.clock_timestamp()",
            vec![digest.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(expires_at)
    }

    /// Revoke the central session and its outstanding gateway tickets; idempotent after expiry.
    ///
    /// # Errors
    /// Reports unavailable storage; does not claim success until the deletion commits.
    pub async fn logout<'a>(&self, session: impl Into<Session<'a>>) -> Result<(), Error> {
        self.db
            .execute_raw(statement(
                "DELETE FROM central_identity.sessions WHERE digest = $1",
                vec![session.into().digest().to_vec().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        Ok(())
    }
}
