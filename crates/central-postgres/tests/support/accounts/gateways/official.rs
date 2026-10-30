//! The official gateway's row (migration 000009): listed and approved by the
//! operator, without owner or credential, so the bridge stays its only
//! authority; its heartbeat, which the bridge sends, moves that row alone.
//! It answers at several origins once it moved (`OFFICIAL_ALIASES`).
use super::super::terms::{texts, version};
use super::{AccountStore, DatabaseConnection, Error, TicketPurpose, Token, approval, metadata};
use central_api::{LEGACY_ORIGIN, OFFICIAL_ORIGIN, gateway::GatewayMetadata};
use sea_orm::{ConnectionTrait, DbBackend, Statement, prelude::Uuid};

/// The operator's statements of `docs/operations.md` §"The official gateway", as the
/// row stood before the move: the old address, which the fixture then moves.
const LIST: &str =
    "INSERT INTO central_identity.gateways (official, origin, name, version, website)
    VALUES (true, 'https://baylee.acevik.de', 'Baylee', '0', 'https://baylee.acevik.de/')";
const APPROVE: &str = "INSERT INTO central_identity.gateway_approvals (gateway_id)
    SELECT id FROM central_identity.gateways WHERE official";
const DEVELOPMENT: &str = "http://127.0.0.1:28799";

fn beat(origin: &str) -> GatewayMetadata {
    GatewayMetadata {
        name: "Synthetic official gateway".into(),
        version: "0.1.0-test".into(),
        website: format!("{origin}/"),
    }
}

/// A row as the directory would show it, and when it was last seen.
async fn row(operator: &DatabaseConnection, gateway: [u8; 16]) -> Vec<String> {
    let row = operator
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT origin, name, version, website, COALESCE(last_seen::text, '') AS seen
             FROM central_identity.gateways WHERE id = $1",
            vec![Uuid::from_bytes(gateway).into()],
        ))
        .await
        .unwrap()
        .unwrap();
    ["origin", "name", "version", "website", "seen"]
        .iter()
        .map(|column| row.try_get("", column).unwrap())
        .collect()
}

async fn listed(store: &AccountStore, gateway: [u8; 16]) -> Option<(String, String, bool)> {
    store
        .gateway_directory(None)
        .await
        .unwrap()
        .into_iter()
        .find(|entry| entry.id == gateway)
        .map(|entry| (entry.origin, entry.metadata.name, entry.online))
}

async fn heartbeat(store: &AccountStore, origin: &str, development: bool) -> Result<(), Error> {
    store
        .official_heartbeat(&[origin], &beat(origin), development)
        .await
}

pub(super) async fn run(
    store: &AccountStore,
    operator: &DatabaseConnection,
    session: &Token,
    registered: [u8; 16],
) {
    // The bridge's origins are never registered, in either mode, and say why:
    // a conflict, not the owner's full quota.
    for origin in [OFFICIAL_ORIGIN, LEGACY_ORIGIN] {
        for development in [false, true] {
            let result = store
                .register_gateway(
                    session,
                    origin,
                    &metadata(origin),
                    &Token::generate().unwrap(),
                    &Token::generate().unwrap(),
                    development,
                )
                .await;
            assert!(matches!(result, Err(Error::Conflict)), "{origin}");
            let added = store
                .add_gateway(session, origin, "Not it", development)
                .await;
            assert!(matches!(added, Err(Error::Conflict)), "{origin}");
        }
    }
    let official = list(store, operator).await;
    let untouched = row(operator, registered).await;
    beat_for_it(store, operator, session, official).await;
    // The official row only: the registered gateway is as it was.
    assert_eq!(row(operator, registered).await, untouched);
    moved(store, operator, session, official).await;
    assert_eq!(row(operator, registered).await, untouched);
    elsewhere(store, operator, official).await;
    assert_eq!(row(operator, registered).await, untouched);
    // Gone again, for the fixtures after this one.
    operator
        .execute_unprepared("DELETE FROM central_identity.gateways WHERE official")
        .await
        .unwrap();
}

/// The operator lists it: one official row at most, with no owner or
/// credential, shown only once approved.
async fn list(store: &AccountStore, operator: &DatabaseConnection) -> [u8; 16] {
    // Before the operator lists it there is no row to beat for.
    assert!(matches!(
        heartbeat(store, LEGACY_ORIGIN, false).await,
        Err(Error::Rejected)
    ));
    operator.execute_unprepared(LIST).await.unwrap();
    let id = operator
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT id FROM central_identity.gateways WHERE official",
        ))
        .await
        .unwrap()
        .unwrap();
    let official = *id.try_get::<Uuid>("", "id").unwrap().as_bytes();
    for refused in [
        "INSERT INTO central_identity.gateways (official, origin, name, version, website)
         VALUES (true, 'https://second.invalid', 'Second', '0', 'https://second.invalid/')",
        "UPDATE central_identity.gateways SET owner_id = (SELECT id FROM central_identity.accounts LIMIT 1)
         WHERE official",
        "UPDATE central_identity.gateways SET credential_digest = decode(repeat('21', 32), 'hex') WHERE official",
        "UPDATE central_identity.gateways SET heartbeat_digest = decode(repeat('22', 32), 'hex') WHERE official",
        "UPDATE central_identity.gateways SET official = true WHERE NOT official",
    ] {
        assert!(operator.execute_unprepared(refused).await.is_err(), "{refused}");
    }
    // Listed only once approved; until then the bridge's heartbeat is refused too.
    assert!(listed(store, official).await.is_none());
    assert!(matches!(
        heartbeat(store, LEGACY_ORIGIN, false).await,
        Err(Error::Rejected)
    ));
    operator.execute_unprepared(APPROVE).await.unwrap();
    assert_eq!(
        listed(store, official).await,
        Some((LEGACY_ORIGIN.into(), "Baylee".into(), false))
    );
    official
}

/// The bridge's heartbeat moves it online with its metadata, once per five
/// seconds; nothing registered reaches it.
async fn beat_for_it(
    store: &AccountStore,
    operator: &DatabaseConnection,
    session: &Token,
    official: [u8; 16],
) {
    // Metadata on another origin, or another origin in a live store, is refused.
    assert!(
        store
            .official_heartbeat(
                &[LEGACY_ORIGIN],
                &beat("https://registry-one.invalid"),
                false
            )
            .await
            .is_err()
    );
    assert!(
        heartbeat(store, "https://registry-one.invalid", false)
            .await
            .is_err()
    );
    heartbeat(store, LEGACY_ORIGIN, false).await.unwrap();
    assert!(matches!(
        heartbeat(store, LEGACY_ORIGIN, false).await,
        Err(Error::Busy)
    ));
    assert_eq!(
        listed(store, official).await,
        Some((
            LEGACY_ORIGIN.into(),
            "Synthetic official gateway".into(),
            true
        ))
    );
    assert_eq!(row(operator, official).await[2], "0.1.0-test");
    // No registered credential reaches it: a heartbeat token finds no row, and
    // no registered ticket is issued for its audience.
    assert!(
        store
            .gateway_heartbeat(&Token::generate().unwrap(), &beat(LEGACY_ORIGIN))
            .await
            .is_err()
    );
    for purpose in [TicketPurpose::Admit, TicketPurpose::Admin] {
        assert!(
            store
                .registered_ticket(session, LEGACY_ORIGIN, &Token::generate().unwrap(), purpose)
                .await
                .is_err()
        );
    }
}

/// The official gateway moved (`OFFICIAL_ORIGIN`, its old address an alias):
/// its heartbeat keeps the row under either origin and a website on either;
/// a ticket an installed client asks for the old address is redeemed and its
/// lease checked under the whole set, and keeps the address it was for; a
/// player who accepted the gateway's terms there has accepted them.
async fn moved(
    store: &AccountStore,
    operator: &DatabaseConnection,
    session: &Token,
    official: [u8; 16],
) {
    let set = [OFFICIAL_ORIGIN, LEGACY_ORIGIN];
    let back = "UPDATE central_identity.gateways SET last_seen = last_seen - interval '10 seconds' WHERE official";
    operator.execute_unprepared(back).await.unwrap();
    // a live set holds the compiled origins only
    assert!(matches!(
        store
            .official_heartbeat(
                &[OFFICIAL_ORIGIN, "https://registry-one.invalid"],
                &beat(OFFICIAL_ORIGIN),
                false
            )
            .await,
        Err(Error::Rejected)
    ));
    // the row still at the old origin, the website there too
    store
        .official_heartbeat(&set, &beat(LEGACY_ORIGIN), false)
        .await
        .unwrap();
    // the operator moves the row; the gateway still names its old website
    operator
        .execute_unprepared(&format!(
            "UPDATE central_identity.gateways SET origin = '{OFFICIAL_ORIGIN}', website = '{OFFICIAL_ORIGIN}/',
             last_seen = last_seen - interval '10 seconds' WHERE official"
        ))
        .await
        .unwrap();
    store
        .official_heartbeat(&set, &beat(LEGACY_ORIGIN), false)
        .await
        .unwrap();
    assert_eq!(
        listed(store, official).await,
        Some((
            OFFICIAL_ORIGIN.into(),
            "Synthetic official gateway".into(),
            true
        ))
    );
    // its terms, accepted under the old address before the move
    let words = texts("Official");
    let terms = version("official-1", &words);
    store
        .publish_gateway_terms(LEGACY_ORIGIN, &terms, &words)
        .await
        .unwrap();
    store
        .accept_gateway_terms(session, LEGACY_ORIGIN, &terms)
        .await
        .unwrap();
    for asked in set {
        let ticket = Token::generate().unwrap();
        let lease = Token::generate().unwrap();
        store
            .gateway_ticket(session, asked, &ticket, TicketPurpose::Admit)
            .await
            .unwrap();
        store
            .redeem_lease(&ticket, &set, TicketPurpose::Admit, None, Some(&lease))
            .await
            .unwrap();
        // the lease is for the address asked, under the set
        let other = if asked == OFFICIAL_ORIGIN {
            LEGACY_ORIGIN
        } else {
            OFFICIAL_ORIGIN
        };
        assert!(
            store
                .gateway_session(&lease, &set, None, TicketPurpose::Admit)
                .await
                .is_ok()
        );
        assert!(
            store
                .gateway_session(&lease, &[asked], None, TicketPurpose::Admit)
                .await
                .is_ok()
        );
        assert!(
            store
                .gateway_session(&lease, &[other], None, TicketPurpose::Admit)
                .await
                .is_err(),
            "{asked}"
        );
        let (_, _, _, accepted) = store
            .gateway_authorize(&lease, &set, None, TicketPurpose::Admit, None, Some(&terms))
            .await
            .unwrap();
        assert!(accepted, "accepted under the old address, for {asked}");
    }
    // a lease for the new address alone does not see the old acceptance
    let ticket = Token::generate().unwrap();
    let lease = Token::generate().unwrap();
    store
        .gateway_ticket(session, OFFICIAL_ORIGIN, &ticket, TicketPurpose::Admit)
        .await
        .unwrap();
    store
        .redeem_lease(
            &ticket,
            &[OFFICIAL_ORIGIN],
            TicketPurpose::Admit,
            None,
            Some(&lease),
        )
        .await
        .unwrap();
    let (_, _, _, accepted) = store
        .gateway_authorize(
            &lease,
            &[OFFICIAL_ORIGIN],
            None,
            TicketPurpose::Admit,
            None,
            Some(&terms),
        )
        .await
        .unwrap();
    assert!(!accepted);
    operator.execute_unprepared(back).await.unwrap();
}

/// It is the official gateway only while its origin is the bridge's: moved to
/// a loopback origin, a live store has nothing to beat for, and a development
/// store's bridge (whose origin that is) beats for it. Disabled, it is hidden
/// and its heartbeat refused.
async fn elsewhere(store: &AccountStore, operator: &DatabaseConnection, official: [u8; 16]) {
    operator
        .execute_unprepared(&format!(
            "UPDATE central_identity.gateways SET origin = '{DEVELOPMENT}', website = '{DEVELOPMENT}/',
             last_seen = last_seen - interval '10 seconds' WHERE official"
        ))
        .await
        .unwrap();
    for (origin, development) in [
        (LEGACY_ORIGIN, false),
        (DEVELOPMENT, false),
        (LEGACY_ORIGIN, true),
    ] {
        assert!(
            heartbeat(store, origin, development).await.is_err(),
            "{origin} {development}"
        );
    }
    heartbeat(store, DEVELOPMENT, true).await.unwrap();
    approval(operator, official, false).await;
    assert!(listed(store, official).await.is_none());
    operator
        .execute_unprepared(
            "UPDATE central_identity.gateways SET last_seen = last_seen - interval '10 seconds' WHERE official",
        )
        .await
        .unwrap();
    assert!(matches!(
        heartbeat(store, DEVELOPMENT, true).await,
        Err(Error::Rejected)
    ));
}
