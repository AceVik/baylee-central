//! An AI fills a lonely person's search (`docs/ai-players.md` section 3).
//! It runs inside the person's own Join or Status, after people were paired:
//! only a ticket that accepts an AI, has waited `fill_in_after`, and has no
//! other person waiting in its season, format and kind gets one.
use super::{Error, column, statement, unavailable};
use crate::accounts::ai::{self, LIVE, TODAY};
use central_api::ranked::ai::AiKind;
use sea_orm::{ConnectionTrait, DatabaseTransaction, QueryResult, prelude::Uuid};

pub(super) mod policy;

/// Most candidates fill-in tries after the first refuses under its lock.
pub(super) const TRIES: usize = 4;

/// An AI's live offer for format `$2`, made under a native session that is
/// live and has `$3` minutes left before its cap.
pub(super) const OFFERED: &str = "EXISTS (SELECT 1 FROM central_identity.ai_offers o
    JOIN central_identity.sessions s ON s.digest = o.session_digest AND s.account_id = o.account_id
    JOIN central_identity.accounts a ON a.id = s.account_id AND a.credential_revision = s.credential_revision
    WHERE o.account_id = p.account_id AND o.format = $2 AND o.expires_at > pg_catalog.clock_timestamp()
      AND s.kind = 'native' AND s.expires_at > pg_catalog.clock_timestamp() AND a.restored
      AND COALESCE(s.proven_at, s.created_at) + interval '12 hours'
          > pg_catalog.clock_timestamp() + pg_catalog.make_interval(mins => $3))";

/// What a ticket accepts: none, or whether a language model may come too.
pub(super) async fn consent(
    db: &impl ConnectionTrait,
    account: Uuid,
    key: Uuid,
) -> Result<Option<bool>, Error> {
    let row = db
        .query_one_raw(statement(
            "SELECT accepts FROM central_identity.ranked_ticket_ai
             WHERE account_id = $1 AND request_id = $2",
            vec![account.into(), key.into()],
        ))
        .await
        .map_err(unavailable)?;
    row.map(|row| Ok(column::<String>(&row, "accepts")? == "any"))
        .transpose()
}

fn eligible_sql() -> String {
    format!(
        "SELECT p.account_id, p.kind,
           COALESCE((SELECT r.queue_elo FROM central_identity.ranked_ratings r
             WHERE r.account_id = p.account_id AND r.format = $2
             ORDER BY r.last_at DESC LIMIT 1), 1500) AS elo,
           (SELECT count(*)::integer FROM (SELECT 1 FROM central_identity.ranked_matches m
             WHERE LEAST(m.account_a, m.account_b) = LEAST(p.account_id, $1)
               AND GREATEST(m.account_a, m.account_b) = GREATEST(p.account_id, $1)
               AND m.state = 'finished' AND m.finished_at > now() - interval '6 hours' LIMIT 3) x) AS meetings,
           (SELECT count(*) FROM central_identity.ranked_matches m
             WHERE p.account_id IN (m.account_a, m.account_b) AND {LIVE}) AS live,
           p.max_games, p.daily_games,
           (SELECT count(*) FROM central_identity.ai_matches am
             JOIN central_identity.ranked_matches m ON m.id = am.match_id
             WHERE p.account_id IN (m.account_a, m.account_b) AND am.created_at >= {TODAY}) AS today
         FROM central_identity.ai_players p
         WHERE p.enabled AND p.account_id <> $1 AND ($4 OR p.kind <> 'model') AND {OFFERED}
           AND ($5::uuid IS NULL OR p.account_id = $5)
         ORDER BY p.account_id LIMIT 64"
    )
}

/// The AIs other than `person` (the person searching, or the AI asking the
/// carousel) that pass every hard rule but the global bound, or just
/// `only`; their meetings are those with `person`.
pub(super) async fn eligible(
    db: &DatabaseTransaction,
    person: Uuid,
    format: &str,
    models: bool,
    drain: u16,
    only: Option<Uuid>,
) -> Result<Vec<policy::Candidate>, Error> {
    let rows = db
        .query_all_raw(statement(
            &eligible_sql(),
            vec![
                person.into(),
                format.into(),
                i32::from(drain).into(),
                models.into(),
                only.into(),
            ],
        ))
        .await
        .map_err(unavailable)?;
    let mut out = Vec::new();
    for row in rows {
        let live: i64 = column(&row, "live")?;
        let max: i16 = column(&row, "max_games")?;
        let daily: Option<i32> = column(&row, "daily_games")?;
        let today: i64 = column(&row, "today")?;
        if live >= i64::from(max) || daily.is_some_and(|cap| today >= i64::from(cap)) {
            continue;
        }
        let kind: String = column(&row, "kind")?;
        out.push(policy::Candidate {
            id: column(&row, "account_id")?,
            kind: AiKind::from_key(&kind).ok_or(Error::Unavailable)?,
            elo: column(&row, "elo")?,
            meetings: column(&row, "meetings")?,
            live,
        });
    }
    Ok(out)
}

/// AI seats in matches that hold a slot; a match between two AIs counts two.
pub(super) async fn seats_live(db: &impl ConnectionTrait) -> Result<i64, Error> {
    column(
        &db.query_one_raw(statement(
            &format!(
                "SELECT COALESCE(sum(CASE am.ai_seats WHEN 3 THEN 2 ELSE 1 END), 0)::bigint AS seats
                 FROM central_identity.ai_matches am
                 JOIN central_identity.ranked_matches m ON m.id = am.match_id WHERE {LIVE}"
            ),
            vec![],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Unavailable)?,
        "seats",
    )
}

/// Whether another person's ticket waits in the same season, format and kind.
async fn person_waiting(db: &DatabaseTransaction, ticket: &QueryResult) -> Result<bool, Error> {
    Ok(db
        .query_one_raw(statement(
            "SELECT 1 AS one FROM central_identity.ranked_tickets t
             WHERE t.season_id = $1 AND t.format = $2 AND t.replicable = $3 AND t.state = 'waiting'
               AND t.expires_at > pg_catalog.clock_timestamp() AND t.account_id <> $4
               AND NOT EXISTS (SELECT 1 FROM central_identity.ai_players p WHERE p.account_id = t.account_id)
             LIMIT 1",
            vec![
                column::<Uuid>(ticket, "season_id")?.into(),
                column::<String>(ticket, "format")?.into(),
                column::<bool>(ticket, "replicable")?.into(),
                column::<Uuid>(ticket, "account_id")?.into(),
            ],
        ))
        .await
        .map_err(unavailable)?
        .is_some())
}

/// The Unix second from which an AI may fill this waiting ticket, while it
/// accepts one and an enabled AI offers a deck for its format. It ignores
/// other people's tickets, which it must not reveal.
pub(super) async fn due(
    db: &DatabaseTransaction,
    ticket: &QueryResult,
) -> Result<Option<i64>, Error> {
    let me: Uuid = column(ticket, "account_id")?;
    let Some(models) = consent(db, me, column(ticket, "request_id")?).await? else {
        return Ok(None);
    };
    let settings = ai::settings(db, false).await?;
    if settings.global_seats == 0 || !settings.fill_in {
        return Ok(None);
    }
    let available = db
        .query_one_raw(statement(
            &format!(
                "SELECT 1 AS one FROM central_identity.ai_players p
                 WHERE p.enabled AND p.account_id <> $1 AND ($4 OR p.kind <> 'model') AND {OFFERED} LIMIT 1"
            ),
            vec![
                me.into(),
                column::<String>(ticket, "format")?.into(),
                i32::from(settings.drain_minutes).into(),
                models.into(),
            ],
        ))
        .await
        .map_err(unavailable)?
        .is_some();
    available
        .then(|| {
            column::<i64>(ticket, "at")
                .map(|at| at.saturating_add(i64::from(settings.fill_in_after)))
        })
        .transpose()
}

/// Seat an AI against this still-waiting ticket when every rule allows it.
/// Nothing happens otherwise; the person's next poll asks again.
pub(super) async fn seat(db: &DatabaseTransaction, ticket: &QueryResult) -> Result<(), Error> {
    let me: Uuid = column(ticket, "account_id")?;
    let key: Uuid = column(ticket, "request_id")?;
    let format: String = column(ticket, "format")?;
    let Some(models) = consent(db, me, key).await? else {
        return Ok(());
    };
    let settings = ai::settings(db, false).await?;
    let waited = db
        .query_one_raw(statement(
            "SELECT 1 AS one FROM central_identity.ranked_tickets WHERE account_id = $1
             AND request_id = $2 AND state = 'waiting'
             AND created_at <= pg_catalog.clock_timestamp() - pg_catalog.make_interval(secs => $3)",
            vec![
                me.into(),
                key.into(),
                f64::from(settings.fill_in_after).into(),
            ],
        ))
        .await
        .map_err(unavailable)?
        .is_some();
    if !waited
        || !settings.fill_in
        || settings.global_seats == 0
        || person_waiting(db, ticket).await?
    {
        return Ok(());
    }
    let candidates = eligible(db, me, &format, models, settings.drain_minutes, None).await?;
    if candidates.is_empty() {
        return Ok(());
    }
    // People first: the ticket is locked as pairing locks it, and still waiting.
    if db
        .query_one_raw(statement(
            "SELECT 1 AS one FROM central_identity.ranked_tickets WHERE account_id = $1
             AND request_id = $2 AND state = 'waiting' AND expires_at > pg_catalog.clock_timestamp()
             FOR UPDATE SKIP LOCKED",
            vec![me.into(), key.into()],
        ))
        .await
        .map_err(unavailable)?
        .is_none()
    {
        return Ok(());
    }
    let settings = ai::settings(db, true).await?;
    // Asked again under the lock: fill-in switched off for calibration
    // seats nothing from that commit on, and a person who joined while this
    // waited for the lock is paired first (people first, section 3).
    if !settings.fill_in
        || seats_live(db).await? >= i64::from(settings.global_seats)
        || person_waiting(db, ticket).await?
    {
        return Ok(());
    }
    let elo: i32 = column(ticket, "queue_elo")?;
    for index in policy::order(elo, &candidates).into_iter().take(TRIES) {
        let id = candidates[index].id;
        // Its row locked (skipped while the switch or another fill-in holds
        // it), then counted again in a fresh statement: max_games and the
        // switch hold, as the administration checks after its locks. Its
        // account's key is shared as the match's foreign key will share it,
        // skipped while a runner's exchange holds the row: an exchange that
        // ends a held offer's session waits for this fill-in, so this must
        // never wait for it (as the carousel does).
        if db
            .query_one_raw(statement(
                "SELECT 1 AS one FROM central_identity.ai_players p
                 JOIN central_identity.accounts a ON a.id = p.account_id
                 WHERE p.account_id = $1 AND p.enabled
                 FOR UPDATE OF p SKIP LOCKED FOR KEY SHARE OF a SKIP LOCKED",
                vec![id.into()],
            ))
            .await
            .map_err(unavailable)?
            .is_none()
        {
            continue;
        }
        if eligible(db, me, &format, models, settings.drain_minutes, Some(id))
            .await?
            .is_empty()
        {
            continue;
        }
        let Some(offer) = db
            .query_one_raw(statement(
                "SELECT commitment FROM central_identity.ai_offers
                 WHERE account_id = $1 AND format = $2 AND expires_at > pg_catalog.clock_timestamp()
                 FOR SHARE",
                vec![id.into(), format.clone().into()],
            ))
            .await
            .map_err(unavailable)?
        else {
            continue;
        };
        return place(db, ticket, id, &offer, settings.host_reserve).await;
    }
    Ok(())
}

/// Make the match between the waiting person and the chosen AI: one host
/// that keeps `reserve` slots, a drawn seat order, the record of how the
/// match was made, and the person's ticket assigned.
async fn place(
    db: &DatabaseTransaction,
    ticket: &QueryResult,
    id: Uuid,
    offer: &QueryResult,
    reserve: u8,
) -> Result<(), Error> {
    let me: Uuid = column(ticket, "account_id")?;
    let key: Uuid = column(ticket, "request_id")?;
    let format: String = column(ticket, "format")?;
    let replicable: bool = column(ticket, "replicable")?;
    let Some(host) =
        super::replication::routing::allocate_single(db, replicable, i16::from(reserve)).await?
    else {
        return Ok(());
    };
    let ai_first = coin(db).await?;
    let person = (me, column::<Vec<u8>>(ticket, "commitment")?);
    let ai = (id, column::<Vec<u8>>(offer, "commitment")?);
    let seats = if ai_first { [ai, person] } else { [person, ai] };
    let arranged = Arranged::FillIn { ai_first };
    let Some(matched) = write(
        db,
        column(ticket, "season_id")?,
        &format,
        seats,
        host,
        arranged,
    )
    .await?
    else {
        return Ok(());
    };
    let changed = db
        .execute_raw(statement(
            "UPDATE central_identity.ranked_tickets SET state='assigned', match_id=$1
             WHERE account_id=$2 AND request_id=$3 AND state='waiting'",
            vec![matched.into(), me.into(), key.into()],
        ))
        .await
        .map_err(unavailable)?;
    if changed.rows_affected() != 1 {
        return Err(Error::Conflict);
    }
    Ok(())
}

/// A fair coin from the database's generator: whether the first of two
/// seats goes first. With an AI seat, the seat order is drawn for each
/// match, never taken from who asked.
pub(super) async fn coin(db: &DatabaseTransaction) -> Result<bool, Error> {
    column(
        &db.query_one_raw(statement(
            "SELECT pg_catalog.random() < 0.5 AS first",
            vec![],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Unavailable)?,
        "first",
    )
}

/// How a match with an AI seat was made (`ai_matches.arranged`), and so
/// which of its seats are AIs.
#[derive(Clone, Copy, Debug)]
pub(super) enum Arranged {
    /// An AI fills a person's search, in seat zero when `ai_first`.
    FillIn {
        /// Whether the AI sits in seat zero.
        ai_first: bool,
    },
    /// The carousel: both seats are AIs.
    Carousel,
}

impl Arranged {
    /// The stored key and `ai_seats`: 1 seat zero, 2 seat one, 3 both.
    const fn row(self) -> (&'static str, i16) {
        match self {
            Self::FillIn { ai_first: true } => ("fill_in", 1),
            Self::FillIn { ai_first: false } => ("fill_in", 2),
            Self::Carousel => ("carousel", 3),
        }
    }
}

/// Write a match with an AI seat as `pair` writes one, on `host` and in
/// `season` while it is open, each seat with its account and the commitment
/// it plays, and the record of how it was made. None once the season has
/// closed.
pub(super) async fn write(
    db: &DatabaseTransaction,
    season: Uuid,
    format: &str,
    [(a, deck_a), (b, deck_b)]: [(Uuid, Vec<u8>); 2],
    host: super::host::Host,
    arranged: Arranged,
) -> Result<Option<Uuid>, Error> {
    let Some(matched) = db
        .query_one_raw(statement(
            "INSERT INTO central_identity.ranked_matches(season_id,format,account_a,account_b,
               commitment_a,commitment_b,host_id,ranked_epoch,origin)
             SELECT s.id,$2,$3,$4,$5,$6,$7,$8,$9 FROM central_identity.ranked_seasons s WHERE s.id=$1
             AND s.starts_at<=pg_catalog.clock_timestamp()
             AND (s.ends_at IS NULL OR s.ends_at>pg_catalog.clock_timestamp()) RETURNING id",
            vec![
                season.into(),
                format.into(),
                a.into(),
                b.into(),
                deck_a.into(),
                deck_b.into(),
                host.id.into(),
                host.epoch.into(),
                host.origin.into(),
            ],
        ))
        .await
        .map_err(unavailable)?
    else {
        return Ok(None);
    };
    let matched: Uuid = column(&matched, "id")?;
    let (key, seats) = arranged.row();
    db.execute_raw(statement(
        "INSERT INTO central_identity.ai_matches(match_id, arranged, ai_seats) VALUES ($1, $2, $3)",
        vec![matched.into(), key.into(), seats.into()],
    ))
    .await
    .map_err(unavailable)?;
    Ok(Some(matched))
}
