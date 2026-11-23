//! Completion, permanent ownership and per-deck entitlement checks.
use super::{
    DatabaseTransaction, Error, QueryResult, Uuid, api, art, col, statement, unavailable, uuid,
};
use sea_orm::ConnectionTrait;

const REWARDS: &str =
    "SELECT a.id,a.kind,a.title_de,a.title_en,a.digest,a.size,a.mime,a.tier,a.width,a.height,
    e.year,e.door,floor(extract(epoch FROM e.earned_at))::bigint AS earned
    FROM central_identity.advent_entitlements e JOIN central_identity.advent_art a ON a.id=e.art_id
    WHERE e.account_id=$1";

fn reward(row: &QueryResult) -> Result<api::Reward, Error> {
    let kind = match col::<String>(row, "kind")?.as_str() {
        "sleeve" => api::Kind::Sleeve,
        "playmat" => api::Kind::Playmat,
        _ => return Err(Error::Unavailable),
    };
    Ok(api::Reward {
        id: col::<Uuid>(row, "id")?.into_bytes(),
        kind,
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
        "SELECT 1 FROM central_identity.advent_entitlements WHERE account_id=$1 AND year=$2 AND door=$3",vars()
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
    let year = i16::try_from(year).map_err(|_| Error::Rejected)?;
    let row = db
        .query_one_raw(statement(
            &format!("{REWARDS} AND e.year=$2 AND e.door=$3"),
            vec![me.into(), year.into(), i16::from(door).into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Rejected)?;
    Ok(api::Response::Reward {
        reward: reward(&row)?,
    })
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
pub(super) async fn owned(
    db: &DatabaseTransaction,
    me: Uuid,
    id: Uuid,
    kind: Option<api::Kind>,
) -> Result<(), Error> {
    db.query_one_raw(statement("SELECT a.id FROM central_identity.advent_entitlements e JOIN central_identity.advent_art a ON a.id=e.art_id
        WHERE e.account_id=$1 AND a.id=$2 AND ($3::text IS NULL OR a.kind=$3)",vec![me.into(),id.into(),kind.map(api::Kind::as_str).into()]))
        .await.map_err(unavailable)?.ok_or(Error::Rejected)?;
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
            owned(db, me, uuid(id), Some(kind)).await?;
        }
    }
    Ok(())
}
