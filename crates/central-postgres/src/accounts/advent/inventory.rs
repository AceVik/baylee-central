//! Completion, permanent ownership and per-deck entitlement checks.
use super::{
    DatabaseTransaction, Error, QueryResult, Uuid, api, art, col, statement, unavailable, uuid,
};
use sea_orm::ConnectionTrait;

const REWARDS: &str =
    "SELECT a.id,a.kind,a.title_de,a.title_en,a.digest,a.size,a.mime,a.tier,a.width,a.height,f.framing::text AS mat_framing,
    e.year,e.door,floor(extract(epoch FROM e.earned_at))::bigint AS earned
    FROM central_identity.advent_entitlements e JOIN central_identity.advent_art a ON a.id=e.art_id
    LEFT JOIN central_identity.playmat_framing f ON f.art_id=a.id
    WHERE e.account_id=$1";

const SOLVED_ITEM:&str="EXISTS (SELECT 1 FROM central_identity.advent_solutions s JOIN central_identity.advent_doors d USING(year,door)
    WHERE s.account_id=e.account_id AND s.year=$2 AND s.door=$3 AND d.art_id=e.art_id)";

fn kind(row: &QueryResult) -> Result<api::Kind, Error> {
    match col::<String>(row, "kind")?.as_str() {
        "sleeve" => Ok(api::Kind::Sleeve),
        "playmat" => Ok(api::Kind::Playmat),
        _ => Err(Error::Unavailable),
    }
}

fn reward(row: &QueryResult) -> Result<api::Reward, Error> {
    Ok(api::Reward {
        id: col::<Uuid>(row, "id")?.into_bytes(),
        kind: kind(row)?,
        title: api::Text {
            de: col(row, "title_de")?,
            en: col(row, "title_en")?,
        },
        year: u16::try_from(col::<i16>(row, "year")?).map_err(unavailable)?,
        door: u8::try_from(col::<i16>(row, "door")?).map_err(unavailable)?,
        earned_at: col(row, "earned")?,
        asset: art::meta(row)?,
    })
}
pub(super) async fn grant(
    db: &DatabaseTransaction,
    me: Uuid,
    year: u16,
    door: u8,
) -> Result<(), Error> {
    let year = i16::try_from(year).map_err(|_| Error::Rejected)?;
    let vars = || vec![me.into(), year.into(), i16::from(door).into()];
    let inserted = db
        .execute_raw(statement(
            "INSERT INTO central_identity.advent_entitlements(account_id,year,door,art_id)
        SELECT s.account_id,s.year,s.door,d.art_id FROM central_identity.advent_solutions s
        JOIN central_identity.advent_doors d USING(year,door)
        JOIN central_identity.advent_art a ON a.id=d.art_id AND a.kind IN ('sleeve','playmat')
        WHERE s.account_id=$1 AND s.year=$2 AND s.door=$3 ON CONFLICT DO NOTHING",
            vars(),
        ))
        .await
        .map_err(unavailable)?;
    if inserted.rows_affected()==0 && db.query_one_raw(statement(
        &format!("SELECT 1 FROM central_identity.advent_entitlements e WHERE e.account_id=$1 AND {SOLVED_ITEM}"),vars()
    )).await.map_err(unavailable)?.is_none(){return Err(Error::Rejected)}
    Ok(())
}

pub(super) async fn claim(
    db: &DatabaseTransaction,
    me: Uuid,
    year: u16,
    door: u8,
) -> Result<api::Response, Error> {
    // An earlier solved row remains collectible even after its campaign closes.
    grant(db, me, year, door).await?;
    let claim_year = i16::try_from(year).map_err(|_| Error::Rejected)?;
    let row = db
        .query_one_raw(statement(
            &format!("{REWARDS} AND {SOLVED_ITEM}"),
            vec![me.into(), claim_year.into(), i16::from(door).into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Rejected)?;
    let mut reward = reward(&row)?;
    reward.year = year;
    reward.door = door;
    Ok(api::Response::Reward { reward })
}
pub(super) async fn list(
    db: &DatabaseTransaction,
    me: Uuid,
    after: Option<Uuid>,
) -> Result<api::Response, Error> {
    let rows = db
        .query_all_raw(statement(
            &format!("{REWARDS} AND ($2::uuid IS NULL OR e.art_id>$2) ORDER BY e.art_id LIMIT 33"),
            vec![me.into(), after.into()],
        ))
        .await
        .map_err(unavailable)?;
    let items = rows
        .iter()
        .take(32)
        .map(reward)
        .collect::<Result<Vec<_>, _>>()?;
    let next = if rows.len() > 32 {
        items.last().map(|r| r.id)
    } else {
        None
    };
    Ok(api::Response::Inventory { items, next })
}
pub(super) async fn owned(db: &DatabaseTransaction, me: Uuid, id: Uuid) -> Result<(), Error> {
    db.query_one_raw(statement(
        "SELECT 1 FROM central_identity.cosmetic_collection WHERE account_id=$1 AND art_id=$2",
        vec![me.into(), id.into()],
    ))
    .await
    .map_err(unavailable)?
    .ok_or(Error::Rejected)?;
    Ok(())
}
pub(super) async fn check_look(
    db: &DatabaseTransaction,
    me: Uuid,
    look: api::Equipment,
) -> Result<(), Error> {
    for (kind, slot) in [
        (api::Kind::Sleeve, look.sleeve),
        (api::Kind::Playmat, look.playmat),
    ] {
        if let Some(id) = slot {
            db.query_one_raw(statement("WITH eligible AS (
                SELECT e.* FROM central_identity.cosmetic_collection e
                JOIN central_identity.advent_art a ON a.id=e.art_id
                WHERE e.account_id=$1 AND e.art_id=$2 AND a.kind=$3
            ), hold AS (
                INSERT INTO central_identity.cosmetic_equip_rights(account_id,art_id,advent_id,league,division)
                SELECT account_id,art_id,CASE WHEN year IS NOT NULL THEN art_id END,league,division FROM eligible
                ON CONFLICT DO NOTHING)
                SELECT 1 FROM eligible",vec![me.into(),uuid(id).into(),kind.as_str().into()]))
                .await.map_err(unavailable)?.ok_or(Error::Rejected)?;
        }
    }
    Ok(())
}

pub(super) async fn collection(
    db: &DatabaseTransaction,
    me: Uuid,
    after: Option<Uuid>,
) -> Result<api::Response, Error> {
    let rows = db.query_all_raw(statement(
        "SELECT a.id,a.kind,a.title_de,a.title_en,a.digest,a.size,a.mime,a.tier,a.width,a.height,
            f.framing::text AS mat_framing,e.year,e.door,e.league,e.division,
            floor(extract(epoch FROM e.earned_at))::bigint AS earned
         FROM central_identity.cosmetic_collection e
         JOIN central_identity.advent_art a ON a.id=e.art_id
         LEFT JOIN central_identity.playmat_framing f ON f.art_id=a.id
         WHERE e.account_id=$1 AND ($2::uuid IS NULL OR e.art_id>$2)
         ORDER BY e.art_id LIMIT 33",
        vec![me.into(), after.into()])).await.map_err(unavailable)?;
    let items = rows
        .iter()
        .take(32)
        .map(collection_item)
        .collect::<Result<Vec<_>, _>>()?;
    let next = (rows.len() > 32)
        .then(|| items.last().map(|item| item.id))
        .flatten();
    Ok(api::Response::Collection { items, next })
}

fn collection_item(row: &QueryResult) -> Result<api::collection::Item, Error> {
    if col::<Option<i16>>(row, "year")?.is_some() {
        return reward(row).map(Into::into);
    }
    let index = usize::try_from(col::<i16>(row, "league")?).map_err(unavailable)?;
    let league = *central_api::ranked::League::PLACED
        .get(index)
        .ok_or(Error::Unavailable)?;
    let numeral = u8::try_from(col::<i16>(row, "division")?).map_err(unavailable)?;
    Ok(api::collection::Item {
        id: col::<Uuid>(row, "id")?.into_bytes(),
        kind: kind(row)?,
        title: api::Text {
            de: col(row, "title_de")?,
            en: col(row, "title_en")?,
        },
        asset: art::meta(row)?,
        earned_at: col(row, "earned")?,
        source: api::collection::Source::Ranked {
            league,
            division: (numeral > 0).then_some(numeral),
        },
    })
}
