//! The session's own ranked rows for its download, one bounded page at a
//! time. Every query is keyed by the account the session resolves to; the
//! request names only a dataset and a key. Another account's columns are
//! reduced away in SQL: a match yields this seat's commitment, claim and
//! operator decisions, a receipt its opponent's public account ID alone.
use super::{Error, Session, column, statement, unavailable, uuid};
use central_api::ranked::{
    self as api, DeckResult, Format, PoolEntry, Rank, Record, Season, Track,
    export::{
        self, Champion, Deck, DeckRating, Entry, Key, Match, MatchState, Operator, Peak, Rating,
        Receipt, SeasonEntry, Table, Ticket, TicketState, Unlock, Verdict,
    },
};
use central_api::terms::TermsVersion;
use sea_orm::{ConnectionTrait, DatabaseTransaction, QueryResult, Value, prelude::Uuid};
use std::collections::BTreeMap;

/// Any live session of an account with a credential: an account that never
/// played (or never restored its vault) gets empty, complete datasets rather
/// than failing the whole download.
pub(super) async fn owner(db: &DatabaseTransaction, session: Session<'_>) -> Result<Uuid, Error> {
    let row = db
        .query_one_raw(statement(
            "SELECT a.id FROM central_identity.accounts a JOIN central_identity.sessions s
         ON s.account_id=a.id AND s.credential_revision=a.credential_revision
         WHERE s.digest=$1 AND s.kind=$2 AND s.expires_at>pg_catalog.clock_timestamp()
         AND a.credential IS NOT NULL",
            vec![session.digest().to_vec().into(), session.kind().into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Rejected)?;
    column(&row, "id")
}

/// One page of `table` after `after`, for the account `me`.
pub(super) async fn page(
    db: &DatabaseTransaction,
    me: Uuid,
    table: Table,
    after: Option<Key>,
) -> Result<api::Response, Error> {
    if after.is_some_and(|key| !export::valid_key(table, &key)) {
        return Err(Error::Rejected);
    }
    let entries = match table {
        Table::Seasons => seasons(db, me, after).await?,
        Table::Decks => decks(db, me, after).await?,
        Table::Receipts => receipts(db, me, after).await?,
        Table::Matches => matches(db, me, after).await?,
        Table::Tickets => tickets(db, me, after).await?,
        Table::Cosmetics => cosmetics(db, me, after).await?,
    };
    let more = entries.len() > table.page_size();
    let entries: Vec<Entry> = entries.into_iter().take(table.page_size()).collect();
    let after = if more {
        entries.last().map(|entry| entry.key().1)
    } else {
        None
    };
    Ok(api::Response::Export { entries, after })
}

/// The page query's `LIMIT`: one more row than a page tells whether another follows.
fn limit(table: Table) -> i64 {
    i64::try_from(table.page_size() + 1).unwrap_or(i64::MAX)
}

fn id(row: &QueryResult, name: &str) -> Result<api::Id, Error> {
    Ok(column::<Uuid>(row, name)?.into_bytes())
}

fn bytes32(row: &QueryResult, name: &str) -> Result<[u8; 32], Error> {
    column::<Vec<u8>>(row, name)?
        .try_into()
        .map_err(unavailable)
}

fn format(row: &QueryResult) -> Result<Format, Error> {
    column::<String>(row, "format")?
        .parse()
        .map_err(unavailable)
}

fn count(row: &QueryResult, name: &str) -> Result<u32, Error> {
    u32::try_from(column::<i32>(row, name)?).map_err(unavailable)
}

fn career(row: &QueryResult) -> Result<Record, Error> {
    Ok(Record {
        games: count(row, "games")?,
        wins: count(row, "wins")?,
        losses: count(row, "losses")?,
        draws: count(row, "draws")?,
    })
}

fn place(row: &QueryResult) -> Result<Option<u8>, Error> {
    column::<Option<i16>>(row, "place")?
        .map(u8::try_from)
        .transpose()
        .map_err(unavailable)
}

/// Stored league and division as the account's own profile names them:
/// Unranked before ten games, never the boards' inactivity clamp.
fn rank(row: &QueryResult, games: i32) -> Result<Rank, Error> {
    super::boards::wire_rank(
        column(row, "league")?,
        column(row, "division")?,
        games,
        place(row)?,
    )
}

fn verdict(mark: i16) -> Result<Verdict, Error> {
    match mark {
        1 => Ok(Verdict::Won),
        0 => Ok(Verdict::Drawn),
        -1 => Ok(Verdict::Lost),
        _ => Err(Error::Unavailable),
    }
}

/// `$2` is the exclusive key and `$3` the page's last key: the rows that
/// belong to the page's parents are exactly those keyed in that range.
fn range(me: Uuid, after: Option<Uuid>, last: Uuid) -> Vec<Value> {
    vec![me.into(), after.into(), last.into()]
}

async fn seasons(
    db: &DatabaseTransaction,
    me: Uuid,
    after: Option<Key>,
) -> Result<Vec<Entry>, Error> {
    let after = match after {
        Some(Key::Id(key)) => Some(uuid(key)),
        _ => None,
    };
    let rows = db
        .query_all_raw(statement(
            "SELECT s.id,COALESCE(s.name,to_char(s.starts_at AT TIME ZONE 'UTC','YYYY-MM-DD')) AS name,
             floor(extract(epoch FROM s.starts_at))::bigint AS starts,
             floor(extract(epoch FROM s.ends_at))::bigint AS ends
             FROM central_identity.ranked_seasons s WHERE ($2::uuid IS NULL OR s.id>$2) AND (
               EXISTS (SELECT 1 FROM central_identity.ranked_ratings r WHERE r.account_id=$1 AND r.season_id=s.id)
               OR EXISTS (SELECT 1 FROM central_identity.ranked_badge_peaks p WHERE p.account_id=$1 AND p.season_id=s.id)
               OR EXISTS (SELECT 1 FROM central_identity.ranked_champions c WHERE c.account_id=$1 AND c.season_id=s.id))
             ORDER BY s.id LIMIT $3",
            vec![me.into(), after.into(), limit(Table::Seasons).into()],
        ))
        .await
        .map_err(unavailable)?;
    let mut page: BTreeMap<Uuid, SeasonEntry> = BTreeMap::new();
    for row in &rows {
        page.insert(
            column(row, "id")?,
            SeasonEntry {
                season: Season {
                    id: id(row, "id")?,
                    name: column(row, "name")?,
                    starts: column(row, "starts")?,
                    ends: column(row, "ends")?,
                },
                ratings: Vec::new(),
                peaks: Vec::new(),
                champion: None,
            },
        );
    }
    let Some(last) = page.keys().next_back().copied() else {
        return Ok(Vec::new());
    };
    let keys = range(me, after, last);
    season_ratings(db, keys.clone(), &mut page).await?;
    season_peaks(db, keys.clone(), &mut page).await?;
    season_champions(db, keys, &mut page).await?;
    Ok(page
        .into_values()
        .map(|season| Entry::Season { season })
        .collect())
}

type Seasons = BTreeMap<Uuid, SeasonEntry>;

fn season_of<'a>(page: &'a mut Seasons, row: &QueryResult) -> Result<&'a mut SeasonEntry, Error> {
    page.get_mut(&column::<Uuid>(row, "season_id")?)
        .ok_or(Error::Unavailable)
}

async fn season_ratings(
    db: &DatabaseTransaction,
    keys: Vec<Value>,
    page: &mut Seasons,
) -> Result<(), Error> {
    let ratings = db
        .query_all_raw(statement(
            "SELECT r.season_id,r.format,r.games,r.wins,r.losses,r.draws,r.league,r.division,r.elo,r.queue_elo,
             round(r.score*1000)::bigint AS score,round(r.placement*1000)::bigint AS placement,
             round(r.allowance*1000)::bigint AS allowance,
             floor(extract(epoch FROM r.first_at))::bigint AS first_at,
             floor(extract(epoch FROM r.last_at))::bigint AS last_at,c.place
             FROM central_identity.ranked_ratings r LEFT JOIN central_identity.ranked_champions c
               ON c.season_id=r.season_id AND c.account_id=r.account_id AND c.player_format=r.format
             WHERE r.account_id=$1 AND ($2::uuid IS NULL OR r.season_id>$2) AND r.season_id<=$3
             ORDER BY r.season_id,r.format",
            keys,
        ))
        .await
        .map_err(unavailable)?;
    for row in &ratings {
        let games = column::<i32>(row, "games")?;
        season_of(page, row)?.ratings.push(Rating {
            format: format(row)?,
            career: career(row)?,
            rank: rank(row, games)?,
            elo: column(row, "elo")?,
            queue_elo: column(row, "queue_elo")?,
            score_thousandths: column(row, "score")?,
            placement_thousandths: column(row, "placement")?,
            allowance_thousandths: column(row, "allowance")?,
            first_at: column(row, "first_at")?,
            last_at: column(row, "last_at")?,
        });
    }
    Ok(())
}

async fn season_peaks(
    db: &DatabaseTransaction,
    keys: Vec<Value>,
    page: &mut Seasons,
) -> Result<(), Error> {
    let peaks = db
        .query_all_raw(statement(
            "SELECT p.season_id,p.track,p.format,p.league,p.division,p.place
             FROM central_identity.ranked_badge_peaks p
             WHERE p.account_id=$1 AND ($2::uuid IS NULL OR p.season_id>$2) AND p.season_id<=$3
             ORDER BY p.season_id,p.track,p.format,p.league,p.division,p.podium",
            keys,
        ))
        .await
        .map_err(unavailable)?;
    for row in &peaks {
        let entry = season_of(page, row)?;
        entry.peaks.push(Peak {
            track: match column::<String>(row, "track")?.as_str() {
                "player" => Track::Player,
                "deck" => Track::Deck,
                _ => return Err(Error::Unavailable),
            },
            format: format(row)?,
            // A peak is a placed design: ten games are behind every one.
            rank: rank(row, 10)?,
        });
        if entry.peaks.len() > export::MAX_SEASON_PEAKS {
            return Err(Error::Unavailable);
        }
    }
    Ok(())
}

async fn season_champions(
    db: &DatabaseTransaction,
    keys: Vec<Value>,
    page: &mut Seasons,
) -> Result<(), Error> {
    let champions = db
        .query_all_raw(statement(
            "SELECT c.season_id,c.place,c.player_format,c.deck_format,v.public_id
             FROM central_identity.ranked_champions c LEFT JOIN central_identity.ranked_deck_versions v
               ON v.account_id=c.account_id AND v.format=c.deck_format AND v.version=c.version
             WHERE c.account_id=$1 AND ($2::uuid IS NULL OR c.season_id>$2) AND c.season_id<=$3",
            keys,
        ))
        .await
        .map_err(unavailable)?;
    for row in &champions {
        let parse = |name| -> Result<Option<Format>, Error> {
            column::<Option<String>>(row, name)?
                .map(|value| value.parse().map_err(unavailable))
                .transpose()
        };
        season_of(page, row)?.champion = Some(Champion {
            place: place(row)?.ok_or(Error::Unavailable)?,
            player_format: parse("player_format")?,
            deck_format: parse("deck_format")?,
            deck: column::<Option<Vec<u8>>>(row, "public_id")?
                .map(<[u8; 32]>::try_from)
                .transpose()
                .map_err(unavailable)?,
        });
    }
    Ok(())
}

/// The stored pool, `{"<64 hex digits>": copies}`, as ascending entries.
fn cards(text: &str) -> Result<Vec<PoolEntry>, Error> {
    let counts: BTreeMap<String, u16> =
        central_api::decode(text.as_bytes()).map_err(unavailable)?;
    let mut cards = Vec::with_capacity(counts.len());
    for (hex, copies) in counts {
        let mut card = [0; 32];
        if hex.len() != 64 {
            return Err(Error::Unavailable);
        }
        for (byte, pair) in card.iter_mut().zip(hex.as_bytes().chunks(2)) {
            let pair = std::str::from_utf8(pair).map_err(unavailable)?;
            *byte = u8::from_str_radix(pair, 16).map_err(unavailable)?;
        }
        cards.push(PoolEntry { card, copies });
    }
    cards.sort_by_key(|entry| entry.card);
    if !api::valid_pool(&cards) {
        return Err(Error::Unavailable);
    }
    Ok(cards)
}

async fn decks(
    db: &DatabaseTransaction,
    me: Uuid,
    after: Option<Key>,
) -> Result<Vec<Entry>, Error> {
    let after = match after {
        Some(Key::Deck(key)) => Some(key.to_vec()),
        _ => None,
    };
    let rows = db
        .query_all_raw(statement(
            "SELECT v.public_id,v.format,v.cards::text AS cards,v.copies,v.elo,v.anchor,v.initial_pool,v.first_season
             FROM central_identity.ranked_deck_versions v
             WHERE v.account_id=$1 AND ($2::bytea IS NULL OR v.public_id>$2)
             ORDER BY v.public_id LIMIT $3",
            vec![me.into(), after.clone().into(), limit(Table::Decks).into()],
        ))
        .await
        .map_err(unavailable)?;
    let mut page: BTreeMap<[u8; 32], Deck> = BTreeMap::new();
    for row in &rows {
        let deck = bytes32(row, "public_id")?;
        page.insert(
            deck,
            Deck {
                deck,
                format: format(row)?,
                cards: cards(&column::<String>(row, "cards")?)?,
                copies: count(row, "copies")?,
                elo: column(row, "elo")?,
                anchor: column(row, "anchor")?,
                initial: column(row, "initial_pool")?,
                first_season: id(row, "first_season")?,
                ratings: Vec::new(),
            },
        );
    }
    let Some(last) = page.keys().next_back().copied() else {
        return Ok(Vec::new());
    };
    let ratings = db
        .query_all_raw(statement(
            "SELECT v.public_id,r.season_id,r.games,r.season_games,r.wins,r.losses,r.draws,r.league,r.division,r.elo,
             floor(extract(epoch FROM r.first_at))::bigint AS first_at,
             floor(extract(epoch FROM r.last_at))::bigint AS last_at,c.place
             FROM central_identity.ranked_deck_ratings r
             JOIN central_identity.ranked_deck_versions v USING (account_id,format,version)
             JOIN central_identity.ranked_seasons s ON s.id=r.season_id
             LEFT JOIN central_identity.ranked_champions c ON c.season_id=r.season_id
               AND c.account_id=r.account_id AND c.deck_format=r.format AND c.version=r.version
             WHERE r.account_id=$1 AND ($2::bytea IS NULL OR v.public_id>$2) AND v.public_id<=$3
             ORDER BY v.public_id,s.starts_at",
            vec![me.into(), after.into(), last.to_vec().into()],
        ))
        .await
        .map_err(unavailable)?;
    for row in &ratings {
        let games = column::<i32>(row, "games")?;
        let deck = page
            .get_mut(&bytes32(row, "public_id")?)
            .ok_or(Error::Unavailable)?;
        deck.ratings.push(DeckRating {
            season: id(row, "season_id")?,
            career: career(row)?,
            season_games: count(row, "season_games")?,
            rank: rank(row, games)?,
            elo: column(row, "elo")?,
            first_at: column(row, "first_at")?,
            last_at: column(row, "last_at")?,
        });
    }
    Ok(page
        .into_values()
        .map(|deck| Entry::Deck { deck })
        .collect())
}

fn uuid_after(after: Option<Key>) -> Option<Uuid> {
    match after {
        Some(Key::Id(key)) => Some(uuid(key)),
        _ => None,
    }
}

async fn receipts(
    db: &DatabaseTransaction,
    me: Uuid,
    after: Option<Key>,
) -> Result<Vec<Entry>, Error> {
    let rows = db
        .query_all_raw(statement(
            "SELECT match_id,format,floor(extract(epoch FROM settled_at))::bigint AS settled_at,outcome,
             opponent_id,elo,delta,before_rank::text AS before_rank,after_rank::text AS after_rank,
             deck_result::text AS deck_result
             FROM central_identity.ranked_receipts WHERE account_id=$1 AND ($2::uuid IS NULL OR match_id>$2)
             ORDER BY match_id LIMIT $3",
            vec![
                me.into(),
                uuid_after(after).into(),
                limit(Table::Receipts).into(),
            ],
        ))
        .await
        .map_err(unavailable)?;
    rows.iter()
        .map(|row| {
            let read = |name| -> Result<Rank, Error> {
                central_api::decode(column::<String>(row, name)?.as_bytes()).map_err(unavailable)
            };
            let deck: DeckResult =
                central_api::decode(column::<String>(row, "deck_result")?.as_bytes())
                    .map_err(unavailable)?;
            Ok(Entry::Receipt {
                receipt: Receipt {
                    match_id: id(row, "match_id")?,
                    format: format(row)?,
                    settled_at: column(row, "settled_at")?,
                    verdict: verdict(column(row, "outcome")?)?,
                    opponent: id(row, "opponent_id")?,
                    elo: column(row, "elo")?,
                    delta: column(row, "delta")?,
                    before: read("before_rank")?,
                    after: read("after_rank")?,
                    deck,
                },
            })
        })
        .collect()
}

async fn matches(
    db: &DatabaseTransaction,
    me: Uuid,
    after: Option<Key>,
) -> Result<Vec<Entry>, Error> {
    let after = uuid_after(after);
    // Only this seat's columns leave the database.
    let rows = db
        .query_all_raw(statement(
            "SELECT m.id,m.season_id,m.format,(CASE WHEN m.account_a=$1 THEN 0 ELSE 1 END)::smallint AS seat,
             CASE WHEN m.account_a=$1 THEN m.commitment_a ELSE m.commitment_b END AS commitment,
             CASE WHEN m.account_a=$1 THEN m.claimed_a ELSE m.claimed_b END AS claimed,
             m.host_id,m.origin,m.state,m.game_id,m.outcome,
             floor(extract(epoch FROM m.created_at))::bigint AS created_at,
             floor(extract(epoch FROM m.finished_at))::bigint AS finished_at
             FROM central_identity.ranked_matches m
             WHERE (m.account_a=$1 OR m.account_b=$1) AND ($2::uuid IS NULL OR m.id>$2)
             ORDER BY m.id LIMIT $3",
            vec![me.into(), after.into(), limit(Table::Matches).into()],
        ))
        .await
        .map_err(unavailable)?;
    let mut page: BTreeMap<Uuid, Match> = BTreeMap::new();
    for row in &rows {
        let seat = u8::try_from(column::<i16>(row, "seat")?).map_err(unavailable)?;
        let outcome = column::<Option<i16>>(row, "outcome")?;
        page.insert(
            column(row, "id")?,
            Match {
                match_id: id(row, "id")?,
                season: id(row, "season_id")?,
                format: format(row)?,
                seat,
                deck: bytes32(row, "commitment")?,
                claimed: column(row, "claimed")?,
                host_id: id(row, "host_id")?,
                origin: column(row, "origin")?,
                state: match column::<String>(row, "state")?.as_str() {
                    "reserved" => MatchState::Reserved,
                    "playing" => MatchState::Playing,
                    "finished" => MatchState::Finished,
                    "void" => MatchState::Void,
                    _ => return Err(Error::Unavailable),
                },
                game: column::<Option<Uuid>>(row, "game_id")?.map(Uuid::into_bytes),
                verdict: outcome
                    .map(|outcome| match (outcome, seat) {
                        (0, _) => Ok(Verdict::Drawn),
                        (1, 0) | (2, 1) => Ok(Verdict::Won),
                        (1, 1) | (2, 0) => Ok(Verdict::Lost),
                        _ => Err(Error::Unavailable),
                    })
                    .transpose()?,
                created_at: column(row, "created_at")?,
                finished_at: column(row, "finished_at")?,
                operators: Vec::new(),
            },
        );
    }
    let Some(last) = page.keys().next_back().copied() else {
        return Ok(Vec::new());
    };
    let operators = db
        .query_all_raw(statement(
            "SELECT p.match_id,p.host_id,p.origin,p.epoch,p.allowed,p.version,p.digest
             FROM central_identity.ranked_host_permissions p
             JOIN central_identity.ranked_matches m ON m.id=p.match_id
             WHERE (m.account_a=$1 OR m.account_b=$1) AND p.seat=(CASE WHEN m.account_a=$1 THEN 0 ELSE 1 END)
               AND ($2::uuid IS NULL OR m.id>$2) AND m.id<=$3
             ORDER BY p.match_id,p.host_id",
            range(me, after, last),
        ))
        .await
        .map_err(unavailable)?;
    for row in &operators {
        let seat = page
            .get_mut(&column::<Uuid>(row, "match_id")?)
            .ok_or(Error::Unavailable)?;
        let terms = match column::<Option<String>>(row, "version")? {
            Some(version) => Some(TermsVersion {
                version,
                digest: bytes32(row, "digest")?,
            }),
            None => None,
        };
        seat.operators.push(Operator {
            host_id: id(row, "host_id")?,
            origin: column(row, "origin")?,
            epoch: u64::try_from(column::<i64>(row, "epoch")?).map_err(unavailable)?,
            allowed: column(row, "allowed")?,
            terms,
        });
        if seat.operators.len() > export::MAX_MATCH_OPERATORS {
            return Err(Error::Unavailable);
        }
    }
    Ok(page
        .into_values()
        .map(|seat| Entry::Match { seat })
        .collect())
}

async fn tickets(
    db: &DatabaseTransaction,
    me: Uuid,
    after: Option<Key>,
) -> Result<Vec<Entry>, Error> {
    let rows = db
        .query_all_raw(statement(
            "SELECT request_id,season_id,format,commitment,queue_elo,replicable,state,match_id,
             floor(extract(epoch FROM created_at))::bigint AS created_at,
             floor(extract(epoch FROM expires_at))::bigint AS expires_at
             FROM central_identity.ranked_tickets WHERE account_id=$1 AND ($2::uuid IS NULL OR request_id>$2)
             ORDER BY request_id LIMIT $3",
            vec![
                me.into(),
                uuid_after(after).into(),
                limit(Table::Tickets).into(),
            ],
        ))
        .await
        .map_err(unavailable)?;
    rows.iter()
        .map(|row| {
            Ok(Entry::Ticket {
                ticket: Ticket {
                    key: id(row, "request_id")?,
                    season: id(row, "season_id")?,
                    format: format(row)?,
                    deck: bytes32(row, "commitment")?,
                    queue_elo: column(row, "queue_elo")?,
                    replicable: column(row, "replicable")?,
                    state: match column::<String>(row, "state")?.as_str() {
                        "waiting" => TicketState::Waiting,
                        "assigned" => TicketState::Assigned,
                        "closed" => TicketState::Closed,
                        _ => return Err(Error::Unavailable),
                    },
                    match_id: column::<Option<Uuid>>(row, "match_id")?.map(Uuid::into_bytes),
                    created_at: column(row, "created_at")?,
                    expires_at: column(row, "expires_at")?,
                },
            })
        })
        .collect()
}

async fn cosmetics(
    db: &DatabaseTransaction,
    me: Uuid,
    after: Option<Key>,
) -> Result<Vec<Entry>, Error> {
    let (league, division) = match after {
        Some(Key::Step { league, division }) => {
            (Some(i16::from(league)), Some(i16::from(division)))
        }
        _ => (None, None),
    };
    let rows = db
        .query_all_raw(statement(
            "SELECT league,division,floor(extract(epoch FROM earned_at))::bigint AS earned_at
             FROM central_identity.ranked_cosmetic_unlocks
             WHERE account_id=$1 AND ($2::smallint IS NULL OR (league,division)>($2::smallint,$3::smallint))
             ORDER BY league,division LIMIT $4",
            vec![
                me.into(),
                league.into(),
                division.into(),
                limit(Table::Cosmetics).into(),
            ],
        ))
        .await
        .map_err(unavailable)?;
    rows.iter()
        .map(|row| {
            Ok(Entry::Cosmetic {
                unlock: Unlock {
                    rank: super::boards::wire_rank(
                        column(row, "league")?,
                        column(row, "division")?,
                        10,
                        None,
                    )?,
                    earned_at: column(row, "earned_at")?,
                },
            })
        })
        .collect()
}
