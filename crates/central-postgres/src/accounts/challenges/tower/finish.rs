//! Terminal host delivery advances one owned run in the same locked transaction.

use super::{progress, save, season, season_row, statement, unavailable};
use central_api::{Error, challenges::AttemptState};
use sea_orm::{ConnectionTrait, DatabaseTransaction, QueryResult, prelude::Uuid};

pub(in crate::accounts::challenges) async fn complete(
    db: &DatabaseTransaction,
    me: Uuid,
    attempt: &QueryResult,
) -> Result<(), Error> {
    let attempt_id: Uuid = attempt.try_get("", "id").map_err(unavailable)?;
    let row = db
        .query_one_raw(statement(
            "SELECT id,season_id,progress::text AS state FROM central_identity.tower_runs
         WHERE account_id=$1 AND active_attempt=$2 AND active",
            vec![me.into(), attempt_id.into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Conflict)?;
    let run: Uuid = row.try_get("", "id").map_err(unavailable)?;
    let season_id: Uuid = row.try_get("", "season_id").map_err(unavailable)?;
    let season = season(&season_row(db, *season_id.as_bytes(), false).await?)?;
    let mut state = progress(&row)?;
    let game: Option<Uuid> = attempt.try_get("", "game_id").map_err(unavailable)?;
    let result = if game.is_none() || season.withdrawn {
        AttemptState::Void
    } else {
        super::super::state(attempt)?
    };
    let turn: Option<i32> = attempt.try_get("", "turn").map_err(unavailable)?;
    let ms: Option<i64> = attempt.try_get("", "milliseconds").map_err(unavailable)?;
    let end = db.query_one_raw(statement(
        "SELECT floor(extract(epoch FROM finished_at))::bigint AS finished FROM central_identity.challenge_attempts WHERE id=$1",
        vec![attempt_id.into()])).await.map_err(unavailable)?.ok_or(Error::Unavailable)?;
    let finished: i64 = end.try_get("", "finished").map_err(unavailable)?;
    let admitted: bool = attempt.try_get("", "live").map_err(unavailable)?;
    let live = admitted && season.theme.period.live_at(finished);
    state.finish(
        result,
        u32::try_from(turn.unwrap_or(0)).map_err(unavailable)?,
        ms.map(u64::try_from).transpose().map_err(unavailable)?,
        live,
    )?;
    save(db, *run.as_bytes(), &state, None).await?;
    if result == AttemptState::Won {
        db.execute_raw(statement(
            "INSERT INTO central_identity.tower_accomplishments(account_id,season_id,floor,flawless)
             VALUES($1,$2,$3,$4) ON CONFLICT(account_id,season_id,floor) DO UPDATE
             SET flawless=central_identity.tower_accomplishments.flawless OR EXCLUDED.flawless",
            vec![me.into(), season_id.into(), i16::from(state.cleared).into(), (state.lives==3).into()]))
            .await.map_err(unavailable)?;
        if state.competitive {
            db.execute_raw(statement(
                "INSERT INTO central_identity.tower_scores(run_id,account_id,season_id,floors,losses,turns,milliseconds)
                 VALUES($1,$2,$3,$4,$5,$6,$7) ON CONFLICT(run_id) DO UPDATE SET floors=EXCLUDED.floors,
                 losses=EXCLUDED.losses,turns=EXCLUDED.turns,milliseconds=EXCLUDED.milliseconds",
                vec![run.into(), me.into(), season_id.into(), i16::from(state.cleared).into(),
                    i16::from(3-state.lives).into(), i64::from(state.turns).into(),
                    state.milliseconds.map(i64::try_from).transpose().map_err(unavailable)?.into()]))
                .await.map_err(unavailable)?;
        }
    }
    Ok(())
}
