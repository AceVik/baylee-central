//! Public projections and rankings; these queries never select a private package.

use super::{
    Challenge, Error, Response, api, attempt, challenge, now, statement, task, unavailable, uuid,
};
use sea_orm::{ConnectionTrait, QueryResult, prelude::Uuid};

pub(super) async fn catalogue(
    db: &impl ConnectionTrait,
    before: Option<i64>,
) -> Result<Response, Error> {
    let rows = db
        .query_all_raw(statement(
            "SELECT id,description::text,digest,withdrawn FROM central_identity.challenges
         WHERE scope=0 AND starts_at<=floor(extract(epoch FROM pg_catalog.clock_timestamp()))
         AND ($1::bigint IS NULL OR starts_at<$1) ORDER BY starts_at DESC,kind LIMIT 31",
            vec![before.into()],
        ))
        .await
        .map_err(unavailable)?;
    let mut challenges: Vec<Challenge> = rows.iter().map(challenge).collect::<Result<_, _>>()?;
    let next = trim_page(&mut challenges);
    Ok(Response::Catalogue {
        now: now(db).await?,
        challenges,
        before: next,
    })
}

/// Never split a daily/weekly pair sharing the same start across a time-only cursor.
fn trim_page(challenges: &mut Vec<Challenge>) -> Option<i64> {
    if challenges.len() <= api::PAGE_SIZE {
        return None;
    }
    let omitted = challenges.pop()?.description.period.starts_at;
    while challenges
        .last()
        .is_some_and(|c| c.description.period.starts_at == omitted)
    {
        challenges.pop();
    }
    challenges.last().map(|c| c.description.period.starts_at)
}

pub(super) async fn detail(
    db: &impl ConnectionTrait,
    id: [u8; 16],
    board: api::Board,
    offset: u32,
    me: Option<Uuid>,
) -> Result<Response, Error> {
    if offset > 3_000 {
        return Err(Error::Rejected);
    }
    let challenge = task(db, id, false).await?;
    let (score, condition) = match board {
        api::Board::FirstWin => ("failures,turn", "AND a.id=p.first_win"),
        api::Board::BestTurns => ("turn", ""),
        api::Board::Sprint => ("milliseconds", "AND a.milliseconds IS NOT NULL"),
    };
    // `score`/`condition` are closed enum choices, never request text.
    let rows = db.query_all_raw(statement(&format!(
        "WITH picked AS (
           SELECT a.*,row_number() OVER(PARTITION BY a.account_id ORDER BY a.{score},a.id) AS pick
           FROM central_identity.challenge_attempts a
           JOIN central_identity.challenges c ON c.id=a.challenge_id AND NOT c.withdrawn
           JOIN central_identity.challenge_participation p ON p.challenge_id=a.challenge_id AND p.account_id=a.account_id
           WHERE a.challenge_id=$1 AND a.state='won' AND a.live AND NOT a.assisted AND p.listed
           AND a.finished_at<to_timestamp(c.ends_at) {condition}
         ), ranked AS (
           SELECT *,dense_rank() OVER(ORDER BY {score}) AS rank FROM picked WHERE pick=1
         ) SELECT r.rank,COALESCE(a.display_name,'Player') AS name,a.tag,
           COALESCE(r.account_id=$2,false) AS own,r.failures,r.turn,r.milliseconds
           FROM ranked r JOIN central_identity.accounts a ON a.id=r.account_id
           ORDER BY {score},r.account_id LIMIT 30 OFFSET $3"
    ), vec![uuid(id).into(), me.into(), i64::from(offset).into()]))
        .await.map_err(unavailable)?;
    let standings = rows.iter().map(standing).collect::<Result<_, _>>()?;
    Ok(Response::Detail {
        now: now(db).await?,
        challenge,
        standings,
    })
}

fn standing(row: &QueryResult) -> Result<api::Standing, Error> {
    let rank: i64 = row.try_get("", "rank").map_err(unavailable)?;
    let tag: Option<i32> = row.try_get("", "tag").map_err(unavailable)?;
    let failures: i32 = row.try_get("", "failures").map_err(unavailable)?;
    let turn: i32 = row.try_get("", "turn").map_err(unavailable)?;
    let milliseconds: Option<i64> = row.try_get("", "milliseconds").map_err(unavailable)?;
    Ok(api::Standing {
        rank: u32::try_from(rank).map_err(unavailable)?,
        name: row.try_get("", "name").map_err(unavailable)?,
        tag: tag.map(u32::try_from).transpose().map_err(unavailable)?,
        own: row.try_get("", "own").map_err(unavailable)?,
        failures: u32::try_from(failures).map_err(unavailable)?,
        turn: u32::try_from(turn).map_err(unavailable)?,
        milliseconds: milliseconds
            .map(u64::try_from)
            .transpose()
            .map_err(unavailable)?,
    })
}

pub(super) async fn progress(
    db: &impl ConnectionTrait,
    me: Uuid,
    host: &str,
) -> Result<Response, Error> {
    let row = db.query_one_raw(statement(
        "WITH earned AS (SELECT * FROM central_identity.challenge_completions WHERE account_id=$1),
         weeks AS (SELECT live_week FROM earned WHERE live_week IS NOT NULL GROUP BY live_week
           HAVING bool_or(kind='weekly') OR count(*) FILTER(WHERE kind='daily')>=3)
         SELECT count(*) FILTER(WHERE kind='daily') AS daily,
           count(*) FILTER(WHERE kind='weekly') AS weekly,
           count(*) FILTER(WHERE tactician) AS tactician,
           count(*) FILTER(WHERE first_try) AS first_try,
           (SELECT count(*) FROM weeks) AS regular FROM earned",
        vec![me.into()],
    )).await.map_err(unavailable)?.ok_or(Error::Unavailable)?;
    let badges = api::BadgeKind::ALL
        .into_iter()
        .zip(["daily", "weekly", "tactician", "first_try", "regular"])
        .map(|(kind, column)| {
            let count: i64 = row.try_get("", column).map_err(unavailable)?;
            Ok(kind.progress(u32::try_from(count).map_err(unavailable)?))
        })
        .collect::<Result<_, Error>>()?;
    let rows = db
        .query_all_raw(statement(
            "SELECT * FROM central_identity.challenge_attempts WHERE account_id=$1
         AND kind!='tower' AND state IN ('reserved','playing') ORDER BY created_at,id LIMIT 2",
            vec![me.into()],
        ))
        .await
        .map_err(unavailable)?;
    let active = rows
        .iter()
        .map(|row| attempt(row, host))
        .collect::<Result<_, _>>()?;
    Ok(Response::Progress { badges, active })
}

#[cfg(test)]
mod tests {
    use super::*;
    use api::{Description, Kind, Period, Words};

    #[test]
    fn a_shared_midnight_is_never_lost_between_archive_pages() {
        let words = Words {
            en: "test".into(),
            de: "test".into(),
        };
        let mut page: Vec<_> = (0..31)
            .map(|index| Challenge {
                id: [1; 16],
                description: Description {
                    period: Period {
                        kind: Kind::Daily,
                        starts_at: 100 - index,
                        ends_at: 101 - index,
                    },
                    title: words.clone(),
                    instructions: words.clone(),
                    deck: "test".into(),
                    executor: "test".into(),
                    target_turn: 1,
                    expected_minutes: 1,
                },
                digest: [1; 32],
                withdrawn: false,
            })
            .collect();
        page[30].description.period.starts_at = page[29].description.period.starts_at;
        let omitted = page[30].description.period.starts_at;
        let cursor = trim_page(&mut page).unwrap();
        assert_eq!(page.len(), 29);
        assert!(
            omitted < cursor,
            "both omitted rows will match the next strict cursor"
        );
        assert!(
            page.iter()
                .all(|row| row.description.period.starts_at >= cursor)
        );
    }
}
