use super::Http;
use central_api::{
    Error, Token,
    gateway::{GatewayActivity, GatewayFigures, GatewayMetadata},
    wire::{Request, Response, TicketPurpose},
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
    assert!(!page[0].online);
    assert!(
        http.exchange(
            &Request::GatewayHeartbeat {
                metadata: metadata.clone()
            },
            Some(&registered.credential)
        )
        .is_err()
    );
    http.exchange(
        &Request::GatewayHeartbeat { metadata },
        Some(&registered.heartbeat),
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
    let ticket = session
        .gateway_capability(http, origin.into(), TicketPurpose::Admin)
        .unwrap();
    let Response::GatewayIdentity {
        legacy_id,
        lease: Some(lease),
        ..
    } = http
        .exchange(
            &Request::RedeemGateway {
                ticket,
                purpose: TicketPurpose::Admin,
            },
            Some(&registered.credential),
        )
        .unwrap()
    else {
        panic!("admin lease expected");
    };
    assert!(
        legacy_id.is_none(),
        "a registered gateway cannot learn the legacy identifier"
    );
    assert!(matches!(
        http.exchange(
            &Request::GatewaySession {
                lease,
                purpose: TicketPurpose::Admin
            },
            Some(&registered.credential)
        )
        .unwrap(),
        Response::GatewaySession { .. }
    ));
}

const COUNTED: GatewayActivity = GatewayActivity {
    players: 12,
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
/// credential and limit and counts within the bound; then anyone reads its
/// counts, and a plain heartbeat withdraws them.
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
    // The heartbeat's credential and no other; nothing above the bound,
    // which the SDK does not send and Central does not take.
    let over = GatewayActivity {
        players: GatewayActivity::MOST + 1,
        ..COUNTED
    };
    for (credential, activity) in [
        (&registered.credential, COUNTED),
        (&Token::generate().unwrap(), COUNTED),
        (&registered.heartbeat, over),
    ] {
        assert_eq!(pulse(credential, activity), Err(Error::Rejected));
    }
    for bearer in [Some(&registered.heartbeat), None] {
        let request = Request::GatewayPulse {
            metadata: metadata(),
            activity: if bearer.is_some() { over } else { COUNTED },
        };
        assert_eq!(http.exchange(&request, bearer).err(), Some(Error::Rejected));
    }
    assert_eq!(figures().gateways, []);
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
        Some(&registered.heartbeat),
    )
    .unwrap();
    assert_eq!(figures().gateways, []);
    assert!(central_client::gateway_directory(http, None).unwrap()[0].online);
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
        http.exchange(&heartbeat, Some(bridge)).err(),
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
        http.exchange(&heartbeat, Some(&Token::generate().unwrap()))
            .is_err()
    );
    assert!(matches!(
        http.exchange(&heartbeat, Some(bridge)).unwrap(),
        Response::Done
    ));
    let listed = entry(http);
    assert!(listed.online);
    assert_eq!(listed.metadata.name, "Synthetic official gateway");
    assert_eq!(
        http.exchange(&heartbeat, Some(bridge)).err(),
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
