//! A separate staging login can publish immutable content, but cannot read its image payloads,
//! award items, or change an existing season. These bytes only exercise opaque DB storage.
use central_api::advent::{
    Kind, Text,
    puzzle::{Goal, Phase, Puzzle, Spell, Step},
};
use central_postgres::{
    StoreError,
    advent_import::{Art, Import},
    connect_dev,
};
use sea_orm::{ConnectionTrait, DatabaseConnection};

const YEAR: u16 = 2194;

fn text(value: &str) -> Text {
    Text {
        de: value.into(),
        en: value.into(),
    }
}
fn art(day: u8) -> Art {
    Art {
        kind: (day > 0).then_some(Kind::Sleeve),
        tier: "standard".into(),
        title: text("Fixture art"),
        dimensions: [1, 1],
        bytes: vec![0xf1, 0xec, 0x7a, day],
    }
}
fn challenge() -> Puzzle {
    Puzzle {
        goal: Goal::Defeat,
        life: [4, 1],
        mana: 1,
        units: [vec![], vec![]],
        hand: vec![Spell::Spark],
        steps: vec![Step {
            new_turn: None,
            draw: None,
            phase: Phase::Cast,
        }],
    }
}

async fn publish(db: &DatabaseConnection, last: u8, change_last: bool) -> Result<(), StoreError> {
    let mut pack = Import::begin(db, YEAR, art(0)).await?;
    for day in 1..=last {
        let hint = if change_last && day == 24 {
            "Edited fixture hint"
        } else {
            "Fixture hint"
        };
        pack.door(
            day,
            &challenge(),
            [text(hint), text("Second fixture hint")],
            art(day),
        )
        .await?;
    }
    pack.finish().await
}

pub(crate) async fn run(operator: &DatabaseConnection) {
    operator
        .execute_unprepared(include_str!("../../../../ops/advent-importer-grants.sql"))
        .await
        .unwrap();
    let db = connect_dev(super::config(
        "central_advent_importer",
        "CENTRAL_TEST_ADVENT_IMPORT_PASSWORD",
    ))
    .await
    .unwrap();
    assert!(matches!(
        Import::begin(operator, YEAR, art(0)).await,
        Err(StoreError::UnsafeRole)
    ));
    for query in [
        "SELECT content FROM central_identity.advent_art",
        "UPDATE central_identity.advent_art SET title_de='changed' WHERE false",
        "DELETE FROM central_identity.advent_doors WHERE false",
        "SELECT account_id FROM central_identity.advent_entitlements",
        "DELETE FROM central_identity.advent_entitlements WHERE false",
    ] {
        assert!(db.execute_unprepared(query).await.is_err());
    }
    assert!(publish(&db, 23, false).await.is_err());
    let mut poisoned = Import::begin(&db, YEAR, art(0)).await.unwrap();
    for number in [2, 1] {
        assert!(
            poisoned
                .door(number, &challenge(), [text("a"), text("b")], art(number))
                .await
                .is_err()
        );
    }
    assert!(poisoned.finish().await.is_err());
    assert_eq!(
        super::scalar(
            operator,
            "SELECT count(*) AS value FROM central_identity.advent_campaigns WHERE year=2194"
        )
        .await,
        0
    );
    let (left, right) = tokio::join!(publish(&db, 24, false), publish(&db, 24, false));
    left.unwrap();
    right.unwrap();
    assert!(publish(&db, 24, true).await.is_err());
    assert_eq!(super::scalar(operator,"SELECT count(*) AS value FROM central_identity.advent_doors WHERE year=2194 AND hint1_en='Fixture hint'").await,24);
    db.close().await.unwrap();
    // Leave the fixture usable for another independent run or a schema clone.
    operator.execute_unprepared("BEGIN;
        DELETE FROM central_identity.advent_doors WHERE year=2194 AND hint1_en='Fixture hint';
        DELETE FROM central_identity.advent_campaigns WHERE year=2194;
        DELETE FROM central_identity.advent_art WHERE size=4 AND title_en='Fixture art' AND title_de='Fixture art'
          AND substring(content FROM 1 FOR 3)=decode('f1ec7a','hex'); COMMIT;").await.unwrap();
}
