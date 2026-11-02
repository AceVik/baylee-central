//! Real database checks for consent, ownership, trigger delivery and lease races.
use super::{AccountStore, Error, ServerKeys, Token, enroll};
use central_api::notifications::{Category, NotificationRequest as Request, Subscription};
use central_api::social::{ContactAction, SocialRequest};
use central_postgres::{PushResult, Session};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, Value, prelude::Uuid};

async fn sql(db: &DatabaseConnection, text: &str, values: Vec<Value>) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        text,
        values,
    ))
    .await
    .unwrap();
}
async fn count(db: &DatabaseConnection, table: &str) -> i64 {
    // Only the two fixed table names in this test are callers.
    db.query_one_raw(Statement::from_string(
        DbBackend::Postgres,
        format!("SELECT count(*) AS n FROM central_identity.{table}"),
    ))
    .await
    .unwrap()
    .unwrap()
    .try_get("", "n")
    .unwrap()
}
fn subscription(label: &str) -> Subscription {
    Subscription {
        endpoint: format!("https://fcm.googleapis.com/fcm/send/synthetic-{label}"),
        p256dh: "A".repeat(87),
        auth: "A".repeat(22),
        lang: "en".into(),
        nonce: "0123456789abcdef0123456789abcdef".into(),
    }
}
async fn browser(store: &AccountStore, identity: super::Identity) -> Token {
    let token = Token::generate().unwrap();
    store
        .issue_session(
            &store.credential(identity).await.unwrap().unwrap(),
            Session::Browser(&token),
        )
        .await
        .unwrap();
    token
}
async fn subscribe(store: &AccountStore, token: &Token, label: &str) {
    assert!(
        store
            .notifications(
                token,
                Request::Subscribe {
                    subscription: subscription(label)
                }
            )
            .await
            .unwrap()
            .subscribed
    );
}
async fn queue(db: &DatabaseConnection, who: Uuid, category: Category) {
    sql(
        db,
        "SELECT central_identity.queue_push($1,$2::smallint)",
        vec![who.into(), category.bit().into()],
    )
    .await;
    due(db).await;
}
async fn due(db: &DatabaseConnection) {
    sql(
        db,
        "UPDATE central_identity.push_jobs SET due_at=clock_timestamp()-interval '1 second'",
        vec![],
    )
    .await;
}

pub(super) async fn run(store: &AccountStore, db: &DatabaseConnection, keys: &ServerKeys) {
    let a = enroll(store, keys, "push-a", "pushalice").await;
    let b = enroll(store, keys, "push-b", "pushbob").await;
    let alice = browser(store, a).await;
    let bob = browser(store, b).await;
    let me = Uuid::from_bytes(a.account);
    let read = || Request::Read { endpoint: None };
    assert_eq!(
        store.notifications(&alice, read()).await.unwrap().enabled,
        7
    );
    assert!(
        !store
            .notifications(&alice, read())
            .await
            .unwrap()
            .subscribed
    );
    assert!(
        store
            .notifications(&Token::generate().unwrap(), read())
            .await
            .is_err()
    );
    let native = super::login(store, keys, b).await;
    assert!(store.notifications(&native, read()).await.is_err());
    assert!(matches!(
        store
            .notifications(
                &alice,
                Request::Set {
                    category: Category::Feedback,
                    enabled: true
                }
            )
            .await,
        Err(Error::Rejected)
    ));
    queue(db, me, Category::Chat).await;
    assert!(
        store.claim_push().await.unwrap().is_none(),
        "no implicit opt-in"
    );
    subscribe(store, &alice, "alice").await;
    assert!(matches!(
        store
            .notifications(
                &bob,
                Request::Subscribe {
                    subscription: subscription("alice")
                }
            )
            .await,
        Err(Error::Conflict)
    ));
    store
        .notifications(
            &bob,
            Request::Unsubscribe {
                endpoint: subscription("alice").endpoint.clone(),
            },
        )
        .await
        .unwrap();
    assert_eq!(
        count(db, "push_subscriptions").await,
        1,
        "another account cannot remove a capability"
    );
    contact_trigger(store, db, &bob).await;
    leases(store, db, me).await;
    consent(store, db, &alice, &bob, me).await;
    limits_and_logout(store, db, &alice, &bob, a).await;
}

async fn contact_trigger(store: &AccountStore, db: &DatabaseConnection, bob: &Token) {
    store
        .legacy_profile("push-a", Some("PushAlice"), 2_147_483_580)
        .await
        .unwrap();
    store
        .legacy_profile("push-b", Some("PushBob"), 2_147_483_581)
        .await
        .unwrap();
    store
        .social(
            Session::Browser(bob),
            SocialRequest::Contact {
                tag: 2_147_483_580,
                change: ContactAction::Request,
            },
        )
        .await
        .unwrap();
    due(db).await;
    let job = store.claim_push().await.unwrap().unwrap();
    assert_eq!(
        job.category,
        Category::Contacts,
        "real contact insert triggers the recipient only"
    );
    store
        .finish_push(&job, PushResult::Delivered)
        .await
        .unwrap();
    assert!(store.claim_push().await.unwrap().is_none());
}

async fn leases(store: &AccountStore, db: &DatabaseConnection, me: Uuid) {
    queue(db, me, Category::Chat).await;
    let (a, b) = tokio::join!(store.claim_push(), store.claim_push());
    let mut jobs: Vec<_> = [a.unwrap(), b.unwrap()].into_iter().flatten().collect();
    assert_eq!(
        jobs.len(),
        1,
        "concurrent workers cannot claim the same job"
    );
    let first = jobs.pop().unwrap();
    queue(db, me, Category::Chat).await;
    store
        .finish_push(&first, PushResult::Delivered)
        .await
        .unwrap();
    let second = store.claim_push().await.unwrap().unwrap();
    assert!(
        second.revision > first.revision,
        "an event arriving during delivery survives"
    );
    assert_ne!(second.lease, first.lease);
    store.finish_push(&first, PushResult::Retry).await.unwrap();
    assert!(
        store.claim_push().await.unwrap().is_none(),
        "old completion cannot release another worker's lease"
    );
    sql(
        db,
        "UPDATE central_identity.push_jobs SET claimed_until=clock_timestamp()-interval '1 second'",
        vec![],
    )
    .await;
    let third = store.claim_push().await.unwrap().unwrap();
    assert_eq!(third.revision, second.revision);
    assert_ne!(third.lease, second.lease);
    store
        .finish_push(&second, PushResult::Delivered)
        .await
        .unwrap();
    store.finish_push(&second, PushResult::Gone).await.unwrap();
    assert_eq!(count(db, "push_subscriptions").await, 1);
    assert_eq!(
        count(db, "push_jobs").await,
        1,
        "timed-out worker cannot acknowledge renewed lease"
    );
    store.finish_push(&third, PushResult::Retry).await.unwrap();
    assert!(
        store.claim_push().await.unwrap().is_none(),
        "retry has bounded backoff"
    );
    due(db).await;
    let fourth = store.claim_push().await.unwrap().unwrap();
    store
        .finish_push(&fourth, PushResult::Delivered)
        .await
        .unwrap();
}

async fn consent(
    store: &AccountStore,
    db: &DatabaseConnection,
    alice: &Token,
    bob: &Token,
    me: Uuid,
) {
    queue(db, me, Category::Chat).await;
    store
        .notifications(
            alice,
            Request::Set {
                category: Category::Chat,
                enabled: false,
            },
        )
        .await
        .unwrap();
    assert!(store.claim_push().await.unwrap().is_none());
    queue(db, me, Category::Chat).await;
    assert!(
        store.claim_push().await.unwrap().is_none(),
        "disabled categories do not queue"
    );
    assert_eq!(
        store
            .notifications(bob, Request::Read { endpoint: None })
            .await
            .unwrap()
            .enabled,
        7
    );
    // Independent updates preserve each other's choices.
    let (a, b) = tokio::join!(
        store.notifications(
            alice,
            Request::Set {
                category: Category::Chat,
                enabled: true
            }
        ),
        store.notifications(
            alice,
            Request::Set {
                category: Category::Contacts,
                enabled: false
            }
        )
    );
    a.unwrap();
    b.unwrap();
    assert_eq!(
        store
            .notifications(alice, Request::Read { endpoint: None })
            .await
            .unwrap()
            .enabled,
        5
    );
    sql(
        db,
        "INSERT INTO central_identity.account_roles(account_id,role) VALUES($1,'admin')",
        vec![me.into()],
    )
    .await;
    store
        .notifications(
            alice,
            Request::Set {
                category: Category::Feedback,
                enabled: true,
            },
        )
        .await
        .unwrap();
    admin_triggers(store, db, alice, me).await;
    queue(db, me, Category::Feedback).await;
    sql(
        db,
        "DELETE FROM central_identity.account_roles WHERE account_id=$1 AND role='admin'",
        vec![me.into()],
    )
    .await;
    assert!(
        store.claim_push().await.unwrap().is_none(),
        "delivery rechecks current admin rights"
    );
    assert_eq!(
        store
            .notifications(alice, Request::Read { endpoint: None })
            .await
            .unwrap()
            .enabled,
        5
    );
    release(store, db).await;
}

async fn admin_triggers(store: &AccountStore, db: &DatabaseConnection, alice: &Token, me: Uuid) {
    store
        .notifications(
            alice,
            Request::Set {
                category: Category::Gateways,
                enabled: true,
            },
        )
        .await
        .unwrap();
    sql(db, "INSERT INTO central_identity.reports(gateway_id,gateway_label,channel,reporter,kind,text) VALUES($1,'synthetic','gateway','a1','feedback','Private synthetic feedback')", vec![me.into()]).await;
    due(db).await;
    let feedback = store.claim_push().await.unwrap().unwrap();
    assert_eq!(feedback.category, Category::Feedback);
    store
        .finish_push(&feedback, PushResult::Delivered)
        .await
        .unwrap();
    sql(db, "INSERT INTO central_identity.gateways(owner_id,origin,name,version,website,credential_digest,heartbeat_digest) VALUES($1,'https://push-test.invalid','Synthetic','test','https://push-test.invalid',$2,$3)",
        vec![me.into(), Token::generate().unwrap().digest().to_vec().into(), Token::generate().unwrap().digest().to_vec().into()]).await;
    due(db).await;
    let gateway = store.claim_push().await.unwrap().unwrap();
    assert_eq!(gateway.category, Category::Gateways);
    store
        .finish_push(&gateway, PushResult::Delivered)
        .await
        .unwrap();
    assert!(store.claim_push().await.unwrap().is_none());
}

async fn release(store: &AccountStore, db: &DatabaseConnection) {
    store.announce_release("push-test-release-1").await.unwrap();
    store.announce_release("push-test-release-1").await.unwrap();
    due(db).await;
    let update = store.claim_push().await.unwrap().unwrap();
    assert_eq!(update.category, Category::Updates);
    assert_eq!(
        update.revision, 1,
        "restart does not repeat release announcement"
    );
    store
        .finish_push(&update, PushResult::Delivered)
        .await
        .unwrap();
}

async fn limits_and_logout(
    store: &AccountStore,
    db: &DatabaseConnection,
    alice: &Token,
    bob: &Token,
    a: super::Identity,
) {
    for i in 1..10 {
        subscribe(store, alice, &format!("alice-{i}")).await;
    }
    assert!(matches!(
        store
            .notifications(
                alice,
                Request::Subscribe {
                    subscription: subscription("overflow")
                }
            )
            .await,
        Err(Error::Busy)
    ));
    subscribe(store, alice, "alice").await; // renewal at the cap is permitted
    let other_browser = browser(store, a).await;
    // Rebinding this same account's capability to its new session is safe.
    subscribe(store, &other_browser, "alice").await;
    subscribe(store, bob, "bob").await;
    store.logout(Session::Browser(alice)).await.unwrap();
    assert_eq!(
        count(db, "push_subscriptions").await,
        2,
        "logout removes only installations bound to that session"
    );
    store
        .logout(Session::Browser(&other_browser))
        .await
        .unwrap();
    assert_eq!(count(db, "push_subscriptions").await, 1);
    let who = Uuid::from_bytes(store.discover("pushbob").await.unwrap().unwrap().account);
    queue(db, who, Category::Chat).await;
    sql(db, "UPDATE central_identity.accounts SET credential_revision=credential_revision+1 WHERE id=$1", vec![who.into()]).await;
    assert!(
        store.claim_push().await.unwrap().is_none(),
        "credential revocation suppresses queued deliveries"
    );
    sql(
        db,
        "DELETE FROM central_identity.push_subscriptions",
        vec![],
    )
    .await;
    assert_eq!(
        count(db, "push_jobs").await,
        0,
        "subscription removal cascades pending work"
    );
}
