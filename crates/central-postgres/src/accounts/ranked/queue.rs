//! Per-account tickets. Only the immutable request id can cancel a waiting seat.

use super::{Error, api, column, matchmaking, scoring, statement, unavailable, uuid};
use sea_orm::{ConnectionTrait, DatabaseTransaction, QueryResult, prelude::Uuid};

/// Hours a closed ticket is kept after its waiting ended, or after its
/// match ended, before the operator's daily sweep deletes it
/// (`docs/ranked.md` §"Publication and storage"). What still reads one:
/// the hourly admission cap counts the account's tickets of the last hour,
/// the sparse-queue check those of the last two minutes, a `Join` retried
/// after a lost answer finds its own key instead of queueing anew, and
/// `Status` shows the last match's result from it. A day covers each with a
/// wide margin. Nothing settles from a closed ticket: a match's consent check
/// reads its tickets only while they are assigned, and rematch spacing reads
/// `ranked_matches`. Once the ticket is gone, `Status` says `Idle` (the
/// receipt stays) and a retry of that key is a new request.
pub const CLOSED_TICKET_HOURS: i32 = 24;
/// Rows one delete takes; the sweep repeats it until a batch comes back short.
const SWEEP_BATCH: i64 = 4096;

/// Delete closed tickets older than [`CLOSED_TICKET_HOURS`], in bounded
/// batches, under the operator's login (the runtime cannot delete one).
pub(in crate::accounts) async fn sweep(db: &DatabaseTransaction) -> Result<u64, Error> {
    let mut deleted = 0;
    loop {
        let batch = db
            .execute_raw(statement(
                "DELETE FROM central_identity.ranked_tickets t WHERE (t.account_id,t.request_id) IN (
                   SELECT o.account_id,o.request_id FROM central_identity.ranked_tickets o
                   WHERE o.state='closed'
                     AND o.expires_at<pg_catalog.clock_timestamp()-pg_catalog.make_interval(hours => $1)
                     AND NOT EXISTS (SELECT 1 FROM central_identity.ranked_matches m WHERE m.id=o.match_id
                       AND (m.finished_at IS NULL
                         OR m.finished_at>=pg_catalog.clock_timestamp()-pg_catalog.make_interval(hours => $1)))
                   LIMIT $2 FOR UPDATE OF o SKIP LOCKED)",
                vec![CLOSED_TICKET_HOURS.into(), SWEEP_BATCH.into()],
            ))
            .await
            .map_err(unavailable)?
            .rows_affected();
        deleted += batch;
        if batch < u64::try_from(SWEEP_BATCH).unwrap_or(u64::MAX) {
            return Ok(deleted);
        }
    }
}

const FIELDS: &str =
    "request_id,account_id,state,replicable,season_id,format,commitment,queue_elo,match_id,
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
    replicable: bool,
) -> Result<(), Error> {
    if let Some(existing) = ticket(db, me, Some(key)).await? {
        return if column::<bool>(&existing, "replicable")? == replicable
            && column::<String>(&existing, "format")? == format.as_str()
            && column::<Vec<u8>>(&existing, "commitment")? == deck
        {
            Ok(())
        } else {
            Err(Error::Conflict)
        };
    }
    let admitted = db.execute_raw(statement(
        "INSERT INTO central_identity.ranked_tickets(account_id,request_id,season_id,format,commitment,queue_elo,replicable)
         SELECT $1,$2,s.id,$3,$4,COALESCE((SELECT r.queue_elo FROM central_identity.ranked_ratings r
          WHERE r.account_id=$1 AND r.format=$3 ORDER BY r.last_at DESC LIMIT 1),1500),$5
         FROM central_identity.ranked_seasons s WHERE s.starts_at<=pg_catalog.clock_timestamp()
          AND (s.ends_at IS NULL OR s.ends_at>pg_catalog.clock_timestamp())
          AND (SELECT count(*) FROM (SELECT 1 FROM central_identity.ranked_tickets
             WHERE account_id=$1 AND created_at>now()-interval '1 hour' LIMIT 24) recent)<24
         ON CONFLICT DO NOTHING",
        vec![me.into(),key.into(),format.as_str().into(),deck.to_vec().into(),replicable.into()]
    )).await.map_err(unavailable)?;
    if admitted.rows_affected() == 0 {
        return match ticket(db, me, None).await? {
            Some(row)
                if column::<bool>(&row, "replicable")? == replicable
                    && column::<Uuid>(&row, "request_id")? == key
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
    replicable: bool,
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
    if let api::Request::Join { key, format, deck } = request
        && !replicable
    {
        let old = ticket(db, me, Some(uuid(*key)))
            .await?
            .ok_or(Error::UpdateRequired)?;
        if column::<bool>(&old, "replicable")?
            || column::<String>(&old, "format")? != format.as_str()
            || column::<Vec<u8>>(&old, "commitment")? != *deck
        {
            return Err(Error::UpdateRequired);
        }
        return render(db, me, &old).await;
    }
    retire(db, me).await?;
    match request {
        api::Request::Join { key, format, deck } => {
            join(db, me, uuid(*key), *format, deck, replicable).await?;
        }
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
            if let Some(closed)=db.query_one_raw(statement(
                "UPDATE central_identity.ranked_matches m SET state='void',finished_at=pg_catalog.clock_timestamp()
                 FROM central_identity.ranked_tickets t WHERE t.account_id=$1 AND t.request_id=$2
                 AND t.match_id=m.id AND t.state='assigned' AND m.state='reserved'
                 AND ((m.account_a=$1 AND NOT m.claimed_a) OR (m.account_b=$1 AND NOT m.claimed_b)) RETURNING m.id",
                vec![me.into(),uuid(*key).into()],
            )).await.map_err(unavailable)? {
                let id=column(&closed,"id")?;
                if super::replication::protected(db,id).await? { super::replication::void(db,id).await?; }
            }
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
