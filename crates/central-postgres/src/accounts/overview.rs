//! The administration's first view (`AdminRequest::Overview`,
//! `docs/administration.md` §"Reads"): how many accounts and gateways there
//! are, by kind and state, in one statement of counts. No row leaves.
use super::tokens::LIVE_TOKEN;
use super::{AccountStore, Error, Token, statement};
use central_api::admin::AdminOverview;
use sea_orm::{ConnectionTrait, TransactionTrait};

impl AccountStore {
    /// The counts for the administrator behind `browser`.
    ///
    /// # Errors
    /// `Rejected` without an administrator's live browser session; storage
    /// failure.
    pub async fn admin_overview(&self, browser: &Token) -> Result<AdminOverview, Error> {
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        Self::administrator(&tx, browser).await?;
        // One pass over each table; the official gateway is not a registered one.
        let row = tx
            .query_one_raw(statement(
                &format!(
                    "SELECT a.accounts, a.accounts_week, a.moved, r.admins,
                         g.pending, g.approved, g.disabled, g.online, t.live_tokens
                     FROM (SELECT count(*) FILTER (WHERE credential IS NOT NULL) AS accounts,
                               count(*) FILTER (WHERE credential IS NOT NULL
                                   AND created_at > pg_catalog.clock_timestamp() - interval '7 days') AS accounts_week,
                               count(*) FILTER (WHERE credential IS NOT NULL AND source = 'legacy') AS moved
                           FROM central_identity.accounts) a,
                          (SELECT count(*) AS admins FROM central_identity.account_roles WHERE role = 'admin') r,
                          (SELECT count(*) FILTER (WHERE p.gateway_id IS NULL) AS pending,
                               count(*) FILTER (WHERE p.enabled) AS approved,
                               count(*) FILTER (WHERE NOT p.enabled) AS disabled,
                               count(*) FILTER (WHERE g.last_seen > pg_catalog.clock_timestamp() - interval '90 seconds') AS online
                           FROM central_identity.gateways g
                           LEFT JOIN central_identity.gateway_approvals p ON p.gateway_id = g.id
                           WHERE NOT g.official) g,
                          (SELECT count(*) AS live_tokens FROM central_identity.gateway_tokens t WHERE {LIVE_TOKEN}) t"
                ),
                vec![],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)?;
        let count = |column: &str| {
            row.try_get::<i64>("", column)
                .ok()
                .and_then(|n| u64::try_from(n).ok())
                .ok_or(Error::Unavailable)
        };
        let overview = AdminOverview {
            accounts: count("accounts")?,
            accounts_week: count("accounts_week")?,
            moved: count("moved")?,
            admins: count("admins")?,
            pending: count("pending")?,
            approved: count("approved")?,
            disabled: count("disabled")?,
            online: count("online")?,
            live_tokens: count("live_tokens")?,
        };
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(overview)
    }
}
