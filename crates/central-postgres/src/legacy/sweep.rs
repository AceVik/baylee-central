//! The legacy store's deadline and sweep (`docs/operations.md` §"The legacy
//! store"), under the migrator login: the operator sets the date the owner
//! chose, and the daily sweep deletes the rows of closed moves at once and,
//! once the deadline has passed, every row left and every legacy reservation
//! that never enrolled, which gives their names up.
use super::statement;
use central_api::Error;
use sea_orm::{ConnectionTrait, DatabaseConnection, TransactionTrait};

/// What one sweep deleted.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct LegacySwept {
    /// Rows whose move had closed (their close deletes them; this catches a
    /// close whose deletion was lost).
    pub moved: u64,
    /// Rows left after the deadline.
    pub expired: u64,
    /// Legacy reservations that never enrolled, after the deadline.
    pub reservations: u64,
}

/// Set the deadline to the end of `date` (`YYYY-MM-DD`) in Europe/Berlin,
/// the last day a legacy sign-in is taken, or clear it with `None`. Answers
/// it in Unix seconds.
///
/// # Errors
/// Rejects anything but a valid date, and reports unavailable storage.
pub async fn set_legacy_deadline(
    db: &DatabaseConnection,
    date: Option<&str>,
) -> Result<Option<i64>, Error> {
    if let Some(date) = date {
        let bytes = date.as_bytes();
        let shaped = bytes.len() == 10
            && bytes.iter().enumerate().all(|(index, byte)| match index {
                4 | 7 => *byte == b'-',
                _ => byte.is_ascii_digit(),
            });
        if !shaped {
            return Err(Error::Rejected);
        }
    }
    db.query_one_raw(statement(
        "UPDATE central_legacy.settings
         SET deadline = (($1::date + 1)::timestamp) AT TIME ZONE 'Europe/Berlin'
         RETURNING floor(extract(epoch FROM deadline))::bigint AS deadline",
        vec![date.map(str::to_owned).into()],
    ))
    .await
    .map_err(|_| Error::Rejected)?
    .ok_or(Error::Unavailable)?
    .try_get("", "deadline")
    .map_err(|_| Error::Unavailable)
}

/// Delete what the store no longer needs; one transaction.
///
/// # Errors
/// Reports unavailable storage; nothing is deleted then.
pub async fn sweep_legacy(db: &DatabaseConnection) -> Result<LegacySwept, Error> {
    let tx = db.begin().await.map_err(|_| Error::Unavailable)?;
    let deleted = |sql: &'static str| {
        let tx = &tx;
        async move {
            tx.execute_raw(statement(sql, vec![]))
                .await
                .map(|done| done.rows_affected())
                .map_err(|_| Error::Unavailable)
        }
    };
    let moved = deleted(
        "DELETE FROM central_legacy.accounts l USING central_identity.accounts a
         WHERE a.legacy_id = l.legacy_id AND a.legacy_closed",
    )
    .await?;
    let passed: bool = tx
        .query_one_raw(statement(
            "SELECT deadline IS NOT NULL AND deadline <= pg_catalog.clock_timestamp() AS passed
             FROM central_legacy.settings",
            vec![],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::Unavailable)?
        .try_get("", "passed")
        .map_err(|_| Error::Unavailable)?;
    let mut swept = LegacySwept {
        moved,
        ..LegacySwept::default()
    };
    if passed {
        swept.expired = deleted("DELETE FROM central_legacy.accounts").await?;
        swept.reservations = deleted(
            "DELETE FROM central_identity.accounts
             WHERE source = 'legacy' AND credential IS NULL AND NOT legacy_closed",
        )
        .await?;
    }
    tx.commit().await.map_err(|_| Error::Unavailable)?;
    Ok(swept)
}
