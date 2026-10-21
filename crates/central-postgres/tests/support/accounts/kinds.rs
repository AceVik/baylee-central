//! One session per account and kind; a browser session is no native bearer.
use super::{AccountStore, Error, Identity, ServerKeys, TicketPurpose, Token, enroll, login};
use central_postgres::Session;
use sea_orm::{ConnectionTrait, DatabaseConnection};

pub(super) async fn run(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let identity = enroll(store, keys, "session-kinds", "session-kinds").await;
    let native = login(store, keys, identity).await;
    store.confirm_restore(&native, 1).await.unwrap();
    let credential = store.credential(identity).await.unwrap().unwrap();
    let browser = Token::generate().unwrap();
    let issued = store
        .issue_session(&credential, Session::Browser(&browser))
        .await
        .unwrap();
    let (status, expires_at, closed) = store
        .session_status(Session::Browser(&browser))
        .await
        .unwrap();
    assert_eq!(status.account, identity.account);
    assert_eq!(expires_at, issued.expires_at);
    assert!(!closed, "an open legacy migration stays open");
    assert_eq!(issued.wrapped_master.len(), 73);
    store.document(&native).await.unwrap();

    // A browser sign-in leaves the client's session; neither token works as the other.
    store.session_status(&native).await.unwrap();
    assert_eq!(
        store.session_status(&browser).await.err(),
        Some(Error::Rejected)
    );
    assert_eq!(
        store.session_status(Session::Browser(&native)).await.err(),
        Some(Error::Rejected)
    );
    assert!(store.document(&browser).await.is_err());
    assert!(store.terms_status(&browser, None).await.is_err());
    assert!(
        store
            .terms_status(Session::Browser(&browser), None)
            .await
            .unwrap()
    );
    let ticket = Token::generate().unwrap();
    assert!(
        store
            .gateway_ticket(&browser, "official", &ticket, TicketPurpose::Admit)
            .await
            .is_err()
    );
    assert!(store.create_object(&browser).await.is_err());

    replacement(store, operator, keys, identity, &browser, &native).await;
}

/// Each sign-in replaces the session of its own kind only, and each logout ends one.
async fn replacement(
    store: &AccountStore,
    operator: &DatabaseConnection,
    keys: &ServerKeys,
    identity: Identity,
    browser: &Token,
    native: &Token,
) {
    let credential = store.credential(identity).await.unwrap().unwrap();
    // A second browser sign-in replaces only the browser session.
    let second = Token::generate().unwrap();
    store
        .issue_session(&credential, Session::Browser(&second))
        .await
        .unwrap();
    assert!(
        store
            .session_status(Session::Browser(browser))
            .await
            .is_err()
    );
    store
        .session_status(Session::Browser(&second))
        .await
        .unwrap();
    store.document(native).await.unwrap();

    // A native sign-in replaces only the native session.
    let client = login(store, keys, identity).await;
    assert!(store.document(native).await.is_err());
    store
        .session_status(Session::Browser(&second))
        .await
        .unwrap();
    assert_eq!(
        kinds(operator, identity.account).await,
        ["browser".to_owned(), "native".to_owned()]
    );

    // Logging out by the native digest cannot end a browser session, nor the reverse.
    store.logout(&second).await.unwrap();
    store
        .session_status(Session::Browser(&second))
        .await
        .unwrap();
    store.logout(Session::Browser(&client)).await.unwrap();
    store.document(&client).await.unwrap();
    store.logout(Session::Browser(&second)).await.unwrap();
    assert!(
        store
            .session_status(Session::Browser(&second))
            .await
            .is_err()
    );
    store.document(&client).await.unwrap();
    assert_eq!(
        kinds(operator, identity.account).await,
        ["native".to_owned()]
    );

    // The schema itself holds one session per account and kind, and only these kinds.
    let account = uuid(identity.account);
    for (digest, kind) in [("11", "native"), ("22", "other")] {
        let duplicate = format!(
            "INSERT INTO central_identity.sessions (digest, account_id, credential_revision, kind, expires_at)
             SELECT decode(repeat('{digest}', 32), 'hex'), '{account}', credential_revision, '{kind}',
             clock_timestamp() + interval '1 minute' FROM central_identity.accounts WHERE id = '{account}'"
        );
        assert!(
            operator.execute_unprepared(&duplicate).await.is_err(),
            "{kind}"
        );
    }
    store.logout(&client).await.unwrap();
}

fn uuid(account: [u8; 16]) -> String {
    sea_orm::prelude::Uuid::from_bytes(account).to_string()
}

async fn kinds(operator: &DatabaseConnection, account: [u8; 16]) -> Vec<String> {
    operator
        .query_all_raw(sea_orm::Statement::from_string(
            sea_orm::DbBackend::Postgres,
            format!(
                "SELECT kind FROM central_identity.sessions WHERE account_id = '{}' ORDER BY kind",
                uuid(account)
            ),
        ))
        .await
        .unwrap()
        .iter()
        .map(|row| row.try_get("", "kind").unwrap())
        .collect()
}
