//! Entitlement and final publication for the one-time oversized legacy import.
//! The legacy role alone can read the source. The runtime role binds each
//! successful page and the verified object inventory to the restored identity.

use super::super::{AccountStore, Error, Token, number, objects, statement};
use central_api::{
    legacy::object_inventory,
    object::{MAX_ACCOUNT_OBJECTS, ObjectState},
};
use sea_orm::{ConnectionTrait, TransactionTrait, prelude::Uuid};

impl AccountStore {
    /// Record a sequential, generation-bound page of this restored native account.
    ///
    /// # Errors
    /// Refuses stale sessions, changed generations, incomplete earlier pages and closed moves.
    pub async fn accept_legacy_page(
        &self,
        token: &Token,
        generation: [u8; 32],
        page: u16,
    ) -> Result<(), Error> {
        let index = i16::try_from(page).map_err(|_| Error::Rejected)?;
        let result=self.db.execute_raw(statement(
            "UPDATE central_identity.legacy_deck_transfers t SET next_page=GREATEST(t.next_page,$3::smallint+1)
             FROM central_identity.accounts a JOIN central_identity.sessions s
               ON s.account_id=a.id AND s.credential_revision=a.credential_revision
             WHERE t.account_id=a.id AND a.restored AND NOT a.legacy_closed AND a.legacy_id IS NOT NULL
               AND s.digest=$1 AND s.kind='native' AND s.expires_at>pg_catalog.clock_timestamp()
               AND t.generation=$2 AND $3::smallint<=t.next_page
               AND $3::smallint<(t.source_bytes+65535)/65536",
            vec![token.digest().to_vec().into(),generation.to_vec().into(),index.into()],
        )).await.map_err(|_|Error::Unavailable)?;
        if result.rows_affected() != 1 {
            return Err(Error::Conflict);
        }
        Ok(())
    }

    /// Verify and close an oversized source atomically with its account vault.
    ///
    /// # Errors
    /// Refuses stale credentials, incomplete sources, source mismatch, missing or changed private
    /// objects and changes to the encrypted root document since the client's readback.
    pub async fn finish_legacy_archive(
        &self,
        token: &Token,
        generation: [u8; 32],
        digest: [u8; 32],
        revision: u64,
        inventory: [u8; 32],
    ) -> Result<(), Error> {
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let owner = objects::locked_owner(&tx, token, true).await?;
        // Object mutations and account revisions take this same owner lock.
        let held = tx
            .query_one_raw(statement(
                "SELECT t.account_id FROM central_identity.legacy_deck_transfers t
             JOIN central_identity.accounts a ON a.id=t.account_id WHERE t.account_id=$1
             AND a.restored AND NOT a.legacy_closed AND a.document_revision=$2
             AND t.generation=$3 AND t.source_digest=$4 AND t.next_page=(t.source_bytes+65535)/65536
             FOR UPDATE OF t",
                vec![
                    owner.into(),
                    number(revision)?.into(),
                    generation.to_vec().into(),
                    digest.to_vec().into(),
                ],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        if held.is_none() {
            return Err(Error::Conflict);
        }
        let rows=tx.query_all_raw(statement("SELECT id,revision FROM central_identity.private_objects
            WHERE account_id=$1 AND NOT deleted AND revision>0 AND document IS NOT NULL ORDER BY id LIMIT 16385",
            vec![owner.into()])).await.map_err(|_|Error::Unavailable)?;
        if rows.is_empty() || rows.len() > MAX_ACCOUNT_OBJECTS {
            return Err(Error::Conflict);
        }
        let mut states = Vec::with_capacity(rows.len());
        for row in rows {
            states.push(ObjectState {
                id: *row
                    .try_get::<Uuid>("", "id")
                    .map_err(|_| Error::Unavailable)?
                    .as_bytes(),
                revision: u64::try_from(
                    row.try_get::<i64>("", "revision")
                        .map_err(|_| Error::Unavailable)?,
                )
                .map_err(|_| Error::Unavailable)?,
                deleted: false,
            });
        }
        if object_inventory(&states) != inventory {
            return Err(Error::Conflict);
        }
        tx.execute_raw(statement(
            "UPDATE central_identity.accounts SET legacy_closed=true,legacy_deck_source=$2 WHERE id=$1",
            vec![owner.into(),digest.to_vec().into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        tx.execute_raw(statement(
            "DELETE FROM central_identity.legacy_deck_transfers WHERE account_id=$1",
            vec![owner.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        tx.commit().await.map_err(|_| Error::Unavailable)
    }
}
