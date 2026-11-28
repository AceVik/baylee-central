//! Stored ranks under the rank rule of the day, once the rule changed: on
//! 28.11.2026 leagues and divisions came to follow the Elo alone (owner, "wie
//! beim Schach oder League of Legends"). A record keeps its Elo, games and
//! dates; only its stored league and division are written again, with the
//! inputs its last settlement had (its age then, no idleness), so the result
//! is what that settlement would store today. The badge and cosmetic
//! triggers then grant what a newly reached division earns, as a settlement
//! would; what was earned before stays.

use super::rules;
use super::{column, statement, unavailable};
use central_api::Error;
use sea_orm::{
    ConnectionTrait, DatabaseConnection, QueryResult, TransactionTrait, Value,
    prelude::{TimeDateTimeWithTimeZone, Uuid},
};

/// One stored rank the rule places elsewhere.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rerank {
    /// `player` or `deck`.
    pub track: &'static str,
    /// The format's storage key.
    pub format: String,
    /// The record's season.
    pub season: Uuid,
    /// The record's account.
    pub account: Uuid,
    /// The record's public Elo.
    pub elo: i32,
    /// Its confirmed games.
    pub games: i32,
    /// League and division as stored.
    pub before: (i16, i16),
    /// League and division under the rule.
    pub after: (i16, i16),
}

impl Rerank {
    /// Whether the rule places the record higher than it was stored: a
    /// league up, or the same league and a better (smaller) division.
    #[must_use]
    pub const fn promoted(&self) -> bool {
        self.after.0 > self.before.0
            || (self.after.0 == self.before.0 && self.after.1 < self.before.1)
    }
}

/// The two tables of ranked records: their track and their key beyond
/// season, format and account.
const TABLES: [(&str, &str, &str); 2] = [
    ("player", "ranked_ratings", ""),
    ("deck", "ranked_deck_ratings", ",version"),
];

fn stored(row: &QueryResult) -> Result<(i16, i16), Error> {
    Ok((column(row, "league")?, column(row, "division")?))
}

/// The rank the rule gives a record, as a settlement stores it.
fn ruled(row: &QueryResult) -> Result<(i16, i16), Error> {
    let first: TimeDateTimeWithTimeZone = column(row, "first_at")?;
    let last: TimeDateTimeWithTimeZone = column(row, "last_at")?;
    let rank = rules::rank(
        column(row, "elo")?,
        column(row, "games")?,
        (last - first).whole_days(),
        0,
    );
    Ok((
        i16::try_from(rank.league.max(0)).map_err(unavailable)?,
        i16::try_from(rank.division).map_err(unavailable)?,
    ))
}

/// Every placed record whose stored rank the rule places elsewhere, read
/// under lock so no settlement writes between; with `apply` they are written
/// in the same transaction, else nothing is (a dry run).
///
/// # Errors
/// [`Error::Unavailable`] when the database fails; then nothing is written.
pub async fn rerank(db: &DatabaseConnection, apply: bool) -> Result<Vec<Rerank>, Error> {
    let txn = db.begin().await.map_err(unavailable)?;
    let mut changes = Vec::new();
    for (track, table, key) in TABLES {
        let rows = txn
            .query_all_raw(statement(
                &format!(
                    "SELECT season_id,format,account_id{key},elo,games,first_at,last_at,league,division
                     FROM central_identity.{table} WHERE games >= $1
                     ORDER BY season_id,format,account_id{key} FOR UPDATE"
                ),
                vec![rules::PLACED_AFTER.into()],
            ))
            .await
            .map_err(unavailable)?;
        for row in &rows {
            let before = stored(row)?;
            let after = ruled(row)?;
            if before == after {
                continue;
            }
            let season: Uuid = column(row, "season_id")?;
            let format: String = column(row, "format")?;
            let account: Uuid = column(row, "account_id")?;
            if apply {
                let mut values: Vec<Value> = vec![
                    after.0.into(),
                    after.1.into(),
                    season.into(),
                    format.as_str().into(),
                    account.into(),
                ];
                let version = if key.is_empty() {
                    ""
                } else {
                    values.push(column::<Vec<u8>>(row, "version")?.into());
                    " AND version=$6"
                };
                txn.execute_raw(statement(
                    &format!(
                        "UPDATE central_identity.{table} SET league=$1,division=$2
                         WHERE season_id=$3 AND format=$4 AND account_id=$5{version}"
                    ),
                    values,
                ))
                .await
                .map_err(unavailable)?;
            }
            changes.push(Rerank {
                track,
                format,
                season,
                account,
                elo: column(row, "elo")?,
                games: column(row, "games")?,
                before,
                after,
            });
        }
    }
    if apply {
        txn.commit().await.map_err(unavailable)?;
    } else {
        txn.rollback().await.map_err(unavailable)?;
    }
    Ok(changes)
}
