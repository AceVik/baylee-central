use super::{AccountStore, Error, Identity, Token, identity, statement, uuid};
use central_api::wire::TicketPurpose;
use sea_orm::{ConnectionTrait, TransactionTrait};

/// Successful redemption metadata; the caller sends the separately generated lease only to a gateway.
pub type Admission = (Identity, Option<String>, Option<i64>);

impl AccountStore {
    /// Redeem a purpose/audience-bound ticket and optionally delegate a live central session.
    /// `gateway=None` is reserved for the already-authenticated official migration bridge.
    ///
    /// # Errors
    /// Rejects replay, revocation, mismatched purpose/audience, disabled gateway or missing admin rights.
    pub async fn redeem_lease(
        &self,
        ticket: &Token,
        audience: &str,
        purpose: TicketPurpose,
        gateway: Option<[u8; 16]>,
        lease: Option<&Token>,
    ) -> Result<Admission, Error> {
        let delegates = matches!(purpose, TicketPurpose::Admit | TicketPurpose::Admin);
        if delegates != lease.is_some()
            || (gateway.is_some() && !delegates)
            || (gateway.is_none() && matches!(purpose, TicketPurpose::Admin))
        {
            return Err(Error::Rejected);
        }
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        // Lock the parent session before deleting a child ticket, matching logout's
        // cascading deletion order. The final DELETE still arbitrates one-use redemption.
        let spent=tx.query_one_raw(statement(
            "SELECT session_digest FROM central_identity.gateway_tickets WHERE digest=$1 AND audience=$2 AND purpose=$3
             AND expires_at>pg_catalog.clock_timestamp()",
            vec![ticket.digest().to_vec().into(),audience.into(),purpose.as_str().into()]
        )).await.map_err(|_|Error::Unavailable)?.ok_or(Error::Rejected)?;
        let session: Vec<u8> = spent
            .try_get("", "session_digest")
            .map_err(|_| Error::Unavailable)?;
        let row=tx.query_one_raw(statement(
            "SELECT a.id,a.vault_epoch,a.legacy_id,
             floor(extract(epoch FROM s.expires_at))::bigint AS expires_at
             FROM central_identity.sessions s JOIN central_identity.accounts a
             ON a.id=s.account_id AND a.credential_revision=s.credential_revision
             WHERE s.digest=$1 AND s.expires_at>pg_catalog.clock_timestamp() AND a.restored
             AND ($2::uuid IS NULL OR EXISTS(SELECT 1 FROM central_identity.gateways g
                 JOIN central_identity.gateway_approvals p ON p.gateway_id=g.id AND p.enabled
                 WHERE g.id=$2 AND g.origin=$3 AND ($4='admit' OR EXISTS
                     (SELECT 1 FROM central_identity.gateway_admins m WHERE m.gateway_id=g.id AND m.account_id=a.id))))
             FOR UPDATE OF s",vec![session.clone().into(),gateway.map(uuid).into(),audience.into(),purpose.as_str().into()]
        )).await.map_err(|_|Error::Unavailable)?.ok_or(Error::Rejected)?;
        let removed = tx
            .execute_raw(statement(
                "DELETE FROM central_identity.gateway_tickets WHERE digest=$1 AND session_digest=$2
             AND audience=$3 AND purpose=$4 AND expires_at>pg_catalog.clock_timestamp()",
                vec![
                    ticket.digest().to_vec().into(),
                    session.clone().into(),
                    audience.into(),
                    purpose.as_str().into(),
                ],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        if removed.rows_affected() != 1 {
            return Err(Error::Rejected);
        }
        let identity = identity(&row)?;
        let legacy = if gateway.is_none() {
            Some(
                row.try_get("", "legacy_id")
                    .map_err(|_| Error::Unavailable)?,
            )
        } else {
            None
        };
        let expires = if let Some(lease) = lease {
            tx.execute_raw(statement(
                "DELETE FROM central_identity.gateway_leases WHERE session_digest=$1 AND expires_at<=pg_catalog.clock_timestamp()",
                vec![session.clone().into()]
            )).await.map_err(|_|Error::Unavailable)?;
            let inserted=tx.execute_raw(statement(
                "INSERT INTO central_identity.gateway_leases(digest,session_digest,gateway_id,audience,purpose,expires_at)
                 SELECT $1,s.digest,$3,$4,$5,s.expires_at FROM central_identity.sessions s WHERE s.digest=$2
                 AND s.expires_at>pg_catalog.clock_timestamp()
                 AND (SELECT count(*) FROM central_identity.gateway_leases WHERE session_digest=$2)<32",
                vec![lease.digest().to_vec().into(),session.into(),gateway.map(uuid).into(),audience.into(),purpose.as_str().into()]
            )).await.map_err(|_|Error::Unavailable)?;
            if inserted.rows_affected() != 1 {
                return Err(Error::Busy);
            }
            Some(
                row.try_get("", "expires_at")
                    .map_err(|_| Error::Unavailable)?,
            )
        } else {
            None
        };
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok((identity, legacy, expires))
    }

    /// Validate a gateway-held lease against current session, approval and administrator state.
    /// Gateway authentication and the choice of official-bridge authority precede this method.
    ///
    /// # Errors
    /// Rejects expiry, central logout, wrong gateway/purpose and removed administrator membership.
    pub async fn gateway_session(
        &self,
        lease: &Token,
        audience: &str,
        gateway: Option<[u8; 16]>,
        purpose: TicketPurpose,
    ) -> Result<(Identity, i64), Error> {
        let (identity, expires_at, _, _) = self
            .gateway_authorize(lease, audience, gateway, purpose, None, None)
            .await?;
        Ok((identity, expires_at))
    }
}
