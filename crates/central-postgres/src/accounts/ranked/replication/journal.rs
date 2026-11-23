//! Immutable opening and inputs. Every mutation shares the match lock, including
//! duplicate detection, quota checks, recovery and agreement on a transition.
use super::{
    DatabaseTransaction, Error, QueryResult, Uuid, column, digest, head, host, rpc, statement,
    unavailable,
};
use sea_orm::ConnectionTrait;

pub(super) async fn setup(
    db: &DatabaseTransaction,
    row: &QueryResult,
    game: [u8; 16],
    body: &rpc::SetupBytes,
    pools: &[Vec<super::api::PoolEntry>; 2],
) -> Result<(), Error> {
    let id = column::<Uuid>(row, "id")?;
    let opening = rpc::Head::opening(
        id.into_bytes(),
        game,
        digest(row, "compatibility")?,
        super::seed(row)?,
        body,
        pools,
    );
    if let Some(chain) = column::<Option<Vec<u8>>>(row, "opening_chain")? {
        return if chain == opening.chain {
            Ok(())
        } else {
            Err(Error::Conflict)
        };
    }
    if !matches!(
        super::phase(row)?,
        rpc::Phase::Preparing | rpc::Phase::Paused
    ) || !super::routing::healthy(db, row).await?
    {
        return Err(Error::Conflict);
    }
    host::start(db, row, game, pools).await?;
    let frozen = String::from_utf8(central_api::encode(pools)?).map_err(unavailable)?;
    db.execute_raw(statement(
        "UPDATE central_identity.ranked_replicas SET setup=$2,opening_chain=$3,head_digest=$3,
        initial_pools=$4 WHERE match_id=$1",
        vec![
            id.into(),
            body.as_slice().to_vec().into(),
            opening.chain.to_vec().into(),
            frozen.into(),
        ],
    ))
    .await
    .map_err(unavailable)?;
    Ok(())
}

fn source_code(source: rpc::Source) -> i16 {
    match source {
        rpc::Source::Seat0 => 0,
        rpc::Source::Seat1 => 1,
        rpc::Source::Authority => 2,
    }
}

pub(super) async fn pending(db: &DatabaseTransaction, id: Uuid) -> Result<bool, Error> {
    super::exists(
        db,
        "SELECT 1 FROM central_identity.ranked_commands c JOIN central_identity.ranked_replicas r
        USING(match_id) WHERE c.match_id=$1 AND c.position>r.position",
        id,
    )
    .await
}

/// Idempotency survives leadership changes and a certified move advancing the head.
/// Compare the original actor, complete body, and both parts of the original parent.
pub(super) async fn prior(
    db: &DatabaseTransaction,
    id: Uuid,
    command: Uuid,
    source: rpc::Source,
    expected: rpc::Head,
    body: &rpc::CommandBytes,
) -> Result<Option<u64>, Error> {
    let existing = db
        .query_one_raw(statement(
            "SELECT position,source,previous_chain,(body=$3) AS same_body
        FROM central_identity.ranked_commands WHERE match_id=$1 AND command_id=$2",
            vec![id.into(), command.into(), body.as_slice().to_vec().into()],
        ))
        .await
        .map_err(unavailable)?;
    let Some(existing) = existing else {
        return Ok(None);
    };
    let pos = u64::try_from(column::<i64>(&existing, "position")?).map_err(unavailable)?;
    if expected.position.checked_add(1) != Some(pos)
        || column::<i16>(&existing, "source")? != source_code(source)
        || digest(&existing, "previous_chain")? != expected.chain
        || !column::<bool>(&existing, "same_body")?
    {
        return Err(Error::Conflict);
    }
    Ok(Some(pos))
}

pub(super) async fn append(
    db: &DatabaseTransaction,
    row: &QueryResult,
    source: rpc::Source,
    command: Uuid,
    body: &rpc::CommandBytes,
) -> Result<rpc::Head, Error> {
    let id = column::<Uuid>(row, "id")?;
    let previous = head(row)?;
    let size = i32::try_from(body.as_slice().len()).map_err(unavailable)?;
    if column::<i32>(row, "journal_bytes")?.saturating_add(size)
        > i32::try_from(rpc::MAX_JOURNAL_BYTES).map_err(unavailable)?
    {
        return Err(Error::Busy);
    }
    let next = previous.next(id.into_bytes(), command.into_bytes(), source, body)?;
    let inserted=db.execute_raw(statement("INSERT INTO central_identity.ranked_commands
        (match_id,generation,position,command_id,source,previous_chain,chain,body) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
        ON CONFLICT DO NOTHING",vec![id.into(),column::<i64>(row,"generation")?.into(),
        i64::try_from(next.position).map_err(unavailable)?.into(),command.into(),source_code(source).into(),
        previous.chain.to_vec().into(),next.chain.to_vec().into(),body.as_slice().to_vec().into()])).await.map_err(unavailable)?;
    if inserted.rows_affected() != 1 {
        return Err(Error::Conflict);
    }
    db.execute_raw(statement("UPDATE central_identity.ranked_replicas SET journal_bytes=journal_bytes+$2 WHERE match_id=$1",
        vec![id.into(),size.into()])).await.map_err(unavailable)?;
    Ok(next)
}

pub(super) async fn append_authority(
    db: &DatabaseTransaction,
    id: Uuid,
    command: Uuid,
    body: &rpc::CommandBytes,
) -> Result<(), Error> {
    let row = super::lock_match(db, id).await?;
    if super::phase(&row)? != rpc::Phase::Playing || pending(db, id).await? {
        return Err(Error::Conflict);
    }
    if exhausted(&row, body)? {
        return super::void(db, id).await;
    }
    append(db, &row, rpc::Source::Authority, command, body).await?;
    Ok(())
}

pub(super) fn exhausted(row: &QueryResult, body: &rpc::CommandBytes) -> Result<bool, Error> {
    Ok(head(row)?.position >= rpc::MAX_POSITION
        || usize::try_from(column::<i32>(row, "journal_bytes")?)
            .map_err(unavailable)?
            .saturating_add(body.as_slice().len())
            > rpc::MAX_JOURNAL_BYTES)
}

fn entry(row: &QueryResult) -> Result<rpc::Entry, Error> {
    Ok(rpc::Entry {
        head: rpc::Head {
            position: u64::try_from(column::<i64>(row, "position")?).map_err(unavailable)?,
            chain: digest(row, "chain")?,
        },
        command_id: column::<Uuid>(row, "command_id")?.into_bytes(),
        source: match column::<i16>(row, "source")? {
            0 => rpc::Source::Seat0,
            1 => rpc::Source::Seat1,
            2 => rpc::Source::Authority,
            _ => return Err(Error::Unavailable),
        },
        body: rpc::CommandBytes::new(column(row, "body")?)?,
        evidence: read_evidence(row, "evidence")?,
    })
}

pub(super) fn read_evidence(row: &QueryResult, name: &str) -> Result<Option<rpc::Evidence>, Error> {
    column::<Option<String>>(row, name)?
        .map(|s| central_api::decode(s.as_bytes()).map_err(unavailable))
        .transpose()
}

pub(super) async fn read(
    db: &DatabaseTransaction,
    row: &QueryResult,
    after: u64,
    include_opening: bool,
) -> Result<rpc::HostResponse, Error> {
    let id = column::<Uuid>(row, "id")?;
    let certified = head(row)?;
    if after > certified.position && (after != certified.position + 1 || !pending(db, id).await?) {
        return Err(Error::Conflict);
    }
    let chain = column::<Option<Vec<u8>>>(row, "opening_chain")?.ok_or(Error::Conflict)?;
    let opening = rpc::Head {
        position: 0,
        chain: chain.try_into().map_err(unavailable)?,
    };
    let (setup, pools, initial) = if after == 0 && include_opening {
        let data=db.query_one_raw(statement("SELECT setup,initial_pools FROM central_identity.ranked_replicas WHERE match_id=$1",
            vec![id.into()])).await.map_err(unavailable)?.ok_or(Error::Conflict)?;
        let bytes = column::<Option<Vec<u8>>>(&data, "setup")?.ok_or(Error::Conflict)?;
        let pools = column::<Option<String>>(&data, "initial_pools")?.ok_or(Error::Conflict)?;
        (
            Some(rpc::SetupBytes::new(bytes)?),
            Some(central_api::decode(pools.as_bytes()).map_err(unavailable)?),
            read_evidence(row, "initial_evidence")?,
        )
    } else {
        (
            None,
            None,
            if after == 0 {
                read_evidence(row, "initial_evidence")?
            } else {
                None
            },
        )
    };
    // The first page may include a maximum-size private opening and both pools.
    // Keep it below the service response cap even with maximal command bodies.
    let limit = if setup.is_some() { 1 } else { rpc::PAGE };
    let rows=db.query_all_raw(statement("SELECT position,chain,command_id,source,body,evidence FROM central_identity.ranked_commands
        WHERE match_id=$1 AND position>$2 ORDER BY position LIMIT $3",vec![id.into(),i64::try_from(after).map_err(unavailable)?.into(),
        i64::try_from(limit).map_err(unavailable)?.into()])).await.map_err(unavailable)?;
    Ok(rpc::HostResponse::Page {
        opening,
        setup,
        pools,
        initial,
        entries: rows.iter().map(entry).collect::<Result<_, _>>()?,
        certified,
    })
}

pub(super) async fn purge(db: &DatabaseTransaction, id: Uuid) -> Result<(), Error> {
    for sql in [
        "DELETE FROM central_identity.ranked_clocks WHERE match_id=$1",
        "DELETE FROM central_identity.ranked_commands WHERE match_id=$1",
        "DELETE FROM central_identity.ranked_witnesses WHERE match_id=$1",
        "UPDATE central_identity.ranked_replicas SET setup=NULL,initial_pools=NULL,initial_evidence=NULL,evidence=NULL,state_digest=NULL,seed=NULL,retire_at=NULL WHERE match_id=$1",
    ] {
        db.execute_raw(statement(sql, vec![id.into()]))
            .await
            .map_err(unavailable)?;
    }
    Ok(())
}
