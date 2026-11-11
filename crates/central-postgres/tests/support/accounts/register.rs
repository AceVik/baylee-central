//! `register` and the registered enrollment: open registration (since
//! 000023, the proof of work checked before, in the identity service),
//! closed registration refused before the name, reservations that expire,
//! reclaimed names and races.
use super::{AccountStore, Error, Identity, ServerKeys, Token, context};
use central_crypto::{
    opaque::Enrollment,
    vault::{MasterKey, ObjectContext, RecoveryCode},
};
use sea_orm::{ConnectionTrait, DatabaseConnection};

/// What a browser sends to enroll.
pub(super) struct Enrolled {
    pub(super) upload: Vec<u8>,
    pub(super) wrapped: Vec<u8>,
    /// The master key wrapped under a fresh recovery code; the server checks
    /// only its 73-byte format.
    pub(super) recovery: Vec<u8>,
    pub(super) document: Vec<u8>,
}

pub(super) fn credentials(keys: &ServerKeys, identity: Identity) -> Enrolled {
    let context = context(identity);
    let (enrollment, request) =
        Enrollment::start("synthetic strong password".into(), context).unwrap();
    let response = keys.enrollment_response(context, &request).unwrap();
    let enrolled = enrollment.finish(&response).unwrap();
    let master = MasterKey::generate().unwrap();
    let wrapped = enrolled.wrapping_key.wrap(&master).unwrap();
    let recovery = RecoveryCode::generate()
        .unwrap()
        .wrap(context, &master)
        .unwrap();
    let object = ObjectContext::new(context, identity.account, 1).unwrap();
    Enrolled {
        upload: enrolled.upload,
        wrapped: wrapped.as_bytes().to_vec(),
        recovery: recovery.as_bytes().to_vec(),
        document: master.seal_object(object, b"registered").unwrap(),
    }
}

async fn scalar(operator: &DatabaseConnection, sql: &str) -> i64 {
    super::super::scalar(operator, sql).await
}

async fn sql(operator: &DatabaseConnection, sql: &str) {
    // Fixed synthetic statements, never user input.
    operator.execute_unprepared(sql).await.unwrap();
}

fn expire(name: &str) -> String {
    format!(
        "UPDATE central_identity.accounts SET enrollment_expires = clock_timestamp() - interval '1 second'
         WHERE username_key = '{name}'"
    )
}

async fn register(store: &AccountStore, name: &str) -> Result<Identity, Error> {
    store
        .register(name, None, &Token::generate().unwrap())
        .await
}

pub(super) async fn run(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    refusals(store, operator).await;
    closed(store, operator).await;
    reservations(store, operator, keys).await;
    names(store, operator).await;
    races(store).await;
    sweep(store, operator).await;
}

async fn refusals(store: &AccountStore, operator: &DatabaseConnection) {
    let before = scalar(
        operator,
        "SELECT count(*)::bigint AS value FROM central_identity.accounts",
    )
    .await;
    // A taken name: enrolled, or a legacy migration's.
    assert_eq!(register(store, "ALICE").await.err(), Some(Error::Conflict));
    assert_eq!(
        register(store, "expired").await.err(),
        Some(Error::Conflict)
    );
    for name in [
        "ab",
        "a b",
        "-alice",
        "alice-",
        "a..b",
        &"x".repeat(25),
        "älice",
    ] {
        assert_eq!(
            register(store, name).await.err(),
            Some(Error::Rejected),
            "{name}"
        );
    }
    assert_eq!(
        scalar(
            operator,
            "SELECT count(*)::bigint AS value FROM central_identity.accounts"
        )
        .await,
        before,
        "no refusal reserved anything"
    );
}

async fn closed(store: &AccountStore, operator: &DatabaseConnection) {
    assert!(store.registration_open().await.unwrap());
    sql(
        operator,
        "UPDATE central_identity.registration SET open = false",
    )
    .await;
    assert!(!store.registration_open().await.unwrap());
    // Closed: refused before the name is looked at, a taken one alike.
    assert_eq!(
        register(store, "closed-name").await.err(),
        Some(Error::Rejected)
    );
    assert_eq!(register(store, "alice").await.err(), Some(Error::Rejected));
    sql(
        operator,
        "UPDATE central_identity.registration SET open = true",
    )
    .await;
    assert_eq!(
        scalar(
            operator,
            "SELECT count(*)::bigint AS value FROM central_identity.accounts WHERE username_key = 'closed-name'"
        )
        .await,
        0
    );
}

async fn reservations(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let first = Token::generate().unwrap();
    let a = store.register("reserve-a", None, &first).await.unwrap();
    // A live reservation holds its name.
    assert_eq!(
        register(store, "reserve-a").await.err(),
        Some(Error::Conflict)
    );
    // The reservation expires; its ticket no longer enrolls.
    sql(operator, &expire("reserve-a")).await;
    let second = Token::generate().unwrap();
    let b = store.register("reserve-b", None, &second).await.unwrap();
    assert_ne!(a.account, b.account);
    let late = credentials(keys, a);
    assert_eq!(
        store
            .enroll(
                &first,
                &late.upload,
                &late.wrapped,
                &late.document,
                Some(&late.recovery)
            )
            .await
            .err(),
        Some(Error::Rejected)
    );
    let enrolled = credentials(keys, b);
    // A registered account enrolls with its recovery wrapper, of the wrapper's format.
    for recovery in [None, Some(&[1_u8; 72][..]), Some(&[2_u8; 73][..])] {
        assert_eq!(
            store
                .enroll(
                    &second,
                    &enrolled.upload,
                    &enrolled.wrapped,
                    &enrolled.document,
                    recovery
                )
                .await
                .err(),
            Some(Error::Rejected)
        );
    }
    store
        .enroll(
            &second,
            &enrolled.upload,
            &enrolled.wrapped,
            &enrolled.document,
            Some(&enrolled.recovery),
        )
        .await
        .unwrap();
    assert_eq!(
        store
            .enroll(
                &second,
                &enrolled.upload,
                &enrolled.wrapped,
                &enrolled.document,
                Some(&enrolled.recovery)
            )
            .await
            .err(),
        Some(Error::Rejected)
    );
    // The wrapper is stored, the reservation's ticket is gone.
    assert_eq!(
        scalar(operator, "SELECT count(*)::bigint AS value FROM central_identity.accounts WHERE username_key = 'reserve-b'
            AND source = 'registered' AND enrollment_digest IS NULL AND octet_length(recovery_wrapped_master) = 73
            AND legacy_id IS NULL AND credential IS NOT NULL").await,
        1
    );
}

async fn names(store: &AccountStore, operator: &DatabaseConnection) {
    // A live reservation holds its name; an expired one is reclaimed.
    let first = Token::generate().unwrap();
    let a = store.register("reclaimed", None, &first).await.unwrap();
    assert_eq!(
        register(store, "reclaimed").await.err(),
        Some(Error::Conflict)
    );
    sql(operator, &expire("reclaimed")).await;
    let second = Token::generate().unwrap();
    let b = store.register("Reclaimed", None, &second).await.unwrap();
    assert_ne!(a.account, b.account);
    assert!(store.enrollment(&first).await.is_err());
    assert_eq!(store.enrollment(&second).await.unwrap().account, b.account);
    // A legacy migration takes a name back from an abandoned registration only,
    // and a registration never takes a legacy name.
    let ticket = Token::generate().unwrap();
    store.register("legacy-name", None, &ticket).await.unwrap();
    assert_eq!(
        store
            .reserve_legacy(
                "legacy-x",
                "legacy-name",
                None,
                None,
                &Token::generate().unwrap()
            )
            .await
            .err(),
        Some(Error::Conflict)
    );
    sql(operator, &expire("legacy-name")).await;
    store
        .reserve_legacy(
            "legacy-x",
            "legacy-name",
            None,
            None,
            &Token::generate().unwrap(),
        )
        .await
        .unwrap();
    assert!(store.enrollment(&ticket).await.is_err());
    sql(operator, &expire("legacy-name")).await;
    assert_eq!(
        register(store, "legacy-name").await.err(),
        Some(Error::Conflict)
    );
}

async fn races(store: &AccountStore) {
    // Two registrations for one name: exactly one gets it.
    let (a, b) = tokio::join!(register(store, "race-name"), register(store, "race-name"));
    assert!(
        matches!(
            (&a, &b),
            (Ok(_), Err(Error::Conflict)) | (Err(Error::Conflict), Ok(_))
        ),
        "{:?} {:?}",
        a.is_ok(),
        b.is_ok()
    );
}

async fn sweep(store: &AccountStore, operator: &DatabaseConnection) {
    sql(operator, "INSERT INTO central_identity.accounts (source, username_key, enrollment_digest, enrollment_expires)
        SELECT 'registered', 'sweep-' || g, decode(md5(g::text) || md5(g::text), 'hex'),
        clock_timestamp() - interval '1 minute'
        FROM generate_series(1, 20) g").await;
    let abandoned = "SELECT count(*)::bigint AS value FROM central_identity.accounts
        WHERE source = 'registered' AND credential IS NULL AND enrollment_expires <= clock_timestamp()";
    let before = scalar(operator, abandoned).await;
    assert!(before >= 20);
    register(store, "sweeper").await.unwrap();
    // Sixteen abandoned registrations go with each registration, oldest first.
    assert_eq!(scalar(operator, abandoned).await, before - 16);
    assert_eq!(
        scalar(operator, "SELECT count(*)::bigint AS value FROM central_identity.accounts WHERE username_key = 'sweeper'").await,
        1
    );
}
