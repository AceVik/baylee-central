use super::{
    Career, ConnectionTrait, DatabaseTransaction, Error, Fixture, Player, get, public_elo, rules,
    statement, unavailable,
};

pub(super) struct Standing {
    pub(super) player: Player,
    pub(super) career: Career,
}

pub(super) async fn load(
    tx: &DatabaseTransaction,
    game: &Fixture,
    seat: usize,
) -> Result<Standing, Error> {
    let row = tx
        .query_one_raw(statement(
            "SELECT * FROM central_identity.ranked_ratings WHERE format=$1 AND account_id=$2
         ORDER BY last_at DESC,season_id DESC LIMIT 1",
            vec![game.format.as_str().into(), game.accounts[seat].into()],
        ))
        .await
        .map_err(unavailable)?;
    let Some(row) = row else {
        return Ok(Standing {
            player: Player {
                score: 1500.0,
                placement: 1500.0,
                allowance: 0.0,
                games: 0,
            },
            career: Career::fresh(game.now),
        });
    };
    let season = get::<sea_orm::prelude::Uuid>(&row, "season_id")?;
    if season != game.season {
        // Freeze every 40-game version once at this account's season cut.
        // Later edits compare directly to these references until the next cut.
        tx.execute_raw(statement(
            "UPDATE central_identity.ranked_deck_versions v SET anchor=d.elo
             FROM central_identity.ranked_deck_ratings d WHERE d.season_id=$1 AND d.format=$2
             AND d.account_id=$3 AND d.season_games>=40 AND
             (v.account_id,v.format,v.version)=(d.account_id,d.format,d.version)",
            vec![
                season.into(),
                game.format.as_str().into(),
                game.accounts[seat].into(),
            ],
        ))
        .await
        .map_err(unavailable)?;
    }
    Ok(Standing {
        player: Player {
            score: get(&row, "score")?,
            placement: get(&row, "placement")?,
            allowance: get(&row, "allowance")?,
            games: get(&row, "games")?,
        },
        career: Career::read(&row)?,
    })
}

pub(super) async fn repeats(tx: &DatabaseTransaction, game: &Fixture) -> Result<i32, Error> {
    let row=tx.query_one_raw(statement(
        "SELECT COUNT(*)::integer AS n FROM (SELECT 1 FROM central_identity.ranked_receipts
         WHERE account_id=$1 AND opponent_id=$2 AND settled_at >= transaction_timestamp() - interval '6 hours'
         LIMIT 4) recent", vec![game.accounts[0].into(),game.accounts[1].into()]
    )).await.map_err(unavailable)?.ok_or(Error::Unavailable)?;
    get(&row, "n")
}

pub(super) async fn evidence(
    tx: &DatabaseTransaction,
    game: &Fixture,
    seat: usize,
    gap: i32,
) -> Result<u8, Error> {
    if gap.abs() < rules::EVIDENCE_GAP {
        return Ok(0);
    }
    // A normal win by a favourite (or a normal loss by an underdog) resets
    // that role's streak. Current opponent is excluded only from evidence,
    // so a normal result against that same opponent still resets the streak.
    let rows=tx.query_all_raw(statement(
        "SELECT opponent_id,opponent_established,elo_gap,outcome FROM central_identity.ranked_receipts
         WHERE account_id=$1 AND format=$2 AND settled_at>=transaction_timestamp()-interval '14 days'
         ORDER BY settled_at DESC,match_id DESC LIMIT 12",
         vec![game.accounts[seat].into(),game.format.as_str().into()],
    )).await.map_err(unavailable)?;
    let mut opponents = std::collections::BTreeSet::new();
    for row in rows {
        let prior_gap: i32 = get(&row, "elo_gap")?;
        let result: i16 = get(&row, "outcome")?;
        if !get::<bool>(&row, "opponent_established")?
            || prior_gap.abs() < rules::EVIDENCE_GAP
            || prior_gap.signum() != gap.signum()
            || result == 0
        {
            continue;
        }
        if i32::from(result).signum() == gap.signum() {
            break;
        }
        let opponent = get::<sea_orm::prelude::Uuid>(&row, "opponent_id")?;
        if opponent != game.accounts[1 - seat] {
            opponents.insert(opponent);
            if opponents.len() == 3 {
                break;
            }
        }
    }
    u8::try_from(opponents.len()).map_err(unavailable)
}

pub(super) async fn save(
    tx: &DatabaseTransaction,
    game: &Fixture,
    seat: usize,
    p: &Standing,
) -> Result<(), Error> {
    let mut values = game.key(seat);
    values.extend(p.career.values()?);
    values.extend([
        public_elo(p.player).into(),
        rules::queue_elo(p.player).into(),
        p.player.score.into(),
        p.player.placement.into(),
        p.player.allowance.into(),
    ]);
    tx.execute_raw(statement(
        "INSERT INTO central_identity.ranked_ratings
         (season_id,format,account_id,games,wins,losses,draws,first_at,league,division,elo,queue_elo,score,placement,allowance)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)
         ON CONFLICT(season_id,format,account_id) DO UPDATE SET games=EXCLUDED.games,
         wins=EXCLUDED.wins,losses=EXCLUDED.losses,draws=EXCLUDED.draws,
         league=EXCLUDED.league,division=EXCLUDED.division,
         elo=EXCLUDED.elo,queue_elo=EXCLUDED.queue_elo,score=EXCLUDED.score,
         placement=EXCLUDED.placement,allowance=EXCLUDED.allowance,last_at=transaction_timestamp()",
         values,
    )).await.map_err(unavailable)?;
    Ok(())
}
