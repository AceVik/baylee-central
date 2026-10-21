//! Bounded opaque object storage. Ownership always comes from a live session, never a supplied ID.
use super::{AccountStore, Error, Token, number, statement, uuid, validate_document};
use central_api::object::{MAX_ACCOUNT_BYTES, MAX_ACCOUNT_OBJECTS, ObjectState};
use sea_orm::{ConnectionTrait, QueryResult, TransactionTrait, prelude::Uuid};

fn state(row: &QueryResult) -> Result<ObjectState, Error> {
    let id: Uuid = row.try_get("", "id").map_err(|_| Error::Unavailable)?;
    let revision: i64 = row
        .try_get("", "revision")
        .map_err(|_| Error::Unavailable)?;
    Ok(ObjectState {
        id: *id.as_bytes(),
        revision: u64::try_from(revision).map_err(|_| Error::Unavailable)?,
        deleted: row.try_get("", "deleted").map_err(|_| Error::Unavailable)?,
    })
}

// The same account lock serializes login replacement, quota changes and object
// mutations. Recheck the session after acquiring it to reject a concurrent logout.
// `require_closed`: the account no longer depends on the legacy gateway, being a
// closed legacy migration or a restored registered account.
pub(super) async fn locked_owner(
    db: &impl ConnectionTrait,
    session: &Token,
    require_closed: bool,
) -> Result<Uuid, Error> {
    let row = db
        .query_one_raw(statement(
            "SELECT a.id FROM central_identity.accounts a JOIN central_identity.sessions s
         ON s.account_id = a.id AND s.credential_revision = a.credential_revision
         WHERE s.digest = $1 AND s.expires_at > pg_catalog.clock_timestamp()
         AND (NOT $2 OR (a.restored AND (a.legacy_closed OR a.source = 'registered'))) FOR UPDATE OF a",
            vec![session.digest().to_vec().into(), require_closed.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::Rejected)?;
    let id: Uuid = row.try_get("", "id").map_err(|_| Error::Unavailable)?;
    let current = db
        .query_one_raw(statement(
            "SELECT s.digest FROM central_identity.sessions s JOIN central_identity.accounts a
         ON a.id = s.account_id AND a.credential_revision = s.credential_revision
         WHERE s.digest = $1 AND s.account_id = $2
         AND s.expires_at > pg_catalog.clock_timestamp() FOR SHARE OF s",
            vec![session.digest().to_vec().into(), id.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
    if current.is_none() {
        return Err(Error::Rejected);
    }
    Ok(id)
}

impl AccountStore {
    /// Reserve one database-generated `UUIDv7` without accepting a client-assigned identity.
    ///
    /// # Errors
    /// Rejects invalid sessions, unfinished migration, account quota or database failure.
    pub async fn create_object(&self, session: &Token) -> Result<ObjectState, Error> {
        let transaction = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let account = locked_owner(&transaction, session, true).await?;
        let maximum = i64::try_from(MAX_ACCOUNT_OBJECTS).map_err(|_| Error::Unavailable)?;
        // IDs are allocated only by PostgreSQL, never supplied for creation. Once
        // the bounded history fills, old tombstones can disappear safely: reads
        // still fail closed and a deleted identity cannot be resurrected.
        transaction.execute_raw(statement(
            "DELETE FROM central_identity.private_objects WHERE id IN
             (SELECT id FROM central_identity.private_objects WHERE account_id = $1 AND deleted
              AND (SELECT count(*) FROM central_identity.private_objects WHERE account_id = $1) >= $2
              ORDER BY id LIMIT 1000)", vec![account.into(), maximum.into()],
        )).await.map_err(|_| Error::Unavailable)?;
        let row = transaction.query_one_raw(statement(
            "INSERT INTO central_identity.private_objects(account_id)
             SELECT $1 WHERE (SELECT count(*) FROM central_identity.private_objects WHERE account_id = $1) < $2
             RETURNING id, revision, deleted",
            vec![account.into(), maximum.into()],
        )).await.map_err(|_| Error::Unavailable)?.ok_or(Error::Busy)?;
        let result = state(&row)?;
        transaction.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(result)
    }

    /// Read only this account's object, including an explicit unwritten/deleted marker.
    ///
    /// # Errors
    /// Rejects invalid sessions, unknown/foreign objects and database failure.
    pub async fn read_object(
        &self,
        session: &Token,
        id: [u8; 16],
    ) -> Result<(ObjectState, Option<Vec<u8>>), Error> {
        let row = self.db.query_one_raw(statement(
            "SELECT o.id, o.revision, o.deleted, o.document FROM central_identity.private_objects o
             JOIN central_identity.accounts a ON a.id = o.account_id
             JOIN central_identity.sessions s ON s.account_id = a.id AND s.credential_revision = a.credential_revision
             WHERE s.digest = $1 AND s.expires_at > pg_catalog.clock_timestamp()
             AND a.restored AND (a.legacy_closed OR a.source = 'registered') AND o.id = $2",
            vec![session.digest().to_vec().into(), uuid(id).into()],
        )).await.map_err(|_| Error::Unavailable)?.ok_or(Error::Rejected)?;
        Ok((
            state(&row)?,
            row.try_get("", "document")
                .map_err(|_| Error::Unavailable)?,
        ))
    }

    /// Paginate owned IDs for reconciliation. No private names or file paths exist server-side.
    ///
    /// # Errors
    /// Rejects invalid sessions and database failure.
    pub async fn list_objects(
        &self,
        session: &Token,
        after: Option<[u8; 16]>,
    ) -> Result<Vec<ObjectState>, Error> {
        let transaction = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let account = locked_owner(&transaction, session, true).await?;
        let rows = transaction
            .query_all_raw(statement(
                "SELECT id, revision, deleted FROM central_identity.private_objects
             WHERE account_id = $1 AND ($2::uuid IS NULL OR id > $2) ORDER BY id LIMIT 100",
                vec![account.into(), after.map(uuid).into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        let result = rows.iter().map(state).collect::<Result<_, _>>()?;
        transaction.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(result)
    }

    /// Compare-and-swap a live object. A tombstone cannot be overwritten or resurrected.
    ///
    /// # Errors
    /// Rejects invalid ownership, stale revisions, malformed data, quota or storage failure.
    pub async fn write_object(
        &self,
        session: &Token,
        id: [u8; 16],
        expected: u64,
        document: &[u8],
    ) -> Result<u64, Error> {
        validate_document(document)?;
        self.mutate_object(session, id, expected, Some(document))
            .await
    }

    /// Retain a monotone tombstone after deletion, including for abandoned reservations.
    ///
    /// # Errors
    /// Rejects invalid ownership, stale revisions, repeated deletion or storage failure.
    pub async fn delete_object(
        &self,
        session: &Token,
        id: [u8; 16],
        expected: u64,
    ) -> Result<u64, Error> {
        self.mutate_object(session, id, expected, None).await
    }

    async fn mutate_object(
        &self,
        session: &Token,
        id: [u8; 16],
        expected: u64,
        document: Option<&[u8]>,
    ) -> Result<u64, Error> {
        let expected = number(expected)?;
        let next = expected.checked_add(1).ok_or(Error::Rejected)?;
        let transaction = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let account = locked_owner(&transaction, session, true).await?;
        let row = transaction.query_one_raw(statement(
            "SELECT coalesce(octet_length(document), 0) AS bytes FROM central_identity.private_objects
             WHERE id = $1 AND account_id = $2 AND revision = $3 AND NOT deleted FOR UPDATE",
            vec![uuid(id).into(), account.into(), expected.into()],
        )).await.map_err(|_| Error::Unavailable)?.ok_or(Error::Conflict)?;
        let old_bytes: i32 = row.try_get("", "bytes").map_err(|_| Error::Unavailable)?;
        let new_bytes = document
            .map_or(Ok(0), |bytes| i64::try_from(bytes.len()))
            .map_err(|_| Error::Rejected)?;
        let delta = new_bytes - i64::from(old_bytes);
        let quota = transaction
            .execute_raw(statement(
                "UPDATE central_identity.accounts SET object_bytes = object_bytes + $2
             WHERE id = $1 AND object_bytes + $2 BETWEEN 0 AND $3",
                vec![
                    account.into(),
                    delta.into(),
                    number(MAX_ACCOUNT_BYTES)?.into(),
                ],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        if quota.rows_affected() != 1 {
            return Err(Error::Busy);
        }
        transaction.execute_raw(statement(
            "UPDATE central_identity.private_objects SET revision = $2, document = $3, deleted = $4 WHERE id = $1",
            vec![uuid(id).into(), next.into(), document.map(<[u8]>::to_vec).into(), document.is_none().into()],
        )).await.map_err(|_| Error::Unavailable)?;
        transaction.commit().await.map_err(|_| Error::Unavailable)?;
        u64::try_from(next).map_err(|_| Error::Unavailable)
    }
}
