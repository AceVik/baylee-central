//! The first ranked read or settlement of a UTC day publishes the season's
//! titles. The write and every awarded badge share the surrounding transaction.

use super::super::{column, rules, statement, unavailable};
use central_api::Error;
use sea_orm::{ConnectionTrait, DatabaseTransaction, prelude::Uuid};

// A game assigned before close may finish in its pinned season. Another
// transaction still sees its open row until the scoring transaction commits.
// Abandoned rows cannot delay the archive past the longest possible lease
// (two minutes to claim and twelve hours to finish).
const TARGET: &str = "SELECT s.id,((LEAST(transaction_timestamp(),
    s.ends_at - interval '1 microsecond')) AT TIME ZONE 'UTC')::date AS publication_day,
    ($2::boolean AND s.ends_at<=transaction_timestamp() AND
     (s.ends_at<=transaction_timestamp()-interval '24 hours' OR NOT EXISTS (
        SELECT 1 FROM central_identity.ranked_matches m WHERE m.season_id=s.id
        AND m.state IN ('reserved','playing')))) IS TRUE AS final
    FROM central_identity.ranked_seasons s WHERE s.id=$1";

pub(in super::super) async fn publish(db: &DatabaseTransaction, season: Uuid) -> Result<(), Error> {
    ensure(db, season, false).await.map(drop)
}

pub(super) async fn visible(db: &DatabaseTransaction, season: Uuid) -> Result<bool, Error> {
    ensure(db, season, true).await
}

async fn ensure(db: &DatabaseTransaction, season: Uuid, reading: bool) -> Result<bool, Error> {
    // The normal hot path is a read. Competing first callers must recheck the
    // date after acquiring the same publication row, before replacing any title.
    let current = db
        .query_one_raw(statement(
            &format!(
                "SELECT COALESCE(p.finalized OR (p.published_on>=t.publication_day AND NOT t.final),false) AS current,
                 COALESCE(p.finalized,false) AS finalized FROM ({TARGET}) t
                 LEFT JOIN central_identity.ranked_publications p ON p.season_id=t.id"
            ),
            vec![season.into(),reading.into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Unavailable)?;
    if column::<bool>(&current, "current")? {
        return column(&current, "finalized");
    }
    let changed = db
        .query_one_raw(statement(
            &format!(
                "INSERT INTO central_identity.ranked_publications AS p (season_id,published_on,finalized)
            SELECT t.id,t.publication_day,t.final FROM ({TARGET}) t
            ON CONFLICT(season_id) DO UPDATE SET published_on=EXCLUDED.published_on,finalized=EXCLUDED.finalized
            WHERE NOT p.finalized AND (p.published_on < EXCLUDED.published_on OR EXCLUDED.finalized)
            RETURNING finalized"
            ),
            vec![season.into(),reading.into()],
        ))
        .await
        .map_err(unavailable)?;
    let Some(changed) = changed else {
        let winner = db
            .query_one_raw(statement(
                "SELECT finalized FROM central_identity.ranked_publications WHERE season_id=$1",
                vec![season.into()],
            ))
            .await
            .map_err(unavailable)?
            .ok_or(Error::Unavailable)?;
        return column(&winner, "finalized");
    };
    publish_holders(db, season).await?;
    column(&changed, "finalized")
}

async fn publish_holders(db: &DatabaseTransaction, season: Uuid) -> Result<(), Error> {
    db.execute_raw(statement(
        "DELETE FROM central_identity.ranked_champions WHERE season_id=$1",
        vec![season.into()],
    ))
    .await
    .map_err(unavailable)?;
    db.execute_raw(statement(
        "WITH cutoff AS (
            SELECT CASE WHEN p.finalized THEN s.ends_at ELSE
                GREATEST(s.starts_at,p.published_on::timestamp AT TIME ZONE 'UTC') END AS instant
            FROM central_identity.ranked_publications p JOIN central_identity.ranked_seasons s
            ON s.id=p.season_id WHERE s.id=$1
        ), candidates AS (
            SELECT r.account_id,'player'::text AS track,r.format,NULL::bytea AS version,r.elo
            FROM central_identity.ranked_ratings r CROSS JOIN cutoff
            WHERE r.season_id=$1 AND r.elo >= $2 AND r.games >= $3
            AND r.first_at <= instant - $4::integer * interval '24 hours'
            AND r.last_at > instant - ($5::integer + 1) * interval '24 hours'
            UNION ALL
            SELECT d.account_id,'deck',d.format,d.version,d.elo
            FROM central_identity.ranked_deck_ratings d CROSS JOIN cutoff
            WHERE d.season_id=$1 AND d.elo >= $2 AND d.games >= $3
            AND d.first_at <= instant - $4::integer * interval '24 hours'
            AND d.last_at > instant - ($5::integer + 1) * interval '24 hours'
        ), track_best AS MATERIALIZED (
            SELECT DISTINCT ON (account_id,track) * FROM candidates
            ORDER BY account_id,track,elo DESC,format,version
        ), top_people AS (
            SELECT account_id,max(elo) AS elo FROM track_best GROUP BY account_id
            ORDER BY max(elo) DESC,account_id LIMIT $6::integer
        ), holders AS (
            SELECT account_id,row_number() OVER (ORDER BY elo DESC,account_id)::smallint AS place
            FROM top_people
        )
        INSERT INTO central_identity.ranked_champions
            (season_id,account_id,player_format,deck_format,version,place)
        SELECT $1,h.account_id,p.format,d.format,d.version,h.place FROM holders h
        LEFT JOIN track_best p ON p.account_id=h.account_id AND p.track='player'
        LEFT JOIN track_best d ON d.account_id=h.account_id AND d.track='deck'",
        vec![
            season.into(),
            rules::CHAMPION_ELO.into(),
            rules::PLACED_AFTER.into(),
            rules::ELITE_AGE_DAYS.into(),
            rules::ELITE_IDLE_DAYS.into(),
            rules::CHAMPION_LIMIT.into(),
        ],
    ))
    .await
    .map_err(unavailable)?;
    db.execute_raw(statement(
        "INSERT INTO central_identity.ranked_badge_peaks AS p
            (account_id,season_id,track,format,league,division,place)
         SELECT c.account_id,c.season_id,t.track,t.format,10,0,c.place
         FROM central_identity.ranked_champions c
         CROSS JOIN LATERAL (VALUES ('player',c.player_format),('deck',c.deck_format)) t(track,format)
         WHERE c.season_id=$1 AND t.format IS NOT NULL
         ON CONFLICT(account_id,season_id,track,format,league,division,podium)
         DO UPDATE SET place=EXCLUDED.place WHERE EXCLUDED.place<p.place",
        vec![season.into()],
    ))
    .await
    .map_err(unavailable)?;
    Ok(())
}

pub(in super::super) async fn champion_place(
    db: &DatabaseTransaction,
    season: Uuid,
    format: &str,
    account: Uuid,
) -> Result<Option<u8>, Error> {
    place(db, season, format, account, None).await
}

pub(in super::super) async fn deck_champion_place(
    db: &DatabaseTransaction,
    season: Uuid,
    format: &str,
    account: Uuid,
    version: &[u8],
) -> Result<Option<u8>, Error> {
    place(db, season, format, account, Some(version)).await
}

async fn place(
    db: &DatabaseTransaction,
    season: Uuid,
    format: &str,
    account: Uuid,
    version: Option<&[u8]>,
) -> Result<Option<u8>, Error> {
    let row = db
        .query_one_raw(statement(
            "SELECT place FROM central_identity.ranked_champions
         WHERE season_id=$1 AND account_id=$2 AND
         (($4::bytea IS NULL AND player_format=$3) OR (deck_format=$3 AND version=$4))",
            vec![
                season.into(),
                account.into(),
                format.into(),
                version.map(<[u8]>::to_vec).into(),
            ],
        ))
        .await
        .map_err(unavailable)?;
    row.map(|r| u8::try_from(column::<i16>(&r, "place")?).map_err(unavailable))
        .transpose()
}
