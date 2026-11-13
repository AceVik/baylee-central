use super::*;

pub(super) async fn run(
    store: &AccountStore,
    keys: &ServerKeys,
    daily: &Challenge,
    weekly: &Challenge,
) {
    let alice = enroll(store, keys, "challenge-results-a", "challenge-results-a").await;
    let bob = enroll(store, keys, "challenge-results-b", "challenge-results-b").await;
    let a = login(store, keys, alice).await;
    let b = login(store, keys, bob).await;
    let first = starts(store, daily, alice, bob, &a, &b).await;
    duplicates(store, &a, &first).await;
    rankings(store, daily, weekly, alice, bob, &a, &b).await;
}

async fn starts(
    store: &AccountStore,
    daily: &Challenge,
    alice: Identity,
    bob: Identity,
    a: &Token,
    b: &Token,
) -> Attempt {
    assert!(public(store, None, Request::Progress).await.is_err());
    let request = Request::Begin {
        challenge: daily.id,
        request_id: id(),
        listed: true,
    };
    // Simultaneous retries have one identity; a different request cannot open a second hand.
    let (first, retry) = tokio::join!(
        public(store, Some(a), request.clone()),
        public(store, Some(a), request)
    );
    let Response::Attempt { attempt: first } = first.unwrap() else {
        panic!();
    };
    let Response::Attempt { attempt: retry } = retry.unwrap() else {
        panic!();
    };
    assert_eq!(first.id, retry.id);
    assert_eq!(
        public(
            store,
            Some(a),
            Request::Begin {
                challenge: daily.id,
                request_id: id(),
                listed: true
            }
        )
        .await
        .err(),
        Some(Error::Conflict)
    );
    assert_eq!(
        public(store, Some(b), Request::Attempt { attempt: first.id })
            .await
            .err(),
        Some(Error::Rejected)
    );
    assert_eq!(
        host(
            store,
            HostRequest::Claim {
                attempt: first.id,
                account: bob.account,
                game: id(),
                executor: EXECUTOR.into()
            }
        )
        .await
        .err(),
        Some(Error::Rejected)
    );
    assert_eq!(
        host(
            store,
            HostRequest::Claim {
                attempt: first.id,
                account: alice.account,
                game: id(),
                executor: "wrong-build".into()
            }
        )
        .await
        .err(),
        Some(Error::Conflict)
    );
    let first = claim(store, first, alice).await;
    let claimed_again = claim(store, first.clone(), alice).await;
    assert_eq!(first.game, claimed_again.game);
    assert_eq!(first.number, 1);
    assert_eq!(claimed_again.number, 1);
    first
}

async fn duplicates(store: &AccountStore, a: &Token, first: &Attempt) {
    let mut wrong_game = completion(first, AttemptState::Won, 5, Some(40_000));
    wrong_game.game = id();
    assert_eq!(
        host(store, HostRequest::Finish { result: wrong_game })
            .await
            .err(),
        Some(Error::Rejected)
    );
    let result = completion(first, AttemptState::Won, 5, Some(40_000));
    let (done, duplicate) =
        tokio::join!(finish(store, result.clone()), finish(store, result.clone()));
    assert_eq!(done.id, duplicate.id);
    let mut conflicting = result;
    conflicting.turn = 4;
    assert_eq!(
        host(
            store,
            HostRequest::Finish {
                result: conflicting
            }
        )
        .await
        .err(),
        Some(Error::Conflict)
    );
    let counts: Vec<_> = badges(store, a)
        .await
        .into_iter()
        .map(|b| b.count)
        .collect();
    assert_eq!(
        counts,
        vec![1, 0, 1, 1, 0],
        "duplicate delivery awards once"
    );
}

async fn rankings(
    store: &AccountStore,
    daily: &Challenge,
    weekly: &Challenge,
    alice: Identity,
    bob: Identity,
    a: &Token,
    b: &Token,
) {
    let other = claim(store, begin(store, b, daily).await, bob).await;
    finish(
        store,
        completion(&other, AttemptState::Won, 5, Some(50_000)),
    )
    .await;
    let tied = board(store, daily, Board::FirstWin).await;
    assert_eq!(tied.len(), 2);
    assert!(
        tied.iter()
            .all(|r| r.rank == 1 && r.failures == 0 && r.turn == 5)
    );
    let sprint = board(store, daily, Board::Sprint).await;
    assert_eq!(
        sprint
            .iter()
            .map(|r| r.milliseconds.unwrap())
            .collect::<Vec<_>>(),
        vec![40_000, 50_000]
    );
    assert_eq!(
        sprint.iter().map(|r| r.rank).collect::<Vec<_>>(),
        vec![1, 2]
    );
    // A paused replay can improve turns, but cannot replace the first win or add a badge.
    let better = claim(store, begin(store, a, daily).await, alice).await;
    finish(store, completion(&better, AttemptState::Won, 3, None)).await;
    assert_eq!(board(store, daily, Board::BestTurns).await[0].turn, 3);
    assert!(
        board(store, daily, Board::FirstWin)
            .await
            .iter()
            .all(|r| r.turn == 5)
    );
    assert_eq!(
        board(store, daily, Board::Sprint).await[0].milliseconds,
        Some(40_000)
    );
    assert_eq!(badges(store, a).await[0].count, 1);
    public(
        store,
        Some(a),
        Request::Visibility {
            challenge: daily.id,
            listed: false,
        },
    )
    .await
    .unwrap();
    assert_eq!(board(store, daily, Board::FirstWin).await.len(), 1);
    assert_eq!(
        badges(store, a).await[0].count,
        1,
        "unlisting preserves rewards"
    );
    let week = claim(store, begin(store, a, weekly).await, alice).await;
    finish(store, completion(&week, AttemptState::Won, 4, None)).await;
    let counts: Vec<_> = badges(store, a)
        .await
        .into_iter()
        .map(|b| b.count)
        .collect();
    assert_eq!(counts, vec![1, 1, 2, 2, 1]);
}
