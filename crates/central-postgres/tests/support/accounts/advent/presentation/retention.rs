//! The operator removes abandoned capabilities and their recovery receipts
//! without touching the same wearer’s independently active table.
use super::{A, Fixture, Gateway, H, HOME, Player, R, Registered, admission, api, begin, table};
use sea_orm::{ConnectionTrait, DbBackend, Statement, prelude::Uuid};

async fn wear(f: &Fixture<'_>, home: &Gateway, owner: &Player, mat: &api::Reward) -> Uuid {
    let row =
        f.db.query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT uuidv7() AS id",
        ))
        .await
        .unwrap()
        .unwrap();
    let id = row.try_get::<Uuid>("", "id").unwrap();
    let look = api::Equipment {
        sleeve: None,
        playmat: Some(mat.id),
    };
    let sid = admission(f, owner, HOME).await;
    let A::Table {
        response: table::Response::Grant { proof },
    } = f
        .call(owner, begin(id.into_bytes(), HOME, look))
        .await
        .unwrap()
    else {
        panic!("table invitation")
    };
    let reply = f
        .store
        .advent_table_host(
            Registered(&home.credential),
            H::Confirm {
                game: id.into_bytes(),
                seat: 1,
                proof,
                admission: sid,
            },
        )
        .await
        .unwrap();
    assert!(matches!(reply, R::Confirmed { .. }));
    id
}

async fn counts(f: &Fixture<'_>, host: Uuid, id: Uuid) -> (i64, i64) {
    let row=f.db.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT (SELECT count(*) FROM central_identity.advent_table_looks WHERE gateway_id=$1 AND game=$2) AS grants,
                (SELECT count(*) FROM central_identity.advent_table_outfits WHERE gateway_id=$1 AND game=$2) AS receipts",
        [host.into(),id.into()])).await.unwrap().unwrap();
    (
        row.try_get("", "grants").unwrap(),
        row.try_get("", "receipts").unwrap(),
    )
}

pub(super) async fn run(f: &Fixture<'_>, home: &Gateway, owner: &Player, mat: &api::Reward) {
    let abandoned = wear(f, home, owner, mat).await;
    let playing = wear(f, home, owner, mat).await;
    for relation in ["advent_table_looks", "advent_table_outfits"] {
        let changed=f.db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            format!("UPDATE central_identity.{relation} SET expires_at=clock_timestamp()-interval '1 second'
                     WHERE gateway_id=$1 AND game=$2"),[home.id.into(),abandoned.into()])).await.unwrap();
        assert_eq!(changed.rows_affected(), 1);
    }
    assert_eq!(counts(f, home.id, abandoned).await, (1, 1));
    central_postgres::sweep(f.db).await.unwrap();
    assert_eq!(counts(f, home.id, abandoned).await, (0, 0));
    assert_eq!(counts(f, home.id, playing).await, (1, 1));
}
