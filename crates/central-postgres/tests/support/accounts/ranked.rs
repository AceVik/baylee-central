//! Two real sessions and a registered game host traverse the same store calls as native ranked.
use super::{AccountStore, DatabaseConnection, Error, ServerKeys, Token, enroll, login};
use central_api::{
    gateway::GatewayMetadata,
    ranked::{self as api, Format, HostRequest, HostResponse, Outcome, QueueState, Request},
};
use sea_orm::{ConnectionTrait, DbBackend, Statement, prelude::Uuid};

mod boards;
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
        api::Response::Queue { state } => Ok(state),
        _ => panic!("ranked queue returned a public response"),
    }
}

fn join(key: u8, deck: u8) -> Request {
    Request::Join {
        key: id(key),
        format: Format::Standard,
        deck: [deck; 32],
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
    assignment
}

async fn verify(db: &DatabaseConnection, host: api::Id, enabled: bool) {
    db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO central_identity.gateway_approvals (gateway_id,enabled,ranked_verified) VALUES($1,true,$2)
         ON CONFLICT(gateway_id) DO UPDATE SET ranked_verified=EXCLUDED.ranked_verified",
        vec![Uuid::from_bytes(host).into(),enabled.into()])).await.unwrap();
}

async fn game(store: &AccountStore, host: api::Id, accounts: [api::Id; 2], tokens: [&Token; 2]) {
    let assigned = play(store, tokens, 11).await;
    let operated = |request| store.ranked_host(Some(host), ORIGIN, request);
    let matched = assigned.match_id;
    let claim = |n: usize| HostRequest::Claim {
        match_id: matched,
        account: accounts[n],
        deck: [u8::try_from(n + 1).unwrap(); 32],
    };
    assert_eq!(
        operated(HostRequest::Claim {
            match_id: matched,
            account: accounts[0],
            deck: [2; 32]
        })
        .await
        .unwrap_err(),
        Error::Rejected
    );
    let HostResponse::Assignment { assignment } = operated(claim(0)).await.unwrap() else {
        panic!("host expects both bound seats")
    };
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
    assert_eq!(operated(start).await.unwrap(), HostResponse::Done);
    let finish = HostRequest::Finish {
        match_id: matched,
        game: id(21),
        outcome: Outcome::FirstWins,
    };
    let (first, again) = tokio::join!(operated(finish.clone()), operated(finish));
    assert_eq!(first.unwrap(), HostResponse::Done);
    assert_eq!(again.unwrap(), HostResponse::Done);
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
        assert_eq!(matched, match_id);
        assert_eq!(accounts[index], assignment.accounts[usize::from(seat)]);
        assert_eq!(seat == 0, delta > 0);
        deltas[index] = delta;
    }
    assert_ne!(deltas[0].signum(), deltas[1].signum());
}

pub(super) async fn run(store: &AccountStore, db: &DatabaseConnection, keys: &ServerKeys) {
    let a = enroll(store, keys, "ranked-fixture-one", "rankedone").await;
    let b = enroll(store, keys, "ranked-fixture-two", "rankedtwo").await;
    store.close_legacy("ranked-fixture-one", a).await.unwrap();
    let alice = login(store, keys, a).await;
    let bob = login(store, keys, b).await;
    let meta = GatewayMetadata {
        name: "Ranked fixture".into(),
        version: "1".into(),
        website: ORIGIN.into(),
    };
    let host = store
        .register_gateway(
            &alice,
            ORIGIN,
            &meta,
            &Token::generate().unwrap(),
            &Token::generate().unwrap(),
            false,
        )
        .await
        .unwrap();
    let ready = || store.ranked_host(Some(host), ORIGIN, HostRequest::Ready { capacity: 1 });
    verify(db, host, false).await;
    assert_eq!(ready().await.unwrap_err(), Error::Rejected);
    verify(db, host, true).await;
    assert_eq!(
        store
            .ranked_host(None, ORIGIN, HostRequest::Ready { capacity: 1 })
            .await
            .unwrap_err(),
        Error::Rejected
    );
    assert_eq!(ready().await.unwrap(), HostResponse::Ready);
    assert_eq!(
        store.ranked(None, join(1, 1)).await.unwrap_err(),
        Error::Rejected
    );
    let (left, right) = tokio::join!(
        queue(store, &alice, join(1, 1)),
        queue(store, &alice, join(1, 1))
    );
    assert_eq!(left.unwrap(), right.unwrap());
    assert_eq!(
        queue(store, &alice, join(1, 9)).await.unwrap_err(),
        Error::Conflict
    );
    assert_eq!(
        queue(store, &alice, join(2, 1)).await.unwrap_err(),
        Error::Conflict
    );
    assert_eq!(
        queue(store, &bob, Request::Cancel { key: id(1) })
            .await
            .unwrap(),
        QueueState::Idle
    );
    assert!(matches!(
        queue(store, &alice, Request::Status).await.unwrap(),
        QueueState::Waiting { .. }
    ));
    assert_eq!(
        queue(store, &alice, Request::Cancel { key: id(1) })
            .await
            .unwrap(),
        QueueState::Idle
    );
    assert_eq!(
        queue(store, &alice, join(1, 1)).await.unwrap(),
        QueueState::Idle
    );
    game(store, host, [a.account, b.account], [&alice, &bob]).await;
    let revoked = play(store, [&alice, &bob], 31).await;
    verify(db, host, false).await;
    verify(db, host, true).await;
    ready().await.unwrap();
    assert_eq!(
        store
            .ranked_host(
                Some(host),
                ORIGIN,
                HostRequest::Void {
                    match_id: revoked.match_id
                }
            )
            .await
            .unwrap_err(),
        Error::Rejected
    );
    assert_eq!(
        queue(store, &alice, Request::Status).await.unwrap(),
        QueueState::Idle
    );
    let timed = play(store, [&alice, &bob], 41).await;
    db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "UPDATE central_identity.ranked_matches SET lease_until=now()-interval '1 second' WHERE id=$1",
        vec![Uuid::from_bytes(timed.match_id).into()])).await.unwrap();
    assert_eq!(
        queue(store, &alice, Request::Status).await.unwrap(),
        QueueState::Idle
    );
    scoring::run(
        store,
        db,
        [(a.account, &alice), (b.account, &bob)],
        host,
        ORIGIN,
    )
    .await;
    boards::run(store, db).await;
}
