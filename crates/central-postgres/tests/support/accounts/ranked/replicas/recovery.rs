//! Advance fixture-only leases; every takeover and outcome still traverses the runtime role.
use super::{
    BUILD, Node, Registered, Table, api,
    game::{Game, attestation},
    privacy, r,
};
use sea_orm::{ConnectionTrait, DbBackend, Statement, prelude::Uuid};

enum Deadline {
    Leader,
    Recovery,
}
async fn expire(table: &Table<'_>, matched: api::Id, deadline: Deadline) {
    let sql = match deadline {
        Deadline::Leader => {
            "UPDATE central_identity.ranked_replicas SET primary_until=clock_timestamp()-interval '1 second' WHERE match_id=$1"
        }
        Deadline::Recovery => {
            "UPDATE central_identity.ranked_replicas SET paused_until=clock_timestamp()-interval '1 second' WHERE match_id=$1"
        }
    };
    let changed = table
        .db
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            [Uuid::from_bytes(matched).into()],
        ))
        .await
        .unwrap();
    assert_eq!(changed.rows_affected(), 1);
}

async fn input(table: &Table<'_>, live: &Game<'_>, key: u8) -> r::Head {
    let id = super::super::id(key);
    let bytes = [0x61, key, 0x07];
    assert_eq!(
        table
            .seat(live.seats[0], live.submit(id, &bytes))
            .await
            .unwrap(),
        r::SeatResponse::Receipt {
            command_id: id,
            position: 1
        }
    );
    live.opening
        .next(
            live.fence.match_id,
            id,
            r::Source::Seat0,
            &r::CommandBytes::new(bytes.to_vec()).unwrap(),
        )
        .unwrap()
}

async fn divergence(table: &Table<'_>) {
    let live = Game::open(table, 120).await;
    let fence = live.fence;
    let head = input(table, &live, 123).await;
    let left = attestation(fence, head, 1, Some(api::Outcome::FirstWins));
    let right = attestation(fence, head, 2, Some(api::Outcome::FirstWins));
    live.leader.host(table.store, left).await.unwrap();
    let _ = live.witness.host(table.store, right).await;
    let paused = table.status(fence.match_id).await;
    assert_eq!(
        (paused.phase, paused.head),
        (r::Phase::Disputed, live.opening)
    );
    assert_eq!(table.receipt_count(fence.match_id).await, 0);
    assert!(
        live.witness
            .host(table.store, r::HostRequest::Promote { fence })
            .await
            .is_err()
    );
    expire(table, fence.match_id, Deadline::Recovery).await;
    assert_eq!(table.status(fence.match_id).await.phase, r::Phase::Void);
    assert_eq!(table.receipt_count(fence.match_id).await, 0);
}

async fn stale(table: &Table<'_>, live: &Game<'_>, head: r::Head) {
    let fence = live.fence;
    let requests = [
        r::HostRequest::Read { fence, after: 0 },
        r::HostRequest::Pulse { fence },
        r::HostRequest::Promote { fence },
        attestation(fence, head, 8, Some(api::Outcome::FirstWins)),
    ];
    for request in requests {
        assert!(live.leader.host(table.store, request).await.is_err());
    }
    for request in [
        api::HostRequest::Void {
            match_id: fence.match_id,
        },
        api::HostRequest::Finish {
            match_id: fence.match_id,
            game: live.engine,
            outcome: api::Outcome::FirstWins,
        },
    ] {
        assert!(
            table
                .store
                .ranked_host(Registered(&live.leader.token), request)
                .await
                .is_err()
        );
    }
}

async fn rotated(table: &Table<'_>, live: &Game<'_>, head: r::Head) -> r::Fence {
    let fence = live.fence;
    assert!(
        live.witness
            .host(table.store, r::HostRequest::Promote { fence })
            .await
            .is_err()
    );
    table.outsider.ready(table.store, 2, BUILD).await;
    expire(table, fence.match_id, Deadline::Leader).await;
    assert!(
        live.leader
            .host(table.store, r::HostRequest::Pulse { fence })
            .await
            .is_err()
    );
    let status = table.status(fence.match_id).await;
    assert_eq!(
        (status.host_id, status.game, status.phase, status.head),
        (
            live.witness.id,
            Some(live.engine),
            r::Phase::Paused,
            live.opening
        )
    );
    assert!(status.fence.generation > fence.generation);
    assert!(
        table
            .seat(live.seats[0], r::SeatRequest::RelayRoute { fence })
            .await
            .is_err()
    );
    stale(table, live, head).await;
    super::verify(table.db, live.leader.id, false).await;
    super::verify(table.db, live.leader.id, true).await;
    live.leader.ready(table.store, 2, BUILD).await;
    stale(table, live, head).await;
    assert_eq!(table.receipt_count(fence.match_id).await, 0);
    status.fence
}

async fn grant_replacement(table: &Table<'_>, fence: r::Fence, replacement: &Node) {
    let (_, old) = privacy::offers(table.store, table.seats[0].1, fence.match_id).await;
    assert!(
        old.iter()
            .any(|item| item.role == r::Role::Primary && item.approved)
    );
    let stale = old
        .into_iter()
        .find(|item| item.host.host_id == replacement.id)
        .unwrap()
        .host;
    privacy::terms(table.store, replacement, "replacement-v2").await;
    let changed = r::SeatRequest::ApproveHosts {
        fence,
        hosts: vec![stale],
    };
    assert!(table.seat(table.seats[0].1, changed).await.is_err());
    let (_, current) = privacy::offers(table.store, table.seats[0].1, fence.match_id).await;
    let new = current
        .into_iter()
        .find(|item| item.host.host_id == replacement.id)
        .unwrap();
    assert!(new.needs_gateway_terms && !new.approved);
    assert!(
        table
            .seat(
                table.seats[0].1,
                r::SeatRequest::ApproveHosts {
                    fence,
                    hosts: vec![new.host]
                }
            )
            .await
            .is_err()
    );
    privacy::approve_all(table.store, table.seats[0].1, fence.match_id).await;
    privacy::private(table, replacement, fence, r::Role::Standby).await;
    privacy::approve_all(table.store, table.seats[1].1, fence.match_id).await;
}

async fn restore(table: &Table<'_>) {
    let live = Game::open(table, 130).await;
    let head = input(table, &live, 133).await;
    let fence = rotated(table, &live, head).await;
    privacy::private(table, &table.outsider, fence, r::Role::Standby).await;
    grant_replacement(table, fence, &table.outsider).await;
    let r::HostResponse::Page {
        opening,
        setup,
        pools,
        entries,
        certified,
        ..
    } = table
        .outsider
        .host(table.store, r::HostRequest::Read { fence, after: 0 })
        .await
        .unwrap()
    else {
        panic!("complete durable replay")
    };
    assert_eq!((opening, certified), (live.opening, live.opening));
    assert!(setup.is_some() && pools.is_some());
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].head, head);
    let agree = attestation(fence, live.opening, 5, None);
    live.witness.host(table.store, agree.clone()).await.unwrap();
    table.outsider.host(table.store, agree).await.unwrap();
    let result = attestation(fence, head, 8, Some(api::Outcome::SecondWins));
    table
        .outsider
        .host(table.store, result.clone())
        .await
        .unwrap();
    assert_eq!(table.receipt_count(fence.match_id).await, 0);
    live.witness.host(table.store, result).await.unwrap();
    assert_eq!(table.status(fence.match_id).await.phase, r::Phase::Finished);
    assert_eq!(table.receipt_count(fence.match_id).await, 2);
    table.outsider.ready(table.store, 0, BUILD).await;
}

async fn refusal(table: &Table<'_>) {
    let live = Game::open(table, 140).await;
    let fence = rotated(table, &live, live.opening).await;
    let (_, hosts) = privacy::offers(table.store, table.seats[0].1, fence.match_id).await;
    let host = hosts
        .into_iter()
        .find(|item| item.role == r::Role::Standby)
        .unwrap()
        .host;
    assert_eq!(host.host_id, table.outsider.id);
    privacy::approve_all(table.store, table.seats[0].1, fence.match_id).await;
    privacy::private(table, &table.outsider, fence, r::Role::Standby).await;
    table
        .seat(
            table.seats[1].1,
            r::SeatRequest::DeclineHost { fence, host },
        )
        .await
        .unwrap();
    assert_eq!(table.status(fence.match_id).await.phase, r::Phase::Void);
    assert_eq!(table.receipt_count(fence.match_id).await, 0);
    assert!(
        table
            .outsider
            .host(table.store, r::HostRequest::Read { fence, after: 0 })
            .await
            .is_err()
    );
}

async fn no_certified_standby(table: &Table<'_>) {
    let assignment = table.start_pair(160).await;
    let matched = assignment.match_id;
    let fence = table.status(matched).await.fence;
    let witness = if assignment.host_id == table.first.id {
        &table.second
    } else {
        &table.first
    };
    for (_, player) in table.seats {
        privacy::approve_all(table.store, player, matched).await;
    }
    expire(table, matched, Deadline::Leader).await;
    let paused = table.status(matched).await;
    assert_eq!(
        (paused.phase, paused.host_id, paused.fence),
        (r::Phase::Paused, assignment.host_id, fence)
    );
    assert!(
        witness
            .host(table.store, r::HostRequest::Promote { fence })
            .await
            .is_err()
    );
    expire(table, matched, Deadline::Recovery).await;
    assert_eq!(table.status(matched).await.phase, r::Phase::Void);
    assert_eq!(table.receipt_count(matched).await, 0);
}

pub(super) async fn run(table: &Table<'_>) {
    no_certified_standby(table).await;
    divergence(table).await;
    restore(table).await;
    refusal(table).await;
}
