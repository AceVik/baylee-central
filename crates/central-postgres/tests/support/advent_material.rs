//! Synthetic storage/transport material. Artwork decoding is exercised by the importer codec.
use central_api::advent::{
    self as api,
    puzzle::{Goal, Line, Move, Phase, Puzzle, Step, Unit},
};
use central_postgres::advent_import::{Art, Import};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};

pub fn hint(day: u8, number: u8) -> api::Text {
    api::Text {
        de: format!("Test-Hinweis {day}/{number}"),
        en: format!("Fixture hint {day}/{number}"),
    }
}

pub fn line(attackers: u8) -> Line {
    Line(vec![Move::Attack { attackers }])
}

pub fn board(day: u8) -> Puzzle {
    let unit = Unit {
        power: 4,
        toughness: 2,
        keywords: vec![],
        tapped: false,
    };
    Puzzle {
        goal: Goal::Defeat,
        life: [day, 4],
        mana: 0,
        units: [vec![unit.clone(), unit], vec![]],
        hand: vec![],
        steps: vec![Step {
            new_turn: None,
            draw: None,
            phase: Phase::Attack { guards: [None; 3] },
        }],
    }
}

// Two complete pages plus seven bytes on the first door, small private markers elsewhere.
pub fn bytes(day: u8) -> Vec<u8> {
    if day == 1 {
        let page = usize::try_from(api::PAGE_BYTES).unwrap();
        let mut bytes = vec![day; page];
        bytes.extend(vec![0x69; page]);
        bytes.extend([0xe7; 7]);
        bytes
    } else {
        vec![day; 19 + usize::from(day)]
    }
}

pub fn mat_framing(day: u8) -> api::framing::MatFraming {
    use api::framing::{Framing, MatFraming, MatVariant};
    let duel = Framing {
        focus: [1_500 + u16::from(day), 3_000],
        safe: Some([750, 1_500, 9_000, 7_000]),
    };
    let variants = [MatVariant {
        seats: 5,
        allies: 3,
        sides: 2,
        framing: Framing {
            focus: [8_400, 4_500],
            safe: Some([7_000, 3_000, 9_500, 5_000]),
        },
    }];
    MatFraming {
        duel,
        variants: variants.as_slice().try_into().unwrap(),
    }
}

fn art(day: u8) -> Art {
    let kind = match day {
        0 => None,
        n if n % 2 == 1 => Some(api::Kind::Sleeve),
        _ => Some(api::Kind::Playmat),
    };
    Art {
        kind,
        tier: if day == 24 { "finale" } else { "standard" }.into(),
        title: api::Text {
            de: format!("Synthetische Belohnung {day}"),
            en: format!("Synthetic reward {day}"),
        },
        dimensions: [32, 32],
        mat_framing: (kind == Some(api::Kind::Playmat)).then(|| mat_framing(day)),
        bytes: bytes(day),
    }
}

pub async fn provision(operator: &DatabaseConnection, year: u16) {
    operator
        .execute_unprepared(include_str!("../../../../ops/advent-importer-grants.sql"))
        .await
        .unwrap();
    let db = central_postgres::connect_dev(crate::config(
        "central_advent_importer",
        "CENTRAL_TEST_ADVENT_IMPORT_PASSWORD",
    ))
    .await
    .unwrap();
    let mut import = Import::begin(&db, year, art(0)).await.unwrap();
    for day in 1..=api::DOORS {
        import
            .door(day, &board(day), [hint(day, 1), hint(day, 2)], art(day))
            .await
            .unwrap();
    }
    import.finish().await.unwrap();
}

/// Move only this fixture campaign. The authority still reads its own PostgreSQL clock.
pub async fn elapsed(db: &DatabaseConnection, year: u16, seconds: i32) {
    let n=db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "WITH test_start AS (SELECT date_trunc('second',clock_timestamp()) - $2::integer * interval '1 second' AS origin)
         UPDATE central_identity.advent_campaigns SET starts_at=t.origin,closes_at=t.origin+interval '888 hours'
         FROM test_start t WHERE year=$1",[i16::try_from(year).unwrap().into(),seconds.into()])).await.unwrap();
    assert_eq!(n.rows_affected(), 1);
}
