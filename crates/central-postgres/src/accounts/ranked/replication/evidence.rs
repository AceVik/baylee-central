//! Immutable votes for exact inputs. A disagreement commits a suspension, so
//! its transaction must return success rather than rolling the suspension back.
use super::{
    DatabaseTransaction, Error, Host, QueryResult, Uuid, clock, column, digest, head, host,
    journal, routing, rpc, statement, unavailable,
};
use sea_orm::ConnectionTrait;

async fn dispute(db: &DatabaseTransaction, id: Uuid) -> Result<(), Error> {
    db.execute_raw(statement(
        "UPDATE central_identity.ranked_replicas SET phase='disputed',
        paused_until=COALESCE(paused_until,clock_timestamp()+interval '5 minutes') WHERE match_id=$1
        AND phase NOT IN ('finished','void')",
        vec![id.into()],
    ))
    .await
    .map_err(unavailable)?;
    Ok(())
}

fn valid(proof: rpc::Evidence) -> bool {
    proof.awaited <= 3
        && proof
            .decisions
            .iter()
            .enumerate()
            .all(|(i, d)| d.is_some() == (proof.awaited & (1 << i) != 0))
        && (proof.terminal.is_none() || (proof.accepted && proof.awaited == 0))
}

struct Target {
    chain: rpc::Digest,
    certified: Option<rpc::Evidence>,
    authority: bool,
}

async fn target(
    db: &DatabaseTransaction,
    row: &QueryResult,
    position: u64,
) -> Result<Target, Error> {
    if position == 0 {
        return Ok(Target {
            chain: column::<Option<Vec<u8>>>(row, "opening_chain")?
                .ok_or(Error::Conflict)?
                .try_into()
                .map_err(unavailable)?,
            certified: journal::read_evidence(row, "initial_evidence")?,
            authority: false,
        });
    }
    let item = db
        .query_one_raw(statement(
            "SELECT chain,evidence,source FROM central_identity.ranked_commands
        WHERE match_id=$1 AND position=$2",
            vec![
                column::<Uuid>(row, "id")?.into(),
                i64::try_from(position).map_err(unavailable)?.into(),
            ],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Conflict)?;
    Ok(Target {
        chain: digest(&item, "chain")?,
        certified: journal::read_evidence(&item, "evidence")?,
        authority: column::<i16>(&item, "source")? == 2,
    })
}

async fn vote(
    db: &DatabaseTransaction,
    row: &QueryResult,
    host: &Host,
    cursor: rpc::Head,
    proof: &str,
) -> Result<bool, Error> {
    let id = column::<Uuid>(row, "id")?;
    let generation = column::<i64>(row, "generation")?;
    let pos = i64::try_from(cursor.position).map_err(unavailable)?;
    let vars = || {
        vec![
            id.into(),
            generation.into(),
            pos.into(),
            host.id.into(),
            cursor.chain.to_vec().into(),
            proof.into(),
        ]
    };
    db.execute_raw(statement("INSERT INTO central_identity.ranked_witnesses(match_id,generation,position,host_id,chain,evidence)
       VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT DO NOTHING",vars())).await.map_err(unavailable)?;
    let stored = db
        .query_one_raw(statement(
            "SELECT (chain=$5 AND evidence=$6) AS same FROM central_identity.ranked_witnesses
        WHERE match_id=$1 AND generation=$2 AND position=$3 AND host_id=$4",
            vars(),
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Unavailable)?;
    column(&stored, "same")
}

async fn peer(
    db: &DatabaseTransaction,
    row: &QueryResult,
    host: &Host,
    cursor: rpc::Head,
) -> Result<Option<rpc::Evidence>, Error> {
    let first = column::<Uuid>(row, "host_id")?;
    let second = column::<Option<Uuid>>(row, "standby_id")?;
    let other = if first == host.id {
        second
    } else {
        Some(first)
    };
    let r = db
        .query_one_raw(statement(
            "SELECT evidence FROM central_identity.ranked_witnesses
        WHERE match_id=$1 AND generation=$2 AND position=$3 AND host_id=$4 AND chain=$5",
            vec![
                column::<Uuid>(row, "id")?.into(),
                column::<i64>(row, "generation")?.into(),
                i64::try_from(cursor.position).map_err(unavailable)?.into(),
                other.into(),
                cursor.chain.to_vec().into(),
            ],
        ))
        .await
        .map_err(unavailable)?;
    r.as_ref()
        .map(|r| journal::read_evidence(r, "evidence"))
        .transpose()
        .map(Option::flatten)
}

async fn certify(
    db: &DatabaseTransaction,
    row: &QueryResult,
    cursor: rpc::Head,
    proof: rpc::Evidence,
    encoded: String,
) -> Result<(), Error> {
    let id = column::<Uuid>(row, "id")?;
    let pos = i64::try_from(cursor.position).map_err(unavailable)?;
    if pos > 0 {
        db.execute_raw(statement("UPDATE central_identity.ranked_commands SET evidence=$3 WHERE match_id=$1 AND position=$2 AND evidence IS NULL",
            vec![id.into(),pos.into(),encoded.as_str().into()])).await.map_err(unavailable)?;
    }
    db.execute_raw(statement(
        "UPDATE central_identity.ranked_replicas SET position=$2,head_digest=$3,state_digest=$4,
       evidence=$5,initial_evidence=CASE WHEN $2=0 THEN $5 ELSE initial_evidence END,
       phase='playing',paused_until=NULL WHERE match_id=$1",
        vec![
            id.into(),
            pos.into(),
            cursor.chain.to_vec().into(),
            proof.state.to_vec().into(),
            encoded.into(),
        ],
    ))
    .await
    .map_err(unavailable)?;
    if let Some(result) = proof.terminal {
        let game = column::<Option<Uuid>>(row, "game_id")?.ok_or(Error::Conflict)?;
        host::finish(db, row, game.into_bytes(), result).await?;
        db.execute_raw(statement(
            "UPDATE central_identity.ranked_replicas SET phase='finished',
            retire_at=clock_timestamp()+interval '2 minutes' WHERE match_id=$1",
            vec![id.into()],
        ))
        .await
        .map_err(unavailable)?;
    }
    clock::certified(db, id, &proof).await
}

pub(super) async fn attest(
    db: &DatabaseTransaction,
    row: &QueryResult,
    host: &Host,
    cursor: rpc::Head,
    proof: rpc::Evidence,
) -> Result<(), Error> {
    if !valid(proof) {
        return Err(Error::Rejected);
    }
    let id = column::<Uuid>(row, "id")?;
    let phase = super::phase(row)?;
    let certified = head(row)?;
    if phase == rpc::Phase::Finished {
        return if certified == cursor && journal::read_evidence(row, "evidence")? == Some(proof) {
            Ok(())
        } else {
            Err(Error::Conflict)
        };
    }
    if matches!(phase, rpc::Phase::Void | rpc::Phase::Disputed)
        || cursor.position > certified.position + 1
    {
        return Err(Error::Conflict);
    }
    let entry = target(db, row, cursor.position).await?;
    if entry.chain != cursor.chain {
        return Err(Error::Conflict);
    }
    if entry.certified.is_some_and(|accepted| accepted != proof)
        || ((!proof.accepted) && (cursor.position == 0 || entry.authority))
    {
        return dispute(db, id).await;
    }
    if cursor.position < certified.position {
        return Ok(());
    } // Full prefix independently replayed, only the current head needs a new vote.
    if cursor.position > certified.position && phase != rpc::Phase::Playing {
        return Err(Error::Conflict);
    }
    let encoded = String::from_utf8(central_api::encode(&proof)?).map_err(unavailable)?;
    if !vote(db, row, host, cursor, &encoded).await? {
        return dispute(db, id).await;
    }
    let Some(other) = peer(db, row, host, cursor).await? else {
        return Ok(());
    };
    if other != proof {
        return dispute(db, id).await;
    }
    if !routing::healthy(db, row).await? {
        return Ok(());
    }
    if entry.certified.is_some() && phase == rpc::Phase::Playing {
        return Ok(());
    }
    certify(db, row, cursor, proof, encoded).await
}
