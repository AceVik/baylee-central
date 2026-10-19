use super::{AccountStore, ServerKeys, Token, enroll, login};
use sea_orm::{ConnectionTrait, DatabaseConnection};

pub(super) async fn run(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    store.invalidate_legacy("never-enrolled").await.unwrap();
    let identity = enroll(store, keys, "restart-legacy", "restartuser").await;
    let credential = store.credential(identity).await.unwrap().unwrap();
    let session = login(store, keys, identity).await;
    store.confirm_restore(&session, 1).await.unwrap();
    store.invalidate_legacy("restart-legacy").await.unwrap();
    assert!(store.session_status(&session).await.is_err());
    assert!(store.document(&session).await.is_err());
    assert!(store.credential(identity).await.unwrap().is_none());
    assert!(
        store
            .issue_session(&credential, &Token::generate().unwrap())
            .await
            .is_err()
    );
    assert_eq!(super::super::scalar(operator, "SELECT count(*)::bigint AS value FROM central_identity.migration_archives ar JOIN central_identity.accounts a ON a.id = ar.account_id WHERE a.legacy_id = 'restart-legacy' AND ar.vault_epoch + 1 = a.vault_epoch AND octet_length(ar.document) > 41").await, 1);
    let next = enroll(store, keys, "restart-legacy", "renameduser").await;
    assert_eq!(next.account, identity.account);
    assert_eq!(next.epoch, identity.epoch + 1);
    let current = login(store, keys, next).await;
    assert!(store.close_legacy("restart-legacy", next).await.is_err());
    store.confirm_restore(&current, 1).await.unwrap();
    assert!(
        store
            .close_legacy("restart-legacy", identity)
            .await
            .is_err()
    );
    store.close_legacy("restart-legacy", next).await.unwrap();
    store.close_legacy("restart-legacy", next).await.unwrap();
    assert!(store.invalidate_legacy("restart-legacy").await.is_err());
    store.session_status(&current).await.unwrap();
    // Runtime permissions cannot mutate archives or approve a gateway it registered.
    for grant in [
        "UPDATE ON central_identity.migration_archives",
        "INSERT ON central_identity.gateway_approvals",
        "SELECT ON central_identity.private_objects WITH GRANT OPTION",
    ] {
        // GRANT OPTION uses a different SQL placement; keep identifiers fixed in this fixture.
        let sql = if grant.ends_with("WITH GRANT OPTION") {
            "GRANT SELECT ON central_identity.private_objects TO central_runtime WITH GRANT OPTION"
                .to_owned()
        } else {
            format!("GRANT {grant} TO central_runtime")
        };
        operator.execute_unprepared(&sql).await.unwrap();
        assert!(
            AccountStore::connect_dev(super::config(
                "central_runtime",
                "CENTRAL_TEST_RUNTIME_PASSWORD"
            ))
            .await
            .is_err()
        );
        let sql = if grant.ends_with("WITH GRANT OPTION") {
            "REVOKE GRANT OPTION FOR SELECT ON central_identity.private_objects FROM central_runtime".to_owned()
        } else {
            format!("REVOKE {grant} FROM central_runtime")
        };
        operator.execute_unprepared(&sql).await.unwrap();
    }
}
