//! Browser-owned preferences and subscriptions; queue payloads contain no private content.
use super::{AccountStore, Error, Session, Token, statement};
use central_api::notifications::{NotificationRequest, NotificationState, Subscription};
use sea_orm::{ConnectionTrait, QueryResult, TransactionTrait, prelude::Uuid};
mod queue;
pub use queue::{PushJob, PushResult};
fn unavailable(_: impl Sized) -> Error {
    Error::Unavailable
}

async fn owner(db: &impl ConnectionTrait, cookie: &Token) -> Result<QueryResult, Error> {
    db.query_one_raw(statement(
        "SELECT a.id,a.credential_revision,EXISTS(SELECT 1 FROM central_identity.account_roles r WHERE r.account_id=a.id AND r.role='admin') AS admin
         FROM central_identity.accounts a JOIN central_identity.sessions s ON s.account_id=a.id AND s.credential_revision=a.credential_revision
         WHERE s.digest=$1 AND s.kind='browser' AND s.expires_at>pg_catalog.clock_timestamp() AND a.credential IS NOT NULL
         FOR UPDATE OF a,s", vec![Session::Browser(cookie).digest().to_vec().into()]
    )).await.map_err(unavailable)?.ok_or(Error::Rejected)
}

impl AccountStore {
    /// Read or mutate only the authenticated browser's preferences/subscriptions.
    /// # Errors
    /// Rejects stale/native sessions, cross-account subscriptions, unsupported endpoints
    /// and admin-only settings without current authority. At most ten installations.
    pub async fn notifications(
        &self,
        cookie: &Token,
        request: NotificationRequest,
    ) -> Result<NotificationState, Error> {
        let tx = self.db.begin().await.map_err(unavailable)?;
        let who = owner(&tx, cookie).await?;
        let me: Uuid = who.try_get("", "id").map_err(unavailable)?;
        let revision: i64 = who
            .try_get("", "credential_revision")
            .map_err(unavailable)?;
        let admin: bool = who.try_get("", "admin").map_err(unavailable)?;
        let endpoint = match request {
            NotificationRequest::Read { endpoint } => endpoint,
            NotificationRequest::Set { category, enabled } => {
                if category.admin() && !admin {
                    return Err(Error::Rejected);
                }
                tx.execute_raw(statement(
                    "INSERT INTO central_identity.notification_preferences(account_id,enabled) VALUES($1,CASE WHEN $3 THEN $4::smallint | $2::smallint ELSE $4::smallint & ~$2::smallint END)
                     ON CONFLICT(account_id) DO UPDATE SET enabled=CASE WHEN $3 THEN central_identity.notification_preferences.enabled | $2::smallint ELSE central_identity.notification_preferences.enabled & ~$2::smallint END",
                    vec![me.into(), category.bit().into(), enabled.into(), (if admin {31_i32} else {7_i32}).into()]
                )).await.map_err(unavailable)?;
                if !enabled {
                    tx.execute_raw(statement("DELETE FROM central_identity.push_jobs j USING central_identity.push_subscriptions s WHERE j.subscription=s.id AND s.account_id=$1 AND j.category=$2", vec![me.into(),category.bit().into()])).await.map_err(unavailable)?;
                }
                None
            }
            NotificationRequest::Subscribe { subscription } => {
                subscription.validate()?;
                tx.execute_raw(statement("INSERT INTO central_identity.notification_preferences(account_id,enabled) VALUES($1,$2) ON CONFLICT DO NOTHING",vec![me.into(),(if admin {31_i32} else {7_i32}).into()])).await.map_err(unavailable)?;
                subscribe(&tx, me, revision, cookie, &subscription).await?;
                Some(subscription.endpoint.clone())
            }
            NotificationRequest::Unsubscribe { endpoint } => {
                central_api::notifications::endpoint(&endpoint)?;
                tx.execute_raw(statement("DELETE FROM central_identity.push_subscriptions WHERE account_id=$1 AND endpoint=$2",vec![me.into(),endpoint.into()])).await.map_err(unavailable)?;
                None
            }
        };
        if let Some(value) = &endpoint {
            central_api::notifications::endpoint(value)?;
        }
        let row=tx.query_one_raw(statement(
            "SELECT COALESCE((SELECT enabled::int FROM central_identity.notification_preferences WHERE account_id=$1),$4::int) AS enabled,
             EXISTS(SELECT 1 FROM central_identity.push_subscriptions WHERE account_id=$1 AND credential_revision=$2 AND endpoint=$3 AND renewed_at>pg_catalog.clock_timestamp()-interval '90 days') AS subscribed",
            vec![me.into(),revision.into(),endpoint.into(),(if admin {31_i32} else {7_i32}).into()]
        )).await.map_err(unavailable)?.ok_or(Error::Unavailable)?;
        let enabled: i32 = row.try_get("", "enabled").map_err(unavailable)?;
        let state = NotificationState {
            enabled: u8::try_from(enabled & if admin { 31 } else { 7 }).map_err(unavailable)?,
            public_key: None,
            subscribed: row.try_get("", "subscribed").map_err(unavailable)?,
        };
        tx.commit().await.map_err(unavailable)?;
        Ok(state)
    }
}

async fn subscribe(
    db: &impl ConnectionTrait,
    me: Uuid,
    revision: i64,
    cookie: &Token,
    sub: &Subscription,
) -> Result<(), Error> {
    db.execute_raw(statement("DELETE FROM central_identity.push_subscriptions WHERE account_id=$1 AND (credential_revision<>$2 OR renewed_at<pg_catalog.clock_timestamp()-interval '90 days')",vec![me.into(),revision.into()])).await.map_err(unavailable)?;
    let row=db.query_one_raw(statement("SELECT count(*)::bigint AS n,COALESCE(bool_or(endpoint=$2),false) AS own FROM central_identity.push_subscriptions WHERE account_id=$1",vec![me.into(),sub.endpoint.clone().into()])).await.map_err(unavailable)?.ok_or(Error::Unavailable)?;
    if row.try_get::<i64>("", "n").map_err(unavailable)? >= 10
        && !row.try_get::<bool>("", "own").map_err(unavailable)?
    {
        return Err(Error::Busy);
    }
    let result=db.query_one_raw(statement(
        "INSERT INTO central_identity.push_subscriptions(account_id,credential_revision,session_digest,endpoint,p256dh,auth,nonce,lang) VALUES($1,$2,$3,$4,$5,$6,$7,$8)
         ON CONFLICT(endpoint) DO UPDATE SET credential_revision=EXCLUDED.credential_revision,session_digest=EXCLUDED.session_digest,p256dh=EXCLUDED.p256dh,auth=EXCLUDED.auth,nonce=EXCLUDED.nonce,lang=EXCLUDED.lang,renewed_at=pg_catalog.clock_timestamp()
         WHERE central_identity.push_subscriptions.account_id=EXCLUDED.account_id RETURNING id",
        vec![me.into(),revision.into(),Session::Browser(cookie).digest().to_vec().into(),sub.endpoint.clone().into(),sub.p256dh.clone().into(),sub.auth.clone().into(),sub.nonce.clone().into(),sub.lang.clone().into()]
    )).await.map_err(unavailable)?;
    if result.is_none() {
        return Err(Error::Conflict);
    }
    Ok(())
}
