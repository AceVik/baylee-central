//! Private executor transport. Inspecting a grant cannot claim either seat or
//! mutate play. Every input, read and attestation binds the current generation.
use super::{
    DatabaseTransaction, Error, Host, QueryResult, Uuid, api, clock, column, evidence, fenced,
    host, journal, phase, privacy, role, routing, rpc,
};

fn live(row: &QueryResult, role: rpc::Role) -> Result<bool, Error> {
    column(
        row,
        match role {
            rpc::Role::Primary => "primary_live",
            rpc::Role::Standby => "standby_live",
        },
    )
}

async fn authorized(
    db: &DatabaseTransaction,
    host: &Host,
    fence: rpc::Fence,
) -> Result<(QueryResult, rpc::Role), Error> {
    let id = host::valid(fence.match_id)?;
    let row = super::lock_match(db, id).await?;
    fenced(&row, fence)?;
    let role = role(db, &row, host).await?;
    let state = phase(&row)?;
    let allowed = if state == rpc::Phase::Finished {
        privacy::consented_for_replay(db, id, host.id).await?
    } else {
        privacy::consented(db, id, host.id).await?
    };
    if !allowed {
        return Err(Error::Rejected);
    }
    if !matches!(state, rpc::Phase::Finished | rpc::Phase::Void)
        && (!live(&row, role)?
            || column::<bool>(&row, "match_expired")?
            || column::<bool>(&row, "recovery_expired")?)
    {
        return Err(Error::Conflict);
    }
    Ok((row, role))
}

pub(in crate::accounts::ranked) async fn legacy(
    db: &DatabaseTransaction,
    host: &Host,
    request: &api::HostRequest,
    id: Uuid,
) -> Result<api::HostResponse, Error> {
    let api::HostRequest::Claim {
        account,
        deck,
        proof,
        ..
    } = request
    else {
        return Err(Error::Rejected);
    };
    let row = super::lock_match(db, id).await?;
    if role(db, &row, host).await? != rpc::Role::Primary
        || !live(&row, rpc::Role::Primary)?
        || !matches!(
            phase(&row)?,
            rpc::Phase::Preparing | rpc::Phase::Playing | rpc::Phase::Paused
        )
        || !privacy::consented(db, id, host.id).await?
    {
        return Err(Error::Rejected);
    }
    host::claim(db, &row, host.id, *account, proof, deck).await
}

pub(in crate::accounts::ranked) async fn handle(
    db: &DatabaseTransaction,
    host: &Host,
    req: &rpc::HostRequest,
) -> Result<rpc::HostResponse, Error> {
    match req {
        rpc::HostRequest::Ready {
            capacity,
            compatibility,
        } => {
            routing::ready(db, host, *capacity, compatibility).await?;
        }
        rpc::HostRequest::Work => return routing::work(db, host).await,
        rpc::HostRequest::Inspect { match_id } => {
            return routing::inspect(db, host, host::valid(*match_id)?).await;
        }
        rpc::HostRequest::Promote { fence } => return routing::promote(db, host, *fence).await,
        rpc::HostRequest::Pulse { fence } => {
            let response = routing::pulse(db, host, *fence).await?;
            clock::sync(db, host::valid(fence.match_id)?).await?;
            return Ok(response);
        }
        rpc::HostRequest::Setup {
            fence,
            game,
            body,
            pools,
        } => {
            let (row, role) = authorized(db, host, *fence).await?;
            if role != rpc::Role::Primary {
                return Err(Error::Rejected);
            }
            journal::setup(db, &row, *game, body, pools).await?;
        }
        rpc::HostRequest::Read { fence, after } | rpc::HostRequest::Tail { fence, after } => {
            let (row, _) = authorized(db, host, *fence).await?;
            if phase(&row)? == rpc::Phase::Void
                || (phase(&row)? == rpc::Phase::Finished && !column::<bool>(&row, "readable")?)
            {
                return Err(Error::Conflict);
            }
            return journal::read(
                db,
                &row,
                *after,
                matches!(req, rpc::HostRequest::Read { .. }),
            )
            .await;
        }
        rpc::HostRequest::Attest {
            fence,
            head,
            state,
            accepted,
            awaited,
            decisions,
            terminal,
        } => {
            let (row, _) = authorized(db, host, *fence).await?;
            evidence::attest(
                db,
                &row,
                host,
                *head,
                rpc::Evidence {
                    state: *state,
                    accepted: *accepted,
                    awaited: *awaited,
                    decisions: *decisions,
                    terminal: *terminal,
                },
            )
            .await?;
        }
        rpc::HostRequest::Presence { fence, attached } => {
            let (row, role) = authorized(db, host, *fence).await?;
            if role != rpc::Role::Primary || *attached > 3 {
                return Err(Error::Rejected);
            }
            clock::host_presence(db, column(&row, "id")?, *attached).await?;
        }
    }
    Ok(rpc::HostResponse::Done)
}
