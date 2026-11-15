use super::{
    AccountStore, ConnectionTrait, Delivery, Disposition, Error, Fact, MAX_DUELS, Progress,
    Receipt, Token, TransactionTrait, Uuid, get, issuer, statement, totals, unavailable, uuid,
};

impl AccountStore {
    /// Commit a verified fact under an explicitly provisioned issuer credential.
    /// Exact retries return the same disposition without updating permanent totals.
    /// `origin` is server configuration, never an incoming destination or player grant.
    /// # Errors
    /// Wrong/revoked issuer, conflict, missing ordinals, capacity or storage failure.
    pub async fn accept_tower_award(
        &self,
        credential: &Token,
        origin: &str,
        delivery: &Delivery,
    ) -> Result<Receipt, Error> {
        delivery.validate()?;
        let bytes = central_api::encode(&delivery.fact)?;
        if bytes.len() > 1024 {
            return Err(Error::Rejected);
        }
        let tx = self.db.begin().await.map_err(unavailable)?;
        let issuer = issuer(&tx, origin, credential).await?;
        let repeated = retry(&tx, issuer, delivery, &bytes).await?;
        let account = uuid(delivery.account);
        let row = tx
            .query_one_raw(statement(
                "SELECT restored AND credential IS NOT NULL AS ready FROM central_identity.accounts
             WHERE id=$1 FOR UPDATE",
                vec![account.into()],
            ))
            .await
            .map_err(unavailable)?;
        let Some(row) = row else {
            tx.commit().await.map_err(unavailable)?;
            return Ok(Receipt::new(delivery, Disposition::AccountGone));
        };
        if !get::<bool>(&row, "ready")? {
            return Err(Error::Conflict);
        }
        if repeated {
            tx.commit().await.map_err(unavailable)?;
            return Ok(Receipt::new(delivery, Disposition::Recorded));
        }
        let mut progress = totals(&tx, account).await?;
        if progress.duels >= MAX_DUELS {
            return Err(Error::Busy);
        }
        let run = advance(&tx, issuer, delivery, &bytes).await?;
        progress.apply(&delivery.fact)?;
        tx.query_one_raw(statement(
            "INSERT INTO central_identity.tower_award_receipts(issuer_id,account_id,run_id,remote_event,stage,fact)
             VALUES($1,$2,$3,$4,$5,$6) RETURNING id",
            vec![issuer.into(), account.into(), run.into(), uuid(delivery.event).into(),
                i16::from(delivery.fact.stage).into(), bytes.into()],
        )).await.map_err(unavailable)?.ok_or(Error::Unavailable)?;
        save(&tx, account, progress).await?;
        tx.commit().await.map_err(unavailable)?;
        Ok(Receipt::new(delivery, Disposition::Recorded))
    }
}

async fn retry(
    db: &impl ConnectionTrait,
    issuer: Uuid,
    delivery: &Delivery,
    bytes: &[u8],
) -> Result<bool, Error> {
    let row = db.query_one_raw(statement(
        "SELECT account_id,fact FROM central_identity.tower_award_receipts WHERE issuer_id=$1 AND remote_event=$2",
        vec![issuer.into(), uuid(delivery.event).into()],
    )).await.map_err(unavailable)?;
    let Some(row) = row else {
        return Ok(false);
    };
    if get::<Uuid>(&row, "account_id")? != uuid(delivery.account)
        || get::<Vec<u8>>(&row, "fact")? != bytes
    {
        return Err(Error::Conflict);
    }
    Ok(true)
}

async fn advance(
    db: &impl ConnectionTrait,
    issuer: Uuid,
    delivery: &Delivery,
    bytes: &[u8],
) -> Result<Uuid, Error> {
    let row = db.query_one_raw(statement(
        "SELECT id,account_id,last_fact FROM central_identity.tower_award_runs WHERE issuer_id=$1 AND remote_run=$2",
        vec![issuer.into(), uuid(delivery.fact.run).into()],
    )).await.map_err(unavailable)?;
    if let Some(row) = row {
        if get::<Uuid>(&row, "account_id")? != uuid(delivery.account) {
            return Err(Error::Conflict);
        }
        let previous: Fact =
            central_api::decode(&get::<Vec<u8>>(&row, "last_fact")?).map_err(unavailable)?;
        delivery.fact.follows(Some(&previous))?;
        let id: Uuid = get(&row, "id")?;
        db.execute_raw(statement(
            "UPDATE central_identity.tower_award_runs SET last_fact=$2 WHERE id=$1",
            vec![id.into(), bytes.to_vec().into()],
        ))
        .await
        .map_err(unavailable)?;
        Ok(id)
    } else {
        delivery.fact.follows(None)?;
        let row = db.query_one_raw(statement(
            "INSERT INTO central_identity.tower_award_runs(issuer_id,account_id,remote_run,last_fact)
             VALUES($1,$2,$3,$4) RETURNING id",
            vec![issuer.into(), uuid(delivery.account).into(), uuid(delivery.fact.run).into(), bytes.to_vec().into()],
        )).await.map_err(unavailable)?.ok_or(Error::Unavailable)?;
        get(&row, "id")
    }
}

async fn save(db: &impl ConnectionTrait, account: Uuid, progress: Progress) -> Result<(), Error> {
    let number = |value| i64::try_from(value).map_err(unavailable);
    db.execute_raw(statement(
        "INSERT INTO central_identity.tower_award_progress(account_id,duels,victories,clears,flawless)
         VALUES($1,$2,$3,$4,$5) ON CONFLICT(account_id) DO UPDATE SET
         duels=EXCLUDED.duels,victories=EXCLUDED.victories,clears=EXCLUDED.clears,flawless=EXCLUDED.flawless",
        vec![account.into(), number(progress.duels)?.into(), number(progress.victories)?.into(),
            number(progress.clears)?.into(), number(progress.flawless)?.into()],
    )).await.map_err(unavailable)?;
    Ok(())
}
