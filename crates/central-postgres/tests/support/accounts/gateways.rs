use super::{
    AccountStore, DatabaseConnection, Error, ServerKeys, TicketPurpose, Token, config, enroll,
    login,
};
use central_api::gateway::GatewayMetadata;
use sea_orm::{ConnectionTrait, DbBackend, Statement, prelude::Uuid};

mod limits;
mod official;

fn metadata(origin: &str) -> GatewayMetadata {
    GatewayMetadata {
        name: "Synthetic gateway".into(),
        version: "1.0-test".into(),
        website: format!("{origin}/status"),
    }
}

async fn approval(operator: &DatabaseConnection, gateway: [u8; 16], enabled: bool) {
    operator
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO central_identity.gateway_approvals(gateway_id,enabled) VALUES($1,$2)
         ON CONFLICT(gateway_id) DO UPDATE SET enabled=EXCLUDED.enabled",
            vec![Uuid::from_bytes(gateway).into(), enabled.into()],
        ))
        .await
        .unwrap();
}

pub(super) async fn run(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    operator.execute_unprepared(
        "GRANT SELECT,INSERT,UPDATE ON central_identity.gateways TO central_runtime;
         GRANT SELECT ON central_identity.gateway_approvals TO central_runtime;
         GRANT SELECT,INSERT,DELETE ON central_identity.gateway_admins,central_identity.gateway_leases TO central_runtime"
    ).await.unwrap();
    privilege_gate(operator).await;
    let owner = enroll(store, keys, "registry-owner", "registry-owner").await;
    let member = enroll(store, keys, "registry-member", "registry-member").await;
    let owner_session = login(store, keys, owner).await;
    let member_session = login(store, keys, member).await;
    store.confirm_restore(&owner_session, 1).await.unwrap();
    store.confirm_restore(&member_session, 1).await.unwrap();
    let origin = "https://registry-one.invalid";
    let foreign_origin = "https://registry-two.invalid";
    let gateway = registration(store, operator, &owner_session, &member_session, origin).await;
    let foreign_credential = Token::generate().unwrap();
    let foreign = store
        .register_gateway(
            &member_session,
            foreign_origin,
            &metadata(foreign_origin),
            &foreign_credential,
            &Token::generate().unwrap(),
            false,
        )
        .await
        .unwrap();
    approval(operator, foreign, true).await;
    assert!(!store.gateway_admin(gateway, member.account).await.unwrap());
    assert!(
        store
            .set_gateway_admins(&member_session, gateway, 1, &[member.account])
            .await
            .is_err()
    );
    assert!(
        store
            .set_gateway_admins(
                &owner_session,
                gateway,
                1,
                &[member.account, member.account]
            )
            .await
            .is_err()
    );
    assert_eq!(
        store
            .set_gateway_admins(&owner_session, gateway, 1, &[member.account])
            .await
            .unwrap(),
        2
    );
    assert!(
        store
            .set_gateway_admins(&owner_session, gateway, 1, &[])
            .await
            .is_err()
    );
    assert!(!store.gateway_admin(gateway, owner.account).await.unwrap());
    assert!(store.gateway_admin(gateway, member.account).await.unwrap());
    assert!(!store.gateway_admin(foreign, owner.account).await.unwrap());
    delegation(
        store,
        &member_session,
        operator,
        gateway,
        origin,
        foreign,
        foreign_origin,
    )
    .await;

    admin_revocation(
        store,
        &owner_session,
        &member_session,
        gateway,
        origin,
        owner.account,
        member.account,
    )
    .await;
    limits::run(store, operator, &owner_session, gateway, origin).await;
    official::run(store, operator, &owner_session, gateway).await;
}

async fn delegation(
    store: &AccountStore,
    session: &Token,
    operator: &DatabaseConnection,
    gateway: [u8; 16],
    origin: &str,
    foreign: [u8; 16],
    foreign_origin: &str,
) {
    let ticket = Token::generate().unwrap();
    let lease = Token::generate().unwrap();
    store
        .registered_ticket(session, origin, &ticket, TicketPurpose::Admit)
        .await
        .unwrap();
    for purpose in [
        TicketPurpose::Export,
        TicketPurpose::Link,
        TicketPurpose::Admin,
    ] {
        assert!(
            store
                .redeem_lease(&ticket, origin, purpose, Some(gateway), Some(&lease))
                .await
                .is_err()
        );
    }
    assert!(
        store
            .redeem_lease(
                &ticket,
                foreign_origin,
                TicketPurpose::Admit,
                Some(foreign),
                Some(&lease)
            )
            .await
            .is_err()
    );
    let (_, legacy, expires) = store
        .redeem_lease(
            &ticket,
            origin,
            TicketPurpose::Admit,
            Some(gateway),
            Some(&lease),
        )
        .await
        .unwrap();
    assert!(legacy.is_none());
    assert!(expires.unwrap() > 0);
    assert!(
        store
            .redeem_lease(
                &ticket,
                origin,
                TicketPurpose::Admit,
                Some(gateway),
                Some(&Token::generate().unwrap())
            )
            .await
            .is_err()
    );
    profile(
        store,
        operator,
        session,
        &lease,
        (origin, gateway),
        (foreign_origin, foreign),
    )
    .await;
    assert!(
        store
            .gateway_session(&lease, foreign_origin, Some(foreign), TicketPurpose::Admit)
            .await
            .is_err()
    );
    assert!(
        store
            .gateway_session(&lease, origin, Some(gateway), TicketPurpose::Admin)
            .await
            .is_err()
    );
    assert!(
        store
            .gateway_session(&lease, origin, None, TicketPurpose::Admit)
            .await
            .is_err()
    );
    approval(operator, gateway, false).await;
    assert!(
        store
            .gateway_session(&lease, origin, Some(gateway), TicketPurpose::Admit)
            .await
            .is_err()
    );
    assert!(
        store
            .gateway_directory(None)
            .await
            .unwrap()
            .iter()
            .all(|entry| entry.id != gateway)
    );
    approval(operator, gateway, true).await;
}

/// `GatewayProfile` answers only for a live lease the asking gateway holds,
/// with the purpose it was given, while the gateway is approved: a moved
/// account reserved before display names has neither name nor tag until the
/// bridge carries them. An expired lease names nobody.
async fn profile(
    store: &AccountStore,
    operator: &DatabaseConnection,
    session: &Token,
    lease: &Token,
    (origin, gateway): (&str, [u8; 16]),
    (foreign_origin, foreign): (&str, [u8; 16]),
) {
    let read = async |lease: &Token| {
        store
            .gateway_profile(lease, origin, Some(gateway), TicketPurpose::Admit)
            .await
    };
    let (admitted, _) = store
        .gateway_session(lease, origin, Some(gateway), TicketPurpose::Admit)
        .await
        .unwrap();
    let before = read(lease).await.unwrap();
    assert_eq!(before.identity.account, admitted.account);
    assert_eq!((before.display_name, before.tag), (None, None));
    store
        .legacy_profile("registry-member", Some("Registry_Member"), 0x331)
        .await
        .unwrap();
    let after = read(lease).await.unwrap();
    assert_eq!(
        (after.display_name.as_deref(), after.tag),
        (Some("Registry_Member"), Some(0x331))
    );
    for (audience, asking, purpose) in [
        (foreign_origin, Some(foreign), TicketPurpose::Admit),
        (origin, Some(foreign), TicketPurpose::Admit),
        (origin, Some(gateway), TicketPurpose::Admin),
        (origin, Some(gateway), TicketPurpose::Export),
        (origin, None, TicketPurpose::Admit),
        (origin, None, TicketPurpose::Admin),
    ] {
        assert_eq!(
            store
                .gateway_profile(lease, audience, asking, purpose)
                .await
                .err(),
            Some(Error::Rejected),
            "{audience} {asking:?} {}",
            purpose.as_str()
        );
    }
    assert_eq!(
        read(&Token::generate().unwrap()).await.err(),
        Some(Error::Rejected)
    );
    approval(operator, gateway, false).await;
    assert_eq!(read(lease).await.err(), Some(Error::Rejected), "disabled");
    approval(operator, gateway, true).await;
    // A second lease of the same session, expired.
    let (ticket, expired) = (Token::generate().unwrap(), Token::generate().unwrap());
    store
        .registered_ticket(session, origin, &ticket, TicketPurpose::Admit)
        .await
        .unwrap();
    store
        .redeem_lease(
            &ticket,
            origin,
            TicketPurpose::Admit,
            Some(gateway),
            Some(&expired),
        )
        .await
        .unwrap();
    assert!(read(&expired).await.is_ok());
    operator
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE central_identity.gateway_leases SET created_at = clock_timestamp() - interval '2 minutes',
             expires_at = clock_timestamp() - interval '1 minute' WHERE digest = $1",
            vec![expired.digest().to_vec().into()],
        ))
        .await
        .unwrap();
    assert_eq!(read(&expired).await.err(), Some(Error::Rejected), "expired");
    assert!(read(lease).await.is_ok(), "the first lease still names it");
}

async fn privilege_gate(operator: &DatabaseConnection) {
    for privilege in [
        "INSERT",
        "UPDATE(enabled)",
        "DELETE",
        "SELECT WITH GRANT OPTION",
    ] {
        let grant = if privilege == "SELECT WITH GRANT OPTION" {
            "GRANT SELECT ON central_identity.gateway_approvals TO central_runtime WITH GRANT OPTION".into()
        } else {
            format!("GRANT {privilege} ON central_identity.gateway_approvals TO central_runtime")
        };
        operator.execute_unprepared(&grant).await.unwrap();
        let connection =
            AccountStore::connect_dev(config("central_runtime", "CENTRAL_TEST_RUNTIME_PASSWORD"))
                .await;
        let revoke = if privilege == "SELECT WITH GRANT OPTION" {
            "REVOKE GRANT OPTION FOR SELECT ON central_identity.gateway_approvals FROM central_runtime".into()
        } else {
            format!("REVOKE {privilege} ON central_identity.gateway_approvals FROM central_runtime")
        };
        operator.execute_unprepared(&revoke).await.unwrap();
        assert!(
            connection.is_err(),
            "unsafe approval privilege: {privilege}"
        );
    }
    let connection =
        AccountStore::connect_dev(config("central_runtime", "CENTRAL_TEST_RUNTIME_PASSWORD"))
            .await
            .unwrap();
    connection.close().await.unwrap();
}

async fn registration(
    store: &AccountStore,
    operator: &DatabaseConnection,
    owner_session: &Token,
    member_session: &Token,
    origin: &str,
) -> [u8; 16] {
    let stale = Token::generate().unwrap();
    let heartbeat = Token::generate().unwrap();
    let gateway = store
        .register_gateway(
            owner_session,
            origin,
            &metadata(origin),
            &stale,
            &heartbeat,
            false,
        )
        .await
        .unwrap();
    assert!(store.gateway_directory(None).await.unwrap().is_empty());
    assert!(store.authenticate_gateway(&stale).await.is_err());
    assert!(
        store
            .gateway_heartbeat(&heartbeat, &metadata(origin))
            .await
            .is_err()
    );
    assert!(
        store
            .register_gateway(
                member_session,
                origin,
                &metadata(origin),
                &Token::generate().unwrap(),
                &Token::generate().unwrap(),
                false
            )
            .await
            .is_err()
    );
    let credential = Token::generate().unwrap();
    assert_eq!(
        store
            .register_gateway(
                owner_session,
                origin,
                &metadata(origin),
                &credential,
                &heartbeat,
                false
            )
            .await
            .unwrap(),
        gateway
    );
    approval(operator, gateway, true).await;
    assert!(store.authenticate_gateway(&stale).await.is_err());
    assert_eq!(
        store.authenticate_gateway(&credential).await.unwrap().0,
        gateway
    );
    assert!(store.authenticate_gateway(&heartbeat).await.is_err());
    assert!(
        store
            .gateway_heartbeat(&credential, &metadata(origin))
            .await
            .is_err()
    );
    assert!(
        store
            .register_gateway(
                owner_session,
                origin,
                &metadata(origin),
                &Token::generate().unwrap(),
                &Token::generate().unwrap(),
                false
            )
            .await
            .is_err()
    );
    directory(store, gateway, origin, &heartbeat).await;
    gateway
}

async fn directory(store: &AccountStore, gateway: [u8; 16], origin: &str, heartbeat: &Token) {
    let directory = store.gateway_directory(None).await.unwrap();
    assert_eq!(directory.len(), 1);
    assert!(!directory[0].online);
    store
        .gateway_heartbeat(heartbeat, &metadata(origin))
        .await
        .unwrap();
    assert!(matches!(
        store.gateway_heartbeat(heartbeat, &metadata(origin)).await,
        Err(Error::Busy)
    ));
    assert!(store.gateway_directory(None).await.unwrap()[0].online);
    assert!(
        store
            .gateway_directory(Some(gateway))
            .await
            .unwrap()
            .is_empty()
    );
    assert!(store.gateway_directory(Some([0; 16])).await.is_err());
}

async fn admin_revocation(
    store: &AccountStore,
    owner_session: &Token,
    member_session: &Token,
    gateway: [u8; 16],
    origin: &str,
    owner: [u8; 16],
    member: [u8; 16],
) {
    let admin_ticket = Token::generate().unwrap();
    store
        .registered_ticket(member_session, origin, &admin_ticket, TicketPurpose::Admin)
        .await
        .unwrap();
    let admin_lease = Token::generate().unwrap();
    store
        .redeem_lease(
            &admin_ticket,
            origin,
            TicketPurpose::Admin,
            Some(gateway),
            Some(&admin_lease),
        )
        .await
        .unwrap();
    let pending_admin = Token::generate().unwrap();
    store
        .registered_ticket(member_session, origin, &pending_admin, TicketPurpose::Admin)
        .await
        .unwrap();
    store
        .set_gateway_admins(owner_session, gateway, 2, &[])
        .await
        .unwrap();
    assert!(!store.gateway_admin(gateway, member).await.unwrap());
    assert!(
        store
            .gateway_session(&admin_lease, origin, Some(gateway), TicketPurpose::Admin)
            .await
            .is_err()
    );
    assert!(
        store
            .redeem_lease(
                &pending_admin,
                origin,
                TicketPurpose::Admin,
                Some(gateway),
                Some(&Token::generate().unwrap())
            )
            .await
            .is_err()
    );
    assert!(
        store
            .registered_ticket(
                member_session,
                origin,
                &Token::generate().unwrap(),
                TicketPurpose::Admin
            )
            .await
            .is_err()
    );
    assert!(!store.gateway_admin(gateway, owner).await.unwrap());
}
