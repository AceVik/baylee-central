//! Central-owned expeditions, reusing the account lock and official result boundary.

use super::{attempt, attempt_row, challenge, now, statement, unavailable, uuid};
use central_api::{Error, tower as api};
use sea_orm::{ConnectionTrait, DatabaseTransaction, QueryResult, prelude::Uuid};

mod finish;
mod host;
mod owned;
mod read;

pub(super) use finish::complete;
pub(super) use host::dispatch as publish;

fn json(bytes: Vec<u8>) -> Result<String, Error> {
    String::from_utf8(bytes).map_err(unavailable)
}

fn id_valid(id: [u8; 16]) -> Result<(), Error> {
    if id[6] >> 4 == 7 && id[8] >> 6 == 2 {
        Ok(())
    } else {
        Err(Error::Rejected)
    }
}

fn manifest(row: &QueryResult) -> Result<api::Manifest, Error> {
    let value: String = row.try_get("", "manifest").map_err(unavailable)?;
    let value: api::Manifest = central_api::decode(value.as_bytes()).map_err(unavailable)?;
    value.validate().map_err(unavailable)?;
    Ok(value)
}

fn season(row: &QueryResult) -> Result<api::Season, Error> {
    let id: Uuid = row.try_get("", "id").map_err(unavailable)?;
    Ok(api::Season {
        id: *id.as_bytes(),
        theme: Box::new(manifest(row)?.theme),
        withdrawn: row.try_get("", "withdrawn").map_err(unavailable)?,
    })
}

async fn season_row(
    db: &impl ConnectionTrait,
    id: [u8; 16],
    lock: bool,
) -> Result<QueryResult, Error> {
    db.query_one_raw(statement(&format!(
        "SELECT id,manifest::text,published,withdrawn FROM central_identity.tower_seasons
         WHERE id=$1 AND published AND starts_at<=floor(extract(epoch FROM pg_catalog.clock_timestamp())){}",
        if lock { " FOR SHARE" } else { "" }), vec![uuid(id).into()]))
        .await.map_err(unavailable)?.ok_or(Error::Rejected)
}

async fn run_row(db: &impl ConnectionTrait, me: Uuid, id: [u8; 16]) -> Result<QueryResult, Error> {
    db.query_one_raw(statement(
        "SELECT *,progress::text AS state FROM central_identity.tower_runs WHERE id=$1 AND account_id=$2",
        vec![uuid(id).into(), me.into()])).await.map_err(unavailable)?.ok_or(Error::Rejected)
}

fn progress(row: &QueryResult) -> Result<api::Progress, Error> {
    let value: String = row.try_get("", "state").map_err(unavailable)?;
    let value: api::Progress = central_api::decode(value.as_bytes()).map_err(unavailable)?;
    value.validate().map_err(unavailable)?;
    Ok(value)
}

async fn node(
    db: &impl ConnectionTrait,
    season: Uuid,
    index: u8,
) -> Result<central_api::challenges::Challenge, Error> {
    let row = db.query_one_raw(statement(
        "SELECT c.id,c.description::text,c.digest,c.withdrawn FROM central_identity.tower_nodes n
         JOIN central_identity.challenges c ON c.id=n.challenge_id WHERE n.season_id=$1 AND n.node=$2",
        vec![season.into(), i16::from(index).into()])).await.map_err(unavailable)?.ok_or(Error::Unavailable)?;
    challenge(&row)
}

async fn load(
    db: &impl ConnectionTrait,
    me: Uuid,
    id: [u8; 16],
    origin: &str,
) -> Result<api::Run, Error> {
    let row = run_row(db, me, id).await?;
    let season_id: Uuid = row.try_get("", "season_id").map_err(unavailable)?;
    let season = season(&season_row(db, *season_id.as_bytes(), false).await?)?;
    let revision: i32 = row.try_get("", "revision").map_err(unavailable)?;
    let progress = progress(&row)?;
    let active: Option<Uuid> = row.try_get("", "active_attempt").map_err(unavailable)?;
    let attempt = match active {
        Some(id) => Some(attempt(
            &attempt_row(db, *id.as_bytes(), Some(me)).await?,
            origin,
        )?),
        None => None,
    };
    let participation = db.query_one_raw(statement(
        "SELECT listed FROM central_identity.tower_participation WHERE account_id=$1 AND season_id=$2",
        vec![me.into(), season_id.into()])).await.map_err(unavailable)?.ok_or(Error::Unavailable)?;
    Ok(api::Run {
        id,
        season,
        revision: u32::try_from(revision).map_err(unavailable)?,
        encounter: node(db, season_id, progress.node()?).await?,
        progress,
        attempt,
        listed: participation.try_get("", "listed").map_err(unavailable)?,
    })
}

async fn save(
    db: &impl ConnectionTrait,
    id: [u8; 16],
    progress: &api::Progress,
    active: Option<Uuid>,
) -> Result<(), Error> {
    progress.validate()?;
    db.execute_raw(statement(
        "UPDATE central_identity.tower_runs SET progress=$2::jsonb,active=$3,active_attempt=$4,
         revision=revision+1 WHERE id=$1",
        vec![
            uuid(id).into(),
            json(central_api::encode(progress)?)?.into(),
            progress.phase.active().into(),
            active.into(),
        ],
    ))
    .await
    .map_err(unavailable)?;
    Ok(())
}

pub(super) async fn dispatch(
    db: &DatabaseTransaction,
    me: Option<Uuid>,
    request: api::Request,
    origin: &str,
) -> Result<api::Response, Error> {
    match request {
        api::Request::Read {
            season,
            board,
            offset,
        } => read::landing(db, me, season, board, offset, origin).await,
        other => owned::dispatch(db, me.ok_or(Error::Rejected)?, other, origin).await,
    }
}
