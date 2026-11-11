//! What gateways report (migration 000025, `docs/gateway-registry.md`
//! §"Activity and the public figures"): a pulse is a heartbeat with three
//! bounded counts, by the heartbeat's credential and under its limit; the
//! figures show them for approved gateways that are online and reported
//! within 90 seconds, and for no other; a plain heartbeat clears them; and
//! the number of accounts is the enrolled ones.
use super::tokens::moved;
use super::{
    AccountStore, DatabaseConnection, Error, ServerKeys, Token, approval, enroll, metadata,
};
use central_api::LEGACY_ORIGIN;
use central_api::gateway::{GatewayActivity, GatewayMetadata};
use central_api::owner::GatewayScopes;
use sea_orm::{ConnectionTrait, DbBackend, Statement, prelude::Uuid};

const PAIR: &str = "https://activity-pair.invalid";
const TOKEN: &str = "https://activity-token.invalid";
const MOST: u32 = GatewayActivity::MOST;
const OFFICIAL: &[&str] = &[LEGACY_ORIGIN];
const LISTING: GatewayScopes = GatewayScopes {
    sso: false,
    listing: true,
};
const SSO: GatewayScopes = GatewayScopes {
    sso: true,
    listing: false,
};

/// A gateway with a credential pair, approved.
struct Pair {
    gateway: [u8; 16],
    credential: Token,
    heartbeat: Token,
}

const fn counts(players: u32, tables: u32, games: u32) -> GatewayActivity {
    GatewayActivity {
        players,
        tables,
        games,
    }
}

async fn text(operator: &DatabaseConnection, sql: &str, gateway: [u8; 16]) -> String {
    operator
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            vec![Uuid::from_bytes(gateway).into()],
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "value")
        .unwrap()
}

/// The whole row as text: every column, the activity among them.
async fn row(operator: &DatabaseConnection, gateway: [u8; 16]) -> String {
    let sql = "SELECT g::text AS value FROM central_identity.gateways g WHERE id = $1";
    text(operator, sql, gateway).await
}

/// The gateway's tokens as text, with when each was last used.
async fn tokens(operator: &DatabaseConnection, gateway: [u8; 16]) -> String {
    let sql = "SELECT pg_catalog.string_agg(t::text, '|' ORDER BY t.id) AS value
        FROM central_identity.gateway_tokens t WHERE gateway_id = $1";
    text(operator, sql, gateway).await
}

/// What the row holds of an activity: nothing, or all four columns.
async fn kept(operator: &DatabaseConnection, gateway: [u8; 16]) -> String {
    let sql = "SELECT pg_catalog.num_nonnulls(activity_players, activity_tables, activity_games,
        activity_at)::text AS value FROM central_identity.gateways WHERE id = $1";
    text(operator, sql, gateway).await
}

/// Move one of the row's times back, as if `seconds` had passed since.
async fn earlier(operator: &DatabaseConnection, gateway: [u8; 16], column: &str, seconds: u32) {
    let moved = operator
        .execute_unprepared(&format!(
            "UPDATE central_identity.gateways SET {column} = {column} - interval '{seconds} seconds'
             WHERE id = '{}' AND {column} IS NOT NULL",
            Uuid::from_bytes(gateway)
        ))
        .await
        .unwrap();
    assert_eq!(moved.rows_affected(), 1, "{column}");
}

/// The gateway's figure, if the public page has one.
async fn shown(store: &AccountStore, gateway: [u8; 16]) -> Option<GatewayActivity> {
    store
        .gateway_figures(None)
        .await
        .unwrap()
        .into_iter()
        .find(|figures| figures.id == gateway)
        .map(|figures| figures.activity)
}

async fn online(store: &AccountStore, gateway: [u8; 16]) -> bool {
    store
        .gateway_directory(None)
        .await
        .unwrap()
        .into_iter()
        .any(|entry| entry.id == gateway && entry.online)
}

pub(super) async fn run(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    // The fixtures before this one beat with plain heartbeats only: whatever
    // is online in the directory, nothing has a figure.
    assert_eq!(store.gateway_figures(None).await.unwrap(), []);
    let (owner, native, browser) = moved(store, keys, "activity-owner").await;
    let credential = Token::generate().unwrap();
    let heartbeat = Token::generate().unwrap();
    let gateway = store
        .register_gateway(
            &native,
            PAIR,
            &metadata(PAIR),
            &credential,
            &heartbeat,
            false,
        )
        .await
        .unwrap();
    approval(operator, gateway, true).await;
    let pair = Pair {
        gateway,
        credential,
        heartbeat,
    };
    bounds(store, operator, &pair).await;
    reported(store, operator, &pair).await;
    window(store, operator, &pair).await;
    by_token(store, operator, &browser, &pair).await;
    official(store, operator, &pair).await;
    paging(store, operator, owner.account).await;
    accounts(store, operator, keys).await;
}

/// A count above the bound, a credential that is no heartbeat's, metadata
/// elsewhere: each is refused, and the row is as it was. The bound itself
/// is a count like any other.
async fn bounds(store: &AccountStore, operator: &DatabaseConnection, pair: &Pair) {
    let before = row(operator, pair.gateway).await;
    for over in [
        counts(MOST + 1, 0, 0),
        counts(0, MOST + 1, 0),
        counts(0, 0, MOST + 1),
        counts(u32::MAX, u32::MAX, u32::MAX),
    ] {
        assert_eq!(
            store
                .gateway_pulse(&pair.heartbeat, &metadata(PAIR), over)
                .await
                .err(),
            Some(Error::Rejected),
            "{over:?}"
        );
    }
    // The redemption credential signs players in; it reports nothing.
    for wrong in [&pair.credential, &Token::generate().unwrap()] {
        assert_eq!(
            store
                .gateway_pulse(wrong, &metadata(PAIR), counts(1, 1, 1))
                .await
                .err(),
            Some(Error::Rejected)
        );
    }
    assert_eq!(
        store
            .gateway_pulse(
                &pair.heartbeat,
                &metadata("https://elsewhere.invalid"),
                counts(1, 1, 1)
            )
            .await
            .err(),
        Some(Error::Rejected)
    );
    assert_eq!(row(operator, pair.gateway).await, before);
    assert_eq!(kept(operator, pair.gateway).await, "0");
    assert!(shown(store, pair.gateway).await.is_none());
    assert!(!online(store, pair.gateway).await);
    store
        .gateway_pulse(&pair.heartbeat, &metadata(PAIR), counts(MOST, MOST, MOST))
        .await
        .unwrap();
    assert_eq!(
        shown(store, pair.gateway).await,
        Some(counts(MOST, MOST, MOST))
    );
    assert!(online(store, pair.gateway).await);
}

/// The heartbeat's limit holds for a pulse and leaves the counts alone; a
/// later pulse replaces them, zeros included; a plain heartbeat clears
/// them, and the gateway stays online without a figure.
async fn reported(store: &AccountStore, operator: &DatabaseConnection, pair: &Pair) {
    let beat = metadata(PAIR);
    let (gateway, token) = (pair.gateway, &pair.heartbeat);
    assert_eq!(
        store
            .gateway_pulse(token, &beat, counts(1, 1, 1))
            .await
            .err(),
        Some(Error::Busy)
    );
    assert_eq!(shown(store, gateway).await, Some(counts(MOST, MOST, MOST)));
    for next in [counts(12, 3, 4), counts(0, 0, 0)] {
        earlier(operator, gateway, "last_seen", 6).await;
        store.gateway_pulse(token, &beat, next).await.unwrap();
        assert_eq!(shown(store, gateway).await, Some(next));
    }
    // It stopped reporting: no figure, nothing kept, and still online.
    earlier(operator, gateway, "last_seen", 6).await;
    store.gateway_heartbeat(token, &beat).await.unwrap();
    assert!(shown(store, gateway).await.is_none());
    assert_eq!(kept(operator, gateway).await, "0");
    assert!(online(store, gateway).await);
    // A plain heartbeat refused for its rate clears nothing.
    earlier(operator, gateway, "last_seen", 6).await;
    store
        .gateway_pulse(token, &beat, counts(12, 3, 4))
        .await
        .unwrap();
    assert_eq!(
        store.gateway_heartbeat(token, &beat).await.err(),
        Some(Error::Busy)
    );
    assert_eq!(shown(store, gateway).await, Some(counts(12, 3, 4)));
    assert_eq!(kept(operator, gateway).await, "4");
}

/// The 90 seconds: a figure needs the gateway seen within them and the
/// activity reported within them.
async fn window(store: &AccountStore, operator: &DatabaseConnection, pair: &Pair) {
    let beat = metadata(PAIR);
    let (gateway, token) = (pair.gateway, &pair.heartbeat);
    // Seen and reported 85 seconds ago: still shown.
    earlier(operator, gateway, "activity_at", 85).await;
    earlier(operator, gateway, "last_seen", 85).await;
    assert_eq!(shown(store, gateway).await, Some(counts(12, 3, 4)));
    // Seen since, but the report is older than the window.
    earlier(operator, gateway, "activity_at", 6).await;
    assert!(shown(store, gateway).await.is_none());
    assert!(online(store, gateway).await);
    assert_eq!(kept(operator, gateway).await, "4");
    store
        .gateway_pulse(token, &beat, counts(5, 2, 1))
        .await
        .unwrap();
    assert_eq!(shown(store, gateway).await, Some(counts(5, 2, 1)));
    // Silent for longer than the window: offline, whatever it said last.
    earlier(operator, gateway, "last_seen", 91).await;
    assert!(shown(store, gateway).await.is_none());
    assert!(!online(store, gateway).await);
    earlier(operator, gateway, "activity_at", 91).await;
    assert!(shown(store, gateway).await.is_none());
    store
        .gateway_pulse(token, &beat, counts(5, 2, 1))
        .await
        .unwrap();
    assert_eq!(shown(store, gateway).await, Some(counts(5, 2, 1)));
}

/// A gateway registered on the website pulses with a `listing` token. While
/// it waits for approval its pulse is kept and nothing is shown; approved,
/// it is shown; disabled, it is hidden at once and its pulse refused. The
/// same for a credential pair's gateway an administrator disabled.
async fn by_token(
    store: &AccountStore,
    operator: &DatabaseConnection,
    browser: &Token,
    pair: &Pair,
) {
    let gateway = store
        .add_gateway(browser, TOKEN, "Activity by token", false)
        .await
        .unwrap();
    let (listing, sso) = (Token::generate().unwrap(), Token::generate().unwrap());
    for (token, name, scopes) in [(&listing, "List", LISTING), (&sso, "Sign-in", SSO)] {
        store
            .create_gateway_token(browser, gateway, name, scopes, 30, token)
            .await
            .unwrap();
    }
    let beat = GatewayMetadata {
        name: "Activity by token".into(),
        version: "1.0-test".into(),
        website: format!("{TOKEN}/"),
    };
    let (before, unused) = (
        row(operator, gateway).await,
        tokens(operator, gateway).await,
    );
    // A token that only signs players in reports nothing; a count above the
    // bound is refused before the token is looked at, so it is not marked used.
    for (token, activity) in [(&sso, counts(1, 1, 1)), (&listing, counts(MOST + 1, 0, 0))] {
        assert_eq!(
            store.gateway_pulse(token, &beat, activity).await.err(),
            Some(Error::Rejected)
        );
    }
    assert_eq!(row(operator, gateway).await, before);
    assert_eq!(tokens(operator, gateway).await, unused);
    store
        .gateway_pulse(&listing, &beat, counts(7, 1, 1))
        .await
        .unwrap();
    assert_ne!(tokens(operator, gateway).await, unused, "marked used");
    assert_eq!(kept(operator, gateway).await, "4");
    assert!(shown(store, gateway).await.is_none(), "pending");
    approval(operator, gateway, true).await;
    assert_eq!(shown(store, gateway).await, Some(counts(7, 1, 1)));
    let page = store.gateway_figures(None).await.unwrap();
    assert_eq!(page.len(), 2);
    assert!(page[0].id < page[1].id);
    for (disabled, token, beat) in [
        (gateway, &listing, beat),
        (pair.gateway, &pair.heartbeat, metadata(PAIR)),
    ] {
        approval(operator, disabled, false).await;
        assert!(shown(store, disabled).await.is_none(), "disabled");
        // Past the heartbeat's limit, so that is not what refuses it.
        earlier(operator, disabled, "last_seen", 6).await;
        let before = row(operator, disabled).await;
        assert_eq!(
            store
                .gateway_pulse(token, &beat, counts(9, 9, 9))
                .await
                .err(),
            Some(Error::Rejected)
        );
        assert_eq!(row(operator, disabled).await, before);
    }
    // Enabled again within the window, the pair's gateway is shown as it last reported.
    approval(operator, pair.gateway, true).await;
    assert_eq!(shown(store, pair.gateway).await, Some(counts(5, 2, 1)));
    assert_eq!(store.gateway_figures(None).await.unwrap().len(), 1);
}

/// The official gateway reports through the bridge's pulse, for its own row
/// and no other, under the same rules.
async fn official(store: &AccountStore, operator: &DatabaseConnection, pair: &Pair) {
    let beat = GatewayMetadata {
        name: "Synthetic official gateway".into(),
        version: "0.1.0-test".into(),
        website: format!("{LEGACY_ORIGIN}/"),
    };
    let pulse = |activity| store.official_pulse(OFFICIAL, &beat, false, activity);
    // Not listed, then listed and not approved: refused as its heartbeat is.
    assert_eq!(pulse(counts(1, 1, 1)).await.err(), Some(Error::Rejected));
    operator
        .execute_unprepared(
            "INSERT INTO central_identity.gateways (official, origin, name, version, website)
             VALUES (true, 'https://baylee.acevik.de', 'Baylee', '0', 'https://baylee.acevik.de/')",
        )
        .await
        .unwrap();
    let id: Uuid = operator
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT id FROM central_identity.gateways WHERE official",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "id")
        .unwrap();
    let id = *id.as_bytes();
    assert_eq!(pulse(counts(1, 1, 1)).await.err(), Some(Error::Rejected));
    approval(operator, id, true).await;
    let (listed, registered) = (row(operator, id).await, row(operator, pair.gateway).await);
    assert_eq!(
        pulse(counts(0, MOST + 1, 0)).await.err(),
        Some(Error::Rejected)
    );
    // The bridge's pulse is for the bridge's origins alone.
    assert_eq!(
        store
            .official_pulse(&[PAIR], &metadata(PAIR), false, counts(1, 1, 1))
            .await
            .err(),
        Some(Error::Rejected)
    );
    assert_eq!(row(operator, id).await, listed);
    pulse(counts(40, 6, 9)).await.unwrap();
    assert_eq!(shown(store, id).await, Some(counts(40, 6, 9)));
    assert_eq!(pulse(counts(41, 6, 9)).await.err(), Some(Error::Busy));
    assert_eq!(shown(store, id).await, Some(counts(40, 6, 9)));
    // Its plain heartbeat clears them, as a registered gateway's does.
    earlier(operator, id, "last_seen", 6).await;
    store
        .official_heartbeat(OFFICIAL, &beat, false)
        .await
        .unwrap();
    assert!(shown(store, id).await.is_none());
    assert_eq!(kept(operator, id).await, "0");
    assert!(online(store, id).await);
    // The registered gateway is as it was through all of it.
    assert_eq!(row(operator, pair.gateway).await, registered);
    operator
        .execute_unprepared("DELETE FROM central_identity.gateways WHERE official")
        .await
        .unwrap();
}

/// More reporting gateways than a page holds, written as the operator: a
/// hundred a page in ascending order, the rest after the last id, nothing
/// twice; a cursor that is no id of Central's is refused.
async fn paging(store: &AccountStore, operator: &DatabaseConnection, owner: [u8; 16]) {
    let before = store.gateway_figures(None).await.unwrap().len();
    operator
        .execute_unprepared(&format!(
            "WITH made AS (INSERT INTO central_identity.gateways (owner_id, origin, name, version,
                 website, credential_digest, heartbeat_digest, last_seen, activity_players,
                 activity_tables, activity_games, activity_at)
             SELECT '{}'::uuid, 'https://page-' || n || '.invalid', 'Page', '1',
                 'https://page-' || n || '.invalid/',
                 pg_catalog.sha256(pg_catalog.convert_to('page-credential-' || n, 'UTF8')),
                 pg_catalog.sha256(pg_catalog.convert_to('page-heartbeat-' || n, 'UTF8')),
                 pg_catalog.clock_timestamp(), n, 1, 0, pg_catalog.clock_timestamp()
             FROM pg_catalog.generate_series(1, 101) n RETURNING id)
             INSERT INTO central_identity.gateway_approvals (gateway_id) SELECT id FROM made",
            Uuid::from_bytes(owner)
        ))
        .await
        .unwrap();
    let first = store.gateway_figures(None).await.unwrap();
    assert_eq!(first.len(), 100);
    let rest = store.gateway_figures(Some(first[99].id)).await.unwrap();
    assert_eq!(first.len() + rest.len(), before + 101);
    let all: Vec<[u8; 16]> = first
        .iter()
        .chain(&rest)
        .map(|figures| figures.id)
        .collect();
    assert!(all.windows(2).all(|pair| pair[0] < pair[1]));
    let last = *all.last().unwrap();
    assert_eq!(store.gateway_figures(Some(last)).await.unwrap(), []);
    for cursor in [[0; 16], [0xff; 16]] {
        assert_eq!(
            store.gateway_figures(Some(cursor)).await.err(),
            Some(Error::Rejected)
        );
    }
    operator
        .execute_unprepared(
            "DELETE FROM central_identity.gateways WHERE origin LIKE 'https://page-%.invalid'",
        )
        .await
        .unwrap();
    assert_eq!(store.gateway_figures(None).await.unwrap().len(), before);
}

/// The number of accounts: those that can sign in. An unfinished move or
/// registration is not one of them yet, and a deleted account no longer.
async fn accounts(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let enrolled = || async {
        let row = operator
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT count(*)::bigint AS value FROM central_identity.accounts
                 WHERE credential IS NOT NULL",
            ))
            .await
            .unwrap()
            .unwrap();
        u64::try_from(row.try_get::<i64>("", "value").unwrap()).unwrap()
    };
    let before = store.account_count().await.unwrap();
    assert_eq!(before, enrolled().await);
    assert!(before >= 2);
    store
        .reserve_legacy(
            "activity-unfinished",
            "activity-unfinished",
            None,
            None,
            &Token::generate().unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(store.account_count().await.unwrap(), before);
    let counted = enroll(store, keys, "activity-counted", "activity-counted").await;
    assert_eq!(store.account_count().await.unwrap(), before + 1);
    operator
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "DELETE FROM central_identity.accounts WHERE id = $1",
            vec![Uuid::from_bytes(counted.account).into()],
        ))
        .await
        .unwrap();
    assert_eq!(store.account_count().await.unwrap(), before);
    assert_eq!(before, enrolled().await);
}
