//! Verify the mapping through the authenticated store entry point, including
//! reversed seat assignment and a host credential presented as a player.
use super::{Error, Table, api, r};
use central_api::profile;
use sea_orm::{ConnectionTrait, DbBackend, Statement, prelude::Uuid};

pub(super) async fn run(table: &Table<'_>, id: api::Id, second_seat: u8) {
    let mut handles = [None, None];
    let request = r::SeatRequest::Participants { match_id: id };
    for (index, name) in ["Birch", "Laurel"].into_iter().enumerate() {
        let tag = 530_100_001 + i32::try_from(index).unwrap();
        let row = table.db.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "UPDATE central_identity.accounts SET display_name=$2, display_name_at=now(), tag=coalesce(tag,$3)
             WHERE id=$1 RETURNING tag",
            vec![Uuid::from_bytes(table.seats[index].0).into(), name.into(), tag.into()]))
            .await.unwrap().unwrap();
        let tag: i32 = row.try_get("", "tag").unwrap();
        let ordinal = if index == 1 {
            second_seat
        } else {
            1 - second_seat
        };
        handles[usize::from(ordinal)] = Some(profile::handle(name, u32::try_from(tag).unwrap()));
    }
    let expected = r::SeatResponse::Participants {
        match_id: id,
        seats: handles.clone(),
        ai: [None, None],
    };
    for (_, seat) in table.seats {
        assert_eq!(table.seat(seat, request.clone()).await.unwrap(), expected);
    }
    table.db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "UPDATE central_identity.accounts SET display_name=NULL, display_name_at=NULL WHERE id=$1",
        vec![Uuid::from_bytes(table.seats[1].0).into()]))
        .await.unwrap();
    handles[usize::from(second_seat)] = None;
    assert_eq!(
        table.seat(table.seats[0].1, request.clone()).await.unwrap(),
        r::SeatResponse::Participants {
            match_id: id,
            seats: handles,
            ai: [None, None],
        }
    );
    for stranger in [table.observer, &table.first.token, &table.second.token] {
        assert_eq!(
            table.seat(stranger, request.clone()).await,
            Err(Error::Rejected)
        );
    }
    assert_eq!(
        table
            .seat(
                table.observer,
                r::SeatRequest::Participants {
                    match_id: super::super::id(249),
                }
            )
            .await,
        Err(Error::Rejected)
    );
}

pub(super) async fn closed(table: &Table<'_>, id: api::Id) {
    for (_, seat) in table.seats {
        assert_eq!(
            table
                .seat(seat, r::SeatRequest::Participants { match_id: id })
                .await,
            Err(Error::Rejected)
        );
    }
}
