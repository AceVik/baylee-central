use super::{Error, statement, unavailable};
use sea_orm::{ConnectionTrait, prelude::Uuid};

pub(super) enum Budget {
    Request,
    Lookup,
}

/// Called with the account row locked; the single row never grows with traffic.
pub(super) async fn charge(
    db: &impl ConnectionTrait,
    me: Uuid,
    budget: Budget,
) -> Result<(), Error> {
    db.execute_raw(statement(
        "INSERT INTO central_identity.social_budgets (account_id) VALUES ($1) ON CONFLICT DO NOTHING",
        vec![me.into()],
    )).await.map_err(unavailable)?;
    db.execute_raw(statement(
        "UPDATE central_identity.social_budgets SET window_at = pg_catalog.clock_timestamp(), requests = 0, lookups = 0
         WHERE account_id = $1 AND window_at <= pg_catalog.clock_timestamp() - interval '1 hour'", vec![me.into()],
    )).await.map_err(unavailable)?;
    let (column, cap) = match budget {
        Budget::Request => ("requests", 60),
        Budget::Lookup => ("lookups", 120),
    };
    let updated = db.execute_raw(statement(
        &format!("UPDATE central_identity.social_budgets SET {column} = {column} + 1 WHERE account_id = $1 AND {column} < $2"),
        vec![me.into(), cap.into()],
    )).await.map_err(unavailable)?;
    if updated.rows_affected() == 1 {
        Ok(())
    } else {
        Err(Error::Busy)
    }
}
