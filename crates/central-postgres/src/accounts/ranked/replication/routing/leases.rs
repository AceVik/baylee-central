//! Lease and verification evidence come from Central's own clock and registry.
use super::{Error, Host, column, offers, statement, unavailable};
use central_api::ranked::replica::Role;
use sea_orm::{ConnectionTrait, DatabaseTransaction, QueryResult, prelude::Uuid};

pub(super) struct Health {
    pub(super) primary: bool,
    pub(super) standby: bool,
    pub(super) expired: bool,
}

// Ready can already own another offer. Defer maintenance when a concurrent
// heartbeat holds either role, preserving both roles until the next poll.
async fn can_maintain(db: &DatabaseTransaction, row: &QueryResult) -> Result<bool, Error> {
    let id: Uuid = column(row, "host_id")?;
    let backup: Option<Uuid> = column(row, "standby_id")?;
    let row=db.query_one_raw(statement("WITH assigned AS MATERIALIZED (
       SELECT h.host_id FROM central_identity.ranked_hosts h JOIN central_identity.gateway_approvals p
       ON p.gateway_id=h.host_id WHERE h.host_id IN ($1,$2)), locked AS MATERIALIZED (
       SELECT h.host_id FROM central_identity.ranked_hosts h JOIN central_identity.gateway_approvals p
       ON p.gateway_id=h.host_id WHERE h.host_id IN ($1,$2) ORDER BY h.host_id FOR SHARE OF h,p SKIP LOCKED)
       SELECT (SELECT count(*) FROM assigned)=(SELECT count(*) FROM locked) AS available",
       vec![id.into(),backup.into()])).await.map_err(unavailable)?.ok_or(Error::Unavailable)?;
    column(&row, "available")
}

pub(super) async fn health(db: &DatabaseTransaction, id: Uuid) -> Result<Health, Error> {
    let row=db.query_one_raw(statement("WITH slots AS (
        SELECT m.id, r.compatibility,m.lease_until,r.paused_until,m.host_id,m.ranked_epoch,r.primary_until,
            r.standby_id,r.standby_epoch,r.standby_until FROM central_identity.ranked_matches m
            JOIN central_identity.ranked_replicas r ON r.match_id=m.id WHERE m.id=$1),
        alive AS (
        SELECT assigned.leader FROM slots s
        CROSS JOIN LATERAL(VALUES (true,s.host_id,s.ranked_epoch,s.primary_until),
                                  (false,s.standby_id,s.standby_epoch,s.standby_until)) assigned(leader,id,epoch,until_at)
        JOIN central_identity.ranked_hosts h ON h.host_id=assigned.id
        JOIN central_identity.gateway_approvals p ON p.gateway_id=h.host_id
        WHERE p.enabled AND p.ranked_verified AND p.ranked_epoch=assigned.epoch AND h.ranked_epoch=assigned.epoch
         AND h.compatibility=s.compatibility AND assigned.until_at>clock_timestamp()
         AND h.continuity_since<=assigned.until_at-interval '45 seconds'
         AND h.seen_at>clock_timestamp()-interval '90 seconds' ORDER BY h.host_id FOR SHARE OF h,p)
        SELECT EXISTS(SELECT 1 FROM alive WHERE leader) AS primary_live,
         EXISTS(SELECT 1 FROM alive WHERE NOT leader) AS standby_live,
         (lease_until<=clock_timestamp() OR COALESCE(paused_until<=clock_timestamp(),false)) AS expired FROM slots",
        vec![id.into()])).await.map_err(unavailable)?.ok_or(Error::Rejected)?;
    Ok(Health {
        primary: column(&row, "primary_live")?,
        standby: column(&row, "standby_live")?,
        expired: column(&row, "expired")?,
    })
}

async fn proven(db: &DatabaseTransaction, id: Uuid) -> Result<bool, Error> {
    let row=db.query_one_raw(statement("SELECT EXISTS(SELECT 1 FROM central_identity.ranked_replicas r
      JOIN central_identity.ranked_witnesses w ON w.match_id=r.match_id AND w.generation=r.generation
       AND w.host_id=r.standby_id AND w.position=r.position AND w.chain=r.head_digest
       AND w.evidence=r.evidence WHERE r.match_id=$1 AND r.state_digest IS NOT NULL) AS confirmed",
      vec![id.into()])).await.map_err(unavailable)?.ok_or(Error::Unavailable)?;
    column(&row, "confirmed")
}

async fn pause(db: &DatabaseTransaction, id: Uuid) -> Result<(), Error> {
    super::super::clock::suspend(db, id).await?;
    db.execute_raw(statement(
        "UPDATE central_identity.ranked_replicas SET phase='paused',
     paused_until=COALESCE(paused_until, clock_timestamp()+interval '5 minutes')
     WHERE match_id=$1 AND phase IN ('playing','preparing')",
        vec![id.into()],
    ))
    .await
    .map_err(unavailable)?;
    Ok(())
}

async fn rotate(
    db: &DatabaseTransaction,
    id: Uuid,
    replacement: Option<&Host>,
) -> Result<(), Error> {
    super::super::clock::suspend(db, id).await?;
    db.execute_raw(statement(
        "UPDATE central_identity.ranked_replicas SET generation=generation+1,
      standby_id=$2,standby_epoch=$3,standby_until=CASE WHEN $2::uuid IS NULL THEN NULL
       ELSE clock_timestamp()+interval '45 seconds' END, phase='paused',
      paused_until=COALESCE(paused_until,clock_timestamp()+interval '5 minutes') WHERE match_id=$1",
        vec![
            id.into(),
            replacement.map(|h| h.id).into(),
            replacement.map(|h| h.epoch).into(),
        ],
    ))
    .await
    .map_err(unavailable)?;
    db.execute_raw(statement(
        "DELETE FROM central_identity.gateway_sids WHERE ranked_match=$1",
        vec![id.into()],
    ))
    .await
    .map_err(unavailable)?;
    Ok(())
}

// Caller holds the match and replica locks, in that order. No observation or
// takeover on the previous generation can race certification in this transaction.
pub(super) async fn maintain(db: &DatabaseTransaction, row: &QueryResult) -> Result<(), Error> {
    let id: Uuid = column(row, "id")?;
    let phase: String = column(row, "phase")?;
    if matches!(phase.as_str(), "finished" | "void") {
        return Ok(());
    }
    if !can_maintain(db, row).await? {
        return Ok(());
    }
    let live = health(db, id).await?;
    if live.expired || column::<String>(row, "state")? == "void" {
        return super::super::void(db, id).await;
    }
    if phase == "disputed" {
        return Ok(());
    }
    let primary: Uuid = column(row, "host_id")?;
    let backup: Option<Uuid> = column(row, "standby_id")?;
    if live.primary && live.standby {
        if phase == "playing"
            && (!super::privacy::consented(db, id, primary).await?
                || !super::privacy::consented(db, id, backup.ok_or(Error::Unavailable)?).await?)
        {
            return pause(db, id).await;
        }
        return Ok(());
    }
    let build: Vec<u8> = column(row, "compatibility")?;
    if !live.primary {
        if !live.standby
            || !proven(db, id).await?
            || !super::privacy::consented(db, id, backup.ok_or(Error::Unavailable)?).await?
        {
            return pause(db, id).await;
        }
        let leader = backup.ok_or(Error::Unavailable)?;
        // The former standby has executed this exact certified prefix. Its
        // evidence, the stable game identity and both home gateways survive.
        let changed=db.execute_raw(statement("UPDATE central_identity.ranked_matches m SET
           host_id=g.id,origin=g.origin,ranked_epoch=r.standby_epoch
           FROM central_identity.ranked_replicas r JOIN central_identity.gateways g ON g.id=r.standby_id
           WHERE r.match_id=$1 AND m.id=r.match_id",vec![id.into()])).await.map_err(unavailable)?;
        if changed.rows_affected() != 1 {
            return Err(Error::Conflict);
        }
        db.execute_raw(statement(
            "UPDATE central_identity.ranked_replicas SET primary_until=standby_until
          WHERE match_id=$1",
            vec![id.into()],
        ))
        .await
        .map_err(unavailable)?;
        let next = offers::replacement(db, leader, Some(primary), &build).await?;
        return rotate(db, id, next.as_ref()).await;
    }
    let next = offers::replacement(db, primary, backup, &build).await?;
    if next.is_some() || backup.is_some() {
        rotate(db, id, next.as_ref()).await
    } else {
        pause(db, id).await
    }
}

pub(super) async fn pulse(db: &DatabaseTransaction, id: Uuid, role: Role) -> Result<(), Error> {
    // Only the caller's still-live role can extend its lease; the generic offer never can.
    let field = match role {
        Role::Primary => "primary_until",
        Role::Standby => "standby_until",
    };
    let changed=db.execute_raw(statement(&format!("UPDATE central_identity.ranked_replicas SET
      {field}=clock_timestamp()+interval '45 seconds' WHERE match_id=$1 AND {field}>clock_timestamp()
      AND phase IN ('preparing','playing','paused')"),vec![id.into()])).await.map_err(unavailable)?;
    if changed.rows_affected() == 1 {
        Ok(())
    } else {
        Err(Error::Conflict)
    }
}
