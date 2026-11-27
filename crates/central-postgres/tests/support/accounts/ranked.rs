//! Two real sessions and a registered game host traverse the same store calls as native ranked.
use super::{AccountStore, DatabaseConnection, Error, ServerKeys, Token, enroll, login};
use central_api::{
    gateway::GatewayMetadata,
    ranked::{self as api, Format, HostRequest, HostResponse, Outcome, QueueState, Request},
};
use central_postgres::RankedHostCredential::{Official, Registered};
use sea_orm::{ConnectionTrait, DbBackend, Statement, prelude::Uuid};

mod ai;
mod ai_operator;
mod ai_runner;
mod boards;
mod carousel;
mod credentials;
mod export;
mod grants;
mod replicas;
mod scoring;

const ORIGIN: &str = "https://ranked-test.example";

fn id(number: u8) -> api::Id {
    let mut bytes = [0; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes[15] = number;
    bytes
}

async fn queue(store: &AccountStore, token: &Token, request: Request) -> Result<QueueState, Error> {
    match store.ranked(Some(token.into()), request).await? {
        api::Response::Queue { state }
        | api::Response::Replica {
            response: api::replica::SeatResponse::Queue { state },
        } => Ok(state),
        _ => panic!("ranked queue returned a public response"),
    }
}

fn join(key: u8, deck: u8) -> Request {
    Request::Replicate {
        request: api::replica::SeatRequest::Join {
            key: id(key),
            format: Format::Standard,
            deck: [deck; 32],
            ai: api::ai::AiOpponents::None,
        },
    }
}

async fn play(store: &AccountStore, tokens: [&Token; 2], seed: u8) -> api::Assignment {
    assert!(matches!(
        queue(store, tokens[0], join(seed, 1)).await.unwrap(),
        QueueState::Waiting { .. }
    ));
    let result = queue(store, tokens[1], join(seed + 1, 2)).await.unwrap();
    let QueueState::Assigned { assignment, .. } = result else {
        panic!("the second arrival should reserve a table")
    };
    assert_eq!(assignment.origin, ORIGIN);
    assert_eq!(assignment.deck, [2; 32]);
    for token in tokens {
        replicas::privacy::approve_all(store, token, assignment.match_id).await;
    }
    assignment
}

async fn verify(db: &DatabaseConnection, host: api::Id, enabled: bool) {
    db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO central_identity.gateway_approvals (gateway_id,enabled,ranked_verified) VALUES($1,true,$2)
         ON CONFLICT(gateway_id) DO UPDATE SET ranked_verified=EXCLUDED.ranked_verified",
        vec![Uuid::from_bytes(host).into(),enabled.into()])).await.unwrap();
}

async fn rejected_claims(
    store: &AccountStore,
    credential: &Token,
    matched: api::Id,
    seats: [(api::Id, [u8; 16]); 2],
) {
    let operated = |request| store.ranked_host(Registered(credential), request);
    let aliases = seats.map(|(alias, _)| alias);
    assert_eq!(
        operated(HostRequest::Claim {
            match_id: matched,
            account: aliases[0],
            proof: api::SeatProof::from_bytes(seats[0].1),
            deck: [2; 32]
        })
        .await
        .unwrap_err(),
        Error::Rejected
    );
    for invalid in [seats[1].1, id(80)] {
        let request = HostRequest::Claim {
            match_id: matched,
            account: aliases[0],
            deck: [1; 32],
            proof: api::SeatProof::from_bytes(invalid),
        };
        assert_eq!(operated(request).await.unwrap_err(), Error::Rejected);
    }
}

async fn game(
    store: &AccountStore,
    db: &DatabaseConnection,
    credential: &Token,
    denied: &[Token],
    accounts: [api::Id; 2],
    tokens: [&Token; 2],
) {
    let assigned = play(store, tokens, 11).await;
    let operated = |request| store.ranked_host(Registered(credential), request);
    let matched = assigned.match_id;
    let seats = [
        grants::assigned(store, tokens[0], matched).await,
        grants::alias(store, tokens[1], matched).await,
    ];
    let aliases = seats.map(|(alias, _)| alias);
    assert_ne!(aliases[0], aliases[1]);
    for alias in aliases {
        assert!(!accounts.contains(&alias));
    }
    let claim = |n: usize| HostRequest::Claim {
        match_id: matched,
        account: aliases[n],
        proof: api::SeatProof::from_bytes(seats[n].1),
        deck: [u8::try_from(n + 1).unwrap(); 32],
    };
    rejected_claims(store, credential, matched, seats).await;
    let HostResponse::Assignment { assignment } = operated(claim(0)).await.unwrap() else {
        panic!("host expects both bound seats")
    };
    assert!(matches!(
        queue(store, tokens[0], Request::Cancel { key: id(11) })
            .await
            .unwrap(),
        QueueState::Assigned { .. }
    ));
    let pools = [
        vec![api::PoolEntry {
            card: [1; 32],
            copies: 60,
        }],
        vec![api::PoolEntry {
            card: [2; 32],
            copies: 60,
        }],
    ];
    let start = HostRequest::Start {
        match_id: matched,
        game: id(21),
        pools: pools.clone(),
    };
    assert_eq!(operated(start.clone()).await.unwrap_err(), Error::Conflict);
    operated(claim(1)).await.unwrap();
    let bad = HostRequest::Start {
        match_id: matched,
        game: id(21),
        pools: [Vec::new(), pools[1].clone()],
    };
    assert_eq!(operated(bad).await.unwrap_err(), Error::Rejected);
    assert_eq!(operated(start.clone()).await.unwrap(), HostResponse::Done);
    assert_eq!(operated(start.clone()).await.unwrap(), HostResponse::Done);
    let finish = HostRequest::Finish {
        match_id: matched,
        game: id(21),
        outcome: Outcome::FirstWins,
    };
    for token in denied {
        assert_eq!(
            store
                .ranked_host(Registered(token), finish.clone())
                .await
                .unwrap_err(),
            Error::Rejected
        );
    }
    let (first, again) = tokio::join!(operated(finish.clone()), operated(finish));
    assert_eq!(first.unwrap(), HostResponse::Done);
    assert_eq!(again.unwrap(), HostResponse::Done);
    compacted(store, db, credential, start).await;
    assert_eq!(
        operated(HostRequest::Finish {
            match_id: matched,
            game: id(21),
            outcome: Outcome::SecondWins
        })
        .await
        .unwrap_err(),
        Error::Conflict
    );
    receipts(store, tokens, aliases, &assignment).await;
}

async fn compacted(
    store: &AccountStore,
    db: &DatabaseConnection,
    credential: &Token,
    mut retry: HostRequest,
) {
    assert_eq!(
        store
            .ranked_host(Registered(credential), retry.clone())
            .await
            .unwrap(),
        HostResponse::Done
    );
    let HostRequest::Start {
        match_id, pools, ..
    } = &mut retry
    else {
        panic!("expected a start request")
    };
    let matched = Uuid::from_bytes(*match_id);
    pools[0][0].copies += 1;
    assert_eq!(
        store
            .ranked_host(Registered(credential), retry)
            .await
            .unwrap_err(),
        Error::Conflict
    );
    let persisted=db.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT deck_a IS NULL AND deck_b IS NULL AND octet_length(start_fingerprint)=32 AS compact
         FROM central_identity.ranked_matches WHERE id=$1",vec![matched.into()])).await.unwrap().unwrap();
    assert!(persisted.try_get::<bool>("", "compact").unwrap());
}

async fn receipts(
    store: &AccountStore,
    tokens: [&Token; 2],
    accounts: [api::Id; 2],
    assignment: &api::HostAssignment,
) {
    let mut deltas = [0; 2];
    for index in 0..2 {
        let QueueState::Finished {
            delta,
            seat,
            match_id,
            ..
        } = queue(store, tokens[index], Request::Status).await.unwrap()
        else {
            panic!("expected a receipt")
        };
        assert_eq!(assignment.match_id, match_id);
        assert_eq!(accounts[index], assignment.accounts[usize::from(seat)]);
        assert_eq!(seat == 0, delta > 0);
        deltas[index] = delta;
    }
    assert_ne!(deltas[0].signum(), deltas[1].signum());
    grants::ended(store, tokens[0]).await;
}

async fn setup_gateway(
    store: &AccountStore,
    db: &DatabaseConnection,
    alice: &Token,
) -> (api::Id, Token, Vec<Token>) {
    let meta = GatewayMetadata {
        name: "Ranked fixture".into(),
        version: "1".into(),
        website: format!("{ORIGIN}/status"),
    };
    let full = Token::generate().unwrap();
    let heartbeat = Token::generate().unwrap();
    let host = store
        .register_gateway(alice, ORIGIN, &meta, &full, &heartbeat, false)
        .await
        .unwrap();
    let (credential, mut denied) = credentials::named(db, host).await;
    denied.push(heartbeat);
    let ready = || store.ranked_host(Registered(&credential), HostRequest::Ready { capacity: 1 });
    verify(db, host, false).await;
    assert_eq!(ready().await.unwrap_err(), Error::Rejected);
    verify(db, host, true).await;
    denied.push(credentials::revocation_in_flight(store, db, host).await);
    assert_eq!(
        store
            .ranked_host(Official(ORIGIN), HostRequest::Ready { capacity: 1 })
            .await
            .unwrap_err(),
        Error::Rejected
    );
    assert_eq!(ready().await.unwrap(), HostResponse::Ready);
    assert_eq!(
        store.authenticate_gateway(&credential).await.unwrap_err(),
        Error::Rejected
    );
    assert!(
        store
            .ranked_host(Registered(&full), HostRequest::Ready { capacity: 1 })
            .await
            .is_ok()
    );
    for token in &denied {
        assert_eq!(
            store
                .ranked_host(Registered(token), HostRequest::Ready { capacity: 1 })
                .await
                .unwrap_err(),
            Error::Rejected
        );
    }
    (host, credential, denied)
}

pub(super) async fn run(store: &AccountStore, db: &DatabaseConnection, keys: &ServerKeys) {
    let a = enroll(store, keys, "ranked-fixture-one", "rankedone").await;
    let b = enroll(store, keys, "ranked-fixture-two", "rankedtwo").await;
    let alice = login(store, keys, a).await;
    store.confirm_restore(&alice, 1).await.unwrap();
    let bob = login(store, keys, b).await;
    assert_eq!(
        queue(store, &bob, join(1, 1)).await.unwrap_err(),
        Error::Rejected
    );
    store.confirm_restore(&bob, 1).await.unwrap();
    assert_eq!(
        queue(store, &bob, join(1, 1)).await.unwrap_err(),
        Error::Rejected
    );
    store.close_legacy("ranked-fixture-one", a).await.unwrap();
    store.close_legacy("ranked-fixture-two", b).await.unwrap();
    boards::own_placements(store, &alice, a.account).await;
    let (host, credential, denied) = setup_gateway(store, db, &alice).await;
    admission(store, &alice, &bob).await;
    available(store, &credential, &alice, &bob).await;
    game(
        store,
        db,
        &credential,
        &denied,
        [a.account, b.account],
        [&alice, &bob],
    )
    .await;
    expiry(store, db, host, &credential, &alice, &bob).await;
    scoring::run(
        store,
        db,
        [(a.account, &alice), (b.account, &bob)],
        host,
        &credential,
        ORIGIN,
    )
    .await;
    store
        .ranked_host(Registered(&credential), HostRequest::Ready { capacity: 0 })
        .await
        .unwrap();
    replicas::run(store, db, keys, [(a.account, &alice), (b.account, &bob)]).await;
    boards::run(store, db).await;
    export::run(store, db, keys).await;
    // Removing our two fixture accounts also removes the gateway owned by the first,
    // its approval and every account-scoped ticket. Other suites share this instance.
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "DELETE FROM central_identity.accounts WHERE id IN ($1,$2)",
        vec![
            Uuid::from_bytes(a.account).into(),
            Uuid::from_bytes(b.account).into(),
        ],
    ))
    .await
    .unwrap();
    ai::run(store, db, keys).await;
    ai_runner::run(store, db, keys).await;
    ai_operator::run(store, db, keys).await;
    carousel::run(store, db, keys).await;
}

async fn admission(store: &AccountStore, alice: &Token, bob: &Token) {
    assert_eq!(
        queue(
            store,
            alice,
            Request::Join {
                key: id(1),
                format: Format::Standard,
                deck: [1; 32]
            }
        )
        .await,
        Err(Error::UpdateRequired)
    );
    assert_eq!(
        store.ranked(None, join(1, 1)).await.unwrap_err(),
        Error::Rejected
    );
    let (left, right) = tokio::join!(
        queue(store, alice, join(1, 1)),
        queue(store, alice, join(1, 1))
    );
    assert_eq!(left.unwrap(), right.unwrap());
    assert_eq!(
        queue(store, alice, join(1, 9)).await.unwrap_err(),
        Error::Conflict
    );
    assert_eq!(
        queue(store, alice, join(2, 1)).await.unwrap_err(),
        Error::Conflict
    );
    assert_eq!(
        queue(store, bob, Request::Cancel { key: id(1) })
            .await
            .unwrap(),
        QueueState::Idle
    );
    assert!(matches!(
        queue(store, alice, Request::Status).await.unwrap(),
        QueueState::Waiting { .. }
    ));
    assert_eq!(
        queue(store, alice, Request::Cancel { key: id(1) })
            .await
            .unwrap(),
        QueueState::Idle
    );
    assert_eq!(
        queue(store, alice, join(1, 1)).await.unwrap(),
        QueueState::Idle
    );
}

fn ready(capacity: u8) -> HostRequest {
    HostRequest::Replicate {
        request: api::replica::HostRequest::Ready {
            capacity,
            compatibility: [9; 32],
        },
    }
}

async fn available(store: &AccountStore, credential: &Token, alice: &Token, bob: &Token) {
    store
        .ranked_host(Registered(credential), ready(0))
        .await
        .unwrap();
    for (token, key) in [(alice, 3), (bob, 4)] {
        assert!(matches!(
            queue(store, token, join(key, key)).await.unwrap(),
            QueueState::Waiting { .. }
        ));
    }
    store
        .ranked_host(Registered(credential), ready(1))
        .await
        .unwrap();
    let (left, right) = tokio::join!(
        queue(store, alice, Request::Status),
        queue(store, bob, Request::Status)
    );
    assert!(matches!(
        left.unwrap(),
        QueueState::Assigned { .. } | QueueState::Waiting { .. }
    ));
    assert!(matches!(
        right.unwrap(),
        QueueState::Assigned { .. } | QueueState::Waiting { .. }
    ));
    let QueueState::Assigned { assignment, .. } =
        queue(store, alice, Request::Status).await.unwrap()
    else {
        panic!("the newly available host must receive both queued seats")
    };
    let release = HostRequest::Void {
        match_id: assignment.match_id,
    };
    for _ in 0..2 {
        assert_eq!(
            store
                .ranked_host(Registered(credential), release.clone())
                .await
                .unwrap(),
            HostResponse::Done
        );
    }
    let declined = play(store, [alice, bob], 6).await;
    assert_eq!(
        queue(store, alice, Request::Cancel { key: id(6) })
            .await
            .unwrap(),
        QueueState::Idle
    );
    assert_eq!(
        queue(store, bob, Request::Status).await.unwrap(),
        QueueState::Idle
    );
    assert_eq!(
        store
            .ranked_host(
                Registered(credential),
                HostRequest::Void {
                    match_id: declined.match_id
                }
            )
            .await
            .unwrap(),
        HostResponse::Done
    );
}

async fn expiry(
    store: &AccountStore,
    db: &DatabaseConnection,
    host: api::Id,
    credential: &Token,
    alice: &Token,
    bob: &Token,
) {
    let revoked = play(store, [alice, bob], 31).await;
    grants::next_match(store, alice, revoked.match_id).await;
    verify(db, host, false).await;
    verify(db, host, true).await;
    store
        .ranked_host(Registered(credential), ready(1))
        .await
        .unwrap();
    assert_eq!(
        store
            .ranked_host(
                Registered(credential),
                HostRequest::Void {
                    match_id: revoked.match_id
                }
            )
            .await
            .unwrap_err(),
        Error::Rejected
    );
    assert_eq!(
        queue(store, alice, Request::Status).await.unwrap(),
        QueueState::Idle
    );
    let timed = play(store, [alice, bob], 41).await;
    db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "UPDATE central_identity.ranked_matches SET lease_until=now()-interval '1 second' WHERE id=$1",
        vec![Uuid::from_bytes(timed.match_id).into()])).await.unwrap();
    assert_eq!(
        queue(store, alice, Request::Status).await.unwrap(),
        QueueState::Idle
    );
}
