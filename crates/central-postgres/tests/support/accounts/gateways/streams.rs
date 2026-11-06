//! Sessions as gateways know them: what a `sid` is tied to, and how every end of one reaches
//! the gateway that holds it, numbered and without holes.
use super::{
    AccountStore, DatabaseConnection, Error, ServerKeys, Token, approval, enroll, login, metadata,
};
use central_api::wire::GatewayPurpose::{Admin, Admit};
use central_postgres::{GatewayPushResult, MAX_SIDS_PER_SESSION, PUSH_ATTEMPTS, REVOCATION_PAGE};
use sea_orm::{ConnectionTrait, TransactionTrait};

/// A `sid` of this file's own, apart from the other tests'. In service they are random.
fn sid(number: u8) -> [u8; 16] {
    let mut id = [number; 16];
    id[0] = 0xa5;
    id
}

/// A signed-in account whose move is closed, so that it can be admitted.
async fn account(store: &AccountStore, keys: &ServerKeys, name: &str) -> (Token, [u8; 16]) {
    let identity = enroll(store, keys, name, name).await;
    let session = login(store, keys, identity).await;
    store.confirm_restore(&session, 1).await.unwrap();
    store.close_legacy(name, identity).await.unwrap();
    (session, identity.account)
}

async fn grant(store: &AccountStore, session: &Token, stream: &str, fresh: u8) -> [u8; 16] {
    store
        .gateway_grant(session, &[stream], false, Admit, sid(fresh))
        .await
        .unwrap()
        .sid
}

pub(super) async fn run(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    a_sid_is_stable_and_separate(store, keys).await;
    a_grant_says_what_the_token_needs(store, keys).await;
    a_session_holds_a_bounded_number(store, keys).await;
    every_end_of_a_session_is_numbered(store, keys).await;
    a_rolled_back_end_leaves_no_hole(store, operator, keys).await;
    ending_sessions_cannot_deadlock(store, keys).await;
    a_page_is_bounded_and_says_where_it_stands(store, keys).await;
    old_numbers_go_by_age_and_never_from_the_middle(store, operator, keys).await;
    only_an_approved_gateway_with_a_path_is_called(store, operator, keys).await;
    attempts_are_bounded_and_a_crash_loses_nothing(store, operator, keys).await;
    a_page_that_aged_out_needs_no_call(store, operator, keys).await;
    a_removed_administrator_loses_the_admin_sid_only(store, operator, keys).await;
    a_disabled_gateway_gets_no_token(store, operator, keys).await;
    only_a_plain_path_is_registered(store, operator).await;
    the_runtime_cannot_rewrite_a_revocation(operator).await;
}

async fn a_sid_is_stable_and_separate(store: &AccountStore, keys: &ServerKeys) {
    let (session, _) = account(store, keys, "sid-stable").await;
    let held = grant(store, &session, "https://stable.example", 1).await;
    assert_eq!(held, sid(1));
    // A refresh keeps it, whatever was offered.
    assert_eq!(
        grant(store, &session, "https://stable.example", 2).await,
        sid(1)
    );
    // Another gateway never sees the same value.
    assert_eq!(
        grant(store, &session, "https://stable2.example", 3).await,
        sid(3)
    );
    let (other, _) = account(store, keys, "sid-other").await;
    assert!(
        matches!(
            store
                .gateway_grant(&other, &["https://stable.example"], false, Admit, sid(1))
                .await
                .err(),
            Some(Error::Unavailable)
        ),
        "a sid is one session's alone"
    );
    assert_eq!(
        grant(store, &other, "https://stable.example", 4).await,
        sid(4)
    );
    store.logout(&session).await.unwrap();
    store.logout(&other).await.unwrap();
}

async fn a_grant_says_what_the_token_needs(store: &AccountStore, keys: &ServerKeys) {
    let (session, account) = account(store, keys, "grant-says").await;
    let grant = store
        .gateway_grant(&session, &["https://says.example"], false, Admit, sid(5))
        .await
        .unwrap();
    assert_eq!(grant.account, account);
    assert_eq!(grant.legacy_id.as_deref(), Some("grant-says"));
    let now = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    )
    .unwrap();
    assert!(
        (grant.cap - (now + 12 * 3600)).abs() < 60,
        "the session's cap, not its next expiry"
    );
    // The official gateway has no Central administrators.
    assert!(matches!(
        store
            .gateway_grant(&session, &["https://says.example"], false, Admin, sid(6))
            .await
            .err(),
        Some(Error::Rejected)
    ));
    // An unknown gateway is refused, and its name opens no stream.
    assert!(matches!(
        store
            .gateway_grant(&session, &["https://nobody.example"], true, Admit, sid(6))
            .await
            .err(),
        Some(Error::Rejected)
    ));
    // A session that ended holds nothing, and a bearer that is no session gets nothing.
    store.logout(&session).await.unwrap();
    for bearer in [session, Token::generate().unwrap()] {
        assert!(matches!(
            store
                .gateway_grant(&bearer, &["https://says.example"], false, Admit, sid(7))
                .await
                .err(),
            Some(Error::Rejected)
        ));
    }
}

async fn a_session_holds_a_bounded_number(store: &AccountStore, keys: &ServerKeys) {
    let (session, _) = account(store, keys, "sid-bound").await;
    for index in 0..MAX_SIDS_PER_SESSION {
        let byte = u8::try_from(100 + index).unwrap();
        grant(
            store,
            &session,
            &format!("https://bound{index}.example"),
            byte,
        )
        .await;
    }
    assert!(matches!(
        store
            .gateway_grant(
                &session,
                &["https://bound-past.example"],
                false,
                Admit,
                sid(99)
            )
            .await
            .err(),
        Some(Error::Busy)
    ));
    // A refresh is no new one: the limit never locks a player out of a gateway they are at.
    assert_eq!(
        grant(store, &session, "https://bound0.example", 98).await,
        sid(100)
    );
    store.logout(&session).await.unwrap();
}

async fn every_end_of_a_session_is_numbered(store: &AccountStore, keys: &ServerKeys) {
    const STREAM: &str = "https://ends.example";
    let identity = enroll(store, keys, "ends", "ends").await;
    let first = login(store, keys, identity).await;
    store.confirm_restore(&first, 1).await.unwrap();
    store.close_legacy("ends", identity).await.unwrap();
    assert_eq!(store.last_revocation(STREAM).await.unwrap(), 0);
    grant(store, &first, STREAM, 10).await;
    // One active session per account: the replacing sign-in names the previous gateway.
    let second = login(store, keys, identity).await;
    let page = store.revocation_page(STREAM, 0).await.unwrap();
    assert_eq!((page.from, page.last), (1, 1));
    assert_eq!(page.sids, vec![sid(10)]);
    // A sign-out, the same again; its number follows without a hole.
    grant(store, &second, STREAM, 11).await;
    store.logout(&second).await.unwrap();
    let page = store.revocation_page(STREAM, 1).await.unwrap();
    assert_eq!((page.from, page.last), (2, 2));
    assert_eq!(page.sids, vec![sid(11)]);
    // A session that ended is not ended twice.
    store.logout(&second).await.unwrap();
    assert_eq!(store.last_revocation(STREAM).await.unwrap(), 2);
}

async fn a_rolled_back_end_leaves_no_hole(
    store: &AccountStore,
    operator: &DatabaseConnection,
    keys: &ServerKeys,
) {
    const STREAM: &str = "https://rollback.example";
    let (session, _) = account(store, keys, "rollback").await;
    grant(store, &session, STREAM, 20).await;
    let tx = operator.begin().await.unwrap();
    tx.execute_unprepared(&format!(
        "DELETE FROM central_identity.gateway_sids WHERE stream = '{STREAM}'"
    ))
    .await
    .unwrap();
    tx.rollback().await.unwrap();
    assert_eq!(store.last_revocation(STREAM).await.unwrap(), 0);
    store.logout(&session).await.unwrap();
    let page = store.revocation_page(STREAM, 0).await.unwrap();
    assert_eq!((page.from, page.last, page.sids.len()), (1, 1, 1));
}

async fn ending_sessions_cannot_deadlock(store: &AccountStore, keys: &ServerKeys) {
    const ONE: &str = "https://deadlock-a.example";
    const TWO: &str = "https://deadlock-b.example";
    for round in 0_u8..8 {
        let (left, _) = account(store, keys, &format!("deadlock-l{round}")).await;
        let (right, _) = account(store, keys, &format!("deadlock-r{round}")).await;
        // Opposite orders: without one locking order these two would wait for each other.
        grant(store, &left, ONE, 30 + round * 4).await;
        grant(store, &left, TWO, 31 + round * 4).await;
        grant(store, &right, TWO, 32 + round * 4).await;
        grant(store, &right, ONE, 33 + round * 4).await;
        let (ended_left, ended_right) = tokio::join!(store.logout(&left), store.logout(&right));
        ended_left.unwrap();
        ended_right.unwrap();
    }
    for stream in [ONE, TWO] {
        let page = store.revocation_page(stream, 0).await.unwrap();
        assert_eq!((page.from, page.last, page.sids.len()), (1, 16, 16));
    }
}

async fn a_page_is_bounded_and_says_where_it_stands(store: &AccountStore, keys: &ServerKeys) {
    const STREAM: &str = "https://pages.example";
    // A stream nobody has heard of stands at nothing.
    let page = store
        .revocation_page("https://nobody.example", 0)
        .await
        .unwrap();
    assert_eq!((page.last, page.floor, page.sids.len()), (0, 1, 0));
    let (session, _) = account(store, keys, "pages").await;
    grant(store, &session, STREAM, 60).await;
    store.logout(&session).await.unwrap();
    let page = store.revocation_page(STREAM, 0).await.unwrap();
    assert_eq!((page.from, page.last, page.floor), (1, 1, 1));
    // Up to date: nothing to send, and the position still answers.
    let page = store.revocation_page(STREAM, 1).await.unwrap();
    assert_eq!((page.last, page.floor, page.sids.len()), (1, 1, 0));
    // A number past the end, and one too large for storage.
    assert!(
        store
            .revocation_page(STREAM, 9)
            .await
            .unwrap()
            .sids
            .is_empty()
    );
    assert!(store.revocation_page(STREAM, u64::MAX).await.is_err());
    assert_eq!(REVOCATION_PAGE, 256);
}

async fn old_numbers_go_by_age_and_never_from_the_middle(
    store: &AccountStore,
    operator: &DatabaseConnection,
    keys: &ServerKeys,
) {
    const STREAM: &str = "https://sweep.example";
    for number in 70_u8..73 {
        let (session, _) = account(store, keys, &format!("sweep-{number}")).await;
        grant(store, &session, STREAM, number).await;
        store.logout(&session).await.unwrap();
    }
    // An hour and a half is inside a token's life and the tolerance: never forgotten.
    backdate(operator, STREAM, 1, "90 minutes").await;
    store.sweep_revocations().await.unwrap();
    assert_eq!(store.revocation_page(STREAM, 0).await.unwrap().floor, 1);
    // A clock step can age a later number before an earlier one. The prefix goes with it.
    backdate(operator, STREAM, 2, "3 hours").await;
    store.sweep_revocations().await.unwrap();
    let page = store.revocation_page(STREAM, 0).await.unwrap();
    assert_eq!((page.floor, page.from, page.last), (3, 3, 3));
    assert_eq!(page.sids, vec![sid(72)]);
}

async fn backdate(operator: &DatabaseConnection, stream: &str, seq: u8, age: &str) {
    operator
        .execute_unprepared(&format!(
            "UPDATE central_identity.gateway_revocations
             SET revoked_at = pg_catalog.clock_timestamp() - interval '{age}'
             WHERE stream = '{stream}' AND seq = {seq}"
        ))
        .await
        .unwrap();
}

/// A registered gateway with one revocation waiting.
async fn gateway(
    store: &AccountStore,
    keys: &ServerKeys,
    name: &str,
    origin: &str,
    fresh: u8,
) -> [u8; 16] {
    let (owner, _) = account(store, keys, name).await;
    let gateway = store
        .register_gateway(
            &owner,
            origin,
            &metadata(origin),
            &Token::generate().unwrap(),
            &Token::generate().unwrap(),
            true,
        )
        .await
        .unwrap();
    store
        .gateway_grant(&owner, &[origin], true, Admit, sid(fresh))
        .await
        .unwrap();
    store.logout(&owner).await.unwrap();
    gateway
}

async fn no_push_due(store: &AccountStore) {
    assert!(store.claim_gateway_push().await.unwrap().is_none());
}

/// Let a claimed gateway be due again at once.
async fn due(operator: &DatabaseConnection, origin: &str) {
    operator
        .execute_unprepared(&format!(
            "UPDATE central_identity.gateway_streams SET next_attempt_at = NULL
             WHERE origin = '{origin}'"
        ))
        .await
        .unwrap();
}

async fn only_an_approved_gateway_with_a_path_is_called(
    store: &AccountStore,
    operator: &DatabaseConnection,
    keys: &ServerKeys,
) {
    const ORIGIN: &str = "http://127.0.0.1:29101";
    let id = gateway(store, keys, "push-approved", ORIGIN, 80).await;
    // Pending, with no path: nothing.
    no_push_due(store).await;
    store
        .register_invalidate(ORIGIN, "/auth/central/invalidate")
        .await
        .unwrap();
    // A path alone does not do it: a pending gateway catches up instead of being called.
    no_push_due(store).await;
    approval(operator, id, false).await;
    no_push_due(store).await;
    approval(operator, id, true).await;
    let push = store.claim_gateway_push().await.unwrap().unwrap();
    assert_eq!(push.origin, ORIGIN);
    assert_eq!(push.path, "/auth/central/invalidate");
    assert_eq!((push.from, push.through), (1, 1));
    assert_eq!(push.sids, vec![sid(80)]);
    // Claimed: it is not handed to a second sender while the first is calling.
    no_push_due(store).await;
    store
        .finish_gateway_push(ORIGIN, push.through, GatewayPushResult::Delivered)
        .await
        .unwrap();
    due(operator, ORIGIN).await;
    no_push_due(store).await;
}

async fn attempts_are_bounded_and_a_crash_loses_nothing(
    store: &AccountStore,
    operator: &DatabaseConnection,
    keys: &ServerKeys,
) {
    const ORIGIN: &str = "http://127.0.0.1:29102";
    let id = gateway(store, keys, "push-retry", ORIGIN, 81).await;
    store
        .register_invalidate(ORIGIN, "/invalidate")
        .await
        .unwrap();
    approval(operator, id, true).await;
    for attempt in 1..=PUSH_ATTEMPTS {
        let push = store.claim_gateway_push().await.unwrap().unwrap();
        // A retry is the same numbers.
        assert_eq!((push.from, push.through), (1, 1), "attempt {attempt}");
        store
            .finish_gateway_push(ORIGIN, push.through, GatewayPushResult::Retry)
            .await
            .unwrap();
        // It waits its turn: nothing is due until the backoff has passed.
        no_push_due(store).await;
        due(operator, ORIGIN).await;
    }
    // Given up: the numbers stay in the log for catch-up, and nothing more is sent.
    no_push_due(store).await;
    assert_eq!(
        store.revocation_page(ORIGIN, 0).await.unwrap().sids,
        vec![sid(81)]
    );

    // A sender that dies mid-call at the last attempt must not stop this gateway for good.
    const STUCK: &str = "http://127.0.0.1:29103";
    let id = gateway(store, keys, "push-stuck", STUCK, 82).await;
    store
        .register_invalidate(STUCK, "/invalidate")
        .await
        .unwrap();
    approval(operator, id, true).await;
    operator
        .execute_unprepared(&format!(
            "UPDATE central_identity.gateway_streams SET attempts = {PUSH_ATTEMPTS},
             next_attempt_at = pg_catalog.clock_timestamp() - interval '1 second'
             WHERE origin = '{STUCK}'"
        ))
        .await
        .unwrap();
    let push = store.claim_gateway_push().await.unwrap().unwrap();
    assert_eq!(push.origin, STUCK);
    store
        .finish_gateway_push(STUCK, push.through, GatewayPushResult::Refused)
        .await
        .unwrap();
    // Refused: not asked again, and it rests, however much more ends there.
    due(operator, STUCK).await;
    no_push_due(store).await;
}

async fn a_page_that_aged_out_needs_no_call(
    store: &AccountStore,
    operator: &DatabaseConnection,
    keys: &ServerKeys,
) {
    const ORIGIN: &str = "http://127.0.0.1:29104";
    let id = gateway(store, keys, "push-aged", ORIGIN, 83).await;
    store
        .register_invalidate(ORIGIN, "/invalidate")
        .await
        .unwrap();
    approval(operator, id, true).await;
    backdate(operator, ORIGIN, 1, "3 hours").await;
    store.sweep_revocations().await.unwrap();
    let push = store.claim_gateway_push().await.unwrap().unwrap();
    assert!(push.sids.is_empty());
    assert_eq!(push.through, 1, "the cursor can pass what aged out");
    store
        .finish_gateway_push(ORIGIN, push.through, GatewayPushResult::Delivered)
        .await
        .unwrap();
    due(operator, ORIGIN).await;
    no_push_due(store).await;
}

async fn a_removed_administrator_loses_the_admin_sid_only(
    store: &AccountStore,
    operator: &DatabaseConnection,
    keys: &ServerKeys,
) {
    const ORIGIN: &str = "http://127.0.0.1:29105";
    let (owner, owner_account) = account(store, keys, "admin-owner").await;
    let (member, member_account) = account(store, keys, "admin-member").await;
    let id = store
        .register_gateway(
            &owner,
            ORIGIN,
            &metadata(ORIGIN),
            &Token::generate().unwrap(),
            &Token::generate().unwrap(),
            true,
        )
        .await
        .unwrap();
    approval(operator, id, true).await;
    // Admin needs membership at issue.
    assert!(matches!(
        store
            .gateway_grant(&member, &[ORIGIN], true, Admin, sid(90))
            .await
            .err(),
        Some(Error::Rejected)
    ));
    store
        .set_gateway_admins(&owner, id, 1, &[owner_account, member_account])
        .await
        .unwrap();
    let admin = store
        .gateway_grant(&member, &[ORIGIN], true, Admin, sid(91))
        .await
        .unwrap();
    let admit = store
        .gateway_grant(&member, &[ORIGIN], true, Admit, sid(92))
        .await
        .unwrap();
    assert_ne!(admin.sid, admit.sid);
    store
        .set_gateway_admins(&owner, id, 2, &[owner_account])
        .await
        .unwrap();
    let page = store.revocation_page(ORIGIN, 0).await.unwrap();
    assert_eq!(
        page.sids,
        vec![sid(91)],
        "the admin sid, and not the player's"
    );
    assert_eq!(
        store
            .gateway_grant(&member, &[ORIGIN], true, Admit, sid(93))
            .await
            .unwrap()
            .sid,
        sid(92)
    );
    store.logout(&owner).await.unwrap();
    store.logout(&member).await.unwrap();
}

/// That disabling one ends its `sid`s is proved through the store's real path, in `tokens`.
async fn a_disabled_gateway_gets_no_token(
    store: &AccountStore,
    operator: &DatabaseConnection,
    keys: &ServerKeys,
) {
    const ORIGIN: &str = "http://127.0.0.1:29106";
    let id = gateway(store, keys, "disabled-owner", ORIGIN, 94).await;
    approval(operator, id, false).await;
    let (player, _) = account(store, keys, "disabled-player").await;
    // A disabled gateway gets no token.
    assert!(matches!(
        store
            .gateway_grant(&player, &[ORIGIN], true, Admit, sid(95))
            .await
            .err(),
        Some(Error::Rejected)
    ));
    approval(operator, id, true).await;
    store
        .gateway_grant(&player, &[ORIGIN], true, Admit, sid(95))
        .await
        .unwrap();
    store.logout(&player).await.unwrap();
}

async fn only_a_plain_path_is_registered(store: &AccountStore, operator: &DatabaseConnection) {
    const ORIGIN: &str = "https://paths.example";
    for path in [
        "",
        "/",
        "//evil.example/x",
        "/a/../b",
        "/a?b",
        "https://evil.example/x",
    ] {
        assert!(
            matches!(
                store.register_invalidate(ORIGIN, path).await.err(),
                Some(Error::Rejected)
            ),
            "{path:?}"
        );
    }
    store.register_invalidate(ORIGIN, "/a/b").await.unwrap();
    // The table holds the rule itself, whoever writes.
    for path in ["/../a", "https://evil.example", "/a/"] {
        assert!(
            operator
                .execute_unprepared(&format!(
                    "UPDATE central_identity.gateway_streams SET invalidate_path = '{path}'
                     WHERE origin = '{ORIGIN}'"
                ))
                .await
                .is_err(),
            "{path}"
        );
    }
}

async fn the_runtime_cannot_rewrite_a_revocation(operator: &DatabaseConnection) {
    for privilege in ["UPDATE", "TRUNCATE"] {
        let row = operator
            .query_one_raw(sea_orm::Statement::from_string(
                sea_orm::DbBackend::Postgres,
                format!(
                    "SELECT pg_catalog.has_table_privilege('central_runtime',
                     'central_identity.gateway_revocations', '{privilege}') AS allowed"
                ),
            ))
            .await
            .unwrap()
            .unwrap();
        assert!(!row.try_get::<bool>("", "allowed").unwrap(), "{privilege}");
    }
}
