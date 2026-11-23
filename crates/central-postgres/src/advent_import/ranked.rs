//! One source-isolated catalog. Repeated imports must verify all 56 slots.
use super::{
    Art, ConnectionTrait, DatabaseConnection, DatabaseTransaction, Kind, MatFraming, Source,
    StoreError, TransactionTrait, Uuid, check_role, sql, store_art,
};
use std::collections::BTreeSet;

/// One restricted operator transaction for all league/division pairs.
pub struct RankedCatalog {
    tx: DatabaseTransaction,
    seen: Option<BTreeSet<(u8, u8)>>,
}

pub(super) async fn check_framing(
    tx: &DatabaseTransaction,
    id: Uuid,
    expected: Option<&MatFraming>,
) -> Result<(), StoreError> {
    let json = expected
        .map(|f| {
            central_api::encode(f)
                .ok()
                .and_then(|v| String::from_utf8(v).ok())
                .ok_or(StoreError::InvalidConfig)
        })
        .transpose()?;
    tx.query_one_raw(sql(
        "SELECT 1 WHERE (SELECT framing FROM central_identity.playmat_framing WHERE art_id=$1)
        IS NOT DISTINCT FROM $2::text::jsonb",
        vec![id.into(), json.into()],
    ))
    .await
    .map_err(|_| StoreError::Unavailable)?
    .ok_or(StoreError::InvalidConfig)?;
    Ok(())
}

impl RankedCatalog {
    /// Begin a new complete import or an exact verification of an existing catalog.
    ///
    /// # Errors
    /// Refuses an unsuitable provisioning role, missing grants or database failure.
    pub async fn begin(db: &DatabaseConnection) -> Result<Self, StoreError> {
        let tx = db.begin().await.map_err(|_| StoreError::Unavailable)?;
        check_role(&tx).await?;
        tx.query_one_raw(sql(
            "SELECT 1 WHERE has_column_privilege(current_user,
            'central_identity.ranked_cosmetic_catalog','art_id','INSERT')",
            vec![],
        ))
        .await
        .map_err(|_| StoreError::Unavailable)?
        .ok_or(StoreError::UnsafeRole)?;
        tx.query_one_raw(sql(
            "SELECT pg_advisory_xact_lock(1650553465, 1633973861)",
            vec![],
        ))
        .await
        .map_err(|_| StoreError::Unavailable)?;
        Ok(Self {
            tx,
            seen: Some(BTreeSet::new()),
        })
    }

    /// Stage both visuals for one legal rank. Any error invalidates this import.
    ///
    /// # Errors
    /// Refuses duplicate ranks, a wrong kind, inconsistent assets or a changed publication.
    pub async fn pair(
        &mut self,
        league: u8,
        division: u8,
        sleeve: Art,
        playmat: Art,
    ) -> Result<(), StoreError> {
        let mut seen = self.seen.take().ok_or(StoreError::InvalidConfig)?;
        if !matches!(
            (league, division),
            (0 | 8..=10, 0) | (1..=4, 1..=3) | (5..=7, 1..=4)
        ) || !seen.insert((league, division))
            || sleeve.kind != Some(Kind::Sleeve)
            || playmat.kind != Some(Kind::Playmat)
        {
            return Err(StoreError::InvalidConfig);
        }
        for (kind, art) in [(Kind::Sleeve, sleeve), (Kind::Playmat, playmat)] {
            let id = store_art(&self.tx, art, Source::Ranked).await?;
            let keys = vec![
                i16::from(league).into(),
                i16::from(division).into(),
                kind.as_str().into(),
                id.into(),
            ];
            self.tx.execute_raw(sql("INSERT INTO central_identity.ranked_cosmetic_catalog (league,division,kind,art_id)
                VALUES($1,$2,$3,$4) ON CONFLICT DO NOTHING",keys.clone())).await.map_err(|_|StoreError::Unavailable)?;
            self.tx
                .query_one_raw(sql(
                    "SELECT 1 FROM central_identity.ranked_cosmetic_catalog
                WHERE league=$1 AND division=$2 AND kind=$3 AND art_id=$4",
                    keys,
                ))
                .await
                .map_err(|_| StoreError::Unavailable)?
                .ok_or(StoreError::InvalidConfig)?;
        }
        self.seen = Some(seen);
        Ok(())
    }

    /// Publish all 28 verified pairs atomically.
    ///
    /// # Errors
    /// Refuses a partial or invalid import and propagates commit failures.
    pub async fn finish(self) -> Result<(), StoreError> {
        if self.seen.is_none_or(|seen| seen.len() != 28) {
            return Err(StoreError::InvalidConfig);
        }
        self.tx.commit().await.map_err(|_| StoreError::Unavailable)
    }
}
