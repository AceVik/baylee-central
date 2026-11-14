//! Bounded private staging followed by atomic activation; no partial public tower.

use super::{api, json, manifest, now, statement, unavailable, uuid};
use central_api::{Error, challenges};
use sea_orm::{ConnectionTrait, DatabaseTransaction, QueryResult, prelude::Uuid};

async fn stock(db: &impl ConnectionTrait, row: &QueryResult) -> Result<api::Stock, Error> {
    let id: Uuid = row.try_get("", "id").map_err(unavailable)?;
    let nodes = db
        .query_all_raw(statement(
            "SELECT node FROM central_identity.tower_nodes WHERE season_id=$1 ORDER BY node",
            vec![id.into()],
        ))
        .await
        .map_err(unavailable)?;
    let staged = nodes
        .iter()
        .map(|row| {
            let node: i16 = row.try_get("", "node").map_err(unavailable)?;
            u8::try_from(node).map_err(unavailable)
        })
        .collect::<Result<_, _>>()?;
    Ok(api::Stock {
        id: *id.as_bytes(),
        period: manifest(row)?.theme.period,
        staged,
        published: row.try_get("", "published").map_err(unavailable)?,
        withdrawn: row.try_get("", "withdrawn").map_err(unavailable)?,
    })
}

async fn row(db: &impl ConnectionTrait, id: [u8; 16]) -> Result<QueryResult, Error> {
    // Season keys never change. Serialize host mutations while allowing reward
    // foreign-key KEY SHARE locks from a result already holding its node SHARE lock.
    db.query_one_raw(statement(
        "SELECT id,manifest::text,published,withdrawn FROM central_identity.tower_seasons WHERE id=$1 FOR NO KEY UPDATE",
        vec![uuid(id).into()])).await.map_err(unavailable)?.ok_or(Error::Rejected)
}

pub(in crate::accounts::challenges) async fn dispatch(
    db: &DatabaseTransaction,
    request: api::HostRequest,
) -> Result<Vec<api::Stock>, Error> {
    let result = match request {
        api::HostRequest::Inventory { from, until } => {
            if from < 0
                || until
                    .checked_sub(from)
                    .is_none_or(|span| !(1..=32 * 86_400).contains(&span))
            {
                return Err(Error::Rejected);
            }
            let rows = db.query_all_raw(statement(
                "SELECT id,manifest::text,published,withdrawn FROM central_identity.tower_seasons
                 WHERE starts_at>=$1 AND starts_at<$2 ORDER BY starts_at LIMIT 7", vec![from.into(), until.into()]))
                .await.map_err(unavailable)?;
            let mut results = Vec::with_capacity(rows.len());
            for row in &rows {
                results.push(stock(db, row).await?);
            }
            return Ok(results);
        }
        api::HostRequest::Stage {
            manifest,
            node,
            publication,
        } => stage(db, &manifest, node, &publication).await?,
        api::HostRequest::Activate { season } => {
            let before = row(db, season).await?;
            if before
                .try_get::<bool>("", "withdrawn")
                .map_err(unavailable)?
            {
                return Err(Error::Conflict);
            }
            let manifest = manifest(&before)?;
            let rows = db.query_all_raw(statement(
                "SELECT n.node,c.digest,c.withdrawn FROM central_identity.tower_nodes n
                 JOIN central_identity.challenges c ON c.id=n.challenge_id WHERE season_id=$1 ORDER BY n.node",
                vec![uuid(season).into()])).await.map_err(unavailable)?;
            if rows.len() != api::NODES {
                return Err(Error::Conflict);
            }
            for (index, row) in rows.iter().enumerate() {
                let node: i16 = row.try_get("", "node").map_err(unavailable)?;
                let digest: Vec<u8> = row.try_get("", "digest").map_err(unavailable)?;
                if usize::try_from(node).ok() != Some(index)
                    || digest != manifest.nodes[index]
                    || row.try_get::<bool>("", "withdrawn").map_err(unavailable)?
                {
                    return Err(Error::Conflict);
                }
            }
            db.execute_raw(statement(
                "UPDATE central_identity.tower_seasons SET published=true WHERE id=$1",
                vec![uuid(season).into()],
            ))
            .await
            .map_err(unavailable)?;
            row(db, season).await?
        }
        api::HostRequest::Withdraw { season } => {
            row(db, season).await?;
            db.execute_raw(statement(
                "UPDATE central_identity.tower_seasons SET withdrawn=true WHERE id=$1",
                vec![uuid(season).into()],
            ))
            .await
            .map_err(unavailable)?;
            db.execute_raw(statement(
                "UPDATE central_identity.challenges SET withdrawn=true WHERE id IN
                 (SELECT challenge_id FROM central_identity.tower_nodes WHERE season_id=$1)",
                vec![uuid(season).into()],
            ))
            .await
            .map_err(unavailable)?;
            row(db, season).await?
        }
    };
    Ok(vec![stock(db, &result).await?])
}

async fn stage(
    db: &DatabaseTransaction,
    value: &api::Manifest,
    node: u8,
    publication: &challenges::Publication,
) -> Result<QueryResult, Error> {
    value.validate_node(node, publication)?;
    let period = value.theme.period;
    let time = now(db).await?;
    if period.starts_at < time - 7 * 86_400 || period.starts_at > time + 32 * 86_400 {
        return Err(Error::Rejected);
    }
    db.execute_raw(statement(
        "INSERT INTO central_identity.tower_seasons(starts_at,ends_at,manifest) VALUES($1,$2,$3::jsonb)
         ON CONFLICT(starts_at) DO NOTHING",
        vec![period.starts_at.into(), period.ends_at.into(), json(central_api::encode(value)?)?.into()])).await.map_err(unavailable)?;
    let season = db.query_one_raw(statement(
        "SELECT id,manifest::text,published,withdrawn FROM central_identity.tower_seasons WHERE starts_at=$1 FOR NO KEY UPDATE",
        vec![period.starts_at.into()])).await.map_err(unavailable)?.ok_or(Error::Unavailable)?;
    if manifest(&season)? != *value
        || season
            .try_get::<bool>("", "withdrawn")
            .map_err(unavailable)?
    {
        return Err(Error::Conflict);
    }
    let id: Uuid = season.try_get("", "id").map_err(unavailable)?;
    db.execute_raw(statement(
        "INSERT INTO central_identity.challenges(kind,starts_at,ends_at,scope,description,digest,publication)
         VALUES('weekly',$1,$2,$3,$4::jsonb,$5,$6) ON CONFLICT(kind,starts_at,scope) DO NOTHING",
        vec![period.starts_at.into(), period.ends_at.into(), (i16::from(node)+1).into(),
            json(central_api::encode(&publication.description)?)?.into(), publication.digest.to_vec().into(), central_api::encode(publication)?.into()]))
        .await.map_err(unavailable)?;
    let encounter = db.query_one_raw(statement(
        "SELECT id,digest FROM central_identity.challenges WHERE kind='weekly' AND starts_at=$1 AND scope=$2",
        vec![period.starts_at.into(), (i16::from(node)+1).into()])).await.map_err(unavailable)?.ok_or(Error::Unavailable)?;
    if encounter
        .try_get::<Vec<u8>>("", "digest")
        .map_err(unavailable)?
        != publication.digest
    {
        return Err(Error::Conflict);
    }
    let challenge: Uuid = encounter.try_get("", "id").map_err(unavailable)?;
    db.execute_raw(statement(
        "INSERT INTO central_identity.tower_nodes(season_id,node,challenge_id) VALUES($1,$2,$3)
         ON CONFLICT(season_id,node) DO NOTHING",
        vec![id.into(), i16::from(node).into(), challenge.into()],
    ))
    .await
    .map_err(unavailable)?;
    Ok(season)
}
