use super::{AccountStore, Error, Token, number, statement, uuid};
use central_api::{
    gateway::{GatewayListing, GatewayMetadata, validate_origin},
    wire::TicketPurpose,
};
use sea_orm::{ConnectionTrait, DatabaseTransaction, TransactionTrait, prelude::Uuid};

fn valid_id(id: [u8; 16]) -> bool {
    id[6] >> 4 == 7 && id[8] >> 6 == 2
}

async fn lock_owner(tx: &DatabaseTransaction, session: &Token) -> Result<Uuid, Error> {
    let owner = super::objects::locked_owner(tx, session, false).await?;
    let row = tx
        .query_one_raw(statement(
            "SELECT restored FROM central_identity.accounts WHERE id=$1",
            vec![owner.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::Rejected)?;
    if !row
        .try_get::<bool>("", "restored")
        .map_err(|_| Error::Unavailable)?
    {
        return Err(Error::Rejected);
    }
    Ok(owner)
}

impl AccountStore {
    /// Reserve a gateway for the authenticated account; separate operator approval is mandatory.
    ///
    /// # Errors
    /// An owned, never-approved reservation can be retried to recover a lost credential response.
    ///
    /// Rejects invalid metadata/session, foreign or approved origins and the owner quota.
    pub async fn register_gateway(
        &self,
        session: &Token,
        origin: &str,
        metadata: &GatewayMetadata,
        credential: &Token,
        heartbeat: &Token,
        development: bool,
    ) -> Result<[u8; 16], Error> {
        validate_origin(origin, development)?;
        metadata.validate(origin)?;
        if credential.digest() == heartbeat.digest() {
            return Err(Error::Rejected);
        }
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let owner = lock_owner(&tx, session).await?;
        let row = tx.query_one_raw(statement(
            "INSERT INTO central_identity.gateways(owner_id,origin,name,version,website,credential_digest,heartbeat_digest)
             SELECT $1,$2,$3,$4,$5,$6,$7 WHERE (SELECT count(*) FROM central_identity.gateways WHERE owner_id=$1)<8
             OR EXISTS(SELECT 1 FROM central_identity.gateways g WHERE g.origin=$2 AND g.owner_id=$1
                 AND NOT EXISTS(SELECT 1 FROM central_identity.gateway_approvals p WHERE p.gateway_id=g.id))
             ON CONFLICT(origin) DO UPDATE SET (name,version,website,credential_digest,heartbeat_digest) =
                 (EXCLUDED.name,EXCLUDED.version,EXCLUDED.website,EXCLUDED.credential_digest,EXCLUDED.heartbeat_digest)
             WHERE gateways.owner_id=EXCLUDED.owner_id AND NOT EXISTS
                 (SELECT 1 FROM central_identity.gateway_approvals p WHERE p.gateway_id=gateways.id)
             RETURNING id",
            vec![owner.into(),origin.into(),metadata.name.clone().into(),metadata.version.clone().into(),metadata.website.clone().into(),credential.digest().to_vec().into(),heartbeat.digest().to_vec().into()]
        )).await.map_err(|_| Error::Conflict)?.ok_or(Error::Busy)?;
        let id: Uuid = row.try_get("", "id").map_err(|_| Error::Unavailable)?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(*id.as_bytes())
    }

    /// Return a bounded page of approved gateways; no private account or credential fields.
    ///
    /// # Errors
    /// Rejects invalid cursors and unavailable storage.
    pub async fn gateway_directory(
        &self,
        after: Option<[u8; 16]>,
    ) -> Result<Vec<GatewayListing>, Error> {
        if after.is_some_and(|id| !valid_id(id)) {
            return Err(Error::Rejected);
        }
        let rows = self.db.query_all_raw(statement(
            "SELECT g.id,g.origin,g.name,g.version,g.website,
             COALESCE(g.last_seen > pg_catalog.clock_timestamp()-interval '90 seconds',false) AS online
             FROM central_identity.gateways g JOIN central_identity.gateway_approvals p ON p.gateway_id=g.id AND p.enabled
             WHERE ($1::uuid IS NULL OR g.id>$1) ORDER BY g.id LIMIT 100", vec![after.map(uuid).into()]
        )).await.map_err(|_| Error::Unavailable)?;
        rows.into_iter()
            .map(|row| {
                let id: Uuid = row.try_get("", "id").map_err(|_| Error::Unavailable)?;
                Ok(GatewayListing {
                    id: *id.as_bytes(),
                    origin: row.try_get("", "origin").map_err(|_| Error::Unavailable)?,
                    metadata: GatewayMetadata {
                        name: row.try_get("", "name").map_err(|_| Error::Unavailable)?,
                        version: row.try_get("", "version").map_err(|_| Error::Unavailable)?,
                        website: row.try_get("", "website").map_err(|_| Error::Unavailable)?,
                    },
                    online: row.try_get("", "online").map_err(|_| Error::Unavailable)?,
                })
            })
            .collect()
    }

    /// Authenticate only a gateway's redemption credential, never its heartbeat credential.
    ///
    /// # Errors
    /// Rejects unapproved/disabled gateways and incorrect credentials.
    pub async fn authenticate_gateway(
        &self,
        credential: &Token,
    ) -> Result<([u8; 16], String), Error> {
        let row=self.db.query_one_raw(statement(
            "SELECT g.id,g.origin FROM central_identity.gateways g JOIN central_identity.gateway_approvals p
             ON p.gateway_id=g.id AND p.enabled WHERE g.credential_digest=$1", vec![credential.digest().to_vec().into()]
        )).await.map_err(|_| Error::Unavailable)?.ok_or(Error::Rejected)?;
        let id: Uuid = row.try_get("", "id").map_err(|_| Error::Unavailable)?;
        Ok((
            *id.as_bytes(),
            row.try_get("", "origin").map_err(|_| Error::Unavailable)?,
        ))
    }

    /// Update public metadata and central-clock liveness only, at most once per five seconds.
    ///
    /// # Errors
    /// Rejects wrong credentials, unapproved gateways, unsafe metadata and excessive heartbeat rate.
    pub async fn gateway_heartbeat(
        &self,
        token: &Token,
        metadata: &GatewayMetadata,
    ) -> Result<(), Error> {
        let row=self.db.query_one_raw(statement(
            "SELECT g.origin FROM central_identity.gateways g JOIN central_identity.gateway_approvals p
             ON p.gateway_id=g.id AND p.enabled WHERE g.heartbeat_digest=$1",vec![token.digest().to_vec().into()]
        )).await.map_err(|_|Error::Unavailable)?.ok_or(Error::Rejected)?;
        let origin: String = row.try_get("", "origin").map_err(|_| Error::Unavailable)?;
        metadata.validate(&origin)?;
        let updated=self.db.execute_raw(statement(
            "UPDATE central_identity.gateways g SET name=$2,version=$3,website=$4,last_seen=pg_catalog.clock_timestamp()
             WHERE heartbeat_digest=$1 AND (last_seen IS NULL OR last_seen<=pg_catalog.clock_timestamp()-interval '5 seconds')
             AND EXISTS(SELECT 1 FROM central_identity.gateway_approvals p WHERE p.gateway_id=g.id AND p.enabled)",
            vec![token.digest().to_vec().into(),metadata.name.clone().into(),metadata.version.clone().into(),metadata.website.clone().into()]
        )).await.map_err(|_|Error::Unavailable)?;
        if updated.rows_affected() != 1 {
            return Err(Error::Busy);
        }
        Ok(())
    }

    /// Replace one owned gateway's administrator list under an account/session and revision check.
    /// Registry ownership alone never authorizes access to the gateway's admin backend.
    ///
    /// # Errors
    /// Rejects foreign ownership, stale revision, invalid/duplicate accounts and oversized lists.
    pub async fn set_gateway_admins(
        &self,
        session: &Token,
        gateway: [u8; 16],
        expected: u64,
        accounts: &[[u8; 16]],
    ) -> Result<u64, Error> {
        if !valid_id(gateway) || accounts.len() > 32 || accounts.iter().any(|id| !valid_id(*id)) {
            return Err(Error::Rejected);
        }
        let unique: std::collections::BTreeSet<_> = accounts.iter().collect();
        if unique.len() != accounts.len() {
            return Err(Error::Rejected);
        }
        let expected = number(expected)?;
        if expected < 1 || expected == i64::MAX {
            return Err(Error::Rejected);
        }
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let owner = lock_owner(&tx, session).await?;
        let result = tx
            .execute_raw(statement(
                "UPDATE central_identity.gateways SET admin_revision=admin_revision+1
             WHERE id=$1 AND owner_id=$2 AND admin_revision=$3",
                vec![uuid(gateway).into(), owner.into(), expected.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        if result.rows_affected() != 1 {
            return Err(Error::Conflict);
        }
        tx.execute_raw(statement(
            "DELETE FROM central_identity.gateway_admins WHERE gateway_id=$1",
            vec![uuid(gateway).into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        for account in accounts {
            let result=tx.execute_raw(statement(
                "INSERT INTO central_identity.gateway_admins(gateway_id,account_id)
                 SELECT $1,id FROM central_identity.accounts WHERE id=$2 AND credential IS NOT NULL",
                vec![uuid(gateway).into(),uuid(*account).into()]
            )).await.map_err(|_|Error::Unavailable)?;
            if result.rows_affected() != 1 {
                return Err(Error::Rejected);
            }
        }
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        u64::try_from(expected + 1).map_err(|_| Error::Unavailable)
    }

    /// Check one account's current role on an already authenticated gateway.
    ///
    /// # Errors
    /// Rejects invalid IDs or unavailable storage; approval removal returns false.
    pub async fn gateway_admin(&self, gateway: [u8; 16], account: [u8; 16]) -> Result<bool, Error> {
        if !valid_id(gateway) || !valid_id(account) {
            return Err(Error::Rejected);
        }
        let row=self.db.query_one_raw(statement(
            "SELECT EXISTS(SELECT 1 FROM central_identity.gateways g JOIN central_identity.gateway_approvals p
             ON p.gateway_id=g.id AND p.enabled WHERE g.id=$1 AND EXISTS
             (SELECT 1 FROM central_identity.gateway_admins m WHERE m.gateway_id=g.id AND m.account_id=$2)) AS allowed",
            vec![uuid(gateway).into(),uuid(account).into()]
        )).await.map_err(|_|Error::Unavailable)?.ok_or(Error::Unavailable)?;
        row.try_get("", "allowed").map_err(|_| Error::Unavailable)
    }

    /// Resolve an approved audience and check Admin membership before issuing a ticket.
    ///
    /// # Errors
    /// Rejects unknown/disabled origins, invalid sessions and foreign admin membership.
    pub async fn registered_ticket(
        &self,
        session: &Token,
        origin: &str,
        ticket: &Token,
        purpose: TicketPurpose,
    ) -> Result<(), Error> {
        if !matches!(purpose, TicketPurpose::Admit | TicketPurpose::Admin) {
            return Err(Error::Rejected);
        }
        let row=self.db.query_one_raw(statement(
            "SELECT g.id FROM central_identity.gateways g JOIN central_identity.gateway_approvals p
             ON p.gateway_id=g.id AND p.enabled JOIN central_identity.sessions s ON s.digest=$2
             JOIN central_identity.accounts a ON a.id=s.account_id AND a.credential_revision=s.credential_revision
             WHERE g.origin=$1 AND s.expires_at>pg_catalog.clock_timestamp() AND a.restored
             AND ($3='admit' OR EXISTS(SELECT 1 FROM central_identity.gateway_admins m
                 WHERE m.gateway_id=g.id AND m.account_id=a.id))",
            vec![origin.into(),session.digest().to_vec().into(),purpose.as_str().into()]
        )).await.map_err(|_|Error::Unavailable)?;
        if row.is_none() {
            return Err(Error::Rejected);
        }
        self.gateway_ticket(session, origin, ticket, purpose).await
    }
}
