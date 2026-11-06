use super::{AccountStore, DatabaseConnection, Error, ServerKeys, Token, config, enroll, login};
use central_api::{
    gateway::GatewayMetadata,
    wire::GatewayPurpose::{Admin, Admit},
};
use sea_orm::{ConnectionTrait, DbBackend, Statement, prelude::Uuid};

mod limits;
mod official;
mod streams;
mod tokens;

/// Whether an account is on a gateway's list, as the operator reads it.
async fn listed(operator: &DatabaseConnection, gateway: [u8; 16], account: [u8; 16]) -> bool {
    let row = operator
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT EXISTS(SELECT 1 FROM central_identity.gateway_admins
             WHERE gateway_id = $1 AND account_id = $2) AS listed",
            vec![
                Uuid::from_bytes(gateway).into(),
                Uuid::from_bytes(account).into(),
            ],
        ))
        .await
        .unwrap()
        .unwrap();
    row.try_get("", "listed").unwrap()
}

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
    operator
        .execute_unprepared(
            "GRANT SELECT,INSERT,UPDATE ON central_identity.gateways TO central_runtime;
         GRANT SELECT ON central_identity.gateway_approvals TO central_runtime;
         GRANT SELECT,INSERT,DELETE ON central_identity.gateway_admins TO central_runtime",
        )
        .await
        .unwrap();
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
    assert!(!listed(operator, gateway, member.account).await);
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
    assert!(!listed(operator, gateway, owner.account).await);
    assert!(listed(operator, gateway, member.account).await);
    assert!(!listed(operator, foreign, owner.account).await);
    delegation(store, &member_session, operator, gateway, origin).await;

    admin_revocation(
        store,
        operator,
        &owner_session,
        &member_session,
        gateway,
        origin,
        owner.account,
        member.account,
    )
    .await;
    limits::run(store, operator, &owner_session).await;
    // The official gateway admits a moved account once its move is closed.
    store.close_legacy("registry-owner", owner).await.unwrap();
    official::run(store, operator, &owner_session, gateway).await;
    tokens::run(store, operator, keys).await;
    streams::run(store, operator, keys).await;
}

/// A registered gateway's token: no legacy id, the session's cap, and no token for an origin
/// nobody registered.
async fn delegation(
    store: &AccountStore,
    session: &Token,
    operator: &DatabaseConnection,
    gateway: [u8; 16],
    origin: &str,
) {
    let grant = store
        .gateway_grant(session, &[origin], true, Admit, [61; 16])
        .await
        .unwrap();
    assert!(grant.legacy_id.is_none(), "never to a registered gateway");
    assert!(grant.cap > 0);
    assert!(matches!(
        store
            .gateway_grant(
                session,
                &["https://unregistered.invalid"],
                true,
                Admit,
                [62; 16]
            )
            .await
            .err(),
        Some(Error::Rejected)
    ));
    profile(store, session, origin).await;
    held(store, operator, session, gateway, origin).await;
}

/// A disabled gateway gets no token, and is hidden; enabled again it has the same `sid`.
async fn held(
    store: &AccountStore,
    operator: &DatabaseConnection,
    session: &Token,
    gateway: [u8; 16],
    origin: &str,
) {
    approval(operator, gateway, false).await;
    assert!(matches!(
        store
            .gateway_grant(session, &[origin], true, Admit, [63; 16])
            .await
            .err(),
        Some(Error::Rejected)
    ));
    assert!(
        store
            .gateway_directory(None)
            .await
            .unwrap()
            .iter()
            .all(|entry| entry.id != gateway)
    );
    approval(operator, gateway, true).await;
    assert_eq!(
        store
            .gateway_grant(session, &[origin], true, Admit, [63; 16])
            .await
            .unwrap()
            .sid,
        [61; 16]
    );
}

/// A token names the account as others see it: a moved account reserved before display
/// names has neither name nor tag until the bridge carries them, and a refresh picks them up.
async fn profile(store: &AccountStore, session: &Token, origin: &str) {
    let before = store
        .gateway_grant(session, &[origin], true, Admit, [64; 16])
        .await
        .unwrap();
    assert_eq!((before.display_name, before.tag), (None, None));
    store
        .legacy_profile("registry-member", Some("Registry_Member"), 0x331)
        .await
        .unwrap();
    let after = store
        .gateway_grant(session, &[origin], true, Admit, [64; 16])
        .await
        .unwrap();
    assert_eq!(
        (after.display_name.as_deref(), after.tag),
        (Some("Registry_Member"), Some(0x331))
    );
}

/// The runtime approves, enables and disables gateways (an administrator's
/// decision, 000014) and stores gateway tokens, and refuses to start with
/// anything wider on either table: deleting an approval, another column, a
/// grant option; or narrower: no `INSERT` on approvals.
async fn privilege_gate(operator: &DatabaseConnection) {
    let connect =
        || AccountStore::connect_dev(config("central_runtime", "CENTRAL_TEST_RUNTIME_PASSWORD"));
    for (grant, revoke) in [
        (
            "GRANT DELETE ON central_identity.gateway_approvals TO central_runtime",
            "REVOKE DELETE ON central_identity.gateway_approvals FROM central_runtime",
        ),
        (
            "GRANT UPDATE (approved_at) ON central_identity.gateway_approvals TO central_runtime",
            "REVOKE UPDATE (approved_at) ON central_identity.gateway_approvals FROM central_runtime",
        ),
        (
            "GRANT SELECT ON central_identity.gateway_approvals TO central_runtime WITH GRANT OPTION",
            "REVOKE GRANT OPTION FOR SELECT ON central_identity.gateway_approvals FROM central_runtime",
        ),
        (
            "GRANT DELETE ON central_identity.gateway_tokens TO central_runtime",
            "REVOKE DELETE ON central_identity.gateway_tokens FROM central_runtime",
        ),
        (
            "GRANT UPDATE (digest) ON central_identity.gateway_tokens TO central_runtime",
            "REVOKE UPDATE (digest) ON central_identity.gateway_tokens FROM central_runtime",
        ),
        (
            "GRANT UPDATE (expires_at) ON central_identity.gateway_tokens TO central_runtime",
            "REVOKE UPDATE (expires_at) ON central_identity.gateway_tokens FROM central_runtime",
        ),
        (
            "REVOKE INSERT ON central_identity.gateway_approvals FROM central_runtime",
            "GRANT INSERT ON central_identity.gateway_approvals TO central_runtime",
        ),
        (
            "REVOKE UPDATE (last_used_at) ON central_identity.gateway_tokens FROM central_runtime",
            "GRANT UPDATE (last_used_at) ON central_identity.gateway_tokens TO central_runtime",
        ),
    ] {
        operator.execute_unprepared(grant).await.unwrap();
        let connection = connect().await;
        operator.execute_unprepared(revoke).await.unwrap();
        assert!(connection.is_err(), "unsafe gateway privilege: {grant}");
    }
    connect().await.unwrap().close().await.unwrap();
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
    operator: &DatabaseConnection,
    owner_session: &Token,
    member_session: &Token,
    gateway: [u8; 16],
    origin: &str,
    owner: [u8; 16],
    member: [u8; 16],
) {
    let sid = store
        .gateway_grant(member_session, &[origin], true, Admin, [65; 16])
        .await
        .unwrap()
        .sid;
    let before = store.last_revocation(origin).await.unwrap();
    store
        .set_gateway_admins(owner_session, gateway, 2, &[])
        .await
        .unwrap();
    assert!(!listed(operator, gateway, member).await);
    // The gateway hears of it, and no new token says otherwise.
    let page = store.revocation_page(origin, before).await.unwrap();
    assert_eq!(page.sids, vec![sid]);
    assert!(matches!(
        store
            .gateway_grant(member_session, &[origin], true, Admin, [66; 16])
            .await
            .err(),
        Some(Error::Rejected)
    ));
    assert!(!listed(operator, gateway, owner).await);
}
