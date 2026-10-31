//! The cutover's import (`docs/operations.md` §"The legacy store"), under the
//! migrator login, once: the official gateway's export checked row by row,
//! stored in one transaction together with each account's Central identity,
//! reserved under its name so that nobody registers the name before the
//! account moves, and its count and digest computed again from what was
//! stored. Anything wrong refuses the whole import and stores nothing.
use super::statement;
use crate::accounts::username;
use central_api::{legacy::MAX_SNAPSHOT_BYTES, profile};
use central_crypto::legacy::{HASH_BYTES, SALT_BYTES};
use sea_orm::{ConnectionTrait, DatabaseConnection, DatabaseTransaction, TransactionTrait};
use sha2::{Digest, Sha256};

/// The most accounts one import takes: the gateway's export bound.
pub const MAX_LEGACY_ROWS: usize = 1000;

/// One account of the gateway's export (Baylee `docs/central-login.md`
/// §"Cutover"). No `Debug`: it holds a verifier.
pub struct LegacyRow {
    /// Its immutable id at the official gateway.
    pub legacy_id: String,
    /// Its username's key, as Central canonicalizes it.
    pub username: String,
    /// Its display name, only one that keeps the rule.
    pub display_name: Option<String>,
    /// Its tag.
    pub tag: Option<u32>,
    /// Argon2id salt under the gateway's parameters.
    pub salt: [u8; SALT_BYTES],
    /// Argon2id output under the gateway's parameters.
    pub hash: [u8; HASH_BYTES],
    /// The version of the gateway's terms it accepted.
    pub terms_version: Option<String>,
    /// The gateway required a new password.
    pub must_change_password: bool,
    /// A ban in force at the export: `Some(None)` for good, `Some(Some(t))`
    /// until `t`, Unix seconds.
    pub ban: Option<Option<i64>>,
    /// The source snapshot, exactly as the gateway exported it.
    pub snapshot: Vec<u8>,
}

/// What an import stored, computed from the stored rows.
#[derive(Debug, PartialEq, Eq)]
pub struct Imported {
    /// The rows stored.
    pub count: u64,
    /// Their digest, [`legacy_digest`].
    pub digest: [u8; 32],
}

/// Why an import stored nothing. A row is named by its legacy id alone.
#[derive(Debug, PartialEq, Eq)]
pub enum ImportRefusal {
    /// A field outside its bounds or rule.
    Malformed {
        /// The row.
        legacy_id: String,
        /// The field.
        field: &'static str,
    },
    /// Two rows with one legacy id or one username.
    Duplicate {
        /// The second row.
        legacy_id: String,
    },
    /// More than [`MAX_LEGACY_ROWS`].
    TooMany,
    /// The declared count is not the rows'.
    CountMismatch,
    /// The declared digest is not the rows', or not what was stored.
    DigestMismatch,
    /// The store holds rows already: the import runs once.
    StoreNotEmpty,
    /// Its move closed already at Central.
    AlreadyMoved {
        /// The row.
        legacy_id: String,
    },
    /// Another Central account holds its username.
    NameTaken {
        /// The row.
        legacy_id: String,
    },
    /// The database refused or was away.
    Unavailable,
}

impl std::fmt::Display for ImportRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed { legacy_id, field } => write!(f, "{legacy_id}: {field} malformed"),
            Self::Duplicate { legacy_id } => write!(f, "{legacy_id}: id or username twice"),
            Self::TooMany => write!(f, "more than {MAX_LEGACY_ROWS} accounts"),
            Self::CountMismatch => f.write_str("the declared count is not the accounts'"),
            Self::DigestMismatch => f.write_str("the digest does not match"),
            Self::StoreNotEmpty => f.write_str("the legacy store holds accounts already"),
            Self::AlreadyMoved { legacy_id } => write!(f, "{legacy_id}: moved already"),
            Self::NameTaken { legacy_id } => {
                write!(f, "{legacy_id}: another Central account holds its username")
            }
            Self::Unavailable => f.write_str("the database refused or is unavailable"),
        }
    }
}

/// SHA-256 over the count as eight big-endian bytes, then each row in the
/// order given (ascending `legacy_id` bytes), each field length-prefixed:
/// the gateway's export computes the same (Baylee `docs/central-login.md`
/// §"Cutover").
#[must_use]
pub fn legacy_digest(rows: &[LegacyRow]) -> [u8; 32] {
    fn field(hash: &mut Sha256, bytes: &[u8]) {
        // Every field is bounded far below 4 GiB before it gets here.
        let length = u32::try_from(bytes.len()).unwrap_or(u32::MAX);
        hash.update(length.to_be_bytes());
        hash.update(bytes);
    }
    fn optional(hash: &mut Sha256, bytes: Option<&[u8]>) {
        match bytes {
            None => hash.update([0]),
            Some(bytes) => {
                hash.update([1]);
                field(hash, bytes);
            }
        }
    }
    let mut hash = Sha256::new();
    hash.update((rows.len() as u64).to_be_bytes());
    for row in rows {
        field(&mut hash, row.legacy_id.as_bytes());
        field(&mut hash, row.username.as_bytes());
        optional(&mut hash, row.display_name.as_deref().map(str::as_bytes));
        let tag = row.tag.map(|tag| tag.to_string());
        optional(&mut hash, tag.as_deref().map(str::as_bytes));
        field(&mut hash, &row.salt);
        field(&mut hash, &row.hash);
        optional(&mut hash, row.terms_version.as_deref().map(str::as_bytes));
        hash.update([u8::from(row.must_change_password)]);
        let ban = row
            .ban
            .map(|until| until.map_or_else(|| "forever".to_owned(), |until| until.to_string()));
        optional(&mut hash, ban.as_deref().map(str::as_bytes));
        field(&mut hash, &row.snapshot);
    }
    hash.finalize().into()
}

fn check(row: &LegacyRow) -> Result<(), ImportRefusal> {
    let malformed = |field| ImportRefusal::Malformed {
        legacy_id: row.legacy_id.clone(),
        field,
    };
    if row.legacy_id.is_empty()
        || row.legacy_id.len() > 64
        || !row
            .legacy_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err(malformed("legacy_id"));
    }
    if username(&row.username).ok().as_deref() != Some(row.username.as_str()) {
        return Err(malformed("username"));
    }
    if row
        .display_name
        .as_deref()
        .is_some_and(|name| !profile::valid_display_name(name))
    {
        return Err(malformed("display_name"));
    }
    if row.tag.is_some_and(|tag| !profile::valid_tag(tag)) {
        return Err(malformed("tag"));
    }
    if row
        .terms_version
        .as_deref()
        .is_some_and(|version| version.is_empty() || version.len() > 64)
    {
        return Err(malformed("terms_version"));
    }
    if row.snapshot.len() < 2
        || row.snapshot.len() > MAX_SNAPSHOT_BYTES
        || std::str::from_utf8(&row.snapshot).is_err()
    {
        return Err(malformed("snapshot"));
    }
    Ok(())
}

/// Check every row, store them all with their reservations, and compute the
/// count and digest again from what was stored: all of it in one
/// transaction, which commits only when both equal the declared ones.
///
/// # Errors
/// Each [`ImportRefusal`]; nothing is stored then.
pub async fn import_legacy(
    db: &DatabaseConnection,
    mut rows: Vec<LegacyRow>,
    count: u64,
    digest: [u8; 32],
) -> Result<Imported, ImportRefusal> {
    if rows.len() > MAX_LEGACY_ROWS {
        return Err(ImportRefusal::TooMany);
    }
    if rows.len() as u64 != count {
        return Err(ImportRefusal::CountMismatch);
    }
    for row in &rows {
        check(row)?;
    }
    rows.sort_by(|a, b| a.legacy_id.as_bytes().cmp(b.legacy_id.as_bytes()));
    for pair in rows.windows(2) {
        if pair[0].legacy_id == pair[1].legacy_id {
            return Err(ImportRefusal::Duplicate {
                legacy_id: pair[1].legacy_id.clone(),
            });
        }
    }
    let mut names: Vec<&str> = rows.iter().map(|row| row.username.as_str()).collect();
    names.sort_unstable();
    if let Some(pair) = names.windows(2).find(|pair| pair[0] == pair[1]) {
        let legacy_id = rows
            .iter()
            .rfind(|row| row.username == pair[1])
            .map(|row| row.legacy_id.clone())
            .unwrap_or_default();
        return Err(ImportRefusal::Duplicate { legacy_id });
    }
    if legacy_digest(&rows) != digest {
        return Err(ImportRefusal::DigestMismatch);
    }
    let tx = db.begin().await.map_err(|_| ImportRefusal::Unavailable)?;
    let present = tx
        .query_one_raw(statement(
            "SELECT count(*)::bigint AS rows FROM central_legacy.accounts",
            vec![],
        ))
        .await
        .map_err(|_| ImportRefusal::Unavailable)?
        .ok_or(ImportRefusal::Unavailable)?
        .try_get::<i64>("", "rows")
        .map_err(|_| ImportRefusal::Unavailable)?;
    if present != 0 {
        return Err(ImportRefusal::StoreNotEmpty);
    }
    for row in &rows {
        store(&tx, row).await?;
    }
    let stored = stored(&tx).await?;
    let imported = Imported {
        count: stored.len() as u64,
        digest: legacy_digest(&stored),
    };
    if imported.count != count || imported.digest != digest {
        return Err(ImportRefusal::DigestMismatch);
    }
    tx.commit().await.map_err(|_| ImportRefusal::Unavailable)?;
    Ok(imported)
}

async fn store(tx: &DatabaseTransaction, row: &LegacyRow) -> Result<(), ImportRefusal> {
    // An abandoned registration gives the name up, as at a reservation.
    tx.execute_raw(statement(
        "DELETE FROM central_identity.accounts WHERE username_key = $1 AND source = 'registered'
         AND credential IS NULL AND enrollment_expires <= pg_catalog.clock_timestamp()",
        vec![row.username.clone().into()],
    ))
    .await
    .map_err(|_| ImportRefusal::Unavailable)?;
    let at_central = tx
        .query_one_raw(statement(
            "SELECT bool_or(legacy_closed AND legacy_id = $1) AS moved,
                 bool_or(legacy_id IS DISTINCT FROM $1) AS taken
             FROM central_identity.accounts WHERE legacy_id = $1 OR username_key = $2",
            vec![row.legacy_id.clone().into(), row.username.clone().into()],
        ))
        .await
        .map_err(|_| ImportRefusal::Unavailable)?
        .ok_or(ImportRefusal::Unavailable)?;
    let flag = |column| {
        at_central
            .try_get::<Option<bool>>("", column)
            .map(|value| value == Some(true))
            .map_err(|_| ImportRefusal::Unavailable)
    };
    if flag("moved")? {
        return Err(ImportRefusal::AlreadyMoved {
            legacy_id: row.legacy_id.clone(),
        });
    }
    if flag("taken")? {
        return Err(ImportRefusal::NameTaken {
            legacy_id: row.legacy_id.clone(),
        });
    }
    let tag = row.tag.map(i64::from);
    let (banned, until) = match row.ban {
        None => (false, None),
        Some(until) => (true, until),
    };
    tx.execute_raw(statement(
        "INSERT INTO central_legacy.accounts (legacy_id, username_key, display_name, tag,
             password_salt, password_hash, terms_version, must_change_password, banned,
             banned_until, snapshot, snapshot_digest)
         VALUES ($1, $2, $3, $4::integer, $5, $6, $7, $8, $9, pg_catalog.to_timestamp($10::bigint),
             $11, pg_catalog.sha256($11))",
        vec![
            row.legacy_id.clone().into(),
            row.username.clone().into(),
            row.display_name.clone().into(),
            tag.into(),
            row.salt.to_vec().into(),
            row.hash.to_vec().into(),
            row.terms_version.clone().into(),
            row.must_change_password.into(),
            banned.into(),
            until.into(),
            row.snapshot.clone().into(),
        ],
    ))
    .await
    .map_err(|_| ImportRefusal::Unavailable)?;
    // The reservation keeps the name for the account; its sign-in draws a
    // tag when another account holds the gateway's (`reserve_legacy`).
    tx.execute_raw(statement(
        "INSERT INTO central_identity.accounts (source, legacy_id, username_key, display_name,
             display_name_at, tag)
         SELECT 'legacy', $1, $2, $3, CASE WHEN $3::text IS NULL THEN NULL ELSE pg_catalog.clock_timestamp() END,
             CASE WHEN EXISTS (SELECT 1 FROM central_identity.accounts WHERE tag = $4::integer)
                 THEN NULL ELSE $4::integer END
         ON CONFLICT (legacy_id) DO NOTHING",
        vec![
            row.legacy_id.clone().into(),
            row.username.clone().into(),
            row.display_name.clone().into(),
            tag.into(),
        ],
    ))
    .await
    .map_err(|_| ImportRefusal::Unavailable)?;
    Ok(())
}

/// Every stored row, in the digest's order.
pub(super) async fn stored(db: &impl ConnectionTrait) -> Result<Vec<LegacyRow>, ImportRefusal> {
    let rows = db
        .query_all_raw(statement(
            "SELECT legacy_id, username_key, display_name, tag, password_salt, password_hash,
                 terms_version, must_change_password, banned,
                 floor(extract(epoch FROM banned_until))::bigint AS banned_until, snapshot
             FROM central_legacy.accounts ORDER BY legacy_id COLLATE \"C\"",
            vec![],
        ))
        .await
        .map_err(|_| ImportRefusal::Unavailable)?;
    rows.iter()
        .map(|row| {
            let get = |column: &str| -> Result<Vec<u8>, ImportRefusal> {
                row.try_get("", column)
                    .map_err(|_| ImportRefusal::Unavailable)
            };
            let tag: Option<i32> = row
                .try_get("", "tag")
                .map_err(|_| ImportRefusal::Unavailable)?;
            let banned: bool = row
                .try_get("", "banned")
                .map_err(|_| ImportRefusal::Unavailable)?;
            let until: Option<i64> = row
                .try_get("", "banned_until")
                .map_err(|_| ImportRefusal::Unavailable)?;
            Ok(LegacyRow {
                legacy_id: row
                    .try_get("", "legacy_id")
                    .map_err(|_| ImportRefusal::Unavailable)?,
                username: row
                    .try_get("", "username_key")
                    .map_err(|_| ImportRefusal::Unavailable)?,
                display_name: row
                    .try_get("", "display_name")
                    .map_err(|_| ImportRefusal::Unavailable)?,
                tag: tag
                    .map(u32::try_from)
                    .transpose()
                    .map_err(|_| ImportRefusal::Unavailable)?,
                salt: get("password_salt")?
                    .try_into()
                    .map_err(|_| ImportRefusal::Unavailable)?,
                hash: get("password_hash")?
                    .try_into()
                    .map_err(|_| ImportRefusal::Unavailable)?,
                terms_version: row
                    .try_get("", "terms_version")
                    .map_err(|_| ImportRefusal::Unavailable)?,
                must_change_password: row
                    .try_get("", "must_change_password")
                    .map_err(|_| ImportRefusal::Unavailable)?,
                ban: banned.then_some(until),
                snapshot: get("snapshot")?,
            })
        })
        .collect()
}
