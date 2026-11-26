//! The operator's two steps after a restore of an older backup
//! (`docs/account-deletion.md` §"The operator's erasure",
//! `docs/operations.md` §"After a restore"): erasing again, by id, the
//! accounts deleted since that backup (GDPR Art. 17), and moving every
//! gateway's revocation stream past each number issued before it. Both run
//! under the migrator login, as the sweep does, from `central-service`'s
//! operator modes; no listener reaches them, and they add no authority the
//! migrator login does not hold already.
use super::{
    Error,
    admin::{Action, audit},
    deletion, statement,
};
use sea_orm::{ConnectionTrait, DatabaseConnection, TransactionTrait, prelude::Uuid};

/// The most ids one erasure document names; a longer list goes in batches.
pub const ERASE_BATCH: usize = 1024;

/// How far each revocation stream moves after a restore: `2^40`, beyond any
/// number of revocations one gateway could be given between a backup and its
/// restore. A backup is kept twelve months at most; filling the step in that
/// time takes 35,000 revocations at one gateway every second of the year,
/// each a row of its own and each the end of a session it was given.
pub const RESTORE_STEP: i64 = 1 << 40;

/// The highest number a stream may reach: `2^53 - 1`, which every JSON
/// reader holds exactly. It leaves room for 8,191 restores.
pub const MAX_REVOCATION: i64 = (1 << 53) - 1;

/// What one erasure found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Erased {
    /// The account was there and is gone now.
    Erased,
    /// No account has this id: erased before, or never one.
    Absent,
}

/// Erase the account `account` with everything that names it, in one
/// transaction, as its own deletion does (`AccountStore::delete_account`):
/// under the account row's lock, the gateways of others that it administers
/// move their list's revision and the foreign keys take the rest, its
/// sessions included, each `sid` of them a revocation its gateway hears. Two
/// things more: the legacy store's row of its old id, which only the
/// migrator may delete, and an audit event with the operator as its actor.
///
/// Nothing is refused for its state: a restored backup can show an account
/// as an administrator or with its move still open, which the person had
/// left before they deleted it. An administrator erased this way can leave
/// Central without one; [`administrators`] says so, and the operator
/// appoints another (`--appoint-admin-stdin`).
///
/// # Errors
/// Reports storage failure; nothing is erased then.
pub async fn erase_account(db: &DatabaseConnection, account: [u8; 16]) -> Result<Erased, Error> {
    let account = Uuid::from_bytes(account);
    let tx = db.begin().await.map_err(|_| Error::Unavailable)?;
    let Some(row) = tx
        .query_one_raw(statement(
            "SELECT legacy_id FROM central_identity.accounts WHERE id = $1 FOR UPDATE",
            vec![account.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
    else {
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        return Ok(Erased::Absent);
    };
    let legacy: Option<String> = row
        .try_get("", "legacy_id")
        .map_err(|_| Error::Unavailable)?;
    if let Some(legacy) = legacy {
        // An open move keeps the old password's hash and the source here;
        // a closed one whose row's deletion was lost does too.
        tx.execute_raw(statement(
            "DELETE FROM central_legacy.accounts WHERE legacy_id = $1",
            vec![legacy.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
    }
    deletion::remove(&tx, account).await?;
    audit(&tx, None, Action::AccountErased, account).await?;
    tx.commit().await.map_err(|_| Error::Unavailable)?;
    Ok(Erased::Erased)
}

/// How many accounts hold the administrator role.
///
/// # Errors
/// Reports storage failure.
pub async fn administrators(db: &DatabaseConnection) -> Result<u64, Error> {
    let count: i64 = db
        .query_one_raw(statement(
            "SELECT count(*) AS n FROM central_identity.account_roles WHERE role = 'admin'",
            vec![],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::Unavailable)?
        .try_get("", "n")
        .map_err(|_| Error::Unavailable)?;
    u64::try_from(count).map_err(|_| Error::Unavailable)
}

/// After a restore, move every gateway's revocation stream [`RESTORE_STEP`]
/// past its last number, in one transaction, and return how many streams
/// moved.
///
/// A gateway keeps the last number it applied. Restored, Central would number
/// new revocations again from the backup's last one, and a gateway further on
/// would take them as known and never read them. After the step, every
/// stream's lowest held number is its last plus one, so each gateway that had
/// read any number finds it missed something it cannot learn, ends every
/// Central session it holds and goes on from there (`docs/gateway-registry.md`
/// §"Revocations"); clients present fresh tokens, which Central gives only to
/// live sessions. The numbers still held are dropped for that reason: one
/// left below the new last would hide the gap. A gateway without a stream
/// row yet, the official one or a registered one, gets one, since it may have
/// read numbers the backup never saw.
///
/// Central's own sessions stay: a backup holds none, and ending them would
/// turn a mistaken run into a password for everyone instead of a fresh token.
///
/// # Errors
/// Rejects a stream that would pass [`MAX_REVOCATION`], changing nothing;
/// reports storage failure.
pub async fn step_revocations(db: &DatabaseConnection, official: &str) -> Result<u64, Error> {
    let tx = db.begin().await.map_err(|_| Error::Unavailable)?;
    tx.execute_raw(statement(
        "INSERT INTO central_identity.gateway_streams(origin)
         SELECT origin FROM central_identity.gateways WHERE deleted_at IS NULL
         UNION SELECT $1
         ON CONFLICT (origin) DO NOTHING",
        vec![official.into()],
    ))
    .await
    .map_err(|_| Error::Unavailable)?;
    // In the order a session's end takes them (`lock_session_streams`), so
    // that neither waits on the other in a circle.
    let streams = tx
        .query_all_raw(statement(
            "SELECT last_seq FROM central_identity.gateway_streams ORDER BY origin FOR UPDATE",
            vec![],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
    for stream in &streams {
        let last: i64 = stream
            .try_get("", "last_seq")
            .map_err(|_| Error::Unavailable)?;
        if last > MAX_REVOCATION - RESTORE_STEP {
            return Err(Error::Rejected);
        }
    }
    tx.execute_raw(statement(
        "DELETE FROM central_identity.gateway_revocations",
        vec![],
    ))
    .await
    .map_err(|_| Error::Unavailable)?;
    let moved = tx
        .execute_raw(statement(
            "UPDATE central_identity.gateway_streams
             SET last_seq = last_seq + $1, pushed_seq = last_seq + $1,
                 attempts = 0, next_attempt_at = NULL",
            vec![RESTORE_STEP.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .rows_affected();
    if Ok(moved) != u64::try_from(streams.len()) {
        return Err(Error::Unavailable);
    }
    tx.commit().await.map_err(|_| Error::Unavailable)?;
    Ok(moved)
}
