use super::{AccountStore, Credential, ServerKeys, TicketPurpose, Token, context, enroll, login};
use central_crypto::opaque::Login;

pub(super) async fn run(store: &AccountStore, keys: &ServerKeys) {
    let identity = enroll(store, keys, "single-session", "single-session").await;
    let first = login(store, keys, identity).await;
    store.confirm_restore(&first, 1).await.unwrap();
    let ticket = Token::generate().unwrap();
    store
        .gateway_ticket(&first, "official", &ticket, TicketPurpose::Admit)
        .await
        .unwrap();
    let lease = Token::generate().unwrap();
    store
        .redeem_lease(
            &ticket,
            &["official"],
            TicketPurpose::Admit,
            None,
            Some(&lease),
        )
        .await
        .unwrap();
    let outstanding = Token::generate().unwrap();
    store
        .gateway_ticket(&first, "official", &outstanding, TicketPurpose::Admit)
        .await
        .unwrap();

    // Failed proof and stale credential revision never reach a successful replacement.
    let credential = store.credential(identity).await.unwrap().unwrap();
    let record = ServerKeys::enrollment_record(&credential.record).unwrap();
    let (wrong, request) =
        Login::start("incorrect synthetic password".into(), context(identity)).unwrap();
    let (_, response) = keys
        .start_login(context(identity), Some(&record), &request)
        .unwrap();
    assert!(wrong.finish(&response).is_err());
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
    store.document(&first).await.unwrap();
    store
        .gateway_session(&lease, &["official"], None, TicketPurpose::Admit)
        .await
        .unwrap();

    let second = login(store, keys, identity).await;
    assert!(store.document(&first).await.is_err());
    assert!(
        store
            .gateway_session(&lease, &["official"], None, TicketPurpose::Admit)
            .await
            .is_err()
    );
    assert!(
        store
            .redeem_lease(
                &outstanding,
                &["official"],
                TicketPurpose::Admit,
                None,
                Some(&Token::generate().unwrap())
            )
            .await
            .is_err()
    );
    store.document(&second).await.unwrap();

    // Same-account concurrent successful proofs serialize on the account row.
    let credential = store.credential(identity).await.unwrap().unwrap();
    let a = Token::generate().unwrap();
    let b = Token::generate().unwrap();
    let (first_issue, second_issue) = tokio::join!(
        store.issue_session(&credential, &a),
        store.issue_session(&credential, &b)
    );
    first_issue.unwrap();
    second_issue.unwrap();
    assert_ne!(
        store.document(&a).await.is_ok(),
        store.document(&b).await.is_ok()
    );
    assert!(store.document(&second).await.is_err());
}
