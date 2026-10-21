//! Terms of use (`docs/terms.md`): published gateway versions and acceptances,
//! both immutable records. The caller validates texts and computes digests.
use super::{AccountStore, Error, Identity, Session, Token, identity, statement};
use central_api::{
    terms::{TermsDocument, TermsText, TermsVersion},
    wire::TicketPurpose,
};
use sea_orm::{ConnectionTrait, QueryResult, TransactionTrait, Value, prelude::Uuid};

/// Most versions one gateway may publish.
pub const MAX_GATEWAY_VERSIONS: i64 = 256;

/// The subject an acceptance of Central's own terms names.
const CENTRAL: &str = "central";

/// A live session joined to its account, as every account operation checks it.
const LIVE_SESSION: &str = "SELECT s.account_id FROM central_identity.sessions s
     JOIN central_identity.accounts a ON a.id = s.account_id AND a.credential_revision = s.credential_revision
     WHERE s.digest = $1 AND s.expires_at > pg_catalog.clock_timestamp()";

fn count(row: &QueryResult, column: &str) -> Result<i64, Error> {
    row.try_get("", column).map_err(|_| Error::Unavailable)
}

fn version_values(version: Option<&TermsVersion>) -> [Value; 2] {
    [
        version.map(|v| v.version.clone()).into(),
        version.map(|v| v.digest.to_vec()).into(),
    ]
}

impl AccountStore {
    /// Whether this live session's account accepted `current` (true when
    /// nothing is in force).
    ///
    /// # Errors
    /// Rejects a dead session; database failures are `Unavailable`, never "accepted".
    pub async fn terms_status<'a>(
        &self,
        session: impl Into<Session<'a>>,
        current: Option<&TermsVersion>,
    ) -> Result<bool, Error> {
        let [version, digest] = version_values(current);
        let row = self
            .db
            .query_one_raw(statement(
                &format!(
                    "WITH live AS ({LIVE_SESSION})
                 SELECT ($2::text IS NULL OR EXISTS (SELECT 1 FROM central_identity.terms_acceptances t
                     WHERE t.account_id = live.account_id AND t.subject = $4 AND t.version = $2 AND t.digest = $3))
                     AS accepted
                 FROM live"
                ),
                vec![
                    session.into().digest().to_vec().into(),
                    version,
                    digest,
                    CENTRAL.into(),
                ],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        row.try_get("", "accepted").map_err(|_| Error::Unavailable)
    }

    /// Record that this session's account accepted Central's `current` terms;
    /// the caller has checked that `current` is in force. Accepting it again is
    /// no error.
    ///
    /// # Errors
    /// Rejects a dead session.
    pub async fn accept_central_terms<'a>(
        &self,
        session: impl Into<Session<'a>>,
        current: &TermsVersion,
    ) -> Result<(), Error> {
        let row = self
            .db
            .query_one_raw(statement(
                &format!(
                    "WITH live AS ({LIVE_SESSION}),
                 recorded AS (INSERT INTO central_identity.terms_acceptances (account_id, subject, version, digest)
                     SELECT account_id, $4, $2, $3 FROM live ON CONFLICT DO NOTHING RETURNING 1)
                 SELECT (SELECT count(*) FROM live) AS live"
                ),
                vec![
                    session.into().digest().to_vec().into(),
                    current.version.clone().into(),
                    current.digest.to_vec().into(),
                    CENTRAL.into(),
                ],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)?;
        if count(&row, "live")? == 0 {
            return Err(Error::Rejected);
        }
        Ok(())
    }

    /// Record that this session's account accepted `accepted` of the gateway
    /// at `audience`, which must be that gateway's current version and digest.
    ///
    /// # Errors
    /// Rejects a dead session; `Conflict` when `accepted` is not current.
    pub async fn accept_gateway_terms(
        &self,
        session: &Token,
        audience: &str,
        accepted: &TermsVersion,
    ) -> Result<(), Error> {
        let row = self
            .db
            .query_one_raw(statement(
                &format!(
                    "WITH live AS ({LIVE_SESSION}),
                 current AS (SELECT version, digest FROM central_identity.gateway_terms WHERE audience = $4
                     ORDER BY published_at DESC, version DESC LIMIT 1),
                 matching AS (SELECT 1 FROM current WHERE version = $2 AND digest = $3),
                 recorded AS (INSERT INTO central_identity.terms_acceptances (account_id, subject, version, digest)
                     SELECT account_id, $4, $2, $3 FROM live WHERE EXISTS (SELECT 1 FROM matching)
                     ON CONFLICT DO NOTHING RETURNING 1)
                 SELECT (SELECT count(*) FROM live) AS live, (SELECT count(*) FROM matching) AS matching"
                ),
                vec![
                    session.digest().to_vec().into(),
                    accepted.version.clone().into(),
                    accepted.digest.to_vec().into(),
                    audience.into(),
                ],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)?;
        if count(&row, "live")? == 0 {
            return Err(Error::Rejected);
        }
        if count(&row, "matching")? == 0 {
            return Err(Error::Conflict);
        }
        Ok(())
    }

    /// Publish a validated version for the gateway at `audience`. The same
    /// version with the same digest again is no error; with another digest it
    /// is `Conflict`. A gateway publishes at most [`MAX_GATEWAY_VERSIONS`].
    ///
    /// # Errors
    /// `Conflict` for a changed published version, `Rejected` past the limit.
    pub async fn publish_gateway_terms(
        &self,
        audience: &str,
        version: &TermsVersion,
        texts: &[TermsText],
    ) -> Result<(), Error> {
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let row = tx
            .query_one_raw(statement(
                "WITH existing AS (SELECT digest FROM central_identity.gateway_terms WHERE audience = $1 AND version = $2),
                 published AS (SELECT count(*) AS n FROM central_identity.gateway_terms WHERE audience = $1),
                 inserted AS (INSERT INTO central_identity.gateway_terms (audience, version, digest)
                     SELECT $1, $2, $3 WHERE NOT EXISTS (SELECT 1 FROM existing)
                     AND (SELECT n FROM published) < $4
                     ON CONFLICT DO NOTHING RETURNING 1)
                 SELECT (SELECT count(*) FROM inserted) AS inserted,
                     (SELECT count(*) FROM existing WHERE digest = $3) AS same,
                     (SELECT count(*) FROM existing) AS existing",
                vec![
                    audience.into(),
                    version.version.clone().into(),
                    version.digest.to_vec().into(),
                    MAX_GATEWAY_VERSIONS.into(),
                ],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)?;
        if count(&row, "inserted")? == 0 {
            // A concurrent publish of the same version wins the key; either way
            // the stored digest decides.
            let _ = tx.rollback().await;
            return match (count(&row, "existing")?, count(&row, "same")?) {
                (_, 1) => Ok(()),
                (1, _) => Err(Error::Conflict),
                _ => self.published_digest(audience, version).await,
            };
        }
        for text in texts {
            tx.execute_raw(statement(
                "INSERT INTO central_identity.gateway_terms_texts (audience, version, lang, updated, markdown)
                 VALUES ($1, $2, $3, $4, $5)",
                vec![
                    audience.into(),
                    version.version.clone().into(),
                    text.lang.clone().into(),
                    text.updated.clone().into(),
                    text.markdown.clone().into(),
                ],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        }
        tx.commit().await.map_err(|_| Error::Unavailable)
    }

    /// After a lost insert race: the version exists with this digest, another
    /// one, or not at all (the limit refused it).
    async fn published_digest(&self, audience: &str, version: &TermsVersion) -> Result<(), Error> {
        let row = self
            .db
            .query_one_raw(statement(
                "SELECT digest FROM central_identity.gateway_terms WHERE audience = $1 AND version = $2",
                vec![audience.into(), version.version.clone().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        let digest: Vec<u8> = row.try_get("", "digest").map_err(|_| Error::Unavailable)?;
        if digest == version.digest {
            Ok(())
        } else {
            Err(Error::Conflict)
        }
    }

    /// The current version of the gateway at `audience`, with its texts in tag
    /// order, or none.
    ///
    /// # Errors
    /// Database failures and inconsistent rows are `Unavailable`.
    pub async fn gateway_terms(&self, audience: &str) -> Result<Option<TermsDocument>, Error> {
        let rows = self
            .db
            .query_all_raw(statement(
                "WITH current AS (SELECT version, digest FROM central_identity.gateway_terms WHERE audience = $1
                     ORDER BY published_at DESC, version DESC LIMIT 1)
                 SELECT c.version, c.digest, t.lang, t.updated, t.markdown FROM current c
                 JOIN central_identity.gateway_terms_texts t ON t.audience = $1 AND t.version = c.version
                 ORDER BY t.lang",
                vec![audience.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        let Some(first) = rows.first() else {
            return Ok(None);
        };
        let version: String = first
            .try_get("", "version")
            .map_err(|_| Error::Unavailable)?;
        let digest: Vec<u8> = first
            .try_get("", "digest")
            .map_err(|_| Error::Unavailable)?;
        let texts = rows
            .iter()
            .map(|row| {
                Ok(TermsText {
                    lang: row.try_get("", "lang").map_err(|_| Error::Unavailable)?,
                    updated: row.try_get("", "updated").map_err(|_| Error::Unavailable)?,
                    markdown: row
                        .try_get("", "markdown")
                        .map_err(|_| Error::Unavailable)?,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let digest: [u8; 32] = digest.try_into().map_err(|_| Error::Unavailable)?;
        // What is served is what was digested, or nothing.
        if central_api::terms::validate(&texts).is_err()
            || central_api::terms::digest(&texts) != digest
        {
            return Err(Error::Unavailable);
        }
        Ok(Some(TermsDocument {
            version: TermsVersion { version, digest },
            texts,
        }))
    }

    /// [`Self::gateway_session`] plus whether the account accepted `central`
    /// (Central's terms in force) and `gateway` (the gateway's own current
    /// terms); a missing one counts as accepted.
    ///
    /// # Errors
    /// As [`Self::gateway_session`].
    pub async fn gateway_authorize(
        &self,
        lease: &Token,
        audience: &str,
        gateway: Option<[u8; 16]>,
        purpose: TicketPurpose,
        central: Option<&TermsVersion>,
        terms: Option<&TermsVersion>,
    ) -> Result<(Identity, i64, bool, bool), Error> {
        if !matches!(purpose, TicketPurpose::Admit | TicketPurpose::Admin)
            || (gateway.is_none() && matches!(purpose, TicketPurpose::Admin))
        {
            return Err(Error::Rejected);
        }
        let [central_version, central_digest] = version_values(central);
        let [gateway_version, gateway_digest] = version_values(terms);
        let row=self.db.query_one_raw(statement(
            "SELECT a.id,a.vault_epoch,floor(extract(epoch FROM LEAST(l.expires_at,s.expires_at)))::bigint AS expires_at,
             ($5::text IS NULL OR EXISTS(SELECT 1 FROM central_identity.terms_acceptances t
                 WHERE t.account_id=a.id AND t.subject=$9 AND t.version=$5 AND t.digest=$6)) AS central_terms,
             ($7::text IS NULL OR EXISTS(SELECT 1 FROM central_identity.terms_acceptances t
                 WHERE t.account_id=a.id AND t.subject=$2 AND t.version=$7 AND t.digest=$8)) AS gateway_terms
             FROM central_identity.gateway_leases l JOIN central_identity.sessions s ON s.digest=l.session_digest
             JOIN central_identity.accounts a ON a.id=s.account_id AND a.credential_revision=s.credential_revision
             WHERE l.digest=$1 AND l.audience=$2 AND l.gateway_id IS NOT DISTINCT FROM $3::uuid AND l.purpose=$4
             AND l.expires_at>pg_catalog.clock_timestamp() AND s.expires_at>pg_catalog.clock_timestamp() AND a.restored
             AND ($3::uuid IS NULL OR EXISTS(SELECT 1 FROM central_identity.gateways g
                 JOIN central_identity.gateway_approvals p ON p.gateway_id=g.id AND p.enabled
                 WHERE g.id=$3 AND g.origin=$2 AND ($4='admit' OR EXISTS
                     (SELECT 1 FROM central_identity.gateway_admins m WHERE m.gateway_id=g.id AND m.account_id=a.id))))",
            vec![lease.digest().to_vec().into(),audience.into(),gateway.map(Uuid::from_bytes).into(),purpose.as_str().into(),
                 central_version,central_digest,gateway_version,gateway_digest,CENTRAL.into()]
        )).await.map_err(|_|Error::Unavailable)?.ok_or(Error::Rejected)?;
        Ok((
            identity(&row)?,
            row.try_get("", "expires_at")
                .map_err(|_| Error::Unavailable)?,
            row.try_get("", "central_terms")
                .map_err(|_| Error::Unavailable)?,
            row.try_get("", "gateway_terms")
                .map_err(|_| Error::Unavailable)?,
        ))
    }
}
