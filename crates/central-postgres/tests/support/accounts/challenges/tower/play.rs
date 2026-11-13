use super::*;

pub(super) async fn start(store: &AccountStore, token: &Token, run: &tower::Run) -> tower::Run {
    mutate(
        store,
        token,
        tower::Request::Start {
            run: run.id,
            revision: run.revision,
            request_id: id(),
        },
    )
    .await
}

pub(super) async fn result(
    store: &AccountStore,
    token: &Token,
    owner: Identity,
    run: &tower::Run,
    state: AttemptState,
    milliseconds: Option<u64>,
) -> tower::Run {
    let started = start(store, token, run).await;
    let attempt = claim(store, started.attempt.unwrap(), owner).await;
    finish(store, completion(&attempt, state, 10, milliseconds)).await;
    let found = mutate(
        store,
        token,
        tower::Request::Locate {
            game: attempt.game.unwrap(),
        },
    )
    .await;
    assert_eq!(
        found.id, run.id,
        "finished games retain their exact expedition context"
    );
    assert!(matches!(
        call(
            store,
            None,
            tower::Request::Locate {
                game: attempt.game.unwrap()
            }
        )
        .await,
        Err(Error::Rejected)
    ));
    read_run(store, token, run.id).await
}

pub(super) async fn choice(
    store: &AccountStore,
    token: &Token,
    run: &tower::Run,
    track: tower::Track,
) -> tower::Run {
    mutate(
        store,
        token,
        tower::Request::Choose {
            run: run.id,
            revision: run.revision,
            track,
            request_id: id(),
        },
    )
    .await
}

async fn first_win(
    store: &AccountStore,
    a: &Token,
    alice: Identity,
    bob: Identity,
    first: tower::Run,
) -> tower::Run {
    let void = result(store, a, alice, &first, AttemptState::Void, None).await;
    assert_eq!(void.progress, first.progress);
    let first = result(store, a, alice, &void, AttemptState::Lost, Some(900)).await;
    assert_eq!(first.progress.lives, 2);
    assert_eq!(
        first.encounter.id, void.encounter.id,
        "a retry never rerolls a deck/opening"
    );
    let request = tower::Request::Start {
        run: first.id,
        revision: first.revision,
        request_id: id(),
    };
    let (started, retried) =
        tokio::join!(mutate(store, a, request.clone()), mutate(store, a, request));
    assert_eq!(started.attempt, retried.attempt);
    let attempt = started.attempt.clone().unwrap();
    assert!(matches!(
        host(
            store,
            HostRequest::Claim {
                attempt: attempt.id,
                account: bob.account,
                game: id(),
                executor: EXECUTOR.into()
            }
        )
        .await,
        Err(Error::Rejected)
    ));
    assert!(
        matches!(host(store, HostRequest::Executor { attempt: attempt.id }).await,
        Ok(HostResponse::Executor { executor }) if executor == EXECUTOR)
    );
    let attempt = claim(store, attempt, alice).await;
    let won = completion(&attempt, AttemptState::Won, 11, Some(800));
    let (done, duplicate) = tokio::join!(finish(store, won.clone()), finish(store, won));
    assert_eq!(done, duplicate);
    let run = read_run(store, a, started.id).await;
    assert_eq!(run.progress.cleared, 1);
    assert_eq!(run.progress.phase, tower::Phase::Choosing);
    assert_eq!(run.progress.lives, 2);
    run
}

async fn choice_race(store: &AccountStore, a: &Token, run: &tower::Run) -> tower::Run {
    let request = tower::Request::Choose {
        run: run.id,
        revision: run.revision,
        track: tower::Track::Assault,
        request_id: id(),
    };
    let competing = tower::Request::Choose {
        run: run.id,
        revision: run.revision,
        track: tower::Track::Resolve,
        request_id: id(),
    };
    let (left, right) = tokio::join!(
        call(store, Some(a), request.clone()),
        call(store, Some(a), competing.clone())
    );
    assert_ne!(left.is_ok(), right.is_ok(), "one concurrent choice wins");
    let winner = if left.is_ok() { request } else { competing };
    let run = mutate(store, a, winner.clone()).await;
    assert_eq!(run.progress.route.len(), 1);
    let tower::Request::Choose {
        run: id,
        revision,
        request_id,
        track,
    } = winner
    else {
        panic!();
    };
    assert!(matches!(
        call(
            store,
            Some(a),
            tower::Request::Choose {
                run: id,
                revision,
                request_id,
                track: if track == tower::Track::Assault {
                    tower::Track::Resolve
                } else {
                    tower::Track::Assault
                }
            }
        )
        .await,
        Err(Error::Conflict)
    ));
    run
}

pub(super) async fn ascent(
    store: &AccountStore,
    a: &Token,
    b: &Token,
    alice: Identity,
    bob: Identity,
    first: tower::Run,
) -> tower::Run {
    let run = first_win(store, a, alice, bob, first).await;
    let mut run = choice_race(store, a, &run).await;
    for floor in 2..=5 {
        run = result(store, a, alice, &run, AttemptState::Won, Some(800)).await;
        assert_eq!(run.progress.cleared, floor);
        if floor < 5 {
            run = choice(store, a, &run, tower::Track::Precision).await;
        }
    }
    assert_eq!(run.progress.phase, tower::Phase::Conquered);
    let tower::Response::Read {
        standings, badges, ..
    } = call(
        store,
        Some(a),
        tower::Request::Read {
            season: Some(run.season.id),
            board: tower::Board::Ascent,
            offset: 0,
        },
    )
    .await
    .unwrap()
    else {
        panic!();
    };
    assert_eq!(standings.len(), 1);
    assert!(standings[0].own);
    assert_eq!(standings[0].floors, 5);
    assert_eq!(standings[0].losses, 1);
    assert_eq!(standings[0].turns, 51);
    assert_eq!(
        badges.iter().map(|badge| badge.count).collect::<Vec<_>>(),
        [5, 1, 0]
    );
    assert!(
        call(
            store,
            Some(b),
            tower::Request::Start {
                run: run.id,
                revision: run.revision,
                request_id: super::id()
            }
        )
        .await
        .is_err()
    );
    run
}
