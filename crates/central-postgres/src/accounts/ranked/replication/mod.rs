//! Every player intent is independently authenticated at Central. Both verified executors
//! certify exactly the same immutable prefix before another input or a score may commit.
use super::{Error, api, column, host, host::Host, privacy, queue, statement, unavailable};
use api::replica as rpc;
use sea_orm::{ConnectionTrait, DatabaseTransaction, QueryResult, prelude::Uuid};

mod clock;
mod evidence;
mod journal;
mod participants;
mod relay;
pub(super) mod routing;
mod seat;
mod service;

pub(super) use seat::{seat, seat_status};
pub(super) use service::{handle, legacy};

const MATCH: &str = "SELECT m.id,m.account_a,m.account_b,m.commitment_a,m.commitment_b,m.state,
 m.claimed_a,m.claimed_b,m.game_id,m.start_fingerprint,m.host_id,m.ranked_epoch,m.origin,m.outcome,m.format,
 r.match_id,r.compatibility,r.seed,r.generation,r.standby_id,r.standby_epoch,r.phase,r.position,r.head_digest,
 r.state_digest,r.evidence,r.initial_evidence,r.opening_chain,r.journal_bytes,
 r.primary_until>clock_timestamp() AS primary_live,
 COALESCE(r.standby_until>clock_timestamp(),false) AS standby_live,
 COALESCE(r.retire_at>clock_timestamp(),false) AS readable,
 COALESCE(r.paused_until<=clock_timestamp(),false) AS recovery_expired,
 m.lease_until<=clock_timestamp() AS match_expired
 FROM central_identity.ranked_matches m JOIN central_identity.ranked_replicas r ON r.match_id=m.id
 WHERE m.id=$1 FOR UPDATE OF r";

async fn exists(db: &DatabaseTransaction, sql: &str, id: Uuid) -> Result<bool, Error> {
    Ok(db
        .query_one_raw(statement(sql, vec![id.into()]))
        .await
        .map_err(unavailable)?
        .is_some())
}

pub(super) async fn protected(db: &DatabaseTransaction, id: Uuid) -> Result<bool, Error> {
    exists(
        db,
        "SELECT match_id FROM central_identity.ranked_replicas WHERE match_id=$1",
        id,
    )
    .await
}

pub(super) async fn lock_match(db: &DatabaseTransaction, id: Uuid) -> Result<QueryResult, Error> {
    if !exists(
        db,
        "SELECT id FROM central_identity.ranked_matches WHERE id=$1 FOR NO KEY UPDATE",
        id,
    )
    .await?
    {
        return Err(Error::Rejected);
    }
    db.query_one_raw(statement(MATCH, vec![id.into()]))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Rejected)
}

fn seed(row: &QueryResult) -> Result<u64, Error> {
    Ok(u64::from_be_bytes(
        column::<Vec<u8>>(row, "seed")?
            .try_into()
            .map_err(unavailable)?,
    ))
}

fn digest(row: &QueryResult, name: &str) -> Result<rpc::Digest, Error> {
    column::<Vec<u8>>(row, name)?
        .try_into()
        .map_err(unavailable)
}

fn head(row: &QueryResult) -> Result<rpc::Head, Error> {
    Ok(rpc::Head {
        position: u64::try_from(column::<i64>(row, "position")?).map_err(unavailable)?,
        chain: digest(row, "head_digest")?,
    })
}

fn generation(row: &QueryResult) -> Result<u64, Error> {
    u64::try_from(column::<i64>(row, "generation")?).map_err(unavailable)
}

fn phase(row: &QueryResult) -> Result<rpc::Phase, Error> {
    Ok(match column::<String>(row, "phase")?.as_str() {
        "preparing" => rpc::Phase::Preparing,
        "playing" => rpc::Phase::Playing,
        "paused" => rpc::Phase::Paused,
        "disputed" => rpc::Phase::Disputed,
        "finished" => rpc::Phase::Finished,
        "void" => rpc::Phase::Void,
        _ => return Err(Error::Unavailable),
    })
}

fn fenced(row: &QueryResult, fence: rpc::Fence) -> Result<(), Error> {
    if generation(row)? != fence.generation
        || column::<Uuid>(row, "id")?.into_bytes() != fence.match_id
    {
        return Err(Error::Conflict);
    }
    Ok(())
}

/// The caller's credential and grant are locked by the outer transaction.
/// A role is bound to a specific grant epoch and compatible offer.
async fn role(
    db: &DatabaseTransaction,
    row: &QueryResult,
    host: &Host,
) -> Result<rpc::Role, Error> {
    let role = if column::<Uuid>(row, "host_id")? == host.id
        && column::<i64>(row, "ranked_epoch")? == host.epoch
    {
        rpc::Role::Primary
    } else if column::<Option<Uuid>>(row, "standby_id")? == Some(host.id)
        && column::<Option<i64>>(row, "standby_epoch")? == Some(host.epoch)
    {
        rpc::Role::Standby
    } else {
        return Err(Error::Rejected);
    };
    if db.query_one_raw(statement("SELECT host_id FROM central_identity.ranked_hosts h JOIN central_identity.ranked_replicas r ON r.match_id=$4
        WHERE h.host_id=$1 AND h.ranked_epoch=$2 AND h.compatibility=$3 AND h.seen_at>clock_timestamp()-interval '90 seconds'
         AND h.continuity_since<=(CASE WHEN $5 THEN r.primary_until ELSE r.standby_until END)-interval '45 seconds'",
        vec![host.id.into(),host.epoch.into(),digest(row,"compatibility")?.to_vec().into(),column::<Uuid>(row,"id")?.into(),(role==rpc::Role::Primary).into()]))
        .await.map_err(unavailable)?.is_none(){return Err(Error::Rejected)}
    Ok(role)
}

async fn clock_availability(
    db: &DatabaseTransaction,
    id: Uuid,
) -> Result<(i64, bool, bool), Error> {
    let row = lock_match(db, id).await?;
    let healthy = phase(&row)? == rpc::Phase::Playing && routing::healthy(db, &row).await?;
    let pending = journal::pending(db, id).await?;
    Ok((column(&row, "generation")?, healthy, pending))
}

/// Technical abandonment is a single atomic transition and never changes ratings.
pub(super) async fn void(db: &DatabaseTransaction, id: Uuid) -> Result<(), Error> {
    db.execute_raw(statement(
        "UPDATE central_identity.ranked_matches SET state='void',deck_a=NULL,deck_b=NULL,
        finished_at=clock_timestamp() WHERE id=$1 AND state IN ('reserved','playing')",
        vec![id.into()],
    ))
    .await
    .map_err(unavailable)?;
    db.execute_raw(statement(
        "UPDATE central_identity.ranked_replicas SET phase='void',paused_until=NULL,
        retire_at=clock_timestamp() WHERE match_id=$1 AND phase NOT IN ('finished','void')",
        vec![id.into()],
    ))
    .await
    .map_err(unavailable)?;
    Ok(())
}

pub(in crate::accounts) async fn sweep(db: &DatabaseTransaction) -> Result<(), Error> {
    let rows = db
        .query_all_raw(statement(include_str!("due.sql"), vec![]))
        .await
        .map_err(unavailable)?;
    for row in rows {
        let id = column(&row, "id")?;
        if matches!(
            column::<String>(&row, "phase")?.as_str(),
            "finished" | "void"
        ) {
            journal::purge(db, id).await?;
        } else {
            routing::maintain(db, id).await?;
        }
    }
    Ok(())
}
