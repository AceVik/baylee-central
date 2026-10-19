use super::Http;
use central_api::{
    gateway::GatewayMetadata,
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
