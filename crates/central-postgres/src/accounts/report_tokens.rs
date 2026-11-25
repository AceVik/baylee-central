//! Report tokens (migration 000052, `docs/reports.md` §"Report tokens"): an
//! administrator's credential for the report routes. Only its digest is
//! stored or read, and no token is ever deleted. Whether one is live, whose
//! it is and which rights it holds is asked by the statement that
//! authenticates it, each time, and again inside the transaction of every
//! change: nothing of it is remembered between two requests.
use super::admin::{Action, id, limit, optional_id};
use super::tokens::LIVE_TOKEN;
use super::{AccountStore, Error, Token, statement, uuid};
use central_api::report_tokens::{
    LISTED_TOKENS, LIVE_TOKENS, ReportScope, ReportScopes, ReportTokenInfo, TOKENS_PER_DAY,
    validate_new,
};
use sea_orm::{ConnectionTrait, QueryResult, prelude::Uuid};

/// The maker of `report_tokens t` is an administrator now: a token never
/// outlives its maker's role.
const MAKER: &str = "EXISTS (SELECT 1 FROM central_identity.account_roles m
    WHERE m.account_id = t.created_by AND m.role = 'admin')";

/// The columns of a [`ReportTokenInfo`], from `report_tokens t` with its maker
/// `c` and its revoker `v` left-joined from the accounts.
const INFO_COLUMNS: &str = "t.id, t.name, t.file, t.read, t.triage, t.purge,
    t.created_by, c.display_name AS created_by_name, c.tag AS created_by_tag,
    t.revoked_by, v.display_name AS revoked_by_name, v.tag AS revoked_by_tag,
    floor(extract(epoch FROM t.created_at))::bigint AS created_at,
    floor(extract(epoch FROM t.expires_at))::bigint AS expires_at,
    floor(extract(epoch FROM t.revoked_at))::bigint AS revoked_at,
    floor(extract(epoch FROM t.last_used_at))::bigint AS last_used_at";

/// A report token that authenticated with the right that was asked of it:
/// its id and its name now, and nothing it could authenticate with again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReportToken {
    /// Its id.
    pub id: [u8; 16],
    /// Its name.
    pub name: String,
}

/// How long what the statement found has to hold.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Hold {
    /// For the statement: a read's check, or the one before a body is read.
    Now,
    /// Until the transaction ends: a change's. A revocation and the end of
    /// its maker's role wait for it (`set_admin` locks the maker's tokens),
    /// and one that committed before is seen here, so nothing a token began
    /// commits once either was answered.
    ToTheEnd,
}

fn found(row: &QueryResult) -> Result<ReportToken, Error> {
    Ok(ReportToken {
        id: id(row, "id")?,
        name: row.try_get("", "name").map_err(|_| Error::Unavailable)?,
    })
}

/// The live report token whose `column` is `$1` and which holds `right`.
/// Both are fixed words (`digest` or `id`, and a right's column).
fn authenticating(column: &'static str, right: ReportScope, hold: Hold) -> String {
    format!(
        "SELECT t.id, t.name FROM central_identity.report_tokens t
         WHERE t.{column} = $1 AND t.{} AND {LIVE_TOKEN} AND {MAKER}{}",
        right.as_str(),
        if hold == Hold::ToTheEnd {
            " FOR SHARE OF t"
        } else {
            ""
        }
    )
}

/// The maker of `token` is an administrator still, asked once its row is
/// locked and in a statement of its own. A statement that waited for the row
/// looks at that row again and at nothing else: a role that went while it
/// waited would be there for it.
async fn made_by_an_administrator(
    db: &impl ConnectionTrait,
    token: ReportToken,
) -> Result<ReportToken, Error> {
    db.query_one_raw(statement(
        &format!(
            "SELECT 1 AS yes FROM central_identity.report_tokens t WHERE t.id = $1 AND {MAKER}"
        ),
        vec![uuid(token.id).into()],
    ))
    .await
    .map_err(|_| Error::Unavailable)?
    .ok_or(Error::Rejected)?;
    Ok(token)
}

/// The report token `token` authenticates as, in `db`, if it is live, its
/// maker is an administrator and it holds `right`.
///
/// # Errors
/// `Rejected` for anything else, whatever the reason; storage failure.
pub(super) async fn authenticated(
    db: &impl ConnectionTrait,
    token: &Token,
    right: ReportScope,
    hold: Hold,
) -> Result<ReportToken, Error> {
    let row = db
        .query_one_raw(statement(
            &authenticating("digest", right, hold),
            vec![token.digest().to_vec().into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::Rejected)?;
    let token = found(&row)?;
    match hold {
        Hold::Now => Ok(token),
        Hold::ToTheEnd => made_by_an_administrator(db, token).await,
    }
}

/// As [`authenticated`], for a token that authenticated before in the same
/// request, by its id and until the transaction ends: a filing's, whose body
/// came in between.
///
/// # Errors
/// `Rejected` for a token that no longer authenticates or holds `right`;
/// storage failure.
pub(super) async fn still(
    db: &impl ConnectionTrait,
    token: [u8; 16],
    right: ReportScope,
) -> Result<ReportToken, Error> {
    let row = db
        .query_one_raw(statement(
            &authenticating("id", right, Hold::ToTheEnd),
            vec![uuid(token).into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::Rejected)?;
    made_by_an_administrator(db, found(&row)?).await
}

/// Lock the live report tokens `maker` made, in `db`'s transaction, before
/// their role ends: what they began is over first, and what begins after
/// asks for the role again ([`made_by_an_administrator`]).
///
/// # Errors
/// Storage failure.
pub(super) async fn hold_back(db: &impl ConnectionTrait, maker: Uuid) -> Result<(), Error> {
    db.query_all_raw(statement(
        "SELECT id FROM central_identity.report_tokens WHERE created_by = $1 AND revoked_at IS NULL
         ORDER BY id FOR UPDATE",
        vec![maker.into()],
    ))
    .await
    .map_err(|_| Error::Unavailable)?;
    Ok(())
}

fn scopes(row: &QueryResult) -> Result<ReportScopes, Error> {
    let mut held = Vec::with_capacity(ReportScope::ALL.len());
    for right in ReportScope::ALL {
        if row
            .try_get::<bool>("", right.as_str())
            .map_err(|_| Error::Unavailable)?
        {
            held.push(right);
        }
    }
    ReportScopes::of(&held)
        .validate()
        .map_err(|_| Error::Unavailable)
}

fn info(row: &QueryResult) -> Result<ReportTokenInfo, Error> {
    let tag = |column| {
        row.try_get::<Option<i32>>("", column)
            .map_err(|_| Error::Unavailable)?
            .map(|tag| u32::try_from(tag).map_err(|_| Error::Unavailable))
            .transpose()
    };
    let time = |column| {
        row.try_get::<Option<i64>>("", column)
            .map_err(|_| Error::Unavailable)
    };
    let maker_admin: bool = row
        .try_get("", "maker_admin")
        .map_err(|_| Error::Unavailable)?;
    Ok(ReportTokenInfo {
        id: id(row, "id")?,
        name: row.try_get("", "name").map_err(|_| Error::Unavailable)?,
        scopes: scopes(row)?,
        created_by: id(row, "created_by")?,
        created_by_name: row
            .try_get("", "created_by_name")
            .map_err(|_| Error::Unavailable)?,
        created_by_tag: tag("created_by_tag")?,
        suspended: !maker_admin,
        created_at: time("created_at")?.ok_or(Error::Unavailable)?,
        expires_at: time("expires_at")?.ok_or(Error::Unavailable)?,
        revoked_at: time("revoked_at")?,
        revoked_by: optional_id(row, "revoked_by")?,
        revoked_by_name: row
            .try_get("", "revoked_by_name")
            .map_err(|_| Error::Unavailable)?,
        revoked_by_tag: tag("revoked_by_tag")?,
        last_used_at: time("last_used_at")?,
    })
}

/// Write one audit event about a report token in `db`'s transaction: its
/// id, never its name or its digest.
async fn audit(
    db: &impl ConnectionTrait,
    actor: Uuid,
    action: Action,
    token: Uuid,
) -> Result<(), Error> {
    db.execute_raw(statement(
        "INSERT INTO central_identity.audit_events (actor_kind, actor_account, action, report_token_id)
         VALUES ('admin', $1, $2, $3)",
        vec![actor.into(), action.as_str().into(), token.into()],
    ))
    .await
    .map_err(|_| Error::Unavailable)?;
    Ok(())
}

impl AccountStore {
    /// The report token `token` authenticates as, if it is live, its maker
    /// is an administrator and it holds `right`. Marks it used, at most once
    /// a minute.
    ///
    /// # Errors
    /// `Rejected` for an unknown token, a revoked or expired one, one whose
    /// maker is no administrator and one without the right, all alike;
    /// storage failure.
    pub async fn report_token(
        &self,
        token: &Token,
        right: ReportScope,
    ) -> Result<ReportToken, Error> {
        let found = authenticated(&self.db, token, right, Hold::Now).await?;
        self.db
            .execute_raw(statement(
                "UPDATE central_identity.report_tokens SET last_used_at = pg_catalog.clock_timestamp()
                 WHERE id = $1 AND revoked_at IS NULL AND (last_used_at IS NULL
                     OR last_used_at < pg_catalog.clock_timestamp() - interval '1 minute')",
                vec![uuid(found.id).into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        Ok(found)
    }

    /// The report tokens for the administrator behind `browser`: the live
    /// ones first, then those revoked or expired in the last 30 days, each
    /// group newest first.
    ///
    /// # Errors
    /// `Rejected` without an administrator's live browser session; storage
    /// failure.
    pub async fn admin_report_tokens(
        &self,
        browser: &Token,
    ) -> Result<Vec<ReportTokenInfo>, Error> {
        use sea_orm::TransactionTrait as _;
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        Self::administrator(&tx, browser).await?;
        let tokens = tx
            .query_all_raw(statement(
                &format!(
                    "SELECT {INFO_COLUMNS}, {MAKER} AS maker_admin
                     FROM central_identity.report_tokens t
                     LEFT JOIN central_identity.accounts c ON c.id = t.created_by
                     LEFT JOIN central_identity.accounts v ON v.id = t.revoked_by
                     WHERE COALESCE(t.revoked_at, t.expires_at) > pg_catalog.clock_timestamp() - interval '30 days'
                     ORDER BY ({LIVE_TOKEN}) DESC, t.id DESC LIMIT $1"
                ),
                vec![limit(LISTED_TOKENS)],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .iter()
            .map(info)
            .collect::<Result<Vec<_>, Error>>()?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(tokens)
    }

    /// Store the digest of `token` as a report token named `name`, with
    /// `scopes`, for `days`, made by the administrator behind `browser`,
    /// whose password sign-in must be less than 30 minutes old. Answers its
    /// id and its expiry in Unix seconds; the audit event is written in the
    /// same transaction.
    ///
    /// # Errors
    /// `Rejected` without a fresh administrator's browser session, for a
    /// malformed name, rights no token holds or a lifetime out of 1 to 366
    /// days; `Busy` past [`LIVE_TOKENS`] live tokens or [`TOKENS_PER_DAY`]
    /// new ones in a day; storage failure.
    pub async fn create_report_token(
        &self,
        browser: &Token,
        name: &str,
        scopes: ReportScopes,
        days: u16,
        token: &Token,
    ) -> Result<([u8; 16], i64), Error> {
        validate_new(name, scopes, days)?;
        // Every administrator's row is locked, so two makings are counted
        // one after the other.
        let (tx, actor) = self.acting_administrator(browser).await?;
        let counts = tx
            .query_one_raw(statement(
                &format!(
                    "SELECT count(*) FILTER (WHERE {LIVE_TOKEN})::bigint AS live,
                         count(*) FILTER (WHERE t.created_at > pg_catalog.clock_timestamp() - interval '1 day')::bigint AS today
                     FROM central_identity.report_tokens t"
                ),
                vec![],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)?;
        let full = |column, most: usize| {
            counts
                .try_get::<i64>("", column)
                .map(|held| usize::try_from(held).unwrap_or(usize::MAX) >= most)
                .map_err(|_| Error::Unavailable)
        };
        if full("live", LIVE_TOKENS)? || full("today", TOKENS_PER_DAY)? {
            return Err(Error::Busy);
        }
        // One clock reading for both ends, so a 366-day token meets the
        // table's bound exactly rather than by a few microseconds over.
        let row = tx
            .query_one_raw(statement(
                "WITH now AS (SELECT pg_catalog.clock_timestamp() AS at)
                 INSERT INTO central_identity.report_tokens
                     (name, digest, file, read, triage, purge, created_by, created_at, expires_at)
                 SELECT $1, $2, $3, $4, $5, $6, $7, now.at, now.at + pg_catalog.make_interval(days => $8) FROM now
                 RETURNING id, floor(extract(epoch FROM expires_at))::bigint AS expires_at",
                vec![
                    name.into(),
                    token.digest().to_vec().into(),
                    scopes.allows(ReportScope::File).into(),
                    scopes.allows(ReportScope::Read).into(),
                    scopes.allows(ReportScope::Triage).into(),
                    scopes.allows(ReportScope::Purge).into(),
                    actor.into(),
                    i32::from(days).into(),
                ],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)?;
        let made = id(&row, "id")?;
        let expires_at: i64 = row
            .try_get("", "expires_at")
            .map_err(|_| Error::Unavailable)?;
        audit(&tx, actor, Action::ReportTokenCreated, uuid(made)).await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok((made, expires_at))
    }

    /// Revoke the report token `token` for the administrator behind
    /// `browser`, whose password sign-in must be less than 30 minutes old.
    /// One revoked already changes nothing; a revocation writes its audit
    /// event in the same transaction.
    ///
    /// # Errors
    /// `Rejected` without a fresh administrator's browser session or for an
    /// unknown token; storage failure.
    pub async fn revoke_report_token(&self, browser: &Token, token: [u8; 16]) -> Result<(), Error> {
        let (tx, actor) = self.acting_administrator(browser).await?;
        let token = uuid(token);
        let revoked = tx
            .execute_raw(statement(
                "UPDATE central_identity.report_tokens
                 SET revoked_at = pg_catalog.clock_timestamp(), revoked_by = $2
                 WHERE id = $1 AND revoked_at IS NULL",
                vec![token.into(), actor.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .rows_affected()
            == 1;
        if revoked {
            audit(&tx, actor, Action::ReportTokenRevoked, token).await?;
        } else {
            tx.query_one_raw(statement(
                "SELECT 1 AS known FROM central_identity.report_tokens WHERE id = $1",
                vec![token.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        }
        tx.commit().await.map_err(|_| Error::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The authenticating statement asks for everything that makes a token
    /// good, whichever right and column it is asked with.
    #[test]
    fn every_authentication_asks_for_the_right_the_life_and_the_maker() {
        for right in ReportScope::ALL {
            for column in ["digest", "id"] {
                for hold in [Hold::Now, Hold::ToTheEnd] {
                    let sql = authenticating(column, right, hold);
                    for asked in [
                        format!("t.{column} = $1"),
                        format!("AND t.{} AND", right.as_str()),
                        "t.revoked_at IS NULL".into(),
                        "t.expires_at > pg_catalog.clock_timestamp()".into(),
                        "m.account_id = t.created_by AND m.role = 'admin'".into(),
                    ] {
                        assert!(sql.contains(&asked), "{asked}: {sql}");
                    }
                    assert_eq!(sql.ends_with("FOR SHARE OF t"), hold == Hold::ToTheEnd);
                }
            }
        }
    }
}
