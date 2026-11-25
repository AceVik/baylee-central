//! Same expanding gap windows, six-hour rematch cost and newcomer protection as the league lab.
//! The oldest and the nearest neighbours are both examined, even in a very large queue.

use super::{Error, column, statement, unavailable};
use sea_orm::{ConnectionTrait, DatabaseTransaction, QueryResult, prelude::Uuid};

#[derive(Clone, Copy)]
struct Seat {
    id: Uuid,
    at: i64,
    elo: i32,
}

fn seat(row: &QueryResult) -> Result<Seat, Error> {
    Ok(Seat {
        id: column(row, "account_id")?,
        at: column(row, "at")?,
        elo: column(row, "queue_elo")?,
    })
}

fn preference(first: Seat, second: Seat, now: i64, sparse: bool, repeats: i32) -> Option<i64> {
    let (older, younger) = if (first.at, first.id) <= (second.at, second.id) {
        (first, second)
    } else {
        (second, first)
    };
    let age = now.saturating_sub(older.at).max(0);
    let other_age = now.saturating_sub(younger.at).max(0);
    let gates = if sparse {
        [10, 20, 35, 60]
    } else {
        [20, 45, 75, 120]
    };
    let phase = (older.at.div_euclid(5) + i64::from(older.id.as_bytes()[15])).rem_euclid(3);
    let variety = !sparse && phase == 0;
    let radius = [100, 200, 350, 600, i64::MAX][gates.partition_point(|t| *t <= age)]
        .max(if variety { 300 } else { 100 });
    let gap = (i64::from(first.elo) - i64::from(second.elo)).abs();
    if gap > radius || (gap > 350 && other_age < 10 && age < 120) {
        return None;
    }
    Some(
        (gap - if variety { 300 } else { 80 }).max(0) + 140 * i64::from(repeats.min(3))
            - other_age.min(90) / 2,
    )
}

async fn candidate(
    db: &DatabaseTransaction,
    mine: Seat,
    season: Uuid,
    format: &str,
    replicated: bool,
) -> Result<Option<(Uuid, Uuid)>, Error> {
    let rows = db
        .query_all_raw(statement(
            "WITH oldest AS (
            SELECT account_id FROM central_identity.ranked_tickets WHERE season_id=$2 AND format=$3 AND replicable=$5
             AND state='waiting' AND expires_at>now() AND account_id<>$1
             ORDER BY created_at,account_id LIMIT 64
         ), lower AS (
            SELECT account_id FROM central_identity.ranked_tickets WHERE season_id=$2 AND format=$3 AND replicable=$5
             AND state='waiting' AND expires_at>now() AND account_id<>$1 AND queue_elo<=$4
             ORDER BY queue_elo DESC LIMIT 16
         ), upper_pool AS (
            SELECT account_id FROM central_identity.ranked_tickets WHERE season_id=$2 AND format=$3 AND replicable=$5
             AND state='waiting' AND expires_at>now() AND account_id<>$1 AND queue_elo>$4
             ORDER BY queue_elo LIMIT 16
         ), recent AS (
            SELECT count(*)<=6 AS sparse FROM (SELECT 1 FROM central_identity.ranked_tickets
              WHERE season_id=$2 AND format=$3 AND replicable=$5 AND created_at>now()-interval '2 minutes' LIMIT 7) a
         )
         SELECT t.account_id,t.request_id,t.queue_elo,floor(extract(epoch FROM t.created_at))::bigint AS at,
          floor(extract(epoch FROM now()))::bigint AS clock,recent.sparse,
          (SELECT count(*)::integer FROM (SELECT 1 FROM central_identity.ranked_matches m
           WHERE LEAST(m.account_a,m.account_b)=LEAST(t.account_id,$1)
           AND GREATEST(m.account_a,m.account_b)=GREATEST(t.account_id,$1)
           AND m.state='finished' AND m.finished_at>now()-interval '6 hours' LIMIT 3) p) AS repeats
         FROM (SELECT * FROM oldest UNION SELECT * FROM lower UNION SELECT * FROM upper_pool) c
         JOIN central_identity.ranked_tickets t ON t.account_id=c.account_id AND t.state='waiting'
         CROSS JOIN recent",
            vec![
                mine.id.into(),
                season.into(),
                format.into(),
                mine.elo.into(),
                replicated.into(),
            ],
        ))
        .await
        .map_err(unavailable)?;
    let mut best = None;
    for row in rows {
        let other = seat(&row)?;
        if let Some(cost) = preference(
            mine,
            other,
            column(&row, "clock")?,
            column(&row, "sparse")?,
            column(&row, "repeats")?,
        ) {
            let priority = (cost, other.at, other.id, column(&row, "request_id")?);
            if best.is_none_or(|prior| priority < prior) {
                best = Some(priority);
            }
        }
    }
    Ok(best.map(|(_, _, id, key)| (id, key)))
}

/// Locks both still-live tickets in account order. Simultaneous status polls do not
/// take one seat apiece while waiting to acquire the other.
pub(super) async fn pair(db: &DatabaseTransaction, ticket: &QueryResult) -> Result<(), Error> {
    let season: Uuid = column(ticket, "season_id")?;
    let format: String = column(ticket, "format")?;
    let mine = seat(ticket)?;
    let replicated: bool = column(ticket, "replicable")?;
    let Some((other, other_key)) = candidate(db, mine, season, &format, replicated).await? else {
        return Ok(());
    };
    let key: Uuid = column(ticket, "request_id")?;
    let rows = db.query_all_raw(statement(
        "SELECT account_id,commitment FROM central_identity.ranked_tickets WHERE
           ((account_id=$1 AND request_id=$2) OR (account_id=$3 AND request_id=$6)) AND season_id=$4 AND format=$5
          AND state='waiting' AND replicable=$7 AND expires_at>pg_catalog.clock_timestamp()
         ORDER BY account_id FOR UPDATE SKIP LOCKED",
        vec![mine.id.into(),key.into(),other.into(),season.into(),format.clone().into(),other_key.into(),replicated.into()]
    )).await.map_err(unavailable)?;
    let [low, high] = rows.as_slice() else {
        return Ok(());
    };
    let Some(allocation) = super::host::allocate(db, replicated).await? else {
        return Ok(());
    };
    let (mine_locked, other_locked) = if mine.id < other {
        (low, high)
    } else {
        (high, low)
    };
    let first: Vec<u8> = column(mine_locked, "commitment")?;
    let second: Vec<u8> = column(other_locked, "commitment")?;
    let host = allocation.primary;
    let matched = db.query_one_raw(statement(
        "INSERT INTO central_identity.ranked_matches(season_id,format,account_a,account_b,commitment_a,commitment_b,host_id,ranked_epoch,origin)
         SELECT s.id,$2,$3,$4,$5,$6,$7,$8,$9 FROM central_identity.ranked_seasons s WHERE s.id=$1
         AND s.starts_at<=pg_catalog.clock_timestamp() AND (s.ends_at IS NULL OR s.ends_at>pg_catalog.clock_timestamp())
         RETURNING id",
        vec![season.into(),format.into(),mine.id.into(),other.into(),first.into(),second.into(),host.id.into(),host.epoch.into(),host.origin.into()]
    )).await.map_err(unavailable)?;
    if let Some(row) = matched {
        let matched: Uuid = column(&row, "id")?;
        if let Some((standby, compatibility)) = allocation.replica {
            super::replication::routing::assign(db, matched, &standby, &compatibility).await?;
        }
        let changed = db.execute_raw(statement(
            "UPDATE central_identity.ranked_tickets SET state='assigned',match_id=$1 WHERE account_id IN ($2,$3) AND state='waiting'",
            vec![matched.into(),mine.id.into(),other.into()]
        )).await.map_err(unavailable)?;
        if changed.rows_affected() != 2 {
            return Err(Error::Conflict);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn player(at: i64, elo: i32, id: u128) -> Seat {
        Seat {
            id: Uuid::from_u128(id),
            at,
            elo,
        }
    }
    #[test]
    fn sparse_fallback_protects_new_arrival_then_allows_game() {
        let old = player(0, 2200, 1);
        let new = player(55, 1200, 2);
        assert_eq!(preference(old, new, 60, true, 0), None);
        assert!(preference(new, old, 65, true, 0).is_some());
        assert!(preference(old, player(120, 1000, 3), 120, true, 0).is_some());
    }
    #[test]
    fn equally_suitable_new_opponent_precedes_repeat() {
        let first = player(0, 1500, 1);
        assert!(
            preference(first, player(0, 1540, 3), 0, true, 0)
                < preference(first, player(0, 1500, 2), 0, true, 1)
        );
    }

    #[test]
    fn a_pair_is_judged_the_same_from_either_side_and_never_overflows() {
        let seats = [
            player(0, 1500, 1),
            player(0, 1650, 3),
            player(7, 1400, 2),
            player(50, 2600, 4),
            player(i64::MIN, i32::MIN, 5),
            player(i64::MAX, i32::MAX, 6),
            player(-3, 0, u128::MAX),
        ];
        for a in seats {
            for b in seats {
                for now in [i64::MIN, 0, 60, 120, i64::MAX] {
                    for sparse in [false, true] {
                        for repeats in [0, 3, i32::MAX] {
                            assert_eq!(
                                preference(a, b, now, sparse, repeats),
                                preference(b, a, now, sparse, repeats),
                                "{now} {sparse} {repeats}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn waiting_only_ever_widens_the_search_and_ends_it_open() {
        for gap in [0, 100, 101, 200, 350, 351, 600, 601, 3000] {
            for sparse in [false, true] {
                let pair =
                    |now| preference(player(0, 0, 1), player(0, gap, 2), now, sparse, 0).is_some();
                let mut open = false;
                for now in 0..=120 {
                    let found = pair(now);
                    assert!(found || !open, "{gap} closes again at {now}");
                    open = found;
                }
                assert!(open, "{gap} never plays");
            }
        }
        // A wide gap takes nobody who has just arrived, until the older has waited long.
        let older = player(0, 0, 1);
        assert_eq!(preference(older, player(115, 500, 2), 119, false, 0), None);
        assert!(preference(older, player(115, 500, 2), 120, false, 0).is_some());
    }
}
