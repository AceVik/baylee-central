//! Streaming transactional import of Scryfall card printings and localized
//! search names into `central_identity.{card_printings, card_search_names}`.

use std::fmt::Write as _;

use sea_orm::{
    ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, Statement,
    TransactionTrait, Value, prelude::Uuid,
};

use crate::StoreError;

/// One Scryfall printing row prepared for catalog ingestion.
pub struct ImportPrinting {
    /// Printing UUID bytes.
    pub scryfall_id: [u8; 16],
    /// Oracle identity UUID bytes.
    pub oracle_id: [u8; 16],
    /// Canonical English card name.
    pub en_name: String,
    /// Localized printing name, if any.
    pub printed_name: Option<String>,
    /// Scryfall language code (`en`, `de`, ...).
    pub lang: String,
    /// Lowercase set code.
    pub set_code: String,
    /// English set name.
    pub set_name: String,
    /// Collector number.
    pub collector_number: String,
    /// Canonical `YYYY-MM-DD` release date.
    pub released_at: String,
    /// Bitmask of finish flags (`0..=7`).
    pub finishes: i16,
    /// Whether the printing belongs to Universes Beyond.
    pub is_ub: bool,
    /// Whether Scryfall has a usable scan.
    pub has_image: bool,
}

/// One localized card search entry prepared for catalog ingestion.
pub struct ImportSearchName {
    /// Oracle identity UUID bytes.
    pub oracle_id: [u8; 16],
    /// Scryfall language code (`en`, `de`, ...).
    pub lang: String,
    /// Display name in `lang`.
    pub name: String,
    /// Canonical English card name.
    pub en_name: String,
}

/// Active transactional card catalog import handle.
pub struct CardsImport {
    tx: DatabaseTransaction,
    /// Number of printings written.
    pub printings: usize,
    /// Number of search-name batches submitted.
    pub search_names: usize,
}

impl CardsImport {
    /// Begins an atomic catalog replacement (`TRUNCATE` + streaming `INSERT`s).
    ///
    /// # Errors
    /// Returns [`StoreError::Unavailable`] if the transaction or truncate fails.
    pub async fn begin(db: &DatabaseConnection) -> Result<Self, StoreError> {
        let tx = db.begin().await.map_err(|_| StoreError::Unavailable)?;
        tx.execute_unprepared(
            "TRUNCATE central_identity.card_printings, central_identity.card_search_names",
        )
        .await
        .map_err(|_| StoreError::Unavailable)?;
        Ok(Self {
            tx,
            printings: 0,
            search_names: 0,
        })
    }

    /// Inserts a batch of printings (at most ~500 rows per call).
    ///
    /// # Errors
    /// Returns [`StoreError::Unavailable`] if PostgreSQL rejects the batch.
    pub async fn insert_printings(&mut self, rows: &[ImportPrinting]) -> Result<(), StoreError> {
        if rows.is_empty() {
            return Ok(());
        }
        let mut sql = String::from(
            "INSERT INTO central_identity.card_printings (
                scryfall_id, oracle_id, en_name, printed_name, lang,
                set_code, set_name, collector_number, released_at, finishes,
                is_ub, has_image
             ) VALUES ",
        );
        let mut args: Vec<Value> = Vec::with_capacity(rows.len() * 12);
        for (i, row) in rows.iter().enumerate() {
            if i > 0 {
                sql.push(',');
            }
            let base = i * 12;
            let _ = write!(
                sql,
                "(${},${},${},${},${},${},${},${},pg_catalog.to_date(${},'YYYY-MM-DD'),${},${},${})",
                base + 1,
                base + 2,
                base + 3,
                base + 4,
                base + 5,
                base + 6,
                base + 7,
                base + 8,
                base + 9,
                base + 10,
                base + 11,
                base + 12,
            );
            args.push(Uuid::from_bytes(row.scryfall_id).into());
            args.push(Uuid::from_bytes(row.oracle_id).into());
            args.push(row.en_name.clone().into());
            args.push(row.printed_name.clone().into());
            args.push(row.lang.clone().into());
            args.push(row.set_code.clone().into());
            args.push(row.set_name.clone().into());
            args.push(row.collector_number.clone().into());
            args.push(row.released_at.clone().into());
            args.push(row.finishes.into());
            args.push(row.is_ub.into());
            args.push(row.has_image.into());
        }
        sql.push_str(" ON CONFLICT (scryfall_id) DO NOTHING");
        self.tx
            .execute_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                &sql,
                args,
            ))
            .await
            .map_err(|_| StoreError::Unavailable)?;
        self.printings += rows.len();
        Ok(())
    }

    /// Inserts a batch of localized search names (up to ~1,000 rows per call).
    ///
    /// # Errors
    /// Returns [`StoreError::Unavailable`] if PostgreSQL rejects the batch.
    pub async fn insert_search_names(
        &mut self,
        rows: &[ImportSearchName],
    ) -> Result<(), StoreError> {
        if rows.is_empty() {
            return Ok(());
        }
        let mut sql = String::from(
            "INSERT INTO central_identity.card_search_names (
                oracle_id, lang, name, name_lower, en_name
             ) VALUES ",
        );
        let mut args: Vec<Value> = Vec::with_capacity(rows.len() * 5);
        for (i, row) in rows.iter().enumerate() {
            if i > 0 {
                sql.push(',');
            }
            let base = i * 5;
            let _ = write!(
                sql,
                "(${},${},${},${},${})",
                base + 1,
                base + 2,
                base + 3,
                base + 4,
                base + 5,
            );
            args.push(Uuid::from_bytes(row.oracle_id).into());
            args.push(row.lang.clone().into());
            args.push(row.name.clone().into());
            args.push(row.name.to_lowercase().into());
            args.push(row.en_name.clone().into());
        }
        sql.push_str(" ON CONFLICT (oracle_id, lang, name_lower) DO NOTHING");
        self.tx
            .execute_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                &sql,
                args,
            ))
            .await
            .map_err(|_| StoreError::Unavailable)?;
        self.search_names += rows.len();
        Ok(())
    }

    /// Commits the replaced catalog tables atomically.
    ///
    /// # Errors
    /// Returns [`StoreError::Unavailable`] on commit failure.
    pub async fn finish(self) -> Result<(), StoreError> {
        self.tx.commit().await.map_err(|_| StoreError::Unavailable)
    }
}
