//! Public rankings and owned progress never read execution packages.

use super::{api, load, now, season, season_row, statement, unavailable, uuid};
use central_api::Error;
use sea_orm::{ConnectionTrait, QueryResult, prelude::Uuid};

pub(super) async fn landing(
    db: &impl ConnectionTrait,
    me: Option<Uuid>,
    id: Option<[u8; 16]>,
    board: api::Board,
    offset: u32,
    origin: &str,
) -> Result<api::Response, Error> {
    if offset > 3_000 {
        return Err(Error::Rejected);
    }
    let selected = match id {
        Some(id) => Some(season(&season_row(db, id, false).await?)?),
        None => db
            .query_one_raw(statement(
                "SELECT id,manifest::text,published,withdrawn FROM central_identity.tower_seasons
             WHERE published AND starts_at<=floor(extract(epoch FROM pg_catalog.clock_timestamp()))
             ORDER BY starts_at DESC LIMIT 1",
                vec![],
            ))
            .await
            .map_err(unavailable)?
            .as_ref()
            .map(season)
            .transpose()?,
    };
    let mut run = None;
    let mut active = None;
    let mut badges = Vec::new();
    if let Some(me) = me {
        if let Some(season) = &selected
            && let Some(row) = db.query_one_raw(statement(
                "SELECT id FROM central_identity.tower_runs WHERE account_id=$1 AND season_id=$2 ORDER BY id DESC LIMIT 1",
                vec![me.into(), uuid(season.id).into()])).await.map_err(unavailable)? {
                let id: Uuid = row.try_get("", "id").map_err(unavailable)?;
                run = Some(Box::new(load(db, me, *id.as_bytes(), origin).await?));
            }
        if let Some(row) = db
            .query_one_raw(statement(
                "SELECT id FROM central_identity.tower_runs WHERE account_id=$1 AND active",
                vec![me.into()],
            ))
            .await
            .map_err(unavailable)?
        {
            let id: Uuid = row.try_get("", "id").map_err(unavailable)?;
            active = Some(Box::new(load(db, me, *id.as_bytes(), origin).await?));
        }
        badges = rewards(db, me).await?;
    }
    let standings = match &selected {
        Some(season) if !season.withdrawn => {
            ranking(db, uuid(season.id), me, board, offset).await?
        }
        _ => Vec::new(),
    };
    Ok(api::Response::Read {
        now: now(db).await?,
        season: selected.map(Box::new),
        standings,
        run,
        active,
        badges,
    })
}

async fn rewards(db: &impl ConnectionTrait, me: Uuid) -> Result<Vec<api::Badge>, Error> {
    let row = db
        .query_one_raw(statement(
            "SELECT count(*) AS pathfinder,count(*) FILTER(WHERE floor=5) AS summit,
         count(*) FILTER(WHERE floor=5 AND flawless) AS unbroken
         FROM central_identity.tower_accomplishments WHERE account_id=$1",
            vec![me.into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Unavailable)?;
    api::BadgeKind::ALL
        .into_iter()
        .zip(["pathfinder", "summit", "unbroken"])
        .map(|(kind, name)| {
            let count: i64 = row.try_get("", name).map_err(unavailable)?;
            Ok(kind.progress(u32::try_from(count).map_err(unavailable)?))
        })
        .collect()
}

async fn ranking(
    db: &impl ConnectionTrait,
    season: Uuid,
    me: Option<Uuid>,
    board: api::Board,
    offset: u32,
) -> Result<Vec<api::Standing>, Error> {
    let (order, condition) = match board {
        api::Board::Ascent => ("floors DESC,losses,turns", ""),
        api::Board::Sprint => (
            "milliseconds",
            "AND s.floors=5 AND s.losses=0 AND s.milliseconds IS NOT NULL",
        ),
    };
    let rows = db.query_all_raw(statement(&format!(
        "WITH picked AS (
            SELECT s.*,row_number() OVER(PARTITION BY s.account_id ORDER BY {order},s.run_id) AS pick
            FROM central_identity.tower_scores s JOIN central_identity.tower_participation p
              ON p.account_id=s.account_id AND p.season_id=s.season_id
            WHERE s.season_id=$1 AND p.listed {condition}
         ), ranked AS (
            SELECT *,dense_rank() OVER(ORDER BY {order}) AS rank FROM picked WHERE pick=1
         ) SELECT r.*,COALESCE(a.display_name,'Player') AS name,a.tag,COALESCE(r.account_id=$2,false) AS own
         FROM ranked r JOIN central_identity.accounts a ON a.id=r.account_id
         ORDER BY {order},r.account_id LIMIT 30 OFFSET $3"),
        vec![season.into(), me.into(), i64::from(offset).into()])).await.map_err(unavailable)?;
    rows.iter().map(standing).collect()
}

fn standing(row: &QueryResult) -> Result<api::Standing, Error> {
    let rank: i64 = row.try_get("", "rank").map_err(unavailable)?;
    let tag: Option<i32> = row.try_get("", "tag").map_err(unavailable)?;
    let floors: i16 = row.try_get("", "floors").map_err(unavailable)?;
    let losses: i16 = row.try_get("", "losses").map_err(unavailable)?;
    let turns: i32 = row.try_get("", "turns").map_err(unavailable)?;
    let ms: Option<i64> = row.try_get("", "milliseconds").map_err(unavailable)?;
    Ok(api::Standing {
        rank: u32::try_from(rank).map_err(unavailable)?,
        name: row.try_get("", "name").map_err(unavailable)?,
        tag: tag.map(u32::try_from).transpose().map_err(unavailable)?,
        own: row.try_get("", "own").map_err(unavailable)?,
        floors: u8::try_from(floors).map_err(unavailable)?,
        losses: u8::try_from(losses).map_err(unavailable)?,
        turns: u32::try_from(turns).map_err(unavailable)?,
        milliseconds: ms.map(u64::try_from).transpose().map_err(unavailable)?,
    })
}
