//! The account download's ranked rows: the session's own and nothing of the
//! opponent's but its public account ID, paged and checked as the website
//! checks them; then the daily sweep's pruning of closed queue tickets.
use super::{AccountStore, DatabaseConnection, Error, ServerKeys, Token, enroll, login};
use central_api::ranked::{
    self as api, DeckResult, Format, League, PoolEntry, Rank, Record, Request, Response, Track,
    export::{self, Entry, Key, MatchState, Table, TicketState, Verdict},
};
use central_postgres::Session;
use sea_orm::{ConnectionTrait, DbBackend, Statement, Value, prelude::Uuid};

const HOST: &str = "https://ranked-export.example";
/// Recent closed tickets beside the scenario's, enough for a second page.
const EXTRA_TICKETS: u8 = 34;

fn v7(number: u8) -> api::Id {
    let mut bytes = [0; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes[14] = 0xe7;
    bytes[15] = number;
    bytes
}

fn rank(league: League, division: Option<u8>, champion_place: Option<u8>) -> Rank {
    Rank {
        league,
        division,
        champion_place,
    }
}

fn json<T: serde::Serialize>(value: &T) -> String {
    String::from_utf8(central_api::encode(value).unwrap()).unwrap()
}

async fn run_sql(db: &DatabaseConnection, sql: &str, values: Vec<Value>) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        values,
    ))
    .await
    .unwrap();
}

async fn one(db: &DatabaseConnection, sql: &str, values: Vec<Value>) -> sea_orm::QueryResult {
    db.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        values,
    ))
    .await
    .unwrap()
    .unwrap()
}

async fn page(
    store: &AccountStore,
    session: Option<Session<'_>>,
    table: Table,
    after: Option<Key>,
) -> Result<(Vec<Entry>, Option<Key>), Error> {
    match store
        .ranked(session, Request::Export { table, after })
        .await?
    {
        Response::Export { entries, after } => Ok((entries, after)),
        _ => panic!("an export answers with a page"),
    }
}

/// Every page of every dataset, each checked like the website checks it.
async fn download(store: &AccountStore, session: Session<'_>) -> Vec<(Table, Vec<Entry>)> {
    let mut all = Vec::new();
    for table in Table::ALL {
        let mut rows = Vec::new();
        let mut after = None;
        loop {
            let (entries, next) = page(store, Some(session), table, after).await.unwrap();
            export::validate_page(table, after, &entries, next).unwrap();
            rows.extend(entries);
            after = next;
            if after.is_none() {
                break;
            }
        }
        all.push((table, rows));
    }
    all
}

fn rows(all: &[(Table, Vec<Entry>)], table: Table) -> &[Entry] {
    &all.iter().find(|(t, _)| *t == table).unwrap().1
}

struct Fixture {
    season: Uuid,
    won: Uuid,
    void: Uuid,
    old: Uuid,
}

/// Synthetic rows as settlement, the badge ledger and the queue leave them;
/// the operator writes them, and the triggers add peaks and unlocks.
async fn seed(db: &DatabaseConnection, me: Uuid, them: Uuid) -> Fixture {
    let season: Uuid = one(
        db,
        "INSERT INTO central_identity.ranked_seasons(starts_at,ends_at,name)
         VALUES ('2003-01-01 UTC','2004-01-01 UTC','Export era') RETURNING id",
        vec![],
    )
    .await
    .try_get("", "id")
    .unwrap();
    let host: Uuid = one(
        db,
        "INSERT INTO central_identity.gateways(owner_id,origin,name,version,website,credential_digest,heartbeat_digest)
         VALUES ($1,$2,'Export host','1',$2||'/status',
           pg_catalog.sha256(pg_catalog.uuid_send(pg_catalog.gen_random_uuid())),
           pg_catalog.sha256(pg_catalog.uuid_send(pg_catalog.gen_random_uuid()))) RETURNING id",
        vec![me.into(), HOST.into()],
    )
    .await
    .try_get("", "id")
    .unwrap();
    let (won, void, old) = games(db, season, host, me, them).await;
    standings(db, season, me, them).await;
    tickets(db, season, me, them, won, old).await;
    Fixture {
        season,
        won,
        void,
        old,
    }
}

/// Three matches with both seats' consents and the won game's two receipts.
async fn games(
    db: &DatabaseConnection,
    season: Uuid,
    host: Uuid,
    me: Uuid,
    them: Uuid,
) -> (Uuid, Uuid, Uuid) {
    let id = |row: sea_orm::QueryResult| -> Uuid { row.try_get("", "id").unwrap() };
    // Seat 0 is mine and won; the void one seats me second; the old one ended two days ago.
    let won = id(one(db,
        "INSERT INTO central_identity.ranked_matches(season_id,format,account_a,account_b,commitment_a,commitment_b,
           claimed_a,claimed_b,start_fingerprint,host_id,ranked_epoch,origin,state,game_id,outcome,finished_at)
         VALUES ($1,'standard',$2,$3,$4,$5,true,true,$6,$7,0,$8,'finished',pg_catalog.uuidv7(),1,now()) RETURNING id",
        vec![season.into(), me.into(), them.into(), vec![0x11u8; 32].into(), vec![0x22u8; 32].into(),
             vec![0x33u8; 32].into(), host.into(), HOST.into()]).await);
    let void = id(one(db,
        "INSERT INTO central_identity.ranked_matches(season_id,format,account_a,account_b,commitment_a,commitment_b,
           claimed_a,host_id,ranked_epoch,origin,state,finished_at)
         VALUES ($1,'highlander',$2,$3,$4,$5,true,$6,0,$7,'void',now()) RETURNING id",
        vec![season.into(), them.into(), me.into(), vec![0x22u8; 32].into(), vec![0x11u8; 32].into(),
             host.into(), HOST.into()]).await);
    let old = id(one(db,
        "INSERT INTO central_identity.ranked_matches(season_id,format,account_a,account_b,commitment_a,commitment_b,
           host_id,ranked_epoch,origin,state,created_at,finished_at)
         VALUES ($1,'standard',$2,$3,$4,$5,$6,0,$7,'void',now()-interval '3 days',now()-interval '2 days') RETURNING id",
        vec![season.into(), me.into(), them.into(), vec![0x11u8; 32].into(), vec![0x22u8; 32].into(),
             host.into(), HOST.into()]).await);
    for (match_id, seat, allowed, version) in [
        (won, 0i16, true, Some("ops-2026-11")),
        (won, 1, false, None),
        (void, 0, true, Some("ops-2026-10")),
    ] {
        run_sql(db,
            "INSERT INTO central_identity.ranked_host_permissions(match_id,host_id,seat,epoch,allowed,origin,version,digest)
             VALUES ($1,$2,$3,0,$4,$5,$6,CASE WHEN $6::text IS NULL THEN NULL ELSE $7::bytea END)",
            vec![match_id.into(), host.into(), seat.into(), allowed.into(), HOST.into(),
                 version.map(str::to_owned).into(), vec![0x44u8; 32].into()]).await;
    }
    let deck = DeckResult {
        before: rank(League::Silver, Some(1), None),
        after: rank(League::Gold, Some(4), None),
        elo: 1561,
        delta: 11,
        placement_games: 10,
    };
    for (account, opponent, mark, elo, delta) in
        [(me, them, 1i16, 1650, 14), (them, me, -1, 1401, -14)]
    {
        run_sql(db,
            "INSERT INTO central_identity.ranked_receipts(match_id,account_id,format,opponent_id,opponent_established,
               elo_gap,outcome,elo,delta,before_rank,after_rank,deck_result)
             VALUES ($1,$2,'standard',$3,true,-8642,$4,$5,$6,$7::jsonb,$8::jsonb,$9::jsonb)",
            vec![won.into(), account.into(), opponent.into(), mark.into(), elo.into(), delta.into(),
                 json(&rank(League::Gold, Some(3), None)).into(), json(&rank(League::Gold, Some(2), None)).into(),
                 json(&deck).into()]).await;
    }
    (won, void, old)
}

/// Ratings, deck versions and a Champion place; the triggers add the peaks
/// and my unlocks.
async fn standings(db: &DatabaseConnection, season: Uuid, me: Uuid, them: Uuid) {
    // Twelve games place both; their ratings carry 0.001-point model values.
    for (account, league, elo, score) in [(me, 5i16, 1650, 1650.125f64), (them, 3, 1401, 1399.875)]
    {
        run_sql(db,
            "INSERT INTO central_identity.ranked_ratings(season_id,format,account_id,elo,queue_elo,score,placement,
               allowance,games,wins,losses,draws,league,division,first_at,last_at)
             VALUES ($1,'standard',$2,$3,$3-10,$4,1600.5,-3.25,12,7,4,1,$5,2,'2003-02-01 UTC','2003-03-01 UTC')",
            vec![season.into(), account.into(), elo.into(), score.into(), league.into()]).await;
    }
    // My deck holds cards 0x01 and 0x02; theirs a card of its own.
    for (account, version, cards) in [
        (
            me,
            [0x77u8; 32],
            format!(r#"{{"{}":20,"{}":40}}"#, "02".repeat(32), "01".repeat(32)),
        ),
        (them, [0x88; 32], format!(r#"{{"{}":60}}"#, "99".repeat(32))),
    ] {
        run_sql(db,
            "INSERT INTO central_identity.ranked_deck_versions(account_id,format,version,cards,copies,elo,anchor,initial_pool,first_season)
             VALUES ($1,'standard',$2,$3::jsonb,60,1561,1500,true,$4)",
            vec![account.into(), version.to_vec().into(), cards.into(), season.into()]).await;
        run_sql(db,
            "INSERT INTO central_identity.ranked_deck_ratings(season_id,account_id,format,version,elo,games,season_games,
               wins,losses,draws,league,division,first_at,last_at)
             VALUES ($1,$2,'standard',$3,1561,12,12,8,4,0,5,4,'2003-02-01 UTC','2003-03-01 UTC')",
            vec![season.into(), account.into(), version.to_vec().into()]).await;
    }
    run_sql(db,
        "INSERT INTO central_identity.ranked_champions(season_id,account_id,deck_format,version,place)
         VALUES ($1,$2,'standard',$3,42)",
        vec![season.into(), me.into(), vec![0x77u8; 32].into()]).await;
}

/// Mine: one waiting, closed ones of every age, and enough recent ones for
/// two pages. Theirs: an old closed one and one left assigned to an old match.
async fn tickets(
    db: &DatabaseConnection,
    season: Uuid,
    me: Uuid,
    them: Uuid,
    won: Uuid,
    old: Uuid,
) {
    let insert = "INSERT INTO central_identity.ranked_tickets(account_id,request_id,season_id,format,commitment,
          queue_elo,state,match_id,created_at,expires_at)
         VALUES ($1,$2,$3,'standard',$4,1500,$5,$6,now()-$7::interval,now()-$7::interval+interval '4 minutes')";
    let rows = [
        (me, 0x11, 1, "waiting", None, "0 seconds"),
        // ended with a match two days ago: pruned
        (me, 0x11, 2, "closed", Some(old), "3 days"),
        // waiting ended two days ago, no match: pruned
        (me, 0x11, 3, "closed", None, "2 days"),
        // old, but its match ended just now: kept a day more
        (me, 0x11, 4, "closed", Some(won), "3 days"),
        // an hour old: kept (the hourly admission cap counts it)
        (me, 0x11, 5, "closed", None, "1 hour"),
        (them, 0x22, 6, "closed", None, "30 hours"),
        // never marked closed, but its match ended two days ago: pruned
        (them, 0x22, 7, "assigned", Some(old), "3 days"),
    ];
    for (account, deck, key, state, match_id, age) in rows {
        run_sql(
            db,
            insert,
            vec![
                account.into(),
                Uuid::from_bytes(v7(key)).into(),
                season.into(),
                vec![deck; 32].into(),
                state.into(),
                match_id.into(),
                age.into(),
            ],
        )
        .await;
    }
    for key in 0..EXTRA_TICKETS {
        run_sql(
            db,
            insert,
            vec![
                me.into(),
                Uuid::from_bytes(v7(100 + key)).into(),
                season.into(),
                vec![0x11u8; 32].into(),
                "closed".into(),
                Option::<Uuid>::None.into(),
                "10 minutes".into(),
            ],
        )
        .await;
    }
}

/// My season and deck rows as stored.
fn mine_standing(all: &[(Table, Vec<Entry>)], f: &Fixture, public_id: [u8; 32]) {
    let [Entry::Season { season }] = rows(all, Table::Seasons) else {
        panic!("one season holds my rows")
    };
    assert_eq!(season.season.id, f.season.into_bytes());
    assert_eq!(season.season.name, "Export era");
    assert_eq!(
        season.ratings,
        [export::Rating {
            format: Format::Standard,
            career: Record {
                games: 12,
                wins: 7,
                losses: 4,
                draws: 1
            },
            rank: rank(League::Gold, Some(2), None),
            elo: 1650,
            queue_elo: 1640,
            score_thousandths: 1_650_125,
            placement_thousandths: 1_600_500,
            allowance_thousandths: -3250,
            first_at: 1_044_057_600,
            last_at: 1_046_476_800,
        }]
    );
    assert_eq!(
        season.peaks,
        [
            export::Peak {
                track: Track::Deck,
                format: Format::Standard,
                rank: rank(League::Gold, Some(4), None)
            },
            export::Peak {
                track: Track::Player,
                format: Format::Standard,
                rank: rank(League::Gold, Some(2), None)
            },
        ]
    );
    assert_eq!(
        season.champion,
        Some(export::Champion {
            place: 42,
            player_format: None,
            deck_format: Some(Format::Standard),
            deck: Some(public_id)
        })
    );
    let [Entry::Deck { deck }] = rows(all, Table::Decks) else {
        panic!("one deck version")
    };
    assert_eq!(deck.deck, public_id);
    assert_eq!(
        deck.cards,
        [
            PoolEntry {
                card: [1; 32],
                copies: 40
            },
            PoolEntry {
                card: [2; 32],
                copies: 20
            }
        ]
    );
    assert_eq!(
        (deck.copies, deck.elo, deck.anchor, deck.initial),
        (60, 1561, Some(1500), true)
    );
    assert_eq!(deck.ratings.len(), 1);
    assert_eq!(deck.ratings[0].rank, rank(League::Champion, None, Some(42)));
    assert_eq!(deck.ratings[0].season_games, 12);
}

/// My games, queue and unlocks: my seat only, the opponent as its public ID.
fn mine_play(all: &[(Table, Vec<Entry>)], f: &Fixture, them: api::Id) {
    let [Entry::Receipt { receipt }] = rows(all, Table::Receipts) else {
        panic!("one settled game")
    };
    assert_eq!(receipt.match_id, f.won.into_bytes());
    assert_eq!(receipt.verdict, Verdict::Won);
    assert_eq!(receipt.opponent, them);
    assert_eq!((receipt.elo, receipt.delta), (1650, 14));
    assert_eq!(receipt.after, rank(League::Gold, Some(2), None));
    assert_eq!(receipt.deck.elo, 1561);
    let matches: Vec<_> = rows(all, Table::Matches)
        .iter()
        .map(|entry| match entry {
            Entry::Match { seat } => seat.clone(),
            _ => panic!("a match page holds matches"),
        })
        .collect();
    assert_eq!(matches.len(), 3);
    for seat in &matches {
        // my commitment, never theirs
        assert_eq!(seat.deck, [0x11; 32]);
    }
    let won = matches
        .iter()
        .find(|m| m.match_id == f.won.into_bytes())
        .unwrap();
    assert_eq!(
        (won.seat, won.state, won.verdict, won.claimed),
        (0, MatchState::Finished, Some(Verdict::Won), true)
    );
    assert!(won.game.is_some());
    assert_eq!(won.operators.len(), 1, "my decision, not theirs");
    assert!(won.operators[0].allowed);
    assert_eq!(
        won.operators[0].terms.as_ref().unwrap().version,
        "ops-2026-11"
    );
    let void = matches
        .iter()
        .find(|m| m.match_id == f.void.into_bytes())
        .unwrap();
    assert_eq!(
        (void.seat, void.state, void.verdict, void.claimed),
        (1, MatchState::Void, None, false)
    );
    assert!(void.operators.is_empty(), "seat 0's decision is theirs");
    assert!(matches.iter().any(|m| m.match_id == f.old.into_bytes()));
    let tickets = rows(all, Table::Tickets);
    assert_eq!(tickets.len(), 5 + usize::from(EXTRA_TICKETS), "two pages");
    let Entry::Ticket { ticket } = &tickets[0] else {
        panic!("a ticket page holds tickets")
    };
    assert_eq!((ticket.key, ticket.state), (v7(1), TicketState::Waiting));
    let unlocks: Vec<_> = rows(all, Table::Cosmetics)
        .iter()
        .map(|entry| match entry {
            Entry::Cosmetic { unlock } => unlock.rank,
            _ => panic!("a cosmetics page holds unlocks"),
        })
        .collect();
    // Paper, three each of Wood to Silver, and Gold IV to II: the player peak's.
    assert_eq!(unlocks.len(), 16);
    assert_eq!(unlocks[0], rank(League::Paper, None, None));
    assert_eq!(unlocks[15], rank(League::Gold, Some(4), None));
}

/// Nothing private of `other` in `file`: its commitment, cards, deck
/// version, model values, consent and opponent evidence have no place.
fn nothing_of(file: &str, other_commitment: u8, other_card: u8, other_version: u8) {
    for byte in [other_commitment, other_card, other_version] {
        let pattern = json(&[byte; 32]);
        assert!(
            !file.contains(&pattern[1..pattern.len() - 1]),
            "{byte:#x} leaked"
        );
    }
    for word in [
        "-8642",
        "elo_gap",
        "established",
        "alias",
        "seed",
        "evidence",
    ] {
        assert!(!file.contains(word), "{word} leaked");
    }
}

pub(super) async fn run(store: &AccountStore, db: &DatabaseConnection, keys: &ServerKeys) {
    let a = enroll(store, keys, "ranked-export-one", "rankedexportone").await;
    let b = enroll(store, keys, "ranked-export-two", "rankedexporttwo").await;
    let c = enroll(store, keys, "ranked-export-three", "rankedexportthree").await;
    let mine_token = login(store, keys, a).await;
    let theirs = login(store, keys, b).await;
    // Never restored: no ranked row can exist, and the download still completes.
    let stranger = login(store, keys, c).await;
    let cookie = Token::generate().unwrap();
    let credential = store.credential(a).await.unwrap().unwrap();
    store
        .issue_session(&credential, Session::Browser(&cookie))
        .await
        .unwrap();
    let (me, them) = (Uuid::from_bytes(a.account), Uuid::from_bytes(b.account));
    let f = seed(db, me, them).await;
    let public_id: [u8; 32] = one(
        db,
        "SELECT public_id FROM central_identity.ranked_deck_versions WHERE account_id=$1",
        vec![me.into()],
    )
    .await
    .try_get::<Vec<u8>>("", "public_id")
    .unwrap()
    .try_into()
    .unwrap();

    // The native session and the website's cookie read the same rows.
    let native = download(store, (&mine_token).into()).await;
    let browser = download(store, Session::Browser(&cookie)).await;
    assert_eq!(native, browser);
    mine_standing(&native, &f, public_id);
    mine_play(&native, &f, b.account);
    let file = json(&native.iter().map(|(_, rows)| rows).collect::<Vec<_>>());
    nothing_of(&file, 0x22, 0x99, 0x88);
    // The opponent appears once: as the receipt's public account ID.
    let id = json(&b.account);
    assert_eq!(file.matches(&id[1..id.len() - 1]).count(), 1);

    // Their session reads theirs: my side of the same game stays mine.
    let other = download(store, (&theirs).into()).await;
    let other_file = json(&other.iter().map(|(_, rows)| rows).collect::<Vec<_>>());
    nothing_of(&other_file, 0x11, 0x01, 0x77);
    let [Entry::Receipt { receipt }] = rows(&other, Table::Receipts) else {
        panic!("their receipt")
    };
    assert_eq!(
        (receipt.verdict, receipt.opponent),
        (Verdict::Lost, a.account)
    );
    assert_eq!(rows(&other, Table::Tickets).len(), 2);
    assert!(
        rows(&other, Table::Matches)
            .iter()
            .all(|entry| matches!(entry,
        Entry::Match { seat } if seat.deck == [0x22; 32]))
    );

    // A third account: every dataset empty and complete, never another's rows.
    for (_, rows) in download(store, (&stranger).into()).await {
        assert_eq!(rows, Vec::new());
    }
    refusals(store, &mine_token).await;

    sweep(db, f.season, Uuid::from_bytes(c.account)).await;
    let tickets = download(store, (&mine_token).into()).await;
    assert_eq!(
        rows(&tickets, Table::Tickets).len(),
        3 + usize::from(EXTRA_TICKETS)
    );
    // The receipt outlives its ticket.
    assert_eq!(rows(&tickets, Table::Receipts).len(), 1);

    run_sql(
        db,
        "DELETE FROM central_identity.accounts WHERE id IN ($1,$2,$3)",
        vec![me.into(), them.into(), Uuid::from_bytes(c.account).into()],
    )
    .await;
    run_sql(
        db,
        "DELETE FROM central_identity.ranked_seasons WHERE id=$1",
        vec![f.season.into()],
    )
    .await;
}

/// The request names no account: without a live session there is nothing,
/// and a key of another dataset, or not a `UUIDv7`, is refused.
async fn refusals(store: &AccountStore, mine_token: &Token) {
    let unknown = Token::generate().unwrap();
    for session in [None, Some((&unknown).into())] {
        assert_eq!(
            page(store, session, Table::Receipts, None)
                .await
                .unwrap_err(),
            Error::Rejected
        );
    }
    for (table, key) in [
        (Table::Seasons, Key::Deck([1; 32])),
        (Table::Receipts, Key::Id([1; 16])),
        (Table::Decks, Key::Id(v7(1))),
        (
            Table::Cosmetics,
            Key::Step {
                league: 11,
                division: 0,
            },
        ),
    ] {
        assert_eq!(
            page(store, Some(mine_token.into()), table, Some(key))
                .await
                .unwrap_err(),
            Error::Rejected
        );
    }
}

/// The operator's daily sweep deletes tickets a day after their waiting or
/// their match ended, marked closed or not; younger ones and the fresh
/// waiting one stay.
async fn sweep(db: &DatabaseConnection, season: Uuid, stranger: Uuid) {
    // Waiting, never marked closed, its four minutes two days past: pruned.
    run_sql(
        db,
        "INSERT INTO central_identity.ranked_tickets(account_id,request_id,season_id,format,commitment,
           queue_elo,created_at,expires_at)
         VALUES ($1,$2,$3,'standard',$4,1500,now()-interval '2 days',now()-interval '2 days'+interval '4 minutes')",
        vec![stranger.into(), Uuid::from_bytes(v7(8)).into(), season.into(), vec![0x55u8; 32].into()],
    )
    .await;
    let left = |key: u8| {
        one(
            db,
            "SELECT count(*)::bigint AS n FROM central_identity.ranked_tickets WHERE request_id=$1",
            vec![Uuid::from_bytes(v7(key)).into()],
        )
    };
    let swept = central_postgres::sweep(db).await.unwrap();
    assert_eq!(swept.tickets, 5, "mine two, theirs two, the stranger's one");
    for (key, kept) in [
        (1, true),
        (2, false),
        (3, false),
        (4, true),
        (5, true),
        (6, false),
        (7, false),
        (8, false),
    ] {
        assert_eq!(
            left(key).await.try_get::<i64>("", "n").unwrap(),
            i64::from(kept),
            "ticket {key}"
        );
    }
    assert_eq!(central_postgres::sweep(db).await.unwrap().tickets, 0);
}
