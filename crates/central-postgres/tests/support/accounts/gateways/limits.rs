use super::{AccountStore, DatabaseConnection, Error, TicketPurpose, Token, metadata};
use sea_orm::ConnectionTrait;

pub(super) async fn run(
    store: &AccountStore,
    operator: &DatabaseConnection,
    session: &Token,
    gateway: [u8; 16],
    origin: &str,
) {
    for index in 1..=8 {
        let candidate = format!("https://quota-{index}.invalid");
        let result = store
            .register_gateway(
                session,
                &candidate,
                &metadata(&candidate),
                &Token::generate().unwrap(),
                &Token::generate().unwrap(),
                false,
            )
            .await;
        if index < 8 {
            result.unwrap();
        } else {
            assert!(matches!(result, Err(Error::Busy)));
        }
    }
    // A full quota must still allow recovery of an owned pending reservation.
    store
        .register_gateway(
            session,
            "https://quota-1.invalid",
            &metadata("https://quota-1.invalid"),
            &Token::generate().unwrap(),
            &Token::generate().unwrap(),
            false,
        )
        .await
        .unwrap();
    for index in 0..=32 {
        let ticket = Token::generate().unwrap();
        store
            .registered_ticket(session, origin, &ticket, TicketPurpose::Admit)
            .await
            .unwrap();
        let result = store
            .redeem_lease(
                &ticket,
                origin,
                TicketPurpose::Admit,
                Some(gateway),
                Some(&Token::generate().unwrap()),
            )
            .await;
        if index < 32 {
            result.unwrap();
        } else {
            assert!(matches!(result, Err(Error::Busy)));
        }
    }
    constraints(store, operator).await;
}

async fn constraints(store: &AccountStore, operator: &DatabaseConnection) {
    // A non-RFC variant yields NULL from uuid_extract_version, which CHECK (=7)
    // used to accept. The strengthened constraint must reject that exact case.
    assert!(
        operator
            .execute_unprepared(
                "INSERT INTO central_identity.accounts(id,legacy_id,username_key)
         VALUES('0194adcc-0000-7000-0000-000000000001','invalid-variant','invalid-variant')"
            )
            .await
            .is_err()
    );
    let enrollment = Token::generate().unwrap();
    store
        .reserve_legacy("unique-ticket-one", "unique-ticket-one", &enrollment)
        .await
        .unwrap();
    assert!(
        store
            .reserve_legacy("unique-ticket-two", "unique-ticket-two", &enrollment)
            .await
            .is_err()
    );
    assert!(store.enrollment(&enrollment).await.is_ok());
}
