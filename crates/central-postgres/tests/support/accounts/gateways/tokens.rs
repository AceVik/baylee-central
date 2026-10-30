//! Gateway tokens and the account area (migration 000014): a gateway
//! registered on the website, its tokens minted only after a recent password,
//! scoped, expiring and revocable, authenticating only while the gateway is
//! approved; administrators approving and disabling (leases end), revoking a
//! token and changing a tag, each audited; an account's sessions and ending
//! the client's.
use super::{
    AccountStore, DatabaseConnection, Error, ServerKeys, TicketPurpose, Token, approval, enroll,
    login,
};
use central_api::LEGACY_ORIGIN;
use central_api::gateway::{GatewayMetadata, GatewayState};
use central_api::owner::{GatewayScopes, SessionKind};
use central_postgres::{Session, appoint_admin};
use sea_orm::{ConnectionTrait, DbBackend, Statement, prelude::Uuid};

const ORIGIN: &str = "https://tokens.invalid";
const BOTH: GatewayScopes = GatewayScopes {
    sso: true,
    listing: true,
};
const SSO: GatewayScopes = GatewayScopes {
    sso: true,
    listing: false,
};
const LISTING: GatewayScopes = GatewayScopes {
    sso: false,
    listing: true,
};

async fn count(operator: &DatabaseConnection, sql: &str, values: Vec<sea_orm::Value>) -> i64 {
    operator
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            values,
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<i64>("", "n")
        .unwrap()
}

/// An account whose move is finished, with its client session and a browser
/// session as the website's sign-in issues it.
async fn moved(
    store: &AccountStore,
    keys: &ServerKeys,
    legacy: &str,
) -> (central_api::wire::Identity, Token, Token) {
    let identity = enroll(store, keys, legacy, legacy).await;
    let native = login(store, keys, identity).await;
    store.confirm_restore(&native, 1).await.unwrap();
    store.close_legacy(legacy, identity).await.unwrap();
    let credential = store.credential(identity).await.unwrap().unwrap();
    let browser = Token::generate().unwrap();
    store
        .issue_session(&credential, Session::Browser(&browser))
        .await
        .unwrap();
    (identity, native, browser)
}

/// Move the browser session's password sign-in back by `minutes`.
async fn age(operator: &DatabaseConnection, account: [u8; 16], minutes: i64) {
    operator
        .execute_unprepared(&format!(
            "UPDATE central_identity.sessions SET created_at = created_at - interval '{minutes} minutes'
             WHERE account_id = '{}' AND kind = 'browser'",
            Uuid::from_bytes(account)
        ))
        .await
        .unwrap();
}

fn beat() -> GatewayMetadata {
    GatewayMetadata {
        name: "Synthetic token gateway".into(),
        version: "1.0-test".into(),
        website: format!("{ORIGIN}/"),
    }
}

pub(super) async fn run(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let (owner, native, browser) = moved(store, keys, "token-owner").await;
    let (other, _, stranger) = moved(store, keys, "token-stranger").await;
    let gateway = registration(store, operator, &browser, &native, &stranger).await;
    let token = minting(store, operator, owner.account, &browser, &stranger, gateway).await;
    authentication(store, operator, &browser, gateway, &token).await;
    let admin = administration(store, operator, keys, gateway, &native, &browser).await;
    tags(store, operator, admin, owner.account, other.account).await;
    sessions(store, &browser, &native).await;
}

/// Registering on the website: pending, token-only, one owner per origin.
async fn registration(
    store: &AccountStore,
    operator: &DatabaseConnection,
    browser: &Token,
    native: &Token,
    stranger: &Token,
) -> [u8; 16] {
    for (origin, refusal) in [
        (LEGACY_ORIGIN, Error::Conflict),
        ("http://tokens.invalid", Error::Rejected),
        ("https://tokens.invalid/path", Error::Rejected),
    ] {
        assert_eq!(
            store
                .add_gateway(browser, origin, "Synthetic", false)
                .await
                .err(),
            Some(refusal),
            "{origin}"
        );
    }
    assert_eq!(
        store
            .add_gateway(browser, ORIGIN, "bad\nname", false)
            .await
            .err(),
        Some(Error::Rejected)
    );
    // a native bearer is no browser session here
    assert_eq!(
        store
            .add_gateway(native, ORIGIN, "Synthetic", false)
            .await
            .err(),
        Some(Error::Rejected)
    );
    let gateway = store
        .add_gateway(browser, ORIGIN, "Synthetic token gateway", false)
        .await
        .unwrap();
    assert_eq!(
        store
            .add_gateway(stranger, ORIGIN, "Taken", false)
            .await
            .err(),
        Some(Error::Conflict)
    );
    assert_eq!(
        count(
            operator,
            "SELECT count(*) AS n FROM central_identity.gateways
             WHERE id = $1 AND credential_digest IS NULL AND heartbeat_digest IS NULL AND NOT official",
            vec![Uuid::from_bytes(gateway).into()],
        )
        .await,
        1,
        "token-only"
    );
    let listed = store.owner_gateways(browser).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].state, GatewayState::Pending);
    assert!(store.owner_gateways(stranger).await.unwrap().is_empty());
    gateway
}

/// Minting: a fresh sign-in, the owner, sane bounds, eight live at most.
async fn minting(
    store: &AccountStore,
    operator: &DatabaseConnection,
    owner: [u8; 16],
    browser: &Token,
    stranger: &Token,
    gateway: [u8; 16],
) -> Token {
    let token = Token::generate().unwrap();
    for (name, scopes, days) in [
        ("", BOTH, 30),
        (
            "Main",
            GatewayScopes {
                sso: false,
                listing: false,
            },
            30,
        ),
        ("Main", BOTH, 0),
        ("Main", BOTH, 367),
    ] {
        assert_eq!(
            store
                .create_gateway_token(browser, gateway, name, scopes, days, &token)
                .await
                .err(),
            Some(Error::Rejected),
            "{name:?} {days}"
        );
    }
    assert_eq!(
        store
            .create_gateway_token(stranger, gateway, "Theirs", BOTH, 30, &token)
            .await
            .err(),
        Some(Error::Rejected),
        "another account's gateway"
    );
    age(operator, owner, 31).await;
    assert_eq!(
        store
            .create_gateway_token(browser, gateway, "Main", BOTH, 30, &token)
            .await
            .err(),
        Some(Error::Rejected),
        "a stale sign-in mints nothing"
    );
    age(operator, owner, -31).await;
    let (_, expires_at) = store
        .create_gateway_token(browser, gateway, "Main", BOTH, 366, &token)
        .await
        .unwrap();
    let now = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    )
    .unwrap();
    assert!(
        (expires_at - now - 366 * 86_400).abs() < 120,
        "366 days exactly"
    );
    assert_eq!(
        count(
            operator,
            "SELECT count(*) AS n FROM central_identity.gateway_tokens WHERE digest = $1",
            vec![token.digest().to_vec().into()],
        )
        .await,
        1,
        "the digest is stored, never the token"
    );
    capacity(store, browser, stranger, gateway).await;
    token
}

/// Eight live tokens at most; revoked ones free their slots, and only their
/// owner revokes them.
async fn capacity(store: &AccountStore, browser: &Token, stranger: &Token, gateway: [u8; 16]) {
    for n in 1..8 {
        store
            .create_gateway_token(
                browser,
                gateway,
                &format!("Spare {n}"),
                SSO,
                1,
                &Token::generate().unwrap(),
            )
            .await
            .unwrap();
    }
    assert_eq!(
        store
            .create_gateway_token(
                browser,
                gateway,
                "Ninth",
                SSO,
                1,
                &Token::generate().unwrap()
            )
            .await
            .err(),
        Some(Error::Busy)
    );
    // the spares go, so their slots are free again
    for info in store.owner_gateways(browser).await.unwrap()[0]
        .tokens
        .iter()
        .filter(|t| t.name.starts_with("Spare"))
    {
        store
            .revoke_own_gateway_token(browser, info.id)
            .await
            .unwrap();
        store
            .revoke_own_gateway_token(browser, info.id)
            .await
            .unwrap();
        assert_eq!(
            store
                .revoke_own_gateway_token(stranger, info.id)
                .await
                .err(),
            Some(Error::Rejected)
        );
    }
}

/// A token authenticates its gateway while it is approved, for its scopes.
async fn authentication(
    store: &AccountStore,
    operator: &DatabaseConnection,
    browser: &Token,
    gateway: [u8; 16],
    token: &Token,
) {
    // pending: nothing authenticates, but the console may ask its status
    assert_eq!(
        store.authenticate_gateway(token).await.err(),
        Some(Error::Rejected)
    );
    assert_eq!(
        store.gateway_heartbeat(token, &beat()).await.err(),
        Some(Error::Rejected)
    );
    let (id, origin, _, state, scopes, _) = store.gateway_status(token).await.unwrap();
    assert_eq!(
        (id, origin.as_str(), state, scopes),
        (gateway, ORIGIN, GatewayState::Pending, BOTH)
    );
    approval(operator, gateway, true).await;
    assert_eq!(
        store.authenticate_gateway(token).await.unwrap(),
        (gateway, ORIGIN.into())
    );
    store.gateway_heartbeat(token, &beat()).await.unwrap();
    assert!(
        store
            .gateway_directory(None)
            .await
            .unwrap()
            .iter()
            .any(|g| g.id == gateway && g.online && g.metadata.name == "Synthetic token gateway")
    );
    let used = store.owner_gateways(browser).await.unwrap()[0]
        .tokens
        .iter()
        .find(|t| t.name == "Main")
        .unwrap()
        .last_used_at;
    assert!(used.is_some(), "a heartbeat marks the token used");
    // scopes: an sso-only token beats for nothing, a listing-only one signs nobody in
    let sso = Token::generate().unwrap();
    let listing = Token::generate().unwrap();
    store
        .create_gateway_token(browser, gateway, "Sign-in", SSO, 30, &sso)
        .await
        .unwrap();
    store
        .create_gateway_token(browser, gateway, "List", LISTING, 30, &listing)
        .await
        .unwrap();
    assert!(store.authenticate_gateway(&sso).await.is_ok());
    assert_eq!(
        store.authenticate_gateway(&listing).await.err(),
        Some(Error::Rejected)
    );
    tokio::time::sleep(std::time::Duration::from_secs(6)).await;
    assert_eq!(
        store.gateway_heartbeat(&sso, &beat()).await.err(),
        Some(Error::Rejected)
    );
    store.gateway_heartbeat(&listing, &beat()).await.unwrap();
    // expired: refused at once
    operator
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE central_identity.gateway_tokens SET created_at = clock_timestamp() - interval '2 days',
             expires_at = clock_timestamp() - interval '1 day' WHERE digest = $1",
            vec![sso.digest().to_vec().into()],
        ))
        .await
        .unwrap();
    assert_eq!(
        store.authenticate_gateway(&sso).await.err(),
        Some(Error::Rejected)
    );
    assert_eq!(
        store.gateway_status(&sso).await.err(),
        Some(Error::Rejected)
    );
    // revoked: refused at once, and only that one
    let id = store.owner_gateways(browser).await.unwrap()[0]
        .tokens
        .iter()
        .find(|t| t.name == "List")
        .unwrap()
        .id;
    store.revoke_own_gateway_token(browser, id).await.unwrap();
    assert_eq!(
        store.gateway_status(&listing).await.err(),
        Some(Error::Rejected)
    );
    assert!(store.authenticate_gateway(token).await.is_ok());
}

/// Administrators approve, disable (its leases end) and revoke, each audited.
async fn administration(
    store: &AccountStore,
    operator: &DatabaseConnection,
    keys: &ServerKeys,
    gateway: [u8; 16],
    native: &Token,
    owner_browser: &Token,
) -> (Token, [u8; 16]) {
    let (admin, _, cookie) = moved(store, keys, "token-admin").await;
    // not yet an administrator
    assert_eq!(
        store
            .set_gateway_approval(&cookie, gateway, false)
            .await
            .err(),
        Some(Error::Rejected)
    );
    appoint_admin(operator, "token-admin").await.unwrap();
    let (page, _) = store
        .admin_gateways(&cookie, Some(GatewayState::Approved), None)
        .await
        .unwrap();
    let row = page.iter().find(|g| g.id == gateway).unwrap();
    assert_eq!(
        (row.live_tokens, row.credential_pair, row.official),
        (1, false, false)
    );
    let (_, tokens) = store.admin_gateway(&cookie, gateway).await.unwrap();
    assert!(tokens.iter().any(|t| t.name == "Main"));
    let lease = admitted(store, native, owner_browser, gateway).await;
    let audit = |action: &'static str| {
        count(
            operator,
            "SELECT count(*) AS n FROM central_identity.audit_events WHERE action = $1 AND gateway_id = $2",
            vec![action.into(), Uuid::from_bytes(gateway).into()],
        )
    };
    store
        .set_gateway_approval(&cookie, gateway, false)
        .await
        .unwrap();
    store
        .set_gateway_approval(&cookie, gateway, false)
        .await
        .unwrap();
    assert_eq!(
        audit("gateway.disabled").await,
        1,
        "the same state writes nothing"
    );
    assert_eq!(
        store
            .gateway_session(&lease, ORIGIN, Some(gateway), TicketPurpose::Admit)
            .await
            .err(),
        Some(Error::Rejected),
        "its leases end"
    );
    assert!(
        store
            .gateway_directory(None)
            .await
            .unwrap()
            .iter()
            .all(|g| g.id != gateway)
    );
    assert_eq!(
        disabled_status(store, owner_browser, gateway).await,
        GatewayState::Disabled,
        "the console still learns why"
    );
    store
        .set_gateway_approval(&cookie, gateway, true)
        .await
        .unwrap();
    assert_eq!(audit("gateway.approved").await, 1);
    // revoking as an administrator
    let main = tokens.iter().find(|t| t.name == "Main").unwrap().id;
    store
        .admin_revoke_gateway_token(&cookie, main)
        .await
        .unwrap();
    store
        .admin_revoke_gateway_token(&cookie, main)
        .await
        .unwrap();
    assert_eq!(audit("gateway.token_revoked").await, 1);
    // a stale sign-in changes nothing
    age(operator, admin.account, 31).await;
    assert_eq!(
        store
            .set_gateway_approval(&cookie, gateway, false)
            .await
            .err(),
        Some(Error::Rejected)
    );
    age(operator, admin.account, -31).await;
    (cookie, admin.account)
}

/// A player admitted at the gateway: a lease, which the owner's session
/// list names by the gateway's origin.
async fn admitted(
    store: &AccountStore,
    native: &Token,
    owner_browser: &Token,
    gateway: [u8; 16],
) -> Token {
    // a player admitted there holds a lease until the gateway is disabled
    let ticket = Token::generate().unwrap();
    store
        .registered_ticket(native, ORIGIN, &ticket, TicketPurpose::Admit)
        .await
        .unwrap();
    let lease = Token::generate().unwrap();
    store
        .redeem_lease(
            &ticket,
            ORIGIN,
            TicketPurpose::Admit,
            Some(gateway),
            Some(&lease),
        )
        .await
        .unwrap();
    let sessions = store.owner_sessions(owner_browser).await.unwrap();
    let client = sessions
        .iter()
        .find(|s| s.kind == SessionKind::Native)
        .unwrap();
    assert_eq!(client.gateways, vec![ORIGIN.to_owned()]);
    lease
}

/// What a disabled gateway's console learns with a new token.
async fn disabled_status(
    store: &AccountStore,
    owner_browser: &Token,
    gateway: [u8; 16],
) -> GatewayState {
    let token = Token::generate().unwrap();
    store
        .create_gateway_token(owner_browser, gateway, "While disabled", SSO, 1, &token)
        .await
        .unwrap();
    store.gateway_status(&token).await.unwrap().3
}

/// Tags: unique, audited with both, the same again nothing, the admin only.
async fn tags(
    store: &AccountStore,
    operator: &DatabaseConnection,
    admin: (Token, [u8; 16]),
    target: [u8; 16],
    other: [u8; 16],
) {
    let (cookie, admin_account) = admin;
    operator
        .execute_unprepared(&format!(
            "UPDATE central_identity.accounts SET tag = 180150001 WHERE id = '{}'",
            Uuid::from_bytes(other)
        ))
        .await
        .unwrap();
    assert_eq!(
        store.set_tag(&cookie, target, 180_150_001).await.err(),
        Some(Error::Conflict),
        "another account holds it"
    );
    for tag in [0, central_api::profile::MAX_TAG + 1] {
        assert_eq!(
            store.set_tag(&cookie, target, tag).await.err(),
            Some(Error::Rejected)
        );
    }
    store.set_tag(&cookie, target, 180_150_000).await.unwrap();
    store.set_tag(&cookie, target, 180_150_000).await.unwrap();
    let detail: String = operator
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT detail::text AS detail FROM central_identity.audit_events
             WHERE action = 'account.tag_changed' AND account_id = $1 AND actor_account = $2",
            vec![
                Uuid::from_bytes(target).into(),
                Uuid::from_bytes(admin_account).into(),
            ],
        ))
        .await
        .unwrap()
        .expect("one event, the second change wrote none")
        .try_get("", "detail")
        .unwrap();
    assert!(detail.contains("\"to\": 180150000"), "{detail}");
    // the administration's own fixture counts its administrators: this one goes
    operator
        .execute_unprepared(&format!(
            "DELETE FROM central_identity.account_roles WHERE account_id = '{}'",
            Uuid::from_bytes(admin_account)
        ))
        .await
        .unwrap();
}

/// The account's sessions, and ending the client's.
async fn sessions(store: &AccountStore, browser: &Token, native: &Token) {
    let before = store.owner_sessions(browser).await.unwrap();
    assert_eq!(before.len(), 2);
    assert!(before[0].current && before[0].kind == SessionKind::Browser);
    store.end_client_session(browser).await.unwrap();
    let after = store.owner_sessions(browser).await.unwrap();
    assert_eq!(after.len(), 1);
    assert!(
        store.session_status(native).await.is_err(),
        "the client's session is gone"
    );
    assert_eq!(
        store.owner_sessions(native).await.err(),
        Some(Error::Rejected)
    );
}
