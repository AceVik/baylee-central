//! The first ranked read or settlement of a UTC day publishes the season's
//! titles. The write and every awarded badge share the surrounding transaction.

use super::super::{column, rules, statement, unavailable};
use central_api::Error;
use sea_orm::{ConnectionTrait, DatabaseTransaction, prelude::Uuid};

const PUBLICATION_DAY: &str = "((LEAST(transaction_timestamp(),
    s.ends_at - interval '1 microsecond')) AT TIME ZONE 'UTC')::date";

pub(in super::super) async fn publish(db: &DatabaseTransaction, season: Uuid) -> Result<(), Error> {
    // The normal hot path is a read. Competing first callers must recheck the
    // date after acquiring the same publication row, before replacing any title.
    let current = db
        .query_one_raw(statement(
            &format!(
                "SELECT EXISTS (SELECT 1 FROM central_identity.ranked_publications p
         JOIN central_identity.ranked_seasons s ON s.id=p.season_id
         WHERE p.season_id=$1 AND p.published_on >= {PUBLICATION_DAY}) AS current"
            ),
            vec![season.into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Unavailable)?;
    if column::<bool>(&current, "current")? {
        return Ok(());
    }
    let changed = db
        .query_one_raw(statement(
            &format!(
                "INSERT INTO central_identity.ranked_publications AS p (season_id,published_on)
            SELECT s.id,{PUBLICATION_DAY} FROM central_identity.ranked_seasons s WHERE s.id=$1
            ON CONFLICT(season_id) DO UPDATE SET published_on=EXCLUDED.published_on
            WHERE p.published_on < EXCLUDED.published_on RETURNING season_id"
            ),
            vec![season.into()],
        ))
        .await
        .map_err(unavailable)?;
    if changed.is_none() {
        return Ok(());
    }
    db.execute_raw(statement(
        "DELETE FROM central_identity.ranked_champions WHERE season_id=$1",
        vec![season.into()],
    ))
    .await
    .map_err(unavailable)?;
    db.execute_raw(statement(
        "WITH cutoff AS (
            SELECT GREATEST(s.starts_at,p.published_on::timestamp AT TIME ZONE 'UTC') AS instant
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
            rules::CHAMPION_GAMES.into(),
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
