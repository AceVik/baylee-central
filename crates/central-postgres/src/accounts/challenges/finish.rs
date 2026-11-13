//! Atomic terminal transition, leaderboard eligibility and once-per-task rewards.

use super::{
    AttemptState, Error, api, attempt, attempt_row, lock_account, state, statement, task,
    unavailable, uuid,
};
use sea_orm::{ConnectionTrait, DatabaseTransaction, QueryResult, prelude::Uuid};

pub(super) async fn complete(
    db: &DatabaseTransaction,
    result: &api::Completion,
    origin: &str,
) -> Result<api::HostResponse, Error> {
    result.validate()?;
    let before = attempt_row(db, result.attempt, None).await?;
    let me: Uuid = before.try_get("", "account_id").map_err(unavailable)?;
    lock_account(db, me).await?;
    let before = attempt_row(db, result.attempt, Some(me)).await?;
    let claimed: Option<Uuid> = before.try_get("", "game_id").map_err(unavailable)?;
    if claimed != Some(uuid(result.game)) {
        return Err(Error::Rejected);
    }
    let previous: Option<String> = db.query_one_raw(statement(
        "SELECT completion::text AS completion FROM central_identity.challenge_attempts WHERE id=$1",
        vec![uuid(result.attempt).into()],
    )).await.map_err(unavailable)?.ok_or(Error::Unavailable)?
        .try_get("", "completion").map_err(unavailable)?;
    if let Some(previous) = previous {
        let previous: api::Completion =
            central_api::decode(previous.as_bytes()).map_err(unavailable)?;
        if previous != *result {
            return Err(Error::Conflict);
        }
        return Ok(api::HostResponse::Attempt {
            attempt: attempt(&before, origin)?,
        });
    }
    if state(&before)? != AttemptState::Playing {
        return Err(Error::Conflict);
    }
    validate_time(db, result).await?;
    let task_id: Uuid = before.try_get("", "challenge_id").map_err(unavailable)?;
    let challenge = task(db, *task_id.as_bytes(), true).await?;
    let effective = if challenge.withdrawn {
        AttemptState::Void
    } else {
        result.state
    };
    let row = db
        .query_one_raw(statement(
            "UPDATE central_identity.challenge_attempts SET state=$2,turn=$3,milliseconds=$4,
         completion=$5::jsonb,finished_at=to_timestamp($6::bigint),received_at=pg_catalog.clock_timestamp() WHERE id=$1
         RETURNING *,floor(extract(epoch FROM finished_at))::bigint AS finished_seconds",
            vec![
                uuid(result.attempt).into(),
                effective.name().into(),
                i64::from(result.turn).into(),
                result
                    .milliseconds
                    .map(i64::try_from)
                    .transpose()
                    .map_err(unavailable)?
                    .into(),
                String::from_utf8(central_api::encode(result)?)
                    .map_err(unavailable)?
                    .into(),
                result.finished_at.into(),
            ],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Unavailable)?;
    match effective {
        AttemptState::Won => award(db, me, &challenge, &row, result).await?,
        AttemptState::Lost | AttemptState::Abandoned => {
            db.execute_raw(statement(
                "UPDATE central_identity.challenge_participation SET failures=failures+1
                 WHERE account_id=$1 AND challenge_id=$2",
                vec![me.into(), task_id.into()],
            ))
            .await
            .map_err(unavailable)?;
        }
        AttemptState::Void => {}
        AttemptState::Reserved | AttemptState::Playing => return Err(Error::Rejected),
    }
    Ok(api::HostResponse::Attempt {
        attempt: attempt(&row, origin)?,
    })
}

/// The official host keeps the actual finish time in its outbox. Delivery after midnight
/// must not turn a victory completed in time into an archive score.
async fn validate_time(db: &impl ConnectionTrait, result: &api::Completion) -> Result<(), Error> {
    let row = db
        .query_one_raw(statement(
            "SELECT $2::bigint>=floor(extract(epoch FROM started_at))::bigint
         AND $2::bigint<=floor(extract(epoch FROM pg_catalog.clock_timestamp()))::bigint+5 AS valid
         FROM central_identity.challenge_attempts WHERE id=$1",
            vec![uuid(result.attempt).into(), result.finished_at.into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Rejected)?;
    if row.try_get::<bool>("", "valid").map_err(unavailable)? {
        Ok(())
    } else {
        Err(Error::Rejected)
    }
}

async fn award(
    db: &DatabaseTransaction,
    me: Uuid,
    task: &api::Challenge,
    row: &QueryResult,
    result: &api::Completion,
) -> Result<(), Error> {
    let period = task.description.period;
    let admitted_live: bool = row.try_get("", "live").map_err(unavailable)?;
    let finished: i64 = row.try_get("", "finished_seconds").map_err(unavailable)?;
    let live = admitted_live && period.live_at(finished);
    let assisted: bool = row.try_get("", "assisted").map_err(unavailable)?;
    let failures: i32 = row.try_get("", "failures").map_err(unavailable)?;
    let live_week = if live {
        Some(api::Period::containing(api::Kind::Weekly, period.starts_at)?.starts_at)
    } else {
        None
    };
    db.execute_raw(statement(
        "INSERT INTO central_identity.challenge_completions
         (account_id,challenge_id,kind,live_week,tactician,first_try) VALUES($1,$2,$3,$4,$5,$6)
         ON CONFLICT(account_id,challenge_id) DO UPDATE
         SET tactician=central_identity.challenge_completions.tactician OR EXCLUDED.tactician,
           first_try=central_identity.challenge_completions.first_try OR EXCLUDED.first_try,
           live_week=COALESCE(central_identity.challenge_completions.live_week,EXCLUDED.live_week)",
        vec![
            me.into(),
            uuid(task.id).into(),
            period.kind.name().into(),
            live_week.into(),
            (result.turn <= task.description.target_turn).into(),
            (live && !assisted && failures == 0).into(),
        ],
    ))
    .await
    .map_err(unavailable)?;
    if live && !assisted {
        db.execute_raw(statement(
            "UPDATE central_identity.challenge_participation SET first_win=$3
             WHERE account_id=$1 AND challenge_id=$2 AND first_win IS NULL",
            vec![me.into(), uuid(task.id).into(), uuid(result.attempt).into()],
        ))
        .await
        .map_err(unavailable)?;
    }
    Ok(())
}
