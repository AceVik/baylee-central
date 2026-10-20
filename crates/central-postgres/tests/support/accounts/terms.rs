//! Terms records at the store: immutable for the runtime role, removed with
//! their account, and every check fails closed (`docs/terms.md`).
use super::{AccountStore, DatabaseConnection, ServerKeys, Token, config, enroll, login};
use central_api::{
    Error,
    terms::{self, TermsText, TermsVersion},
};
use central_postgres::MAX_GATEWAY_VERSIONS;
use sea_orm::{ConnectionTrait, DbBackend, Statement};

fn texts(title: &str) -> Vec<TermsText> {
    vec![TermsText {
        lang: "en".into(),
        updated: None,
        markdown: format!("# {title}\n"),
    }]
}

fn version(name: &str, texts: &[TermsText]) -> TermsVersion {
    TermsVersion {
        version: name.into(),
        digest: terms::digest(texts),
    }
}

async fn scalar(operator: &DatabaseConnection, sql: &str) -> i64 {
    operator
        .query_one_raw(Statement::from_string(DbBackend::Postgres, sql))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "value")
        .unwrap()
}

const AUDIENCE: &str = "http://127.0.0.1:28791";

pub(super) async fn run(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let account = enroll(store, keys, "terms-reader", "terms-reader").await;
    let session = login(store, keys, account).await;
    central(store, &session).await;
    gateway(store, &session).await;
    immutable().await;
    served_only_as_digested(store, operator).await;
    limited(store, operator).await;
    removed_with_the_account(operator, account.account).await;
}

async fn central(store: &AccountStore, session: &Token) {
    let central = version("c-1", &texts("Central"));

    assert!(store.terms_status(session, None).await.unwrap());
    assert!(!store.terms_status(session, Some(&central)).await.unwrap());
    assert_eq!(
        store.terms_status(&Token::generate().unwrap(), None).await,
        Err(Error::Rejected),
        "a dead session is refused even when nothing is in force"
    );
    store.accept_central_terms(session, &central).await.unwrap();
    store.accept_central_terms(session, &central).await.unwrap();
    assert!(store.terms_status(session, Some(&central)).await.unwrap());
    let mut changed = central.clone();
    changed.digest[0] ^= 1;
    assert!(
        !store.terms_status(session, Some(&changed)).await.unwrap(),
        "a text changed under the same version is not accepted"
    );
    assert_eq!(
        store
            .accept_central_terms(&Token::generate().unwrap(), &central)
            .await,
        Err(Error::Rejected)
    );
}

async fn gateway(store: &AccountStore, session: &Token) {
    let audience = AUDIENCE;
    let first = texts("Gateway");
    let published = version("g-1", &first);
    assert!(store.gateway_terms(audience).await.unwrap().is_none());
    assert_eq!(
        store
            .accept_gateway_terms(session, audience, &published)
            .await,
        Err(Error::Conflict)
    );
    store
        .publish_gateway_terms(audience, &published, &first)
        .await
        .unwrap();
    store
        .publish_gateway_terms(audience, &published, &first)
        .await
        .unwrap();
    let other = texts("Other");
    assert_eq!(
        store
            .publish_gateway_terms(audience, &version("g-1", &other), &other)
            .await,
        Err(Error::Conflict)
    );
    let current = store.gateway_terms(audience).await.unwrap().unwrap();
    assert!(current.version == published && current.texts == first);
    store
        .accept_gateway_terms(session, audience, &published)
        .await
        .unwrap();
    store
        .accept_gateway_terms(session, audience, &published)
        .await
        .unwrap();
}

/// The runtime role can neither rewrite nor remove a record.
async fn immutable() {
    let runtime =
        central_postgres::connect_dev(config("central_runtime", "CENTRAL_TEST_RUNTIME_PASSWORD"))
            .await
            .unwrap();
    for sql in [
        "UPDATE central_identity.terms_acceptances SET version = 'forged'",
        "DELETE FROM central_identity.terms_acceptances",
        "UPDATE central_identity.gateway_terms SET digest = digest",
        "DELETE FROM central_identity.gateway_terms",
        "UPDATE central_identity.gateway_terms_texts SET markdown = 'forged'",
        "DELETE FROM central_identity.gateway_terms_texts",
    ] {
        assert!(runtime.execute_unprepared(sql).await.is_err(), "{sql}");
    }
    runtime.close().await.unwrap();
}

/// Stored texts that no longer match their digest are never served.
async fn served_only_as_digested(store: &AccountStore, operator: &DatabaseConnection) {
    let audience = AUDIENCE;
    operator
        .execute_unprepared(
            "UPDATE central_identity.gateway_terms_texts SET markdown = '# Tampered' || chr(10)
             WHERE audience = 'http://127.0.0.1:28791'",
        )
        .await
        .unwrap();
    assert_eq!(store.gateway_terms(audience).await, Err(Error::Unavailable));
    operator
        .execute_unprepared(
            "UPDATE central_identity.gateway_terms_texts SET markdown = '# Gateway' || chr(10)
             WHERE audience = 'http://127.0.0.1:28791'",
        )
        .await
        .unwrap();
    assert!(store.gateway_terms(audience).await.unwrap().is_some());
}

/// A gateway publishes at most so many versions.
async fn limited(store: &AccountStore, operator: &DatabaseConnection) {
    let crowded = "http://127.0.0.1:28792";
    operator
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO central_identity.gateway_terms (audience, version, digest)
             SELECT $1, 'v' || n, decode(repeat('00', 32), 'hex') FROM generate_series(1, $2) n",
            vec![crowded.into(), MAX_GATEWAY_VERSIONS.into()],
        ))
        .await
        .unwrap();
    let late = texts("Late");
    assert_eq!(
        store
            .publish_gateway_terms(crowded, &version("late", &late), &late)
            .await,
        Err(Error::Rejected)
    );
}

/// Acceptances go with their account.
async fn removed_with_the_account(operator: &DatabaseConnection, account: [u8; 16]) {
    let before = scalar(
        operator,
        "SELECT count(*)::bigint AS value FROM central_identity.terms_acceptances",
    )
    .await;
    assert!(before >= 2);
    operator
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "DELETE FROM central_identity.accounts WHERE id = $1",
            vec![sea_orm::prelude::Uuid::from_bytes(account).into()],
        ))
        .await
        .unwrap();
    assert_eq!(
        scalar(
            operator,
            "SELECT count(*)::bigint AS value FROM central_identity.terms_acceptances"
        )
        .await,
        before - 2
    );
}
