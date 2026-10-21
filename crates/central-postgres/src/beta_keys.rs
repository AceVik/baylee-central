//! Issuing beta keys, an operator task under the migrator login
//! (`central-service --issue-beta-keys-stdin`). The runtime role may only spend
//! a key's uses; it can neither issue nor revoke one.
use crate::StoreError;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, TransactionTrait};

/// Most keys one run issues.
pub const MAX_BETA_KEYS: usize = 100;
/// Most uses one key admits.
pub const MAX_BETA_KEY_USES: u32 = 1_000;
/// Longest lifetime of a key, in days.
pub const MAX_BETA_KEY_DAYS: u32 = 3_650;
/// Longest note, in characters.
pub const MAX_BETA_KEY_NOTE: usize = 200;

/// The keys of one issuing run, validated before anything is written.
pub struct BetaKeyBatch<'a> {
    digests: &'a [[u8; 32]],
    uses: i32,
    expires_days: Option<i32>,
    note: &'a str,
}

impl<'a> BetaKeyBatch<'a> {
    /// Bound one run: 1 to [`MAX_BETA_KEYS`] distinct digests, 1 to
    /// [`MAX_BETA_KEY_USES`] uses each, an optional lifetime of 1 to
    /// [`MAX_BETA_KEY_DAYS`] days and a note of at most [`MAX_BETA_KEY_NOTE`]
    /// characters without control characters.
    ///
    /// # Errors
    /// [`StoreError::InvalidConfig`] for anything out of bounds; no value is echoed.
    pub fn new(
        digests: &'a [[u8; 32]],
        uses: u32,
        expires_days: Option<u32>,
        note: &'a str,
    ) -> Result<Self, StoreError> {
        let distinct = digests
            .iter()
            .enumerate()
            .all(|(index, digest)| !digests[..index].contains(digest));
        if !(1..=MAX_BETA_KEYS).contains(&digests.len())
            || !distinct
            || !(1..=MAX_BETA_KEY_USES).contains(&uses)
            || expires_days.is_some_and(|days| !(1..=MAX_BETA_KEY_DAYS).contains(&days))
            || note.chars().count() > MAX_BETA_KEY_NOTE
            || note.chars().any(char::is_control)
        {
            return Err(StoreError::InvalidConfig);
        }
        Ok(Self {
            digests,
            uses: i32::try_from(uses).map_err(|_| StoreError::InvalidConfig)?,
            expires_days: expires_days
                .map(i32::try_from)
                .transpose()
                .map_err(|_| StoreError::InvalidConfig)?,
            note,
        })
    }

    /// Store every digest in one transaction: all keys of the run exist, or none.
    ///
    /// # Errors
    /// [`StoreError::Unavailable`] for any database failure, a duplicate digest
    /// among them, without values or diagnostics.
    pub async fn issue(&self, db: &DatabaseConnection) -> Result<(), StoreError> {
        let transaction = db.begin().await.map_err(|_| StoreError::Unavailable)?;
        for digest in self.digests {
            transaction
                .execute_raw(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    "INSERT INTO central_identity.beta_keys (digest, uses_left, created_at, expires_at, note)
                     SELECT $1, $2, t, t + pg_catalog.make_interval(days => $3), $4
                     FROM (SELECT pg_catalog.clock_timestamp() AS t) clock",
                    [
                        digest.to_vec().into(),
                        self.uses.into(),
                        self.expires_days.into(),
                        self.note.into(),
                    ],
                ))
                .await
                .map_err(|_| StoreError::Unavailable)?;
        }
        transaction
            .commit()
            .await
            .map_err(|_| StoreError::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_run_is_bounded_before_anything_is_written() {
        let digests: Vec<[u8; 32]> = (0..=100_u8).map(|i| [i; 32]).collect();
        assert!(BetaKeyBatch::new(&digests[..1], 1, None, "").is_ok());
        assert!(BetaKeyBatch::new(&digests[..100], 1_000, Some(3_650), &"n".repeat(200)).is_ok());
        assert!(BetaKeyBatch::new(&digests[..1], 5, Some(1), "Spieltest Oktober — ä").is_ok());
        for (count, uses, days, note) in [
            (0, 1, None, String::new()),
            (101, 1, None, String::new()),
            (1, 0, None, String::new()),
            (1, 1_001, None, String::new()),
            (1, 1, Some(0), String::new()),
            (1, 1, Some(3_651), String::new()),
            (1, 1, None, "n".repeat(201)),
            (1, 1, None, "line\nbreak".into()),
            (1, 1, None, "escape\u{1b}[2J".into()),
        ] {
            assert!(
                BetaKeyBatch::new(&digests[..count], uses, days, &note).is_err(),
                "{count} {uses} {days:?}"
            );
        }
        assert!(BetaKeyBatch::new(&[[7; 32], [7; 32]], 1, None, "").is_err());
    }
}
