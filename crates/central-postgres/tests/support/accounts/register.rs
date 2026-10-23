//! `register` and the registered enrollment: the key before the name,
//! reservations that give their use back, reclaimed names and races.
use super::{AccountStore, Error, Identity, ServerKeys, Token, context};
use central_api::beta_key::BetaKey;
use central_crypto::{
    opaque::Enrollment,
    vault::{MasterKey, ObjectContext, RecoveryCode},
};
use central_postgres::BetaKeyBatch;
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

pub(super) async fn issue(operator: &DatabaseConnection, uses: u32, note: &str) -> BetaKey {
    let key = BetaKey::generate().unwrap();
    BetaKeyBatch::new(&[key.digest()], uses, None, note)
        .unwrap()
        .issue(operator)
        .await
        .unwrap();
    key
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

async fn register(store: &AccountStore, name: &str, key: &BetaKey) -> Result<Identity, Error> {
    store
        .register(name, None, &key.digest(), &Token::generate().unwrap())
        .await
}

pub(super) async fn run(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    refusals(store, operator).await;
    reservations(store, operator, keys).await;
    revocation(store, operator, keys).await;
    names(store, operator).await;
    races(store, operator).await;
    sweep(store, operator).await;
}

async fn refusals(store: &AccountStore, operator: &DatabaseConnection) {
    let good = issue(operator, 10, "refusals").await;
    let unknown = BetaKey::generate().unwrap();
    let revoked = issue(operator, 10, "revoked").await;
    sql(operator, "UPDATE central_identity.beta_keys SET revoked_at = clock_timestamp() WHERE note = 'revoked'").await;
    let expired = issue(operator, 10, "expired").await;
    sql(
        operator,
        "UPDATE central_identity.beta_keys SET created_at = clock_timestamp() - interval '2 days',
        expires_at = clock_timestamp() - interval '1 day' WHERE note = 'expired'",
    )
    .await;
    let spent = issue(operator, 1, "spent").await;
    sql(
        operator,
        "UPDATE central_identity.beta_keys SET uses_left = 0 WHERE note = 'spent'",
    )
    .await;
    for key in [&unknown, &revoked, &expired, &spent] {
        // Refused before the name is looked at: a taken name answers the same.
        assert_eq!(
            register(store, "alice", key).await.err(),
            Some(Error::Rejected)
        );
        assert_eq!(
            register(store, "free-name", key).await.err(),
            Some(Error::Rejected)
        );
    }
    // A valid key learns that a name is taken: enrolled, or a legacy migration's.
    assert_eq!(
        register(store, "ALICE", &good).await.err(),
        Some(Error::Conflict)
    );
    assert_eq!(
        register(store, "expired", &good).await.err(),
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
            register(store, name, &good).await.err(),
            Some(Error::Rejected),
            "{name}"
        );
    }
    assert_eq!(
        scalar(
            operator,
            "SELECT count(*)::bigint AS value FROM central_identity.accounts a
            JOIN central_identity.beta_keys k ON k.id = a.beta_key_id WHERE k.note = 'refusals'"
        )
        .await,
        0,
        "no refusal reserved anything"
    );
}

async fn reservations(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let key = issue(operator, 1, "one-use").await;
    let first = Token::generate().unwrap();
    let a = store
        .register("reserve-a", None, &key.digest(), &first)
        .await
        .unwrap();
    // The key's only use is reserved.
    assert_eq!(
        register(store, "reserve-b", &key).await.err(),
        Some(Error::Rejected)
    );
    // The reservation expires and gives the use back; its ticket no longer enrolls.
    sql(operator, &expire("reserve-a")).await;
    let second = Token::generate().unwrap();
    let b = store
        .register("reserve-b", None, &key.digest(), &second)
        .await
        .unwrap();
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
    // The use is spent, the link is gone, the wrapper is stored.
    assert_eq!(scalar(operator, "SELECT uses_left::bigint AS value FROM central_identity.beta_keys WHERE note = 'one-use'").await, 0);
    assert_eq!(
        scalar(operator, "SELECT count(*)::bigint AS value FROM central_identity.accounts WHERE username_key = 'reserve-b'
            AND source = 'registered' AND beta_key_id IS NULL AND octet_length(recovery_wrapped_master) = 73
            AND legacy_id IS NULL AND credential IS NOT NULL").await,
        1
    );
    assert_eq!(
        register(store, "reserve-c", &key).await.err(),
        Some(Error::Rejected)
    );
}

async fn revocation(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    // Revoking a key ends its pending registrations, and spends nothing.
    let revocable = issue(operator, 5, "revoke-later").await;
    let ticket = Token::generate().unwrap();
    let c = store
        .register("reserve-c", None, &revocable.digest(), &ticket)
        .await
        .unwrap();
    sql(operator, "UPDATE central_identity.beta_keys SET revoked_at = clock_timestamp() WHERE note = 'revoke-later'").await;
    let enrolled = credentials(keys, c);
    assert_eq!(
        store
            .enroll(
                &ticket,
                &enrolled.upload,
                &enrolled.wrapped,
                &enrolled.document,
                Some(&enrolled.recovery)
            )
            .await
            .err(),
        Some(Error::Rejected)
    );
    assert_eq!(scalar(operator, "SELECT uses_left::bigint AS value FROM central_identity.beta_keys WHERE note = 'revoke-later'").await, 5);
}

async fn names(store: &AccountStore, operator: &DatabaseConnection) {
    let key = issue(operator, 10, "names").await;
    // A live reservation holds its name; an expired one is reclaimed.
    let first = Token::generate().unwrap();
    let a = store
        .register("reclaimed", None, &key.digest(), &first)
        .await
        .unwrap();
    assert_eq!(
        register(store, "reclaimed", &key).await.err(),
        Some(Error::Conflict)
    );
    sql(operator, &expire("reclaimed")).await;
    let second = Token::generate().unwrap();
    let b = store
        .register("Reclaimed", None, &key.digest(), &second)
        .await
        .unwrap();
    assert_ne!(a.account, b.account);
    assert!(store.enrollment(&first).await.is_err());
    assert_eq!(store.enrollment(&second).await.unwrap().account, b.account);
    // A legacy migration takes a name back from an abandoned registration only,
    // and a registration never takes a legacy name.
    let ticket = Token::generate().unwrap();
    store
        .register("legacy-name", None, &key.digest(), &ticket)
        .await
        .unwrap();
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
        register(store, "legacy-name", &key).await.err(),
        Some(Error::Conflict)
    );
}

async fn races(store: &AccountStore, operator: &DatabaseConnection) {
    // Two registrations for a key's last use: exactly one gets it.
    let key = issue(operator, 1, "race").await;
    let (a, b) = tokio::join!(
        register(store, "race-a", &key),
        register(store, "race-b", &key)
    );
    assert!(
        matches!(
            (&a, &b),
            (Ok(_), Err(Error::Rejected)) | (Err(Error::Rejected), Ok(_))
        ),
        "{:?} {:?}",
        a.is_ok(),
        b.is_ok()
    );
    // Two registrations for one name: exactly one gets it.
    let key = issue(operator, 5, "race-name").await;
    let (a, b) = tokio::join!(
        register(store, "race-name", &key),
        register(store, "race-name", &key)
    );
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
    let key = issue(operator, 100, "sweep").await;
    sql(operator, "INSERT INTO central_identity.accounts (source, username_key, enrollment_digest, enrollment_expires, beta_key_id)
        SELECT 'registered', 'sweep-' || g, decode(md5(g::text) || md5(g::text), 'hex'),
        clock_timestamp() - interval '1 minute', (SELECT id FROM central_identity.beta_keys WHERE note = 'sweep')
        FROM generate_series(1, 20) g").await;
    let abandoned = "SELECT count(*)::bigint AS value FROM central_identity.accounts
        WHERE source = 'registered' AND credential IS NULL AND enrollment_expires <= clock_timestamp()";
    let before = scalar(operator, abandoned).await;
    assert!(before >= 20);
    register(store, "sweeper", &key).await.unwrap();
    // Sixteen abandoned registrations go with each registration, oldest first.
    assert_eq!(scalar(operator, abandoned).await, before - 16);
    assert_eq!(
        scalar(operator, "SELECT count(*)::bigint AS value FROM central_identity.accounts WHERE username_key = 'sweeper'").await,
        1
    );
}
