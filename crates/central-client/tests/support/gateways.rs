use super::Http;
use central_api::{
    Error, Token,
    gateway::{GatewayActivity, GatewayFigures, GatewayMetadata},
    wire::{GatewayPurpose, Request, Response},
};
use central_client::{Session, Transport};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, prelude::Uuid};
use tokio::runtime::Runtime;

pub(crate) fn run(
    http: &Http,
    session: &Session,
    runtime: &Runtime,
    operator: &DatabaseConnection,
) {
    let origin = "http://127.0.0.1:28767";
    let metadata = GatewayMetadata {
        name: "Synthetic gateway".into(),
        version: "test-1".into(),
        website: format!("{origin}/status"),
    };
    let registered = session
        .register_gateway(http, origin.into(), metadata.clone())
        .unwrap();
    assert!(
        central_client::gateway_directory(http, None)
            .unwrap()
            .iter()
            .all(|entry| entry.id != registered.gateway)
    );
    runtime
        .block_on(operator.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO central_identity.gateway_approvals(gateway_id) VALUES ($1)",
            vec![Uuid::from_bytes(registered.gateway).into()],
        )))
        .unwrap();
    let page = central_client::gateway_directory(http, None).unwrap();
    assert_eq!(page.len(), 1);
    assert_eq!(page[0].id, registered.gateway);
    super::challenges::refuse_gateway(http, &registered.credential, &registered.heartbeat);
    assert!(!page[0].online);
    assert!(
        http.exchange(
            &Request::GatewayHeartbeat {
                metadata: metadata.clone()
            },
            Some((&registered.credential).into())
        )
        .is_err()
    );
    http.exchange(
        &Request::GatewayHeartbeat { metadata },
        Some((&registered.heartbeat).into()),
    )
    .unwrap();
    assert!(central_client::gateway_directory(http, None).unwrap()[0].online);
    activity(http, &registered, origin, runtime, operator);
    assert_eq!(
        session
            .set_gateway_admins(
                http,
                registered.gateway,
                1,
                vec![session.identity().account]
            )
            .unwrap(),
        2
    );
    let (token, _) = session
        .gateway_token(http, origin.into(), GatewayPurpose::Admin)
        .unwrap();
    let claims = http.gateway_claims(&token, origin);
    assert!(
        claims.legacy_id.is_none(),
        "a registered gateway cannot learn the legacy identifier"
    );
    assert!(claims.purpose == central_crypto::jwt::Purpose::Admin);
    super::reports::run(
        http,
        &registered.credential,
        session.identity().account,
        runtime,
        operator,
    );
}

const COUNTED: GatewayActivity = GatewayActivity {
    players: 12,
    playing: 8,
    tables: 3,
    games: 4,
};

/// As if the gateway's last heartbeat were `seconds` old.
fn earlier(runtime: &Runtime, operator: &DatabaseConnection, gateway: [u8; 16], seconds: u32) {
    runtime
        .block_on(operator.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            format!(
                "UPDATE central_identity.gateways SET last_seen = last_seen - interval '{seconds} seconds'
                 WHERE id = $1"
            ),
            vec![Uuid::from_bytes(gateway).into()],
        )))
        .unwrap();
}

/// A registered gateway's pulse over HTTP and the public figures it makes:
/// online and silent it has no figure; the pulse takes the heartbeat's
/// credential and limit, counts within the bound and no more players in a
/// game than connected; then anyone reads its counts, and a plain heartbeat
/// withdraws them.
fn activity(
    http: &Http,
    registered: &central_client::GatewayRegistration,
    origin: &str,
    runtime: &Runtime,
    operator: &DatabaseConnection,
) {
    let metadata = || GatewayMetadata {
        name: "Synthetic gateway".into(),
        version: "test-2".into(),
        website: format!("{origin}/status"),
    };
    let pulse = |credential: &Token, activity| {
        central_client::gateway_pulse(http, credential, origin, metadata(), activity)
    };
    let figures = || central_client::public_figures(http, None).unwrap();
    let silent = figures();
    assert!(silent.gateways.is_empty(), "online, and it reports nothing");
    assert!(silent.accounts >= 1);
    // Five seconds after its heartbeat, like a heartbeat.
    assert_eq!(pulse(&registered.heartbeat, COUNTED), Err(Error::Busy));
    earlier(runtime, operator, registered.gateway, 6);
    // The heartbeat's credential and no other; nothing above the bound and
    // no more in a game than connected, which the SDK does not send and
    // Central does not take.
    let over = GatewayActivity {
        players: GatewayActivity::MOST + 1,
        ..COUNTED
    };
    let more = GatewayActivity {
        playing: COUNTED.players + 1,
        ..COUNTED
    };
    for (credential, activity) in [
        (&registered.credential, COUNTED),
        (&Token::generate().unwrap(), COUNTED),
        (&registered.heartbeat, over),
        (&registered.heartbeat, more),
    ] {
        assert_eq!(pulse(credential, activity), Err(Error::Rejected));
    }
    for (bearer, activity) in [
        (Some(&registered.heartbeat), over),
        (Some(&registered.heartbeat), more),
        (None, COUNTED),
    ] {
        let request = Request::GatewayPulse {
            metadata: metadata(),
            activity,
        };
        assert_eq!(
            http.exchange(&request, bearer.map(Into::into)).err(),
            Some(Error::Rejected)
        );
    }
    assert_eq!(figures().gateways, []);
    former_shape(http, registered, origin);
    earlier(runtime, operator, registered.gateway, 6);
    pulse(&registered.heartbeat, COUNTED).unwrap();
    let reported = figures();
    assert_eq!(reported.accounts, silent.accounts);
    assert_eq!(
        reported.gateways,
        vec![GatewayFigures {
            id: registered.gateway,
            activity: COUNTED,
        }]
    );
    assert_eq!(
        central_client::public_figures(http, Some(registered.gateway))
            .unwrap()
            .gateways,
        []
    );
    assert!(central_client::public_figures(http, Some([0; 16])).is_err());
    let listed = central_client::gateway_directory(http, None)
        .unwrap()
        .remove(0);
    assert!(listed.online);
    assert_eq!(listed.metadata.version, "test-2", "a pulse is a heartbeat");
    // It stops reporting: the next plain heartbeat withdraws the counts.
    earlier(runtime, operator, registered.gateway, 6);
    http.exchange(
        &Request::GatewayHeartbeat {
            metadata: metadata(),
        },
        Some((&registered.heartbeat).into()),
    )
    .unwrap();
    assert_eq!(figures().gateways, []);
    assert!(central_client::gateway_directory(http, None).unwrap()[0].online);
}

/// The pulse as a gateway built before the fourth count would send it, with
/// the heartbeat's own credential and past its limit: refused whole, and
/// nothing is shown. The same bytes with the fourth count are taken, so it
/// is the shape that was refused; everybody in a game is a count like any
/// other.
fn former_shape(http: &Http, registered: &central_client::GatewayRegistration, origin: &str) {
    let pulse = |activity: &str| {
        let body = format!(
            r#"{{"operation":"gateway_pulse","metadata":{{"name":"Synthetic gateway","version":"test-2","website":"{origin}/status"}},"activity":{activity}}}"#
        );
        http.written(&body, Some((&registered.heartbeat).into()))
    };
    assert_eq!(
        pulse(r#"{"players":12,"tables":3,"games":4}"#).err(),
        Some(Error::Rejected)
    );
    assert_eq!(
        central_client::public_figures(http, None).unwrap().gateways,
        []
    );
    assert!(matches!(
        pulse(r#"{"players":12,"playing":12,"tables":3,"games":4}"#),
        Ok(Response::GatewayHeartbeat { .. })
    ));
    let all = GatewayActivity {
        playing: COUNTED.players,
        ..COUNTED
    };
    assert_eq!(
        central_client::public_figures(http, None).unwrap().gateways[0].activity,
        all
    );
}

/// The official gateway over HTTP: its origin is the bridge's, so nobody
/// registers it; the operator lists it, and the bridge's heartbeat moves it
/// online with the metadata it sends.
pub(crate) fn official(
    http: &Http,
    session: &Session,
    bridge: &Token,
    origin: &str,
    runtime: &Runtime,
    operator: &DatabaseConnection,
) {
    let metadata = GatewayMetadata {
        name: "Synthetic official gateway".into(),
        version: "test-1".into(),
        website: format!("{origin}/"),
    };
    assert_eq!(
        session
            .register_gateway(http, origin.into(), metadata.clone())
            .err(),
        Some(Error::Conflict)
    );
    let heartbeat = Request::GatewayHeartbeat { metadata };
    assert_eq!(
        http.exchange(&heartbeat, Some(bridge.into())).err(),
        Some(Error::Rejected),
        "nothing is listed for the bridge to beat for"
    );
    runtime
        .block_on(operator.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO central_identity.gateways (official, origin, name, version, website)
             VALUES (true, $1, 'Baylee', '0', $2)",
            vec![origin.into(), format!("{origin}/").into()],
        )))
        .unwrap();
    runtime
        .block_on(operator.execute_unprepared(
            "INSERT INTO central_identity.gateway_approvals (gateway_id)
             SELECT id FROM central_identity.gateways WHERE official",
        ))
        .unwrap();
    let entry = |http: &Http| {
        central_client::gateway_directory(http, None)
            .unwrap()
            .into_iter()
            .find(|entry| entry.origin == origin)
            .unwrap()
    };
    assert!(!entry(http).online);
    assert!(
        http.exchange(&heartbeat, Some((&Token::generate().unwrap()).into()))
            .is_err()
    );
    assert!(matches!(
        http.exchange(&heartbeat, Some(bridge.into())).unwrap(),
        Response::GatewayHeartbeat { .. }
    ));
    let listed = entry(http);
    assert!(listed.online);
    assert_eq!(listed.metadata.name, "Synthetic official gateway");
    assert_eq!(
        http.exchange(&heartbeat, Some(bridge.into())).err(),
        Some(Error::Busy)
    );
    // The bridge's pulse reports for the official gateway, and no other
    // credential's does.
    let beat = || GatewayMetadata {
        name: "Synthetic official gateway".into(),
        version: "test-1".into(),
        website: format!("{origin}/"),
    };
    assert_eq!(
        central_client::gateway_pulse(http, bridge, origin, beat(), COUNTED),
        Err(Error::Busy)
    );
    earlier(runtime, operator, listed.id, 6);
    assert_eq!(
        central_client::gateway_pulse(http, &Token::generate().unwrap(), origin, beat(), COUNTED),
        Err(Error::Rejected)
    );
    central_client::gateway_pulse(http, bridge, origin, beat(), COUNTED).unwrap();
    assert!(
        central_client::public_figures(http, None)
            .unwrap()
            .gateways
            .contains(&GatewayFigures {
                id: listed.id,
                activity: COUNTED,
            })
    );
    runtime
        .block_on(
            operator.execute_unprepared("DELETE FROM central_identity.gateways WHERE official"),
        )
        .unwrap();
}
