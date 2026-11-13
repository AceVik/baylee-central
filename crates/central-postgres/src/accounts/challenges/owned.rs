//! Operations serialized by the authenticated account lock acquired by the parent.

use super::{
    AttemptState, Error, Request, Response, attempt, attempt_row, now, personal, read, state,
    statement, task, unavailable, uuid,
};
use sea_orm::{ConnectionTrait, DatabaseTransaction, prelude::Uuid};

pub(super) async fn dispatch(
    db: &DatabaseTransaction,
    me: Uuid,
    request: Request,
    host: &str,
) -> Result<Response, Error> {
    match request {
        Request::Progress => read::progress(db, me, host).await,
        Request::Personal { challenge } => {
            task(db, challenge, false).await?;
            Ok(Response::Personal {
                progress: personal::progress(db, me, challenge).await?,
            })
        }
        Request::Export { table, after } => personal::export(db, me, table, after).await,
        Request::Begin {
            challenge,
            request_id,
            listed,
        } => begin(db, me, challenge, request_id, listed, host).await,
        Request::Attempt { attempt: id } => Ok(Response::Attempt {
            attempt: attempt(&attempt_row(db, id, Some(me)).await?, host)?,
        }),
        Request::Abandon { attempt: id } => abandon(db, me, id, host).await,
        Request::Hint { challenge, level } => hint(db, me, challenge, level).await,
        Request::Visibility { challenge, listed } => {
            task(db, challenge, false).await?;
            participation(db, me, challenge, listed).await?;
            Ok(Response::Updated)
        }
        Request::Catalogue { .. } | Request::Detail { .. } => Err(Error::Rejected),
    }
}

async fn participation(
    db: &impl ConnectionTrait,
    me: Uuid,
    challenge: [u8; 16],
    listed: bool,
) -> Result<(), Error> {
    db.execute_raw(statement(
        "INSERT INTO central_identity.challenge_participation (account_id,challenge_id,listed)
         VALUES($1,$2,$3) ON CONFLICT(account_id,challenge_id) DO UPDATE SET listed=EXCLUDED.listed",
        vec![me.into(), uuid(challenge).into(), listed.into()],
    )).await.map_err(unavailable)?;
    Ok(())
}

async fn begin(
    db: &DatabaseTransaction,
    me: Uuid,
    challenge: [u8; 16],
    request: [u8; 16],
    listed: bool,
    host: &str,
) -> Result<Response, Error> {
    if request[6] >> 4 != 7 || request[8] >> 6 != 2 {
        return Err(Error::Rejected);
    }
    if let Some(row) = db.query_one_raw(statement(
        "SELECT * FROM central_identity.challenge_attempts WHERE account_id=$1 AND request_id=$2",
        vec![me.into(), uuid(request).into()],
    )).await.map_err(unavailable)? {
        let previous = attempt(&row, host)?;
        if previous.challenge != challenge { return Err(Error::Conflict); }
        return Ok(Response::Attempt { attempt: previous });
    }
    let challenge = task(db, challenge, false).await?;
    if challenge.withdrawn {
        return Err(Error::Conflict);
    }
    let period = challenge.description.period;
    if db
        .query_one_raw(statement(
            "SELECT id FROM central_identity.challenge_attempts WHERE account_id=$1 AND kind=$2
         AND state IN ('reserved','playing') LIMIT 1",
            vec![me.into(), period.kind.name().into()],
        ))
        .await
        .map_err(unavailable)?
        .is_some()
    {
        // A different key did not successfully reserve anything: the client reads Progress
        // and resumes its existing attempt. Never acknowledge an unrecorded idempotency key.
        return Err(Error::Conflict);
    }
    let count = db
        .query_one_raw(statement(
            "SELECT count(*) AS count FROM central_identity.challenge_attempts WHERE account_id=$1
         AND created_at>pg_catalog.clock_timestamp()-interval '1 hour'",
            vec![me.into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Unavailable)?;
    if count.try_get::<i64>("", "count").map_err(unavailable)? >= 120 {
        return Err(Error::Busy);
    }
    participation(db, me, challenge.id, listed).await?;
    let live = period.live_at(now(db).await?);
    let row = db
        .query_one_raw(statement(
            "INSERT INTO central_identity.challenge_attempts
         (account_id,challenge_id,kind,request_id,assisted,live)
         SELECT $1,$2,$3,$4,hint_level>0,$5 FROM central_identity.challenge_participation
         WHERE account_id=$1 AND challenge_id=$2 RETURNING *",
            vec![
                me.into(),
                uuid(challenge.id).into(),
                period.kind.name().into(),
                uuid(request).into(),
                live.into(),
            ],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Unavailable)?;
    Ok(Response::Attempt {
        attempt: attempt(&row, host)?,
    })
}

async fn abandon(
    db: &DatabaseTransaction,
    me: Uuid,
    id: [u8; 16],
    host: &str,
) -> Result<Response, Error> {
    let row = attempt_row(db, id, Some(me)).await?;
    let previous = state(&row)?;
    if previous == AttemptState::Abandoned {
        return Ok(Response::Attempt {
            attempt: attempt(&row, host)?,
        });
    }
    if !previous.active() {
        return Err(Error::Conflict);
    }
    if previous == AttemptState::Playing {
        db.execute_raw(statement(
            "UPDATE central_identity.challenge_participation SET failures=failures+1
             WHERE account_id=$1 AND challenge_id=(SELECT challenge_id FROM central_identity.challenge_attempts WHERE id=$2)",
            vec![me.into(), uuid(id).into()],
        )).await.map_err(unavailable)?;
    }
    let ended = db.query_one_raw(statement(
        "UPDATE central_identity.challenge_attempts SET state='abandoned',finished_at=pg_catalog.clock_timestamp()
         WHERE id=$1 RETURNING *", vec![uuid(id).into()],
    )).await.map_err(unavailable)?.ok_or(Error::Unavailable)?;
    Ok(Response::Attempt {
        attempt: attempt(&ended, host)?,
    })
}

async fn hint(
    db: &DatabaseTransaction,
    me: Uuid,
    id: [u8; 16],
    level: u8,
) -> Result<Response, Error> {
    if !(1..=2).contains(&level) {
        return Err(Error::Rejected);
    }
    if task(db, id, false).await?.withdrawn {
        return Err(Error::Conflict);
    }
    db.execute_raw(statement(
        "INSERT INTO central_identity.challenge_participation (account_id,challenge_id,hint_level)
         VALUES($1,$2,$3) ON CONFLICT(account_id,challenge_id) DO UPDATE
         SET hint_level=GREATEST(central_identity.challenge_participation.hint_level,EXCLUDED.hint_level)",
        vec![me.into(), uuid(id).into(), i16::from(level).into()],
    )).await.map_err(unavailable)?;
    db.execute_raw(statement(
        "UPDATE central_identity.challenge_attempts SET assisted=true WHERE account_id=$1 AND challenge_id=$2
         AND state IN ('reserved','playing')", vec![me.into(), uuid(id).into()],
    )).await.map_err(unavailable)?;
    let publication = super::host::publication(db, id).await?;
    // The caller commits the transaction before the response is returned to transport.
    Ok(Response::Hint {
        text: publication.hints[usize::from(level - 1)].clone(),
    })
}
