//! An account can act only for its assigned seat. A caller can never supply the
//! source of an input, the authority clock, or the identity of another seat.
use super::{
    DatabaseTransaction, Error, QueryResult, Uuid, api, clock, column, fenced, generation, head,
    host, journal, participants, phase, privacy, protected, queue, routing, rpc, statement,
    unavailable,
};
use sea_orm::ConnectionTrait;

async fn owned(db: &DatabaseTransaction, id: Uuid, me: Uuid) -> Result<QueryResult, Error> {
    db.query_one_raw(statement("SELECT m.id,m.host_id,m.origin,m.game_id,m.state,(m.account_a=$2) AS first
        FROM central_identity.ranked_matches m WHERE m.id=$1 AND $2 IN (m.account_a,m.account_b) FOR NO KEY UPDATE OF m",
        vec![id.into(),me.into()])).await.map_err(unavailable)?.ok_or(Error::Rejected)
}

async fn display(
    db: &DatabaseTransaction,
    id: Uuid,
    me: Uuid,
    row: &QueryResult,
) -> Result<rpc::SeatStatus, Error> {
    let phase = phase(row)?;
    Ok(rpc::SeatStatus {
        fence: rpc::Fence {
            match_id: id.into_bytes(),
            generation: generation(row)?,
        },
        host_id: column::<Uuid>(row, "host_id")?.into_bytes(),
        origin: column(row, "origin")?,
        game: column::<Option<Uuid>>(row, "game_id")?.map(Uuid::into_bytes),
        head: head(row)?,
        phase,
        consent: consent(db, id, me, phase).await?,
        clock: clock::view(db, id).await?,
    })
}

async fn consent(
    db: &DatabaseTransaction,
    id: Uuid,
    me: Uuid,
    phase: rpc::Phase,
) -> Result<rpc::ConsentState, Error> {
    if matches!(phase, rpc::Phase::Void | rpc::Phase::Finished) {
        Ok(rpc::ConsentState::Ready)
    } else {
        privacy::consent_state(db, id, me).await
    }
}

pub(in crate::accounts::ranked) async fn seat_status(
    db: &DatabaseTransaction,
    id: Uuid,
    me: Uuid,
) -> Result<rpc::SeatStatus, Error> {
    let owner = owned(db, id, me).await?;
    if !protected(db, id).await? {
        let phase = match column::<String>(&owner, "state")?.as_str() {
            "finished" => rpc::Phase::Finished,
            "void" => rpc::Phase::Void,
            _ => rpc::Phase::Single,
        };
        return Ok(rpc::SeatStatus {
            fence: rpc::Fence {
                match_id: id.into_bytes(),
                generation: 0,
            },
            host_id: column::<Uuid>(&owner, "host_id")?.into_bytes(),
            origin: column(&owner, "origin")?,
            game: column::<Option<Uuid>>(&owner, "game_id")?.map(Uuid::into_bytes),
            head: rpc::Head {
                position: 0,
                chain: [0; 32],
            },
            phase,
            consent: consent(db, id, me, phase).await?,
            clock: None,
        });
    }
    routing::maintain(db, id).await?;
    clock::sync(db, id).await?;
    display(db, id, me, &super::lock_match(db, id).await?).await
}

async fn submit(
    db: &DatabaseTransaction,
    me: Uuid,
    fence: rpc::Fence,
    expected: rpc::Head,
    command: [u8; 16],
    body: &rpc::CommandBytes,
) -> Result<rpc::SeatResponse, Error> {
    let id = host::valid(fence.match_id)?;
    let command = host::valid(command)?;
    let owner = owned(db, id, me).await?;
    let row = super::lock_match(db, id).await?;
    fenced(&row, fence)?;
    let source = if column::<bool>(&owner, "first")? {
        rpc::Source::Seat0
    } else {
        rpc::Source::Seat1
    };
    if let Some(position) = journal::prior(db, id, command, source, expected, body).await? {
        return Ok(rpc::SeatResponse::Receipt {
            command_id: command.into_bytes(),
            position,
        });
    }
    routing::maintain(db, id).await?;
    clock::sync(db, id).await?;
    let row = super::lock_match(db, id).await?;
    if generation(&row)? != fence.generation
        || phase(&row)? != rpc::Phase::Playing
        || !routing::healthy(db, &row).await?
        || !clock::actionable(db, id).await?
    {
        return Ok(rpc::SeatResponse::Status {
            status: display(db, id, me, &row).await?,
        });
    }
    if head(&row)? != expected {
        return Err(Error::Conflict);
    }
    if journal::pending(db, id).await? {
        return Ok(rpc::SeatResponse::Status {
            status: display(db, id, me, &row).await?,
        });
    }
    if journal::exhausted(&row, body)? {
        super::void(db, id).await?;
        let row = super::lock_match(db, id).await?;
        return Ok(rpc::SeatResponse::Status {
            status: display(db, id, me, &row).await?,
        });
    }
    let next = journal::append(db, &row, source, command, body).await?;
    Ok(rpc::SeatResponse::Receipt {
        command_id: command.into_bytes(),
        position: next.position,
    })
}

pub(in crate::accounts::ranked) async fn seat(
    db: &DatabaseTransaction,
    me: Uuid,
    request: rpc::SeatRequest,
) -> Result<rpc::SeatResponse, Error> {
    match request {
        rpc::SeatRequest::RelayRoute { fence } => super::relay::route(db, me, fence).await,
        rpc::SeatRequest::Relay { .. } => Err(Error::Rejected),
        rpc::SeatRequest::Join {
            key,
            format,
            deck,
            ai,
        } => Ok(rpc::SeatResponse::Queue {
            state: queue::handle(db, me, &api::Request::Join { key, format, deck }, true, ai)
                .await?,
        }),
        rpc::SeatRequest::Status { match_id } => Ok(rpc::SeatResponse::Status {
            status: seat_status(db, host::valid(match_id)?, me).await?,
        }),
        rpc::SeatRequest::Participants { match_id } => {
            participants::read(db, me, host::valid(match_id)?).await
        }
        rpc::SeatRequest::Hosts { .. }
        | rpc::SeatRequest::ApproveHosts { .. }
        | rpc::SeatRequest::DeclineHost { .. } => privacy::seat(db, me, &request).await,
        rpc::SeatRequest::Submit {
            fence,
            expected,
            command_id,
            body,
        } => submit(db, me, fence, expected, command_id, &body).await,
        rpc::SeatRequest::Presence { fence, connected } => {
            let id = host::valid(fence.match_id)?;
            let owner = owned(db, id, me).await?;
            let row = super::lock_match(db, id).await?;
            fenced(&row, fence)?;
            let index = usize::from(!column::<bool>(&owner, "first")?);
            clock::seat_presence(db, id, index, connected).await?;
            Ok(rpc::SeatResponse::Status {
                status: seat_status(db, id, me).await?,
            })
        }
    }
}
