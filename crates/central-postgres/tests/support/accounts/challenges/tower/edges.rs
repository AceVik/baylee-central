use super::*;

pub(super) async fn run(
    store: &AccountStore,
    operator: &DatabaseConnection,
    a: &Token,
    b: &Token,
    alice: Identity,
    season: [u8; 16],
    completed: tower::Run,
) {
    rewards(store, a, alice, season).await;
    privacy(store, a, b, season, completed.id).await;
    super::export::privacy(store, a, b, season).await;
    defeats(store, a, alice, season).await;
    withdrawal(store, a, alice, season).await;
    deletion(store, operator, a, alice, completed.id).await;
}

async fn rewards(store: &AccountStore, a: &Token, alice: Identity, season: [u8; 16]) {
    let mut perfect = mutate(
        store,
        a,
        tower::Request::Begin {
            season,
            request_id: id(),
            listed: true,
        },
    )
    .await;
    for floor in 1..=5 {
        perfect = play::result(store, a, alice, &perfect, AttemptState::Won, None).await;
        if floor < 5 {
            perfect = play::choice(store, a, &perfect, tower::Track::Resolve).await;
        }
    }
    let tower::Response::Read {
        standings,
        badges,
        active,
        ..
    } = call(
        store,
        Some(a),
        tower::Request::Read {
            season: Some(season),
            board: tower::Board::Ascent,
            offset: 0,
        },
    )
    .await
    .unwrap()
    else {
        panic!();
    };
    assert!(active.is_none());
    assert_eq!(standings.len(), 1, "best run per account only");
    assert_eq!(standings[0].losses, 0);
    assert_eq!(
        badges.iter().map(|badge| badge.count).collect::<Vec<_>>(),
        [5, 1, 1],
        "no repeated-floor farming"
    );
    let tower::Response::Read {
        standings,
        run,
        badges,
        active,
        ..
    } = call(
        store,
        None,
        tower::Request::Read {
            season: Some(season),
            board: tower::Board::Sprint,
            offset: 0,
        },
    )
    .await
    .unwrap()
    else {
        panic!();
    };
    assert!(
        standings.is_empty(),
        "neither a defeated nor an interrupted run is a flawless sprint"
    );
    assert!(run.is_none() && active.is_none() && badges.is_empty());
}

async fn privacy(
    store: &AccountStore,
    a: &Token,
    b: &Token,
    season: [u8; 16],
    completed: [u8; 16],
) {
    call(
        store,
        Some(a),
        tower::Request::Visibility {
            season,
            listed: false,
        },
    )
    .await
    .unwrap();
    let tower::Response::Read { standings, .. } = call(
        store,
        None,
        tower::Request::Read {
            season: Some(season),
            board: tower::Board::Ascent,
            offset: 0,
        },
    )
    .await
    .unwrap() else {
        panic!();
    };
    assert!(standings.is_empty());
    let tower::Response::Export { entries, after } = call(
        store,
        Some(a),
        tower::Request::Export {
            table: tower::ExportTable::Runs,
            after: None,
        },
    )
    .await
    .unwrap() else {
        panic!();
    };
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].identity().1, completed);
    assert!(
        matches!(&entries[1], tower::ExportEntry::Run { run, .. } if run.progress.route.len() == 4)
    );
    assert!(after.is_none());
    let Response::Export { entries, .. } = public(
        store,
        Some(a),
        Request::Export {
            table: ExportTable::Attempts,
            after: None,
        },
    )
    .await
    .unwrap() else {
        panic!();
    };
    assert!(entries.iter().all(|entry| matches!(entry, ExportEntry::Attempt { attempt } if attempt.kind == AttemptKind::Tower)));
    let tower::Response::Export { entries, .. } = call(
        store,
        Some(b),
        tower::Request::Export {
            table: tower::ExportTable::Runs,
            after: None,
        },
    )
    .await
    .unwrap() else {
        panic!();
    };
    assert!(entries.is_empty());
}

async fn defeats(store: &AccountStore, a: &Token, alice: Identity, season: [u8; 16]) {
    let mut exhausted = mutate(
        store,
        a,
        tower::Request::Begin {
            season,
            request_id: id(),
            listed: false,
        },
    )
    .await;
    // Cancelling an unseen reservation spends no life; cancelling a revealed game does.
    exhausted = play::start(store, a, &exhausted).await;
    public(
        store,
        Some(a),
        Request::Abandon {
            attempt: exhausted.attempt.unwrap().id,
        },
    )
    .await
    .unwrap();
    exhausted = read_run(store, a, exhausted.id).await;
    assert_eq!(exhausted.progress.lives, 3);
    for remaining in (0..3).rev() {
        let playing = play::start(store, a, &exhausted).await;
        let attempt = claim(store, playing.attempt.unwrap(), alice).await;
        public(
            store,
            Some(a),
            Request::Abandon {
                attempt: attempt.id,
            },
        )
        .await
        .unwrap();
        public(
            store,
            Some(a),
            Request::Abandon {
                attempt: attempt.id,
            },
        )
        .await
        .unwrap();
        exhausted = read_run(store, a, exhausted.id).await;
        assert_eq!(exhausted.progress.lives, remaining);
    }
    assert_eq!(exhausted.progress.phase, tower::Phase::Exhausted);
}

async fn withdrawal(store: &AccountStore, a: &Token, alice: Identity, season: [u8; 16]) {
    let ready = mutate(
        store,
        a,
        tower::Request::Begin {
            season,
            request_id: id(),
            listed: true,
        },
    )
    .await;
    let playing = play::start(store, a, &ready).await;
    assert!(matches!(
        host(
            store,
            HostRequest::Withdraw {
                challenge: ready.encounter.id
            }
        )
        .await,
        Err(Error::Rejected)
    ));
    let attempt = claim(store, playing.attempt.unwrap(), alice).await;
    publish_tower(store, tower::HostRequest::Withdraw { season })
        .await
        .unwrap();
    finish(
        store,
        completion(&attempt, AttemptState::Won, 10, Some(300)),
    )
    .await;
    let withdrawn = read_run(store, a, ready.id).await;
    assert_eq!(withdrawn.progress.cleared, 0);
    assert_eq!(withdrawn.progress.lives, 3);
    assert!(withdrawn.season.withdrawn);
    assert!(matches!(
        call(
            store,
            Some(a),
            tower::Request::Start {
                run: withdrawn.id,
                revision: withdrawn.revision,
                request_id: id()
            }
        )
        .await,
        Err(Error::Conflict)
    ));
    mutate(
        store,
        a,
        tower::Request::Abandon {
            run: withdrawn.id,
            revision: withdrawn.revision,
        },
    )
    .await;
    let tower::Response::Read {
        standings, badges, ..
    } = call(
        store,
        Some(a),
        tower::Request::Read {
            season: Some(season),
            board: tower::Board::Ascent,
            offset: 0,
        },
    )
    .await
    .unwrap()
    else {
        panic!();
    };
    assert!(standings.is_empty());
    assert_eq!(
        badges.iter().map(|badge| badge.count).collect::<Vec<_>>(),
        [5, 1, 1]
    );
}

async fn deletion(
    store: &AccountStore,
    operator: &DatabaseConnection,
    a: &Token,
    alice: Identity,
    completed: [u8; 16],
) {
    operator
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "DELETE FROM central_identity.accounts WHERE id=$1",
            vec![sea_orm::prelude::Uuid::from_bytes(alice.account).into()],
        ))
        .await
        .unwrap();
    for table in [
        "tower_participation",
        "tower_runs",
        "tower_actions",
        "tower_scores",
        "tower_accomplishments",
    ] {
        let row = operator
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                format!(
                    "SELECT count(*) AS count FROM central_identity.{table} WHERE account_id=$1"
                ),
                vec![sea_orm::prelude::Uuid::from_bytes(alice.account).into()],
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.try_get::<i64>("", "count").unwrap(), 0);
    }
    assert!(matches!(
        call(store, Some(a), tower::Request::Run { run: completed }).await,
        Err(Error::Rejected)
    ));
}
