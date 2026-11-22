//! The legacy store (`docs/native-login.md` §"Legacy login"): the official
//! gateway's accounts that had not moved when Central took their sign-in over.
//! It lives in its own schema, read through its own login role
//! (`central_legacy`): the runtime role cannot see it (`AccountStore`'s startup
//! check), and this store reaches nothing else (`legacy-role.sql`). The
//! operator imports it once (`import`), sets its deadline and sweeps it
//! (`sweep`) under the migrator login.
//!
//! A row is deleted, never marked: when its move closes, or by the sweep once
//! the deadline has passed. After the deadline no row is handed out, swept or
//! not.
use crate::{DevConfig, ServerConfig, StoreError, accounts::username, connect_dev, connect_server};
use central_api::{
    Error,
    legacy::{LegacyAccount, LegacyProgress, LegacySnapshot},
};
use central_crypto::legacy::{HASH_BYTES, SALT_BYTES};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, QueryResult, Statement, Value};
use secrecy::zeroize::Zeroizing;

mod import;
mod sweep;
pub use import::{
    ImportRefusal, Imported, LegacyRow, MAX_LEGACY_ROWS, import_legacy, legacy_digest,
};
pub use sweep::{LegacySwept, set_legacy_deadline, sweep_legacy};

/// Consecutive failed sign-ins that lock an account out.
pub const LOCKOUT_FAILURES: i32 = 10;
/// How long a lockout lasts, in minutes.
pub const LOCKOUT_MINUTES: i32 = 60;

/// The condition every runtime read adds: the deadline has not passed.
const BEFORE_DEADLINE: &str =
    "(SELECT s.deadline IS NULL OR pg_catalog.clock_timestamp() < s.deadline
     FROM central_legacy.settings s)";

fn statement(sql: &str, values: Vec<Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, values)
}

/// What a legacy sign-in needs of its account, read only while it is not
/// locked out and before the deadline. No `Debug`: it holds the verifier.
pub struct Candidate {
    /// Its id at the official gateway.
    pub legacy_id: String,
    /// Its canonical username.
    pub username: String,
    /// Its display name, when it kept the rule.
    pub display_name: Option<String>,
    /// Its tag.
    pub tag: Option<u32>,
    /// Argon2id salt, under the gateway's parameters.
    pub salt: [u8; SALT_BYTES],
    /// Argon2id output, under the gateway's parameters.
    pub hash: Zeroizing<[u8; HASH_BYTES]>,
    /// The gateway required a new password.
    pub must_change_password: bool,
    /// A ban is in force now.
    pub banned: bool,
}

/// The legacy role's pool. Its own credentials; never the runtime's.
#[derive(Clone)]
pub struct LegacyStore {
    db: DatabaseConnection,
}

impl LegacyStore {
    /// Open a loopback development pool under the legacy role.
    ///
    /// # Errors
    /// Rejects a role with any privilege beyond its contract, a missing
    /// schema, an unsupported version or an inaccessible database.
    pub async fn connect_dev(config: DevConfig) -> Result<Self, StoreError> {
        Self::checked(connect_dev(config).await?).await
    }

    /// Open a pool to a deployed server over verified TLS, with the same
    /// role contract as [`Self::connect_dev`].
    ///
    /// # Errors
    /// As [`Self::connect_dev`], and an unverified session.
    pub async fn connect(config: ServerConfig) -> Result<Self, StoreError> {
        Self::checked(connect_server(config).await?).await
    }

    async fn checked(db: DatabaseConnection) -> Result<Self, StoreError> {
        let allowed = db
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                include_str!("legacy-role.sql"),
            ))
            .await
            .ok()
            .flatten()
            .and_then(|row| row.try_get::<bool>("", "allowed").ok());
        if allowed == Some(true) {
            Ok(Self { db })
        } else {
            let _ = db.close().await;
            Err(StoreError::UnsafeRole)
        }
    }

    /// The account a sign-in names (as typed; canonicalized here), unless it
    /// is unknown, locked out, or the deadline has passed: the three read
    /// alike, as none, and so does a name the rule refuses.
    ///
    /// # Errors
    /// Only for unavailable storage.
    pub async fn candidate(&self, typed: &str) -> Result<Option<Candidate>, Error> {
        let Ok(username_key) = username(typed) else {
            return Ok(None);
        };
        let row = self
            .db
            .query_one_raw(statement(
                &format!(
                    "SELECT legacy_id, username_key, display_name, tag, password_salt, password_hash,
                         must_change_password,
                         banned AND (banned_until IS NULL OR banned_until > pg_catalog.clock_timestamp())
                             AS banned
                     FROM central_legacy.accounts
                     WHERE username_key = $1
                         AND (locked_until IS NULL OR locked_until <= pg_catalog.clock_timestamp())
                         AND {BEFORE_DEADLINE}"
                ),
                vec![username_key.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        row.map(|row| candidate(&row)).transpose()
    }

    /// Count a failed sign-in for this name (as typed): the tenth in a row locks the
    /// account out for an hour and starts the count again. A name that is
    /// nobody's, or one locked out already, changes nothing, at the cost of
    /// the same statement.
    ///
    /// # Errors
    /// Only for unavailable storage.
    pub async fn failed(&self, typed: &str) -> Result<(), Error> {
        let Ok(username_key) = username(typed) else {
            return Ok(());
        };
        self.db
            .execute_raw(statement(
                "UPDATE central_legacy.accounts SET
                     failures = CASE WHEN failures + 1 >= $2 THEN 0 ELSE failures + 1 END,
                     locked_until = CASE WHEN failures + 1 >= $2
                         THEN pg_catalog.clock_timestamp() + pg_catalog.make_interval(mins => $3)
                         ELSE locked_until END
                 WHERE username_key = $1
                     AND (locked_until IS NULL OR locked_until <= pg_catalog.clock_timestamp())",
                vec![
                    username_key.into(),
                    LOCKOUT_FAILURES.into(),
                    LOCKOUT_MINUTES.into(),
                ],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        Ok(())
    }

    /// A proven password: the count starts again.
    ///
    /// # Errors
    /// Only for unavailable storage.
    pub async fn succeeded(&self, legacy_id: &str) -> Result<(), Error> {
        self.db
            .execute_raw(statement(
                "UPDATE central_legacy.accounts SET failures = 0, locked_until = NULL
                 WHERE legacy_id = $1 AND (failures <> 0 OR locked_until IS NOT NULL)",
                vec![legacy_id.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        Ok(())
    }

    /// The account's source, while it is in the store and the deadline has
    /// not passed.
    ///
    /// # Errors
    /// Only for unavailable storage.
    pub async fn snapshot(&self, legacy_id: &str) -> Result<Option<LegacySnapshot>, Error> {
        let row = self
            .db
            .query_one_raw(statement(
                &format!(
                    "SELECT legacy_id,username_key,snapshot_digest,octet_length(snapshot) AS bytes,
                     CASE WHEN octet_length(snapshot)<=$2 THEN snapshot END AS inline_snapshot
                     FROM central_legacy.accounts WHERE legacy_id = $1 AND {BEFORE_DEADLINE}"
                ),
                vec![
                    legacy_id.into(),
                    i32::try_from(central_api::legacy::MAX_SNAPSHOT_BYTES)
                        .map_err(|_| Error::Unavailable)?
                        .into(),
                ],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        row.map(|row| {
            let digest: Vec<u8> = row
                .try_get("", "snapshot_digest")
                .map_err(|_| Error::Unavailable)?;
            let digest: [u8; 32] = digest.try_into().map_err(|_| Error::Unavailable)?;
            let inline: Option<Vec<u8>> = row
                .try_get("", "inline_snapshot")
                .map_err(|_| Error::Unavailable)?;
            let (document, digest) = if let Some(inline) = inline {
                (inline, digest)
            } else {
                use central_api::legacy::{
                    ArchiveManifest, ArchiveSource, source_digest, source_generation,
                };
                let bytes = u32::try_from(
                    row.try_get::<i32>("", "bytes")
                        .map_err(|_| Error::Unavailable)?,
                )
                .map_err(|_| Error::Unavailable)?;
                let manifest = ArchiveManifest {
                    version: 1,
                    bytes,
                    generation: source_generation(legacy_id, digest, bytes),
                };
                if !manifest.valid() {
                    return Err(Error::Unavailable);
                }
                let wrapped = central_api::encode(&ArchiveSource {
                    paged_deck_source: manifest,
                })?;
                let envelope_digest = source_digest(&wrapped);
                (wrapped, envelope_digest)
            };
            Ok(LegacySnapshot {
                legacy_id: row
                    .try_get("", "legacy_id")
                    .map_err(|_| Error::Unavailable)?,
                username: row
                    .try_get("", "username_key")
                    .map_err(|_| Error::Unavailable)?,
                digest,
                document,
            })
        })
        .transpose()
    }

    /// Read a single staged source page while the operator's deadline remains open.
    ///
    /// # Errors
    /// Rejects a wrong generation, out-of-range page, changed source or unavailable store.
    pub async fn archive_page(
        &self,
        legacy_id: &str,
        generation: [u8; 32],
        page: u16,
    ) -> Result<Vec<u8>, Error> {
        use central_api::legacy::{ARCHIVE_PAGE_BYTES, ArchiveManifest, source_generation};
        let size = i32::try_from(ARCHIVE_PAGE_BYTES).map_err(|_| Error::Unavailable)?;
        let offset = i32::from(page)
            .checked_mul(size)
            .and_then(|n| n.checked_add(1))
            .ok_or(Error::Rejected)?;
        let row=self.db.query_one_raw(statement(&format!(
            "SELECT snapshot_digest,octet_length(snapshot) AS size,substring(snapshot FROM $2 FOR $3) AS chunk
             FROM central_legacy.accounts WHERE legacy_id=$1 AND octet_length(snapshot)>=$2 AND {BEFORE_DEADLINE}"),
             vec![legacy_id.into(),offset.into(),size.into()])).await.map_err(|_|Error::Unavailable)?.ok_or(Error::Rejected)?;
        let digest = row
            .try_get::<Vec<u8>>("", "snapshot_digest")
            .map_err(|_| Error::Unavailable)?
            .try_into()
            .map_err(|_| Error::Unavailable)?;
        let bytes = u32::try_from(
            row.try_get::<i32>("", "size")
                .map_err(|_| Error::Unavailable)?,
        )
        .map_err(|_| Error::Unavailable)?;
        let manifest = ArchiveManifest {
            version: 1,
            bytes,
            generation: source_generation(legacy_id, digest, bytes),
        };
        if generation != manifest.generation || page >= manifest.pages() {
            return Err(Error::Conflict);
        }
        row.try_get("", "chunk").map_err(|_| Error::Unavailable)
    }

    /// Delete the account's row once its move is closed. Gone already is no
    /// error: a lost answer is retried.
    ///
    /// # Errors
    /// Only for unavailable storage.
    pub async fn remove(&self, legacy_id: &str) -> Result<(), Error> {
        self.db
            .execute_raw(statement(
                "DELETE FROM central_legacy.accounts WHERE legacy_id = $1",
                vec![legacy_id.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        Ok(())
    }

    /// The deadline, Unix seconds, and every row left, by username, without
    /// verifiers or snapshots. Each is [`LegacyProgress::Unstarted`]: the
    /// account store knows how far its move got.
    ///
    /// # Errors
    /// Only for unavailable storage.
    pub async fn listing(&self) -> Result<(Option<i64>, Vec<LegacyAccount>), Error> {
        let deadline = self
            .db
            .query_one_raw(statement(
                "SELECT floor(extract(epoch FROM deadline))::bigint AS deadline
                 FROM central_legacy.settings",
                vec![],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)?
            .try_get::<Option<i64>>("", "deadline")
            .map_err(|_| Error::Unavailable)?;
        let rows = self
            .db
            .query_all_raw(statement(
                "SELECT legacy_id, username_key, display_name, tag, failures, must_change_password,
                     floor(extract(epoch FROM imported_at))::bigint AS imported_at,
                     CASE WHEN locked_until > pg_catalog.clock_timestamp()
                         THEN floor(extract(epoch FROM locked_until))::bigint END AS locked_until,
                     banned AND (banned_until IS NULL OR banned_until > pg_catalog.clock_timestamp())
                         AS banned,
                     octet_length(snapshot) AS snapshot_bytes
                 FROM central_legacy.accounts ORDER BY username_key",
                vec![],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        let get = |row: &QueryResult| -> Result<LegacyAccount, Error> {
            let failures: i32 = row
                .try_get("", "failures")
                .map_err(|_| Error::Unavailable)?;
            let bytes: i32 = row
                .try_get("", "snapshot_bytes")
                .map_err(|_| Error::Unavailable)?;
            Ok(LegacyAccount {
                legacy_id: row
                    .try_get("", "legacy_id")
                    .map_err(|_| Error::Unavailable)?,
                username: row
                    .try_get("", "username_key")
                    .map_err(|_| Error::Unavailable)?,
                display_name: row
                    .try_get("", "display_name")
                    .map_err(|_| Error::Unavailable)?,
                tag: tag(row)?,
                progress: LegacyProgress::Unstarted,
                imported_at: row
                    .try_get("", "imported_at")
                    .map_err(|_| Error::Unavailable)?,
                failures: u32::try_from(failures).map_err(|_| Error::Unavailable)?,
                locked_until: row
                    .try_get("", "locked_until")
                    .map_err(|_| Error::Unavailable)?,
                must_change_password: row
                    .try_get("", "must_change_password")
                    .map_err(|_| Error::Unavailable)?,
                banned: row.try_get("", "banned").map_err(|_| Error::Unavailable)?,
                snapshot_bytes: u32::try_from(bytes).map_err(|_| Error::Unavailable)?,
            })
        };
        Ok((deadline, rows.iter().map(get).collect::<Result<_, _>>()?))
    }

    /// Close the pool.
    ///
    /// # Errors
    /// Reports a pool that did not close cleanly.
    pub async fn close(self) -> Result<(), Error> {
        self.db.close().await.map_err(|_| Error::Unavailable)
    }
}

fn tag(row: &QueryResult) -> Result<Option<u32>, Error> {
    row.try_get::<Option<i32>>("", "tag")
        .map_err(|_| Error::Unavailable)?
        .map(|tag| u32::try_from(tag).map_err(|_| Error::Unavailable))
        .transpose()
}

fn candidate(row: &QueryResult) -> Result<Candidate, Error> {
    let salt: Vec<u8> = row
        .try_get("", "password_salt")
        .map_err(|_| Error::Unavailable)?;
    let hash: Zeroizing<Vec<u8>> = Zeroizing::new(
        row.try_get("", "password_hash")
            .map_err(|_| Error::Unavailable)?,
    );
    let mut stored = Zeroizing::new([0; HASH_BYTES]);
    if hash.len() != HASH_BYTES {
        return Err(Error::Unavailable);
    }
    stored.copy_from_slice(&hash);
    Ok(Candidate {
        legacy_id: row
            .try_get("", "legacy_id")
            .map_err(|_| Error::Unavailable)?,
        username: row
            .try_get("", "username_key")
            .map_err(|_| Error::Unavailable)?,
        display_name: row
            .try_get("", "display_name")
            .map_err(|_| Error::Unavailable)?,
        tag: tag(row)?,
        salt: salt.try_into().map_err(|_| Error::Unavailable)?,
        hash: stored,
        must_change_password: row
            .try_get("", "must_change_password")
            .map_err(|_| Error::Unavailable)?,
        banned: row.try_get("", "banned").map_err(|_| Error::Unavailable)?,
    })
}
