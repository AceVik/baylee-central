//! Atomic native account persistence. SQL identifiers are fixed; every supplied value is bound.
use crate::{DevConfig, PendingAccountStore};
use central_api::{
    Error, Token,
    wire::{Identity, MigrationState},
};
use central_crypto::{opaque::server::ServerKeys, vault::WrappedMasterKey};
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DbBackend, QueryResult, Statement, Value, prelude::Uuid,
};

mod gateways;
mod grants;
mod leases;
mod migration;
mod objects;
mod sessions;

/// An OPAQUE credential snapshot. It cannot issue sessions without proof verification.
pub struct Credential {
    /// Immutable context, checked against the client's expected account.
    pub identity: Identity,
    /// Credential revision used to prevent a stale login racing a password change.
    pub revision: i64,
    /// Canonical OPAQUE registration upload; never a plaintext password.
    pub record: Vec<u8>,
}

/// Server-only account store. Separate from the DDL/migration entry point.
#[derive(Clone)]
pub struct AccountStore {
    db: DatabaseConnection,
}

fn statement(sql: &str, values: Vec<Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, values)
}
fn uuid(id: [u8; 16]) -> Uuid {
    Uuid::from_bytes(id)
}
fn number(value: u64) -> Result<i64, Error> {
    i64::try_from(value).map_err(|_| Error::Rejected)
}
fn identity(row: &QueryResult) -> Result<Identity, Error> {
    let id: Uuid = row.try_get("", "id").map_err(|_| Error::Unavailable)?;
    let epoch: i64 = row
        .try_get("", "vault_epoch")
        .map_err(|_| Error::Unavailable)?;
    let account = *id.as_bytes();
    let epoch = u64::try_from(epoch).map_err(|_| Error::Unavailable)?;
    if account[6] >> 4 != 7 || account[8] >> 6 != 2 || epoch == 0 {
        return Err(Error::Unavailable);
    }
    Ok(Identity { account, epoch })
}

/// Canonical native username. Clients normalize existing NFKC names before this boundary.
fn username(value: &str) -> Result<String, Error> {
    let name = value.to_ascii_lowercase();
    if !(3..=24).contains(&name.len())
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
        || !name.as_bytes()[0].is_ascii_alphanumeric()
        || !name.as_bytes()[name.len() - 1].is_ascii_alphanumeric()
        || name
            .as_bytes()
            .windows(2)
            .any(|p| p.iter().all(|b| !b.is_ascii_alphanumeric()))
    {
        return Err(Error::Rejected);
    }
    Ok(name)
}

impl AccountStore {
    /// Open a restricted development pool; the role must additionally hold native-table CRUD.
    ///
    /// # Errors
    /// Rejects an unsafe role, wrong schema/version or inaccessible database.
    pub async fn connect_dev(config: DevConfig) -> Result<Self, Error> {
        let pending = PendingAccountStore::connect_dev(config)
            .await
            .map_err(|_| Error::Unavailable)?;
        let db = pending.into_connection();
        let store = Self { db };
        if !store.valid_grants().await {
            let _ = store.db.close().await;
            return Err(Error::Unavailable);
        }
        Ok(store)
    }

    /// Find an enrolled account by its canonical private login name.
    ///
    /// # Errors
    /// Returns generic input or database failures.
    pub async fn discover(&self, name: &str) -> Result<Option<Identity>, Error> {
        self.db.query_one_raw(statement(
            "SELECT id, vault_epoch FROM central_identity.accounts WHERE username_key = $1 AND credential IS NOT NULL",
            vec![username(name)?.into()],
        )).await.map_err(|_| Error::Unavailable)?.as_ref().map(identity).transpose()
    }

    /// Reserve after the bridge has freshly verified legacy ownership. Existing credentials stay intact.
    ///
    /// # Errors
    /// Rejects conflicting links/names, committed accounts or inaccessible storage.
    pub async fn reserve_legacy(
        &self,
        legacy: &str,
        name: &str,
        ticket: &Token,
    ) -> Result<Identity, Error> {
        if legacy.is_empty()
            || legacy.len() > 64
            || !legacy
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return Err(Error::Rejected);
        }
        let row = self.db.query_one_raw(statement(
            "INSERT INTO central_identity.accounts (legacy_id, username_key, enrollment_digest, enrollment_expires)
             VALUES ($1, $2, $3, pg_catalog.clock_timestamp() + interval '10 minutes')
             ON CONFLICT (legacy_id) DO UPDATE SET enrollment_digest = EXCLUDED.enrollment_digest,
                enrollment_expires = EXCLUDED.enrollment_expires, username_key = EXCLUDED.username_key
             WHERE central_identity.accounts.credential IS NULL AND NOT central_identity.accounts.legacy_closed
             RETURNING id, vault_epoch",
            vec![legacy.into(), username(name)?.into(), ticket.digest().to_vec().into()],
        )).await.map_err(|_| Error::Conflict)?.ok_or(Error::Conflict)?;
        identity(&row)
    }

    /// Read the account bound to a still-valid enrollment capability.
    ///
    /// # Errors
    /// Expired, consumed, missing and malformed tickets all reject generically.
    pub async fn enrollment(&self, ticket: &Token) -> Result<Identity, Error> {
        let row = self
            .db
            .query_one_raw(statement(
                "SELECT id, vault_epoch FROM central_identity.accounts WHERE enrollment_digest = $1
             AND enrollment_expires > pg_catalog.clock_timestamp() AND credential IS NULL",
                vec![ticket.digest().to_vec().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        identity(&row)
    }

    /// Commit credential, wrapper and document together, consuming a valid ticket atomically.
    ///
    /// # Errors
    /// Rejects malformed cryptography, invalid/expired capabilities or concurrent completion.
    pub async fn enroll(
        &self,
        ticket: &Token,
        record: &[u8],
        wrapped: &[u8],
        document: &[u8],
    ) -> Result<(), Error> {
        ServerKeys::enrollment_record(record).map_err(|_| Error::Rejected)?;
        WrappedMasterKey::from_bytes(wrapped).map_err(|_| Error::Rejected)?;
        validate_document(document)?;
        let result = self.db.execute_raw(statement(
            "UPDATE central_identity.accounts SET credential = $2, wrapped_master = $3, document = $4,
             document_revision = 1, enrollment_digest = NULL, enrollment_expires = NULL
             WHERE enrollment_digest = $1 AND enrollment_expires > pg_catalog.clock_timestamp() AND credential IS NULL",
            vec![ticket.digest().to_vec().into(), record.to_vec().into(), wrapped.to_vec().into(), document.to_vec().into()],
        )).await.map_err(|_| Error::Unavailable)?;
        if result.rows_affected() != 1 {
            return Err(Error::Rejected);
        }
        Ok(())
    }

    /// Read a credential for an OPAQUE exchange; the context is checked independently.
    ///
    /// # Errors
    /// Rejects stale account/epoch mappings and database failure.
    pub async fn credential(&self, expected: Identity) -> Result<Option<Credential>, Error> {
        let row = self.db.query_one_raw(statement(
            "SELECT id, vault_epoch, credential_revision, credential FROM central_identity.accounts
             WHERE id = $1 AND vault_epoch = $2 AND credential IS NOT NULL",
            vec![uuid(expected.account).into(), number(expected.epoch)?.into()],
        )).await.map_err(|_| Error::Unavailable)?;
        row.as_ref()
            .map(|row| {
                Ok(Credential {
                    identity: identity(row)?,
                    revision: row
                        .try_get("", "credential_revision")
                        .map_err(|_| Error::Unavailable)?,
                    record: row
                        .try_get("", "credential")
                        .map_err(|_| Error::Unavailable)?,
                })
            })
            .transpose()
    }

    /// Bridge-only durable progress; never trusts a client-supplied account identifier.
    ///
    /// # Errors
    /// Rejects unknown legacy links or storage failure.
    pub async fn legacy_status(&self, legacy: &str) -> Result<(Identity, MigrationState), Error> {
        let row = self.db.query_one_raw(statement(
            "SELECT id, vault_epoch, credential IS NOT NULL AS enrolled, restored FROM central_identity.accounts WHERE legacy_id = $1",
            vec![legacy.into()],
        )).await.map_err(|_| Error::Unavailable)?.ok_or(Error::Rejected)?;
        let enrolled: bool = row
            .try_get("", "enrolled")
            .map_err(|_| Error::Unavailable)?;
        let restored: bool = row
            .try_get("", "restored")
            .map_err(|_| Error::Unavailable)?;
        let state = if restored {
            MigrationState::Restored
        } else if enrolled {
            MigrationState::Enrolled
        } else {
            MigrationState::Pending
        };
        Ok((identity(&row)?, state))
    }

    /// Close this pool explicitly during owned-service cleanup.
    ///
    /// # Errors
    /// Reports generic connection cleanup failure.
    pub async fn close(self) -> Result<(), Error> {
        self.db.close().await.map_err(|_| Error::Unavailable)
    }
}

fn validate_document(document: &[u8]) -> Result<(), Error> {
    if !(41..=central_crypto::vault::MAX_OBJECT_BYTES + 41).contains(&document.len())
        || document[0] != 1
    {
        return Err(Error::Rejected);
    }
    Ok(())
}
