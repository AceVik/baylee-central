//! Per-account tickets. Only the immutable request id can cancel a waiting seat.

use super::{Error, api, column, matchmaking, scoring, statement, unavailable, uuid};
use sea_orm::{ConnectionTrait, DatabaseTransaction, QueryResult, prelude::Uuid};

const FIELDS: &str = "request_id,account_id,state,season_id,format,commitment,queue_elo,match_id,
    floor(extract(epoch FROM created_at))::bigint AS at";

async fn ticket(
    db: &DatabaseTransaction,
    account: Uuid,
    key: Option<Uuid>,
) -> Result<Option<QueryResult>, Error> {
    db.query_one_raw(statement(
        &format!(
            "SELECT {FIELDS} FROM central_identity.ranked_tickets WHERE account_id=$1
        AND ($2::uuid IS NULL OR request_id=$2) ORDER BY created_at DESC,request_id DESC LIMIT 1"
        ),
        vec![account.into(), key.into()],
    ))
    .await
    .map_err(unavailable)
}

async fn retire(db: &DatabaseTransaction, account: Uuid) -> Result<(), Error> {
    // Resolve a timed-out reservation, deleted gateway, removed permission or vanished host.
    // A later Ready must not resurrect a match that expired during an outage.
    super::host::expire(db, account).await?;
    db.execute_raw(statement(
        "UPDATE central_identity.ranked_tickets t SET state='closed' WHERE t.account_id=$1 AND
         ((t.state='waiting' AND (t.expires_at<=pg_catalog.clock_timestamp() OR NOT EXISTS
           (SELECT 1 FROM central_identity.ranked_seasons s WHERE s.id=t.season_id AND
            (s.ends_at IS NULL OR s.ends_at>pg_catalog.clock_timestamp()))))
          OR (t.state='assigned' AND NOT EXISTS (SELECT 1 FROM central_identity.ranked_matches m
            WHERE m.id=t.match_id AND m.state IN ('reserved','playing'))))",
        vec![account.into()],
    ))
    .await
    .map_err(unavailable)?;
    Ok(())
}

async fn join(
    db: &DatabaseTransaction,
    me: Uuid,
    key: Uuid,
    format: api::Format,
    deck: &[u8; 32],
) -> Result<(), Error> {
    if let Some(existing) = ticket(db, me, Some(key)).await? {
        return if column::<String>(&existing, "format")? == format.as_str()
            && column::<Vec<u8>>(&existing, "commitment")? == deck
        {
            Ok(())
        } else {
            Err(Error::Conflict)
        };
    }
    let admitted = db.execute_raw(statement(
        "INSERT INTO central_identity.ranked_tickets(account_id,request_id,season_id,format,commitment,queue_elo)
         SELECT $1,$2,s.id,$3,$4,COALESCE((SELECT r.queue_elo FROM central_identity.ranked_ratings r
          WHERE r.account_id=$1 AND r.format=$3 ORDER BY r.last_at DESC LIMIT 1),1500)
         FROM central_identity.ranked_seasons s WHERE s.starts_at<=pg_catalog.clock_timestamp()
          AND (s.ends_at IS NULL OR s.ends_at>pg_catalog.clock_timestamp())
          AND (SELECT count(*) FROM (SELECT 1 FROM central_identity.ranked_tickets
             WHERE account_id=$1 AND created_at>now()-interval '1 hour' LIMIT 24) recent)<24
         ON CONFLICT DO NOTHING",
        vec![me.into(),key.into(),format.as_str().into(),deck.to_vec().into()]
    )).await.map_err(unavailable)?;
    if admitted.rows_affected() == 0 {
        return match ticket(db, me, None).await? {
            Some(row)
                if column::<Uuid>(&row, "request_id")? == key
                    && column::<String>(&row, "format")? == format.as_str()
                    && column::<Vec<u8>>(&row, "commitment")? == deck =>
            {
                Ok(())
            }
            Some(row) if column::<String>(&row, "state")? != "closed" => Err(Error::Conflict),
            _ => Err(Error::Busy),
        };
    }
    Ok(())
}

pub(super) async fn handle(
    db: &DatabaseTransaction,
    me: Uuid,
    request: &api::Request,
) -> Result<api::QueueState, Error> {
    let key = match request {
        api::Request::Join { key, .. } | api::Request::Cancel { key } => {
            if !super::super::gateways::valid_id(*key) {
                return Err(Error::Rejected);
            }
            Some(uuid(*key))
        }
        api::Request::Status => None,
        _ => return Err(Error::Rejected),
    };
    retire(db, me).await?;
    match request {
        api::Request::Join { key, format, deck } => join(db, me, uuid(*key), *format, deck).await?,
        api::Request::Cancel { key } => {
            db.execute_raw(statement(
                "UPDATE central_identity.ranked_tickets SET state='closed'
                 WHERE account_id=$1 AND request_id=$2 AND state='waiting'",
                vec![me.into(), uuid(*key).into()],
            ))
            .await
            .map_err(unavailable)?;
            // A player can decline the selected host's terms before their own seat is
            // claimed. Start and claim serialize on this same match row. A delayed
            // cancellation for an older request cannot release a newer reservation.
            db.execute_raw(statement(
                "UPDATE central_identity.ranked_matches m SET state='void',finished_at=pg_catalog.clock_timestamp()
                 FROM central_identity.ranked_tickets t WHERE t.account_id=$1 AND t.request_id=$2
                 AND t.match_id=m.id AND t.state='assigned' AND m.state='reserved'
                 AND ((m.account_a=$1 AND NOT m.claimed_a) OR (m.account_b=$1 AND NOT m.claimed_b))",
                vec![me.into(),uuid(*key).into()],
            )).await.map_err(unavailable)?;
        }
        _ => {}
    }
    let Some(row) = ticket(db, me, key).await? else {
        return Ok(api::QueueState::Idle);
    };
    if column::<String>(&row, "state")? != "waiting" {
        return render(db, me, &row).await;
    }
    matchmaking::pair(db, &row).await?;
    let row = ticket(db, me, key).await?.ok_or(Error::Unavailable)?;
    render(db, me, &row).await
}

async fn render(
    db: &DatabaseTransaction,
    me: Uuid,
    row: &QueryResult,
) -> Result<api::QueueState, Error> {
    let key = column::<Uuid>(row, "request_id")?.into_bytes();
    let format = column::<String>(row, "format")?
        .parse()
        .map_err(unavailable)?;
    if column::<String>(row, "state")? == "waiting" {
        return Ok(api::QueueState::Waiting {
            key,
            format,
            queued_at: column(row, "at")?,
        });
    }
    let Some(id) = column::<Option<Uuid>>(row, "match_id")? else {
        return Ok(api::QueueState::Idle);
    };
    let row=db.query_one_raw(statement(
        "SELECT state,origin,host_id,account_a,commitment_a,commitment_b,game_id,outcome FROM central_identity.ranked_matches WHERE id=$1",
        vec![id.into()]
    )).await.map_err(unavailable)?.ok_or(Error::Unavailable)?;
    let seat = u8::from(column::<Uuid>(&row, "account_a")? != me);
    match column::<String>(&row, "state")?.as_str() {
        "reserved" | "playing" => Ok(api::QueueState::Assigned {
            key,
            assignment: api::Assignment {
                match_id: id.into_bytes(),
                host_id: column::<Uuid>(&row, "host_id")?.into_bytes(),
                format,
                origin: column(&row, "origin")?,
                account: me.into_bytes(),
                seat,
                deck: column::<Vec<u8>>(
                    &row,
                    if seat == 0 {
                        "commitment_a"
                    } else {
                        "commitment_b"
                    },
                )?
                .try_into()
                .map_err(unavailable)?,
                game: column::<Option<Uuid>>(&row, "game_id")?.map(Uuid::into_bytes),
            },
        }),
        "finished" => {
            let (elo, delta, before, after, deck) = scoring::receipt(db, id, me).await?;
            Ok(api::QueueState::Finished {
                key,
                match_id: id.into_bytes(),
                format,
                seat,
                outcome: super::host::outcome(column(&row, "outcome")?)?,
                elo,
                delta,
                before,
                after,
                deck,
            })
        }
        "void" => Ok(api::QueueState::Idle),
        _ => Err(Error::Unavailable),
    }
}
