use super::{
    AccountStore, ConnectionTrait, DatabaseConnection, DbBackend, Error, ServerKeys, Statement,
    Token, config, enroll, login,
};
use central_api::tower_awards::{Delivery, Disposition, Fact, Outcome, Progress};
use central_postgres::{Session, connect_dev, provision_tower_award_issuer};
use sea_orm::prelude::Uuid;

const ORIGIN: &str = "http://127.0.0.1:28790";

// Deterministic synthetic external Tower references only. Central's own rows
// still obtain their identifiers from DEFAULT uuidv7() and RETURNING.
fn remote_id() -> [u8; 16] {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let mut id = [0; 16];
    id[6] = 0x70;
    id[8..].copy_from_slice(&NEXT.fetch_add(1, Ordering::Relaxed).to_be_bytes());
    id[8] = (id[8] & 0x3f) | 0x80;
    id
}

fn delivery(account: [u8; 16]) -> Delivery {
    Delivery {
        event: remote_id(),
        account,
        fact: Fact {
            schema: 2,
            run: remote_id(),
            cohort: remote_id(),
            stage: 1,
            outcome: Outcome::Victory,
            wins: 1,
            defeats: 0,
        },
    }
}

pub(super) async fn run(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let credential = Token::generate().unwrap();
    let first = enroll(store, keys, "award-native-a", "award_alice").await;
    upgrade(operator, first.account).await;
    let issuer = provision_tower_award_issuer(operator, ORIGIN, &credential, true)
        .await
        .unwrap();
    assert_eq!(Uuid::from_bytes(issuer).get_version_num(), 7);
    let source = delivery(first.account);
    assert!(matches!(
        store.accept_tower_award(&credential, ORIGIN, &source).await,
        Err(Error::Conflict)
    ));
    let alice = login(store, keys, first).await;
    store.confirm_restore(&alice, 1).await.unwrap();
    let other = enroll(store, keys, "award-native-b", "award_bob").await;
    let bob = login(store, keys, other).await;
    store.confirm_restore(&bob, 1).await.unwrap();
    permissions(store, operator, &credential, &alice, &source).await;
    let (a, b) = tokio::join!(
        store.accept_tower_award(&credential, ORIGIN, &source),
        store.accept_tower_award(&credential, ORIGIN, &source)
    );
    assert!(a.unwrap() == b.unwrap());
    let absent_rebind = Delivery {
        account: remote_id(),
        ..source
    };
    assert!(matches!(
        store
            .accept_tower_award(&credential, ORIGIN, &absent_rebind)
            .await,
        Err(Error::Conflict)
    ));
    assert_eq!(
        store.tower_award_progress((&alice).into()).await.unwrap(),
        Progress {
            duels: 1,
            victories: 1,
            clears: 0,
            flawless: 0
        }
    );
    let rebound = Delivery {
        account: other.account,
        ..source
    };
    assert!(matches!(
        store
            .accept_tower_award(&credential, ORIGIN, &rebound)
            .await,
        Err(Error::Conflict)
    ));
    let altered = Delivery {
        fact: Fact {
            outcome: Outcome::Defeat,
            wins: 0,
            defeats: 1,
            ..source.fact
        },
        ..source
    };
    assert!(matches!(
        store
            .accept_tower_award(&credential, ORIGIN, &altered)
            .await,
        Err(Error::Conflict)
    ));
    progression(store, operator, &credential, &alice, source).await;
    pagination(store, &credential, &alice, &bob, first.account).await;
    unrestored(store, keys, &credential, &alice).await;
    deletion_race(store, operator, &credential, other.account).await;
    lifecycle(store, operator, &credential, &alice, issuer, source).await;
}

async fn upgrade(operator: &DatabaseConnection, account: [u8; 16]) {
    use sea_orm::TransactionTrait;
    let tx = operator.begin().await.unwrap();
    // Reconstruct v30 around a real enrolled account, apply the exact new SQL,
    // then rollback this probe so the production grants stay installed.
    tx.execute_unprepared(
        "DROP TABLE central_identity.tower_award_receipts,
        central_identity.tower_award_progress, central_identity.tower_award_runs,
        central_identity.tower_award_issuers",
    )
    .await
    .unwrap();
    tx.execute_unprepared(include_str!(
        "../../../../central-migration/src/m20261115_000031_tower_awards.sql"
    ))
    .await
    .unwrap();
    let row = tx.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT credential IS NOT NULL AND document_revision=1 AS preserved FROM central_identity.accounts WHERE id=$1",
        vec![Uuid::from_bytes(account).into()])).await.unwrap().unwrap();
    assert!(row.try_get::<bool>("", "preserved").unwrap());
    tx.rollback().await.unwrap();
}

async fn permissions(
    store: &AccountStore,
    operator: &DatabaseConnection,
    credential: &Token,
    session: &Token,
    source: &Delivery,
) {
    for token in [session, &Token::generate().unwrap()] {
        assert!(matches!(
            store.accept_tower_award(token, ORIGIN, source).await,
            Err(Error::Rejected)
        ));
    }
    assert!(matches!(
        store
            .accept_tower_award(credential, "http://127.0.0.1:28791", source)
            .await,
        Err(Error::Rejected)
    ));
    assert!(
        store
            .tower_award_progress(Session::Browser(session))
            .await
            .is_err()
    );
    assert!(
        store
            .export_tower_awards(Session::Native(credential), None)
            .await
            .is_err()
    );
    let runtime = connect_dev(config("central_runtime", "CENTRAL_TEST_RUNTIME_PASSWORD"))
        .await
        .unwrap();
    assert!(
        provision_tower_award_issuer(&runtime, "http://127.0.0.1:28791", credential, true)
            .await
            .is_err()
    );
    for sql in [
        "UPDATE central_identity.tower_award_issuers SET enabled=false",
        "UPDATE central_identity.tower_award_issuers SET credential_digest=decode(repeat('00',32),'hex')",
        "DELETE FROM central_identity.tower_award_issuers",
        "UPDATE central_identity.tower_award_receipts SET stage=1",
    ] {
        assert!(runtime.execute_unprepared(sql).await.is_err());
    }
    runtime.close().await.unwrap();
    operator
        .execute_unprepared(
            "GRANT UPDATE (enabled) ON central_identity.tower_award_issuers TO central_runtime",
        )
        .await
        .unwrap();
    let excessive =
        AccountStore::connect_dev(config("central_runtime", "CENTRAL_TEST_RUNTIME_PASSWORD")).await;
    operator
        .execute_unprepared(
            "REVOKE UPDATE (enabled) ON central_identity.tower_award_issuers FROM central_runtime",
        )
        .await
        .unwrap();
    assert!(excessive.is_err());
}

async fn progression(
    store: &AccountStore,
    operator: &DatabaseConnection,
    credential: &Token,
    session: &Token,
    source: Delivery,
) {
    let mut next = source;
    next.event = remote_id();
    next.fact.stage = 3;
    next.fact.wins = 3;
    assert!(matches!(
        store.accept_tower_award(credential, ORIGIN, &next).await,
        Err(Error::Conflict)
    ));
    next.fact.stage = 2;
    next.fact.wins = 2;
    operator.execute_unprepared("ALTER TABLE central_identity.tower_award_progress ADD CONSTRAINT injected_award_failure CHECK (duels<2) NOT VALID").await.unwrap();
    assert!(matches!(
        store.accept_tower_award(credential, ORIGIN, &next).await,
        Err(Error::Unavailable)
    ));
    assert_eq!(
        store
            .tower_award_progress(session.into())
            .await
            .unwrap()
            .duels,
        1
    );
    operator.execute_unprepared("ALTER TABLE central_identity.tower_award_progress DROP CONSTRAINT injected_award_failure").await.unwrap();
    for stage in 2..=8 {
        next.event = remote_id();
        next.fact.stage = stage;
        next.fact.wins = stage;
        let receipt = store
            .accept_tower_award(credential, ORIGIN, &next)
            .await
            .unwrap();
        assert_eq!(receipt.disposition, Disposition::Recorded);
        assert_eq!(receipt.event, next.event);
        assert_eq!(receipt.account, source.account);
        assert_eq!(receipt.run, source.fact.run);
        assert_eq!(receipt.stage, stage);
    }
    assert_eq!(
        store.tower_award_progress(session.into()).await.unwrap(),
        Progress {
            duels: 8,
            victories: 8,
            clears: 1,
            flawless: 1
        }
    );
    next.event = remote_id();
    assert!(matches!(
        store.accept_tower_award(credential, ORIGIN, &next).await,
        Err(Error::Conflict)
    ));
    let absent = delivery(remote_id());
    assert_eq!(
        store
            .accept_tower_award(credential, ORIGIN, &absent)
            .await
            .unwrap()
            .disposition,
        Disposition::AccountGone
    );
}

async fn pagination(
    store: &AccountStore,
    credential: &Token,
    alice: &Token,
    bob: &Token,
    account: [u8; 16],
) {
    for _ in 0..121 {
        store
            .accept_tower_award(credential, ORIGIN, &delivery(account))
            .await
            .unwrap();
    }
    let page = store.export_tower_awards(alice.into(), None).await.unwrap();
    assert_eq!(page.progress.duels, 129);
    assert_eq!(page.receipts.len(), 128);
    assert!(page.next.is_some());
    assert!(
        page.receipts
            .iter()
            .all(|row| row.origin == ORIGIN && row.accepted_at > 0)
    );
    let next = store
        .export_tower_awards(alice.into(), page.next)
        .await
        .unwrap();
    assert_eq!(next.receipts.len(), 1);
    assert!(next.next.is_none());
    assert!(
        page.receipts
            .iter()
            .all(|row| row.id != next.receipts[0].id)
    );
    assert!(
        store
            .export_tower_awards(bob.into(), None)
            .await
            .unwrap()
            .receipts
            .is_empty()
    );
    assert_eq!(
        store.tower_award_progress(bob.into()).await.unwrap(),
        Progress::default()
    );
    assert!(
        store
            .export_tower_awards(alice.into(), Some([0; 16]))
            .await
            .is_err()
    );
}

async fn deletion_race(
    store: &AccountStore,
    operator: &DatabaseConnection,
    credential: &Token,
    account: [u8; 16],
) {
    let source = delivery(account);
    let (accepted, deleted) = tokio::join!(
        store.accept_tower_award(credential, ORIGIN, &source),
        operator.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "DELETE FROM central_identity.accounts WHERE id=$1",
            vec![Uuid::from_bytes(account).into()]
        ))
    );
    accepted.unwrap();
    assert_eq!(deleted.unwrap().rows_affected(), 1);
    assert_eq!(
        store
            .accept_tower_award(credential, ORIGIN, &source)
            .await
            .unwrap()
            .disposition,
        Disposition::AccountGone
    );
}

async fn lifecycle(
    store: &AccountStore,
    operator: &DatabaseConnection,
    credential: &Token,
    session: &Token,
    issuer: [u8; 16],
    source: Delivery,
) {
    operator
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE central_identity.tower_award_progress SET duels=1000000 WHERE account_id=$1",
            vec![Uuid::from_bytes(source.account).into()],
        ))
        .await
        .unwrap();
    assert_eq!(
        store
            .accept_tower_award(credential, ORIGIN, &source)
            .await
            .unwrap()
            .disposition,
        Disposition::Recorded
    );
    assert!(matches!(
        store
            .accept_tower_award(credential, ORIGIN, &delivery(source.account))
            .await,
        Err(Error::Busy)
    ));
    operator
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE central_identity.tower_award_issuers SET enabled=false WHERE id=$1",
            vec![Uuid::from_bytes(issuer).into()],
        ))
        .await
        .unwrap();
    assert!(matches!(
        store.accept_tower_award(credential, ORIGIN, &source).await,
        Err(Error::Rejected)
    ));
    operator
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE central_identity.tower_award_issuers SET enabled=true WHERE id=$1",
            vec![Uuid::from_bytes(issuer).into()],
        ))
        .await
        .unwrap();
    operator
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "DELETE FROM central_identity.accounts WHERE id=$1",
            vec![Uuid::from_bytes(source.account).into()],
        ))
        .await
        .unwrap();
    assert_eq!(
        store
            .accept_tower_award(credential, ORIGIN, &source)
            .await
            .unwrap()
            .disposition,
        Disposition::AccountGone
    );
    assert!(store.tower_award_progress(session.into()).await.is_err());
    let row = operator.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT (SELECT count(*) FROM central_identity.tower_award_receipts WHERE account_id=$1)
         +(SELECT count(*) FROM central_identity.tower_award_runs WHERE account_id=$1)
         +(SELECT count(*) FROM central_identity.tower_award_progress WHERE account_id=$1) AS remaining",
        vec![Uuid::from_bytes(source.account).into()])).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "remaining").unwrap(), 0);
}

/// An account the game client never restored (one that has only used the
/// website) reads its own zero totals and empty history on both surfaces,
/// so its download completes; nobody else's receipts reach it, and a
/// delivery to it still waits for the restore.
async fn unrestored(store: &AccountStore, keys: &ServerKeys, credential: &Token, alice: &Token) {
    let identity = enroll(store, keys, "award-web-c", "award_carol").await;
    let native = login(store, keys, identity).await;
    let cookie = Token::generate().unwrap();
    let enrolled = store.credential(identity).await.unwrap().unwrap();
    store
        .issue_session(&enrolled, Session::Browser(&cookie))
        .await
        .unwrap();
    let nothing = Progress {
        duels: 0,
        victories: 0,
        clears: 0,
        flawless: 0,
    };
    for session in [Session::Native(&native), Session::Browser(&cookie)] {
        assert_eq!(store.tower_award_progress(session).await.unwrap(), nothing);
        let export = store.export_tower_awards(session, None).await.unwrap();
        assert_eq!(export.progress, nothing);
        assert!(export.receipts.is_empty() && export.next.is_none());
    }
    // Alice's history is hers alone, read by her own session as before.
    let theirs = store.export_tower_awards(alice.into(), None).await.unwrap();
    assert!(!theirs.receipts.is_empty());
    // The surfaces stay apart: the cookie is no native bearer, and the
    // native bearer no cookie.
    assert!(
        store
            .tower_award_progress(Session::Native(&cookie))
            .await
            .is_err()
    );
    assert!(
        store
            .export_tower_awards(Session::Browser(&native), None)
            .await
            .is_err()
    );
    // Writing still needs the restore: the delivery is kept for later.
    assert!(matches!(
        store
            .accept_tower_award(credential, ORIGIN, &delivery(identity.account))
            .await,
        Err(Error::Conflict)
    ));
    let later = Token::generate().unwrap();
    assert!(
        store
            .tower_award_progress(Session::Native(&later))
            .await
            .is_err()
    );
}
