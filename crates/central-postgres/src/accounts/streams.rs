//! Sessions as gateways know them, and what each gateway must learn when one ends.
//!
//! A `sid` is issued here; the migration's trigger turns every end of one into a numbered
//! revocation, so no path that ends a session can forget. Nothing here signs: the identity
//! service does, and the sender takes the page it is given.
use super::{AccountStore, Error, NOT_DISABLED, Token, any_origin, number, origins, statement};
use central_api::{gateway::valid_invalidate_path, terms::TermsVersion, wire::GatewayPurpose};
use sea_orm::{ConnectionTrait, DatabaseTransaction, TransactionTrait, prelude::Uuid};

/// Most `sid`s one session holds: each gateway and purpose is one.
pub const MAX_SIDS_PER_SESSION: i64 = 32;
/// Most numbers one revocation message covers.
pub const REVOCATION_PAGE: i64 = 256;
/// How many times a gateway is called for one page before catch-up is left to carry it.
pub const PUSH_ATTEMPTS: i16 = 5;

/// What a token for one gateway says about its account.
pub struct GatewayGrant {
    /// The session as this gateway knows it.
    pub sid: [u8; 16],
    /// The account.
    pub account: [u8; 16],
    /// The session's 12-hour cap, in Unix seconds. No token outlives it.
    pub cap: i64,
    /// The account's display name, while it has one.
    pub display_name: Option<String>,
    /// The account's tag, while it has one.
    pub tag: Option<u32>,
    /// The official gateway's old id, for a closed move only.
    pub legacy_id: Option<String>,
    /// This gateway's terms the account accepted last.
    pub terms: Option<TermsVersion>,
}

/// One gateway's position, and the next numbers after the one asked for.
pub struct RevocationPage {
    /// The first number of `sids`, when there are any.
    pub from: u64,
    /// The `sid`s, in order, one for each number from `from`.
    pub sids: Vec<[u8; 16]>,
    /// The gateway's last number.
    pub last: u64,
    /// The lowest number held, or `last + 1` when none is.
    pub floor: u64,
}

/// A page claimed for sending to one gateway.
pub struct GatewayPush {
    /// The gateway's canonical origin, which is the message's `aud`.
    pub origin: String,
    /// The path it registered.
    pub path: String,
    /// The first number of `sids`.
    pub from: u64,
    /// The `sid`s to name; none when the numbers aged out, which then need no call.
    pub sids: Vec<[u8; 16]>,
    /// The last number this push covers, for [`AccountStore::finish_gateway_push`].
    pub through: u64,
}

/// What became of a push.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GatewayPushResult {
    /// Taken.
    Delivered,
    /// Refused: asking again would change nothing, so catch-up carries it.
    Refused,
    /// Not answered, or asked to wait.
    Retry,
}

fn count(value: i64) -> Result<u64, Error> {
    u64::try_from(value).map_err(|_| Error::Unavailable)
}

fn sid_bytes(bytes: &[u8]) -> Result<[u8; 16], Error> {
    bytes.try_into().map_err(|_| Error::Unavailable)
}

/// The `sid` of this session, gateway and purpose: the one it has, or `fresh`.
async fn held_sid(
    tx: &DatabaseTransaction,
    digest: &[u8],
    stream: &str,
    purpose: GatewayPurpose,
    fresh: [u8; 16],
) -> Result<[u8; 16], Error> {
    let found = tx
        .query_one_raw(statement(
            "SELECT sid FROM central_identity.gateway_sids
             WHERE session_digest = $1 AND stream = $2 AND purpose = $3",
            vec![
                digest.to_vec().into(),
                stream.into(),
                purpose.as_str().into(),
            ],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
    if let Some(row) = found {
        let sid: Vec<u8> = row.try_get("", "sid").map_err(|_| Error::Unavailable)?;
        return sid_bytes(&sid);
    }
    tx.execute_raw(statement(
        "INSERT INTO central_identity.gateway_streams(origin) VALUES ($1)
         ON CONFLICT (origin) DO NOTHING",
        vec![stream.into()],
    ))
    .await
    .map_err(|_| Error::Unavailable)?;
    let inserted = tx
        .execute_raw(statement(
            "INSERT INTO central_identity.gateway_sids(sid, session_digest, stream, purpose)
             SELECT $1, $2, $3, $4 WHERE (SELECT count(*) FROM central_identity.gateway_sids
                 WHERE session_digest = $2) < $5",
            vec![
                fresh.to_vec().into(),
                digest.to_vec().into(),
                stream.into(),
                purpose.as_str().into(),
                MAX_SIDS_PER_SESSION.into(),
            ],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
    if inserted.rows_affected() != 1 {
        return Err(Error::Busy);
    }
    Ok(fresh)
}

impl AccountStore {
    /// Record a session at one gateway and read what its token says. `set` holds the origins
    /// the gateway answers for, canonical first; `registered` is false for the official
    /// gateway, whose authority is the bridge. A repeat keeps its `sid`, so that one session
    /// at one gateway stays one thing to revoke; `fresh` is used only for a first time.
    ///
    /// # Errors
    /// Rejects a session not live, an unfinished move, a disabled or unknown gateway and an
    /// `admin` purpose without membership; `Busy` past the session's `sid`s.
    pub async fn gateway_grant<S: AsRef<str> + Sync>(
        &self,
        session: &Token,
        set: &[S],
        registered: bool,
        purpose: GatewayPurpose,
        fresh: [u8; 16],
    ) -> Result<GatewayGrant, Error> {
        let stream = set.first().ok_or(Error::Rejected)?.as_ref();
        let digest = session.digest().to_vec();
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        // Lock the session before its children, as ending it does, so that a token is never
        // recorded for a session being ended, and the count below cannot be raced.
        let row = tx
            .query_one_raw(statement(
                "SELECT a.id, a.display_name, a.tag, a.legacy_id, a.legacy_closed,
                 floor(extract(epoch FROM COALESCE(s.proven_at, s.created_at)
                     + interval '12 hours'))::bigint AS cap
                 FROM central_identity.sessions s JOIN central_identity.accounts a
                 ON a.id = s.account_id AND a.credential_revision = s.credential_revision
                 WHERE s.digest = $1 AND s.kind = 'native'
                 AND s.expires_at > pg_catalog.clock_timestamp() AND a.restored
                 FOR UPDATE OF s",
                vec![digest.clone().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        let account: Uuid = row.try_get("", "id").map_err(|_| Error::Unavailable)?;
        let legacy_id: Option<String> = row
            .try_get("", "legacy_id")
            .map_err(|_| Error::Unavailable)?;
        let closed: bool = row
            .try_get("", "legacy_closed")
            .map_err(|_| Error::Unavailable)?;
        if registered {
            tx.query_one_raw(statement(
                &format!(
                    "SELECT g.id FROM central_identity.gateways g
                     WHERE g.origin = $1 AND NOT g.official AND {NOT_DISABLED}
                     AND ($2 = 'admit' OR EXISTS(SELECT 1 FROM central_identity.gateway_admins m
                         WHERE m.gateway_id = g.id AND m.account_id = $3))"
                ),
                vec![stream.into(), purpose.as_str().into(), account.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        } else if purpose != GatewayPurpose::Admit || (legacy_id.is_some() && !closed) {
            // Admission needs a closed move; the official gateway has no Central admins.
            return Err(Error::Rejected);
        }
        let sid = held_sid(&tx, &digest, stream, purpose, fresh).await?;
        let accepted = tx
            .query_one_raw(statement(
                &format!(
                    "SELECT version, digest FROM central_identity.terms_acceptances
                     WHERE account_id = $1 AND subject = {}
                     ORDER BY accepted_at DESC, version DESC LIMIT 1",
                    any_origin(2)
                ),
                vec![account.into(), origins(set)?],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        let terms = accepted
            .map(|row| -> Result<TermsVersion, Error> {
                let digest: Vec<u8> = row.try_get("", "digest").map_err(|_| Error::Unavailable)?;
                Ok(TermsVersion {
                    version: row.try_get("", "version").map_err(|_| Error::Unavailable)?,
                    digest: digest.try_into().map_err(|_| Error::Unavailable)?,
                })
            })
            .transpose()?;
        let (display_name, tag) = super::profiles::columns(&row)?;
        let cap = row.try_get("", "cap").map_err(|_| Error::Unavailable)?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(GatewayGrant {
            sid,
            account: *account.as_bytes(),
            cap,
            display_name,
            tag,
            // Only the official gateway ever learns an old id.
            legacy_id: legacy_id.filter(|_| !registered),
            terms,
        })
    }

    /// Name the path this gateway takes pushes on. `stream` comes from its credential.
    ///
    /// # Errors
    /// Rejects a path outside the rule and reports storage failure.
    pub async fn register_invalidate(&self, stream: &str, path: &str) -> Result<(), Error> {
        if !valid_invalidate_path(path) {
            return Err(Error::Rejected);
        }
        self.db
            .execute_raw(statement(
                "INSERT INTO central_identity.gateway_streams(origin, invalidate_path)
                 VALUES ($1, $2)
                 ON CONFLICT (origin) DO UPDATE SET invalidate_path = EXCLUDED.invalidate_path",
                vec![stream.into(), path.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        Ok(())
    }

    /// This gateway's last revocation number; 0 when it has none.
    ///
    /// # Errors
    /// Reports storage failure.
    pub async fn last_revocation(&self, stream: &str) -> Result<u64, Error> {
        let row = self
            .db
            .query_one_raw(statement(
                "SELECT last_seq FROM central_identity.gateway_streams WHERE origin = $1",
                vec![stream.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        match row {
            Some(row) => count(
                row.try_get("", "last_seq")
                    .map_err(|_| Error::Unavailable)?,
            ),
            None => Ok(0),
        }
    }

    /// The numbers after `after`, at most a page, with the gateway's position.
    ///
    /// # Errors
    /// Rejects a number too large for storage and reports storage failure.
    pub async fn revocation_page(&self, stream: &str, after: u64) -> Result<RevocationPage, Error> {
        let after = number(after)?;
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        // One snapshot: the page, `last` and `floor` describe the same moment.
        tx.execute_raw(statement(
            "SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY",
            vec![],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        let position = tx
            .query_one_raw(statement(
                "SELECT COALESCE((SELECT last_seq FROM central_identity.gateway_streams
                     WHERE origin = $1), 0) AS last,
                 (SELECT min(seq) FROM central_identity.gateway_revocations WHERE stream = $1) AS low",
                vec![stream.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)?;
        let last: i64 = position
            .try_get("", "last")
            .map_err(|_| Error::Unavailable)?;
        let low: Option<i64> = position
            .try_get("", "low")
            .map_err(|_| Error::Unavailable)?;
        let rows = tx
            .query_all_raw(statement(
                "SELECT seq, sid FROM central_identity.gateway_revocations
                 WHERE stream = $1 AND seq > $2 ORDER BY seq LIMIT $3",
                vec![stream.into(), after.into(), REVOCATION_PAGE.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        let mut from = 0;
        let mut sids = Vec::with_capacity(rows.len());
        for row in &rows {
            if sids.is_empty() {
                from = count(row.try_get("", "seq").map_err(|_| Error::Unavailable)?)?;
            }
            let sid: Vec<u8> = row.try_get("", "sid").map_err(|_| Error::Unavailable)?;
            sids.push(sid_bytes(&sid)?);
        }
        Ok(RevocationPage {
            from,
            sids,
            last: count(last)?,
            floor: count(low.unwrap_or(last + 1))?,
        })
    }

    /// Claim one gateway with numbers to push, if one is due. Only an approved, enabled
    /// gateway with a registered path is ever called: a pending gateway catches up instead.
    /// The claim counts an attempt and sets the next, so that a crash loses nothing and a
    /// sender that hangs holds nothing up.
    ///
    /// # Errors
    /// Reports storage failure.
    pub async fn claim_gateway_push(&self) -> Result<Option<GatewayPush>, Error> {
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let claimed = tx
            .query_one_raw(statement(
                "UPDATE central_identity.gateway_streams t
                 SET attempts = CASE WHEN t.attempts >= $1 THEN 1 ELSE t.attempts + 1 END,
                 next_attempt_at = pg_catalog.clock_timestamp() + pg_catalog.make_interval(
                     secs => pow(2, CASE WHEN t.attempts >= $1 THEN 0 ELSE t.attempts END)::double precision)
                 WHERE t.origin = (SELECT d.origin FROM central_identity.gateway_streams d
                     JOIN central_identity.gateways g ON g.origin = d.origin
                     JOIN central_identity.gateway_approvals p ON p.gateway_id = g.id AND p.enabled
                     WHERE d.pushed_seq < d.last_seq AND d.invalidate_path IS NOT NULL
                     AND (d.next_attempt_at IS NULL
                         OR d.next_attempt_at <= pg_catalog.clock_timestamp())
                     ORDER BY d.next_attempt_at NULLS FIRST LIMIT 1
                     FOR NO KEY UPDATE OF d SKIP LOCKED)
                 RETURNING t.origin, t.invalidate_path, t.pushed_seq",
                vec![PUSH_ATTEMPTS.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        let Some(claimed) = claimed else {
            return Ok(None);
        };
        let origin: String = claimed
            .try_get("", "origin")
            .map_err(|_| Error::Unavailable)?;
        let path: String = claimed
            .try_get("", "invalidate_path")
            .map_err(|_| Error::Unavailable)?;
        let pushed: i64 = claimed
            .try_get("", "pushed_seq")
            .map_err(|_| Error::Unavailable)?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        let page = self.revocation_page(&origin, count(pushed)?).await?;
        let through = if page.sids.is_empty() {
            page.last
        } else {
            page.from + count(i64::try_from(page.sids.len()).map_err(|_| Error::Unavailable)?)? - 1
        };
        Ok(Some(GatewayPush {
            origin,
            path,
            from: page.from,
            sids: page.sids,
            through,
        }))
    }

    /// Record what became of a push of numbers up to `through`. A page given up on is left
    /// to catch-up, and the gateway rests a minute, so that a gateway which never answers
    /// costs a few attempts per minute however many sessions end.
    ///
    /// # Errors
    /// Reports storage failure.
    pub async fn finish_gateway_push(
        &self,
        origin: &str,
        through: u64,
        result: GatewayPushResult,
    ) -> Result<(), Error> {
        let through = number(through)?;
        let sql = match result {
            GatewayPushResult::Delivered => {
                "UPDATE central_identity.gateway_streams
                 SET pushed_seq = GREATEST(pushed_seq, LEAST($2, last_seq)), attempts = 0,
                     next_attempt_at = NULL WHERE origin = $1"
            }
            GatewayPushResult::Refused => {
                "UPDATE central_identity.gateway_streams
                 SET pushed_seq = GREATEST(pushed_seq, LEAST($2, last_seq)), attempts = 0,
                     next_attempt_at = pg_catalog.clock_timestamp() + interval '1 minute'
                 WHERE origin = $1"
            }
            GatewayPushResult::Retry => {
                "UPDATE central_identity.gateway_streams
                 SET pushed_seq = CASE WHEN attempts >= $3
                         THEN GREATEST(pushed_seq, LEAST($2, last_seq)) ELSE pushed_seq END,
                     next_attempt_at = CASE WHEN attempts >= $3
                         THEN pg_catalog.clock_timestamp() + interval '1 minute'
                         ELSE next_attempt_at END,
                     attempts = CASE WHEN attempts >= $3 THEN 0 ELSE attempts END
                 WHERE origin = $1"
            }
        };
        let mut values: Vec<sea_orm::Value> = vec![origin.into(), through.into()];
        if result == GatewayPushResult::Retry {
            values.push(PUSH_ATTEMPTS.into());
        }
        self.db
            .execute_raw(statement(sql, values))
            .await
            .map_err(|_| Error::Unavailable)?;
        Ok(())
    }

    /// Forget revocations older than two hours, which outlives every token they could name.
    /// By age alone: a gateway reading from 0 must find every one still in force. It removes
    /// a prefix of each stream, so that a clock step can never leave a hole in a page. Returns
    /// how many went.
    ///
    /// # Errors
    /// Reports storage failure.
    pub async fn sweep_revocations(&self) -> Result<u64, Error> {
        let swept = self
            .db
            .execute_raw(statement(
                "DELETE FROM central_identity.gateway_revocations r USING
                 (SELECT stream, max(seq) AS upto FROM central_identity.gateway_revocations
                  WHERE revoked_at < pg_catalog.clock_timestamp() - interval '2 hours'
                  GROUP BY stream) old
                 WHERE r.stream = old.stream AND r.seq <= old.upto",
                vec![],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        Ok(swept.rows_affected())
    }
}
