//! An AI runner's operations on its account's native session
//! (`docs/ai-players.md` sections 4 and 7): standing offers, its live
//! matches, and a match against another AI.
use super::{Error, api, carousel, column, statement, unavailable};
use crate::accounts::ai::{LIVE, badge};
use api::ai::{
    AiBadge, AiKind, FINISHED, Finished, MAX_GAMES, MAX_OFFERS, MatchResult, Offer, Request,
    Response, Settings,
};
use sea_orm::{ConnectionTrait, DatabaseTransaction, QueryResult, prelude::Uuid};

/// How long an offer stands without renewal.
const OFFER_SECONDS: f64 = 45.0;

/// The columns of a match an assignment is read from.
const SEAT: &str = "m.id, m.host_id, m.origin, m.format, m.account_a, m.account_b,
    m.commitment_a, m.commitment_b, m.game_id";

pub(super) async fn handle(
    db: &DatabaseTransaction,
    me: Uuid,
    session: [u8; 32],
    request: &Request,
) -> Result<Response, Error> {
    // Only an offer locks the AI's row here, so that a switch-off waits for
    // it and then withdraws what it stored; the work reads it as it stands,
    // and the carousel locks it later, after the settings, as fill-in does.
    let row = db
        .query_one_raw(statement(
            if matches!(request, Request::Offer { .. }) {
                "SELECT kind, label, profile, enabled, max_games
                 FROM central_identity.ai_players WHERE account_id = $1 FOR SHARE"
            } else {
                "SELECT kind, label, profile, enabled, max_games
                 FROM central_identity.ai_players WHERE account_id = $1"
            },
            vec![me.into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Rejected)?;
    match request {
        Request::Offer { offers } => offer(db, me, session, column(&row, "enabled")?, offers).await,
        Request::Work => work(db, me, &row).await,
        Request::Carousel { format } => match carousel::seat(db, me, *format).await? {
            Ok(matched) => Ok(Response::Seated {
                assignment: seated(db, me, matched).await?,
            }),
            Err(reason) => Ok(Response::Unseated { reason }),
        },
    }
}

/// Replace the account's offers, one per format, under this session.
async fn offer(
    db: &DatabaseTransaction,
    me: Uuid,
    session: [u8; 32],
    enabled: bool,
    offers: &[Offer],
) -> Result<Response, Error> {
    let mut formats: Vec<_> = offers.iter().map(|offer| offer.format).collect();
    formats.sort_unstable();
    formats.dedup();
    if offers.len() > MAX_OFFERS || formats.len() != offers.len() {
        return Err(Error::Rejected);
    }
    if !enabled && !offers.is_empty() {
        return Err(Error::Rejected);
    }
    db.execute_raw(statement(
        "DELETE FROM central_identity.ai_offers WHERE account_id = $1",
        vec![me.into()],
    ))
    .await
    .map_err(unavailable)?;
    let mut until = None;
    for offer in offers {
        let row = db
            .query_one_raw(statement(
                "INSERT INTO central_identity.ai_offers
                   (account_id, format, session_digest, commitment, renewed_at, expires_at)
                 SELECT $1, $2, $3, $4, c.at, c.at + pg_catalog.make_interval(secs => $5)
                 FROM (SELECT pg_catalog.clock_timestamp() AS at) c
                 RETURNING floor(extract(epoch FROM expires_at))::bigint AS until",
                vec![
                    me.into(),
                    offer.format.as_str().into(),
                    session.to_vec().into(),
                    offer.deck.to_vec().into(),
                    OFFER_SECONDS.into(),
                ],
            ))
            .await
            .map_err(unavailable)?
            .ok_or(Error::Unavailable)?;
        until = Some(column::<i64>(&row, "until")?);
    }
    Ok(Response::Offered { until })
}

/// The account's settings and every live match, as a person's assignments:
/// all of them, up to [`MAX_GAMES`], even while an administrator has just
/// lowered `max_games` below how many are live.
async fn work(db: &DatabaseTransaction, me: Uuid, row: &QueryResult) -> Result<Response, Error> {
    let max: i16 = column(row, "max_games")?;
    let kind: String = column(row, "kind")?;
    let settings = Settings {
        badge: AiBadge {
            kind: AiKind::from_key(&kind).ok_or(Error::Unavailable)?,
            label: column(row, "label")?,
        },
        profile: column(row, "profile")?,
        enabled: column(row, "enabled")?,
        max_games: u8::try_from(max).map_err(unavailable)?,
    };
    let rows = db
        .query_all_raw(statement(
            &format!(
                "SELECT {SEAT} FROM central_identity.ranked_matches m
                 WHERE $1 IN (m.account_a, m.account_b) AND {LIVE}
                 ORDER BY m.created_at, m.id LIMIT $2"
            ),
            vec![me.into(), i64::from(MAX_GAMES).into()],
        ))
        .await
        .map_err(unavailable)?;
    let mut assignments = Vec::with_capacity(rows.len());
    for row in rows {
        assignments.push(assignment(db, me, &row).await?);
    }
    Ok(Response::Work {
        settings,
        assignments,
        finished: finished(db, me).await?,
    })
}

/// The account's matches settled in the last day, newest first, at most
/// [`FINISHED`]: the receipts written in the transaction that settled each,
/// read from the AI's seat.
async fn finished(db: &DatabaseTransaction, me: Uuid) -> Result<Vec<Finished>, Error> {
    let rows = db
        .query_all_raw(statement(
            "SELECT r.match_id, r.format, r.outcome, r.elo, r.delta,
                 floor(extract(epoch FROM r.settled_at))::bigint AS settled_at,
                 m.account_a <> r.account_id AS second,
                 p.kind AS ai_kind, p.label AS ai_label
             FROM central_identity.ranked_receipts r
             JOIN central_identity.ranked_matches m ON m.id = r.match_id
             LEFT JOIN central_identity.ai_players p ON p.account_id = r.opponent_id
             WHERE r.account_id = $1
             AND r.settled_at > pg_catalog.clock_timestamp() - interval '1 day'
             ORDER BY r.settled_at DESC, r.match_id DESC LIMIT $2",
            vec![me.into(), i64::from(FINISHED).into()],
        ))
        .await
        .map_err(unavailable)?;
    rows.iter()
        .map(|row| {
            Ok(Finished {
                match_id: column::<Uuid>(row, "match_id")?.into_bytes(),
                format: column::<String>(row, "format")?
                    .parse()
                    .map_err(unavailable)?,
                seat: u8::from(column::<bool>(row, "second")?),
                result: match column::<i16>(row, "outcome")? {
                    1 => MatchResult::Win,
                    0 => MatchResult::Draw,
                    -1 => MatchResult::Loss,
                    _ => return Err(Error::Unavailable),
                },
                elo: column(row, "elo")?,
                delta: column(row, "delta")?,
                settled_at: column(row, "settled_at")?,
                opponent_ai: crate::accounts::ai::badge_of(row)?,
            })
        })
        .collect()
}

/// The account's seat in the match the carousel has just made.
async fn seated(
    db: &DatabaseTransaction,
    me: Uuid,
    matched: Uuid,
) -> Result<api::Assignment, Error> {
    let row = db
        .query_one_raw(statement(
            &format!("SELECT {SEAT} FROM central_identity.ranked_matches m WHERE m.id = $1"),
            vec![matched.into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Unavailable)?;
    assignment(db, me, &row).await
}

/// The account's seat in a match read with [`SEAT`], as a person's assignment.
async fn assignment(
    db: &DatabaseTransaction,
    me: Uuid,
    row: &QueryResult,
) -> Result<api::Assignment, Error> {
    let first: Uuid = column(row, "account_a")?;
    let seat = u8::from(first != me);
    let (other, deck) = if seat == 0 {
        ("account_b", "commitment_a")
    } else {
        ("account_a", "commitment_b")
    };
    Ok(api::Assignment {
        match_id: column::<Uuid>(row, "id")?.into_bytes(),
        host_id: column::<Uuid>(row, "host_id")?.into_bytes(),
        origin: column(row, "origin")?,
        format: column::<String>(row, "format")?
            .parse()
            .map_err(unavailable)?,
        account: me.into_bytes(),
        seat,
        deck: column::<Vec<u8>>(row, deck)?
            .try_into()
            .map_err(unavailable)?,
        game: column::<Option<Uuid>>(row, "game_id")?.map(Uuid::into_bytes),
        opponent_ai: badge(db, column(row, other)?).await?,
    })
}
