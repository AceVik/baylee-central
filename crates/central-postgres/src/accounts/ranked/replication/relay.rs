//! Resolve an authenticated participant to a canonical verified destination.
//! The native relay never accepts an arbitrary origin, route, player identity,
//! or game from the participant's network request.
use super::{
    DatabaseTransaction, Error, Uuid, column, fenced, host, privacy, protected, rpc, statement,
    unavailable,
};
use sea_orm::ConnectionTrait;

pub(super) async fn route(
    db: &DatabaseTransaction,
    me: Uuid,
    fence: rpc::Fence,
) -> Result<rpc::SeatResponse, Error> {
    let id = host::valid(fence.match_id)?;
    let row=db.query_one_raw(statement("SELECT m.host_id,m.origin,m.game_id,(m.account_b=$2) AS second,m.ranked_epoch,
        m.created_at>=h.continuity_since AS continuous FROM central_identity.ranked_matches m
        JOIN central_identity.gateways g ON g.id=m.host_id AND g.origin=m.origin
        JOIN central_identity.gateway_approvals p ON p.gateway_id=g.id
        JOIN central_identity.ranked_hosts h ON h.host_id=g.id
        WHERE m.id=$1 AND $2 IN (m.account_a,m.account_b) AND m.state IN ('reserved','playing')
        AND m.lease_until>clock_timestamp() AND p.enabled AND p.ranked_verified
        AND m.ranked_epoch=p.ranked_epoch AND h.ranked_epoch=m.ranked_epoch AND h.seen_at>clock_timestamp()-interval '90 seconds'
        FOR NO KEY UPDATE OF m FOR SHARE OF p,g,h",vec![id.into(),me.into()]))
        .await.map_err(unavailable)?.ok_or(Error::Rejected)?;
    let authority = host::Host {
        id: column::<Uuid>(&row, "host_id")?,
        epoch: column(&row, "ranked_epoch")?,
        origin: column(&row, "origin")?,
    };
    if protected(db, id).await? {
        let current = super::lock_match(db, id).await?;
        fenced(&current, fence)?;
        if super::role(db, &current, &authority).await? != rpc::Role::Primary
            || !column::<bool>(&current, "primary_live")?
            || !matches!(
                super::phase(&current)?,
                rpc::Phase::Preparing | rpc::Phase::Playing | rpc::Phase::Paused
            )
        {
            return Err(Error::Conflict);
        }
    } else if fence.generation != 0 || !column::<bool>(&row, "continuous")? {
        return Err(Error::Conflict);
    }
    if !privacy::consented(db, id, authority.id).await? {
        return Err(Error::Rejected);
    }
    Ok(rpc::SeatResponse::RelayRoute {
        origin: authority.origin,
        game: column::<Option<Uuid>>(&row, "game_id")?.map(Uuid::into_bytes),
        seat: u8::from(column::<bool>(&row, "second")?),
    })
}
