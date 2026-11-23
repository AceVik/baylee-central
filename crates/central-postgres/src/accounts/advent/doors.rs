//! Half-open UTC instants come from the privately provisioned campaign and Central's clock.
use super::{DatabaseTransaction, Error, Uuid, api, art, col, statement, unavailable};
use sea_orm::ConnectionTrait;

fn year_num(year: u16) -> Result<i16, Error> {
    i16::try_from(year).map_err(|_| Error::Rejected)
}
const RELEASED:&str=" FROM central_identity.advent_doors d JOIN central_identity.advent_campaigns c USING(year)
    WHERE d.year=$1 AND d.door=$2 AND statement_timestamp()>=c.starts_at+(d.door-1)*interval '24 hours'
    AND statement_timestamp()<c.closes_at";

pub(super) async fn current(db: &DatabaseTransaction, me: Uuid) -> Result<api::Response, Error> {
    let row=db.query_one_raw(statement("SELECT year FROM central_identity.advent_campaigns WHERE closes_at>transaction_timestamp()
        ORDER BY starts_at,year LIMIT 1",vec![])).await.map_err(unavailable)?;
    if let Some(row) = row {
        calendar(
            db,
            me,
            u16::try_from(col::<i16>(&row, "year")?).map_err(unavailable)?,
        )
        .await
    } else {
        Ok(api::Response::NoCalendar {})
    }
}

pub(super) async fn calendar(
    db: &DatabaseTransaction,
    me: Uuid,
    year: u16,
) -> Result<api::Response, Error> {
    let now=db.query_one_raw(statement("SELECT floor(extract(epoch FROM c.starts_at))::bigint AS starts,
        floor(extract(epoch FROM c.closes_at))::bigint AS closes, floor(EXTRACT(epoch FROM transaction_timestamp()))::bigint AS now,
        CASE WHEN transaction_timestamp()>=c.starts_at AND transaction_timestamp()<c.closes_at THEN c.hero_id END AS hero
        FROM central_identity.advent_campaigns c WHERE c.year=$1",vec![year_num(year)?.into()])).await.map_err(unavailable)?.ok_or(Error::Rejected)?;
    let rows=db.query_all_raw(statement("SELECT d.door,s.account_id IS NOT NULL AS solved,e.account_id IS NOT NULL AS claimed
        FROM central_identity.advent_doors d LEFT JOIN central_identity.advent_solutions s ON s.year=d.year AND s.door=d.door AND s.account_id=$2
        LEFT JOIN central_identity.advent_entitlements e ON e.account_id=s.account_id AND e.year=s.year AND e.door=s.door
        WHERE d.year=$1 ORDER BY d.door",vec![year_num(year)?.into(),me.into()])).await.map_err(unavailable)?;
    if rows.len() != usize::from(api::DOORS) {
        return Err(Error::Unavailable);
    }
    let (starts_at, closes_at, server) = (
        col::<i64>(&now, "starts")?,
        col::<i64>(&now, "closes")?,
        col::<i64>(&now, "now")?,
    );
    let mut doors = Vec::with_capacity(rows.len());
    for row in rows {
        let door = u8::try_from(col::<i16>(&row, "door")?).map_err(unavailable)?;
        let enables_at = starts_at + i64::from(door - 1) * 86_400;
        let state = if col(&row, "claimed")? {
            api::DoorState::Claimed
        } else if col(&row, "solved")? {
            api::DoorState::Solved
        } else if server < enables_at {
            api::DoorState::Locked
        } else if server >= closes_at {
            api::DoorState::Expired
        } else {
            api::DoorState::Open
        };
        doors.push(api::Door {
            door,
            state,
            enables_at,
        });
    }
    let hero = if let Some(id) = col::<Option<Uuid>>(&now, "hero")? {
        Some(art::metadata(db, id).await?)
    } else {
        None
    };
    Ok(api::Response::Calendar {
        year,
        starts_at,
        closes_at,
        now: server,
        doors,
        hero,
    })
}
async fn released(
    db: &DatabaseTransaction,
    year: u16,
    door: u8,
    fields: &str,
) -> Result<sea_orm::QueryResult, Error> {
    db.query_one_raw(statement(
        &format!("SELECT {fields}{RELEASED}"),
        vec![year_num(year)?.into(), i16::from(door).into()],
    ))
    .await
    .map_err(unavailable)?
    .ok_or(Error::Rejected)
}
async fn board(
    db: &DatabaseTransaction,
    year: u16,
    door: u8,
) -> Result<api::puzzle::Puzzle, Error> {
    let row = released(db, year, door, "d.puzzle").await?;
    central_api::decode(col::<String>(&row, "puzzle")?.as_bytes()).map_err(unavailable)
}
pub(super) async fn puzzle(
    db: &DatabaseTransaction,
    year: u16,
    door: u8,
) -> Result<api::Response, Error> {
    Ok(api::Response::Puzzle {
        year,
        door,
        puzzle: board(db, year, door).await?,
    })
}
pub(super) async fn hint(
    db: &DatabaseTransaction,
    year: u16,
    door: u8,
    number: u8,
) -> Result<api::Response, Error> {
    let fields = match number {
        1 => "d.hint1_de AS de,d.hint1_en AS en",
        2 => "d.hint2_de AS de,d.hint2_en AS en",
        _ => return Err(Error::Rejected),
    };
    let row = released(db, year, door, fields).await?;
    Ok(api::Response::Hint {
        number,
        text: api::Text {
            de: col(&row, "de")?,
            en: col(&row, "en")?,
        },
    })
}
pub(super) async fn submit(
    db: &DatabaseTransaction,
    me: Uuid,
    year: u16,
    door: u8,
    line: &api::puzzle::Line,
) -> Result<api::Response, Error> {
    let won = board(db, year, door).await?.won(line)?;
    if won {
        db.execute_raw(statement(
            "INSERT INTO central_identity.advent_solutions(account_id,year,door)
            VALUES($1,$2,$3) ON CONFLICT DO NOTHING",
            vec![me.into(), year_num(year)?.into(), i16::from(door).into()],
        ))
        .await
        .map_err(unavailable)?;
        super::inventory::grant(db, me, year, door).await?;
    }
    Ok(api::Response::Submission { won })
}
