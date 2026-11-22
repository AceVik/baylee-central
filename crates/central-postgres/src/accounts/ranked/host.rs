//! Fresh verified offers, immutable seat claims and retry-safe engine results.

use super::{Error, api, column, scoring, statement, unavailable, uuid};
use sea_orm::{ConnectionTrait, DatabaseTransaction, QueryResult, prelude::Uuid};
use sha2::{Digest, Sha256};

pub(super) struct Host {
    pub(super) id: Uuid,
    pub(super) epoch: i64,
    pub(super) origin: String,
}

pub(super) async fn allocate(db: &DatabaseTransaction) -> Result<Option<Host>, Error> {
    let row=db.query_one_raw(statement(
        "SELECT h.host_id,g.origin FROM central_identity.ranked_hosts h
         JOIN central_identity.gateways g ON g.id=h.host_id
         JOIN central_identity.gateway_approvals p ON p.gateway_id=g.id AND p.ranked_epoch=h.ranked_epoch
         CROSS JOIN LATERAL (SELECT count(*) AS games FROM central_identity.ranked_matches m
           WHERE m.host_id=h.host_id AND m.ranked_epoch=h.ranked_epoch AND m.created_at>=h.continuity_since
            AND m.state IN ('reserved','playing') AND m.lease_until>pg_catalog.clock_timestamp()) load
         WHERE p.enabled AND p.ranked_verified AND h.capacity>0 AND load.games<h.capacity
          AND h.seen_at>pg_catalog.clock_timestamp()-interval '90 seconds'
         ORDER BY load.games::double precision/h.capacity,load.games,h.seen_at DESC,h.host_id
         LIMIT 1 FOR UPDATE OF h SKIP LOCKED",
        vec![]
    )).await.map_err(unavailable)?;
    let Some(row) = row else { return Ok(None) };
    let id: Uuid = column(&row, "host_id")?;
    // Re-read occupancy after locking the offer. A concurrent allocator may have
    // committed a reservation between the first statement's snapshot and its row lock.
    let row = db
        .query_one_raw(statement(
            "SELECT g.origin,p.ranked_epoch FROM central_identity.ranked_hosts h
         JOIN central_identity.gateways g ON g.id=h.host_id
         JOIN central_identity.gateway_approvals p ON p.gateway_id=h.host_id
         WHERE h.host_id=$1 AND p.enabled AND p.ranked_verified AND p.ranked_epoch=h.ranked_epoch
          AND h.seen_at>pg_catalog.clock_timestamp()-interval '90 seconds' AND h.capacity>(
            SELECT count(*) FROM central_identity.ranked_matches m WHERE m.host_id=h.host_id
            AND m.ranked_epoch=h.ranked_epoch AND m.created_at>=h.continuity_since
            AND m.state IN ('reserved','playing') AND m.lease_until>pg_catalog.clock_timestamp())
         FOR SHARE OF p",
            vec![id.into()],
        ))
        .await
        .map_err(unavailable)?;
    row.as_ref()
        .map(|row| {
            Ok(Host {
                id,
                origin: column(row, "origin")?,
                epoch: column(row, "ranked_epoch")?,
            })
        })
        .transpose()
}

/// Check only the polling account's assigned match, including host revocation and liveness.
pub(super) async fn expire(db: &DatabaseTransaction, account: Uuid) -> Result<(), Error> {
    db.execute_raw(statement(
        "WITH dead AS (SELECT m.id FROM central_identity.ranked_matches m
         WHERE m.state IN ('reserved','playing') AND
           m.id=(SELECT t.match_id FROM central_identity.ranked_tickets t
              WHERE t.account_id=$1 AND t.state='assigned')
           AND (m.lease_until<=pg_catalog.clock_timestamp() OR NOT EXISTS (
             SELECT 1 FROM central_identity.gateway_approvals p JOIN central_identity.ranked_hosts h
              ON h.host_id=p.gateway_id WHERE p.gateway_id=m.host_id AND p.enabled AND p.ranked_verified
              AND p.ranked_epoch=m.ranked_epoch AND h.ranked_epoch=m.ranked_epoch
              AND m.created_at>=h.continuity_since AND h.seen_at>pg_catalog.clock_timestamp()-interval '5 minutes'))
         FOR UPDATE OF m SKIP LOCKED)
         UPDATE central_identity.ranked_matches m SET state='void',deck_a=NULL,deck_b=NULL,finished_at=pg_catalog.clock_timestamp()
         FROM dead WHERE m.id=dead.id",vec![account.into()]
    )).await.map_err(unavailable)?;
    Ok(())
}

async fn ready(
    db: &DatabaseTransaction,
    host: &Host,
    capacity: u8,
) -> Result<api::HostResponse, Error> {
    if capacity > 64 {
        return Err(Error::Rejected);
    }
    db.execute_raw(statement(
        "INSERT INTO central_identity.ranked_hosts AS h(host_id,ranked_epoch,capacity) VALUES ($1,$2,$3)
         ON CONFLICT(host_id) DO UPDATE SET capacity=EXCLUDED.capacity,ranked_epoch=EXCLUDED.ranked_epoch,
          continuity_since=CASE WHEN h.ranked_epoch<>EXCLUDED.ranked_epoch OR
            h.seen_at<=pg_catalog.clock_timestamp()-interval '5 minutes' THEN EXCLUDED.seen_at ELSE h.continuity_since END,
          seen_at=EXCLUDED.seen_at",
        vec![host.id.into(),host.epoch.into(),i16::from(capacity).into()]
    )).await.map_err(unavailable)?;
    db.execute_raw(statement(
        "WITH stale AS (SELECT id FROM central_identity.ranked_matches WHERE state IN ('reserved','playing')
          AND lease_until<=now() ORDER BY lease_until LIMIT 64 FOR UPDATE SKIP LOCKED)
         UPDATE central_identity.ranked_matches m SET state='void',deck_a=NULL,deck_b=NULL,finished_at=pg_catalog.clock_timestamp()
          FROM stale WHERE m.id=stale.id",
        vec![],
    )).await.map_err(unavailable)?;
    db.execute_raw(statement(
        "WITH stale AS (SELECT account_id,request_id FROM central_identity.ranked_tickets
           WHERE state='waiting' AND expires_at<=pg_catalog.clock_timestamp() ORDER BY expires_at
           LIMIT 128 FOR UPDATE SKIP LOCKED)
         UPDATE central_identity.ranked_tickets t SET state='closed' FROM stale
          WHERE t.account_id=stale.account_id AND t.request_id=stale.request_id",
        vec![],
    ))
    .await
    .map_err(unavailable)?;
    Ok(api::HostResponse::Ready)
}

pub(super) fn outcome(value: i16) -> Result<api::Outcome, Error> {
    match value {
        0 => Ok(api::Outcome::Draw),
        1 => Ok(api::Outcome::FirstWins),
        2 => Ok(api::Outcome::SecondWins),
        _ => Err(Error::Unavailable),
    }
}

fn valid(id: api::Id) -> Result<Uuid, Error> {
    if super::super::gateways::valid_id(id) {
        Ok(uuid(id))
    } else {
        Err(Error::Rejected)
    }
}

fn assignment(row: &QueryResult) -> Result<api::HostAssignment, Error> {
    Ok(api::HostAssignment {
        match_id: column::<Uuid>(row, "id")?.into_bytes(),
        format: column::<String>(row, "format")?
            .parse()
            .map_err(unavailable)?,
        accounts: [
            column::<Uuid>(row, "account_a")?.into_bytes(),
            column::<Uuid>(row, "account_b")?.into_bytes(),
        ],
        decks: [
            column::<Vec<u8>>(row, "commitment_a")?
                .try_into()
                .map_err(unavailable)?,
            column::<Vec<u8>>(row, "commitment_b")?
                .try_into()
                .map_err(unavailable)?,
        ],
        game: column::<Option<Uuid>>(row, "game_id")?.map(Uuid::into_bytes),
    })
}

async fn claim(
    db: &DatabaseTransaction,
    row: &QueryResult,
    account: api::Id,
    deck: &[u8; 32],
) -> Result<api::HostResponse, Error> {
    let assignment = assignment(row)?;
    let seat = assignment
        .accounts
        .iter()
        .position(|&id| id == account)
        .ok_or(Error::Rejected)?;
    if assignment.decks[seat] != *deck {
        return Err(Error::Rejected);
    }
    if !matches!(
        column::<String>(row, "state")?.as_str(),
        "reserved" | "playing"
    ) {
        return Err(Error::Conflict);
    }
    let sql = if seat == 0 {
        "UPDATE central_identity.ranked_matches SET claimed_a=true WHERE id=$1 AND NOT claimed_a"
    } else {
        "UPDATE central_identity.ranked_matches SET claimed_b=true WHERE id=$1 AND NOT claimed_b"
    };
    db.execute_raw(statement(sql, vec![uuid(assignment.match_id).into()]))
        .await
        .map_err(unavailable)?;
    Ok(api::HostResponse::Assignment { assignment })
}

/// Validates the shape of a host-attested combined deck. The trusted game host verifies format
/// legality against its catalog. Distinct sorted card digests keep persistence deterministic.
fn pool(entries: &[api::PoolEntry]) -> Result<String, Error> {
    if entries.is_empty()
        || entries.len() > 250
        || entries.windows(2).any(|p| p[0].card >= p[1].card)
        || entries.iter().any(|card| card.copies == 0)
        || entries
            .iter()
            .map(|card| u32::from(card.copies))
            .sum::<u32>()
            > 250
    {
        return Err(Error::Rejected);
    }
    String::from_utf8(central_api::encode(&entries)?).map_err(unavailable)
}

async fn start(
    db: &DatabaseTransaction,
    row: &QueryResult,
    game: api::Id,
    pools: &[Vec<api::PoolEntry>; 2],
) -> Result<api::HostResponse, Error> {
    let id: Uuid = column(row, "id")?;
    let game = valid(game)?;
    let decks = [pool(&pools[0])?, pool(&pools[1])?];
    if !column::<bool>(row, "claimed_a")? || !column::<bool>(row, "claimed_b")? {
        return Err(Error::Conflict);
    }
    // The sealed receipt retains this digest, not another copy of both players' lists.
    // Fixed-width binary card/count pairs and a length for each seat make boundaries unique.
    let mut fingerprint = Sha256::new();
    fingerprint.update(b"Baylee ranked host start v1\0");
    for entries in pools {
        fingerprint.update(
            u16::try_from(entries.len())
                .map_err(unavailable)?
                .to_be_bytes(),
        );
        for entry in entries {
            fingerprint.update(entry.card);
            fingerprint.update(entry.copies.to_be_bytes());
        }
    }
    let fingerprint = fingerprint.finalize().to_vec();
    match column::<String>(row, "state")?.as_str() {
        "reserved" => {
            db.execute_raw(statement(
                "UPDATE central_identity.ranked_matches SET game_id=$2,deck_a=$3::jsonb,deck_b=$4::jsonb,start_fingerprint=$5,state='playing',
                 lease_until=pg_catalog.clock_timestamp()+interval '12 hours' WHERE id=$1",
                vec![id.into(),game.into(),decks[0].as_str().into(),decks[1].as_str().into(),fingerprint.into()]
            )).await.map_err(|error| match error.sql_err() {Some(sea_orm::SqlErr::UniqueConstraintViolation(_))=>Error::Conflict,_=>Error::Unavailable})?;
        }
        "playing" | "finished" => {
            if column::<Option<Uuid>>(row, "game_id")? != Some(game)
                || column::<Option<Vec<u8>>>(row, "start_fingerprint")?.as_ref()
                    != Some(&fingerprint)
            {
                return Err(Error::Conflict);
            }
        }
        _ => return Err(Error::Conflict),
    }
    Ok(api::HostResponse::Done)
}

async fn finish(
    db: &DatabaseTransaction,
    row: &QueryResult,
    game: api::Id,
    result: api::Outcome,
) -> Result<api::HostResponse, Error> {
    let id: Uuid = column(row, "id")?;
    if column::<Option<Uuid>>(row, "game_id")? != Some(valid(game)?) {
        return Err(Error::Conflict);
    }
    let score = match result {
        api::Outcome::Draw => 0_i16,
        api::Outcome::FirstWins => 1,
        api::Outcome::SecondWins => 2,
    };
    match column::<String>(row, "state")?.as_str() {
        "finished" if column::<Option<i16>>(row, "outcome")? == Some(score) => {}
        "playing" => {
            // The scorer saves the exact deck versions before the two temporary pools are
            // removed. Ratings, receipt and terminal state commit together under the row lock.
            scoring::settle(db, id, score).await?;
            db.execute_raw(statement(
                "UPDATE central_identity.ranked_matches SET state='finished',outcome=$2,
                 deck_a=NULL,deck_b=NULL,finished_at=pg_catalog.clock_timestamp() WHERE id=$1",
                vec![id.into(), score.into()],
            ))
            .await
            .map_err(unavailable)?;
        }
        _ => return Err(Error::Conflict),
    }
    Ok(api::HostResponse::Done)
}

pub(super) async fn handle(
    db: &DatabaseTransaction,
    host: &Host,
    request: &api::HostRequest,
) -> Result<api::HostResponse, Error> {
    let id = match request {
        api::HostRequest::Ready { capacity } => return ready(db, host, *capacity).await,
        api::HostRequest::Claim { match_id, .. }
        | api::HostRequest::Start { match_id, .. }
        | api::HostRequest::Void { match_id }
        | api::HostRequest::Finish { match_id, .. } => valid(*match_id)?,
    };
    let row = db
        .query_one_raw(statement(
            "SELECT m.*, (m.lease_until>pg_catalog.clock_timestamp() AND EXISTS (
         SELECT 1 FROM central_identity.ranked_hosts h WHERE h.host_id=m.host_id AND
         h.ranked_epoch=m.ranked_epoch AND m.created_at>=h.continuity_since
         AND h.seen_at>pg_catalog.clock_timestamp()-interval '5 minutes')) AS live
         FROM central_identity.ranked_matches m WHERE id=$1 AND host_id=$2 AND ranked_epoch=$3
         FOR UPDATE OF m",
            vec![id.into(), host.id.into(), host.epoch.into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Rejected)?;
    let state: String = column(&row, "state")?;
    if matches!(state.as_str(), "reserved" | "playing")
        && !column::<bool>(&row, "live")?
        && !matches!(request, api::HostRequest::Void { .. })
    {
        return Err(Error::Conflict);
    }
    match request {
        api::HostRequest::Claim { account, deck, .. } => claim(db, &row, *account, deck).await,
        api::HostRequest::Start { game, pools, .. } => start(db, &row, *game, pools).await,
        api::HostRequest::Finish { game, outcome, .. } => finish(db, &row, *game, *outcome).await,
        api::HostRequest::Void { .. } => {
            if state == "finished" {
                return Err(Error::Conflict);
            }
            if state != "void" {
                db.execute_raw(statement("UPDATE central_identity.ranked_matches SET state='void',deck_a=NULL,deck_b=NULL,finished_at=pg_catalog.clock_timestamp() WHERE id=$1",vec![id.into()])).await.map_err(unavailable)?;
            }
            Ok(api::HostResponse::Done)
        }
        api::HostRequest::Ready { .. } => Err(Error::Rejected),
    }
}
