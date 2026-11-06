use super::Http;
use central_api::{
    Error, Token,
    gateway::GatewayMetadata,
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
    runtime
        .block_on(
            operator.execute_unprepared("DELETE FROM central_identity.gateways WHERE official"),
        )
        .unwrap();
}
