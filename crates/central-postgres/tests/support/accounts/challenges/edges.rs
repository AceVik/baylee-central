use super::*;

pub(super) async fn run(
    store: &AccountStore,
    keys: &ServerKeys,
    now: i64,
    daily: &Challenge,
    weekly: &Challenge,
) {
    let who = enroll(store, keys, "challenge-edges", "challenge-edges").await;
    let token = login(store, keys, who).await;
    publications(store, &token, now, daily).await;
    failures_and_hints(store, &token, who, daily, weekly).await;
    archive(store, &token, who, now).await;
}

async fn publications(store: &AccountStore, token: &Token, now: i64, daily: &Challenge) {
    // Immutable publication retries work; replacing a period or leaking a future task does not.
    assert_eq!(
        publish(store, publication(Kind::Daily, now)).await.id,
        daily.id
    );
    let mut changed = publication(Kind::Daily, now);
    changed.package.push(5);
    changed.digest = changed.calculated_digest().unwrap();
    assert_eq!(
        host(
            store,
            HostRequest::Publish {
                publication: Box::new(changed)
            }
        )
        .await
        .err(),
        Some(Error::Conflict)
    );
    let future = publish(store, publication(Kind::Daily, now + 86_400)).await;
    let Response::Catalogue { challenges, .. } =
        public(store, None, Request::Catalogue { before: None })
            .await
            .unwrap()
    else {
        panic!();
    };
    assert!(!challenges.iter().any(|c| c.id == future.id));
    for request in [
        Request::Detail {
            challenge: future.id,
            board: Board::FirstWin,
            offset: 0,
        },
        Request::Begin {
            challenge: future.id,
            request_id: id(),
            listed: true,
        },
        Request::Hint {
            challenge: future.id,
            level: 1,
        },
    ] {
        assert_eq!(
            public(store, Some(token), request).await.err(),
            Some(Error::Rejected)
        );
    }
}

async fn failures_and_hints(
    store: &AccountStore,
    token: &Token,
    who: Identity,
    daily: &Challenge,
    weekly: &Challenge,
) {
    let reserved = begin(store, token, daily).await;
    public(
        store,
        Some(token),
        Request::Abandon {
            attempt: reserved.id,
        },
    )
    .await
    .unwrap();
    // Reserving without seeing the hand is not a failure. A technical abort is not one either.
    let void = claim(store, begin(store, token, daily).await, who).await;
    finish(store, completion(&void, AttemptState::Void, 0, None)).await;
    let lost = claim(store, begin(store, token, daily).await, who).await;
    finish(store, completion(&lost, AttemptState::Lost, 7, None)).await;
    let won = claim(store, begin(store, token, daily).await, who).await;
    finish(store, completion(&won, AttemptState::Won, 6, None)).await;
    let rows = board(store, daily, Board::FirstWin).await;
    assert_eq!(rows.last().unwrap().failures, 1);
    assert_eq!(
        badges(store, token).await[3].count,
        0,
        "a failure excludes first-try credit"
    );
    let started = claim(store, begin(store, token, weekly).await, who).await;
    let Response::Hint { text } = public(
        store,
        Some(token),
        Request::Hint {
            challenge: weekly.id,
            level: 2,
        },
    )
    .await
    .unwrap() else {
        panic!();
    };
    assert_eq!(text.en, "Synthetic hint two");
    let Response::Attempt { attempt: assisted } = public(
        store,
        Some(token),
        Request::Attempt {
            attempt: started.id,
        },
    )
    .await
    .unwrap() else {
        panic!();
    };
    assert!(assisted.assisted);
    finish(store, completion(&started, AttemptState::Won, 3, Some(500))).await;
    assert_eq!(
        board(store, weekly, Board::BestTurns).await.len(),
        1,
        "hinted completion stays off competitive boards"
    );
    assert_eq!(
        badges(store, token).await[1].count,
        1,
        "hints still allow completion rewards"
    );
    let replay = begin(store, token, weekly).await;
    assert!(
        replay.assisted,
        "a new attempt cannot erase a revealed hint"
    );
    public(store, Some(token), Request::Abandon { attempt: replay.id })
        .await
        .unwrap();
}

async fn archive(store: &AccountStore, token: &Token, who: Identity, now: i64) {
    let archive = publish(store, publication(Kind::Daily, now - 86_400)).await;
    let old = claim(store, begin(store, token, &archive).await, who).await;
    assert!(!old.live);
    finish(store, completion(&old, AttemptState::Won, 3, Some(100))).await;
    assert_eq!(board(store, &archive, Board::BestTurns).await, []);
    let progress = badges(store, token).await;
    assert_eq!(progress[0].count, 2);
    assert_eq!(progress[3].count, 0);
    host(
        store,
        HostRequest::Withdraw {
            challenge: archive.id,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        badges(store, token).await[0].count,
        2,
        "withdrawal preserves prior rewards"
    );
    assert_eq!(
        public(
            store,
            Some(token),
            Request::Begin {
                challenge: archive.id,
                request_id: id(),
                listed: true
            }
        )
        .await
        .err(),
        Some(Error::Conflict)
    );
}
