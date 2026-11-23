//! Private reconnect record. Only a fresh owner session can turn it into an
//! invitation, and every new invitation still needs a real host seat grant.
use super::{DatabaseTransaction, Error, Proof, Uuid, api, col, statement, unavailable};
use sea_orm::ConnectionTrait;

pub(super) async fn prune(db: &DatabaseTransaction) -> Result<(), Error> {
    db.execute_raw(statement("DELETE FROM central_identity.advent_table_outfits WHERE
        (gateway_id,game,seat) IN (SELECT gateway_id,game,seat FROM central_identity.advent_table_outfits
        WHERE expires_at<=clock_timestamp() ORDER BY expires_at LIMIT 64)",vec![])).await.map_err(unavailable)?;
    Ok(())
}

pub(super) async fn clear(
    db: &DatabaseTransaction,
    owner: Uuid,
    host: Uuid,
    game: Uuid,
    seat: i16,
) -> Result<(), Error> {
    let bind = || vec![owner.into(), host.into(), game.into(), seat.into()];
    db.execute_raw(statement(
        "DELETE FROM central_identity.advent_table_outfits
        WHERE account_id=$1 AND gateway_id=$2 AND game=$3 AND seat=$4",
        bind(),
    ))
    .await
    .map_err(unavailable)?;
    db.execute_raw(statement(
        "DELETE FROM central_identity.advent_table_looks
        WHERE account_id=$1 AND gateway_id=$2 AND game=$3 AND seat=$4",
        bind(),
    ))
    .await
    .map_err(unavailable)?;
    Ok(())
}

pub(super) async fn prior(
    db: &DatabaseTransaction,
    owner: Uuid,
    host: Uuid,
    game: Uuid,
    seat: i16,
) -> Result<Option<api::Equipment>, Error> {
    let row=db.query_one_raw(statement("SELECT sleeve,playmat FROM central_identity.advent_table_outfits
        WHERE account_id=$1 AND gateway_id=$2 AND game=$3 AND seat=$4 AND expires_at>clock_timestamp()",
        vec![owner.into(),host.into(),game.into(),seat.into()])).await.map_err(unavailable)?;
    row.map(|row| {
        Ok(api::Equipment {
            sleeve: col::<Option<Uuid>>(&row, "sleeve")?.map(Uuid::into_bytes),
            playmat: col::<Option<Uuid>>(&row, "playmat")?.map(Uuid::into_bytes),
        })
    })
    .transpose()
}

pub(super) async fn remember(
    db: &DatabaseTransaction,
    host: Uuid,
    game: Uuid,
    seat: i16,
    proof: &Proof,
) -> Result<(), Error> {
    db.execute_raw(statement(
        "DELETE FROM central_identity.advent_table_outfits
        WHERE gateway_id=$1 AND game=$2 AND seat=$3",
        vec![host.into(), game.into(), seat.into()],
    ))
    .await
    .map_err(unavailable)?;
    let saved=db.execute_raw(statement("INSERT INTO central_identity.advent_table_outfits
        (account_id,gateway_id,game,seat,stream,sleeve,playmat)
        SELECT l.account_id,l.gateway_id,l.game,l.seat,i.stream,l.sleeve,l.playmat
        FROM central_identity.advent_table_looks l JOIN central_identity.gateway_sids i ON i.sid=l.sid
        WHERE l.proof=$1 AND l.confirmed",vec![proof.digest().to_vec().into()])).await.map_err(unavailable)?;
    if saved.rows_affected() != 1 {
        return Err(Error::Rejected);
    }
    Ok(())
}

pub(super) async fn close(db: &DatabaseTransaction, host: Uuid, game: Uuid) -> Result<(), Error> {
    let bind = || vec![host.into(), game.into()];
    db.execute_raw(statement(
        "DELETE FROM central_identity.advent_table_looks WHERE gateway_id=$1 AND game=$2",
        bind(),
    ))
    .await
    .map_err(unavailable)?;
    db.execute_raw(statement(
        "DELETE FROM central_identity.advent_table_outfits WHERE gateway_id=$1 AND game=$2",
        bind(),
    ))
    .await
    .map_err(unavailable)?;
    Ok(())
}
