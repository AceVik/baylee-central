use super::{
    Career, ConnectionTrait, DatabaseTransaction, Error, Fixture, get, statement, unavailable,
};
use central_api::ranked::PoolEntry;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fmt::Write};
mod valuation;
pub(in super::super) use valuation::preview;

struct Pool {
    version: Vec<u8>,
    counts: String,
    copies: i32,
}

impl Pool {
    fn read(text: &str) -> Result<Self, Error> {
        let entries: Vec<PoolEntry> = central_api::decode(text.as_bytes()).map_err(unavailable)?;
        Self::from_entries(&entries).map_err(unavailable)
    }

    fn from_entries(entries: &[PoolEntry]) -> Result<Self, Error> {
        if !central_api::ranked::valid_pool(entries) {
            return Err(Error::Rejected);
        }
        let mut counts = BTreeMap::new();
        let mut digest = Sha256::new();
        digest.update(b"Baylee ranked deck version v1\0");
        let mut copies = 0;
        for entry in entries {
            copies += i32::from(entry.copies);
            let mut key = String::with_capacity(64);
            for byte in entry.card {
                write!(&mut key, "{byte:02x}").map_err(unavailable)?;
            }
            counts.insert(key, entry.copies);
            digest.update(entry.card);
            digest.update(entry.copies.to_be_bytes());
        }
        Ok(Self {
            version: digest.finalize().to_vec(),
            counts: String::from_utf8(central_api::encode(&counts).map_err(unavailable)?)
                .map_err(unavailable)?,
            copies,
        })
    }
}

pub(super) struct Standing {
    version: Vec<u8>,
    pub(super) elo: i32,
    pub(super) career: Career,
    season_games: i32,
}

async fn register(
    tx: &DatabaseTransaction,
    game: &Fixture,
    seat: usize,
    pool: &Pool,
) -> Result<i32, Error> {
    let quote = valuation::inherit(tx, game.accounts[seat], &game.format, pool, None).await?;
    let initial = quote.is_none();
    let elo = quote.map_or(1500, |quote| quote.elo);
    tx.execute_raw(statement(
        "INSERT INTO central_identity.ranked_deck_versions(account_id,format,version,cards,copies,elo,first_season,initial_pool,anchor)
         VALUES ($1,$2,$3,$4::jsonb,$5,$6,$7,$8,CASE WHEN $8 THEN $6 ELSE NULL END)",
        vec![game.accounts[seat].into(),game.format.as_str().into(),pool.version.clone().into(),
        pool.counts.as_str().into(),pool.copies.into(),elo.into(),game.season.into(),initial.into()],
    )).await.map_err(unavailable)?;
    Ok(elo)
}

pub(super) async fn load(
    tx: &DatabaseTransaction,
    game: &Fixture,
    seat: usize,
) -> Result<Standing, Error> {
    let pool = Pool::read(&game.decks[seat])?;
    let row = valuation::known(tx, game.accounts[seat], &game.format, &pool).await?;
    if let Some(row) = row {
        let same_season = get::<sea_orm::prelude::Uuid>(&row, "season_id")? == game.season;
        return Ok(Standing {
            version: pool.version,
            elo: get(&row, "elo")?,
            career: Career::read(&row)?,
            season_games: if same_season {
                get(&row, "season_games")?
            } else {
                0
            },
        });
    }
    let elo = register(tx, game, seat, &pool).await?;
    Ok(Standing {
        version: pool.version,
        elo,
        career: Career::fresh(game.now),
        season_games: 0,
    })
}

pub(super) async fn save(
    tx: &DatabaseTransaction,
    game: &Fixture,
    seat: usize,
    deck: &mut Standing,
    elo: i32,
    mark: i16,
) -> Result<super::api::DeckResult, Error> {
    let place = super::boards::deck_champion_place(
        tx,
        game.season,
        &game.format,
        game.accounts[seat],
        &deck.version,
    )
    .await?;
    let before = super::published(&deck.career, deck.elo, game.now, place)?;
    deck.career.played(mark, elo, game.now)?;
    deck.season_games += 1;
    let mut values = game.key(seat);
    values.extend(deck.career.values()?);
    values.extend([
        elo.into(),
        deck.version.clone().into(),
        deck.season_games.into(),
    ]);
    tx.execute_raw(statement(
        "INSERT INTO central_identity.ranked_deck_ratings
         (season_id,format,account_id,games,wins,losses,draws,first_at,league,division,elo,version,season_games)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)
         ON CONFLICT(season_id,format,account_id,version) DO UPDATE SET games=EXCLUDED.games,
         wins=EXCLUDED.wins,losses=EXCLUDED.losses,draws=EXCLUDED.draws,
         league=EXCLUDED.league,division=EXCLUDED.division,
         elo=EXCLUDED.elo,season_games=EXCLUDED.season_games,last_at=transaction_timestamp()",
        values,
    )).await.map_err(unavailable)?;
    // The very first registered pool bootstraps a cold format. It cannot be
    // exchanged for a new anchor by playing intermediate edited versions.
    tx.execute_raw(statement(
        "UPDATE central_identity.ranked_deck_versions SET elo=$4,
         anchor=CASE WHEN initial_pool AND first_season=$5 THEN $4 ELSE anchor END
         WHERE account_id=$1 AND format=$2 AND version=$3",
        vec![
            game.accounts[seat].into(),
            game.format.as_str().into(),
            deck.version.clone().into(),
            elo.into(),
            game.season.into(),
        ],
    ))
    .await
    .map_err(unavailable)?;
    Ok(super::api::DeckResult {
        before,
        after: super::published(&deck.career, elo, game.now, place)?,
        elo,
        delta: elo - deck.elo,
        placement_games: u8::try_from(deck.career.games().min(10)).map_err(unavailable)?,
    })
}
