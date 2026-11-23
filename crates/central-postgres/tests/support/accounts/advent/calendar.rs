use super::{A, DAY, Fixture, Player, Q, YEAR, api, material};
use api::DoorState;
use sea_orm::{ConnectionTrait, DbBackend, Statement};

async fn state(f: &Fixture<'_>) -> (Vec<api::Door>, Option<api::AssetMeta>) {
    let A::Calendar {
        year,
        doors,
        hero,
        starts_at,
        closes_at,
        now,
    } = f.call(&f.alice, Q::Calendar { year: YEAR }).await.unwrap()
    else {
        panic!("signed-in calendar")
    };
    assert_eq!(year, YEAR);
    assert_eq!(doors.len(), 24);
    assert_eq!(closes_at - starts_at, i64::from(37 * DAY));
    for (index, door) in doors.iter().enumerate() {
        assert_eq!(door.door, u8::try_from(index + 1).unwrap());
        assert_eq!(
            door.enables_at,
            starts_at + i64::try_from(index).unwrap() * i64::from(DAY)
        );
    }
    let db_now =
        f.db.query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT floor(extract(epoch FROM clock_timestamp()))::bigint AS t",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<i64>("", "t")
        .unwrap();
    assert!((0..30).contains(&(db_now - now)));
    (doors, hero)
}

async fn hidden(f: &Fixture<'_>, player: &Player, door: u8) {
    let (id, digest) = f.asset(door).await;
    for q in [
        Q::Puzzle { year: YEAR, door },
        Q::Hint {
            year: YEAR,
            door,
            number: 1,
        },
        Q::Submit {
            year: YEAR,
            door,
            line: material::line(2),
        },
        Q::Claim { year: YEAR, door },
        Q::Asset {
            scope: api::AssetScope::Owned { item: id },
            offset: 0,
            cached: None,
        },
        Q::Asset {
            scope: api::AssetScope::Owned { item: id },
            offset: 0,
            cached: Some(digest),
        },
    ] {
        assert!(matches!(
            f.call(player, q).await,
            Err(central_api::Error::Rejected)
        ));
    }
}

async fn dates(f: &Fixture<'_>) {
    let row=f.db.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT starts_at=timestamptz '2105-11-30 23:00:00Z' AS first, starts_at+interval '552 hours'=timestamptz '2105-12-23 23:00:00Z' AS last,
         closes_at=timestamptz '2106-01-06 23:00:00Z' AS close FROM central_identity.advent_campaigns WHERE year=2105")).await.unwrap().unwrap();
    for column in ["first", "last", "close"] {
        assert!(row.try_get::<bool>("", column).unwrap());
    }
    f.elapsed(-600).await;
    assert!(matches!(
        f.call(&f.alice, Q::CurrentCalendar {}).await.unwrap(),
        A::Calendar {
            year: YEAR,
            hero: None,
            ..
        }
    ));
    let (doors, hero) = state(f).await;
    assert!(hero.is_none() && doors.iter().all(|d| d.state == DoorState::Locked));
    assert_eq!(f.inventory(&f.alice).await, Vec::<api::Reward>::new());
    for day in [1, 24] {
        hidden(f, &f.alice, day).await;
    }
    let q = Q::Asset {
        scope: api::AssetScope::Calendar { year: YEAR },
        offset: 0,
        cached: None,
    };
    assert!(f.call(&f.alice, q.clone()).await.is_err());
    f.elapsed(0).await;
    let (doors, hero) = state(f).await;
    assert!(hero.is_some());
    assert_eq!(doors[0].state, DoorState::Open);
    assert!(doors[1..].iter().all(|d| d.state == DoorState::Locked));
    assert!(matches!(
        f.call(&f.alice, q).await.unwrap(),
        A::Asset { .. }
    ));
    assert!(
        matches!(f.call(&f.alice,Q::Puzzle{year:YEAR,door:1}).await.unwrap(),A::Puzzle{puzzle,..}if puzzle==material::board(1))
    );
    hidden(f, &f.alice, 24).await;
}

async fn interrupted_award(f: &Fixture<'_>) {
    let account = f.alice.account;
    f.db.execute_unprepared(&format!(
        "ALTER TABLE central_identity.advent_entitlements ADD CONSTRAINT advent_fixture_pause
        CHECK (account_id<>'{account}'::uuid OR year<>2105 OR door<>1) NOT VALID"
    ))
    .await
    .unwrap();
    let answer = f
        .call(
            &f.alice,
            Q::Submit {
                year: YEAR,
                door: 1,
                line: material::line(1),
            },
        )
        .await;
    f.db.execute_unprepared(
        "ALTER TABLE central_identity.advent_entitlements DROP CONSTRAINT advent_fixture_pause",
    )
    .await
    .unwrap();
    assert!(answer.is_err());
    let row=f.db.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT EXISTS(SELECT 1 FROM central_identity.advent_solutions WHERE account_id=$1 AND year=2105 AND door=1) AS solved,
         EXISTS(SELECT 1 FROM central_identity.advent_entitlements WHERE account_id=$1 AND year=2105 AND door=1) AS awarded",
         [account.into()])).await.unwrap().unwrap();
    assert!(!row.try_get::<bool>("", "solved").unwrap());
    assert!(!row.try_get::<bool>("", "awarded").unwrap());
}

async fn independent_awards(f: &Fixture<'_>) -> api::Reward {
    let door = 1;
    assert!(matches!(
        f.call(
            &f.alice,
            Q::Submit {
                year: YEAR,
                door,
                line: material::line(0)
            }
        )
        .await
        .unwrap(),
        A::Submission { won: false }
    ));
    assert!(
        f.call(&f.alice, Q::Claim { year: YEAR, door })
            .await
            .is_err()
    );
    interrupted_award(f).await;
    tokio::join!(
        f.grant(&f.alice, door, 1),
        f.grant(&f.alice, door, 2),
        f.grant(&f.bob, door, 3)
    );
    // A client may close immediately after its winning move, before reading the prize.
    for player in [&f.alice, &f.bob] {
        let owned = f.inventory(player).await;
        assert_eq!(owned.len(), 1);
        assert_eq!((owned[0].year, owned[0].door), (YEAR, door));
    }
    let (first, retry, bob) = tokio::join!(
        f.claim(&f.alice, door),
        f.claim(&f.alice, door),
        f.claim(&f.bob, door)
    );
    assert_eq!(first, retry);
    assert_eq!(first.id, bob.id);
    assert_eq!(f.inventory(&f.alice).await, vec![first.clone()]);
    assert_eq!(f.inventory(&f.bob).await, vec![bob]);
    assert_eq!(f.inventory(&f.observer).await, Vec::<api::Reward>::new());
    let row=f.db.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT (SELECT count(*) FROM central_identity.advent_solutions WHERE year=2105 AND door=1) AS solved,
         (SELECT count(*) FROM central_identity.advent_entitlements WHERE year=2105 AND door=1) AS owned")).await.unwrap().unwrap();
    for name in ["solved", "owned"] {
        assert_eq!(row.try_get::<i64>("", name).unwrap(), 2);
    }
    first
}

async fn catch_up(f: &Fixture<'_>) -> api::Reward {
    f.elapsed(23 * DAY - 600).await;
    assert_eq!(state(f).await.0[23].state, DoorState::Locked);
    f.elapsed(23 * DAY).await;
    assert_eq!(state(f).await.0[23].state, DoorState::Open);
    f.elapsed(37 * DAY - 600).await;
    assert!(matches!(
        f.call(&f.alice, Q::CurrentCalendar {}).await.unwrap(),
        A::Calendar {
            year: YEAR,
            hero: Some(_),
            ..
        }
    ));
    f.grant(&f.alice, 24, 2).await;
    assert_eq!(state(f).await.0[1].state, DoorState::Open);
    assert!(
        matches!(f.call(&f.alice,Q::Hint{year:YEAR,door:2,number:2}).await.unwrap(),A::Hint{text,..}if text==material::hint(2,2))
    );
    f.elapsed(37 * DAY).await;
    assert!(matches!(
        f.call(&f.alice, Q::CurrentCalendar {}).await.unwrap(),
        A::NoCalendar {}
    ));
    assert!(
        f.inventory(&f.alice)
            .await
            .iter()
            .any(|item| item.year == YEAR && item.door == 24)
    );
    let (doors, hero) = state(f).await;
    assert!(hero.is_none());
    assert_eq!(doors[0].state, DoorState::Claimed);
    assert_eq!(doors[1].state, DoorState::Expired);
    hidden(f, &f.alice, 2).await;
    assert!(
        f.call(
            &f.alice,
            Q::Asset {
                scope: api::AssetScope::Calendar { year: YEAR },
                offset: 0,
                cached: None
            }
        )
        .await
        .is_err()
    );
    let late = f.claim(&f.alice, 24).await;
    assert_eq!(late.kind, api::Kind::Playmat);
    assert_eq!(f.claim(&f.alice, 24).await, late);
    assert!(
        f.call(
            &f.bob,
            Q::Claim {
                year: YEAR,
                door: 24
            }
        )
        .await
        .is_err()
    );
    late
}

pub(super) async fn run(f: &Fixture<'_>) -> (api::Reward, api::Reward) {
    dates(f).await;
    let sleeve = independent_awards(f).await;
    let mat = catch_up(f).await;
    assert_eq!(f.inventory(&f.alice).await.len(), 2);
    assert_eq!(f.inventory(&f.bob).await.len(), 1);
    assert_eq!(f.claim(&f.alice, 1).await, sleeve);
    (sleeve, mat)
}
