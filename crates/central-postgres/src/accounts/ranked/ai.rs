//! An AI runner's operations on its account's native session
//! (`docs/ai-players.md` section 4): standing offers and its live matches.
use super::{Error, api, column, statement, unavailable};
use crate::accounts::ai::{LIVE, badge};
use api::ai::{AiBadge, AiKind, MAX_OFFERS, Offer, Request, Response, Settings};
use sea_orm::{ConnectionTrait, DatabaseTransaction, QueryResult, prelude::Uuid};

/// How long an offer stands without renewal.
const OFFER_SECONDS: f64 = 45.0;

pub(super) async fn handle(
    db: &DatabaseTransaction,
    me: Uuid,
    session: [u8; 32],
    request: &Request,
) -> Result<Response, Error> {
    let row = db
        .query_one_raw(statement(
            "SELECT kind, label, profile, enabled, max_games
             FROM central_identity.ai_players WHERE account_id = $1 FOR SHARE",
            vec![me.into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Rejected)?;
    match request {
        Request::Offer { offers } => offer(db, me, session, column(&row, "enabled")?, offers).await,
        Request::Work => work(db, me, &row).await,
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

/// The account's settings and its live matches, as a person's assignments.
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
                "SELECT m.id, m.host_id, m.origin, m.format, m.account_a, m.account_b,
                   m.commitment_a, m.commitment_b, m.game_id
                 FROM central_identity.ranked_matches m
                 WHERE $1 IN (m.account_a, m.account_b) AND {LIVE}
                 ORDER BY m.created_at, m.id LIMIT 16"
            ),
            vec![me.into()],
        ))
        .await
        .map_err(unavailable)?;
    let mut assignments = Vec::with_capacity(rows.len());
    for row in rows {
        let first: Uuid = column(&row, "account_a")?;
        let seat = u8::from(first != me);
        let (other, deck) = if seat == 0 {
            ("account_b", "commitment_a")
        } else {
            ("account_a", "commitment_b")
        };
        assignments.push(api::Assignment {
            match_id: column::<Uuid>(&row, "id")?.into_bytes(),
            host_id: column::<Uuid>(&row, "host_id")?.into_bytes(),
            origin: column(&row, "origin")?,
            format: column::<String>(&row, "format")?
                .parse()
                .map_err(unavailable)?,
            account: me.into_bytes(),
            seat,
            deck: column::<Vec<u8>>(&row, deck)?
                .try_into()
                .map_err(unavailable)?,
            game: column::<Option<Uuid>>(&row, "game_id")?.map(Uuid::into_bytes),
            opponent_ai: badge(db, column(&row, other)?).await?,
        });
    }
    Ok(Response::Work {
        settings,
        assignments,
    })
}
