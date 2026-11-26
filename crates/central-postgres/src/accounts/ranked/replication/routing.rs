//! Verified, capacity-bounded placement and generation-fenced host routing.
use super::super::{
    Error, api, column,
    host::{self, Host},
    privacy, statement, unavailable,
};
use api::replica::{Digest, Fence, HostResponse, Phase, Role, Work};
use sea_orm::{ConnectionTrait, DatabaseTransaction, QueryResult, prelude::Uuid};

mod leases;
mod offers;
pub(in crate::accounts::ranked) use offers::{Allocation, allocate, allocate_single};

pub(in crate::accounts::ranked) async fn assign(
    db: &DatabaseTransaction,
    id: Uuid,
    standby: &Host,
    compatibility: &[u8],
) -> Result<(), Error> {
    db.execute_raw(statement("INSERT INTO central_identity.ranked_replicas(match_id,standby_id,standby_epoch,compatibility)
         VALUES ($1,$2,$3,$4)",vec![id.into(),standby.id.into(),standby.epoch.into(),compatibility.to_vec().into()])).await.map_err(unavailable)?;
    Ok(())
}

pub(super) async fn ready(
    db: &DatabaseTransaction,
    host: &Host,
    capacity: u8,
    compatibility: &Digest,
) -> Result<(), Error> {
    host::ready(db, host, capacity, Some(compatibility)).await?;
    Ok(())
}

pub(in crate::accounts::ranked) async fn maintain(
    db: &DatabaseTransaction,
    id: Uuid,
) -> Result<(), Error> {
    let row = super::lock_match(db, id).await?;
    leases::maintain(db, &row).await
}

pub(super) async fn healthy(db: &DatabaseTransaction, row: &QueryResult) -> Result<bool, Error> {
    let live = leases::health(db, column(row, "id")?).await?;
    if live.expired || !live.primary || !live.standby {
        return Ok(false);
    }
    privacy::consented(db, column(row, "id")?, column(row, "host_id")?).await
}

fn fence(row: &QueryResult) -> Result<Fence, Error> {
    Ok(Fence {
        match_id: column::<Uuid>(row, "id")?.into_bytes(),
        generation: super::generation(row)?,
    })
}

async fn assignment(
    db: &DatabaseTransaction,
    row: &QueryResult,
    host: &Host,
    role: Role,
) -> Result<HostResponse, Error> {
    let id = column(row, "id")?;
    let fence = fence(row)?;
    if !privacy::consented(db, id, host.id).await? {
        return Ok(HostResponse::AwaitingConsent { fence, role });
    }
    Ok(HostResponse::Assignment {
        work: Work {
            fence,
            game: column::<Option<Uuid>>(row, "game_id")?.map(Uuid::into_bytes),
            role,
            seed: super::seed(row)?,
            compatibility: super::digest(row, "compatibility")?,
            format: column::<String>(row, "format")?
                .parse()
                .map_err(unavailable)?,
            accounts: privacy::aliases(db, id, host.id).await?,
            decks: [
                super::digest(row, "commitment_a")?,
                super::digest(row, "commitment_b")?,
            ],
            phase: super::phase(row)?,
            head: super::head(row)?,
        },
    })
}

pub(super) async fn work(db: &DatabaseTransaction, host: &Host) -> Result<HostResponse, Error> {
    // One bounded read locks only idle matches. Gameplay keeps priority, and
    // subsequent polls can discover a skipped slot after its command commits.
    let rows=db.query_all_raw(statement("SELECT m.id,m.game_id,m.format,m.account_a,m.account_b,
       m.commitment_a,m.commitment_b,r.generation,r.compatibility,r.seed,r.phase,r.position,r.head_digest,
       m.host_id=$1 AS leader FROM central_identity.ranked_matches m
       JOIN central_identity.ranked_replicas r ON r.match_id=m.id
       JOIN central_identity.ranked_hosts h ON h.host_id=$1 AND h.ranked_epoch=$2 AND h.compatibility=r.compatibility
       WHERE ((m.host_id=$1 AND m.ranked_epoch=$2 AND r.primary_until>clock_timestamp()
                AND h.continuity_since<=r.primary_until-interval '45 seconds')
           OR (r.standby_id=$1 AND r.standby_epoch=$2 AND r.standby_until>clock_timestamp()
                AND h.continuity_since<=r.standby_until-interval '45 seconds'))
       AND h.seen_at>clock_timestamp()-interval '90 seconds'
       AND r.phase IN ('preparing','playing','paused') AND m.state IN ('reserved','playing')
       AND m.lease_until>clock_timestamp() AND (r.paused_until IS NULL OR r.paused_until>clock_timestamp())
       ORDER BY m.id LIMIT 64 FOR SHARE OF m,h SKIP LOCKED",vec![host.id.into(),host.epoch.into()])).await.map_err(unavailable)?;
    let mut items = Vec::with_capacity(rows.len());
    let mut pending = Vec::new();
    for row in rows {
        let role = if column(&row, "leader")? {
            Role::Primary
        } else {
            Role::Standby
        };
        match assignment(db, &row, host, role).await? {
            HostResponse::Assignment { work } => items.push(work),
            HostResponse::AwaitingConsent { fence, role } => {
                pending.push(api::replica::PendingWork { fence, role });
            }
            _ => return Err(Error::Unavailable),
        }
    }
    Ok(HostResponse::Work { items, pending })
}

pub(super) async fn inspect(
    db: &DatabaseTransaction,
    host: &Host,
    id: Uuid,
) -> Result<HostResponse, Error> {
    let member=db.query_one_raw(statement("SELECT r.match_id IS NOT NULL AS protected
       FROM central_identity.ranked_matches m LEFT JOIN central_identity.ranked_replicas r ON r.match_id=m.id
       WHERE m.id=$1 AND ((m.host_id=$2 AND m.ranked_epoch=$3) OR (r.standby_id=$2 AND r.standby_epoch=$3))
       AND m.state IN ('reserved','playing') AND m.lease_until>clock_timestamp()
       AND (r.match_id IS NOT NULL OR EXISTS (SELECT 1 FROM central_identity.ranked_hosts h WHERE h.host_id=m.host_id
           AND h.ranked_epoch=m.ranked_epoch AND m.created_at>=h.continuity_since
           AND h.seen_at>clock_timestamp()-interval '5 minutes')) FOR NO KEY UPDATE OF m",
       vec![id.into(),host.id.into(),host.epoch.into()])).await.map_err(unavailable)?.ok_or(Error::Rejected)?;
    if !column::<bool>(&member, "protected")? {
        return Ok(HostResponse::Single);
    }
    maintain(db, id).await?;
    let row = super::lock_match(db, id).await?;
    let role = super::role(db, &row, host).await?;
    role_live(db, &row, role).await?;
    assignment(db, &row, host, role).await
}

async fn role_live(
    db: &DatabaseTransaction,
    row: &QueryResult,
    role: Role,
) -> Result<leases::Health, Error> {
    let state = super::phase(row)?;
    let live = leases::health(db, column(row, "id")?).await?;
    let current = match role {
        Role::Primary => live.primary,
        Role::Standby => live.standby,
    };
    if current
        && !live.expired
        && matches!(state, Phase::Preparing | Phase::Playing | Phase::Paused)
    {
        Ok(live)
    } else {
        Err(Error::Conflict)
    }
}

pub(super) async fn pulse(
    db: &DatabaseTransaction,
    host: &Host,
    expected: Fence,
) -> Result<HostResponse, Error> {
    let id = Uuid::from_bytes(expected.match_id);
    // A stale pulse never revives an expired role or extends a later term.
    let row = super::lock_match(db, id).await?;
    if fence(&row)? != expected {
        return Err(Error::Conflict);
    }
    let role = super::role(db, &row, host).await?;
    let live = role_live(db, &row, role).await?;
    leases::pulse(db, id, role).await?;
    if role == Role::Primary && !live.standby {
        // A replacement may register after promotion, while both seats wait for a quorum.
        // Commit the new fence, then return only what its current operators may see.
        leases::maintain(db, &row).await?;
        let current = super::lock_match(db, id).await?;
        if matches!(super::phase(&current)?, Phase::Void | Phase::Finished) {
            return Ok(HostResponse::Done);
        }
        return assignment(db, &current, host, role).await;
    }
    assignment(db, &row, host, role).await
}

pub(super) async fn promote(
    db: &DatabaseTransaction,
    host: &Host,
    expected: Fence,
) -> Result<HostResponse, Error> {
    let id = Uuid::from_bytes(expected.match_id);
    let original = super::lock_match(db, id).await?;
    if fence(&original)? != expected || super::role(db, &original, host).await? != Role::Standby {
        return Err(Error::Conflict);
    }
    leases::maintain(db, &original).await?;
    let latest = super::lock_match(db, id).await?;
    if fence(&latest)?.generation <= expected.generation
        || super::role(db, &latest, host).await? != Role::Primary
    {
        return Err(Error::Conflict);
    }
    assignment(db, &latest, host, Role::Primary).await
}
