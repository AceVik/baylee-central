//! The carousel (`docs/ai-players.md` §7): a runner asks for a match between
//! its AI and another AI offering the same format; Central makes one, with
//! both seats AIs, only while no person waits and every cap has room. Each
//! refusal is followed by the same request once its one cause is gone, so
//! the refusal is shown to come from that cause. A settled carousel match
//! scores, repeat damping included, as a match between people does.
use super::ai::{
    ai, count, desk, forget, person, runner, runner_token, settings as ai_settings, sql,
};
use super::{
    AccountStore, DatabaseConnection, Error, ORIGIN, ServerKeys, Token, api, id, queue, ready,
};
use super::{replicas, setup_gateway};
use central_api::admin::{AiPlayerSettings, AiSettings};
use central_api::ranked::ai::{
    AiKind, AiOpponents, MatchResult, Offer, Request as AiRequest, Response as AiResponse, Unseated,
};
use central_api::ranked::{Format, HostRequest, Outcome, PoolEntry, QueueState, Request};
use central_api::wire::GatewayPurpose;
use central_postgres::{RankedHostCredential::Registered, appoint_admin};
use sea_orm::{ConnectionTrait, DbBackend, Statement, TransactionTrait, prelude::Uuid};

/// No other suite queues in this format, so nobody else's ticket waits in it.
const FORMAT: Format = Format::Highlander;

/// One AI of the fixture: its account, its kind, its runner session and
/// its deck.
struct Bot {
    account: [u8; 16],
    kind: AiKind,
    session: Token,
    deck: [u8; 32],
}

impl Bot {
    fn id(&self) -> Uuid {
        Uuid::from_bytes(self.account)
    }
}

/// The four AIs: three house levels and a language model with a daily cap
/// of one game.
struct Bots {
    a: Bot,
    b: Bot,
    c: Bot,
    m: Bot,
}

impl Bots {
    fn all(&self) -> [&Bot; 4] {
        [&self.a, &self.b, &self.c, &self.m]
    }

    fn by(&self, account: Uuid) -> &Bot {
        self.all()
            .into_iter()
            .find(|bot| bot.id() == account)
            .unwrap()
    }
}

/// The settings for every AI: the carousel's two caps, `global_seats` and
/// `host_reserve`, the rest at their defaults.
async fn set(db: &DatabaseConnection, games: i16, daily: i32, seats: i16, reserve: i16) {
    sql(
        db,
        "UPDATE central_identity.ai_settings SET fill_in_after = 60, drain_minutes = 120,
           carousel_games = $1, carousel_daily = $2, global_seats = $3, host_reserve = $4",
        vec![games.into(), daily.into(), seats.into(), reserve.into()],
    )
    .await;
}

async fn offer(store: &AccountStore, bot: &Bot) {
    let AiResponse::Offered { until: Some(_) } = ai(
        store,
        &bot.session,
        AiRequest::Offer {
            offers: vec![Offer {
                format: FORMAT,
                deck: bot.deck,
            }],
        },
    )
    .await
    .unwrap() else {
        panic!("the offer should stand");
    };
}

/// The carousel's answer to `bot`.
async fn ask(store: &AccountStore, bot: &Bot) -> AiResponse {
    ai(store, &bot.session, AiRequest::Carousel { format: FORMAT })
        .await
        .unwrap()
}

/// The carousel refuses `bot` for `reason`, and makes no match.
async fn refused(store: &AccountStore, db: &DatabaseConnection, bot: &Bot, reason: Unseated) {
    let before = carousels(db).await;
    assert_eq!(
        ask(store, bot).await,
        AiResponse::Unseated { reason },
        "{reason:?}"
    );
    assert_eq!(carousels(db).await, before, "{reason:?}");
}

/// The carousel seats `bot` now: its seat, checked against the match it
/// names, and its partner.
async fn seated(
    store: &AccountStore,
    db: &DatabaseConnection,
    bots: &Bots,
    bot: &Bot,
) -> (api::Assignment, Uuid) {
    let AiResponse::Seated { assignment } = ask(store, bot).await else {
        panic!("the carousel should seat it");
    };
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT m.account_a, m.account_b, m.commitment_a, m.commitment_b, m.state,
               am.arranged, am.ai_seats,
               EXISTS (SELECT 1 FROM central_identity.ranked_replicas r WHERE r.match_id = m.id)
                 AS protected,
               EXISTS (SELECT 1 FROM central_identity.ranked_tickets t WHERE t.match_id = m.id)
                 AS ticketed
             FROM central_identity.ranked_matches m
             JOIN central_identity.ai_matches am ON am.match_id = m.id WHERE m.id = $1",
            vec![Uuid::from_bytes(assignment.match_id).into()],
        ))
        .await
        .unwrap()
        .unwrap();
    let get = |name: &str| row.try_get::<Uuid>("", name).unwrap();
    let deck = |name: &str| row.try_get::<Vec<u8>>("", name).unwrap();
    let (first, second) = (get("account_a"), get("account_b"));
    // Both seats are AIs with the decks they offered; the match is the
    // carousel's, reserved, never protected, and no ticket holds it.
    assert_eq!(row.try_get::<String>("", "arranged").unwrap(), "carousel");
    assert_eq!(row.try_get::<i16>("", "ai_seats").unwrap(), 3);
    assert_eq!(row.try_get::<String>("", "state").unwrap(), "reserved");
    assert!(!row.try_get::<bool>("", "protected").unwrap());
    assert!(!row.try_get::<bool>("", "ticketed").unwrap());
    assert_eq!(deck("commitment_a"), bots.by(first).deck.to_vec());
    assert_eq!(deck("commitment_b"), bots.by(second).deck.to_vec());
    let partner = if first == bot.id() { second } else { first };
    assert!(first == bot.id() || second == bot.id());
    assert_ne!(partner, bot.id());
    assert_eq!(
        (
            assignment.account,
            assignment.seat,
            assignment.format,
            assignment.deck
        ),
        (bot.account, u8::from(second == bot.id()), FORMAT, bot.deck)
    );
    assert_eq!(assignment.origin, ORIGIN);
    assert_eq!(
        assignment.opponent_ai.as_ref().map(|badge| badge.kind),
        Some(bots.by(partner).kind)
    );
    // The partner's runner finds the same match in its work.
    let AiResponse::Work { assignments, .. } =
        ai(store, &bots.by(partner).session, AiRequest::Work)
            .await
            .unwrap()
    else {
        panic!("work");
    };
    assert!(
        assignments
            .iter()
            .any(|a| a.match_id == assignment.match_id)
    );
    (assignment, partner)
}

/// The carousel's matches, in every state.
async fn carousels(db: &DatabaseConnection) -> i64 {
    count(
        db,
        "SELECT count(*) AS n FROM central_identity.ai_matches WHERE arranged = 'carousel'",
        vec![],
    )
    .await
}

/// The carousel's matches that hold a slot.
async fn live(db: &DatabaseConnection) -> i64 {
    count(
        db,
        "SELECT count(*) AS n FROM central_identity.ai_matches am
         JOIN central_identity.ranked_matches m ON m.id = am.match_id
         WHERE am.arranged = 'carousel' AND m.state IN ('reserved', 'playing')
           AND m.lease_until > pg_catalog.clock_timestamp()",
        vec![],
    )
    .await
}

async fn void(store: &AccountStore, host: &Token, match_id: api::Id) {
    store
        .ranked_host(Registered(host), HostRequest::Void { match_id })
        .await
        .unwrap();
}

async fn enabled(db: &DatabaseConnection, bots: &[&Bot], on: bool) {
    let ids: Vec<Uuid> = bots.iter().map(|bot| bot.id()).collect();
    sql(
        db,
        "UPDATE central_identity.ai_players SET enabled = $2 WHERE account_id = ANY($1)",
        vec![ids.into(), on.into()],
    )
    .await;
}

async fn max_games(db: &DatabaseConnection, bots: &[&Bot], games: i16) {
    let ids: Vec<Uuid> = bots.iter().map(|bot| bot.id()).collect();
    sql(
        db,
        "UPDATE central_identity.ai_players SET max_games = $2 WHERE account_id = ANY($1)",
        vec![ids.into(), games.into()],
    )
    .await;
}

pub(super) async fn run(store: &AccountStore, db: &DatabaseConnection, keys: &ServerKeys) {
    let (owner, _, _) = person(store, keys, "cr-fixture-owner").await;
    let (_, admin_id, admin) = person(store, keys, "cr-fixture-admin").await;
    let pia = person(store, keys, "cr-fixture-pia").await;
    let ann = person(store, keys, "cr-fixture-ann").await;
    let ben = person(store, keys, "cr-fixture-ben").await;
    appoint_admin(db, "cr-fixture-admin").await.unwrap();
    let cookie = desk(store, admin).await;
    assert_eq!(
        count(
            db,
            "SELECT count(*) AS n FROM central_identity.ai_matches am
             JOIN central_identity.ranked_matches m ON m.id = am.match_id
             WHERE m.state IN ('reserved', 'playing')",
            vec![],
        )
        .await,
        0,
        "no AI match is left by an earlier suite"
    );
    let bots = creating(store, &cookie).await;
    let (host_id, host, _) = setup_gateway(store, db, &owner).await;
    store
        .ranked_host(Registered(&host), ready(8))
        .await
        .unwrap();

    let first = opening(store, db, &cookie, &bots, (&pia.0, pia.1)).await;
    capping(store, db, &host, &bots).await;
    choosing(store, db, &host, &bots).await;
    void(store, &host, first.match_id).await;
    racing(store, db, &host, &bots).await;
    scoring(
        store,
        db,
        (host_id, &host),
        &bots,
        [(&ann.0, ann.1), (&ben.0, ben.1)],
    )
    .await;

    // Clean up: this suite shares the instance. Its administrator's events
    // go too, as the administration's suite pages through every event.
    set(db, 0, 0, 8, 2).await;
    let accounts: Vec<[u8; 16]> = bots.all().iter().map(|bot| bot.account).collect();
    let named: Vec<Uuid> = accounts
        .iter()
        .chain([&admin_id])
        .map(|account| Uuid::from_bytes(*account))
        .collect();
    sql(
        db,
        "DELETE FROM central_identity.audit_events
         WHERE actor_account = ANY($1) OR account_id = ANY($1)",
        vec![named.into()],
    )
    .await;
    forget(db, &accounts).await;
    sql(
        db,
        "DELETE FROM central_identity.accounts WHERE username_key LIKE 'cr-fixture-%'",
        vec![],
    )
    .await;
}

/// Four AIs made by an administrator, switched on, each with a runner
/// session under one token.
async fn creating(store: &AccountStore, cookie: &Token) -> Bots {
    let mut accounts = Vec::new();
    for name in ["KI-Karussell-A", "KI-Karussell-B", "KI-Karussell-C"] {
        let (account, _) = store
            .create_ai_player(cookie, &ai_settings("house:expert", name), None)
            .await
            .unwrap();
        accounts.push(account);
    }
    let model = AiPlayerSettings {
        kind: AiKind::Model,
        profile: "model:haiku".into(),
        label: "Claude Haiku · Hoch".into(),
        max_games: 2,
        daily_games: Some(1),
        display_name: "KI-Karussell-M".into(),
    };
    let (account, _) = store.create_ai_player(cookie, &model, None).await.unwrap();
    accounts.push(account);
    for account in &accounts {
        store.set_ai_enabled(cookie, *account, true).await.unwrap();
    }
    let token = runner_token(store, cookie, &accounts).await;
    let mut bots = Vec::new();
    let made = [
        (AiKind::House, 0x41),
        (AiKind::House, 0x42),
        (AiKind::House, 0x43),
        (AiKind::Model, 0x4d),
    ];
    for (account, (kind, deck)) in accounts.into_iter().zip(made) {
        bots.push(Bot {
            account,
            kind,
            session: runner(store, &token, account).await,
            deck: [deck; 32],
        });
    }
    let [a, b, c, m] = <[Bot; 4]>::try_from(bots).ok().unwrap();
    Bots { a, b, c, m }
}

/// Off by default; asked without an offer, the runner offers first; with
/// one offer alone there is nobody to pair; a person who waits comes first,
/// and once she has gone the two offers make a match. The administration
/// takes the calibration's settings.
async fn opening(
    store: &AccountStore,
    db: &DatabaseConnection,
    cookie: &Token,
    bots: &Bots,
    (pia, pia_id): (&Token, [u8; 16]),
) -> api::Assignment {
    // A person's session has no carousel, as it has no AI requests.
    assert_eq!(
        ai(store, pia, AiRequest::Carousel { format: FORMAT }).await,
        Err(Error::Rejected)
    );
    offer(store, &bots.a).await;
    // As installed: no carousel game at all.
    set(db, 0, 0, 8, 2).await;
    refused(store, db, &bots.a, Unseated::Off).await;
    // The calibration's settings pass the administration's bounds.
    let calibration = AiSettings {
        fill_in: false,
        fill_in_after: 600,
        drain_minutes: 120,
        global_seats: 16,
        host_reserve: 2,
        carousel_games: 8,
        carousel_daily: 10_000,
    };
    store
        .set_ai_settings(cookie, Some(&calibration))
        .await
        .unwrap();
    for wrong in [
        AiSettings {
            carousel_games: 9,
            ..calibration
        },
        AiSettings {
            carousel_daily: 10_001,
            ..calibration
        },
    ] {
        assert!(store.set_ai_settings(cookie, Some(&wrong)).await.is_err());
    }
    set(db, 1, 1000, 8, 2).await;
    refused(store, db, &bots.b, Unseated::Offer).await;
    refused(store, db, &bots.a, Unseated::Later).await;
    offer(store, &bots.b).await;
    // People first: while a person's ticket waits in the format, even one
    // that refused AIs, no carousel match is made.
    let QueueState::Waiting { .. } = queue(
        store,
        pia,
        Request::Replicate {
            request: api::replica::SeatRequest::Join {
                key: id(240),
                format: FORMAT,
                deck: [24; 32],
                ai: AiOpponents::None,
            },
        },
    )
    .await
    .unwrap() else {
        panic!("pia waits");
    };
    refused(store, db, &bots.a, Unseated::Later).await;
    refused(store, db, &bots.b, Unseated::Later).await;
    queue(store, pia, Request::Cancel { key: id(240) })
        .await
        .unwrap();
    assert_eq!(
        count(
            db,
            "SELECT count(*) AS n FROM central_identity.ranked_tickets
             WHERE account_id = $1 AND state = 'waiting'",
            vec![Uuid::from_bytes(pia_id).into()],
        )
        .await,
        0
    );
    let (assignment, partner) = seated(store, db, bots, &bots.a).await;
    assert_eq!(partner, bots.b.id());
    assignment
}

/// With A and B in one match: the carousel's live cap, `global_seats` with
/// one seat left and a host at its reserve each refuse, and the next match
/// comes once that one cause is gone.
async fn capping(store: &AccountStore, db: &DatabaseConnection, host: &Token, bots: &Bots) {
    offer(store, &bots.c).await;
    assert_eq!(live(db).await, 1);
    // `carousel_games` reached, though C and both partners have a free slot.
    refused(store, db, &bots.c, Unseated::Later).await;
    // `global_seats`: two of three are taken, one is no room for two AIs.
    set(db, 2, 1000, 3, 2).await;
    refused(store, db, &bots.c, Unseated::Later).await;
    // One seat is no carousel at all.
    set(db, 2, 1000, 1, 2).await;
    refused(store, db, &bots.c, Unseated::Off).await;
    set(db, 2, 1000, 4, 2).await;
    let (second, _) = seated(store, db, bots, &bots.c).await;
    void(store, host, second.match_id).await;
    // The host: capacity 3, one live match, a reserve of 2 leaves no slot.
    set(db, 2, 1000, 8, 2).await;
    store.ranked_host(Registered(host), ready(3)).await.unwrap();
    refused(store, db, &bots.c, Unseated::Later).await;
    set(db, 2, 1000, 8, 1).await;
    let (third, _) = seated(store, db, bots, &bots.c).await;
    void(store, host, third.match_id).await;
    set(db, 2, 1000, 8, 2).await;
    store.ranked_host(Registered(host), ready(8)).await.unwrap();
}

/// The partner is one that passes every hard rule: never one at its
/// `max_games`, at its `daily_games` or switched off, even with its offer
/// standing; the AI asking is held to the same rules.
async fn choosing(store: &AccountStore, db: &DatabaseConnection, host: &Token, bots: &Bots) {
    // A sits in the first match: with a cap of one it is full, so C gets B.
    max_games(db, &[&bots.a], 1).await;
    refused(store, db, &bots.a, Unseated::Later).await;
    let (seat, partner) = seated(store, db, bots, &bots.c).await;
    assert_eq!(partner, bots.b.id());
    void(store, host, seat.match_id).await;
    // While a transaction holds B's account row for update, as a runner's
    // exchange does before it ends an older session (and that session's
    // offer with it), C's one partner is skipped, never waited for.
    let held = db.begin().await.unwrap();
    held.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT 1 FROM central_identity.accounts WHERE id = $1 FOR UPDATE",
        vec![bots.b.id().into()],
    ))
    .await
    .unwrap();
    refused(store, db, &bots.c, Unseated::Later).await;
    held.rollback().await.unwrap();
    let (seat, partner) = seated(store, db, bots, &bots.c).await;
    assert_eq!(partner, bots.b.id());
    void(store, host, seat.match_id).await;
    max_games(db, &[&bots.a], 2).await;

    // The model plays one game a day, a carousel game among them.
    offer(store, &bots.m).await;
    let (played, _) = seated(store, db, bots, &bots.m).await;
    void(store, host, played.match_id).await;
    refused(store, db, &bots.m, Unseated::Tomorrow).await;
    // Switched off with their offers standing, A and B are no partners; the
    // model, at its cap, is none either: C finds nobody.
    enabled(db, &[&bots.a, &bots.b], false).await;
    refused(store, db, &bots.c, Unseated::Later).await;
    refused(store, db, &bots.a, Unseated::Off).await;
    enabled(db, &[&bots.b], true).await;
    let (seat, partner) = seated(store, db, bots, &bots.c).await;
    assert_eq!(partner, bots.b.id());
    void(store, host, seat.match_id).await;
    enabled(db, &[&bots.a], true).await;

    // `carousel_daily`: today's carousel matches reached, then one more.
    let today = i32::try_from(
        count(
            db,
            "SELECT count(*) AS n FROM central_identity.ai_matches
             WHERE arranged = 'carousel'
               AND created_at >= pg_catalog.date_trunc('day', pg_catalog.clock_timestamp(), 'UTC')",
            vec![],
        )
        .await,
    )
    .unwrap();
    set(db, 2, today, 8, 2).await;
    refused(store, db, &bots.c, Unseated::Tomorrow).await;
    set(db, 2, today + 1, 8, 2).await;
    let (seat, _) = seated(store, db, bots, &bots.c).await;
    void(store, host, seat.match_id).await;
    refused(store, db, &bots.c, Unseated::Tomorrow).await;
    set(db, 2, 1000, 8, 2).await;
}

/// Two AIs ask for each other at once for the last slot: the carousel's
/// last live match, then each one's last game. Exactly one match is made
/// each round, and neither request fails.
async fn racing(store: &AccountStore, db: &DatabaseConnection, host: &Token, bots: &Bots) {
    enabled(db, &[&bots.c, &bots.m], false).await;
    assert_eq!(live(db).await, 0);
    for round in 0..4 {
        if round < 2 {
            set(db, 1, 1000, 8, 2).await;
        } else {
            set(db, 2, 1000, 8, 2).await;
            max_games(db, &[&bots.a, &bots.b], 1).await;
        }
        let made = carousels(db).await;
        let (left, right) = tokio::join!(
            ai(
                store,
                &bots.a.session,
                AiRequest::Carousel { format: FORMAT }
            ),
            ai(
                store,
                &bots.b.session,
                AiRequest::Carousel { format: FORMAT }
            ),
        );
        let answers = [left.unwrap(), right.unwrap()];
        let seats: Vec<_> = answers
            .iter()
            .filter_map(|answer| match answer {
                AiResponse::Seated { assignment } => Some(assignment.match_id),
                _ => None,
            })
            .collect();
        assert_eq!(seats.len(), 1, "round {round}: {answers:?}");
        assert!(
            answers.contains(&AiResponse::Unseated {
                reason: Unseated::Later
            }),
            "round {round}: {answers:?}"
        );
        assert_eq!(carousels(db).await, made + 1, "round {round}");
        assert_eq!(live(db).await, 1, "round {round}");
        void(store, host, seats[0]).await;
    }
    max_games(db, &[&bots.a, &bots.b], 2).await;
}

fn pools() -> [Vec<PoolEntry>; 2] {
    [
        vec![PoolEntry {
            card: [1; 32],
            copies: 60,
        }],
        vec![PoolEntry {
            card: [2; 32],
            copies: 60,
        }],
    ]
}

/// Both runners approve the host and take their seat's grant, the host
/// claims both seats, starts the game and reports it won by `winner`.
async fn play_out(
    store: &AccountStore,
    db: &DatabaseConnection,
    host: &Token,
    bots: &Bots,
    match_id: api::Id,
    (game, winner): (u8, Uuid),
) {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT account_a, account_b FROM central_identity.ranked_matches WHERE id = $1",
            vec![Uuid::from_bytes(match_id).into()],
        ))
        .await
        .unwrap()
        .unwrap();
    let seats: [Uuid; 2] = [
        row.try_get("", "account_a").unwrap(),
        row.try_get("", "account_b").unwrap(),
    ];
    let fence = api::replica::Fence {
        match_id,
        generation: 0,
    };
    // The host learns its seats' aliases once both runners consented.
    for account in seats {
        replicas::privacy::approve_all(store, &bots.by(account).session, match_id).await;
    }
    for (index, account) in (0_u8..).zip(seats) {
        let bot = bots.by(account);
        let grant = store
            .gateway_grant_scoped(
                &bot.session,
                &[ORIGIN],
                true,
                GatewayPurpose::Ranked,
                id(game.wrapping_add(index)),
                Some(fence),
            )
            .await
            .unwrap();
        store
            .ranked_host(
                Registered(host),
                HostRequest::Claim {
                    match_id,
                    account: grant.account,
                    proof: api::SeatProof::from_bytes(grant.sid),
                    deck: bot.deck,
                },
            )
            .await
            .unwrap();
    }
    store
        .ranked_host(
            Registered(host),
            HostRequest::Start {
                match_id,
                game: id(game),
                pools: pools(),
            },
        )
        .await
        .unwrap();
    let outcome = if seats[0] == winner {
        Outcome::FirstWins
    } else {
        Outcome::SecondWins
    };
    store
        .ranked_host(
            Registered(host),
            HostRequest::Finish {
                match_id,
                game: id(game),
                outcome,
            },
        )
        .await
        .unwrap();
}

/// Every rating of these accounts in the format back at 1500, so that each
/// settlement starts from the same gap and differs by its meetings alone.
async fn level(db: &DatabaseConnection, accounts: [Uuid; 2]) {
    sql(
        db,
        "UPDATE central_identity.ranked_ratings SET score = 1500
         WHERE format = 'highlander' AND account_id = ANY($1)",
        vec![accounts.to_vec().into()],
    )
    .await;
}

async fn delta(db: &DatabaseConnection, match_id: Uuid, account: Uuid) -> i64 {
    count(
        db,
        "SELECT delta::bigint AS n FROM central_identity.ranked_receipts
         WHERE match_id = $1 AND account_id = $2",
        vec![match_id.into(), account.into()],
    )
    .await
}

/// A wins five carousel matches against B, each from level ratings, played
/// through the host as a game is. The winner's gains are people's: the same
/// five results between two people give the same, and the third and fourth
/// meeting in six hours count half, the fifth a quarter (`rules::settle`).
async fn scoring(
    store: &AccountStore,
    db: &DatabaseConnection,
    (host_id, host): (api::Id, &Token),
    bots: &Bots,
    [(_, ann), (_, ben)]: [(&Token, [u8; 16]); 2],
) {
    set(db, 1, 1000, 8, 2).await;
    let pair = [bots.a.id(), bots.b.id()];
    let mut carousel = Vec::new();
    for round in 0..5_u8 {
        let (assignment, partner) = seated(store, db, bots, &bots.a).await;
        assert_eq!(partner, bots.b.id());
        level(db, pair).await;
        play_out(
            store,
            db,
            host,
            bots,
            assignment.match_id,
            (230 + 2 * round, bots.a.id()),
        )
        .await;
        let matched = Uuid::from_bytes(assignment.match_id);
        let won = delta(db, matched, bots.a.id()).await;
        assert_eq!(delta(db, matched, bots.b.id()).await, -won);
        carousel.push(won);
    }
    let (ann, ben) = (Uuid::from_bytes(ann), Uuid::from_bytes(ben));
    let mut people = Vec::new();
    for _ in 0..5 {
        level(db, [ann, ben]).await;
        let row = db
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "INSERT INTO central_identity.ranked_matches(season_id,format,account_a,account_b,
                   commitment_a,commitment_b,claimed_a,claimed_b,deck_a,deck_b,host_id,ranked_epoch,
                   origin,state,game_id,start_fingerprint)
                 SELECT s.id,'highlander',$1,$2,decode(repeat('00',32),'hex'),decode(repeat('01',32),'hex'),
                   true,true,$3::jsonb,$3::jsonb,$4,p.ranked_epoch,$5,'playing',pg_catalog.uuidv7(),
                   decode(repeat('02',32),'hex')
                 FROM central_identity.ranked_seasons s, central_identity.gateway_approvals p
                 WHERE p.gateway_id = $4 AND s.starts_at <= now() AND (s.ends_at IS NULL OR s.ends_at > now())
                 RETURNING id, game_id",
                vec![
                    ann.into(),
                    ben.into(),
                    String::from_utf8(central_api::encode(&pools()[0]).unwrap())
                        .unwrap()
                        .into(),
                    Uuid::from_bytes(host_id).into(),
                    ORIGIN.into(),
                ],
            ))
            .await
            .unwrap()
            .unwrap();
        let matched: Uuid = row.try_get("", "id").unwrap();
        let game: Uuid = row.try_get("", "game_id").unwrap();
        store
            .ranked_host(
                Registered(host),
                HostRequest::Finish {
                    match_id: matched.into_bytes(),
                    game: game.into_bytes(),
                    outcome: Outcome::FirstWins,
                },
            )
            .await
            .unwrap();
        people.push(delta(db, matched, ann).await);
    }
    assert_eq!(carousel, people, "an AI's match scores as people's");
    assert_eq!(carousel, [12, 12, 6, 6, 3]);
    results_in_work(store, bots, &carousel).await;
    // Both AIs' ratings moved; the carousel's matches show in their records.
    assert_eq!(
        count(
            db,
            "SELECT count(*) AS n FROM central_identity.ranked_ratings
             WHERE format = 'highlander' AND account_id = ANY($1) AND games = 5",
            vec![pair.to_vec().into()],
        )
        .await,
        2
    );
}

/// Each runner reads every match's result and rating change in its work,
/// newest first, from its own seat: several in one format stay apart.
async fn results_in_work(store: &AccountStore, bots: &Bots, carousel: &[i64]) {
    for (bot, sign, result) in [
        (&bots.a, 1, MatchResult::Win),
        (&bots.b, -1, MatchResult::Loss),
    ] {
        let AiResponse::Work { finished, .. } =
            ai(store, &bot.session, AiRequest::Work).await.unwrap()
        else {
            panic!("work");
        };
        let last: Vec<_> = finished.iter().take(5).collect();
        let deltas: Vec<i64> = last.iter().rev().map(|f| i64::from(f.delta)).collect();
        assert_eq!(
            deltas,
            carousel.iter().map(|d| sign * d).collect::<Vec<_>>()
        );
        assert!(last.iter().all(|f| f.result == result
            && f.format == Format::Highlander
            && f.opponent_ai.is_some()
            && f.seat < 2));
        let ids: std::collections::BTreeSet<_> = last.iter().map(|f| f.match_id).collect();
        assert_eq!(ids.len(), 5);
    }
}
