//! Each mutation runs under the already revalidated account lock, in one transaction.

use super::{api, id_valid, json, load, now, save, season_row, statement, unavailable, uuid};
use central_api::Error;
use sea_orm::{ConnectionTrait, DatabaseTransaction, prelude::Uuid};

pub(super) async fn dispatch(
    db: &DatabaseTransaction,
    me: Uuid,
    request: api::Request,
    origin: &str,
) -> Result<api::Response, Error> {
    let run = match &request {
        api::Request::Read { .. } => return Err(Error::Rejected),
        api::Request::Run { run } => *run,
        api::Request::Locate { game } => locate(db, me, *game).await?,
        api::Request::Begin {
            season,
            request_id,
            listed,
        } => {
            if let Some(run) = retry(db, me, *request_id, &request).await? {
                run
            } else {
                budget(db, me).await?;
                let run = begin(db, me, *season, *request_id, *listed).await?;
                remember(db, me, *request_id, run, &request).await?;
                *run.as_bytes()
            }
        }
        api::Request::Start {
            run,
            revision,
            request_id,
        } => {
            if let Some(run) = retry(db, me, *request_id, &request).await? {
                run
            } else {
                budget(db, me).await?;
                let current = checked(db, me, *run, *revision, origin).await?;
                reserve(db, me, current, *request_id).await?;
                remember(db, me, *request_id, uuid(*run), &request).await?;
                *run
            }
        }
        api::Request::Choose {
            run,
            revision,
            track,
            request_id,
        } => {
            if let Some(run) = retry(db, me, *request_id, &request).await? {
                run
            } else {
                budget(db, me).await?;
                let current = checked(db, me, *run, *revision, origin).await?;
                let mut state = current.progress;
                state.choose(*track)?;
                save(db, *run, &state, None).await?;
                remember(db, me, *request_id, uuid(*run), &request).await?;
                *run
            }
        }
        api::Request::Abandon { run, revision } => {
            let current = load(db, me, *run, origin).await?;
            if current.progress.phase != api::Phase::Abandoned {
                if current.revision != *revision || !current.progress.phase.active() {
                    return Err(Error::Conflict);
                }
                if let Some(attempt) = current.attempt {
                    db.execute_raw(statement(
                        "UPDATE central_identity.challenge_attempts SET state='abandoned',
                         finished_at=pg_catalog.clock_timestamp() WHERE id=$1 AND account_id=$2
                         AND state IN ('reserved','playing')",
                        vec![uuid(attempt.id).into(), me.into()],
                    ))
                    .await
                    .map_err(unavailable)?;
                }
                let mut state = current.progress;
                state.phase = api::Phase::Abandoned;
                save(db, *run, &state, None).await?;
            }
            *run
        }
        api::Request::Visibility { season, listed } => {
            season_row(db, *season, false).await?;
            consent(db, me, *season, *listed).await?;
            return Ok(api::Response::Updated);
        }
        api::Request::Export { after } => return export(db, me, *after, origin).await,
    };
    Ok(api::Response::Run {
        run: Box::new(load(db, me, run, origin).await?),
    })
}

async fn locate(db: &DatabaseTransaction, me: Uuid, game: [u8; 16]) -> Result<[u8; 16], Error> {
    id_valid(game)?;
    let row = db.query_one_raw(statement(
        "SELECT t.run_id FROM central_identity.challenge_attempts a
         JOIN central_identity.tower_actions t ON t.account_id=a.account_id AND t.request_id=a.request_id
         WHERE a.account_id=$1 AND a.game_id=$2 AND a.kind='tower'",
        vec![me.into(), uuid(game).into()])).await.map_err(unavailable)?.ok_or(Error::Rejected)?;
    let run: Uuid = row.try_get("", "run_id").map_err(unavailable)?;
    Ok(*run.as_bytes())
}

async fn begin(
    db: &DatabaseTransaction,
    me: Uuid,
    season_id: [u8; 16],
    request_id: [u8; 16],
    listed: bool,
) -> Result<Uuid, Error> {
    let season = super::season(&season_row(db, season_id, true).await?)?;
    if season.withdrawn {
        return Err(Error::Conflict);
    }
    if db
        .query_one_raw(statement(
            "SELECT id FROM central_identity.tower_runs WHERE account_id=$1 AND active",
            vec![me.into()],
        ))
        .await
        .map_err(unavailable)?
        .is_some()
    {
        return Err(Error::Conflict);
    }
    consent(db, me, season.id, listed).await?;
    let state = api::Progress::new(season.theme.period.live_at(now(db).await?));
    let row = db
        .query_one_raw(statement(
            "INSERT INTO central_identity.tower_runs(account_id,season_id,request_id,progress)
         VALUES($1,$2,$3,$4::jsonb) RETURNING id",
            vec![
                me.into(),
                uuid(season.id).into(),
                uuid(request_id).into(),
                json(central_api::encode(&state)?)?.into(),
            ],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Unavailable)?;
    row.try_get("", "id").map_err(unavailable)
}

async fn reserve(
    db: &DatabaseTransaction,
    me: Uuid,
    current: api::Run,
    request_id: [u8; 16],
) -> Result<(), Error> {
    let mut state = current.progress;
    state.start()?;
    let period = current.season.theme.period;
    if db
        .query_one_raw(statement(
            "SELECT id FROM central_identity.challenge_attempts WHERE account_id=$1
         AND (request_id=$2 OR (kind='tower' AND state IN ('reserved','playing')))",
            vec![me.into(), uuid(request_id).into()],
        ))
        .await
        .map_err(unavailable)?
        .is_some()
    {
        return Err(Error::Conflict);
    }
    db.execute_raw(statement(
        "INSERT INTO central_identity.challenge_participation(account_id,challenge_id)
         VALUES($1,$2) ON CONFLICT(account_id,challenge_id) DO NOTHING",
        vec![me.into(), uuid(current.encounter.id).into()],
    ))
    .await
    .map_err(unavailable)?;
    let row = db.query_one_raw(statement(
        "INSERT INTO central_identity.challenge_attempts(account_id,challenge_id,kind,request_id,assisted,live)
         VALUES($1,$2,'tower',$3,false,$4) RETURNING id",
        vec![me.into(), uuid(current.encounter.id).into(), uuid(request_id).into(),
            (state.competitive && period.live_at(now(db).await?)).into()]))
        .await.map_err(unavailable)?.ok_or(Error::Unavailable)?;
    let attempt: Uuid = row.try_get("", "id").map_err(unavailable)?;
    save(db, current.id, &state, Some(attempt)).await
}

async fn checked(
    db: &DatabaseTransaction,
    me: Uuid,
    run: [u8; 16],
    revision: u32,
    origin: &str,
) -> Result<api::Run, Error> {
    let current = load(db, me, run, origin).await?;
    let season = super::season(&season_row(db, current.season.id, true).await?)?;
    if current.revision != revision || season.withdrawn || current.encounter.withdrawn {
        return Err(Error::Conflict);
    }
    Ok(current)
}

async fn consent(
    db: &impl ConnectionTrait,
    me: Uuid,
    season: [u8; 16],
    listed: bool,
) -> Result<(), Error> {
    db.execute_raw(statement(
        "INSERT INTO central_identity.tower_participation(account_id,season_id,listed) VALUES($1,$2,$3)
         ON CONFLICT(account_id,season_id) DO UPDATE SET listed=EXCLUDED.listed",
        vec![me.into(), uuid(season).into(), listed.into()])).await.map_err(unavailable)?;
    Ok(())
}

async fn retry(
    db: &impl ConnectionTrait,
    me: Uuid,
    key: [u8; 16],
    request: &api::Request,
) -> Result<Option<[u8; 16]>, Error> {
    id_valid(key)?;
    let Some(row) = db.query_one_raw(statement(
        "SELECT run_id,request::text FROM central_identity.tower_actions WHERE account_id=$1 AND request_id=$2",
        vec![me.into(), uuid(key).into()])).await.map_err(unavailable)? else { return Ok(None); };
    let stored: String = row.try_get("", "request").map_err(unavailable)?;
    let stored: api::Request = central_api::decode(stored.as_bytes()).map_err(unavailable)?;
    if stored != *request {
        return Err(Error::Conflict);
    }
    let run: Uuid = row.try_get("", "run_id").map_err(unavailable)?;
    Ok(Some(*run.as_bytes()))
}

async fn remember(
    db: &impl ConnectionTrait,
    me: Uuid,
    key: [u8; 16],
    run: Uuid,
    request: &api::Request,
) -> Result<(), Error> {
    db.execute_raw(statement(
        "INSERT INTO central_identity.tower_actions(account_id,request_id,run_id,request) VALUES($1,$2,$3,$4::jsonb)",
        vec![me.into(), uuid(key).into(), run.into(), json(central_api::encode(request)?)?.into()])).await.map_err(unavailable)?;
    Ok(())
}

async fn budget(db: &impl ConnectionTrait, me: Uuid) -> Result<(), Error> {
    let row = db
        .query_one_raw(statement(
            "SELECT count(*) AS count FROM central_identity.tower_actions WHERE account_id=$1
         AND created_at>pg_catalog.clock_timestamp()-interval '1 hour'",
            vec![me.into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Unavailable)?;
    if row.try_get::<i64>("", "count").map_err(unavailable)? >= 120 {
        return Err(Error::Busy);
    }
    Ok(())
}

async fn export(
    db: &impl ConnectionTrait,
    me: Uuid,
    after: Option<[u8; 16]>,
    origin: &str,
) -> Result<api::Response, Error> {
    if let Some(id) = after {
        id_valid(id)?;
    }
    let mut rows = db.query_all_raw(statement(
        "SELECT id FROM central_identity.tower_runs WHERE account_id=$1 AND ($2::uuid IS NULL OR id>$2)
         ORDER BY id LIMIT 11", vec![me.into(), after.map(uuid).into()])).await.map_err(unavailable)?;
    let more = rows.len() > 10;
    if more {
        rows.pop();
    }
    let mut runs = Vec::with_capacity(rows.len());
    for row in rows {
        let id: Uuid = row.try_get("", "id").map_err(unavailable)?;
        runs.push(load(db, me, *id.as_bytes(), origin).await?);
    }
    let after = if more {
        runs.last().map(|run| run.id)
    } else {
        None
    };
    Ok(api::Response::Export { runs, after })
}
