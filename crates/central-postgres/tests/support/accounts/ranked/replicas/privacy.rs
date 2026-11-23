//! Real per-seat consent for a specific assigned operator and terms version.
use super::{AccountStore, Error, Node, Table, Token, api, r};
use central_api::terms::{self, TermsText, TermsVersion};

pub(in super::super) async fn offers(
    store: &AccountStore,
    seat: &Token,
    matched: api::Id,
) -> (r::Fence, Vec<r::HostConsentOffer>) {
    let request = r::SeatRequest::Hosts { match_id: matched };
    let response = store
        .ranked(Some(seat.into()), api::Request::Replicate { request })
        .await
        .unwrap();
    let api::Response::Replica {
        response: r::SeatResponse::Hosts { fence, hosts },
    } = response
    else {
        panic!("operator offers expected")
    };
    (fence, hosts)
}

pub(in super::super) async fn approve_all(store: &AccountStore, seat: &Token, matched: api::Id) {
    let (fence, offers) = offers(store, seat, matched).await;
    let mut hosts = Vec::new();
    for offer in offers {
        if offer.needs_gateway_terms {
            store
                .accept_gateway_terms(seat, &offer.host.origin, offer.host.terms.as_ref().unwrap())
                .await
                .unwrap();
        }
        hosts.push(offer.host);
    }
    let request = r::SeatRequest::ApproveHosts { fence, hosts };
    store
        .ranked(Some(seat.into()), api::Request::Replicate { request })
        .await
        .unwrap();
}

pub(super) async fn private(table: &Table<'_>, node: &Node, fence: r::Fence, role: r::Role) {
    let inspect = r::HostRequest::Inspect {
        match_id: fence.match_id,
    };
    assert_eq!(
        node.host(table.store, inspect).await.unwrap(),
        r::HostResponse::AwaitingConsent { fence, role }
    );
    let r::HostResponse::Work { items, .. } =
        node.host(table.store, r::HostRequest::Work).await.unwrap()
    else {
        panic!("bounded assigned list")
    };
    assert!(
        items
            .iter()
            .all(|work| work.fence.match_id != fence.match_id)
    );
    assert!(
        node.host(table.store, r::HostRequest::Read { fence, after: 0 })
            .await
            .is_err()
    );
}

pub(super) async fn terms(store: &AccountStore, node: &Node, version: &str) {
    let texts = [TermsText {
        lang: "en".into(),
        updated: None,
        markdown: format!("Fixture terms {version}"),
    }];
    let v = TermsVersion {
        version: version.into(),
        digest: terms::digest(&texts),
    };
    store
        .publish_gateway_terms(node.origin, &v, &texts)
        .await
        .unwrap();
}

pub(super) async fn protect(table: &Table<'_>, matched: api::Id, leader: &Node, witness: &Node) {
    let (fence, offered) = offers(table.store, table.seats[0].1, matched).await;
    assert_eq!(offered.len(), 2);
    assert!(offered.iter().all(|offer| !offer.approved));
    assert!(
        table
            .seat(table.seats[0].1, r::SeatRequest::RelayRoute { fence })
            .await
            .is_err()
    );
    private(table, leader, fence, r::Role::Primary).await;
    private(table, witness, fence, r::Role::Standby).await;
    let mut altered = offered[0].host.clone();
    altered.origin = "https://different.example".into();
    let forged = r::SeatRequest::ApproveHosts {
        fence,
        hosts: vec![altered],
    };
    assert!(table.seat(table.seats[0].1, forged).await.is_err());
    assert_eq!(
        table
            .seat(table.observer, r::SeatRequest::Hosts { match_id: matched })
            .await
            .unwrap_err(),
        Error::Rejected
    );
    approve_all(table.store, table.seats[0].1, matched).await;
    assert!(
        table
            .seat(table.seats[0].1, r::SeatRequest::RelayRoute { fence })
            .await
            .is_err()
    );
    private(table, leader, fence, r::Role::Primary).await;
    private(table, witness, fence, r::Role::Standby).await;
    assert_eq!(
        table.status(matched).await.consent,
        r::ConsentState::Waiting
    );
    approve_all(table.store, table.seats[1].1, matched).await;
    assert_eq!(table.status(matched).await.consent, r::ConsentState::Ready);
}
