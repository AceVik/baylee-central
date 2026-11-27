//! The operator's command for AI players (`central-service
//! --ai-players-stdin`, `docs/operations.md` §"AI players"): one plan,
//! applied whole or not at all under the migrator's login, through the
//! administration's own functions, with the operator as the audit's actor.
use super::ai::{count, forget, person, sql};
use super::{AccountStore, DatabaseConnection, Error, ServerKeys, Token};
use central_api::admin::AiPlayerSettings;
use central_api::ranked::ai::AiKind;
use central_api::social::{ContactAction, SocialRequest};
use central_postgres::{
    AiPlan, AiPlayerPlan, AiSettingsPlan, PlayerOutcome, RemoveTarget, RunnerTokenPlan,
    ai_runner_tokens, apply_ai_plan,
};
use sea_orm::prelude::Uuid;

/// Tags this suite uses: above every drawn tag of the other suites' fixtures.
const ALPHA: u32 = 0x7fff_0a01;
const BETA: u32 = 0x7fff_0a02;
const PERSON: u32 = 0x7fff_0a03;
const SEEN: u32 = 0x7fff_0a04;

fn house(name: &str, label: &str) -> AiPlayerSettings {
    AiPlayerSettings {
        kind: AiKind::House,
        profile: "house:expert".into(),
        label: label.into(),
        max_games: 4,
        daily_games: None,
        display_name: name.into(),
    }
}

fn wanted(name: &str, tag: u32, label: &str) -> AiPlayerPlan {
    AiPlayerPlan {
        settings: house(name, label),
        tag,
        enabled: Some(true),
    }
}

fn runner(name: &str, accounts: &[&str]) -> RunnerTokenPlan {
    RunnerTokenPlan {
        name: name.into(),
        accounts: accounts.iter().map(|&a| a.to_owned()).collect(),
        days: 90,
    }
}

/// The operator's audit events of `action`.
async fn operator_events(db: &DatabaseConnection, action: &str) -> i64 {
    count(
        db,
        "SELECT count(*) AS n FROM central_identity.audit_events
         WHERE actor_kind = 'operator' AND actor_account IS NULL AND action = $1",
        vec![action.into()],
    )
    .await
}

pub(super) async fn run(store: &AccountStore, db: &DatabaseConnection, keys: &ServerKeys) {
    let (someone_session, someone, _) = person(store, keys, "op-fixture-person").await;
    let (_, seen, _) = person(store, keys, "op-fixture-seen").await;
    // A moved account may have no tag or name yet; these hold the ones the
    // tests ask for: one an AI asks for, and a person others find.
    for (account, tag, name) in [(someone, PERSON, "OpSomeone"), (seen, SEEN, "OpSeen")] {
        sql(
            db,
            "UPDATE central_identity.accounts SET tag = $2, display_name = $3,
               display_name_at = pg_catalog.clock_timestamp() WHERE id = $1",
            vec![
                Uuid::from_bytes(account).into(),
                i32::try_from(tag).unwrap().into(),
                name.into(),
            ],
        )
        .await;
    }
    let (alpha, token_id, session) = first(store, db).await;
    again(db, alpha).await;
    token_bound(store, db, alpha).await;
    unfindable(store, &someone_session).await;
    refusals(db, alpha).await;
    revoking(store, db, token_id, &session).await;
    removing(store, db).await;
    clean(db, alpha).await;
    forget(db, &[someone, seen]).await;
}

/// One plan: a player made and switched on, calibration's settings, and a
/// token that works at once. Answers the player, the token and its session.
async fn first(store: &AccountStore, db: &DatabaseConnection) -> ([u8; 16], [u8; 16], Token) {
    let secret = Token::generate().unwrap();
    let first = AiPlan {
        players: vec![wanted("KI-Op-Alpha", ALPHA, "Haus-KI · Expert")],
        settings: Some(AiSettingsPlan {
            fill_in: Some(false),
            carousel_games: Some(8),
            carousel_daily: Some(10_000),
            ..AiSettingsPlan::default()
        }),
        revoke: Vec::new(),
        remove: Vec::new(),
        token: Some(runner("op-runner-1", &["KI-Op-Alpha"])),
    };
    let applied = apply_ai_plan(db, &first, Some(&secret)).await.unwrap();
    assert_eq!(applied.players.len(), 1);
    assert_eq!(applied.players[0].outcome, PlayerOutcome::Created);
    assert!(applied.players[0].switched);
    let alpha = applied.players[0].account;
    let settings = applied.settings.unwrap();
    assert!(!settings.fill_in);
    assert_eq!(
        (settings.carousel_games, settings.carousel_daily),
        (8, 10_000)
    );
    assert_eq!(settings.fill_in_after, 60, "what the plan left out stays");
    let (token_id, _) = applied.token.unwrap();
    for action in [
        "ai.created",
        "ai.enabled",
        "ai.settings_changed",
        "ai.runner_token_created",
    ] {
        assert_eq!(operator_events(db, action).await, 1, "{action}");
    }
    // The account is the administration's kind of AI account: no password,
    // switched on, and the token is the operator's, by digest only.
    assert_eq!(
        count(
            db,
            "SELECT count(*) AS n FROM central_identity.accounts a
             JOIN central_identity.ai_players p ON p.account_id = a.id
             WHERE a.id = $1 AND a.tag = $2 AND a.display_name = 'KI-Op-Alpha'
             AND a.username_key LIKE 'ai-%' AND a.restored AND p.enabled AND p.set_by IS NULL",
            vec![
                Uuid::from_bytes(alpha).into(),
                i32::try_from(ALPHA).unwrap().into()
            ],
        )
        .await,
        1
    );
    assert_eq!(
        count(
            db,
            "SELECT count(*) AS n FROM central_identity.ai_runner_tokens
             WHERE id = $1 AND created_by IS NULL AND digest = $2",
            vec![
                Uuid::from_bytes(token_id).into(),
                secret.digest().to_vec().into()
            ],
        )
        .await,
        1
    );
    // The token works: a runner session of the account it lists.
    let session = Token::generate().unwrap();
    store
        .ai_runner_session(&secret, alpha, &session)
        .await
        .unwrap();
    (alpha, token_id, session)
}

/// Again: reported, not made twice; a new label is a change.
async fn again(db: &DatabaseConnection, alpha: [u8; 16]) {
    let same = AiPlan {
        players: vec![wanted("KI-Op-Alpha", ALPHA, "Haus-KI · Expert")],
        ..AiPlan::default()
    };
    let applied = apply_ai_plan(db, &same, None).await.unwrap();
    assert_eq!(applied.players[0].outcome, PlayerOutcome::Unchanged);
    assert!(!applied.players[0].switched);
    assert_eq!(applied.players[0].account, alpha);
    let relabelled = AiPlan {
        players: vec![wanted("KI-Op-Alpha", ALPHA, "Haus-KI · Meister")],
        ..AiPlan::default()
    };
    let applied = apply_ai_plan(db, &relabelled, None).await.unwrap();
    assert_eq!(applied.players[0].outcome, PlayerOutcome::Changed);
}

/// Revoked by the operator: its session ends in the same transaction, and
/// the listing names it as the operator's, revoked.
async fn revoking(
    store: &AccountStore,
    db: &DatabaseConnection,
    token_id: [u8; 16],
    session: &Token,
) {
    let revoked = AiPlan {
        revoke: vec![token_id],
        ..AiPlan::default()
    };
    let applied = apply_ai_plan(db, &revoked, None).await.unwrap();
    assert_eq!(applied.revoked, vec![(token_id, 1)]);
    assert_eq!(
        store.renew_session(session).await,
        Err(Error::Rejected),
        "the revoked token's session is gone"
    );
    let listed = ai_runner_tokens(db).await.unwrap();
    let ours = listed.iter().find(|t| t.id == token_id).unwrap();
    assert_eq!(ours.created_by, None);
    assert!(ours.revoked_at.is_some());
    assert_eq!(ours.revoked_by, None);
    assert_eq!(ours.accounts.len(), 1);
}

/// Clean up: this suite shares the instance.
async fn clean(db: &DatabaseConnection, alpha: [u8; 16]) {
    let restore = AiPlan {
        settings: Some(AiSettingsPlan {
            fill_in: Some(true),
            carousel_games: Some(0),
            carousel_daily: Some(0),
            ..AiSettingsPlan::default()
        }),
        ..AiPlan::default()
    };
    apply_ai_plan(db, &restore, None).await.unwrap();
    forget(db, &[alpha]).await;
    sql(
        db,
        "DELETE FROM central_identity.ai_runner_tokens WHERE name LIKE 'op-runner-%'",
        vec![],
    )
    .await;
    sql(
        db,
        "DELETE FROM central_identity.audit_events WHERE actor_kind = 'operator'
         AND (account_id = $1 OR ai_runner_token_id IS NOT NULL OR action = 'ai.settings_changed')",
        vec![Uuid::from_bytes(alpha).into()],
    )
    .await;
}

/// Each ambiguous or wrong plan is refused whole, saying what and where:
/// the new player it also asks for is not made, and nothing else changes.
async fn refusals(db: &DatabaseConnection, alpha: [u8; 16]) {
    let beta = || wanted("KI-Op-Beta", BETA, "Haus-KI · Expert");
    let mut other_profile = wanted("KI-Op-Alpha", ALPHA, "Haus-KI · Meister");
    other_profile.settings.profile = "house:novice".into();
    let secret = Token::generate().unwrap();
    for (case, plan, token, refusal, said) in [
        (
            "a name too long, before the database is asked",
            vec![beta(), wanted("KI-Op-Much-Too-Long", ALPHA + 9, "x")],
            None,
            Error::Rejected,
            "players[1].display_name \"KI-Op-Much-Too-Long\": 19 characters, at most 16",
        ),
        (
            "the tag under another name",
            vec![beta(), wanted("KI-Op-Gamma", ALPHA, "x")],
            None,
            Error::Conflict,
            "the tag is AI player \"KI-Op-Alpha\"'s",
        ),
        (
            "the name under another tag",
            vec![beta(), wanted("KI-Op-Alpha", ALPHA + 7, "x")],
            None,
            Error::Conflict,
            "an AI player of that name has the tag #7fff0a01",
        ),
        (
            "a person's tag",
            vec![beta(), wanted("KI-Op-Gamma", PERSON, "x")],
            None,
            Error::Conflict,
            "players[1] \"KI-Op-Gamma\"#7fff0a03: the tag is a person's",
        ),
        (
            "another mind",
            vec![beta(), other_profile.clone()],
            None,
            Error::Conflict,
            "it plays as house \"house:expert\"; a mind is fixed",
        ),
        (
            "a token for a name no AI holds",
            vec![beta()],
            Some(runner("op-runner-2", &["KI-Op-Nobody"])),
            Error::Rejected,
            "token.accounts[0] \"KI-Op-Nobody\": no AI player has that name",
        ),
        (
            "a second live token of one name",
            vec![beta()],
            Some(runner("op-runner-1", &["KI-Op-Alpha"])),
            Error::Conflict,
            "token.name \"op-runner-1\": a live token has that name",
        ),
    ] {
        let secret = token.is_some().then_some(&secret);
        let plan = AiPlan {
            players: plan,
            token,
            ..AiPlan::default()
        };
        let fault = apply_ai_plan(db, &plan, secret).await.unwrap_err();
        assert_eq!(fault.error, refusal, "{case}");
        assert!(
            fault.faults.iter().any(|f| f.contains(said)),
            "{case}: {:?}",
            fault.faults
        );
        assert_eq!(
            count(
                db,
                "SELECT count(*) AS n FROM central_identity.accounts WHERE display_name = 'KI-Op-Beta'",
                vec![],
            )
            .await,
            0,
            "{case}: nothing of the plan stays"
        );
    }
    // An unknown token to revoke refuses the plan too.
    let unknown = AiPlan {
        players: vec![beta()],
        revoke: vec![[9; 16]],
        ..AiPlan::default()
    };
    let fault = apply_ai_plan(db, &unknown, None).await.unwrap_err();
    assert_eq!(fault.error, Error::Rejected);
    assert!(
        fault.faults[0].ends_with(": no runner token has that id"),
        "{fault:?}"
    );
    // The account stays as the last good plan left it.
    assert_eq!(label(db, alpha).await, "Haus-KI · Meister");
}

async fn label(db: &DatabaseConnection, account: [u8; 16]) -> String {
    use sea_orm::{ConnectionTrait, DbBackend, Statement};
    db.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT label FROM central_identity.ai_players WHERE account_id = $1",
        vec![Uuid::from_bytes(account).into()],
    ))
    .await
    .unwrap()
    .unwrap()
    .try_get("", "label")
    .unwrap()
}

/// A person finds no AI player by its tag or by a search, and can neither
/// ask one for contact nor block one: the AI is no one in social requests,
/// so no social reply names it without its badge.
async fn unfindable(store: &AccountStore, someone: &Token) {
    let session = central_postgres::Session::Native(someone);
    let names = |answer: &central_api::social::SocialResponse, name: &str| {
        String::from_utf8(central_api::encode(answer).unwrap())
            .unwrap()
            .contains(name)
    };
    let names_it = |answer: &central_api::social::SocialResponse| names(answer, "KI-Op-Alpha");
    // The control: the same person finds a person by tag.
    let found = store
        .social(session, SocialRequest::Lookup { tag: SEEN })
        .await
        .unwrap();
    assert!(names(&found, "OpSeen"));
    for request in [
        SocialRequest::Lookup { tag: ALPHA },
        SocialRequest::Search {
            query: "KI-Op".into(),
        },
        SocialRequest::Search {
            query: format!("#{ALPHA:x}"),
        },
    ] {
        if let Ok(answer) = store.social(session, request).await {
            assert!(!names_it(&answer));
        }
    }
    for change in [ContactAction::Request, ContactAction::Block] {
        assert!(
            store
                .social(session, SocialRequest::Contact { tag: ALPHA, change })
                .await
                .is_err()
        );
    }
    // The control: the same person may ask a person for contact.
    store
        .social(
            session,
            SocialRequest::Contact {
                tag: SEEN,
                change: ContactAction::Request,
            },
        )
        .await
        .unwrap();
}

/// A runner session never outlives its token: minted no later than the
/// token's end, and renewed no more once it has passed.
async fn token_bound(store: &AccountStore, db: &DatabaseConnection, alpha: [u8; 16]) {
    let secret = Token::generate().unwrap();
    let plan = AiPlan {
        token: Some(runner("op-runner-short", &["KI-Op-Alpha"])),
        ..AiPlan::default()
    };
    let (token_id, _) = apply_ai_plan(db, &plan, Some(&secret))
        .await
        .unwrap()
        .token
        .unwrap();
    let id = Uuid::from_bytes(token_id);
    // Ten minutes left: the session gets ten, not thirty.
    sql(
        db,
        "UPDATE central_identity.ai_runner_tokens
         SET expires_at = pg_catalog.clock_timestamp() + interval '10 minutes' WHERE id = $1",
        vec![id.into()],
    )
    .await;
    let session = Token::generate().unwrap();
    let minted = store
        .ai_runner_session(&secret, alpha, &session)
        .await
        .unwrap();
    assert_eq!(
        count(
            db,
            "SELECT count(*) AS n FROM central_identity.sessions s
             JOIN central_identity.ai_runner_tokens r ON r.id = s.runner_token
             WHERE r.id = $1 AND s.expires_at <= r.expires_at
             AND s.expires_at < pg_catalog.clock_timestamp() + interval '11 minutes'",
            vec![id.into()],
        )
        .await,
        1
    );
    let renewed = store.renew_session(&session).await.unwrap();
    assert!(
        renewed <= minted.expires_at + 1,
        "renewal stays within the token"
    );
    // The token past its end: the live session renews no more.
    sql(
        db,
        "UPDATE central_identity.ai_runner_tokens
         SET expires_at = GREATEST(created_at + interval '1 microsecond',
             pg_catalog.clock_timestamp() - interval '1 millisecond') WHERE id = $1",
        vec![id.into()],
    )
    .await;
    assert_eq!(store.renew_session(&session).await, Err(Error::Rejected));
    // An expired token mints nothing either.
    assert_eq!(
        store
            .ai_runner_session(&secret, alpha, &Token::generate().unwrap())
            .await
            .err(),
        Some(Error::Rejected)
    );
}

/// The operator removes AI players that have never played, by name and tag
/// or by id, through the administration's own removal: their accounts go,
/// their runner sessions with them, each audited with the operator as the
/// actor. One no AI player is, or one named twice, refuses the whole plan.
async fn removing(store: &AccountStore, db: &DatabaseConnection) {
    let (gone, spare, twice) = (ALPHA + 0x10, ALPHA + 0x11, ALPHA + 0x12);
    let secret = Token::generate().unwrap();
    let made = apply_ai_plan(
        db,
        &AiPlan {
            players: vec![
                wanted("KI-Op-Gone", gone, "Haus-KI"),
                wanted("KI-Op-Spare", spare, "Haus-KI"),
                wanted("KI-Op-Twice", twice, "Haus-KI"),
            ],
            token: Some(runner("op-runner-gone", &["KI-Op-Gone"])),
            ..AiPlan::default()
        },
        Some(&secret),
    )
    .await
    .unwrap();
    let [gone_id, spare_id, twice_id] = [0, 1, 2].map(|i| made.players[i].account);
    let session = Token::generate().unwrap();
    store
        .ai_runner_session(&secret, gone_id, &session)
        .await
        .unwrap();
    let named = |name: &str, tag: u32| RemoveTarget::Named {
        display_name: name.into(),
        tag,
    };
    // Every fault at once, and nothing removed.
    let fault = apply_ai_plan(
        db,
        &AiPlan {
            remove: vec![
                named("KI-Op-Gone", gone),
                named("KI-Op-Nobody", ALPHA + 0x13),
                named("KI-Op-Twice", twice),
                RemoveTarget::Id(twice_id),
                RemoveTarget::Id([9; 16]),
            ],
            ..AiPlan::default()
        },
        None,
    )
    .await
    .unwrap_err();
    assert_eq!(fault.error, Error::Rejected);
    assert_eq!(
        fault.faults,
        [
            "remove[1] \"KI-Op-Nobody#7fff0a14\": no AI player has that name and tag".to_owned(),
            "remove[3] \"KI-Op-Twice#7fff0a13\": the same AI player as remove[2]".to_owned(),
            format!(
                "remove[4] {}: no AI player has that id",
                Uuid::from_bytes([9; 16])
            ),
        ]
    );
    store.renew_session(&session).await.unwrap();
    removed_go(store, db, &session, [(gone, gone_id), (spare, spare_id)]).await;
    // Clean up: the third, and what was written about all three.
    forget(db, &[twice_id]).await;
    let all: Vec<Uuid> = [gone_id, spare_id, twice_id].map(Uuid::from_bytes).to_vec();
    sql(
        db,
        "DELETE FROM central_identity.audit_events WHERE account_id = ANY($1)",
        vec![all.into()],
    )
    .await;
}

/// By name and tag, and by id: both go, the runner's session with the
/// first, each audited as the operator's; asked again, it is said, not done.
async fn removed_go(
    store: &AccountStore,
    db: &DatabaseConnection,
    session: &Token,
    [(gone, gone_id), (spare, spare_id)]: [(u32, [u8; 16]); 2],
) {
    let named = |name: &str, tag: u32| RemoveTarget::Named {
        display_name: name.into(),
        tag,
    };
    let removed = apply_ai_plan(
        db,
        &AiPlan {
            remove: vec![named("KI-Op-Gone", gone), RemoveTarget::Id(spare_id)],
            ..AiPlan::default()
        },
        None,
    )
    .await
    .unwrap()
    .removed;
    assert_eq!(removed.len(), 2);
    assert_eq!(
        (removed[0].account, removed[0].sessions),
        (gone_id, 1),
        "{removed:?}"
    );
    assert_eq!(
        (
            removed[1].display_name.as_str(),
            removed[1].tag,
            removed[1].sessions
        ),
        ("KI-Op-Spare", spare, 0)
    );
    assert_eq!(store.renew_session(session).await, Err(Error::Rejected));
    let ids: Vec<Uuid> = [gone_id, spare_id].map(Uuid::from_bytes).to_vec();
    assert_eq!(
        count(
            db,
            "SELECT count(*) AS n FROM central_identity.accounts WHERE id = ANY($1)",
            vec![ids.clone().into()],
        )
        .await,
        0
    );
    assert_eq!(
        count(
            db,
            "SELECT count(*) AS n FROM central_identity.audit_events
             WHERE action = 'ai.removed' AND actor_kind = 'operator' AND account_id = ANY($1)",
            vec![ids.clone().into()],
        )
        .await,
        2
    );
    // Gone means gone: asking again is said, not done twice.
    let again = apply_ai_plan(
        db,
        &AiPlan {
            remove: vec![named("KI-Op-Gone", gone)],
            ..AiPlan::default()
        },
        None,
    )
    .await
    .unwrap_err();
    assert_eq!(
        again.faults,
        ["remove[0] \"KI-Op-Gone#7fff0a11\": no AI player has that name and tag"]
    );
}
