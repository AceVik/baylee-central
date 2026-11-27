//! AI players (`docs/ai-players.md`): an administrator creates one with no
//! password, its runner offers a deck under a runner session, and a lonely
//! person's search gets it after 60 s; one AI session holds a seat in two
//! matches at once; no flow gives an AI a password, and one that never
//! played can be removed.
use super::{
    AccountStore, DatabaseConnection, Error, ORIGIN, ServerKeys, Token, api, enroll, id, login,
    queue, ready, replicas, setup_gateway,
};
use central_api::admin::{AiPlayerSettings, AiSettings};
use central_api::ranked::ai::{
    AiKind, AiOpponents, Offer, Request as AiRequest, Response as AiResponse,
};
use central_api::ranked::{Format, QueueState, Request};
use central_api::terms::TermsVersion;
use central_api::wire::{GatewayPurpose, Identity};
use central_postgres::{Credential, RankedHostCredential::Registered, Session, appoint_admin};
use sea_orm::{ConnectionTrait, DbBackend, Statement, prelude::Uuid};

pub(super) fn join(key: u8, ai: AiOpponents) -> Request {
    Request::Replicate {
        request: api::replica::SeatRequest::Join {
            key: id(key),
            format: Format::Standard,
            deck: [key; 32],
            ai,
        },
    }
}

pub(super) fn settings(profile: &str, name: &str) -> AiPlayerSettings {
    AiPlayerSettings {
        kind: AiKind::House,
        profile: profile.into(),
        label: "Haus-KI · Expert".into(),
        max_games: 2,
        daily_games: None,
        display_name: name.into(),
    }
}

pub(super) async fn sql(db: &DatabaseConnection, text: &str, values: Vec<sea_orm::Value>) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        text,
        values,
    ))
    .await
    .unwrap();
}

pub(super) async fn count(db: &DatabaseConnection, text: &str, values: Vec<sea_orm::Value>) -> i64 {
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
pub(super) async fn waited(db: &DatabaseConnection, person: [u8; 16]) {
    sql(
        db,
        "UPDATE central_identity.ranked_tickets SET created_at = created_at - interval '61 seconds'
         WHERE account_id = $1 AND state = 'waiting'",
        vec![Uuid::from_bytes(person).into()],
    )
    .await;
}

pub(super) async fn ai(
    store: &AccountStore,
    token: &Token,
    request: AiRequest,
) -> Result<AiResponse, Error> {
    match store
        .ranked(Some(token.into()), Request::Ai { request })
        .await?
    {
        api::Response::Ai { response } => Ok(response),
        _ => panic!("an AI request answered something else"),
    }
}

pub(super) async fn person(
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

/// A browser session of the administrator `identity`, signed in just now.
pub(super) async fn desk(store: &AccountStore, identity: Identity) -> Token {
    let credential = store.credential(identity).await.unwrap().unwrap();
    let cookie = Token::generate().unwrap();
    store
        .issue_session(&credential, Session::Browser(&cookie))
        .await
        .unwrap();
    cookie
}

/// An AI player created by the administrator behind `cookie`, switched on:
/// the one way an account becomes an AI.
pub(super) async fn created(store: &AccountStore, cookie: &Token, name: &str) -> [u8; 16] {
    let (account, _) = store
        .create_ai_player(cookie, &settings("house:expert", name), None)
        .await
        .unwrap();
    store.set_ai_enabled(cookie, account, true).await.unwrap();
    account
}

/// A runner token that lists `accounts`.
pub(super) async fn runner_token(
    store: &AccountStore,
    cookie: &Token,
    accounts: &[[u8; 16]],
) -> Token {
    let token = Token::generate().unwrap();
    store
        .create_ai_runner_token(cookie, "ai-fixture-runner", accounts, 90, &token)
        .await
        .unwrap();
    token
}

/// A runner session of `account`, minted with `token`.
pub(super) async fn runner(store: &AccountStore, token: &Token, account: [u8; 16]) -> Token {
    let session = Token::generate().unwrap();
    store
        .ai_runner_session(token, account, &session)
        .await
        .unwrap();
    session
}

/// Delete the AI accounts this suite created: by id, since their login
/// names are drawn.
pub(super) async fn forget(db: &DatabaseConnection, accounts: &[[u8; 16]]) {
    let ids: Vec<Uuid> = accounts.iter().map(|a| Uuid::from_bytes(*a)).collect();
    sql(
        db,
        "DELETE FROM central_identity.accounts WHERE id = ANY($1)",
        vec![ids.into()],
    )
    .await;
}

/// A tag no account holds, of four hex digits.
async fn free_tag(db: &DatabaseConnection) -> u32 {
    u32::try_from(
        count(
            db,
            "SELECT min(t)::bigint AS n FROM pg_catalog.generate_series(4096, 65535) t
             WHERE NOT EXISTS (SELECT 1 FROM central_identity.accounts WHERE tag = t)",
            vec![],
        )
        .await,
    )
    .unwrap()
}

fn standard() -> AiRequest {
    AiRequest::Offer {
        offers: vec![Offer {
            format: Format::Standard,
            deck: [77; 32],
        }],
    }
}

pub(super) async fn run(store: &AccountStore, db: &DatabaseConnection, keys: &ServerKeys) {
    let (owner, _, _) = person(store, keys, "ai-fixture-owner").await;
    let (admin, admin_id, admin_identity) = person(store, keys, "ai-fixture-admin").await;
    let (pia, pia_id, _) = person(store, keys, "ai-fixture-pia").await;
    let (quinn, quinn_id, _) = person(store, keys, "ai-fixture-quinn").await;
    let (rosa, rosa_id, _) = person(store, keys, "ai-fixture-rosa").await;
    let (sam, _, _) = person(store, keys, "ai-fixture-sam").await;
    let tess = person(store, keys, "ai-fixture-tess").await;
    appoint_admin(db, "ai-fixture-admin").await.unwrap();
    let cookie = desk(store, admin_identity).await;
    drop(admin);

    let bot_id = creating(store, db, &cookie, admin_id).await;
    changing(store, db, &cookie, bot_id, pia_id).await;
    let removed = removing(store, db, &cookie, pia_id).await;
    refusing(store, db, &cookie, bot_id).await;
    let bot = runner(
        store,
        &runner_token(store, &cookie, &[bot_id]).await,
        bot_id,
    )
    .await;
    offering(store, &pia, &bot, &cookie, bot_id).await;
    let (host_credential, first, second) = seating(
        store,
        db,
        &owner,
        (&pia, pia_id),
        (&quinn, quinn_id),
        (&rosa, rosa_id),
    )
    .await;
    let people = crowding(
        store,
        db,
        (&rosa, rosa_id),
        (&bot, bot_id),
        &sam,
        &cookie,
        [&first, &second],
    )
    .await;
    admitting(store, db, &pia, &quinn, &bot, &first, &second).await;
    switching(store, db, &bot, &cookie, bot_id).await;
    for match_id in [first.match_id, second.match_id, people.match_id] {
        store
            .ranked_host(
                Registered(&host_credential),
                api::HostRequest::Void { match_id },
            )
            .await
            .unwrap();
    }
    let raced = racing(
        store,
        db,
        admin_identity,
        &host_credential,
        (&tess.0, tess.1),
    )
    .await;

    // Clean up: this suite shares the instance.
    sql(
        db,
        "UPDATE central_identity.ai_settings SET global_seats = 8",
        vec![],
    )
    .await;
    forget(db, &[[bot_id].as_slice(), &removed, &raced].concat()).await;
    sql(
        db,
        "DELETE FROM central_identity.accounts WHERE username_key LIKE 'ai-fixture-%'",
        vec![],
    )
    .await;
}

/// Only an administrator creates an AI, named as one and with a fair mind;
/// the account it makes holds a sealed credential, nothing that opens, and
/// an unguessable login name.
async fn creating(
    store: &AccountStore,
    db: &DatabaseConnection,
    cookie: &Token,
    admin_id: [u8; 16],
) -> [u8; 16] {
    for refused in [
        settings("house:seer", "KI-Haus-Seer"),
        settings("house:expert", "HausExpert"),
        settings("house:expert", "AI-Haus-Expert"),
    ] {
        assert_eq!(
            store.create_ai_player(cookie, &refused, None).await,
            Err(Error::Rejected),
            "{refused:?}"
        );
    }
    let expert = settings("house:expert", "KI-Haus-Expert");
    assert_eq!(
        store.create_ai_player(cookie, &expert, Some(0)).await,
        Err(Error::Rejected)
    );
    let (bot, tag) = store.create_ai_player(cookie, &expert, None).await.unwrap();
    assert!(tag >= central_api::profile::REGISTERED_TAG_FLOOR);
    // The tag it drew is taken now: another AI gets a conflict, not another tag.
    assert_eq!(
        store.create_ai_player(cookie, &expert, Some(tag)).await,
        Err(Error::Conflict)
    );
    let account = Uuid::from_bytes(bot);
    // Enrolled as every check asks, and nothing in it opens.
    assert_eq!(
        count(
            db,
            "SELECT count(*) AS n FROM central_identity.accounts
             WHERE id = $1 AND source = 'registered' AND legacy_id IS NULL
             AND username_key ~ '^ai-[0-9a-f]{20}$' AND credential = 'baylee:ai:sealed'::bytea
             AND octet_length(wrapped_master) = 73 AND octet_length(recovery_wrapped_master) = 73
             AND octet_length(document) = 41 AND document_revision = 1 AND restored
             AND display_name = 'KI-Haus-Expert' AND tag = $2 AND reset_digest IS NULL",
            vec![account.into(), i32::try_from(tag).unwrap().into()],
        )
        .await,
        1
    );
    // Switched off, made by this administrator, audited with numbers alone.
    assert_eq!(
        count(
            db,
            "SELECT count(*) AS n FROM central_identity.ai_players p
             JOIN central_identity.audit_events e ON e.account_id = p.account_id
             WHERE p.account_id = $1 AND NOT p.enabled AND p.set_by = $2 AND p.kind = 'house'
             AND e.action = 'ai.created' AND e.actor_account = $2
             AND e.detail = '{\"max_games\": 2}'::jsonb",
            vec![account.into(), Uuid::from_bytes(admin_id).into()],
        )
        .await,
        1
    );
    bot
}

/// Its kind and profile stay; a person has nothing to change; the label,
/// the caps and the name change, and a person's account never became an AI.
async fn changing(
    store: &AccountStore,
    db: &DatabaseConnection,
    cookie: &Token,
    bot: [u8; 16],
    pia_id: [u8; 16],
) {
    let expert = settings("house:expert", "KI-Haus-Expert");
    assert_eq!(
        store
            .set_ai_player(cookie, bot, &settings("house:novice", "KI-Haus-Expert"))
            .await,
        Err(Error::Conflict)
    );
    assert_eq!(
        store
            .set_ai_player(cookie, pia_id, &settings("house:expert", "KI-Pia"))
            .await,
        Err(Error::Rejected)
    );
    let mut renamed = settings("house:expert", "KI-Haus-Profi");
    renamed.label = "Haus-KI · Profi".into();
    store.set_ai_player(cookie, bot, &renamed).await.unwrap();
    store.set_ai_player(cookie, bot, &expert).await.unwrap();
    assert_eq!(
        count(
            db,
            "SELECT count(*) AS n FROM central_identity.ai_players WHERE account_id = $1",
            vec![Uuid::from_bytes(pia_id).into()],
        )
        .await,
        0
    );
}

/// An AI that never played goes with everything it holds, its runner
/// sessions among them; a person is no AI to remove.
async fn removing(
    store: &AccountStore,
    db: &DatabaseConnection,
    cookie: &Token,
    pia_id: [u8; 16],
) -> Vec<[u8; 16]> {
    let free = free_tag(db).await;
    let (spare, tag) = store
        .create_ai_player(cookie, &settings("house:expert", "KI-Spare"), Some(free))
        .await
        .unwrap();
    assert_eq!(tag, free, "the administrator's tag, never another");
    store.set_ai_enabled(cookie, spare, true).await.unwrap();
    let session = runner(store, &runner_token(store, cookie, &[spare]).await, spare).await;
    let AiResponse::Offered { until: Some(_) } = ai(store, &session, standard()).await.unwrap()
    else {
        panic!("the spare offers");
    };
    assert_eq!(
        store.remove_ai_player(cookie, pia_id).await,
        Err(Error::Rejected)
    );
    assert_eq!(
        store.remove_ai_player(cookie, spare).await,
        Ok(vec![Session::Native(&session).digest()])
    );
    assert_eq!(store.renew_session(&session).await, Err(Error::Rejected));
    let account = Uuid::from_bytes(spare);
    for table in [
        "accounts WHERE id",
        "ai_players WHERE account_id",
        "ai_offers WHERE account_id",
    ] {
        assert_eq!(
            count(
                db,
                &format!("SELECT count(*) AS n FROM central_identity.{table} = $1"),
                vec![account.into()],
            )
            .await,
            0,
            "{table}"
        );
    }
    assert_eq!(
        count(
            db,
            "SELECT count(*) AS n FROM central_identity.audit_events
             WHERE account_id = $1 AND action = 'ai.removed' AND detail IS NULL",
            vec![account.into()],
        )
        .await,
        1
    );
    // Gone already: nothing to remove.
    assert_eq!(
        store.remove_ai_player(cookie, spare).await,
        Err(Error::Rejected)
    );
    vec![spare]
}

/// No path the store serves without a session gives an AI a password or
/// a session: its name and credential answer as an unknown one's, every
/// reset refuses it, even with an address and a code planted by the
/// operator; no role opens the administration for it; its tag changes.
async fn refusing(store: &AccountStore, db: &DatabaseConnection, cookie: &Token, bot: [u8; 16]) {
    let account = Uuid::from_bytes(bot);
    let name = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT username_key FROM central_identity.accounts WHERE id = $1",
            vec![account.into()],
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<String>("", "username_key")
        .unwrap();
    let identity = Identity {
        account: bot,
        epoch: 1,
    };
    assert!(store.discover(&name).await.unwrap().is_none());
    assert!(store.credential(identity).await.unwrap().is_none());
    let sealed = Credential {
        identity,
        revision: 1,
        record: b"baylee:ai:sealed".to_vec(),
    };
    for session in [
        Session::Native(&Token::generate().unwrap()),
        Session::Browser(&Token::generate().unwrap()),
    ] {
        assert_eq!(
            store.issue_session(&sealed, session).await.map(|_| ()),
            Err(Error::Rejected)
        );
    }
    assert_eq!(
        store.issue_reset(cookie, bot, &[7; 32]).await.map(|_| ()),
        Err(Error::Rejected)
    );
    // An address and a code planted by the operator open nothing either.
    sql(
        db,
        "INSERT INTO central_identity.account_emails (account_id, address, lang)
         VALUES ($1, 'ai-fixture@example.org', 'en')",
        vec![account.into()],
    )
    .await;
    assert!(
        store
            .issue_mailed_reset(&name, &[8; 32])
            .await
            .unwrap()
            .is_none()
    );
    sql(
        db,
        "UPDATE central_identity.accounts SET reset_digest = $2,
           reset_expires = pg_catalog.clock_timestamp() + interval '1 hour' WHERE id = $1",
        vec![account.into(), vec![9_u8; 32].into()],
    )
    .await;
    assert_eq!(
        store.reset_credential(&name, &[9; 32]).await.map(|_| ()),
        Err(Error::Rejected)
    );
    sql(
        db,
        "UPDATE central_identity.accounts SET reset_digest = NULL, reset_expires = NULL WHERE id = $1",
        vec![account.into()],
    )
    .await;
    sql(
        db,
        "DELETE FROM central_identity.account_emails WHERE account_id = $1",
        vec![account.into()],
    )
    .await;
    assert_eq!(
        store.set_admin(cookie, bot, true).await,
        Err(Error::Rejected)
    );
    assert_eq!(
        appoint_admin(db, &name).await.map(|_| ()),
        Err(Error::Rejected)
    );
    let tag = u32::try_from(
        count(
            db,
            "SELECT tag::bigint AS n FROM central_identity.accounts WHERE id = $1",
            vec![account.into()],
        )
        .await,
    )
    .unwrap();
    store
        .set_tag(cookie, bot, free_tag(db).await)
        .await
        .unwrap();
    store.set_tag(cookie, bot, tag).await.unwrap();
}

async fn offering(
    store: &AccountStore,
    pia: &Token,
    bot: &Token,
    cookie: &Token,
    bot_id: [u8; 16],
) {
    // Switched off, it offers nothing; a person's session has no AI requests.
    assert_eq!(ai(store, bot, standard()).await, Err(Error::Rejected));
    assert_eq!(ai(store, pia, AiRequest::Work).await, Err(Error::Rejected));
    store.set_ai_enabled(cookie, bot_id, true).await.unwrap();
    let AiResponse::Offered { until: Some(_) } = ai(store, bot, standard()).await.unwrap() else {
        panic!("the offer should stand");
    };
    // An AI never queues as a person.
    assert_eq!(
        queue(store, bot, join(90, AiOpponents::None)).await,
        Err(Error::Rejected)
    );
    // It keeps no library; it says it is an AI in the statement that reads
    // the terms, and its runner accepts them with its own session.
    assert_eq!(
        store.create_object(bot).await.map(|_| ()),
        Err(Error::Rejected)
    );
    let current = TermsVersion {
        version: "2026-11-25".into(),
        digest: [5; 32],
    };
    let gate = store.session_gate(bot, Some(&current)).await.unwrap();
    assert!(gate.ai && !gate.accepted);
    store.accept_central_terms(bot, &current).await.unwrap();
    assert!(
        store
            .session_gate(bot, Some(&current))
            .await
            .unwrap()
            .accepted
    );
    assert!(!store.session_gate(pia, None).await.unwrap().ai);
}

async fn seating(
    store: &AccountStore,
    db: &DatabaseConnection,
    owner: &Token,
    (pia, pia_id): (&Token, [u8; 16]),
    (quinn, quinn_id): (&Token, [u8; 16]),
    (rosa, rosa_id): (&Token, [u8; 16]),
) -> (Token, api::Assignment, api::Assignment) {
    // A host with room for two matches and the reserve of two.
    let (_, host_credential, _) = setup_gateway(store, db, owner).await;
    store
        .ranked_host(Registered(&host_credential), ready(4))
        .await
        .unwrap();

    // "Nur Menschen": no AI, however long the wait; no countdown either.
    let QueueState::Waiting { ai_fill_in_at, .. } = queue(store, rosa, join(91, AiOpponents::None))
        .await
        .unwrap()
    else {
        panic!("rosa waits");
    };
    assert_eq!(ai_fill_in_at, None);
    waited(db, rosa_id).await;
    assert!(matches!(
        queue(store, rosa, Request::Status).await.unwrap(),
        QueueState::Waiting { .. }
    ));
    queue(store, rosa, Request::Cancel { key: id(91) })
        .await
        .unwrap();
    // A search that accepts an AI shows when one may come; before 60 s none does.
    // Two identical Joins at once, with the client's default choice, are one;
    // the same key with another choice is not.
    let (left, right) = tokio::join!(
        queue(store, pia, join(92, AiOpponents::Local)),
        queue(store, pia, join(92, AiOpponents::Local))
    );
    assert_eq!(left.as_ref().unwrap(), right.as_ref().unwrap());
    assert_eq!(
        queue(store, pia, join(92, AiOpponents::Any)).await,
        Err(Error::Conflict)
    );
    let QueueState::Waiting {
        ai_fill_in_at: Some(due),
        queued_at,
        ..
    } = left.unwrap()
    else {
        panic!("pia waits with a countdown");
    };
    assert_eq!(due, queued_at + 60);
    assert!(matches!(
        queue(store, pia, Request::Status).await.unwrap(),
        QueueState::Waiting { .. }
    ));
    waited(db, pia_id).await;
    let QueueState::Assigned {
        assignment: first, ..
    } = queue(store, pia, Request::Status).await.unwrap()
    else {
        panic!("pia should get the AI");
    };
    assert_eq!(
        first.opponent_ai.as_ref().map(|b| b.kind),
        Some(AiKind::House)
    );
    queue(store, quinn, join(93, AiOpponents::Local))
        .await
        .unwrap();
    waited(db, quinn_id).await;
    let QueueState::Assigned {
        assignment: second, ..
    } = queue(store, quinn, Request::Status).await.unwrap()
    else {
        panic!("quinn should get the AI too");
    };
    assert_ne!(first.match_id, second.match_id);
    (host_credential, first, second)
}

/// Its runner sees both matches, and still both once an administrator has
/// lowered `max_games` below them; a third person waits; people meet people.
async fn crowding(
    store: &AccountStore,
    db: &DatabaseConnection,
    (rosa, rosa_id): (&Token, [u8; 16]),
    (bot, bot_id): (&Token, [u8; 16]),
    sam: &Token,
    cookie: &Token,
    [first, second]: [&api::Assignment; 2],
) -> api::Assignment {
    let mut made = vec![first.match_id, second.match_id];
    made.sort_unstable();
    let seen = || async {
        let AiResponse::Work {
            settings,
            assignments,
            ..
        } = ai(store, bot, AiRequest::Work).await.unwrap()
        else {
            panic!("work");
        };
        let mut seen: Vec<_> = assignments.iter().map(|a| a.match_id).collect();
        seen.sort_unstable();
        (settings, seen)
    };
    let (held, matches) = seen().await;
    assert_eq!((held.profile.as_str(), held.max_games), ("house:expert", 2));
    assert_eq!(matches, made);
    let mut lowered = settings("house:expert", "KI-Haus-Expert");
    lowered.max_games = 1;
    store.set_ai_player(cookie, bot_id, &lowered).await.unwrap();
    let (held, matches) = seen().await;
    assert_eq!(held.max_games, 1);
    assert_eq!(matches, made, "every live match, past the lowered cap");
    queue(store, rosa, join(94, AiOpponents::Local))
        .await
        .unwrap();
    waited(db, rosa_id).await;
    assert!(matches!(
        queue(store, rosa, Request::Status).await.unwrap(),
        QueueState::Waiting { .. }
    ));
    // People first: a person who arrives is paired with the person waiting,
    // never given the AI.
    let QueueState::Assigned {
        assignment: people, ..
    } = queue(store, sam, join(95, AiOpponents::Local))
        .await
        .unwrap()
    else {
        panic!("sam meets rosa");
    };
    assert_eq!(people.opponent_ai, None);
    people
}

async fn admitting(
    store: &AccountStore,
    db: &DatabaseConnection,
    pia: &Token,
    quinn: &Token,
    bot: &Token,
    first: &api::Assignment,
    second: &api::Assignment,
) {
    // One AI session holds a ranked sid for each match; the second grant
    // leaves the first in place. A person's session still holds one.
    replicas::privacy::approve_all(store, pia, first.match_id).await;
    replicas::privacy::approve_all(store, quinn, second.match_id).await;
    for (seat, fresh) in [(first, 201), (second, 202)] {
        replicas::privacy::approve_all(store, bot, seat.match_id).await;
        store
            .gateway_grant_scoped(
                bot,
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
    let digest = Session::Native(bot).digest().to_vec();
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
            .gateway_grant(bot, &[ORIGIN], true, GatewayPurpose::Admit, id(203))
            .await
            .map(|_| ()),
        Err(Error::Rejected)
    );
    assert!(
        store
            .gateway_grant_scoped(
                bot,
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
}

async fn switching(
    store: &AccountStore,
    db: &DatabaseConnection,
    bot: &Token,
    cookie: &Token,
    bot_id: [u8; 16],
) {
    // After its first match the AI stays.
    assert_eq!(
        store.remove_ai_player(cookie, bot_id).await,
        Err(Error::Conflict)
    );
    // Switched off without a fresh sign-in: no new match; running ones stay.
    sql(
        db,
        "UPDATE central_identity.sessions SET created_at = created_at - interval '40 minutes',
           expires_at = expires_at - interval '1 minute' WHERE digest = $1",
        vec![Session::Browser(cookie).digest().to_vec().into()],
    )
    .await;
    assert_eq!(
        store.set_ai_enabled(cookie, bot_id, true).await,
        Err(Error::Rejected)
    );
    store.set_ai_enabled(cookie, bot_id, false).await.unwrap();
    assert_eq!(
        store
            .set_ai_settings(
                cookie,
                Some(&AiSettings {
                    fill_in: true,
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
    store.set_ai_settings(cookie, None).await.unwrap();
    let AiResponse::Work { assignments, .. } = ai(store, bot, AiRequest::Work).await.unwrap()
    else {
        panic!("work");
    };
    assert_eq!(assignments.len(), 2);
    // Created, renamed twice, switched on, its cap lowered, switched off.
    assert_eq!(
        count(
            db,
            "SELECT count(*) AS n FROM central_identity.audit_events
             WHERE account_id = $1 AND action LIKE 'ai.%'",
            vec![Uuid::from_bytes(bot_id).into()],
        )
        .await,
        6
    );
    // A stale sign-in neither changes nor removes one.
    assert_eq!(
        store.remove_ai_player(cookie, bot_id).await,
        Err(Error::Rejected)
    );
    assert_eq!(
        store
            .set_ai_player(cookie, bot_id, &settings("house:expert", "KI-Haus-Expert"))
            .await,
        Err(Error::Rejected)
    );
}

/// How one round of [`racing`] orders the removal and the fill-in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Order {
    RemovalFirst,
    FillInFirst,
    Together,
}

/// A removal and a fill-in for one AI, each round with an AI of its own:
/// one after the other in either order, which takes each way for certain,
/// then twice at once. Either the match is made and the removal refused,
/// or the AI goes and the person waits; never both, and never a deadlock.
/// Answers the AIs that stayed.
async fn racing(
    store: &AccountStore,
    db: &DatabaseConnection,
    admin: Identity,
    host: &Token,
    (person, person_id): (&Token, [u8; 16]),
) -> Vec<[u8; 16]> {
    sql(
        db,
        "UPDATE central_identity.ai_settings SET global_seats = 8",
        vec![],
    )
    .await;
    let cookie = desk(store, admin).await;
    let mut stayed = Vec::new();
    let orders = [
        Order::RemovalFirst,
        Order::FillInFirst,
        Order::Together,
        Order::Together,
    ];
    for (round, order) in (0_u8..).zip(orders) {
        // Seen again, as a live host is every few seconds.
        store.ranked_host(Registered(host), ready(4)).await.unwrap();
        let racer = created(store, &cookie, &format!("KI-Racer-{round}")).await;
        let session = runner(store, &runner_token(store, &cookie, &[racer]).await, racer).await;
        ai(store, &session, standard()).await.unwrap();
        queue(store, person, join(120 + round, AiOpponents::Local))
            .await
            .unwrap();
        waited(db, person_id).await;
        let status = || queue(store, person, Request::Status);
        let (removed, state) = match order {
            Order::RemovalFirst => {
                let removed = store.remove_ai_player(&cookie, racer).await;
                (removed, status().await)
            }
            Order::FillInFirst => {
                let state = status().await;
                (store.remove_ai_player(&cookie, racer).await, state)
            }
            Order::Together => tokio::join!(store.remove_ai_player(&cookie, racer), status()),
        };
        let seated = count(
            db,
            "SELECT count(*) AS n FROM central_identity.ranked_matches
             WHERE $1 IN (account_a, account_b)",
            vec![Uuid::from_bytes(racer).into()],
        )
        .await;
        match (removed, state.unwrap()) {
            (Ok(ended), QueueState::Waiting { .. }) if order != Order::FillInFirst => {
                assert_eq!(ended, vec![Session::Native(&session).digest()]);
                assert_eq!(seated, 0, "round {round}");
                queue(
                    store,
                    person,
                    Request::Cancel {
                        key: id(120 + round),
                    },
                )
                .await
                .unwrap();
            }
            (Err(Error::Conflict), QueueState::Assigned { assignment, .. })
                if order != Order::RemovalFirst =>
            {
                assert_eq!(seated, 1, "round {round}");
                assert!(assignment.opponent_ai.is_some());
                store.set_ai_enabled(&cookie, racer, false).await.unwrap();
                store
                    .ranked_host(
                        Registered(host),
                        api::HostRequest::Void {
                            match_id: assignment.match_id,
                        },
                    )
                    .await
                    .unwrap();
                stayed.push(racer);
            }
            (removed, state) => panic!("round {round}, {order:?}: {removed:?} with {state:?}"),
        }
    }
    stayed
}
