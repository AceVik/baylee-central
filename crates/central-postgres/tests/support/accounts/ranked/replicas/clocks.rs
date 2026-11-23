//! Advance only the operator-owned fixture checkpoint. All game operations,
//! pulse evidence and the settlement traverse the ordinary restricted runtime.
use super::{
    Table, api,
    game::{Game, attestation},
    r,
};
use sea_orm::{ConnectionTrait, DbBackend, Statement, prelude::Uuid};

async fn entries(table: &Table<'_>, game: &Game<'_>) -> Vec<r::Entry> {
    let r::HostResponse::Page { entries, .. } = game
        .witness
        .host(
            table.store,
            r::HostRequest::Read {
                fence: game.fence,
                after: 0,
            },
        )
        .await
        .unwrap()
    else {
        panic!("journal page")
    };
    entries
}

pub(super) async fn run(table: &Table<'_>) {
    let game = Game::open(table, 150).await;
    let fence = game.fence;
    let presence = r::SeatRequest::Presence {
        fence,
        connected: true,
    };
    assert!(table.seat(table.observer, presence.clone()).await.is_err());
    for seat in game.seats {
        table.seat(seat, presence.clone()).await.unwrap();
    }
    let mask = r::HostRequest::Presence { fence, attached: 3 };
    assert!(game.witness.host(table.store, mask.clone()).await.is_err());
    game.leader.host(table.store, mask).await.unwrap();
    let view = table.status(fence.match_id).await.clock.unwrap();
    assert!(view.running);
    assert_eq!(view.decision_secs, [Some(180), None]);

    let changed=table.db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "UPDATE central_identity.ranked_clocks SET checkpoint=convert_to(
            jsonb_set(convert_from(checkpoint,'UTF8')::jsonb,'{thinking_ms,0}','180000'::jsonb)::text,'UTF8')
            WHERE match_id=$1", [Uuid::from_bytes(fence.match_id).into()])).await.unwrap();
    assert_eq!(changed.rows_affected(), 1);
    let status = table.status(fence.match_id).await;
    assert_eq!(status.clock.unwrap().decision_secs, [Some(0), None]);
    let late = game.submit(super::super::id(156), b"late decision");
    assert!(matches!(
        table.seat(game.seats[0], late).await.unwrap(),
        r::SeatResponse::Status { .. }
    ));
    assert_eq!(entries(table, &game).await, Vec::<r::Entry>::new());
    let pulse = r::HostRequest::Pulse { fence };
    game.leader.host(table.store, pulse.clone()).await.unwrap();
    assert_eq!(entries(table, &game).await, Vec::<r::Entry>::new());
    game.witness.host(table.store, pulse.clone()).await.unwrap();
    table.status(fence.match_id).await;
    let commands = entries(table, &game).await;
    let [event] = commands.as_slice() else {
        panic!("exactly one ordered time ruling")
    };
    assert_eq!(event.source, r::Source::Authority);
    assert_eq!(
        r::clock::ClockEvent::parse(&event.body),
        r::clock::ClockEvent::new(0, 7, r::clock::Reason::Decision)
    );
    game.leader.host(table.store, pulse).await.unwrap();
    assert_eq!(entries(table, &game).await, commands);
    assert_eq!(table.receipt_count(fence.match_id).await, 0);
    let end = attestation(fence, event.head, 13, Some(api::Outcome::SecondWins));
    game.witness.host(table.store, end.clone()).await.unwrap();
    assert_eq!(table.receipt_count(fence.match_id).await, 0);
    game.leader.host(table.store, end).await.unwrap();
    assert_eq!(table.receipt_count(fence.match_id).await, 2);
}
