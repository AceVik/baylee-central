use super::{
    AccountStore, ConnectionTrait, Error, Subscription, TransactionTrait, Uuid, statement,
    unavailable,
};
use central_api::notifications::Category;

/// One leased delivery hint. Debug deliberately exposes none of its capability material.
pub struct PushJob {
    /// PostgreSQL-generated job ID.
    pub id: [u8; 16],
    /// Coalescing revision: an old acknowledgement cannot delete a newer event.
    pub revision: i64,
    /// Unique database-generated lease; a timed-out worker cannot acknowledge its successor.
    pub lease: [u8; 16],
    /// Fixed category only, without private content.
    pub category: Category,
    /// Browser-owned destination, keys, language and installation generation.
    pub subscription: Subscription,
}
/// A provider outcome, without URLs, tokens, response bodies or private errors.
pub enum PushResult {
    /// Provider accepted the notification.
    Delivered,
    /// Provider permanently removed this subscription (404/410).
    Gone,
    /// Retry with bounded backoff, never hold the chat transaction open.
    Retry,
}
impl AccountStore {
    /// Lease one due job; current consent, credential and admin role are rechecked.
    /// # Errors
    /// Database failure, without exposing subscription capabilities.
    pub async fn claim_push(&self) -> Result<Option<PushJob>, Error> {
        let tx = self.db.begin().await.map_err(unavailable)?;
        tx.execute_raw(statement(
            "DELETE FROM central_identity.push_jobs WHERE expires_at<=pg_catalog.clock_timestamp()",
            vec![],
        ))
        .await
        .map_err(unavailable)?;
        let row=tx.query_one_raw(statement(
            "SELECT j.id,j.revision,j.category,s.endpoint,s.p256dh,s.auth,s.nonce,s.lang FROM central_identity.push_jobs j
             JOIN central_identity.push_subscriptions s ON s.id=j.subscription
             JOIN central_identity.accounts a ON a.id=s.account_id AND a.credential_revision=s.credential_revision
             JOIN central_identity.notification_preferences p ON p.account_id=a.id AND (p.enabled & j.category)<>0
             WHERE a.credential IS NOT NULL AND j.due_at<=pg_catalog.clock_timestamp() AND (j.claimed_until IS NULL OR j.claimed_until<pg_catalog.clock_timestamp())
             AND s.renewed_at>pg_catalog.clock_timestamp()-interval '90 days'
             AND (j.category<8 OR EXISTS(SELECT 1 FROM central_identity.account_roles r WHERE r.account_id=a.id AND r.role='admin'))
             ORDER BY j.due_at,j.id LIMIT 1 FOR UPDATE OF j SKIP LOCKED",vec![]
        )).await.map_err(unavailable)?;
        let Some(row) = row else {
            tx.commit().await.map_err(unavailable)?;
            return Ok(None);
        };
        let id: Uuid = row.try_get("", "id").map_err(unavailable)?;
        let leased = tx.query_one_raw(statement("UPDATE central_identity.push_jobs SET claimed_until=pg_catalog.clock_timestamp()+interval '30 seconds',lease=pg_catalog.uuidv7() WHERE id=$1 RETURNING lease",vec![id.into()])).await.map_err(unavailable)?.ok_or(Error::Unavailable)?;
        let lease: Uuid = leased.try_get("", "lease").map_err(unavailable)?;
        let category = match row.try_get::<i16>("", "category").map_err(unavailable)? {
            1 => Category::Chat,
            2 => Category::Contacts,
            4 => Category::Updates,
            8 => Category::Feedback,
            16 => Category::Gateways,
            _ => return Err(Error::Unavailable),
        };
        let job = PushJob {
            id: *id.as_bytes(),
            lease: *lease.as_bytes(),
            revision: row.try_get("", "revision").map_err(unavailable)?,
            category,
            subscription: Subscription {
                endpoint: row.try_get("", "endpoint").map_err(unavailable)?,
                p256dh: row.try_get("", "p256dh").map_err(unavailable)?,
                auth: row.try_get("", "auth").map_err(unavailable)?,
                nonce: row.try_get("", "nonce").map_err(unavailable)?,
                lang: row.try_get("", "lang").map_err(unavailable)?,
            },
        };
        tx.commit().await.map_err(unavailable)?;
        Ok(Some(job))
    }
    /// Complete only the claimed revision; a concurrent newer event remains queued.
    /// # Errors
    /// Database failure; a lease then expires and can be retried.
    pub async fn finish_push(&self, job: &PushJob, result: PushResult) -> Result<(), Error> {
        let id = Uuid::from_bytes(job.id);
        let lease = Uuid::from_bytes(job.lease);
        let (sql, values) = match result {
            PushResult::Gone => (
                "DELETE FROM central_identity.push_subscriptions s USING central_identity.push_jobs j WHERE s.endpoint=$1 AND s.nonce=$2 AND j.subscription=s.id AND j.id=$3 AND j.lease=$4",
                vec![
                    job.subscription.endpoint.clone().into(),
                    job.subscription.nonce.clone().into(),
                    id.into(),
                    lease.into(),
                ],
            ),
            PushResult::Delivered => (
                "DELETE FROM central_identity.push_jobs WHERE id=$1 AND revision=$2 AND lease=$3",
                vec![id.into(), job.revision.into(), lease.into()],
            ),
            PushResult::Retry => (
                "UPDATE central_identity.push_jobs SET claimed_until=NULL,lease=NULL,attempts=LEAST(attempts+1,10),due_at=pg_catalog.clock_timestamp()+make_interval(secs=>LEAST(300,5*(1<<LEAST(attempts,6)))) WHERE id=$1 AND revision=$2 AND lease=$3",
                vec![id.into(), job.revision.into(), lease.into()],
            ),
        };
        self.db
            .execute_raw(statement(sql, values))
            .await
            .map_err(unavailable)?;
        // If an event arrived during delivery, release the old lease without
        // resetting that event's content or deleting its job.
        self.db.execute_raw(statement("UPDATE central_identity.push_jobs SET claimed_until=NULL,lease=NULL WHERE id=$1 AND revision<>$2 AND lease=$3",vec![id.into(),job.revision.into(),lease.into()])).await.map_err(unavailable)?;
        Ok(())
    }
    /// Announce each deployed service release once; no caller-supplied message text.
    /// # Errors
    /// Invalid version or storage failure.
    pub async fn announce_release(&self, version: &str) -> Result<(), Error> {
        if version.is_empty()
            || version.len() > 100
            || !version
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'+'))
        {
            return Err(Error::Rejected);
        }
        self.db.execute_raw(statement("INSERT INTO central_identity.push_releases(version) VALUES($1) ON CONFLICT DO NOTHING",vec![version.into()])).await.map_err(unavailable)?;
        Ok(())
    }
}
