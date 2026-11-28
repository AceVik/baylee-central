//! AI runner tokens (`docs/ai-players.md` §2): an administrator makes one for
//! a runner machine; it trades itself for native sessions of the AI accounts
//! it lists, at most four at once for an account; two of them hold a seat in
//! two matches at once; a revocation ends every session it minted.
use super::ai::{ai, count, created, forget, join, person, sql, waited};
use super::{AccountStore, DatabaseConnection, Error, ORIGIN, ServerKeys, Token, api, id, login};
use super::{queue, ready, replicas, setup_gateway};
use central_api::ranked::ai::{AiOpponents, Offer, Request as AiRequest, Response as AiResponse};
use central_api::ranked::{Format, QueueState, Request};
use central_api::wire::{GatewayPurpose, Identity};
use central_postgres::{RankedHostCredential::Registered, RunnerSession, Session, appoint_admin};
use sea_orm::{ConnectionTrait, DbBackend, Statement, prelude::Uuid};

/// The administrator's browser session, signed in just now.
struct Admin {
    id: [u8; 16],
    cookie: Token,
}

/// An AI account as an administrator creates it, switched on: it has no
/// password, so every session it ever holds is a runner's.
async fn ai_account(store: &AccountStore, admin: &Admin, display: &str) -> [u8; 16] {
    created(store, &admin.cookie, display).await
}

/// The runner sessions `account` holds.
async fn runner_sessions(db: &DatabaseConnection, account: [u8; 16]) -> i64 {
    count(
        db,
        "SELECT count(*) AS n FROM central_identity.sessions
         WHERE account_id = $1 AND runner_token IS NOT NULL",
        vec![Uuid::from_bytes(account).into()],
    )
    .await
}

/// The digests of the runner sessions `account` holds, oldest first.
async fn held_digests(db: &DatabaseConnection, account: [u8; 16]) -> Vec<[u8; 32]> {
    db.query_all_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT digest FROM central_identity.sessions
         WHERE account_id = $1 AND runner_token IS NOT NULL ORDER BY created_at, digest",
        vec![Uuid::from_bytes(account).into()],
    ))
    .await
    .unwrap()
    .iter()
    .map(|row| {
        row.try_get::<Vec<u8>>("", "digest")
            .unwrap()
            .try_into()
            .unwrap()
    })
    .collect()
}

/// A fresh runner session of `account` under `token`, or the refusal.
async fn exchange(
    store: &AccountStore,
    token: &Token,
    account: [u8; 16],
) -> Result<(Token, RunnerSession), Error> {
    let session = Token::generate().unwrap();
    let minted = store.ai_runner_session(token, account, &session).await?;
    Ok((session, minted))
}

fn digest(token: &Token) -> [u8; 32] {
    Session::Native(token).digest()
}

pub(super) async fn run(store: &AccountStore, db: &DatabaseConnection, keys: &ServerKeys) {
    let (owner, _, _) = person(store, keys, "rt-fixture-owner").await;
    let (_, admin_id, admin_identity) = person(store, keys, "rt-fixture-admin").await;
    appoint_admin(db, "rt-fixture-admin").await.unwrap();
    let cookie = Token::generate().unwrap();
    let credential = store.credential(admin_identity).await.unwrap().unwrap();
    store
        .issue_session(&credential, Session::Browser(&cookie))
        .await
        .unwrap();
    let admin = Admin {
        id: admin_id,
        cookie,
    };
    let pia = person(store, keys, "rt-fixture-pia").await;
    let quinn = person(store, keys, "rt-fixture-quinn").await;
    let (_, outsider, outsider_identity) = person(store, keys, "rt-fixture-out").await;
    let bot = ai_account(store, &admin, "KI-Runner-A").await;
    let other = ai_account(store, &admin, "KI-Runner-B").await;

    let (token, token_id) = making(store, db, &admin, [bot, other, outsider]).await;
    let minted = exchanging(store, db, &token, [bot, other, outsider]).await;
    let held = capping(store, db, &token, bot, minted).await;
    let (host, matches, sids) = seating(
        store,
        db,
        &owner,
        [&held[2], &held[3]],
        [(&pia.0, pia.1), (&quinn.0, quinn.1)],
    )
    .await;
    relayed(store, db, (&held, token_id), bot, &pia.0).await;
    revoking(store, db, &admin, (&token, token_id), bot, &held, &sids).await;
    // Revoked: none of its sessions is a runner's relay any more.
    for session in &held {
        assert_eq!(store.relay_runner(session).await, Ok(None));
    }
    people(store, db, keys, outsider_identity).await;
    grants(db).await;

    // Clean up: this suite shares the instance.
    for match_id in matches {
        store
            .ranked_host(Registered(&host), api::HostRequest::Void { match_id })
            .await
            .unwrap();
    }
    forget(db, &[bot, other]).await;
    sql(
        db,
        "DELETE FROM central_identity.accounts WHERE username_key LIKE 'rt-fixture-%'",
        vec![],
    )
    .await;
}

/// Only an AI player is listed, each once; the token is stored as its digest
/// and named in the audit by its id alone.
async fn making(
    store: &AccountStore,
    db: &DatabaseConnection,
    admin: &Admin,
    [bot, other, outsider]: [[u8; 16]; 3],
) -> (Token, [u8; 16]) {
    let cookie = &admin.cookie;
    let token = Token::generate().unwrap();
    for (name, accounts, days) in [
        ("rt-runner", vec![outsider], 90),
        ("rt-runner", vec![bot, outsider], 90),
        ("rt-runner", vec![bot, bot], 90),
        ("rt-runner", vec![], 90),
        ("rt-runner", vec![id(250)], 90),
        ("rt-runner", vec![bot], 0),
        ("rt-runner", vec![bot], 367),
        ("", vec![bot], 90),
    ] {
        assert_eq!(
            store
                .create_ai_runner_token(cookie, name, &accounts, days, &token)
                .await,
            Err(Error::Rejected),
            "{name:?} {accounts:?} {days}"
        );
    }
    let (token_id, expires_at) = store
        .create_ai_runner_token(cookie, "rt-runner", &[bot], 90, &token)
        .await
        .unwrap();
    let listed = store.admin_ai_runner_tokens(cookie).await.unwrap();
    let info = listed.iter().find(|t| t.id == token_id).unwrap();
    assert_eq!(info.name, "rt-runner");
    assert_eq!(info.expires_at, expires_at);
    // 90 days, give or take an hour of daylight saving in the server's zone
    assert!(
        (90 * 86_400 - 3_600..=90 * 86_400 + 3_600).contains(&(info.expires_at - info.created_at))
    );
    assert_eq!(info.created_by, Some(admin.id));
    assert_eq!(
        info.accounts.iter().map(|a| a.id).collect::<Vec<_>>(),
        vec![bot]
    );
    assert_eq!(
        info.accounts[0].display_name.as_deref(),
        Some("KI-Runner-A")
    );
    assert_eq!((info.revoked_at, info.last_used_at), (None, None));
    // The digest alone is kept, and the audit names the token by its id.
    let token_uuid = Uuid::from_bytes(token_id);
    assert_eq!(
        count(
            db,
            "SELECT count(*) AS n FROM central_identity.ai_runner_tokens
             WHERE id = $1 AND digest = $2",
            vec![token_uuid.into(), token.digest().to_vec().into()],
        )
        .await,
        1
    );
    assert_eq!(
        count(
            db,
            "SELECT count(*) AS n FROM central_identity.audit_events
             WHERE ai_runner_token_id = $1 AND action = 'ai.runner_token_created'
             AND actor_account = $2 AND account_id IS NULL AND detail IS NULL",
            vec![token_uuid.into(), Uuid::from_bytes(admin.id).into()],
        )
        .await,
        1
    );
    // Its list is fixed: the other AI is on none.
    assert_eq!(
        count(
            db,
            "SELECT count(*) AS n FROM central_identity.ai_runner_accounts WHERE account_id = $1",
            vec![Uuid::from_bytes(other).into()],
        )
        .await,
        0
    );
    (token, token_id)
}

/// The exchange mints a working native session; every refusal is the same
/// `Rejected` and mints nothing; two exchanges of one token at once wait
/// for each other and both mint. Answers the three sessions minted.
async fn exchanging(
    store: &AccountStore,
    db: &DatabaseConnection,
    token: &Token,
    [bot, other, outsider]: [[u8; 16]; 3],
) -> Vec<Token> {
    let (session, minted) = exchange(store, token, bot).await.unwrap();
    assert_eq!(minted.identity.account, bot);
    assert_eq!(minted.ended, Vec::<[u8; 32]>::new());
    assert!(store.renew_session(&session).await.unwrap() >= minted.expires_at);
    let status = store.session_status(&session).await.unwrap();
    assert_eq!(status.identity.account, bot);
    let AiResponse::Work { settings, .. } = ai(store, &session, AiRequest::Work).await.unwrap()
    else {
        panic!("work");
    };
    assert_eq!(settings.profile, "house:expert");
    // Its use and the account's sign-in are recorded, once.
    assert_eq!(
        count(
            db,
            "SELECT count(*) AS n FROM central_identity.ai_runner_signins s
             JOIN central_identity.ai_runner_tokens t ON t.id = s.token_id
             JOIN central_identity.accounts a ON a.id = s.account_id
             WHERE s.account_id = $1 AND t.last_used_at IS NOT NULL
             AND a.last_sign_in_at IS NOT NULL",
            vec![Uuid::from_bytes(bot).into()],
        )
        .await,
        1
    );
    // An unlisted AI, a person, an unknown account, an unknown token and an
    // expired one: all alike.
    let expired = Token::generate().unwrap();
    sql(
        db,
        "WITH made AS (
           INSERT INTO central_identity.ai_runner_tokens (name, digest, created_by, created_at, expires_at)
           VALUES ('rt-expired', $1, $2, pg_catalog.clock_timestamp() - interval '2 days',
                   pg_catalog.clock_timestamp() - interval '1 day')
           RETURNING id)
         INSERT INTO central_identity.ai_runner_accounts (token_id, account_id) SELECT id, $3 FROM made",
        vec![
            expired.digest().to_vec().into(),
            Uuid::from_bytes(bot).into(),
            Uuid::from_bytes(bot).into(),
        ],
    )
    .await;
    let unknown = Token::generate().unwrap();
    for (presented, account) in [
        (token, other),
        (token, outsider),
        (token, id(250)),
        (&unknown, bot),
        (&expired, bot),
    ] {
        assert_eq!(
            exchange(store, presented, account).await.err(),
            Some(Error::Rejected)
        );
    }
    for account in [other, outsider] {
        assert_eq!(runner_sessions(db, account).await, 0);
    }
    assert_eq!(runner_sessions(db, bot).await, 1);
    let (left, right) = tokio::join!(exchange(store, token, bot), exchange(store, token, bot));
    let (left, right) = (left.unwrap().0, right.unwrap().0);
    assert_eq!(runner_sessions(db, bot).await, 3);
    vec![session, left, right]
}

/// An account never holds more than four runner sessions: an exchange past
/// them ends the oldest, and an expired one goes first. Answers the four
/// live ones, oldest first.
async fn capping(
    store: &AccountStore,
    db: &DatabaseConnection,
    token: &Token,
    bot: [u8; 16],
    minted: Vec<Token>,
) -> Vec<Token> {
    let mut sessions = minted;
    let oldest = held_digests(db, bot).await;
    sessions.sort_by_key(|s| oldest.iter().position(|d| *d == digest(s)));
    assert_eq!(sessions.iter().map(digest).collect::<Vec<_>>(), oldest);
    let (fourth, minted) = exchange(store, token, bot).await.unwrap();
    assert_eq!(minted.ended, Vec::<[u8; 32]>::new());
    assert_eq!(runner_sessions(db, bot).await, 4);
    let (fifth, minted) = exchange(store, token, bot).await.unwrap();
    assert_eq!(minted.ended, vec![oldest[0]]);
    assert_eq!(runner_sessions(db, bot).await, 4);
    // An expired session goes before any live one.
    sql(
        db,
        "UPDATE central_identity.sessions SET created_at = pg_catalog.clock_timestamp() - interval '1 hour',
           expires_at = pg_catalog.clock_timestamp() - interval '1 minute' WHERE digest = $1",
        vec![oldest[1].to_vec().into()],
    )
    .await;
    let (sixth, minted) = exchange(store, token, bot).await.unwrap();
    assert_eq!(minted.ended, vec![oldest[1]]);
    assert_eq!(runner_sessions(db, bot).await, 4);
    let mut sessions = sessions.split_off(2);
    sessions.extend([fourth, fifth, sixth]);
    assert_eq!(
        held_digests(db, bot).await,
        sessions.iter().map(digest).collect::<Vec<_>>()
    );
    // The ended ones renew no more; the rest do.
    for ended in [oldest[0], oldest[1]] {
        assert_eq!(
            count(
                db,
                "SELECT count(*) AS n FROM central_identity.sessions WHERE digest = $1",
                vec![ended.to_vec().into()],
            )
            .await,
            0
        );
    }
    for session in &sessions {
        store.renew_session(session).await.unwrap();
    }
    sessions
}

/// Two runner sessions of one AI each make an offer, each is seated in a
/// match by fill-in, and each holds that match's ranked sid at once.
async fn seating(
    store: &AccountStore,
    db: &DatabaseConnection,
    owner: &Token,
    runners: [&Token; 2],
    people: [(&Token, [u8; 16]); 2],
) -> (Token, [api::Id; 2], Vec<[u8; 16]>) {
    let (_, host, _) = setup_gateway(store, db, owner).await;
    store
        .ranked_host(Registered(&host), ready(4))
        .await
        .unwrap();
    let mut matches = [[0; 16]; 2];
    for (n, (runner, (person, person_id))) in runners.into_iter().zip(people).enumerate() {
        let key = 110 + u8::try_from(n).unwrap();
        let AiResponse::Offered { until: Some(_) } = ai(
            store,
            runner,
            AiRequest::Offer {
                offers: vec![Offer {
                    format: Format::Standard,
                    deck: [77; 32],
                }],
            },
        )
        .await
        .unwrap() else {
            panic!("the offer should stand");
        };
        queue(store, person, join(key, AiOpponents::Local))
            .await
            .unwrap();
        waited(db, person_id).await;
        let QueueState::Assigned { assignment, .. } =
            queue(store, person, Request::Status).await.unwrap()
        else {
            panic!("the AI should fill the search");
        };
        assert!(assignment.opponent_ai.is_some());
        matches[n] = assignment.match_id;
        replicas::privacy::approve_all(store, person, assignment.match_id).await;
        replicas::privacy::approve_all(store, runner, assignment.match_id).await;
    }
    assert_ne!(matches[0], matches[1]);
    let mut sids = Vec::new();
    for (n, (runner, match_id)) in runners.into_iter().zip(matches).enumerate() {
        let grant = store
            .gateway_grant_scoped(
                runner,
                &[ORIGIN],
                true,
                GatewayPurpose::Ranked,
                id(130 + u8::try_from(n).unwrap()),
                Some(api::replica::Fence {
                    match_id,
                    generation: 0,
                }),
            )
            .await
            .unwrap();
        assert_eq!(grant.ranked_match, Some(match_id));
        sids.push(grant.sid);
    }
    // Both at once, each session with its own match's.
    for (runner, match_id) in runners.into_iter().zip(matches) {
        assert_eq!(
            count(
                db,
                "SELECT count(*) AS n FROM central_identity.gateway_sids
                 WHERE session_digest = $1 AND ranked_match = $2 AND purpose = 'ranked' AND ai_seat",
                vec![
                    digest(runner).to_vec().into(),
                    Uuid::from_bytes(match_id).into()
                ],
            )
            .await,
            1
        );
    }
    (host, matches, sids)
}

/// A revocation needs no fresh sign-in, ends every session the token minted
/// with their sids in its transaction, and leaves the token useless; making
/// one needs a fresh sign-in.
async fn revoking(
    store: &AccountStore,
    db: &DatabaseConnection,
    admin: &Admin,
    (token, token_id): (&Token, [u8; 16]),
    bot: [u8; 16],
    held: &[Token],
    sids: &[[u8; 16]],
) {
    let cookie = &admin.cookie;
    sql(
        db,
        "UPDATE central_identity.sessions SET created_at = created_at - interval '40 minutes',
           expires_at = expires_at - interval '1 minute' WHERE digest = $1",
        vec![Session::Browser(cookie).digest().to_vec().into()],
    )
    .await;
    assert_eq!(
        store
            .create_ai_runner_token(cookie, "rt-stale", &[bot], 90, &Token::generate().unwrap())
            .await,
        Err(Error::Rejected)
    );
    let mut ended = store
        .revoke_ai_runner_token(cookie, token_id)
        .await
        .unwrap();
    ended.sort_unstable();
    let mut minted: Vec<[u8; 32]> = held.iter().map(digest).collect();
    minted.sort_unstable();
    assert_eq!(ended, minted);
    assert_eq!(runner_sessions(db, bot).await, 0);
    for session in held {
        assert_eq!(store.renew_session(session).await, Err(Error::Rejected));
    }
    // Their sids went with them, each revoked at its host.
    for sid in sids {
        for (table, left) in [("gateway_sids", 0), ("gateway_revocations", 1)] {
            assert_eq!(
                count(
                    db,
                    &format!("SELECT count(*) AS n FROM central_identity.{table} WHERE sid = $1"),
                    vec![sid.to_vec().into()],
                )
                .await,
                left,
                "{table}"
            );
        }
    }
    assert_eq!(
        exchange(store, token, bot).await.err(),
        Some(Error::Rejected)
    );
    // Again: done, and nothing more; an unknown one is refused.
    assert_eq!(
        store.revoke_ai_runner_token(cookie, token_id).await,
        Ok(Vec::new())
    );
    assert_eq!(
        store.revoke_ai_runner_token(cookie, id(250)).await,
        Err(Error::Rejected)
    );
    let info = store
        .admin_ai_runner_tokens(cookie)
        .await
        .unwrap()
        .into_iter()
        .find(|t| t.id == token_id)
        .unwrap();
    assert!(info.revoked_at.is_some() && info.last_used_at.is_some());
    assert_eq!(info.revoked_by, Some(admin.id));
    assert_eq!(
        count(
            db,
            "SELECT count(*) AS n FROM central_identity.audit_events
             WHERE ai_runner_token_id = $1 AND action = 'ai.runner_token_revoked'
             AND detail IS NULL",
            vec![Uuid::from_bytes(token_id).into()],
        )
        .await,
        1
    );
}

/// Whose relay a socket ticket for a session is (`AccountStore::relay_runner`,
/// which Central's relay asks when it issues one): each runner session names
/// its AI account; a person's session and one that does not exist name none,
/// and so does a runner session whose token is revoked or expired, though the
/// exchange never leaves one alive then, and a session past its expiry.
async fn relayed(
    store: &AccountStore,
    db: &DatabaseConnection,
    (held, token_id): (&[Token], [u8; 16]),
    bot: [u8; 16],
    person: &Token,
) {
    for session in held {
        assert_eq!(store.relay_runner(session).await, Ok(Some(bot)));
    }
    assert_eq!(store.relay_runner(person).await, Ok(None));
    assert_eq!(
        store.relay_runner(&Token::generate().unwrap()).await,
        Ok(None)
    );
    let token = Uuid::from_bytes(token_id);
    for (aged, restored) in [
        (
            "UPDATE central_identity.ai_runner_tokens SET revoked_at = pg_catalog.clock_timestamp()
             WHERE id = $1",
            "UPDATE central_identity.ai_runner_tokens SET revoked_at = NULL WHERE id = $1",
        ),
        (
            "UPDATE central_identity.ai_runner_tokens SET created_at = created_at - interval '1 day',
               expires_at = pg_catalog.clock_timestamp() - interval '1 second' WHERE id = $1",
            "UPDATE central_identity.ai_runner_tokens
             SET expires_at = pg_catalog.clock_timestamp() + interval '90 days' WHERE id = $1",
        ),
    ] {
        sql(db, aged, vec![token.into()]).await;
        assert_eq!(store.relay_runner(&held[0]).await, Ok(None), "{aged}");
        sql(db, restored, vec![token.into()]).await;
        assert_eq!(store.relay_runner(&held[0]).await, Ok(Some(bot)), "{restored}");
    }
    // The first session past its expiry: no runner's relay, the others still.
    sql(
        db,
        "UPDATE central_identity.sessions SET created_at = created_at - interval '1 hour',
           proven_at = proven_at - interval '1 hour',
           expires_at = pg_catalog.clock_timestamp() - interval '1 second' WHERE digest = $1",
        vec![digest(&held[0]).to_vec().into()],
    )
    .await;
    assert_eq!(store.relay_runner(&held[0]).await, Ok(None));
    assert_eq!(store.relay_runner(&held[1]).await, Ok(Some(bot)));
}

/// A person's second native sign-in still replaces their first.
async fn people(
    store: &AccountStore,
    db: &DatabaseConnection,
    keys: &ServerKeys,
    identity: Identity,
) {
    let first = login(store, keys, identity).await;
    let second = login(store, keys, identity).await;
    assert_eq!(store.renew_session(&first).await, Err(Error::Rejected));
    store.renew_session(&second).await.unwrap();
    assert_eq!(
        count(
            db,
            "SELECT count(*) AS n FROM central_identity.sessions
             WHERE account_id = $1 AND kind = 'native'",
            vec![Uuid::from_bytes(identity.account).into()],
        )
        .await,
        1
    );
}

/// The runtime may change a runner token's revocation and last use, and
/// nothing else of it; it never deletes a token or a listing.
async fn grants(db: &DatabaseConnection) {
    let column = |c: &str, p: &str| {
        format!(
            "pg_catalog.has_column_privilege('central_runtime', 'central_identity.ai_runner_tokens', '{c}', '{p}')"
        )
    };
    let table = |t: &str, p: &str| {
        format!("pg_catalog.has_table_privilege('central_runtime', 'central_identity.{t}', '{p}')")
    };
    for (asked, allowed) in [
        (column("last_used_at", "UPDATE"), true),
        (column("revoked_by", "UPDATE"), true),
        (column("name", "UPDATE"), false),
        (column("digest", "UPDATE"), false),
        (column("expires_at", "UPDATE"), false),
        (column("revoked_at", "INSERT"), false),
        (table("ai_runner_tokens", "DELETE"), false),
        (table("ai_runner_accounts", "DELETE"), false),
        (table("ai_runner_accounts", "UPDATE"), false),
        (table("ai_runner_signins", "DELETE"), true),
        (table("ai_runner_signins", "UPDATE"), false),
    ] {
        assert_eq!(
            count(
                db,
                &format!("SELECT ({asked})::integer::bigint AS n"),
                vec![]
            )
            .await,
            i64::from(allowed),
            "{asked}"
        );
    }
}
