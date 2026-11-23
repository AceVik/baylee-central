//! Per-game choices are frozen from the authenticated account's own deck.
//! Other accounts receive only the metadata/bytes displayed at their live table.
use super::{
    DatabaseTransaction, Error, Uuid, api, art, col, inventory, statement, unavailable, uuid,
};
use sea_orm::ConnectionTrait;

pub(super) async fn present(
    db: &DatabaseTransaction,
    me: Uuid,
    id: Uuid,
    look: api::Equipment,
) -> Result<api::Response, Error> {
    db.query_one_raw(statement("SELECT id FROM central_identity.ranked_matches WHERE id=$1 AND $2 IN (account_a,account_b)
        AND state IN ('reserved','playing') AND lease_until>statement_timestamp() FOR NO KEY UPDATE",vec![id.into(),me.into()]))
        .await.map_err(unavailable)?.ok_or(Error::Rejected)?;
    inventory::check_look(db, me, look).await?;
    let vars = || {
        vec![
            id.into(),
            me.into(),
            look.sleeve.map(uuid).into(),
            look.playmat.map(uuid).into(),
        ]
    };
    db.execute_raw(statement(
        "INSERT INTO central_identity.advent_ranked_looks(match_id,account_id,sleeve,playmat)
        VALUES ($1,$2,$3,$4) ON CONFLICT DO NOTHING",
        vars(),
    ))
    .await
    .map_err(unavailable)?;
    let row = db
        .query_one_raw(statement(
            "SELECT 1 FROM central_identity.advent_ranked_looks WHERE match_id=$1 AND account_id=$2
        AND sleeve IS NOT DISTINCT FROM $3 AND playmat IS NOT DISTINCT FROM $4",
            vars(),
        ))
        .await
        .map_err(unavailable)?;
    if row.is_none() {
        return Err(Error::Conflict);
    }
    Ok(api::Response::Presented)
}

async fn chosen(
    db: &DatabaseTransaction,
    me: Uuid,
    id: Uuid,
) -> Result<[api::Equipment; 2], Error> {
    let rows=db.query_all_raw(statement("SELECT l.sleeve,l.playmat FROM central_identity.ranked_matches m
        CROSS JOIN LATERAL (VALUES(0,m.account_a),(1,m.account_b)) s(seat,account_id)
        LEFT JOIN central_identity.advent_ranked_looks l ON l.match_id=m.id AND l.account_id=s.account_id
        WHERE m.id=$1 AND $2 IN(m.account_a,m.account_b) AND m.state='playing' AND m.lease_until>statement_timestamp()
        ORDER BY s.seat",vec![id.into(),me.into()])).await.map_err(unavailable)?;
    let [a, b] = rows.as_slice() else {
        return Err(Error::Rejected);
    };
    let read = |r| -> Result<api::Equipment, Error> {
        Ok(api::Equipment {
            sleeve: col::<Option<Uuid>>(r, "sleeve")?.map(Uuid::into_bytes),
            playmat: col::<Option<Uuid>>(r, "playmat")?.map(Uuid::into_bytes),
        })
    };
    Ok([read(a)?, read(b)?])
}
pub(super) async fn item(
    db: &DatabaseTransaction,
    me: Uuid,
    id: Uuid,
    seat: u8,
    kind: api::Kind,
) -> Result<Uuid, Error> {
    let selected = chosen(db, me, id).await?;
    let selection = selected.get(usize::from(seat)).ok_or(Error::Rejected)?;
    match kind {
        api::Kind::Sleeve => selection.sleeve,
        api::Kind::Playmat => selection.playmat,
    }
    .map(uuid)
    .ok_or(Error::Rejected)
}
pub(super) async fn visible(
    db: &DatabaseTransaction,
    me: Uuid,
    id: Uuid,
) -> Result<api::Response, Error> {
    let mut seats = [api::SeatLooks::default(); 2];
    for (target, selected) in seats.iter_mut().zip(chosen(db, me, id).await?) {
        if let Some(s) = selected.sleeve {
            target.sleeve = Some(art::metadata(db, uuid(s)).await?);
        }
        if let Some(s) = selected.playmat {
            target.playmat = Some(art::metadata(db, uuid(s)).await?);
        }
    }
    Ok(api::Response::TableLooks { seats })
}
