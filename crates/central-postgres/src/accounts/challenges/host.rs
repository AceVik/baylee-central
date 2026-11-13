//! Official-bridge publications and claims. Ordinary gateway credentials never reach here.

use super::{
    AttemptState, Error, api, attempt, attempt_row, challenge, finish, lock_account, now, state,
    statement, task, unavailable, uuid,
};
use sea_orm::{ConnectionTrait, DatabaseTransaction, prelude::Uuid};

pub(super) async fn dispatch(
    db: &DatabaseTransaction,
    request: api::HostRequest,
    origin: &str,
) -> Result<api::HostResponse, Error> {
    match request {
        api::HostRequest::Inventory { from, until } => inventory(db, from, until).await,
        api::HostRequest::Publish { publication } => publish(db, &publication).await,
        api::HostRequest::Claim {
            attempt,
            account,
            game,
            executor,
        } => claim(db, attempt, account, game, &executor, origin).await,
        api::HostRequest::Status { attempt: id } => Ok(api::HostResponse::Attempt {
            attempt: attempt(&attempt_row(db, id, None).await?, origin)?,
        }),
        api::HostRequest::Finish { result } => finish::complete(db, &result, origin).await,
        api::HostRequest::Withdraw { challenge: id } => {
            db.execute_raw(statement(
                "UPDATE central_identity.challenges SET withdrawn=true WHERE id=$1",
                vec![uuid(id).into()],
            ))
            .await
            .map_err(unavailable)?;
            Ok(api::HostResponse::Challenge {
                challenge: task(db, id, true).await?,
            })
        }
    }
}

async fn inventory(
    db: &impl ConnectionTrait,
    from: i64,
    until: i64,
) -> Result<api::HostResponse, Error> {
    if from < 0
        || until
            .checked_sub(from)
            .is_none_or(|span| !(1..=32 * 86_400).contains(&span))
    {
        return Err(Error::Rejected);
    }
    let rows = db
        .query_all_raw(statement(
            "SELECT id,description::text,digest,withdrawn FROM central_identity.challenges
         WHERE starts_at>=$1 AND starts_at<$2 ORDER BY starts_at,kind LIMIT 64",
            vec![from.into(), until.into()],
        ))
        .await
        .map_err(unavailable)?;
    Ok(api::HostResponse::Inventory {
        challenges: rows.iter().map(challenge).collect::<Result<_, _>>()?,
    })
}

async fn publish(
    db: &impl ConnectionTrait,
    value: &api::Publication,
) -> Result<api::HostResponse, Error> {
    value.validate()?;
    let period = value.description.period;
    let now = now(db).await?;
    if period.starts_at < now - 7 * 86_400 || period.starts_at > now + 32 * 86_400 {
        return Err(Error::Rejected);
    }
    let description =
        String::from_utf8(central_api::encode(&value.description)?).map_err(unavailable)?;
    if description.len() > 24_000 {
        return Err(Error::Rejected);
    }
    db.execute_raw(statement(
        "INSERT INTO central_identity.challenges(kind,starts_at,ends_at,description,digest,publication)
         VALUES($1,$2,$3,$4::jsonb,$5,$6) ON CONFLICT(kind,starts_at) DO NOTHING",
        vec![period.kind.name().into(), period.starts_at.into(), period.ends_at.into(), description.into(),
            value.digest.to_vec().into(), central_api::encode(value)?.into()],
    )).await.map_err(unavailable)?;
    let row = db.query_one_raw(statement(
        "SELECT id,description::text,digest,withdrawn FROM central_identity.challenges WHERE kind=$1 AND starts_at=$2",
        vec![period.kind.name().into(), period.starts_at.into()],
    )).await.map_err(unavailable)?.ok_or(Error::Unavailable)?;
    let challenge = challenge(&row)?;
    if challenge.digest != value.digest {
        return Err(Error::Conflict);
    }
    Ok(api::HostResponse::Challenge { challenge })
}

pub(super) async fn publication(
    db: &impl ConnectionTrait,
    id: [u8; 16],
) -> Result<api::Publication, Error> {
    let row = db
        .query_one_raw(statement(
            "SELECT publication FROM central_identity.challenges WHERE id=$1",
            vec![uuid(id).into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Rejected)?;
    let bytes: Vec<u8> = row.try_get("", "publication").map_err(unavailable)?;
    let value: api::Publication = central_api::decode(&bytes).map_err(unavailable)?;
    value.validate().map_err(unavailable)?;
    Ok(value)
}

async fn claim(
    db: &DatabaseTransaction,
    id: [u8; 16],
    account: [u8; 16],
    game: [u8; 16],
    executor: &str,
    origin: &str,
) -> Result<api::HostResponse, Error> {
    if game[6] >> 4 != 7 || game[8] >> 6 != 2 {
        return Err(Error::Rejected);
    }
    let me = uuid(account);
    lock_account(db, me).await?;
    let row = attempt_row(db, id, Some(me)).await?;
    let task_id: Uuid = row.try_get("", "challenge_id").map_err(unavailable)?;
    let task = task(db, *task_id.as_bytes(), true).await?;
    if task.withdrawn || executor != task.description.executor {
        return Err(Error::Conflict);
    }
    let previous = state(&row)?;
    if !previous.active() {
        return Err(Error::Conflict);
    }
    if previous == AttemptState::Reserved {
        let counts = db
            .query_one_raw(statement(
                "UPDATE central_identity.challenge_participation SET starts=starts+1
             WHERE account_id=$1 AND challenge_id=$2 RETURNING starts,failures,hint_level",
                vec![me.into(), task_id.into()],
            ))
            .await
            .map_err(unavailable)?
            .ok_or(Error::Unavailable)?;
        let starts: i32 = counts.try_get("", "starts").map_err(unavailable)?;
        let failures: i32 = counts.try_get("", "failures").map_err(unavailable)?;
        let hints: i16 = counts.try_get("", "hint_level").map_err(unavailable)?;
        let live: bool = row.try_get("", "live").map_err(unavailable)?;
        db.execute_raw(statement(
            "UPDATE central_identity.challenge_attempts SET state='playing',game_id=$2,number=$3,failures=$4,
             assisted=$5,live=$6,started_at=pg_catalog.clock_timestamp() WHERE id=$1",
            vec![uuid(id).into(), uuid(game).into(), starts.into(), failures.into(), (hints>0).into(),
                (live && task.description.period.live_at(now(db).await?)).into()],
        )).await.map_err(unavailable)?;
    }
    let attempt = attempt(&attempt_row(db, id, Some(me)).await?, origin)?;
    // A repeated claim always returns its original game, never the newly suggested one.
    Ok(api::HostResponse::Claimed {
        attempt,
        publication: Box::new(publication(db, task.id).await?),
    })
}
