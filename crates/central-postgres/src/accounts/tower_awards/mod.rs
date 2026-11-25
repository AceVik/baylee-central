//! Scoped verified fact ingestion; no gateway credential implies issuer authority.
use super::{AccountStore, Error, Session, Token, statement, uuid};
use central_api::tower_awards::{Delivery, Disposition, Fact, Progress, Receipt};
use sea_orm::{ConnectionTrait, DatabaseConnection, QueryResult, TransactionTrait, prelude::Uuid};

mod read;
mod write;

const MAX_DUELS: u64 = 1_000_000;

fn unavailable(_: impl Sized) -> Error {
    Error::Unavailable
}
fn get<T: sea_orm::TryGetable>(row: &QueryResult, name: &str) -> Result<T, Error> {
    row.try_get("", name).map_err(unavailable)
}

/// Offline provisioning through a separately held migration/operator connection.
/// Runtime grants cannot insert issuers or change their origin, digest or enabled state.
/// The caller transports the independent reviewed Token through stdin/memory only.
/// # Errors
/// Invalid origin, duplicate origin/credential or insufficient database privileges.
pub async fn provision_tower_award_issuer(
    db: &DatabaseConnection,
    origin: &str,
    credential: &Token,
    development: bool,
) -> Result<[u8; 16], Error> {
    central_api::gateway::validate_origin(origin, development)?;
    let row = db
        .query_one_raw(statement(
            "INSERT INTO central_identity.tower_award_issuers(origin,credential_digest)
         VALUES($1,$2) ON CONFLICT DO NOTHING RETURNING id",
            vec![origin.into(), credential.digest().to_vec().into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Conflict)?;
    Ok(*get::<Uuid>(&row, "id")?.as_bytes())
}

async fn issuer(
    db: &impl ConnectionTrait,
    origin: &str,
    credential: &Token,
) -> Result<Uuid, Error> {
    let row = db.query_one_raw(statement(
        "UPDATE central_identity.tower_award_issuers SET last_used_at=pg_catalog.clock_timestamp()
         WHERE origin=$1 AND credential_digest=$2 AND enabled RETURNING id",
        vec![origin.into(), credential.digest().to_vec().into()],
    )).await.map_err(unavailable)?.ok_or(Error::Rejected)?;
    get(&row, "id")
}

pub(in crate::accounts) async fn totals(
    db: &impl ConnectionTrait,
    account: Uuid,
) -> Result<Progress, Error> {
    let row = db.query_one_raw(statement(
        "SELECT duels,victories,clears,flawless FROM central_identity.tower_award_progress WHERE account_id=$1",
        vec![account.into()],
    )).await.map_err(unavailable)?;
    let Some(row) = row else {
        return Ok(Progress::default());
    };
    let count = |name| u64::try_from(get::<i64>(&row, name)?).map_err(unavailable);
    let progress = Progress {
        duels: count("duels")?,
        victories: count("victories")?,
        clears: count("clears")?,
        flawless: count("flawless")?,
    };
    progress.validate().map_err(unavailable)?;
    Ok(progress)
}
