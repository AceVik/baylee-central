use super::{
    AccountStore, Error, Identity, NOT_DISABLED, Token, any_origin, identity, origins, statement,
    uuid,
};
use central_api::wire::TicketPurpose;
use sea_orm::{ConnectionTrait, TransactionTrait};

/// Successful redemption metadata; the caller sends the separately generated lease only to a gateway.
pub type Admission = (Identity, Option<String>, Option<i64>);

impl AccountStore {
    /// Redeem a purpose/audience-bound ticket and optionally delegate a live central session.
    /// `gateway=None` is reserved for the already-authenticated official migration bridge.
    /// `audiences` are the origins the redeeming gateway answers for (the
    /// official gateway's several); the lease keeps the ticket's own.
    ///
    /// # Errors
    /// Rejects replay, revocation, mismatched purpose/audience, disabled gateway or missing admin rights.
    pub async fn redeem_lease<S: AsRef<str> + Sync>(
        &self,
        ticket: &Token,
        audiences: &[S],
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
            &format!("SELECT session_digest,audience FROM central_identity.gateway_tickets WHERE digest=$1 AND audience={}
             AND purpose=$3 AND expires_at>pg_catalog.clock_timestamp()", any_origin(2)),
            vec![ticket.digest().to_vec().into(),origins(audiences)?,purpose.as_str().into()]
        )).await.map_err(|_|Error::Unavailable)?.ok_or(Error::Rejected)?;
        let session: Vec<u8> = spent
            .try_get("", "session_digest")
            .map_err(|_| Error::Unavailable)?;
        // the origin the client asked for, one of `audiences`
        let audience: String = spent
            .try_get("", "audience")
            .map_err(|_| Error::Unavailable)?;
        let row=tx.query_one_raw(statement(
            &format!("SELECT a.id,a.vault_epoch,a.legacy_id,
             floor(extract(epoch FROM s.expires_at))::bigint AS expires_at
             FROM central_identity.sessions s JOIN central_identity.accounts a
             ON a.id=s.account_id AND a.credential_revision=s.credential_revision
             WHERE s.digest=$1 AND s.expires_at>pg_catalog.clock_timestamp() AND a.restored
             AND ($2::uuid IS NOT NULL OR a.legacy_id IS NULL OR a.legacy_closed)
             AND ($2::uuid IS NULL OR EXISTS(SELECT 1 FROM central_identity.gateways g
                 WHERE g.id=$2 AND g.origin=$3 AND {NOT_DISABLED} AND ($4='admit' OR EXISTS
                     (SELECT 1 FROM central_identity.gateway_admins m WHERE m.gateway_id=g.id AND m.account_id=a.id))))
             FOR UPDATE OF s"),vec![session.clone().into(),gateway.map(uuid).into(),audience.clone().into(),purpose.as_str().into()]
        )).await.map_err(|_|Error::Unavailable)?.ok_or(Error::Rejected)?;
        let removed = tx
            .execute_raw(statement(
                "DELETE FROM central_identity.gateway_tickets WHERE digest=$1 AND session_digest=$2
             AND audience=$3 AND purpose=$4 AND expires_at>pg_catalog.clock_timestamp()",
                vec![
                    ticket.digest().to_vec().into(),
                    session.clone().into(),
                    audience.clone().into(),
                    purpose.as_str().into(),
                ],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        if removed.rows_affected() != 1 {
            return Err(Error::Rejected);
        }
        let identity = identity(&row)?;
        // Only the bridge learns a legacy ID, and only for a closed move (the
        // query refuses an open one), so the gateway may link it at once; a
        // registered account has none.
        let legacy = if gateway.is_none() {
            row.try_get("", "legacy_id")
                .map_err(|_| Error::Unavailable)?
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
    /// Gateway authentication and the choice of official-bridge authority precede this method;
    /// `audiences` are the origins that gateway answers for.
    ///
    /// # Errors
    /// Rejects expiry, central logout, wrong gateway/purpose, a disabled gateway and removed
    /// administrator membership.
    pub async fn gateway_session<S: AsRef<str> + Sync>(
        &self,
        lease: &Token,
        audiences: &[S],
        gateway: Option<[u8; 16]>,
        purpose: TicketPurpose,
    ) -> Result<(Identity, i64), Error> {
        let (identity, expires_at, _, _) = self
            .gateway_authorize(lease, audiences, gateway, purpose, None, None)
            .await?;
        Ok((identity, expires_at))
    }
}
