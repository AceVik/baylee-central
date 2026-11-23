//! The two assigned players may resolve their chairs to public handles while
//! their duel is live. No host identity, watch ticket or global player id is
//! returned by this presentation-only operation.
use super::{Error, rpc, statement, unavailable};
use central_api::profile;
use sea_orm::{ConnectionTrait, DatabaseTransaction, prelude::Uuid};

pub(super) async fn read(
    db: &DatabaseTransaction,
    me: Uuid,
    id: Uuid,
) -> Result<rpc::SeatResponse, Error> {
    let rows = db
        .query_all_raw(statement(
            "SELECT a.display_name, a.tag
         FROM central_identity.ranked_matches m
         CROSS JOIN LATERAL (VALUES (0,m.account_a),(1,m.account_b)) AS s(seat,account_id)
         JOIN central_identity.accounts a ON a.id=s.account_id
         WHERE m.id=$1 AND $2 IN (m.account_a,m.account_b)
           AND m.state IN ('reserved','playing')
         ORDER BY s.seat",
            vec![id.into(), me.into()],
        ))
        .await
        .map_err(unavailable)?;
    let [first, second] = rows.as_slice() else {
        return Err(Error::Rejected);
    };
    let handle = |row| -> Result<Option<String>, Error> {
        let (name, tag) = super::super::super::profiles::columns(row)?;
        Ok(name.zip(tag).map(|(name, tag)| profile::handle(&name, tag)))
    };
    Ok(rpc::SeatResponse::Participants {
        match_id: *id.as_bytes(),
        seats: [handle(first)?, handle(second)?],
    })
}
