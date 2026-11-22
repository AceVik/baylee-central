//! Settlement under the verified host's locked match and its two exclusive seats.
//! All four ratings and both evidence receipts live in the caller's transaction.

use super::rules::{self, Outcome, Player, Rank};
use super::{Error, api, boards, column as get, statement, unavailable};
use sea_orm::{
    ConnectionTrait, DatabaseTransaction, QueryResult, Value,
    prelude::{TimeDateTimeWithTimeZone, Uuid},
};

mod decks;
mod players;
pub(super) use decks::preview;

struct Fixture {
    id: Uuid,
    season: Uuid,
    format: String,
    accounts: [Uuid; 2],
    decks: [String; 2],
    now: TimeDateTimeWithTimeZone,
}

impl Fixture {
    fn key(&self, seat: usize) -> Vec<Value> {
        vec![
            self.season.into(),
            self.format.as_str().into(),
            self.accounts[seat].into(),
        ]
    }
}

struct Career {
    wins: i32,
    losses: i32,
    draws: i32,
    first: TimeDateTimeWithTimeZone,
    rank: Rank,
    last: TimeDateTimeWithTimeZone,
}

impl Career {
    fn fresh(now: TimeDateTimeWithTimeZone) -> Self {
        Self {
            wins: 0,
            losses: 0,
            draws: 0,
            first: now,
            last: now,
            rank: Rank {
                league: 0,
                division: 0,
            },
        }
    }

    fn read(row: &QueryResult) -> Result<Self, Error> {
        Ok(Self {
            wins: get(row, "wins")?,
            losses: get(row, "losses")?,
            draws: get(row, "draws")?,
            first: get(row, "first_at")?,
            last: get(row, "last_at")?,
            rank: Rank {
                league: i32::from(get::<i16>(row, "league")?),
                division: i32::from(get::<i16>(row, "division")?),
            },
        })
    }

    fn games(&self) -> i32 {
        self.wins + self.losses + self.draws
    }

    fn played(&mut self, mark: i16, elo: i32, now: TimeDateTimeWithTimeZone) -> Result<(), Error> {
        self.games().checked_add(1).ok_or(Error::Unavailable)?;
        match mark {
            1 => self.wins += 1,
            -1 => self.losses += 1,
            0 => self.draws += 1,
            _ => return Err(Error::Unavailable),
        }
        self.rank = rules::rank(
            elo,
            self.games(),
            (now - self.first).whole_days(),
            0,
            self.rank,
        );
        self.last = now;
        Ok(())
    }

    fn values(&self) -> Result<Vec<Value>, Error> {
        Ok(vec![
            self.games().into(),
            self.wins.into(),
            self.losses.into(),
            self.draws.into(),
            self.first.into(),
            i16::try_from(self.rank.league.max(0))
                .map_err(unavailable)?
                .into(),
            i16::try_from(self.rank.division)
                .map_err(unavailable)?
                .into(),
        ])
    }
}

fn public_elo(player: Player) -> i32 {
    rules::rounded(player.score)
}

fn published(
    career: &Career,
    elo: i32,
    now: TimeDateTimeWithTimeZone,
    champion: Option<u8>,
) -> Result<api::Rank, Error> {
    let rank = rules::rank(
        elo,
        career.games(),
        (now - career.first).whole_days(),
        (now - career.last).whole_days(),
        career.rank,
    );
    let league = if champion.is_some() {
        10
    } else {
        i16::try_from(rank.league).map_err(unavailable)?
    };
    boards::wire_rank(
        league,
        i16::try_from(rank.division).map_err(unavailable)?,
        career.games(),
        champion,
    )
}

fn serialized(rank: api::Rank) -> Result<String, Error> {
    String::from_utf8(central_api::encode(&rank).map_err(unavailable)?).map_err(unavailable)
}

/// Read the immutable scores saved with the engine result, even after later matches.
pub(super) async fn receipt(
    tx: &DatabaseTransaction,
    match_id: Uuid,
    account: Uuid,
) -> Result<(i32, i32, api::Rank, api::Rank, api::DeckResult), Error> {
    let row=tx.query_one_raw(statement(
        "SELECT elo,delta,before_rank::text,after_rank::text,deck_result::text FROM central_identity.ranked_receipts
         WHERE match_id=$1 AND account_id=$2",vec![match_id.into(),account.into()],
    )).await.map_err(unavailable)?.ok_or(Error::Unavailable)?;
    let read =
        |name| central_api::decode(get::<String>(&row, name)?.as_bytes()).map_err(unavailable);
    Ok((
        get(&row, "elo")?,
        get(&row, "delta")?,
        read("before_rank")?,
        read("after_rank")?,
        central_api::decode(get::<String>(&row, "deck_result")?.as_bytes()).map_err(unavailable)?,
    ))
}

pub(super) async fn settle(
    tx: &DatabaseTransaction,
    match_id: Uuid,
    outcome: i16,
) -> Result<(), Error> {
    let (result, marks) = match outcome {
        0 => (Outcome::Draw, [0, 0]),
        1 => (Outcome::FirstWin, [1, -1]),
        2 => (Outcome::SecondWin, [-1, 1]),
        _ => return Err(Error::Rejected),
    };
    let row=tx.query_one_raw(statement(
        "SELECT season_id,format,account_a,account_b,deck_a::text,deck_b::text,
         transaction_timestamp() AS now FROM central_identity.ranked_matches WHERE id=$1 FOR UPDATE",
        vec![match_id.into()],
    )).await.map_err(unavailable)?.ok_or(Error::Rejected)?;
    let fixture = Fixture {
        id: match_id,
        season: get(&row, "season_id")?,
        format: get(&row, "format")?,
        accounts: [get(&row, "account_a")?, get(&row, "account_b")?],
        decks: [get(&row, "deck_a")?, get(&row, "deck_b")?],
        now: get(&row, "now")?,
    };
    boards::publish(tx, fixture.season).await?;
    let mut players = [
        players::load(tx, &fixture, 0).await?,
        players::load(tx, &fixture, 1).await?,
    ];
    let prior = players.each_ref().map(|p| p.player);
    let elos = prior.map(public_elo);
    let gap = elos[0] - elos[1];
    let upsets = [
        players::evidence(tx, &fixture, 0, gap).await?,
        players::evidence(tx, &fixture, 1, -gap).await?,
    ];
    let repeats = players::repeats(tx, &fixture).await?;
    let next = rules::settle(prior, result, upsets, repeats);
    let mut decks = [
        decks::load(tx, &fixture, 0).await?,
        decks::load(tx, &fixture, 1).await?,
    ];
    let old_deck = decks.each_ref().map(|d| d.elo);
    for seat in 0..2 {
        let other = 1 - seat;
        let seat_gap = elos[seat] - elos[other];
        let seat_result = match marks[seat] {
            1 => Outcome::FirstWin,
            -1 => Outcome::SecondWin,
            _ => Outcome::Draw,
        };
        let deck_elo = rules::deck_after_match(
            old_deck[seat],
            old_deck[other],
            seat_gap,
            seat_result,
            decks[seat].career.games(),
        );
        let deck_result =
            decks::save(tx, &fixture, seat, &mut decks[seat], deck_elo, marks[seat]).await?;
        let player = &mut players[seat];
        let champion =
            boards::champion_place(tx, fixture.season, &fixture.format, fixture.accounts[seat])
                .await?;
        let before = published(&player.career, elos[seat], fixture.now, champion)?;
        player.player = next[seat];
        player
            .career
            .played(marks[seat], public_elo(next[seat]), fixture.now)?;
        players::save(tx, &fixture, seat, player).await?;
        let after = published(
            &player.career,
            public_elo(next[seat]),
            fixture.now,
            champion,
        )?;
        tx.execute_raw(statement(
            "INSERT INTO central_identity.ranked_receipts
            (match_id,account_id,format,opponent_id,opponent_established,elo_gap,outcome,delta,elo,before_rank,after_rank,deck_result)
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10::jsonb,$11::jsonb,$12::jsonb)",
            vec![fixture.id.into(),fixture.accounts[seat].into(),fixture.format.as_str().into(),
            fixture.accounts[other].into(),(prior[other].games >= rules::PLACEMENT_GAMES).into(),
            seat_gap.into(),marks[seat].into(),(public_elo(next[seat])-elos[seat]).into(),
            public_elo(next[seat]).into(),serialized(before)?.into(),serialized(after)?.into(),
            String::from_utf8(central_api::encode(&deck_result)?).map_err(unavailable)?.into()],
        )).await.map_err(unavailable)?;
    }
    Ok(())
}
