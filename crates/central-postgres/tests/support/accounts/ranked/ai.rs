//! AI players (`docs/ai-players.md`): an administrator flags an account, its
//! runner offers a deck, and a lonely person's search gets it after 60 s;
//! one AI session holds a seat in two matches at once.
use super::{
    AccountStore, DatabaseConnection, Error, ORIGIN, ServerKeys, Token, api, enroll, id, login,
    queue, ready, replicas, setup_gateway,
};
use central_api::admin::{AiPlayerSettings, AiSettings};
use central_api::ranked::ai::{
    AiKind, AiOpponents, Offer, Request as AiRequest, Response as AiResponse,
};
use central_api::ranked::{Format, QueueState, Request};
use central_api::wire::GatewayPurpose;
use central_postgres::{RankedHostCredential::Registered, Session, appoint_admin};
use sea_orm::{ConnectionTrait, DbBackend, Statement, prelude::Uuid};

fn join(key: u8, ai: AiOpponents) -> Request {
    Request::Replicate {
        request: api::replica::SeatRequest::Join {
            key: id(key),
            format: Format::Standard,
            deck: [key; 32],
            ai,
        },
    }
}

fn settings(profile: &str, name: &str) -> AiPlayerSettings {
    AiPlayerSettings {
        kind: AiKind::House,
        profile: profile.into(),
        label: "Haus-KI · Expert".into(),
        max_games: 2,
        daily_games: None,
        display_name: name.into(),
    }
}

async fn sql(db: &DatabaseConnection, text: &str, values: Vec<sea_orm::Value>) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        text,
        values,
    ))
    .await
    .unwrap();
}

async fn count(db: &DatabaseConnection, text: &str, values: Vec<sea_orm::Value>) -> i64 {
    db.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        text,
        values,
    ))
    .await
    .unwrap()
    .unwrap()
    .try_get("", "n")
    .unwrap()
}

/// A person's waiting ticket, made 61 seconds older.
async fn waited(db: &DatabaseConnection, person: [u8; 16]) {
    sql(
        db,
        "UPDATE central_identity.ranked_tickets SET created_at = created_at - interval '61 seconds'
         WHERE account_id = $1 AND state = 'waiting'",
        vec![Uuid::from_bytes(person).into()],
    )
    .await;
}

async fn ai(store: &AccountStore, token: &Token, request: AiRequest) -> Result<AiResponse, Error> {
    match store
        .ranked(Some(token.into()), Request::Ai { request })
        .await?
    {
        api::Response::Ai { response } => Ok(response),
        _ => panic!("an AI request answered something else"),
    }
}

async fn person(
    store: &AccountStore,
    keys: &ServerKeys,
    name: &str,
) -> (Token, [u8; 16], central_api::wire::Identity) {
    let identity = enroll(store, keys, name, name).await;
    let token = login(store, keys, identity).await;
    store.confirm_restore(&token, 1).await.unwrap();
    store.close_legacy(name, identity).await.unwrap();
    (token, identity.account, identity)
}

pub(super) async fn run(store: &AccountStore, db: &DatabaseConnection, keys: &ServerKeys) {
    let (owner, _, _) = person(store, keys, "ai-fixture-owner").await;
    let (admin, admin_id, admin_identity) = person(store, keys, "ai-fixture-admin").await;
    let (pia, pia_id, _) = person(store, keys, "ai-fixture-pia").await;
    let (quinn, quinn_id, _) = person(store, keys, "ai-fixture-quinn").await;
    let (rosa, rosa_id, _) = person(store, keys, "ai-fixture-rosa").await;
    let (bot, bot_id, bot_identity) = person(store, keys, "ai-fixture-bot").await;
    let (sam, _, _) = person(store, keys, "ai-fixture-sam").await;
    appoint_admin(db, "ai-fixture-admin").await.unwrap();
    let credential = store.credential(admin_identity).await.unwrap().unwrap();
    let cookie = Token::generate().unwrap();
    store
        .issue_session(&credential, Session::Browser(&cookie))
        .await
        .unwrap();
    drop(admin);

    // The unfair level never plays Ranked; a person's name and an unknown
    // account are refused; the flag goes on once.
    assert_eq!(
        store
            .set_ai_player(
                &cookie,
                bot_id,
                Some(&settings("house:seer", "KI-Haus-Seer"))
            )
            .await,
        Err(Error::Rejected)
    );
    assert_eq!(
        store
            .set_ai_player(
                &cookie,
                bot_id,
                Some(&settings("house:expert", "HausExpert"))
            )
            .await,
        Err(Error::Rejected)
    );
    assert_eq!(
        store
            .set_ai_player(
                &cookie,
                id(250),
                Some(&settings("house:expert", "KI-Haus-Expert"))
            )
            .await,
        Err(Error::Rejected)
    );
    assert_eq!(
        store
            .set_ai_player(
                &cookie,
                admin_id,
                Some(&settings("house:expert", "KI-Admin"))
            )
            .await,
        Err(Error::Conflict)
    );
    store
        .set_ai_player(
            &cookie,
            bot_id,
            Some(&settings("house:expert", "KI-Haus-Expert")),
        )
        .await
        .unwrap();
    // The kind and the profile never change; the AI never becomes an administrator.
    assert_eq!(
        store
            .set_ai_player(
                &cookie,
                bot_id,
                Some(&settings("house:novice", "KI-Haus-Expert"))
            )
            .await,
        Err(Error::Conflict)
    );
    assert_eq!(
        store.set_admin(&cookie, bot_id, true).await,
        Err(Error::Rejected)
    );
    // Native only: a website session is refused.
    let bot_credential = store.credential(bot_identity).await.unwrap().unwrap();
    assert_eq!(
        store
            .issue_session(
                &bot_credential,
                Session::Browser(&Token::generate().unwrap())
            )
            .await
            .map(|_| ()),
        Err(Error::Rejected)
    );

    // Switched off, it offers nothing; a person's session has no AI requests.
    let offer = || AiRequest::Offer {
        offers: vec![Offer {
            format: Format::Standard,
            deck: [77; 32],
        }],
    };
    assert_eq!(ai(store, &bot, offer()).await, Err(Error::Rejected));
    assert_eq!(ai(store, &pia, AiRequest::Work).await, Err(Error::Rejected));
    store.set_ai_enabled(&cookie, bot_id, true).await.unwrap();
    let AiResponse::Offered { until: Some(_) } = ai(store, &bot, offer()).await.unwrap() else {
        panic!("the offer should stand");
    };
    // An AI never queues as a person.
    assert_eq!(
        queue(store, &bot, join(90, AiOpponents::None)).await,
        Err(Error::Rejected)
    );

    // A host with room for two matches and the reserve of two.
    let (_, host_credential, _) = setup_gateway(store, db, &owner).await;
    store
        .ranked_host(Registered(&host_credential), ready(4))
        .await
        .unwrap();

    // "Nur Menschen": no AI, however long the wait; no countdown either.
    let QueueState::Waiting { ai_fill_in_at, .. } =
        queue(store, &rosa, join(91, AiOpponents::None))
            .await
            .unwrap()
    else {
        panic!("rosa waits");
    };
    assert_eq!(ai_fill_in_at, None);
    waited(db, rosa_id).await;
    assert!(matches!(
        queue(store, &rosa, Request::Status).await.unwrap(),
        QueueState::Waiting { .. }
    ));
    queue(store, &rosa, Request::Cancel { key: id(91) })
        .await
        .unwrap();
    // A search that accepts an AI shows when one may come; before 60 s none does.
    let QueueState::Waiting {
        ai_fill_in_at: Some(due),
        queued_at,
        ..
    } = queue(store, &pia, join(92, AiOpponents::Local))
        .await
        .unwrap()
    else {
        panic!("pia waits with a countdown");
    };
    assert_eq!(due, queued_at + 60);
    assert!(matches!(
        queue(store, &pia, Request::Status).await.unwrap(),
        QueueState::Waiting { .. }
    ));
    waited(db, pia_id).await;
    let QueueState::Assigned {
        assignment: first, ..
    } = queue(store, &pia, Request::Status).await.unwrap()
    else {
        panic!("pia should get the AI");
    };
    assert_eq!(
        first.opponent_ai.as_ref().map(|b| b.kind),
        Some(AiKind::House)
    );
    queue(store, &quinn, join(93, AiOpponents::Local))
        .await
        .unwrap();
    waited(db, quinn_id).await;
    let QueueState::Assigned {
        assignment: second, ..
    } = queue(store, &quinn, Request::Status).await.unwrap()
    else {
        panic!("quinn should get the AI too");
    };
    assert_ne!(first.match_id, second.match_id);

    // Its runner sees both; a third person waits, since max_games is two.
    let AiResponse::Work {
        settings,
        assignments,
    } = ai(store, &bot, AiRequest::Work).await.unwrap()
    else {
        panic!("work");
    };
    assert_eq!(settings.profile, "house:expert");
    let mut seen: Vec<_> = assignments.iter().map(|a| a.match_id).collect();
    seen.sort_unstable();
    let mut made = vec![first.match_id, second.match_id];
    made.sort_unstable();
    assert_eq!(seen, made);
    queue(store, &rosa, join(94, AiOpponents::Local))
        .await
        .unwrap();
    waited(db, rosa_id).await;
    assert!(matches!(
        queue(store, &rosa, Request::Status).await.unwrap(),
        QueueState::Waiting { .. }
    ));
    // People first: a person who arrives is paired with the person waiting,
    // never given the AI.
    let QueueState::Assigned {
        assignment: people, ..
    } = queue(store, &sam, join(95, AiOpponents::Local))
        .await
        .unwrap()
    else {
        panic!("sam meets rosa");
    };
    assert_eq!(people.opponent_ai, None);

    // One AI session holds a ranked sid for each match; the second grant
    // leaves the first in place. A person's session still holds one.
    replicas::privacy::approve_all(store, &pia, first.match_id).await;
    replicas::privacy::approve_all(store, &quinn, second.match_id).await;
    for (seat, fresh) in [(&first, 201), (&second, 202)] {
        replicas::privacy::approve_all(store, &bot, seat.match_id).await;
        store
            .gateway_grant_scoped(
                &bot,
                &[ORIGIN],
                true,
                GatewayPurpose::Ranked,
                id(fresh),
                Some(api::replica::Fence {
                    match_id: seat.match_id,
                    generation: 0,
                }),
            )
            .await
            .unwrap();
    }
    let digest = Session::Native(&bot).digest().to_vec();
    assert_eq!(
        count(
            db,
            "SELECT count(*) AS n FROM central_identity.gateway_sids
             WHERE session_digest = $1 AND purpose = 'ranked' AND ai_seat",
            vec![digest.clone().into()],
        )
        .await,
        2
    );
    // Not for the lobby, and not for a match it does not sit in.
    assert_eq!(
        store
            .gateway_grant(&bot, &[ORIGIN], true, GatewayPurpose::Admit, id(203))
            .await
            .map(|_| ()),
        Err(Error::Rejected)
    );
    assert!(
        store
            .gateway_grant_scoped(
                &bot,
                &[ORIGIN],
                true,
                GatewayPurpose::Ranked,
                id(204),
                Some(api::replica::Fence {
                    match_id: id(205),
                    generation: 0,
                }),
            )
            .await
            .is_err()
    );

    // After its first match the flag stays.
    assert_eq!(
        store.set_ai_player(&cookie, bot_id, None).await,
        Err(Error::Conflict)
    );
    // Switched off without a fresh sign-in: no new match; running ones stay.
    sql(
        db,
        "UPDATE central_identity.sessions SET created_at = created_at - interval '40 minutes',
           expires_at = expires_at - interval '1 minute' WHERE digest = $1",
        vec![Session::Browser(&cookie).digest().to_vec().into()],
    )
    .await;
    assert_eq!(
        store.set_ai_enabled(&cookie, bot_id, true).await,
        Err(Error::Rejected)
    );
    store.set_ai_enabled(&cookie, bot_id, false).await.unwrap();
    assert_eq!(
        store
            .set_ai_settings(
                &cookie,
                Some(&AiSettings {
                    fill_in_after: 60,
                    drain_minutes: 120,
                    global_seats: 8,
                    host_reserve: 2,
                    carousel_games: 0,
                    carousel_daily: 0,
                })
            )
            .await,
        Err(Error::Rejected)
    );
    store.set_ai_settings(&cookie, None).await.unwrap();
    let AiResponse::Work { assignments, .. } = ai(store, &bot, AiRequest::Work).await.unwrap()
    else {
        panic!("work");
    };
    assert_eq!(assignments.len(), 2);
    assert_eq!(
        count(
            db,
            "SELECT count(*) AS n FROM central_identity.audit_events
             WHERE account_id = $1 AND action LIKE 'ai.%'",
            vec![Uuid::from_bytes(bot_id).into()],
        )
        .await,
        3
    );
    // A stale sign-in clears nothing either.
    assert_eq!(
        store.set_ai_player(&cookie, bot_id, None).await,
        Err(Error::Rejected)
    );

    // Clean up: this suite shares the instance.
    for match_id in [first.match_id, second.match_id, people.match_id] {
        store
            .ranked_host(
                Registered(&host_credential),
                api::HostRequest::Void { match_id },
            )
            .await
            .unwrap();
    }
    sql(
        db,
        "UPDATE central_identity.ai_settings SET global_seats = 8",
        vec![],
    )
    .await;
    sql(
        db,
        "DELETE FROM central_identity.accounts WHERE username_key LIKE 'ai-fixture-%'",
        vec![],
    )
    .await;
}
