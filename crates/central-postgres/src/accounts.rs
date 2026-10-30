//! Atomic native account persistence. SQL identifiers are fixed; every supplied value is bound.
use crate::{DevConfig, PendingAccountStore, ServerConfig};
use central_api::{
    Error, Token,
    wire::{Identity, MigrationState},
};
use central_crypto::{opaque::server::ServerKeys, vault::WrappedMasterKey};
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DbBackend, QueryResult, Statement, TransactionTrait,
    Value, prelude::Uuid,
};

mod admin;
mod deletion;
mod export;
mod gateways;
mod grants;
mod leases;
mod migration;
mod objects;
mod password;
mod profiles;
mod recovery;
mod registration;
mod reset;
mod sessions;
mod terms;
mod tokens;
pub use admin::{AUDIT_YEARS, Action, Appointed, Swept, appoint_admin, sweep};
pub use migration::Status;
pub use profiles::{Profile, TAG_DRAWS, TagDraw};
pub use recovery::Summary;
pub use sessions::{Issued, Session};
pub use terms::MAX_GATEWAY_VERSIONS;

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
    tags: profiles::Tags,
}

fn statement(sql: &str, values: Vec<Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, values)
}
/// The origins a gateway answers for as one parameter, read back by
/// [`any_origin`]: a validated origin holds no space, so a space divides them.
fn origins<S: AsRef<str>>(set: &[S]) -> Result<Value, Error> {
    let set: Vec<&str> = set.iter().map(AsRef::as_ref).collect();
    if set.is_empty() || set.iter().any(|o| o.is_empty() || o.contains(' ')) {
        return Err(Error::Rejected);
    }
    Ok(set.join(" ").into())
}
/// `ANY(...)` over the [`origins`] parameter number `n`.
fn any_origin(n: u8) -> String {
    format!("ANY(pg_catalog.string_to_array(${n}::text,' '))")
}
/// A registered gateway `g` an administrator has not disabled: approved or
/// still pending. A pending gateway signs players in by token
/// (`GatewayState`); only the directory waits for the approval.
const NOT_DISABLED: &str = "NOT EXISTS(SELECT 1 FROM central_identity.gateway_approvals d
    WHERE d.gateway_id=g.id AND NOT d.enabled)";
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
        Self::checked(PendingAccountStore::connect_dev(config).await).await
    }

    /// Open a restricted pool to a deployed server over verified TLS, with the same
    /// role and grant contract as [`Self::connect_dev`].
    ///
    /// # Errors
    /// Rejects an unsafe role, wrong schema/version, an unverified session or an
    /// inaccessible database.
    pub async fn connect(config: ServerConfig) -> Result<Self, Error> {
        Self::checked(PendingAccountStore::connect(config).await).await
    }

    async fn checked(
        pending: Result<PendingAccountStore, crate::StoreError>,
    ) -> Result<Self, Error> {
        let db = pending.map_err(|_| Error::Unavailable)?.into_connection();
        let store = Self {
            db,
            tags: profiles::Tags::default(),
        };
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
    /// An abandoned registration holding the name gives it up; a live one keeps it.
    /// The bridge's display name and tag are stored with it, and a renewed
    /// reservation of a pending account refreshes them: the name it carries
    /// replaces the one held, and a tag, once held, never changes. A carried
    /// tag another account holds already is replaced by a drawn one.
    ///
    /// # Errors
    /// Rejects conflicting links/names, committed accounts, a malformed
    /// carried name or tag, or inaccessible storage; `Busy` when a taken
    /// tag cannot be replaced.
    pub async fn reserve_legacy(
        &self,
        legacy: &str,
        name: &str,
        display_name: Option<&str>,
        tag: Option<u32>,
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
        let name = username(name)?;
        let display_name = profiles::carried_name(display_name)?;
        let carried = profiles::carried_tag(tag)?;
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        tx.execute_raw(statement(
            "DELETE FROM central_identity.accounts WHERE username_key = $1 AND source = 'registered'
             AND credential IS NULL AND enrollment_expires <= pg_catalog.clock_timestamp()",
            vec![name.clone().into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        let reserve = |tag: Option<i32>| {
            statement(
                "INSERT INTO central_identity.accounts (source, legacy_id, username_key, enrollment_digest,
                    enrollment_expires, display_name, display_name_at, tag)
                 SELECT 'legacy', $1, $2, $3, now + interval '10 minutes', $4,
                    CASE WHEN $4::text IS NULL THEN NULL ELSE now END, $5
                 FROM (SELECT pg_catalog.clock_timestamp() AS now) clock
                 ON CONFLICT (legacy_id) DO UPDATE SET enrollment_digest = EXCLUDED.enrollment_digest,
                    enrollment_expires = EXCLUDED.enrollment_expires, username_key = EXCLUDED.username_key,
                    display_name = COALESCE(EXCLUDED.display_name, central_identity.accounts.display_name),
                    display_name_at = CASE
                        WHEN EXCLUDED.display_name IS NULL THEN central_identity.accounts.display_name_at
                        WHEN EXCLUDED.display_name = central_identity.accounts.display_name
                            THEN central_identity.accounts.display_name_at
                        ELSE EXCLUDED.display_name_at END,
                    tag = COALESCE(central_identity.accounts.tag, EXCLUDED.tag)
                 WHERE central_identity.accounts.credential IS NULL AND NOT central_identity.accounts.legacy_closed
                 RETURNING id, vault_epoch",
                vec![
                    legacy.into(),
                    name.clone().into(),
                    ticket.digest().to_vec().into(),
                    display_name.into(),
                    tag.into(),
                ],
            )
        };
        let row = match carried {
            // The row being refreshed is this legacy account's, whose own tag is no collision.
            Some(_) => {
                let own: Option<Uuid> = tx
                    .query_one_raw(statement(
                        "SELECT id FROM central_identity.accounts WHERE legacy_id = $1",
                        vec![legacy.into()],
                    ))
                    .await
                    .map_err(|_| Error::Unavailable)?
                    .map(|row| row.try_get("", "id"))
                    .transpose()
                    .map_err(|_| Error::Unavailable)?;
                self.first_free_tag(&tx, own, carried, Error::Conflict, |tag| reserve(Some(tag)))
                    .await?
            }
            // A bridge from before: no tag, and none drawn; it waits for the bridge.
            None => tx
                .query_one_raw(reserve(None))
                .await
                .map_err(|_| Error::Conflict)?,
        }
        .ok_or(Error::Conflict)?;
        let identity = identity(&row)?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(identity)
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
    /// A registered account also commits its recovery wrapper and spends its key's
    /// reserved use; a legacy account takes no recovery wrapper.
    ///
    /// # Errors
    /// Rejects malformed cryptography, a recovery wrapper missing for a registered
    /// or present for a legacy account, invalid/expired capabilities, a key revoked
    /// since the reservation, or concurrent completion.
    pub async fn enroll(
        &self,
        ticket: &Token,
        record: &[u8],
        wrapped: &[u8],
        document: &[u8],
        recovery: Option<&[u8]>,
    ) -> Result<(), Error> {
        ServerKeys::enrollment_record(record).map_err(|_| Error::Rejected)?;
        WrappedMasterKey::from_bytes(wrapped).map_err(|_| Error::Rejected)?;
        if let Some(recovery) = recovery {
            WrappedMasterKey::from_bytes(recovery).map_err(|_| Error::Rejected)?;
        }
        validate_document(document)?;
        let digest = ticket.digest().to_vec();
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let pending = tx
            .query_one_raw(statement(
                "SELECT source, beta_key_id FROM central_identity.accounts WHERE enrollment_digest = $1
                 AND enrollment_expires > pg_catalog.clock_timestamp() AND credential IS NULL",
                vec![digest.clone().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        let source: String = pending
            .try_get("", "source")
            .map_err(|_| Error::Unavailable)?;
        let key: Option<Uuid> = pending
            .try_get("", "beta_key_id")
            .map_err(|_| Error::Unavailable)?;
        match (source.as_str(), recovery, key) {
            ("registered", Some(_), Some(key)) => {
                // The key's row before the account's, the order registration takes.
                let spent = tx
                    .execute_raw(statement(
                        "UPDATE central_identity.beta_keys SET uses_left = uses_left - 1
                         WHERE id = $1 AND uses_left > 0 AND revoked_at IS NULL",
                        vec![key.into()],
                    ))
                    .await
                    .map_err(|_| Error::Unavailable)?;
                if spent.rows_affected() != 1 {
                    return Err(Error::Rejected);
                }
            }
            ("legacy", None, None) => {}
            _ => return Err(Error::Rejected),
        }
        let result = tx.execute_raw(statement(
            "UPDATE central_identity.accounts SET credential = $2, wrapped_master = $3, document = $4,
             recovery_wrapped_master = $5, document_revision = 1, enrollment_digest = NULL,
             enrollment_expires = NULL, beta_key_id = NULL
             WHERE enrollment_digest = $1 AND enrollment_expires > pg_catalog.clock_timestamp() AND credential IS NULL
             AND source = $6 AND beta_key_id IS NOT DISTINCT FROM $7",
            vec![digest.into(), record.to_vec().into(), wrapped.to_vec().into(), document.to_vec().into(),
                recovery.map(<[u8]>::to_vec).into(), source.into(), key.into()],
        )).await.map_err(|_| Error::Unavailable)?;
        if result.rows_affected() != 1 {
            return Err(Error::Rejected);
        }
        tx.commit().await.map_err(|_| Error::Unavailable)
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
