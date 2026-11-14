//! Force the node → reward-FK / season → node interleaving without timing guesses.
use super::*;
use sea_orm::{TransactionTrait, prelude::Uuid};
use std::time::Duration;

pub(super) async fn withdraw_during_first_reward(
    store: &AccountStore,
    operator: &DatabaseConnection,
    keys: &ServerKeys,
    season: [u8; 16],
) {
    let owner = enroll(store, keys, "tower-withdraw-race", "tower-withdraw-race").await;
    let token = login(store, keys, owner).await;
    let run = mutate(
        store,
        &token,
        tower::Request::Begin {
            season,
            request_id: id(),
            listed: false,
        },
    )
    .await;
    let run = play::start(store, &token, &run).await;
    let attempt = claim(store, run.attempt.clone().unwrap(), owner).await;
    let held = operator.begin().await.unwrap();
    held.query_one_raw(Statement::from_string(
        DbBackend::Postgres,
        format!(
            "SELECT id FROM central_identity.challenges WHERE id='{}' FOR SHARE",
            Uuid::from_bytes(run.encounter.id)
        ),
    ))
    .await
    .unwrap()
    .unwrap();
    let backend: i32 = held
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT pg_backend_pid() AS pid",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "pid")
        .unwrap();
    let (withdrawal, (blocked, finished)) = tokio::join!(
        publish_tower(store, tower::HostRequest::Withdraw { season }),
        async {
            let blocked = tokio::time::timeout(Duration::from_secs(3), async {
                loop {
                    let row = operator.query_one_raw(Statement::from_string(DbBackend::Postgres, format!(
                        "SELECT count(*)::bigint AS count FROM pg_stat_activity WHERE {backend}=ANY(pg_blocking_pids(pid))"
                    ))).await.unwrap().unwrap();
                    if row.try_get::<i64>("", "count").unwrap() > 0 { break; }
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            }).await;
            let finished = tokio::time::timeout(
                Duration::from_secs(3),
                host(
                    store,
                    HostRequest::Finish {
                        result: completion(&attempt, AttemptState::Won, 10, Some(1000)),
                    },
                ),
            )
            .await;
            held.rollback().await.unwrap();
            (blocked, finished)
        }
    );
    assert!(
        blocked.is_ok(),
        "withdrawal must own its season lock and wait for the held node"
    );
    let HostResponse::Attempt { attempt } = finished.unwrap().unwrap() else {
        panic!("terminal response")
    };
    assert_eq!(
        attempt.state,
        AttemptState::Won,
        "the already admitted result linearizes before withdrawal"
    );
    assert!(withdrawal.unwrap()[0].withdrawn);
    let finished = read_run(store, &token, run.id).await;
    assert_eq!(finished.progress.cleared, 1);
}
