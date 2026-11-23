use super::{
    Registered, Table, api,
    game::{Game, attestation},
    r,
};
use sea_orm::{ConnectionTrait, prelude::Uuid};

// A synthetic constraint targets this one fixture match and forces failure
// exactly when Elo settlement would write its first receipt.
async fn rollback(table: &Table<'_>, live: &Game<'_>, terminal: &r::HostRequest) {
    let matched = Uuid::from_bytes(live.fence.match_id);
    table
        .db
        .execute_unprepared(&format!(
        "ALTER TABLE central_identity.ranked_receipts ADD CONSTRAINT ranked_fixture_receipt_failure
         CHECK (match_id <> '{matched}'::uuid) NOT VALID"
    ))
        .await
        .unwrap();
    let failed = live.witness.host(table.store, terminal.clone()).await;
    table.db.execute_unprepared(
        "ALTER TABLE central_identity.ranked_receipts DROP CONSTRAINT ranked_fixture_receipt_failure"
    ).await.unwrap();
    assert!(failed.is_err());
    assert_eq!(table.receipt_count(live.fence.match_id).await, 0);
    assert_eq!(table.status(live.fence.match_id).await.head, live.opening);
}

pub(super) async fn run(table: &Table<'_>) {
    let live = Game::open(table, 110).await;
    let fence = live.fence;
    let matched = fence.match_id;
    let command = super::super::id(114);
    let bytes = [0x44, 0xb3, 0x53];
    let submit = live.submit(command, &bytes);
    assert!(
        table
            .seat(table.observer, r::SeatRequest::Status { match_id: matched })
            .await
            .is_err()
    );
    assert!(
        table
            .seat(&live.leader.token, submit.clone())
            .await
            .is_err()
    );
    assert!(table.seat(table.observer, submit.clone()).await.is_err());
    let (first, retry) = tokio::join!(
        table.seat(live.seats[0], submit.clone()),
        table.seat(live.seats[0], submit.clone())
    );
    let ack = r::SeatResponse::Receipt {
        command_id: command,
        position: 1,
    };
    assert_eq!(first.unwrap(), ack);
    assert_eq!(retry.unwrap(), ack);
    assert!(
        table
            .seat(live.seats[0], live.submit(command, &[0xff]))
            .await
            .is_err()
    );
    assert!(table.seat(live.seats[1], submit).await.is_err());
    let bad = live.submit(super::super::id(115), &[0xff]);
    assert!(table.seat(live.seats[0], bad).await.is_err());
    let head = live
        .opening
        .next(
            matched,
            command,
            r::Source::Seat0,
            &r::CommandBytes::new(bytes.to_vec()).unwrap(),
        )
        .unwrap();
    let request = r::HostRequest::Read { fence, after: 0 };
    let r::HostResponse::Page {
        entries, certified, ..
    } = live.witness.host(table.store, request).await.unwrap()
    else {
        panic!("journal page")
    };
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].source, r::Source::Seat0);
    assert_eq!(entries[0].head, head);
    assert_eq!(entries[0].body.as_slice(), bytes);
    assert_eq!(certified, live.opening);
    let r::HostResponse::Page {
        setup,
        pools,
        entries,
        ..
    } = live
        .witness
        .host(table.store, r::HostRequest::Read { fence, after: 1 })
        .await
        .unwrap()
    else {
        panic!("suffix page")
    };
    assert!(setup.is_none() && pools.is_none() && entries.is_empty());
    settle(table, &live, head).await;
}

async fn settle(table: &Table<'_>, live: &Game<'_>, head: r::Head) {
    let fence = live.fence;
    let matched = fence.match_id;
    let premature = api::HostRequest::Finish {
        match_id: matched,
        game: live.engine,
        outcome: api::Outcome::FirstWins,
    };
    assert!(
        table
            .store
            .ranked_host(Registered(&live.leader.token), premature)
            .await
            .is_err()
    );
    let bad_head = r::Head {
        position: 1,
        chain: [0x77; 32],
    };
    assert!(
        live.witness
            .host(
                table.store,
                attestation(fence, bad_head, 6, Some(api::Outcome::FirstWins))
            )
            .await
            .is_err()
    );
    let terminal = attestation(fence, head, 6, Some(api::Outcome::FirstWins));
    live.leader
        .host(table.store, terminal.clone())
        .await
        .unwrap();
    assert_eq!(table.status(matched).await.head, live.opening);
    assert_eq!(table.receipt_count(matched).await, 0);
    rollback(table, live, &terminal).await;
    let (a, b) = tokio::join!(
        live.witness.host(table.store, terminal.clone()),
        live.witness.host(table.store, terminal)
    );
    assert!(a.is_ok() || b.is_ok());
    assert_eq!(table.receipt_count(matched).await, 2);
    retention(table, live, head).await;
    for seat in live.seats {
        let queue = super::super::queue(table.store, seat, api::Request::Status)
            .await
            .unwrap();
        assert!(
            matches!(queue,api::QueueState::Finished{match_id,outcome:api::Outcome::FirstWins,..}if match_id==matched)
        );
    }
}

async fn retention(table: &Table<'_>, live: &Game<'_>, terminal: r::Head) {
    use sea_orm::{DbBackend, Statement};
    let id = Uuid::from_bytes(live.fence.match_id);
    table.db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "UPDATE central_identity.ranked_replicas SET retire_at=clock_timestamp()-interval '1 second' WHERE match_id=$1",[id.into()])).await.unwrap();
    live.leader.ready(table.store, 2, super::BUILD).await;
    assert!(
        live.witness
            .host(
                table.store,
                r::HostRequest::Read {
                    fence: live.fence,
                    after: 0
                }
            )
            .await
            .is_err()
    );
    let row=table.db.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT setup IS NULL AND seed IS NULL AND initial_pools IS NULL AND initial_evidence IS NULL AND evidence IS NULL
         AND state_digest IS NULL AND retire_at IS NULL AND NOT EXISTS(SELECT 1 FROM central_identity.ranked_commands c
           WHERE c.match_id=r.match_id) AND NOT EXISTS(SELECT 1 FROM central_identity.ranked_witnesses w
           WHERE w.match_id=r.match_id) AS erased FROM central_identity.ranked_replicas r WHERE match_id=$1",[id.into()])).await.unwrap().unwrap();
    assert!(row.try_get::<bool>("", "erased").unwrap());
    let status = table.status(live.fence.match_id).await;
    assert_eq!((status.head, status.phase), (terminal, r::Phase::Finished));
    assert_eq!(table.receipt_count(live.fence.match_id).await, 2);
}
