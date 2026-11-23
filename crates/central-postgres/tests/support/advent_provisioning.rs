//! A separate staging login can publish immutable content, but cannot read its image payloads,
//! award items, or change an existing season. These bytes only exercise opaque DB storage.
use central_api::advent::{
    Kind, Text,
    framing::{Framing, MatFraming},
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
        mat_framing: None,
        kind: match day {
            0 => None,
            1 => Some(Kind::Playmat),
            _ => Some(Kind::Sleeve),
        },
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

async fn publish(
    db: &DatabaseConnection,
    last: u8,
    change_last: bool,
    composition: Option<MatFraming>,
) -> Result<(), StoreError> {
    let mut pack = Import::begin(db, YEAR, art(0)).await?;
    for day in 1..=last {
        let hint = if change_last && day == 24 {
            "Edited fixture hint"
        } else {
            "Fixture hint"
        };
        let mut image = art(day);
        if day == 1 {
            image.mat_framing = composition;
        }
        pack.door(
            day,
            &challenge(),
            [text(hint), text("Second fixture hint")],
            image,
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
        "UPDATE central_identity.playmat_framing SET framing='{}' WHERE false",
        "DELETE FROM central_identity.playmat_framing WHERE false",
    ] {
        assert!(db.execute_unprepared(query).await.is_err());
    }
    assert!(publish(&db, 23, false, None).await.is_err());
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
    let (left, right) = tokio::join!(publish(&db, 24, false, None), publish(&db, 24, false, None));
    left.unwrap();
    right.unwrap();
    let authored = MatFraming {
        duel: Framing {
            focus: [6800, 4200],
            safe: Some([4500, 1800, 7600, 5200]),
        },
        ..MatFraming::default()
    };
    // An incomplete or conflicting republish also rolls back its framing backfill.
    let with_layout = "SELECT count(*) AS value FROM central_identity.playmat_framing f
        JOIN central_identity.advent_doors d ON d.art_id=f.art_id WHERE d.year=2194";
    assert!(publish(&db, 24, true, Some(authored)).await.is_err());
    assert_eq!(super::scalar(operator, with_layout).await, 0);
    publish(&db, 24, false, Some(authored)).await.unwrap();
    publish(&db, 24, false, Some(authored)).await.unwrap();
    publish(&db, 24, false, None).await.unwrap();
    assert!(
        publish(&db, 24, false, Some(MatFraming::default()))
            .await
            .is_err()
    );
    assert_eq!(super::scalar(operator, with_layout).await, 1);
    assert!(
        operator
            .execute_unprepared(
                "INSERT INTO central_identity.playmat_framing(art_id,framing)
        SELECT art_id,'{}'::jsonb FROM central_identity.advent_doors WHERE year=2194 AND door=2"
            )
            .await
            .is_err()
    );
    assert_eq!(super::scalar(operator,"SELECT count(*) AS value FROM central_identity.advent_doors WHERE year=2194 AND hint1_en='Fixture hint'").await,24);
    db.close().await.unwrap();
    // Leave the fixture usable for another independent run or a schema clone.
    operator.execute_unprepared("BEGIN;
        DELETE FROM central_identity.advent_doors WHERE year=2194 AND hint1_en='Fixture hint';
        DELETE FROM central_identity.advent_campaigns WHERE year=2194;
        DELETE FROM central_identity.advent_art WHERE size=4 AND title_en='Fixture art' AND title_de='Fixture art'
          AND substring(content FROM 1 FOR 3)=decode('f1ec7a','hex'); COMMIT;").await.unwrap();
}
