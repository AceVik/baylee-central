use super::*;
use sea_orm::prelude::Uuid;

pub(super) async fn run(
    store: &AccountStore,
    operator: &DatabaseConnection,
    keys: &ServerKeys,
    now: i64,
) {
    let task = publish(store, publication(Kind::Daily, now - 2 * 86_400)).await;
    let alice = enroll(store, keys, "challenge-deadline-a", "challenge-deadline-a").await;
    let bob = enroll(store, keys, "challenge-deadline-b", "challenge-deadline-b").await;
    let a = login(store, keys, alice).await;
    let b = login(store, keys, bob).await;
    let first = claim(store, begin(store, &a, &task).await, alice).await;
    let second = claim(store, begin(store, &b, &task).await, bob).await;
    // Reconstruct two claims made while this real UTC slot was live. The isolated fixture
    // shifts only those rows; production has no clock override or privileged test route.
    operator.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "UPDATE central_identity.challenge_attempts SET live=true,started_at=to_timestamp($1::bigint)
         WHERE id IN ($2,$3)", vec![(task.description.period.ends_at-60).into(), Uuid::from_bytes(first.id).into(), Uuid::from_bytes(second.id).into()]
    )).await.unwrap();
    let mut result = completion(&first, AttemptState::Won, 4, None);
    result.finished_at = task.description.period.ends_at - 61;
    assert_eq!(
        host(
            store,
            HostRequest::Finish {
                result: result.clone()
            }
        )
        .await
        .err(),
        Some(Error::Rejected)
    );
    result.finished_at = now + 600;
    assert_eq!(
        host(
            store,
            HostRequest::Finish {
                result: result.clone()
            }
        )
        .await
        .err(),
        Some(Error::Rejected)
    );
    result.finished_at = task.description.period.ends_at - 1;
    finish(store, result.clone()).await;
    finish(store, result).await;
    let mut too_late = completion(&second, AttemptState::Won, 2, None);
    too_late.finished_at = task.description.period.ends_at;
    finish(store, too_late).await;
    let ranked = board(store, &task, Board::FirstWin).await;
    assert_eq!(
        ranked.len(),
        1,
        "receipt after midnight keeps an on-time score; the exact deadline is excluded"
    );
    assert_eq!(ranked[0].turn, 4);
    assert_eq!(badges(store, &a).await[3].count, 1);
    assert_eq!(badges(store, &b).await[3].count, 0);
    personal_and_export(store, &a, &b, &task, &first).await;
    withdraw_active(store, &a, alice, now).await;
}

async fn personal_and_export(
    store: &AccountStore,
    a: &Token,
    b: &Token,
    task: &Challenge,
    first: &Attempt,
) {
    let Response::Personal { progress } =
        public(store, Some(a), Request::Personal { challenge: task.id })
            .await
            .unwrap()
    else {
        panic!();
    };
    assert_eq!(progress.starts, 1);
    assert_eq!(progress.best_turn, Some(4));
    assert_eq!(progress.first_win, Some(first.id));
    let Response::Export { entries, after } = public(
        store,
        Some(a),
        Request::Export {
            table: ExportTable::Participation,
            after: None,
        },
    )
    .await
    .unwrap() else {
        panic!();
    };
    assert_eq!(entries, vec![ExportEntry::Participation { progress }]);
    assert_eq!(after, None);
    let Response::Export { entries, after } = public(
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
    assert_eq!(entries.len(), 1);
    let ExportEntry::Attempt { attempt } = &entries[0] else {
        panic!();
    };
    assert_eq!(attempt.id, first.id);
    assert!(attempt.received_at.unwrap() > attempt.finished_at.unwrap());
    assert_eq!(after, None);
    let Response::Export { entries, .. } = public(
        store,
        Some(b),
        Request::Export {
            table: ExportTable::Attempts,
            after: None,
        },
    )
    .await
    .unwrap() else {
        panic!();
    };
    assert_eq!(entries.len(), 1);
    let ExportEntry::Attempt { attempt } = &entries[0] else {
        panic!();
    };
    assert_ne!(attempt.id, first.id);
}

async fn withdraw_active(store: &AccountStore, token: &Token, who: Identity, now: i64) {
    let task = publish(store, publication(Kind::Daily, now - 3 * 86_400)).await;
    let playing = claim(store, begin(store, token, &task).await, who).await;
    host(store, HostRequest::Withdraw { challenge: task.id })
        .await
        .unwrap();
    let result = completion(&playing, AttemptState::Won, 1, None);
    let ended = finish(store, result.clone()).await;
    assert_eq!(ended.state, AttemptState::Void);
    assert_eq!(finish(store, result).await.state, AttemptState::Void);
    assert_eq!(
        badges(store, token).await[0].count,
        1,
        "a withdrawn running task cannot award a new win"
    );
}
