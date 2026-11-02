//! Real PostgreSQL: isolation, crossed requests, blocks and session separation.
use super::{AccountStore, Error, ServerKeys, Token, enroll, login};
use central_api::social::{
    Contact, ContactAction as Action, Relation, SocialRequest as Request,
    SocialResponse as Response,
};
use central_postgres::Session;
use sea_orm::{ConnectionTrait, DatabaseConnection};

async fn list(store: &AccountStore, token: &Token) -> Vec<Contact> {
    match store
        .social(Session::Native(token), Request::Contacts)
        .await
        .unwrap()
    {
        Response::Contacts { contacts } => contacts,
        _ => panic!("contacts response required"),
    }
}

async fn change(
    store: &AccountStore,
    token: &Token,
    tag: u32,
    change: Action,
) -> Result<(), Error> {
    match store
        .social(Session::Native(token), Request::Contact { tag, change })
        .await?
    {
        Response::Done => Ok(()),
        _ => panic!("mutation response required"),
    }
}

pub(super) async fn run(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let a = enroll(store, keys, "social-a", "socialalice").await;
    let b = enroll(store, keys, "social-b", "socialbob").await;
    let c = enroll(store, keys, "social-c", "socialcarol").await;
    // Synthetic public profiles; tags do not reveal the private login names.
    for (legacy, name, tag) in [
        ("social-a", "Public_Alice", 2_147_483_601),
        ("social-b", "Public_Bob", 2_147_483_602),
        ("social-c", "Public_Carol", 2_147_483_603),
    ] {
        store.legacy_profile(legacy, Some(name), tag).await.unwrap();
    }
    let alice = login(store, keys, a).await;
    let bob = login(store, keys, b).await;
    let carol = login(store, keys, c).await;
    let (at, bt, ct) = (2_147_483_601, 2_147_483_602, 2_147_483_603);
    assert_eq!(list(store, &alice).await, []);
    assert!(change(store, &alice, at, Action::Request).await.is_err());
    assert!(change(store, &alice, bt, Action::Accept).await.is_err());
    change(store, &alice, bt, Action::Request).await.unwrap();
    change(store, &alice, bt, Action::Request).await.unwrap();
    assert_eq!(list(store, &alice).await[0].relation, Relation::Outgoing);
    assert_eq!(list(store, &bob).await[0].relation, Relation::Incoming);
    assert_eq!(list(store, &carol).await, []);
    assert!(change(store, &alice, bt, Action::Accept).await.is_err());
    assert!(change(store, &carol, bt, Action::Accept).await.is_err());
    change(store, &bob, at, Action::Accept).await.unwrap();
    assert_eq!(list(store, &alice).await[0].relation, Relation::Friend);
    change(store, &carol, bt, Action::Remove).await.unwrap();
    assert_eq!(list(store, &bob).await.len(), 1);

    // A block removes the relationship atomically and is visible only to its owner.
    change(store, &bob, at, Action::Block).await.unwrap();
    assert_eq!(list(store, &alice).await, []);
    assert_eq!(list(store, &bob).await[0].relation, Relation::Blocked);
    assert!(change(store, &alice, bt, Action::Request).await.is_err());
    change(store, &alice, bt, Action::Unblock).await.unwrap();
    assert!(change(store, &alice, bt, Action::Request).await.is_err());
    change(store, &bob, at, Action::Unblock).await.unwrap();
    assert_eq!(list(store, &alice).await, []);
    assert_eq!(list(store, &bob).await, []);

    // Opposing simultaneous requests become exactly one friendship, without deadlock.
    let (left, right) = tokio::join!(
        change(store, &alice, bt, Action::Request),
        change(store, &bob, at, Action::Request)
    );
    left.unwrap();
    right.unwrap();
    assert_eq!(list(store, &alice).await.len(), 1);
    assert_eq!(list(store, &alice).await[0].relation, Relation::Friend);
    // A simultaneous block always wins over a new request.
    change(store, &alice, bt, Action::Remove).await.unwrap();
    let (_, blocked) = tokio::join!(
        change(store, &alice, bt, Action::Request),
        change(store, &bob, at, Action::Block)
    );
    blocked.unwrap();
    assert_eq!(list(store, &alice).await, []);
    assert_eq!(list(store, &bob).await[0].relation, Relation::Blocked);

    sessions_and_budget(store, operator, a, alice, bt, ct).await;
}

async fn sessions_and_budget(
    store: &AccountStore,
    operator: &DatabaseConnection,
    a: super::Identity,
    alice: Token,
    bt: u32,
    ct: u32,
) {
    // Browser and native credentials cannot cross surfaces; the same account shares contacts.
    let browser = Token::generate().unwrap();
    store
        .issue_session(
            &store.credential(a).await.unwrap().unwrap(),
            Session::Browser(&browser),
        )
        .await
        .unwrap();
    assert!(
        store
            .social(Session::Browser(&alice), Request::Contacts)
            .await
            .is_err()
    );
    assert!(
        store
            .social(Session::Native(&browser), Request::Contacts)
            .await
            .is_err()
    );
    store
        .social(
            Session::Browser(&browser),
            Request::Contact {
                tag: ct,
                change: Action::Request,
            },
        )
        .await
        .unwrap();
    assert_eq!(list(store, &alice).await[0].person.tag, ct);
    let raw = central_api::encode(
        &store
            .social(Session::Browser(&browser), Request::Lookup { tag: bt })
            .await
            .unwrap(),
    )
    .unwrap();
    let text = String::from_utf8(raw).unwrap();
    assert!(text.contains("Public_Bob"));
    assert!(!text.contains("socialbob") && !text.contains("account_id"));

    // Request churn cannot evade the per-account hourly budget by cancelling.
    change(store, &alice, ct, Action::Remove).await.unwrap();
    operator.execute_unprepared("UPDATE central_identity.social_budgets SET requests = 60 WHERE account_id = (SELECT id FROM central_identity.accounts WHERE legacy_id = 'social-a')").await.unwrap();
    assert!(matches!(
        change(store, &alice, ct, Action::Request).await,
        Err(Error::Busy)
    ));
    assert_eq!(list(store, &alice).await, []);
    store.logout(Session::Browser(&browser)).await.unwrap();
    assert!(
        store
            .social(Session::Browser(&browser), Request::Contacts)
            .await
            .is_err()
    );
    store.logout(&alice).await.unwrap();
    assert!(
        store
            .social(Session::Native(&alice), Request::Contacts)
            .await
            .is_err()
    );
}
