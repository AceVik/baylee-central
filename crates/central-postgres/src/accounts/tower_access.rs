//! Separate Tower credentials bound to a live native session, never gateway grants.
use super::{AccountStore, Error, Token, profiles, statement};
use central_api::{terms::TermsVersion, tower_access::Identity};
use sea_orm::{ConnectionTrait, TransactionTrait, prelude::Uuid};

impl AccountStore {
    /// Retain at most two independent, short-lived Tower credentials per native session.
    /// The caller supplies an operator-configured canonical origin and fresh random token.
    /// # Errors
    /// Dead/unrestored/wrong-kind session, invalid origin or persistence failure.
    pub async fn issue_tower_access(
        &self,
        session: &Token,
        origin: &str,
        fresh: &Token,
    ) -> Result<i64, Error> {
        central_api::gateway::validate_origin(origin, false)
            .or_else(|_| central_api::gateway::validate_origin(origin, true))?;
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let digest = session.digest().to_vec();
        let row = tx
            .query_one_raw(statement(
                "SELECT floor(extract(epoch FROM LEAST(s.expires_at,
                 COALESCE(s.proven_at,s.created_at) + interval '12 hours',
                 pg_catalog.clock_timestamp() + interval '5 minutes')))::bigint AS expiry
             FROM central_identity.sessions s JOIN central_identity.accounts a
                 ON a.id=s.account_id AND a.credential_revision=s.credential_revision
             WHERE s.digest=$1 AND s.kind='native' AND a.restored
                 AND s.expires_at > pg_catalog.clock_timestamp()
             FOR UPDATE OF s",
                vec![digest.clone().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        let expiry: i64 = row.try_get("", "expiry").map_err(|_| Error::Unavailable)?;
        // Session lock serializes refreshes. Keep only the newest unexpired grant.
        tx.execute_raw(statement(
            "DELETE FROM central_identity.tower_access WHERE session_digest=$1
             AND (expires_at <= pg_catalog.clock_timestamp() OR id IN (
                 SELECT id FROM central_identity.tower_access WHERE session_digest=$1
                 ORDER BY created_at DESC,id DESC OFFSET 1))",
            vec![digest.clone().into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        tx.query_one_raw(statement(
            "INSERT INTO central_identity.tower_access(digest,session_digest,slot,origin,expires_at)
             SELECT $2,$1,CASE WHEN EXISTS(SELECT 1 FROM central_identity.tower_access
                 WHERE session_digest=$1 AND slot=0) THEN 1 ELSE 0 END,$3,to_timestamp($4::bigint)
             WHERE to_timestamp($4::bigint) > pg_catalog.clock_timestamp() RETURNING id",
            vec![digest.into(), fresh.digest().to_vec().into(), origin.into(), expiry.into()],
        )).await.map_err(|_| Error::Unavailable)?.ok_or(Error::Rejected)?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(expiry)
    }

    /// Recheck session, credential revision, restoration, terms and current profile.
    /// No cached JWT assertion can substitute for this database read.
    /// # Errors
    /// Wrong scope/audience, revoked/expired session, missing terms or storage failure.
    pub async fn tower_identity(
        &self,
        token: &Token,
        origin: &str,
        terms: Option<&TermsVersion>,
    ) -> Result<Identity, Error> {
        let row = self.db.query_one_raw(statement(
            "SELECT a.id,a.display_name,a.tag,
                 floor(extract(epoch FROM t.expires_at))::bigint AS expiry,
                 ($3::text IS NULL OR EXISTS(SELECT 1 FROM central_identity.terms_acceptances v
                     WHERE v.account_id=a.id AND v.subject='central' AND v.version=$3 AND v.digest=$4)) AS accepted
             FROM central_identity.tower_access t JOIN central_identity.sessions s ON s.digest=t.session_digest
             JOIN central_identity.accounts a ON a.id=s.account_id AND a.credential_revision=s.credential_revision
             WHERE t.digest=$1 AND t.origin=$2 AND t.expires_at > pg_catalog.clock_timestamp()
                 AND s.expires_at > pg_catalog.clock_timestamp() AND s.kind='native' AND a.restored
                 AND COALESCE(s.proven_at,s.created_at) + interval '12 hours' > pg_catalog.clock_timestamp()",
            vec![token.digest().to_vec().into(), origin.into(),
                terms.map(|v| v.version.clone()).into(), terms.map(|v| v.digest.to_vec()).into()],
        )).await.map_err(|_| Error::Unavailable)?.ok_or(Error::Rejected)?;
        if !row
            .try_get::<bool>("", "accepted")
            .map_err(|_| Error::Unavailable)?
        {
            return Err(Error::TermsRequired);
        }
        let account: Uuid = row.try_get("", "id").map_err(|_| Error::Unavailable)?;
        let (display_name, tag) = profiles::columns(&row)?;
        Ok(Identity {
            origin: origin.into(),
            account: *account.as_bytes(),
            display_name,
            tag,
            expires_at: row.try_get("", "expiry").map_err(|_| Error::Unavailable)?,
        })
    }
}
