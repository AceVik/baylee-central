//! AI runner tokens (migration 000054, `docs/ai-players.md` §2): an
//! administrator's credential for a machine that runs AI players, and the
//! exchange that trades it for a native session of one AI account it lists.
//! Only its digest is stored or read, and no token is ever deleted. Whether
//! one is live and lists the account is asked by the statement that takes
//! its row, inside the exchange's transaction: nothing of it is remembered
//! between two requests. Unlike a report token it does not end with its
//! maker's role.
use super::admin::{Action, id, limit, optional_id};
use super::sessions::delete_session_tx;
use super::tokens::LIVE_TOKEN;
use super::{AccountStore, Error, Token, identity, statement, uuid};
use central_api::ai_runner::{
    AiRunnerAccount, AiRunnerTokenInfo, LISTED_TOKENS, LIVE_TOKENS, MAX_SESSIONS, SIGNIN_DAYS,
    TOKENS_PER_DAY, validate_new,
};
use central_api::wire::Identity;
use sea_orm::{ConnectionTrait, DatabaseTransaction, QueryResult, TransactionTrait, prelude::Uuid};
use std::collections::BTreeMap;
use subtle::ConstantTimeEq;

/// The live runner token `t` whose digest is `$1` and which lists the
/// account `$2`. Its row is held until the exchange ends: a revocation
/// waits for the exchange and then ends its session too, and an exchange
/// that waited for a revocation finds no row. `NO KEY UPDATE` rather than
/// `SHARE`: the exchange updates the row's last use, and two exchanges that
/// each held a share would wait for each other there.
const EXCHANGED: &str = "SELECT t.id, t.digest FROM central_identity.ai_runner_tokens t
    JOIN central_identity.ai_runner_accounts r ON r.token_id = t.id AND r.account_id = $2
    WHERE t.digest = $1 AND t.revoked_at IS NULL AND t.expires_at > pg_catalog.clock_timestamp()
    FOR NO KEY UPDATE OF t";

/// The columns of an [`AiRunnerTokenInfo`] but its accounts, from
/// `ai_runner_tokens t` with its maker `c` and its revoker `v` left-joined
/// from the accounts.
const INFO_COLUMNS: &str = "t.id, t.name,
    t.created_by, c.display_name AS created_by_name, c.tag AS created_by_tag,
    t.revoked_by, v.display_name AS revoked_by_name, v.tag AS revoked_by_tag,
    floor(extract(epoch FROM t.created_at))::bigint AS created_at,
    floor(extract(epoch FROM t.expires_at))::bigint AS expires_at,
    floor(extract(epoch FROM t.revoked_at))::bigint AS revoked_at,
    floor(extract(epoch FROM t.last_used_at))::bigint AS last_used_at";

/// A runner session just minted.
pub struct RunnerSession {
    /// The AI account and its vault epoch.
    pub identity: Identity,
    /// Absolute Unix-seconds expiry, 30 minutes after the exchange; renewed
    /// by [`AccountStore::renew_session`] up to 12 hours after it.
    pub expires_at: i64,
    /// The digests of the account's runner sessions the exchange ended: the
    /// expired ones, and the oldest past [`MAX_SESSIONS`].
    pub ended: Vec<[u8; 32]>,
}

fn unavailable(_: impl Sized) -> Error {
    Error::Unavailable
}

fn digest(row: &QueryResult) -> Result<[u8; 32], Error> {
    let bytes: Vec<u8> = row.try_get("", "digest").map_err(unavailable)?;
    bytes.as_slice().try_into().map_err(unavailable)
}

fn info(row: &QueryResult, accounts: Vec<AiRunnerAccount>) -> Result<AiRunnerTokenInfo, Error> {
    let tag = |column| {
        row.try_get::<Option<i32>>("", column)
            .map_err(unavailable)?
            .map(|tag| u32::try_from(tag).map_err(unavailable))
            .transpose()
    };
    let time = |column| row.try_get::<Option<i64>>("", column).map_err(unavailable);
    Ok(AiRunnerTokenInfo {
        id: id(row, "id")?,
        name: row.try_get("", "name").map_err(unavailable)?,
        accounts,
        created_by: optional_id(row, "created_by")?,
        created_by_name: row.try_get("", "created_by_name").map_err(unavailable)?,
        created_by_tag: tag("created_by_tag")?,
        created_at: time("created_at")?.ok_or(Error::Unavailable)?,
        expires_at: time("expires_at")?.ok_or(Error::Unavailable)?,
        revoked_at: time("revoked_at")?,
        revoked_by: optional_id(row, "revoked_by")?,
        revoked_by_name: row.try_get("", "revoked_by_name").map_err(unavailable)?,
        revoked_by_tag: tag("revoked_by_tag")?,
        last_used_at: time("last_used_at")?,
    })
}

/// Write one audit event about a runner token in `db`'s transaction: its
/// id, never its name or its digest; an administrator's, or the operator's
/// (`actor` none).
async fn audit(
    db: &impl ConnectionTrait,
    actor: Option<Uuid>,
    action: Action,
    token: Uuid,
) -> Result<(), Error> {
    db.execute_raw(statement(
        "INSERT INTO central_identity.audit_events (actor_kind, actor_account, action, ai_runner_token_id)
         VALUES (CASE WHEN $1::uuid IS NULL THEN 'operator' ELSE 'admin' END, $1, $2, $3)",
        vec![actor.into(), action.as_str().into(), token.into()],
    ))
    .await
    .map_err(unavailable)?;
    Ok(())
}

/// End the sessions with these digests in `db`'s transaction, their push
/// subscriptions first, as a sign-out ends one.
async fn end(db: &impl ConnectionTrait, digests: &[[u8; 32]]) -> Result<(), Error> {
    for digest in digests {
        delete_session_tx(db, digest).await?;
    }
    Ok(())
}

/// Of the account's runner sessions, newest first with whether each is
/// live, those an exchange ends: the expired ones, and every live one past
/// the newest `MAX_SESSIONS - 1`, so that with the new one there are never
/// more than [`MAX_SESSIONS`].
fn past_the_cap(sessions: impl IntoIterator<Item = ([u8; 32], bool)>) -> Vec<[u8; 32]> {
    let mut kept = 0;
    let mut ended = Vec::new();
    for (digest, live) in sessions {
        if live && kept < MAX_SESSIONS - 1 {
            kept += 1;
        } else {
            ended.push(digest);
        }
    }
    ended
}

/// Step 3 of the exchange: end the account's expired runner sessions and
/// the oldest past [`MAX_SESSIONS`], of every token together, holding the
/// rest. Answers the digests it ended.
async fn end_past_the_cap(tx: &DatabaseTransaction, account: Uuid) -> Result<Vec<[u8; 32]>, Error> {
    let held = tx
        .query_all_raw(statement(
            "SELECT digest, expires_at > pg_catalog.clock_timestamp() AS live
             FROM central_identity.sessions
             WHERE account_id = $1 AND runner_token IS NOT NULL
             ORDER BY created_at DESC, digest DESC FOR UPDATE",
            vec![account.into()],
        ))
        .await
        .map_err(unavailable)?
        .iter()
        .map(|row| {
            let live = row.try_get::<bool>("", "live").map_err(unavailable)?;
            Ok((digest(row)?, live))
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let ended = past_the_cap(held);
    end(tx, &ended).await?;
    Ok(ended)
}

/// Step 4: the new session, native, naming the token that minted it, 30
/// minutes from now. Answers its expiry in Unix seconds.
async fn mint(
    tx: &DatabaseTransaction,
    session: &Token,
    account: Uuid,
    revision: i64,
    token: Uuid,
) -> Result<i64, Error> {
    tx.query_one_raw(statement(
        "INSERT INTO central_identity.sessions
           (digest, account_id, credential_revision, kind, created_at, expires_at, runner_token)
         SELECT $1, $2, $3, 'native', t,
             -- never past the token's own end (docs/ai-players.md §2)
             LEAST(t + interval '30 minutes', (SELECT r.expires_at
                 FROM central_identity.ai_runner_tokens r WHERE r.id = $4)),
             $4
         FROM (SELECT pg_catalog.clock_timestamp() AS t) clock
         RETURNING floor(extract(epoch FROM expires_at))::bigint AS expires_at",
        vec![
            session.digest().to_vec().into(),
            account.into(),
            revision.into(),
            token.into(),
        ],
    ))
    .await
    .map_err(unavailable)?
    .ok_or(Error::Unavailable)?
    .try_get("", "expires_at")
    .map_err(unavailable)
}

/// Step 5: the token's last use, the account's last sign-in, a row of
/// sign-ins, and that token's sign-ins older than [`SIGNIN_DAYS`] pruned.
async fn record(tx: &DatabaseTransaction, token: Uuid, account: Uuid) -> Result<(), Error> {
    let steps: [(&str, Vec<sea_orm::Value>); 4] = [
        (
            "UPDATE central_identity.ai_runner_tokens
             SET last_used_at = pg_catalog.clock_timestamp() WHERE id = $1",
            vec![token.into()],
        ),
        (
            "UPDATE central_identity.accounts
             SET last_sign_in_at = pg_catalog.clock_timestamp() WHERE id = $1",
            vec![account.into()],
        ),
        (
            "INSERT INTO central_identity.ai_runner_signins (token_id, account_id)
             VALUES ($1, $2)",
            vec![token.into(), account.into()],
        ),
        (
            "DELETE FROM central_identity.ai_runner_signins
             WHERE token_id = $1
             AND at < pg_catalog.clock_timestamp() - pg_catalog.make_interval(days => $2)",
            vec![token.into(), i32::from(SIGNIN_DAYS).into()],
        ),
    ];
    for (sql, values) in steps {
        tx.execute_raw(statement(sql, values))
            .await
            .map_err(unavailable)?;
    }
    Ok(())
}

impl AccountStore {
    /// Trade the runner token `token` for a native session of `account`,
    /// stored under `session`'s digest (`docs/ai-players.md` §2), in one
    /// transaction: the token's row, live and listing the account, held to
    /// the end and its digest compared in constant time; the account's row,
    /// enrolled and an AI player, locked; its expired runner sessions and
    /// the oldest past [`MAX_SESSIONS`] ended; the new session, 30 minutes,
    /// naming the token; the token's last use, the account's last sign-in
    /// and a row of sign-ins written, and that token's sign-ins older than
    /// [`SIGNIN_DAYS`] pruned. Whether the AI is switched on is not asked:
    /// a switched-off AI still rolls over to end its running matches.
    ///
    /// # Errors
    /// `Rejected` for an unknown, revoked or expired token, an account it
    /// does not list and an account that is no AI any more, all alike;
    /// storage failure.
    pub async fn ai_runner_session(
        &self,
        token: &Token,
        account: [u8; 16],
        session: &Token,
    ) -> Result<RunnerSession, Error> {
        let presented = token.digest();
        let account = uuid(account);
        let tx = self.db.begin().await.map_err(unavailable)?;
        let row = tx
            .query_one_raw(statement(
                EXCHANGED,
                vec![presented.to_vec().into(), account.into()],
            ))
            .await
            .map_err(unavailable)?
            .ok_or(Error::Rejected)?;
        if !bool::from(digest(&row)?.ct_eq(&presented)) {
            return Err(Error::Rejected);
        }
        let token_id: Uuid = row.try_get("", "id").map_err(unavailable)?;
        let row = tx
            .query_one_raw(statement(
                "SELECT a.id, a.vault_epoch, a.credential_revision
                 FROM central_identity.accounts a
                 JOIN central_identity.ai_players p ON p.account_id = a.id
                 WHERE a.id = $1 AND a.credential IS NOT NULL FOR UPDATE OF a",
                vec![account.into()],
            ))
            .await
            .map_err(unavailable)?
            .ok_or(Error::Rejected)?;
        let identity = identity(&row)?;
        let revision: i64 = row
            .try_get("", "credential_revision")
            .map_err(unavailable)?;
        let ended = end_past_the_cap(&tx, account).await?;
        let expires_at = mint(&tx, session, account, revision, token_id).await?;
        record(&tx, token_id, account).await?;
        tx.commit().await.map_err(unavailable)?;
        Ok(RunnerSession {
            identity,
            expires_at,
            ended,
        })
    }

    /// The AI account whose runner holds the native session `session`:
    /// live, minted by a runner token that is neither revoked nor expired,
    /// of an enrolled account that is an AI player. `None` for every other
    /// session, a person's among them. Central's relay keeps the answer with
    /// the socket ticket it issues for the session, so that a runner's relay
    /// sockets are counted in a budget of their own and not against its
    /// address (`docs/ai-players.md`, "More AI seats at once"); nothing a
    /// client sends decides it.
    ///
    /// # Errors
    /// Storage failure.
    pub async fn relay_runner(&self, session: &Token) -> Result<Option<[u8; 16]>, Error> {
        let row = self
            .db
            .query_one_raw(statement(
                "SELECT s.account_id FROM central_identity.sessions s
                 JOIN central_identity.accounts a
                   ON a.id = s.account_id AND a.credential_revision = s.credential_revision
                 JOIN central_identity.ai_players p ON p.account_id = s.account_id
                 JOIN central_identity.ai_runner_tokens t ON t.id = s.runner_token
                 WHERE s.digest = $1 AND s.kind = 'native'
                   AND s.expires_at > pg_catalog.clock_timestamp()
                   AND a.credential IS NOT NULL
                   AND t.revoked_at IS NULL AND t.expires_at > pg_catalog.clock_timestamp()",
                vec![session.digest().to_vec().into()],
            ))
            .await
            .map_err(unavailable)?;
        row.map(|row| {
            row.try_get::<Uuid>("", "account_id")
                .map(Uuid::into_bytes)
                .map_err(unavailable)
        })
        .transpose()
    }

    /// The runner tokens for the administrator behind `browser`: the live
    /// ones first, then those revoked or expired in the last 30 days, each
    /// group newest first, with the accounts each lists.
    ///
    /// # Errors
    /// `Rejected` without an administrator's live browser session; storage
    /// failure.
    pub async fn admin_ai_runner_tokens(
        &self,
        browser: &Token,
    ) -> Result<Vec<AiRunnerTokenInfo>, Error> {
        let tx = self.db.begin().await.map_err(unavailable)?;
        Self::administrator(&tx, browser).await?;
        let tokens = listed(&tx).await?;
        tx.commit().await.map_err(unavailable)?;
        Ok(tokens)
    }

    /// Store the digest of `token` as a runner token named `name`, listing
    /// `accounts`, for `days`, made by the administrator behind `browser`,
    /// whose password sign-in must be less than 30 minutes old. Answers its
    /// id and its expiry in Unix seconds; the audit event is written in the
    /// same transaction.
    ///
    /// # Errors
    /// `Rejected` without a fresh administrator's browser session, for a
    /// malformed name, accounts that are not 1 to 16 distinct AI players or
    /// a lifetime out of 1 to 366 days; `Busy` past [`LIVE_TOKENS`] live
    /// tokens or [`TOKENS_PER_DAY`] new ones in a day; storage failure.
    pub async fn create_ai_runner_token(
        &self,
        browser: &Token,
        name: &str,
        accounts: &[[u8; 16]],
        days: u16,
        token: &Token,
    ) -> Result<([u8; 16], i64), Error> {
        validate_new(name, accounts, days)?;
        // Every administrator's row is locked, so two makings are counted
        // one after the other.
        let (tx, actor) = self.acting_administrator(browser).await?;
        let made = make(&tx, Some(actor), name, accounts, days, token).await?;
        tx.commit().await.map_err(unavailable)?;
        Ok(made)
    }

    /// Revoke the runner token `token` for the administrator behind
    /// `browser`, and end every session it minted, in one transaction. It
    /// needs no fresh sign-in: it only takes exposure away. One revoked
    /// already changes nothing; a revocation writes its audit event in the
    /// same transaction. Answers the digests of the sessions it ended, for
    /// the push sockets.
    ///
    /// # Errors
    /// `Rejected` without an administrator's live browser session or for an
    /// unknown token; storage failure.
    pub async fn revoke_ai_runner_token(
        &self,
        browser: &Token,
        token: [u8; 16],
    ) -> Result<Vec<[u8; 32]>, Error> {
        let (tx, actor) = self.acting_administrator_fresh(browser, false).await?;
        let ended = revoke(&tx, Some(actor), uuid(token)).await?;
        tx.commit().await.map_err(unavailable)?;
        Ok(ended)
    }
}

/// The runner tokens in `db`: the live ones first, then those revoked or
/// expired in the last 30 days, each group newest first, with the accounts
/// each lists. Never a digest.
///
/// # Errors
/// Storage failure.
pub(crate) async fn listed(db: &impl ConnectionTrait) -> Result<Vec<AiRunnerTokenInfo>, Error> {
    let rows = db
        .query_all_raw(statement(
            &format!(
                "SELECT {INFO_COLUMNS}
                 FROM central_identity.ai_runner_tokens t
                 LEFT JOIN central_identity.accounts c ON c.id = t.created_by
                 LEFT JOIN central_identity.accounts v ON v.id = t.revoked_by
                 WHERE COALESCE(t.revoked_at, t.expires_at) > pg_catalog.clock_timestamp() - interval '30 days'
                 ORDER BY ({LIVE_TOKEN}) DESC, t.id DESC LIMIT $1"
            ),
            vec![limit(LISTED_TOKENS)],
        ))
        .await
        .map_err(unavailable)?;
    let ids = rows
        .iter()
        .map(|row| row.try_get::<Uuid>("", "id").map_err(unavailable))
        .collect::<Result<Vec<_>, Error>>()?;
    let mut listed: BTreeMap<[u8; 16], Vec<AiRunnerAccount>> = BTreeMap::new();
    for row in db
        .query_all_raw(statement(
            "SELECT r.token_id, a.id, a.display_name, a.tag
             FROM central_identity.ai_runner_accounts r
             JOIN central_identity.accounts a ON a.id = r.account_id
             WHERE r.token_id = ANY($1) ORDER BY r.token_id, a.id",
            vec![ids.into()],
        ))
        .await
        .map_err(unavailable)?
    {
        let (display_name, tag) = super::profiles::columns(&row)?;
        listed
            .entry(id(&row, "token_id")?)
            .or_default()
            .push(AiRunnerAccount {
                id: id(&row, "id")?,
                display_name,
                tag,
            });
    }
    rows.iter()
        .map(|row| info(row, listed.remove(&id(row, "id")?).unwrap_or_default()))
        .collect()
}

/// Make the runner token whose digest is `token`'s in `tx`, for an
/// administrator or the operator (`actor` none), after the caller locked
/// every administrator's row, so that two makings are counted one after
/// the other: the caps, every listed account an AI player, the token and
/// its list, and the audit event. The caller validated `name`, `accounts`
/// and `days` ([`validate_new`]). Answers its id and expiry.
///
/// # Errors
/// `Rejected` for an account that is no AI player; `Busy` past
/// [`LIVE_TOKENS`] live tokens or [`TOKENS_PER_DAY`] new ones in a day;
/// storage failure.
pub(crate) async fn make(
    tx: &DatabaseTransaction,
    actor: Option<Uuid>,
    name: &str,
    accounts: &[[u8; 16]],
    days: u16,
    token: &Token,
) -> Result<([u8; 16], i64), Error> {
    validate_new(name, accounts, days)?;
    let counts = tx
        .query_one_raw(statement(
            &format!(
                "SELECT count(*) FILTER (WHERE {LIVE_TOKEN})::bigint AS live,
                     count(*) FILTER (WHERE t.created_at > pg_catalog.clock_timestamp() - interval '1 day')::bigint AS today
                 FROM central_identity.ai_runner_tokens t"
            ),
            vec![],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Unavailable)?;
    let full = |column, most: usize| {
        counts
            .try_get::<i64>("", column)
            .map(|held| usize::try_from(held).unwrap_or(usize::MAX) >= most)
            .map_err(unavailable)
    };
    if full("live", LIVE_TOKENS)? || full("today", TOKENS_PER_DAY)? {
        return Err(Error::Busy);
    }
    // Each must be an AI player, kept one until the listing is written.
    let ids: Vec<Uuid> = accounts.iter().map(|account| uuid(*account)).collect();
    let players = tx
        .query_all_raw(statement(
            "SELECT account_id FROM central_identity.ai_players
             WHERE account_id = ANY($1) ORDER BY account_id FOR KEY SHARE",
            vec![ids.clone().into()],
        ))
        .await
        .map_err(unavailable)?;
    if players.len() != ids.len() {
        return Err(Error::Rejected);
    }
    // One clock reading for both ends, so a 366-day token meets the
    // table's bound exactly rather than by a few microseconds over.
    let row = tx
        .query_one_raw(statement(
            "WITH now AS (SELECT pg_catalog.clock_timestamp() AS at)
             INSERT INTO central_identity.ai_runner_tokens
                 (name, digest, created_by, created_at, expires_at)
             SELECT $1, $2, $3, now.at, now.at + pg_catalog.make_interval(days => $4) FROM now
             RETURNING id, floor(extract(epoch FROM expires_at))::bigint AS expires_at",
            vec![
                name.into(),
                token.digest().to_vec().into(),
                actor.into(),
                i32::from(days).into(),
            ],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Unavailable)?;
    let made: Uuid = row.try_get("", "id").map_err(unavailable)?;
    let expires_at: i64 = row.try_get("", "expires_at").map_err(unavailable)?;
    tx.execute_raw(statement(
        "INSERT INTO central_identity.ai_runner_accounts (token_id, account_id)
         SELECT $1, pg_catalog.unnest($2::uuid[])",
        vec![made.into(), ids.into()],
    ))
    .await
    .map_err(unavailable)?;
    audit(tx, actor, Action::AiRunnerTokenCreated, made).await?;
    Ok((made.into_bytes(), expires_at))
}

/// Revoke the runner token `token` in `tx`, for an administrator or the
/// operator (`actor` none), and end every session it minted. One revoked
/// already changes nothing. Answers the digests of the sessions it ended.
///
/// # Errors
/// `Rejected` for an unknown token; storage failure.
pub(crate) async fn revoke(
    tx: &DatabaseTransaction,
    actor: Option<Uuid>,
    token: Uuid,
) -> Result<Vec<[u8; 32]>, Error> {
    // Waits for an exchange that holds the row; one that comes later
    // waits for this and finds the token revoked.
    let revoked = tx
        .execute_raw(statement(
            "UPDATE central_identity.ai_runner_tokens
             SET revoked_at = pg_catalog.clock_timestamp(), revoked_by = $2
             WHERE id = $1 AND revoked_at IS NULL",
            vec![token.into(), actor.into()],
        ))
        .await
        .map_err(unavailable)?
        .rows_affected()
        == 1;
    if !revoked {
        tx.query_one_raw(statement(
            "SELECT 1 AS known FROM central_identity.ai_runner_tokens WHERE id = $1",
            vec![token.into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Rejected)?;
        return Ok(Vec::new());
    }
    // A statement of its own, after the update: it sees the session of
    // an exchange the update waited for.
    let ended = tx
        .query_all_raw(statement(
            "SELECT digest FROM central_identity.sessions WHERE runner_token = $1
             ORDER BY digest FOR UPDATE",
            vec![token.into()],
        ))
        .await
        .map_err(unavailable)?
        .iter()
        .map(digest)
        .collect::<Result<Vec<_>, Error>>()?;
    end(tx, &ended).await?;
    audit(tx, actor, Action::AiRunnerTokenRevoked, token).await?;
    Ok(ended)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The exchange's statement asks for everything that makes a token good
    /// and holds the row without waiting on itself.
    #[test]
    fn the_exchange_asks_for_the_life_and_the_listing_and_holds_the_row() {
        for asked in [
            "t.digest = $1",
            "r.account_id = $2",
            "t.revoked_at IS NULL",
            "t.expires_at > pg_catalog.clock_timestamp()",
        ] {
            assert!(EXCHANGED.contains(asked), "{asked}");
        }
        assert!(EXCHANGED.ends_with("FOR NO KEY UPDATE OF t"));
        assert!(!INFO_COLUMNS.contains("digest"));
    }

    /// Expired sessions go, and the live ones past the newest three; the
    /// new one makes four.
    #[test]
    fn an_exchange_ends_the_expired_and_the_oldest_past_the_cap() {
        let at = |n: u8| [n; 32];
        assert_eq!(past_the_cap([]), Vec::<[u8; 32]>::new());
        assert_eq!(
            past_the_cap([(at(3), true), (at(2), true), (at(1), true)]),
            Vec::<[u8; 32]>::new()
        );
        assert_eq!(
            past_the_cap([(at(4), true), (at(3), true), (at(2), true), (at(1), true)]),
            vec![at(1)]
        );
        assert_eq!(
            past_the_cap([
                (at(6), true),
                (at(5), false),
                (at(4), true),
                (at(3), true),
                (at(2), true),
                (at(1), false),
            ]),
            vec![at(5), at(2), at(1)]
        );
        assert_eq!(
            past_the_cap([(at(2), false), (at(1), false)]),
            vec![at(2), at(1)]
        );
    }
}
