//! Delayed outbox receipt, exact week boundary, owned recovery and real sprint ties.
use super::*;
use sea_orm::prelude::Uuid;

pub(super) async fn run(
    store: &AccountStore,
    operator: &DatabaseConnection,
    keys: &ServerKeys,
    now: i64,
) {
    let season = historical(store, operator, now).await;
    let alice = enroll(store, keys, "tower-boundary-a", "tower-boundary-a").await;
    let bob = enroll(store, keys, "tower-boundary-b", "tower-boundary-b").await;
    let a = login(store, keys, alice).await;
    let b = login(store, keys, bob).await;
    let run = begin(store, &a, season).await;
    let end = run.season.theme.period.ends_at;
    let first = delivered(store, operator, &a, alice, &run, end - 1).await;
    assert_eq!(first.progress.cleared, 1);
    assert!(
        first.progress.competitive,
        "late receipt retains a timely victory"
    );
    let second = play::choice(store, &a, &first, tower::Track::Assault).await;
    let second = play::start(store, &a, &second).await;
    let attempt = claim(store, second.attempt.clone().unwrap(), alice).await;
    assert!(matches!(
        call(
            store,
            Some(&b),
            tower::Request::Locate {
                game: attempt.game.unwrap()
            }
        )
        .await,
        Err(Error::Rejected)
    ));
    reconstruct_claim(operator, &second, &attempt, end - 60).await;
    let mut completion = completion(&attempt, AttemptState::Won, 10, Some(1000));
    completion.finished_at = end;
    finish(store, completion).await;
    let second = read_run(store, &a, run.id).await;
    assert_eq!(second.progress.cleared, 2);
    assert!(!second.progress.competitive);
    let tower::Response::Read {
        standings, badges, ..
    } = read(store, Some(&a), season, tower::Board::Ascent).await
    else {
        panic!()
    };
    assert_eq!(standings.len(), 1);
    assert_eq!(
        standings[0].floors, 1,
        "the previously ranked prefix stays frozen"
    );
    assert_eq!(badges[0].count, 2, "practice still grants a distinct floor");
    let tower::Response::Read {
        season: current,
        active,
        ..
    } = call(
        store,
        Some(&a),
        tower::Request::Read {
            season: None,
            board: tower::Board::Ascent,
            offset: 0,
        },
    )
    .await
    .unwrap()
    else {
        panic!()
    };
    assert_ne!(current.unwrap().id, season);
    assert_eq!(
        active.unwrap().id,
        run.id,
        "rollover preserves discovery of the old active run"
    );
    mutate(
        store,
        &a,
        tower::Request::Abandon {
            run: run.id,
            revision: second.revision,
        },
    )
    .await;
    sprint_ties(store, operator, [(&a, alice), (&b, bob)], season, end).await;
}

async fn sprint_ties(
    store: &AccountStore,
    operator: &DatabaseConnection,
    owners: [(&Token, Identity); 2],
    season: [u8; 16],
    end: i64,
) {
    for (token, owner) in owners {
        let mut run = begin(store, token, season).await;
        for floor in 0..5 {
            run = delivered(store, operator, token, owner, &run, end - 1).await;
            if floor < 4 {
                run = play::choice(store, token, &run, tower::Track::Precision).await;
            }
        }
        assert_eq!(run.progress.milliseconds, Some(5000));
    }
    for board in [tower::Board::Ascent, tower::Board::Sprint] {
        let tower::Response::Read { standings, .. } = read(store, None, season, board).await else {
            panic!()
        };
        assert_eq!(standings.len(), 2);
        assert!(standings.iter().all(|row| row.rank == 1
            && row.floors == 5
            && row.losses == 0
            && row.milliseconds == Some(5000)));
    }
}

async fn historical(store: &AccountStore, operator: &DatabaseConnection, now: i64) -> [u8; 16] {
    // Publishing an already expired week is intentionally refused. Reconstruct a
    // previously published immutable season through the isolated operator only.
    let season = staging(store, now + 604_800).await;
    let (manifest, nodes) = candidates(now - 604_800);
    let period = manifest.theme.period;
    operator.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "UPDATE central_identity.tower_seasons SET starts_at=$1,ends_at=$2,manifest=$3::jsonb WHERE id=$4",
        vec![period.starts_at.into(),period.ends_at.into(),String::from_utf8(central_api::encode(&manifest).unwrap()).unwrap().into(),Uuid::from_bytes(season).into()])).await.unwrap();
    for (index, publication) in nodes.iter().enumerate() {
        operator.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "UPDATE central_identity.challenges SET starts_at=$1,ends_at=$2,description=$3::jsonb,digest=$4,publication=$5
             WHERE id=(SELECT challenge_id FROM central_identity.tower_nodes WHERE season_id=$6 AND node=$7)",
            vec![period.starts_at.into(),period.ends_at.into(),String::from_utf8(central_api::encode(&publication.description).unwrap()).unwrap().into(),
                publication.digest.to_vec().into(),central_api::encode(publication).unwrap().into(),Uuid::from_bytes(season).into(),i16::try_from(index).unwrap().into()])).await.unwrap();
    }
    season
}

async fn begin(store: &AccountStore, token: &Token, season: [u8; 16]) -> tower::Run {
    mutate(
        store,
        token,
        tower::Request::Begin {
            season,
            request_id: id(),
            listed: true,
        },
    )
    .await
}

async fn read(
    store: &AccountStore,
    token: Option<&Token>,
    season: [u8; 16],
    board: tower::Board,
) -> tower::Response {
    call(
        store,
        token,
        tower::Request::Read {
            season: Some(season),
            board,
            offset: 0,
        },
    )
    .await
    .unwrap()
}

async fn delivered(
    store: &AccountStore,
    operator: &DatabaseConnection,
    token: &Token,
    owner: Identity,
    run: &tower::Run,
    finished: i64,
) -> tower::Run {
    let run = play::start(store, token, run).await;
    let attempt = claim(store, run.attempt.clone().unwrap(), owner).await;
    reconstruct_claim(operator, &run, &attempt, finished - 59).await;
    let mut result = completion(&attempt, AttemptState::Won, 10, Some(1000));
    result.finished_at = finished;
    finish(store, result.clone()).await;
    finish(store, result).await;
    read_run(store, token, run.id).await
}

async fn reconstruct_claim(
    operator: &DatabaseConnection,
    run: &tower::Run,
    attempt: &Attempt,
    started: i64,
) {
    // The isolated operator recreates historical claims; no production clock override exists.
    operator.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "UPDATE central_identity.challenge_attempts SET live=true,started_at=to_timestamp($1::bigint) WHERE id=$2",
        vec![started.into(),Uuid::from_bytes(attempt.id).into()])).await.unwrap();
    operator.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "UPDATE central_identity.tower_runs SET progress=jsonb_set(progress,'{competitive}','true') WHERE id=$1",
        vec![Uuid::from_bytes(run.id).into()])).await.unwrap();
}
