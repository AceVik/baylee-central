//! AI against AI (`docs/ai-players.md` section 7): a runner asks for a match
//! between its AI and another AI that offers a deck for the same format.
//! Central makes one only while no person's ticket waits in that format and
//! the carousel's caps, both AIs' own caps, `global_seats` and the hosts'
//! reserve have room. It takes fill-in's locks in fill-in's order: the
//! settings, then the AIs' rows (here both, in account order), then the
//! host by `allocate_single`'s own locks.
use super::fill_in::{self, Arranged, OFFERED, TRIES, policy};
use super::replication::routing::{allocate_single, room_single};
use super::{Error, column, statement, unavailable};
use crate::accounts::ai::{self, LIVE, TODAY};
use central_api::admin::AiSettings;
use central_api::ranked::{Format, ai::Unseated};
use sea_orm::{ConnectionTrait, DatabaseTransaction, prelude::Uuid};

mod reason;
use reason::Facts;

/// A carousel match always takes a host that offers a compatibility
/// digest: both seats are runners, which play as the live client does, and
/// fill-in takes such a host for the live client's searches.
const COMPATIBLE: bool = true;

/// What the carousel counts for the AI asking (`$1`) in format `$2`, with
/// `$3` the drain minutes its offer's session must have left.
fn counted_sql() -> String {
    format!(
        "WITH season AS (SELECT s.id FROM central_identity.ranked_seasons s
           WHERE s.starts_at <= pg_catalog.clock_timestamp()
             AND (s.ends_at IS NULL OR s.ends_at > pg_catalog.clock_timestamp())
           ORDER BY s.starts_at DESC LIMIT 1)
         SELECT p.enabled, p.max_games, p.daily_games, {OFFERED} AS offered,
           (SELECT count(*) FROM central_identity.ranked_matches m
             WHERE p.account_id IN (m.account_a, m.account_b) AND {LIVE}) AS live,
           (SELECT count(*) FROM central_identity.ai_matches am
             JOIN central_identity.ranked_matches m ON m.id = am.match_id
             WHERE p.account_id IN (m.account_a, m.account_b) AND am.created_at >= {TODAY}) AS today,
           COALESCE((SELECT r.queue_elo FROM central_identity.ranked_ratings r
             WHERE r.account_id = p.account_id AND r.format = $2
             ORDER BY r.last_at DESC LIMIT 1), 1500) AS elo,
           (SELECT count(*) FROM central_identity.ai_matches am
             JOIN central_identity.ranked_matches m ON m.id = am.match_id
             WHERE am.arranged = 'carousel' AND {LIVE}) AS carousel_live,
           (SELECT count(*) FROM central_identity.ai_matches am
             WHERE am.arranged = 'carousel' AND am.created_at >= {TODAY}) AS carousel_today,
           (SELECT id FROM season) AS season,
           EXISTS (SELECT 1 FROM central_identity.ranked_tickets t
             WHERE t.season_id = (SELECT id FROM season) AND t.format = $2 AND t.state = 'waiting'
               AND t.expires_at > pg_catalog.clock_timestamp()
               AND NOT EXISTS (SELECT 1 FROM central_identity.ai_players q
                 WHERE q.account_id = t.account_id)) AS people
         FROM central_identity.ai_players p WHERE p.account_id = $1"
    )
}

/// What one count found: the facts the rules decide on and the asking AI's
/// queue Elo in the format.
struct Counted {
    facts: Facts,
    elo: i32,
}

/// Count everything the rules ask about, in fresh statements.
async fn count(
    db: &DatabaseTransaction,
    me: Uuid,
    format: &str,
    settings: &AiSettings,
) -> Result<Counted, Error> {
    let row = db
        .query_one_raw(statement(
            &counted_sql(),
            vec![
                me.into(),
                format.into(),
                i32::from(settings.drain_minutes).into(),
            ],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Rejected)?;
    Ok(Counted {
        facts: Facts {
            season: column(&row, "season")?,
            carousel_games: settings.carousel_games,
            carousel_daily: settings.carousel_daily,
            global_seats: settings.global_seats,
            enabled: column(&row, "enabled")?,
            offered: column(&row, "offered")?,
            live: column(&row, "live")?,
            max_games: column(&row, "max_games")?,
            today: column(&row, "today")?,
            daily_games: column(&row, "daily_games")?,
            carousel_live: column(&row, "carousel_live")?,
            carousel_today: column(&row, "carousel_today")?,
            seats: fill_in::seats_live(db).await?,
            people: column(&row, "people")?,
        },
        elo: column(&row, "elo")?,
    })
}

/// Lock the two AIs' rows in account order, skipping a row the switch, an
/// offer or another transaction holds, and only while each is switched on.
/// Answers the accounts locked.
async fn lock(db: &DatabaseTransaction, pair: [Uuid; 2]) -> Result<Vec<Uuid>, Error> {
    db.query_all_raw(statement(
        "SELECT account_id FROM central_identity.ai_players
         WHERE account_id IN ($1, $2) AND enabled ORDER BY account_id FOR UPDATE SKIP LOCKED",
        vec![pair[0].into(), pair[1].into()],
    ))
    .await
    .map_err(unavailable)?
    .iter()
    .map(|row| column(row, "account_id"))
    .collect()
}

/// Both AIs' live offers for the format, in account order and shared, so
/// that the end of a session they were made under waits for this match.
async fn offers(
    db: &DatabaseTransaction,
    pair: [Uuid; 2],
    format: &str,
) -> Result<Vec<(Uuid, Vec<u8>)>, Error> {
    db.query_all_raw(statement(
        "SELECT account_id, commitment FROM central_identity.ai_offers
         WHERE account_id IN ($1, $2) AND format = $3 AND expires_at > pg_catalog.clock_timestamp()
         ORDER BY account_id FOR SHARE",
        vec![pair[0].into(), pair[1].into(), format.into()],
    ))
    .await
    .map_err(unavailable)?
    .iter()
    .map(|row| Ok((column(row, "account_id")?, column(row, "commitment")?)))
    .collect()
}

/// Make a match between the AI `me` and another AI in `format` when every
/// rule allows it: the new match's id, or why none was made. Nothing is
/// changed otherwise.
pub(super) async fn seat(
    db: &DatabaseTransaction,
    me: Uuid,
    format: Format,
) -> Result<Result<Uuid, Unseated>, Error> {
    let format = format.as_str();
    // Every refusal that needs no lock is read first, as fill-in does, so a
    // runner that asks often holds the settings only when a match is near.
    let settings = ai::settings(db, false).await?;
    let counted = count(db, me, format, &settings).await?;
    if let Some(reason) = reason::unseated(&counted.facts) {
        return Ok(Err(reason));
    }
    let reserve = i16::from(settings.host_reserve);
    let candidates = fill_in::eligible(db, me, format, true, settings.drain_minutes, None).await?;
    if candidates.is_empty() || !room_single(db, COMPATIBLE, reserve).await? {
        return Ok(Err(Unseated::Later));
    }
    // The settings serialize the carousel and fill-in against every bound;
    // what they hold is counted again after the lock.
    let settings = ai::settings(db, true).await?;
    let counted = count(db, me, format, &settings).await?;
    if let Some(reason) = reason::unseated(&counted.facts) {
        return Ok(Err(reason));
    }
    let Some(season) = counted.facts.season else {
        return Ok(Err(Unseated::Off));
    };
    let reserve = i16::from(settings.host_reserve);
    for index in policy::order(counted.elo, &candidates)
        .into_iter()
        .take(TRIES)
    {
        let partner = candidates[index].id;
        let mut pair = [me, partner];
        pair.sort_unstable();
        let locked = lock(db, pair).await?;
        if !locked.contains(&me) {
            return Ok(Err(Unseated::Later));
        }
        if locked.len() != 2 {
            continue;
        }
        // Both rows held: max_games, daily_games and the switch are counted
        // again in fresh statements, as the administration checks after its
        // locks.
        let counted = count(db, me, format, &settings).await?;
        if let Some(reason) = reason::unseated(&counted.facts) {
            return Ok(Err(reason));
        }
        if fill_in::eligible(db, me, format, true, settings.drain_minutes, Some(partner))
            .await?
            .is_empty()
        {
            continue;
        }
        let offered = offers(db, pair, format).await?;
        let [low, high] = offered.as_slice() else {
            continue;
        };
        let Some(host) = allocate_single(db, COMPATIBLE, reserve).await? else {
            return Ok(Err(Unseated::Later));
        };
        let seats = if fill_in::coin(db).await? {
            [low.clone(), high.clone()]
        } else {
            [high.clone(), low.clone()]
        };
        return Ok(
            fill_in::write(db, season, format, seats, host, Arranged::Carousel)
                .await?
                .ok_or(Unseated::Off),
        );
    }
    Ok(Err(Unseated::Later))
}
