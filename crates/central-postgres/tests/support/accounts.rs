use super::config;
use central_api::{
    Endpoint, Error, Token,
    wire::{Identity, MigrationState, TicketPurpose},
};
use central_crypto::{
    Context,
    opaque::{Enrollment, Login, server::ServerKeys},
    vault::{MasterKey, ObjectContext, WrappedMasterKey},
};
use central_postgres::{AccountStore, Credential};
use sea_orm::{ConnectionTrait, DatabaseConnection};

mod admin;
mod gateways;
mod kinds;
mod migration;
mod objects;
mod password;
mod profiles;
mod recovery;
mod register;
mod registration;
mod renewal;
mod reset;
mod revocation_race;
mod single_session;
mod terms;

fn context(identity: Identity) -> Context {
    Context::new(
        Endpoint::development("http://127.0.0.1:28780")
            .unwrap()
            .issuer(),
        identity.account,
        identity.epoch,
    )
    .unwrap()
}

/// The runtime role's grants as the deploy hook applies them.
const GRANTS: &str = include_str!("../../../../ops/grants.sql");

pub(crate) async fn run(operator: &DatabaseConnection) {
    // Default grants are insufficient. Grant only the runtime CRUD operations.
    assert!(
        AccountStore::connect_dev(config("central_runtime", "CENTRAL_TEST_RUNTIME_PASSWORD"))
            .await
            .is_err()
    );
    // The deployed grants, exactly: the store's startup check must accept them.
    operator.execute_unprepared(GRANTS).await.unwrap();
    let store =
        AccountStore::connect_dev(config("central_runtime", "CENTRAL_TEST_RUNTIME_PASSWORD"))
            .await
            .unwrap();
    assert!(
        AccountStore::connect_dev(config("central_migrator", "CENTRAL_TEST_MIGRATOR_PASSWORD"))
            .await
            .is_err()
    );
    let generated = ServerKeys::generate();
    let persisted = generated.to_secret_bytes();
    assert_eq!(persisted.len(), 129);
    let keys = ServerKeys::from_secret_bytes(&persisted).unwrap();
    for length in [0, 1, 128, 130] {
        assert!(ServerKeys::from_secret_bytes(&vec![1; length]).is_err());
    }
    let first = enroll(&store, &keys, "synthetic-a", "alice").await;
    let second = enroll(&store, &keys, "synthetic-b", "bob").await;
    let alice = login(&store, &keys, first).await;
    let bob = login(&store, &keys, second).await;
    assert!(
        store
            .reserve_legacy(
                "synthetic-a",
                "alice",
                None,
                None,
                &Token::generate().unwrap()
            )
            .await
            .is_err()
    );
    assert!(
        store
            .reserve_legacy(
                "synthetic-c",
                "alice",
                None,
                None,
                &Token::generate().unwrap()
            )
            .await
            .is_err()
    );
    assert_eq!(
        store.discover("ALICE").await.unwrap().unwrap().account,
        first.account
    );
    assert!(store.discover("nobody").await.unwrap().is_none());
    sessions(&store, &alice, &bob, first).await;
    expiry_and_revision(&store, operator, &keys, first).await;
    gateways::run(&store, operator, &keys).await;
    single_session::run(&store, &keys).await;
    revocation_race::run(&store, operator, &keys).await;
    objects::run(&store, operator, &keys).await;
    migration::run(&store, operator, &keys).await;
    terms::run(&store, operator, &keys).await;
    kinds::run(&store, operator, &keys).await;
    registration::run(&store, operator, &keys).await;
    register::run(&store, operator, &keys).await;
    renewal::run(&store, operator, &keys).await;
    recovery::run(&store, operator, &keys).await;
    profiles::run(&store, operator, &keys).await;
    password::run(&store, operator, &keys).await;
    admin::run(&store, operator, &keys).await;
    reset::run(&store, operator, &keys).await;
    store.close().await.unwrap();
}

async fn enroll(store: &AccountStore, keys: &ServerKeys, legacy: &str, name: &str) -> Identity {
    let stale = Token::generate().unwrap();
    let identity = store
        .reserve_legacy(legacy, name, None, None, &stale)
        .await
        .unwrap();
    let ticket = Token::generate().unwrap();
    assert_eq!(
        store
            .reserve_legacy(legacy, name, None, None, &ticket)
            .await
            .unwrap()
            .account,
        identity.account
    );
    assert!(store.enrollment(&stale).await.is_err());
    let context = context(identity);
    let (enrollment, request) =
        Enrollment::start("synthetic strong password".into(), context).unwrap();
    let response = keys.enrollment_response(context, &request).unwrap();
    let enrolled = enrollment.finish(&response).unwrap();
    let master = MasterKey::generate().unwrap();
    let wrapped = enrolled.wrapping_key.wrap(&master).unwrap();
    let object = ObjectContext::new(context, identity.account, 1).unwrap();
    let document = master.seal_object(object, name.as_bytes()).unwrap();
    assert!(
        store
            .enroll(&ticket, &enrolled.upload, &[1; 72], &document, None)
            .await
            .is_err()
    );
    assert_eq!(
        store
            .enroll(
                &ticket,
                &enrolled.upload,
                wrapped.as_bytes(),
                &document,
                Some(wrapped.as_bytes())
            )
            .await,
        Err(Error::Rejected),
        "a legacy account takes no recovery wrapper, refused before the schema's rule"
    );
    assert!(
        store.credential(identity).await.unwrap().is_none(),
        "a failed enrollment creates no partial credential"
    );
    store
        .enroll(
            &ticket,
            &enrolled.upload,
            wrapped.as_bytes(),
            &document,
            None,
        )
        .await
        .unwrap();
    assert!(
        store
            .enroll(
                &ticket,
                &enrolled.upload,
                wrapped.as_bytes(),
                &document,
                None
            )
            .await
            .is_err()
    );
    assert!(store.enrollment(&ticket).await.is_err());
    assert!(store.legacy_status(legacy).await.unwrap().1 == MigrationState::Enrolled);
    identity
}

async fn login(store: &AccountStore, keys: &ServerKeys, identity: Identity) -> Token {
    let credential = store.credential(identity).await.unwrap().unwrap();
    let record = ServerKeys::enrollment_record(&credential.record).unwrap();
    let (login, request) =
        Login::start("synthetic strong password".into(), context(identity)).unwrap();
    let (attempt, response) = keys
        .start_login(context(identity), Some(&record), &request)
        .unwrap();
    let result = login.finish(&response).unwrap();
    attempt.finish(&result.finalization).unwrap();
    let token = Token::generate().unwrap();
    let wrapped = store
        .issue_session(&credential, &token)
        .await
        .unwrap()
        .wrapped_master;
    let master = result
        .wrapping_key
        .unwrap(&WrappedMasterKey::from_bytes(&wrapped).unwrap())
        .unwrap();
    let (revision, document) = store.document(&token).await.unwrap();
    master
        .open_object(
            ObjectContext::new(context(identity), identity.account, revision).unwrap(),
            &document,
        )
        .unwrap();
    token
}

async fn sessions(store: &AccountStore, alice: &Token, bob: &Token, first: Identity) {
    assert!(store.document(&Token::generate().unwrap()).await.is_err());
    let (_, original) = store.document(alice).await.unwrap();
    let (_, bob_original) = store.document(bob).await.unwrap();
    assert_ne!(original, bob_original);
    let ticket = Token::generate().unwrap();
    assert!(
        store
            .gateway_ticket(alice, "synthetic-gateway", &ticket, TicketPurpose::Admit)
            .await
            .is_err()
    );
    assert!(store.confirm_restore(alice, 2).await.is_err());
    store.confirm_restore(alice, 1).await.unwrap();
    assert!(store.legacy_status("synthetic-a").await.unwrap().1 == MigrationState::Restored);
    store
        .gateway_ticket(alice, "synthetic-gateway", &ticket, TicketPurpose::Admit)
        .await
        .unwrap();
    for purpose in [TicketPurpose::Export, TicketPurpose::Link] {
        assert!(
            store
                .redeem_gateway(&ticket, "synthetic-gateway", purpose)
                .await
                .is_err()
        );
    }
    assert!(
        store
            .redeem_gateway(&ticket, "wrong-audience", TicketPurpose::Admit)
            .await
            .is_err()
    );
    assert_eq!(
        store
            .redeem_gateway(&ticket, "synthetic-gateway", TicketPurpose::Admit)
            .await
            .unwrap()
            .0
            .account,
        first.account
    );
    assert!(
        store
            .redeem_gateway(&ticket, "synthetic-gateway", TicketPurpose::Admit)
            .await
            .is_err()
    );
    let (a, b) = tokio::join!(
        store.write_document(alice, 1, &original),
        store.write_document(alice, 1, &original)
    );
    assert!(matches!(
        (a, b),
        (Ok(()), Err(Error::Conflict)) | (Err(Error::Conflict), Ok(()))
    ));
    assert_eq!(store.document(alice).await.unwrap().0, 2);
    assert_eq!(store.document(bob).await.unwrap(), (1, bob_original));
    let ticket = Token::generate().unwrap();
    store
        .gateway_ticket(alice, "synthetic-gateway", &ticket, TicketPurpose::Admit)
        .await
        .unwrap();
    store.logout(alice).await.unwrap();
    assert!(store.document(alice).await.is_err());
    assert!(
        store
            .redeem_gateway(&ticket, "synthetic-gateway", TicketPurpose::Admit)
            .await
            .is_err()
    );
    store.logout(alice).await.unwrap();
}

async fn expiry_and_revision(
    store: &AccountStore,
    operator: &DatabaseConnection,
    keys: &ServerKeys,
    first: Identity,
) {
    let credential = store.credential(first).await.unwrap().unwrap();
    let stale = Credential {
        revision: credential.revision - 1,
        ..credential
    };
    assert!(
        store
            .issue_session(&stale, &Token::generate().unwrap())
            .await
            .is_err()
    );
    let token = Token::generate().unwrap();
    store
        .reserve_legacy("expired", "expired", None, None, &token)
        .await
        .unwrap();
    operator.execute_unprepared("UPDATE central_identity.accounts SET enrollment_expires = clock_timestamp() - interval '1 second' WHERE legacy_id = 'expired'").await.unwrap();
    assert!(store.enrollment(&token).await.is_err());
    // Current ciphertext has deliberately stale AEAD metadata after the CAS race fixture.
    // A real client must refuse this, rather than silently create a new account document.
    let record = ServerKeys::enrollment_record(&stale.record).unwrap();
    let (wrong, request) =
        Login::start("incorrect synthetic password".into(), context(first)).unwrap();
    let (_, response) = keys
        .start_login(context(first), Some(&record), &request)
        .unwrap();
    assert!(wrong.finish(&response).is_err());
}
