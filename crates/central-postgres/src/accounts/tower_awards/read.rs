use super::{
    AccountStore, ConnectionTrait, Error, Progress, Session, TransactionTrait, Uuid, get,
    statement, totals, unavailable,
};

impl AccountStore {
    /// Read only the live session owner's permanent v2 projection.
    /// # Errors
    /// Wrong/expired session, incomplete account restoration or storage failure.
    pub async fn tower_award_progress(&self, session: Session<'_>) -> Result<Progress, Error> {
        let tx = self
            .db
            .begin_with_config(
                Some(sea_orm::IsolationLevel::RepeatableRead),
                Some(sea_orm::AccessMode::ReadOnly),
            )
            .await
            .map_err(unavailable)?;
        let account = caller(&tx, session).await?;
        let progress = totals(&tx, account).await?;
        tx.commit().await.map_err(unavailable)?;
        Ok(progress)
    }

    /// Export a bounded page of this live session owner's permanent receipt history.
    /// # Errors
    /// Invalid cursor/session, incomplete account restoration or storage failure.
    pub async fn export_tower_awards(
        &self,
        session: Session<'_>,
        after: Option<[u8; 16]>,
    ) -> Result<central_api::tower_awards::Export, Error> {
        if after.is_some_and(|id| id[6] >> 4 != 7 || id[8] >> 6 != 2) {
            return Err(Error::Rejected);
        }
        let tx = self
            .db
            .begin_with_config(
                Some(sea_orm::IsolationLevel::RepeatableRead),
                Some(sea_orm::AccessMode::ReadOnly),
            )
            .await
            .map_err(unavailable)?;
        let account = caller(&tx, session).await?;
        let progress = totals(&tx, account).await?;
        let rows = tx.query_all_raw(statement(
            "SELECT r.id,r.remote_event,r.fact,i.origin,floor(extract(epoch FROM r.created_at))::bigint AS accepted_at
             FROM central_identity.tower_award_receipts r JOIN central_identity.tower_award_issuers i ON i.id=r.issuer_id
             WHERE r.account_id=$1 AND ($2::uuid IS NULL OR r.id>$2) ORDER BY r.id LIMIT 129",
            vec![account.into(), after.map(Uuid::from_bytes).into()],
        )).await.map_err(unavailable)?;
        let receipts = rows
            .iter()
            .take(128)
            .map(|row| {
                Ok::<_, Error>(central_api::tower_awards::ExportedReceipt {
                    id: *get::<Uuid>(row, "id")?.as_bytes(),
                    event: *get::<Uuid>(row, "remote_event")?.as_bytes(),
                    origin: get(row, "origin")?,
                    fact: central_api::decode(&get::<Vec<u8>>(row, "fact")?)
                        .map_err(unavailable)?,
                    accepted_at: get(row, "accepted_at")?,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let next = if rows.len() > 128 {
            receipts.last().map(|item| item.id)
        } else {
            None
        };
        tx.commit().await.map_err(unavailable)?;
        Ok(central_api::tower_awards::Export {
            progress,
            receipts,
            next,
        })
    }
}

async fn caller(db: &impl ConnectionTrait, session: Session<'_>) -> Result<Uuid, Error> {
    let row = db
        .query_one_raw(statement(
            "SELECT a.id FROM central_identity.accounts a JOIN central_identity.sessions s
         ON s.account_id=a.id AND s.credential_revision=a.credential_revision
         WHERE s.digest=$1 AND s.kind=$2 AND s.expires_at>pg_catalog.clock_timestamp()
         AND a.restored AND a.credential IS NOT NULL
         AND COALESCE(s.proven_at,s.created_at)+interval '12 hours'>pg_catalog.clock_timestamp()",
            vec![session.digest().to_vec().into(), session.kind().into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Rejected)?;
    get(&row, "id")
}
