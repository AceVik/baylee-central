use super::{AccountStore, Error, ServerKeys, enroll, login};
use sea_orm::{ConnectionTrait, DatabaseConnection};

pub(super) async fn run(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let identity = enroll(store, keys, "objects-a", "objectalice").await;
    let other = enroll(store, keys, "objects-b", "objectbob").await;
    let alice = login(store, keys, identity).await;
    let bob = login(store, keys, other).await;
    assert!(store.create_object(&alice).await.is_err());
    store.confirm_restore(&alice, 1).await.unwrap();
    store.close_legacy("objects-a", identity).await.unwrap();
    store.confirm_restore(&bob, 1).await.unwrap();
    store.close_legacy("objects-b", other).await.unwrap();
    let object = store.create_object(&alice).await.unwrap();
    assert_eq!(object.id[6] >> 4, 7);
    assert_eq!(object.id[8] >> 6, 2);
    assert_eq!(
        store.read_object(&alice, object.id).await.unwrap(),
        (object, None)
    );
    let ciphertext = vec![1; 64];
    assert!(
        store
            .write_object(&bob, object.id, 0, &ciphertext)
            .await
            .is_err()
    );
    assert!(store.read_object(&bob, object.id).await.is_err());
    assert_eq!(store.list_objects(&bob, None).await.unwrap(), []);
    assert!(
        store
            .write_object(&alice, object.id, 0, &[1; 40])
            .await
            .is_err()
    );
    let (one, two) = tokio::join!(
        store.write_object(&alice, object.id, 0, &ciphertext),
        store.write_object(&alice, object.id, 0, &ciphertext)
    );
    assert!(matches!(
        (one, two),
        (Ok(1), Err(Error::Conflict)) | (Err(Error::Conflict), Ok(1))
    ));
    quota_and_tombstone(store, operator, &alice, object.id).await;
    let replacement = login(store, keys, identity).await;
    assert!(store.create_object(&alice).await.is_err());
    assert!(store.read_object(&alice, object.id).await.is_err());
    assert!(store.list_objects(&alice, None).await.is_err());
    assert!(
        store
            .write_object(&alice, object.id, 2, &ciphertext)
            .await
            .is_err()
    );
    assert!(store.delete_object(&alice, object.id, 2).await.is_err());
    assert_eq!(
        store.list_objects(&replacement, None).await.unwrap().len(),
        2
    );
    reclamation(store, operator, &replacement, object.id).await;
}

async fn reclamation(
    store: &AccountStore,
    operator: &DatabaseConnection,
    session: &central_api::Token,
    deleted: [u8; 16],
) {
    operator.execute_unprepared("INSERT INTO central_identity.private_objects(account_id,revision,deleted) SELECT a.id,1,true FROM central_identity.accounts a CROSS JOIN generate_series(1,16382) WHERE a.legacy_id = 'objects-a'").await.unwrap();
    let live = store.create_object(session).await.unwrap();
    assert_ne!(live.id, deleted);
    assert!(
        store.read_object(session, deleted).await.is_err(),
        "forgotten tombstones remain unreadable"
    );
    operator.execute_unprepared("INSERT INTO central_identity.private_objects(account_id) SELECT a.id FROM central_identity.accounts a CROSS JOIN generate_series(1,999) WHERE a.legacy_id = 'objects-a'").await.unwrap();
    // Reclamation removes only retired IDs, never existing live reservations.
    store.create_object(session).await.unwrap();
    assert_eq!(store.read_object(session, live.id).await.unwrap().0, live);
}

async fn quota_and_tombstone(
    store: &AccountStore,
    operator: &DatabaseConnection,
    session: &central_api::Token,
    id: [u8; 16],
) {
    let count = super::super::scalar(
        operator,
        "SELECT object_bytes AS value FROM central_identity.accounts WHERE legacy_id = 'objects-a'",
    )
    .await;
    assert_eq!(count, 64, "a losing CAS must not charge quota twice");
    operator.execute_unprepared("UPDATE central_identity.accounts SET object_bytes = 536870912 WHERE legacy_id = 'objects-a'").await.unwrap();
    assert_eq!(
        store.write_object(session, id, 1, &[1; 65]).await,
        Err(Error::Busy)
    );
    assert_eq!(store.read_object(session, id).await.unwrap().0.revision, 1);
    operator
        .execute_unprepared(
            "UPDATE central_identity.accounts SET object_bytes = 64 WHERE legacy_id = 'objects-a'",
        )
        .await
        .unwrap();
    assert_eq!(store.delete_object(session, id, 1).await.unwrap(), 2);
    assert!(store.delete_object(session, id, 2).await.is_err());
    assert!(store.write_object(session, id, 2, &[1; 64]).await.is_err());
    let (state, content) = store.read_object(session, id).await.unwrap();
    assert!(state.deleted && content.is_none());
    assert_eq!(super::super::scalar(operator, "SELECT object_bytes AS value FROM central_identity.accounts WHERE legacy_id = 'objects-a'").await, 0);
    let reservation = store.create_object(session).await.unwrap();
    assert_eq!(
        store
            .delete_object(session, reservation.id, 0)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        store
            .list_objects(session, Some(reservation.id))
            .await
            .unwrap(),
        []
    );
}
