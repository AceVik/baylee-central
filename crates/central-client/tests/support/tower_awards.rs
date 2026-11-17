//! Real native HTTP and SDK scope tests, using only owned synthetic identities.
#![forbid(unsafe_code)]
use super::*;
use central_api::tower_awards::{Delivery, Disposition, Fact, Outcome, Progress};
use sea_orm::{DbBackend, Statement, prelude::Uuid};

const ORIGIN: &str = "http://127.0.0.1:29871";

#[test]
#[ignore = "owned fixture: cargo xtask postgres --tower-awards-only"]
fn scoped_delivery() {
    tower_access::with_fixture(exercise);
}

fn id(tail: u8) -> [u8; 16] {
    let mut id = [0; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id[15] = tail;
    id
}

fn exercise(
    http: &Http,
    bridge: &Token,
    runtime: &Runtime,
    operator: &DatabaseConnection,
    _store: &AccountStore,
) {
    let credential = Token::generate().unwrap();
    runtime
        .block_on(central_postgres::provision_tower_award_issuer(
            operator,
            ORIGIN,
            &credential,
            true,
        ))
        .unwrap();
    let (account, session) = support::password::moved(http, bridge, "tower-awards-fixture");
    let grant = session.tower_token(http).unwrap();
    let delivery = Delivery {
        event: id(1),
        account: account.account,
        fact: Fact {
            schema: 2,
            run: id(2),
            cohort: id(3),
            stage: 1,
            outcome: Outcome::Victory,
            wins: 1,
            defeats: 0,
        },
    };
    browser_listener(http, runtime, &credential, delivery, account);
    assert_eq!(
        session.tower_award_progress(http).unwrap(),
        Progress::default()
    );
    for forbidden in [bridge, &grant.token, &Token::generate().unwrap()] {
        assert!(central_client::deliver_tower_award(http, forbidden, delivery).is_err());
    }
    assert!(!central_identity::browser_operation(&Request::TowerAward {
        delivery
    }));
    assert!(central_identity::browser_operation(
        &Request::TowerProgress {}
    ));
    assert!(central_identity::browser_operation(
        &Request::TowerAwardExport { after: None }
    ));
    let recorded = central_client::deliver_tower_award(http, &credential, delivery).unwrap();
    assert_eq!(recorded.disposition, Disposition::Recorded);
    assert!(central_client::deliver_tower_award(http, &credential, delivery).unwrap() == recorded);
    assert_eq!(session.tower_award_progress(http).unwrap().victories, 1);
    let export = session.tower_award_export(http, None).unwrap();
    assert_eq!(export.receipts.len(), 1);
    assert!(export.receipts[0].fact == delivery.fact);
    for operation in [
        Request::TowerProgress {},
        Request::TowerAwardExport { after: None },
        Request::TowerIdentity {},
    ] {
        assert!(
            http.exchange(&operation, Some((&credential).into()))
                .is_err()
        );
    }
    signed_session_refuses_issuer_scope(http, account, delivery);
    malformed_requests(http, &credential, delivery);
    runtime.block_on(operator.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "DELETE FROM central_identity.terms_acceptances WHERE account_id=$1 AND subject='central'",
        vec![Uuid::from_bytes(account.account).into()]))).unwrap();
    let (native, _) = support::password::sign_in(http, account, support::password::OLD);
    assert!(
        http.exchange(&Request::TowerProgress {}, Some((&native).into()))
            .is_ok()
    );
    assert!(
        http.exchange(
            &Request::TowerAwardExport { after: None },
            Some((&native).into())
        )
        .is_ok()
    );
    revoke_and_delete(http, runtime, operator, &credential, delivery);
}

fn signed_session_refuses_issuer_scope(http: &Http, account: Identity, delivery: Delivery) {
    let (native, _) = support::password::sign_in(http, account, support::password::OLD);
    assert!(
        http.exchange(&Request::TowerAward { delivery }, Some((&native).into()))
            .is_err()
    );
}

/// The website's listener: it refuses the issuer's delivery, and it serves the
/// two reads to the cookie's session and to nobody without one.
fn browser_listener(
    http: &Http,
    runtime: &Runtime,
    credential: &Token,
    delivery: Delivery,
    account: Identity,
) {
    let server = runtime
        .block_on(DevelopmentServer::bind(browser_policy_config()))
        .unwrap();
    let address = server.local_addr();
    let (stop, stopped) = oneshot::channel();
    let service = http.identity.clone();
    let task = runtime.spawn(server.run_browser(service, BrowserOrigin::live(), async {
        let _ = stopped.await;
    }));
    let tested = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let browser = support::browser::Browser {
            address,
            client: "198.51.100.7",
        };
        assert!(
            browser
                .answer(&Request::TowerAward { delivery }, Some(credential))
                .is_err()
        );
        let cookie = support::password::browser_sign_in(
            &browser,
            account,
            support::password::context(http, account),
            support::password::OLD,
        );
        // Nothing has been delivered yet: the signed-in page reads an empty
        // record, and the same requests without the cookie read nothing.
        assert!(matches!(
            browser.answer(&Request::TowerProgress {}, Some(&cookie)),
            Ok(Response::TowerProgress { progress }) if progress == Progress::default()
        ));
        assert!(matches!(
            browser.answer(&Request::TowerAwardExport { after: None }, Some(&cookie)),
            Ok(Response::TowerAwardExport { export }) if export.receipts.is_empty()
        ));
        for read in [
            Request::TowerProgress {},
            Request::TowerAwardExport { after: None },
        ] {
            assert!(browser.answer(&read, None).is_err());
        }
        browser.answer(&Request::Logout, Some(&cookie)).unwrap();
    }));
    let _ = stop.send(());
    runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    });
    assert!(TcpStream::connect_timeout(&address, Duration::from_millis(100)).is_err());
    if let Err(panic) = tested {
        std::panic::resume_unwind(panic);
    }
}

fn malformed_requests(http: &Http, credential: &Token, delivery: Delivery) {
    let mut text =
        String::from_utf8(central_api::encode(&Request::TowerAward { delivery }).unwrap()).unwrap();
    assert_eq!(text.pop(), Some('}'));
    text.push_str(",\"gold\":999}");
    assert!(http.written(&text, Some(credential.into())).is_err());
    let mut changed = delivery;
    changed.fact.stage = 2;
    changed.fact.wins = 2;
    assert!(central_client::deliver_tower_award(http, credential, changed).is_err());
}

fn revoke_and_delete(
    http: &Http,
    runtime: &Runtime,
    operator: &DatabaseConnection,
    credential: &Token,
    delivery: Delivery,
) {
    runtime
        .block_on(
            operator.execute_unprepared(
                "UPDATE central_identity.tower_award_issuers SET enabled=false",
            ),
        )
        .unwrap();
    assert!(central_client::deliver_tower_award(http, credential, delivery).is_err());
    runtime
        .block_on(
            operator
                .execute_unprepared("UPDATE central_identity.tower_award_issuers SET enabled=true"),
        )
        .unwrap();
    runtime
        .block_on(operator.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "DELETE FROM central_identity.accounts WHERE id=$1",
            vec![Uuid::from_bytes(delivery.account).into()],
        )))
        .unwrap();
    assert_eq!(
        central_client::deliver_tower_award(http, credential, delivery)
            .unwrap()
            .disposition,
        Disposition::AccountGone
    );
}
