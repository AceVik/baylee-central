//! Clock updates share the match transaction with authenticated inputs,
//! role changes and both executors' certificates. Only database time is used.
use super::{
    DatabaseTransaction, Error, Uuid, clock_availability, column, journal, rpc, statement,
    unavailable,
};
use sea_orm::ConnectionTrait;
mod policy;
use policy::{Due, State};

enum Observation {
    Poll,
    Certificate([Option<u64>; 2]),
    Seat { seat: usize, connected: bool },
    Primary(u8),
    Suspend,
}

pub(super) async fn sync(db: &DatabaseTransaction, id: Uuid) -> Result<(), Error> {
    update(db, id, Observation::Poll).await
}

pub(super) async fn certified(
    db: &DatabaseTransaction,
    id: Uuid,
    evidence: &rpc::Evidence,
) -> Result<(), Error> {
    if evidence.terminal.is_some() {
        return Ok(());
    }
    update(db, id, Observation::Certificate(evidence.decisions)).await
}

pub(super) async fn seat_presence(
    db: &DatabaseTransaction,
    id: Uuid,
    seat: usize,
    connected: bool,
) -> Result<(), Error> {
    if seat > 1 {
        return Err(Error::Rejected);
    }
    update(db, id, Observation::Seat { seat, connected }).await
}

pub(super) async fn host_presence(
    db: &DatabaseTransaction,
    id: Uuid,
    attached: u8,
) -> Result<(), Error> {
    if attached > 3 {
        return Err(Error::Rejected);
    }
    update(db, id, Observation::Primary(attached)).await
}

pub(super) async fn suspend(db: &DatabaseTransaction, id: Uuid) -> Result<(), Error> {
    update(db, id, Observation::Suspend).await
}

pub(super) async fn view(
    db: &DatabaseTransaction,
    id: Uuid,
) -> Result<Option<rpc::clock::ClockView>, Error> {
    Ok(checkpoint(db, id).await?.and_then(|state| state.view()))
}

/// Provisional expiry reserves the next input until both hosts confirm
/// liveness or the recovery transaction restores the player's time.
pub(super) async fn actionable(db: &DatabaseTransaction, id: Uuid) -> Result<bool, Error> {
    Ok(checkpoint(db, id)
        .await?
        .is_some_and(|state| state.actionable()))
}

async fn checkpoint(db: &DatabaseTransaction, id: Uuid) -> Result<Option<State>, Error> {
    // Seat status advances protected clocks under the same match lock before
    // it reads this already-rounded public projection.
    let Some(row) = db
        .query_one_raw(statement(
            "SELECT c.checkpoint FROM central_identity.ranked_clocks c
             JOIN central_identity.ranked_replicas r USING(match_id)
             WHERE c.match_id=$1 AND r.phase NOT IN ('finished','void')",
            vec![id.into()],
        ))
        .await
        .map_err(unavailable)?
    else {
        return Ok(None);
    };
    let bytes: Vec<u8> = column(&row, "checkpoint")?;
    Ok(Some(central_api::decode(&bytes).map_err(unavailable)?))
}

async fn update(db: &DatabaseTransaction, id: Uuid, observation: Observation) -> Result<(), Error> {
    let (generation, healthy, pending) = clock_availability(db, id).await?;
    let Some(row) = db
        .query_one_raw(statement(
            "SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint AS now_ms,
            floor(extract(epoch FROM (LEAST(r.primary_until,r.standby_until)-interval '45 seconds'))*1000)::bigint AS paired_ms,
            c.checkpoint FROM central_identity.ranked_replicas r LEFT JOIN central_identity.ranked_clocks c USING(match_id)
            WHERE r.match_id=$1 AND r.phase NOT IN ('finished','void')",
            vec![id.into()],
        ))
        .await
        .map_err(unavailable)? else {return Ok(())};
    let stored: Option<Vec<u8>> = column(&row, "checkpoint")?;
    if stored.is_none() && matches!(observation, Observation::Poll | Observation::Suspend) {
        return Ok(());
    }
    let mut state: State = stored
        .as_ref()
        .map(|b| central_api::decode(b).map_err(unavailable))
        .transpose()?
        .unwrap_or_default();
    let now = column(&row, "now_ms")?;
    let paired = column(&row, "paired_ms")?;
    let coalesce = matches!(observation, Observation::Poll)
        && healthy
        && state.recent_observation(generation, now);
    if !healthy || matches!(observation, Observation::Suspend) {
        state.refund_unwitnessed(paired);
    }
    state.generation(generation);
    // A certified input was already ordered ahead of this tick. Charge time,
    // then evaluate its resulting questions. An old timeout cannot consume
    // the same player's replacement question.
    let before = if matches!(observation, Observation::Suspend) {
        None
    } else {
        state.tick(now, healthy)
    };
    if coalesce && before.is_none() {
        return Ok(());
    }
    let mut due = match observation {
        Observation::Suspend => {
            state.suspend();
            None
        }
        Observation::Certificate(tokens) => {
            state.certify(tokens);
            state.tick(now, healthy)
        }
        Observation::Seat { seat, connected } => {
            state.seat(now, seat, connected);
            before.or_else(|| state.tick(now, healthy))
        }
        Observation::Primary(attached) => {
            state.host(now, attached);
            before.or_else(|| state.tick(now, healthy))
        }
        Observation::Poll => before,
    };
    // A player input that was durable first might replace the question. Wait
    // for both executors' verdict on that input before reserving any ruling.
    due = if pending || !healthy {
        None
    } else {
        state.witnessed(due, now, paired)
    };
    let checkpoint = central_api::encode(&state)?;
    if stored.as_ref() != Some(&checkpoint) {
        db.execute_raw(statement(
            "INSERT INTO central_identity.ranked_clocks(match_id,checkpoint)
            VALUES($1,$2) ON CONFLICT(match_id) DO UPDATE SET checkpoint=EXCLUDED.checkpoint",
            vec![id.into(), checkpoint.into()],
        ))
        .await
        .map_err(unavailable)?;
    }
    match due {
        None => Ok(()),
        Some(Due::Void) => super::void(db, id).await,
        Some(Due::Concede(event)) => {
            let row = db
                .query_one_raw(statement("SELECT uuidv7() AS command_id", vec![]))
                .await
                .map_err(unavailable)?
                .ok_or(Error::Unavailable)?;
            journal::append_authority(db, id, column(&row, "command_id")?, &event.encode()?).await
        }
    }
}

#[cfg(test)]
mod tests;
