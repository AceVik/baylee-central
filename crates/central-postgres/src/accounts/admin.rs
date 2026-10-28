//! Central administration (`docs/administration.md`, migration 000012): an
//! administrator's reads and changes, each checking the role in the statement
//! or transaction that acts, and the operator's appointment and sweep. The
//! queries name their columns and never read a credential, a wrapper or a
//! document.
use super::{AccountStore, Error, Session, Token, statement, username};
use central_api::admin::{
    ACCOUNT_LIST, AccountQuery, AccountState, AdminAccount, AuditEvent, OwnedGateway, PAGE,
};
use sea_orm::{
    ConnectionTrait, DatabaseConnection, QueryResult, TransactionTrait, Value, prelude::Uuid,
};

/// The closed list of audited actions. Each is `subject.verb`, as the
/// schema's CHECK requires; later steps add theirs here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// An account was given the administrator role.
    AdminGranted,
    /// An account's administrator role was removed.
    AdminRemoved,
    /// A reset code was made for an account.
    ResetIssued,
}

impl Action {
    /// The stored name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AdminGranted => "account.admin_granted",
            Self::AdminRemoved => "account.admin_removed",
            Self::ResetIssued => "account.reset_issued",
        }
    }
}

/// How long the audit keeps an event, as the privacy notice says; the
/// operator's [`sweep`] deletes older ones.
pub const AUDIT_YEARS: i32 = 3;

/// What the operator's appointment found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Appointed {
    /// The account holds the role now.
    Appointed,
    /// It held it already; nothing changed.
    Already,
}

/// The columns of an [`AdminAccount`], from `accounts a`.
const ACCOUNT_COLUMNS: &str = "a.id, a.username_key, a.display_name, a.tag, a.source,
    a.credential IS NOT NULL AS enrolled, a.restored,
    floor(extract(epoch FROM a.created_at))::bigint AS created_at,
    floor(extract(epoch FROM a.last_sign_in_at))::bigint AS last_sign_in_at,
    EXISTS (SELECT 1 FROM central_identity.account_roles r
        WHERE r.account_id = a.id AND r.role = 'admin') AS admin,
    CASE WHEN a.reset_expires > pg_catalog.clock_timestamp()
        THEN floor(extract(epoch FROM a.reset_expires))::bigint END AS reset_until";

/// The columns of an [`AuditEvent`], from `audit_events e`.
const EVENT_COLUMNS: &str = "e.id, floor(extract(epoch FROM e.at))::bigint AS at,
    e.actor_account, e.action, e.account_id, e.gateway_id, e.beta_key_id";

/// The live browser session `browser` of an administrator, as a condition on
/// `accounts a` joined to `sessions s`.
const ADMIN_SESSION: &str = "s.digest = $1 AND s.kind = 'browser'
    AND s.expires_at > pg_catalog.clock_timestamp() AND a.credential IS NOT NULL
    AND EXISTS (SELECT 1 FROM central_identity.account_roles r
        WHERE r.account_id = a.id AND r.role = 'admin')";

fn id(row: &QueryResult, column: &str) -> Result<[u8; 16], Error> {
    let id: Uuid = row.try_get("", column).map_err(|_| Error::Unavailable)?;
    Ok(*id.as_bytes())
}

fn optional_id(row: &QueryResult, column: &str) -> Result<Option<[u8; 16]>, Error> {
    let id: Option<Uuid> = row.try_get("", column).map_err(|_| Error::Unavailable)?;
    Ok(id.map(|id| *id.as_bytes()))
}

fn account(row: &QueryResult) -> Result<AdminAccount, Error> {
    let (display_name, tag) = super::profiles::columns(row)?;
    let get = |column| {
        row.try_get::<bool>("", column)
            .map_err(|_| Error::Unavailable)
    };
    Ok(AdminAccount {
        id: id(row, "id")?,
        username: row
            .try_get("", "username_key")
            .map_err(|_| Error::Unavailable)?,
        display_name,
        tag,
        source: super::recovery::source(
            &row.try_get::<String>("", "source")
                .map_err(|_| Error::Unavailable)?,
        )?,
        state: match (get("enrolled")?, get("restored")?) {
            (false, _) => AccountState::Pending,
            (true, false) => AccountState::Enrolled,
            (true, true) => AccountState::Restored,
        },
        created_at: row
            .try_get("", "created_at")
            .map_err(|_| Error::Unavailable)?,
        last_sign_in_at: row
            .try_get("", "last_sign_in_at")
            .map_err(|_| Error::Unavailable)?,
        admin: get("admin")?,
        reset_until: row
            .try_get("", "reset_until")
            .map_err(|_| Error::Unavailable)?,
    })
}

fn event(row: &QueryResult) -> Result<AuditEvent, Error> {
    Ok(AuditEvent {
        id: id(row, "id")?,
        at: row.try_get("", "at").map_err(|_| Error::Unavailable)?,
        actor: optional_id(row, "actor_account")?,
        action: row.try_get("", "action").map_err(|_| Error::Unavailable)?,
        account: optional_id(row, "account_id")?,
        gateway: optional_id(row, "gateway_id")?,
        beta_key: optional_id(row, "beta_key_id")?,
    })
}

/// A page of at most [`PAGE`] rows, and whether another follows.
fn page<T>(
    rows: &[QueryResult],
    read: impl Fn(&QueryResult) -> Result<T, Error>,
) -> Result<(Vec<T>, bool), Error> {
    let items = rows
        .iter()
        .take(PAGE)
        .map(read)
        .collect::<Result<Vec<_>, _>>()?;
    Ok((items, rows.len() > PAGE))
}

fn limit(n: usize) -> Value {
    i64::try_from(n).unwrap_or(i64::MAX).into()
}

/// Write one audit event in `db`'s transaction.
pub(super) async fn audit(
    db: &impl ConnectionTrait,
    actor: Option<Uuid>,
    action: Action,
    account: Uuid,
) -> Result<(), Error> {
    db.execute_raw(statement(
        "INSERT INTO central_identity.audit_events (actor_kind, actor_account, action, account_id)
         VALUES (CASE WHEN $1::uuid IS NULL THEN 'operator' ELSE 'admin' END, $1, $2, $3)",
        vec![actor.into(), action.as_str().into(), account.into()],
    ))
    .await
    .map_err(|_| Error::Unavailable)?;
    Ok(())
}

impl AccountStore {
    /// The administrator behind the live browser session `browser`, with
    /// whether its password sign-in is less than 30 minutes old.
    pub(super) async fn administrator(
        db: &impl ConnectionTrait,
        browser: &Token,
    ) -> Result<(Uuid, bool), Error> {
        let session = Session::Browser(browser);
        let row = db
            .query_one_raw(statement(
                &format!(
                    "SELECT a.id, s.created_at > pg_catalog.clock_timestamp() - interval '30 minutes' AS fresh
                     FROM central_identity.accounts a JOIN central_identity.sessions s
                     ON s.account_id = a.id AND s.credential_revision = a.credential_revision
                     WHERE {ADMIN_SESSION}"
                ),
                vec![session.digest().to_vec().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        Ok((
            row.try_get("", "id").map_err(|_| Error::Unavailable)?,
            row.try_get("", "fresh").map_err(|_| Error::Unavailable)?,
        ))
    }

    /// A page of accounts for the administrator behind `browser`, newest
    /// first: all, those whose username or display name contains the text,
    /// or the one with the tag.
    ///
    /// # Errors
    /// `Rejected` without an administrator's live browser session; storage
    /// failure.
    pub async fn admin_accounts(
        &self,
        browser: &Token,
        query: Option<&AccountQuery>,
        before: Option<[u8; 16]>,
    ) -> Result<(Vec<AdminAccount>, bool), Error> {
        let (text, tag) = match query {
            None => (None, None),
            Some(AccountQuery::Text(text)) => (Some(text.clone()), None),
            Some(AccountQuery::Tag(tag)) => (
                None,
                Some(i32::try_from(*tag).map_err(|_| Error::Rejected)?),
            ),
        };
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        Self::administrator(&tx, browser).await?;
        let rows = tx
            .query_all_raw(statement(
                &format!(
                    "SELECT {ACCOUNT_COLUMNS} FROM central_identity.accounts a
                     WHERE ($1::text IS NULL OR strpos(a.username_key, $1) > 0
                         OR strpos(lower(a.display_name), $1) > 0)
                     AND ($2::integer IS NULL OR a.tag = $2)
                     AND ($3::uuid IS NULL OR a.id < $3)
                     ORDER BY a.id DESC LIMIT $4"
                ),
                vec![
                    text.into(),
                    tag.into(),
                    before.map(Uuid::from_bytes).into(),
                    limit(PAGE + 1),
                ],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        let page = page(&rows, account)?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(page)
    }

    /// One account for the administrator behind `browser`: its row, whether
    /// it has a recovery wrapper, the gateways it owns and its latest audit
    /// events.
    ///
    /// # Errors
    /// `Rejected` without an administrator's live browser session or for an
    /// unknown account; storage failure.
    pub async fn admin_account(
        &self,
        browser: &Token,
        target: [u8; 16],
    ) -> Result<(AdminAccount, bool, Vec<OwnedGateway>, Vec<AuditEvent>), Error> {
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        Self::administrator(&tx, browser).await?;
        let target = Uuid::from_bytes(target);
        let row = tx
            .query_one_raw(statement(
                &format!(
                    "SELECT {ACCOUNT_COLUMNS}, a.recovery_wrapped_master IS NOT NULL AS recovery
                     FROM central_identity.accounts a WHERE a.id = $1"
                ),
                vec![target.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        let recovery: bool = row
            .try_get("", "recovery")
            .map_err(|_| Error::Unavailable)?;
        let gateways = tx
            .query_all_raw(statement(
                "SELECT g.id, g.origin, EXISTS (SELECT 1 FROM central_identity.gateway_approvals p
                     WHERE p.gateway_id = g.id) AS approved
                 FROM central_identity.gateways g WHERE g.owner_id = $1 ORDER BY g.id DESC LIMIT $2",
                vec![target.into(), limit(ACCOUNT_LIST)],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .iter()
            .map(|row| {
                Ok(OwnedGateway {
                    id: id(row, "id")?,
                    origin: row.try_get("", "origin").map_err(|_| Error::Unavailable)?,
                    approved: row
                        .try_get("", "approved")
                        .map_err(|_| Error::Unavailable)?,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let audit = tx
            .query_all_raw(statement(
                &format!(
                    "SELECT {EVENT_COLUMNS} FROM central_identity.audit_events e
                     WHERE e.account_id = $1 ORDER BY e.id DESC LIMIT $2"
                ),
                vec![target.into(), limit(ACCOUNT_LIST)],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .iter()
            .map(event)
            .collect::<Result<Vec<_>, Error>>()?;
        let account = account(&row)?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok((account, recovery, gateways, audit))
    }

    /// A page of audit events for the administrator behind `browser`, newest
    /// first, of one account or all.
    ///
    /// # Errors
    /// `Rejected` without an administrator's live browser session; storage
    /// failure.
    pub async fn admin_audit(
        &self,
        browser: &Token,
        target: Option<[u8; 16]>,
        before: Option<[u8; 16]>,
    ) -> Result<(Vec<AuditEvent>, bool), Error> {
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        Self::administrator(&tx, browser).await?;
        let rows = tx
            .query_all_raw(statement(
                &format!(
                    "SELECT {EVENT_COLUMNS} FROM central_identity.audit_events e
                     WHERE ($1::uuid IS NULL OR e.account_id = $1) AND ($2::uuid IS NULL OR e.id < $2)
                     ORDER BY e.id DESC LIMIT $3"
                ),
                vec![
                    target.map(Uuid::from_bytes).into(),
                    before.map(Uuid::from_bytes).into(),
                    limit(PAGE + 1),
                ],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        let page = page(&rows, event)?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(page)
    }

    /// Grant or remove the administrator role of `target` for the
    /// administrator behind `browser`, whose password sign-in must be less
    /// than 30 minutes old. Every administrator's account row is locked
    /// first, and the acting role checked after the lock, so two
    /// administrators removing each other at once leave one. Nobody removes their own role,
    /// which keeps the last administrator; only an enrolled account is
    /// appointed. The same state again changes nothing; a change writes its
    /// audit event in the same transaction.
    ///
    /// # Errors
    /// `Rejected` without an administrator's live browser session, for a
    /// sign-in 30 minutes old or older, for one's own role, and for an
    /// unknown or unenrolled account to appoint; storage failure.
    pub async fn set_admin(
        &self,
        browser: &Token,
        target: [u8; 16],
        admin: bool,
    ) -> Result<(), Error> {
        // Anyone else is refused before any lock is taken.
        Self::administrator(&self.db, browser).await?;
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        // The administrators' account rows, in one order so two changes cannot
        // deadlock; the role rows themselves need an UPDATE grant to lock.
        tx.query_all_raw(statement(
            "SELECT a.id FROM central_identity.accounts a JOIN central_identity.account_roles r
             ON r.account_id = a.id WHERE r.role = 'admin' ORDER BY a.id FOR UPDATE OF a",
            vec![],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        // A fresh snapshot after the lock: a removal that committed meanwhile is seen.
        let (actor, fresh) = Self::administrator(&tx, browser).await?;
        let target = Uuid::from_bytes(target);
        if !fresh || (!admin && actor == target) {
            return Err(Error::Rejected);
        }
        let changed = if admin {
            tx.execute_raw(statement(
                "INSERT INTO central_identity.account_roles (account_id, role, granted_by)
                 SELECT id, 'admin', $2 FROM central_identity.accounts
                 WHERE id = $1 AND credential IS NOT NULL
                 ON CONFLICT (account_id, role) DO NOTHING",
                vec![target.into(), actor.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .rows_affected()
                == 1
        } else {
            tx.execute_raw(statement(
                "DELETE FROM central_identity.account_roles WHERE account_id = $1 AND role = 'admin'",
                vec![target.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .rows_affected()
                == 1
        };
        if changed {
            let action = if admin {
                Action::AdminGranted
            } else {
                Action::AdminRemoved
            };
            audit(&tx, Some(actor), action, target).await?;
        } else if admin {
            // Already an administrator, or no enrolled account to appoint.
            let held = tx
                .query_one_raw(statement(
                    "SELECT 1 AS held FROM central_identity.account_roles WHERE account_id = $1 AND role = 'admin'",
                    vec![target.into()],
                ))
                .await
                .map_err(|_| Error::Unavailable)?
                .is_some();
            if !held {
                return Err(Error::Rejected);
            }
        }
        tx.commit().await.map_err(|_| Error::Unavailable)
    }
}

/// Appoint the enrolled account `name` as an administrator, under the
/// operator's (migrator's) login: the first administrator, and the way back
/// in when none can sign in. Appointing again changes nothing; an
/// appointment writes an audit event with the operator as its actor.
///
/// # Errors
/// `Rejected` for a malformed name or no enrolled account of that name;
/// storage failure.
pub async fn appoint_admin(db: &DatabaseConnection, name: &str) -> Result<Appointed, Error> {
    let name = username(name)?;
    let tx = db.begin().await.map_err(|_| Error::Unavailable)?;
    let account: Uuid = tx
        .query_one_raw(statement(
            "SELECT id FROM central_identity.accounts WHERE username_key = $1 AND credential IS NOT NULL
             FOR UPDATE",
            vec![name.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::Rejected)?
        .try_get("", "id")
        .map_err(|_| Error::Unavailable)?;
    let inserted = tx
        .execute_raw(statement(
            "INSERT INTO central_identity.account_roles (account_id, role) VALUES ($1, 'admin')
             ON CONFLICT (account_id, role) DO NOTHING",
            vec![account.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .rows_affected()
        == 1;
    if inserted {
        audit(&tx, None, Action::AdminGranted, account).await?;
    }
    tx.commit().await.map_err(|_| Error::Unavailable)?;
    Ok(if inserted {
        Appointed::Appointed
    } else {
        Appointed::Already
    })
}

/// Delete audit events older than [`AUDIT_YEARS`], under the operator's
/// (migrator's) login, as the privacy notice allows. Returns how many.
///
/// # Errors
/// Storage failure.
pub async fn sweep(db: &DatabaseConnection) -> Result<u64, Error> {
    Ok(db
        .execute_raw(statement(
            "DELETE FROM central_identity.audit_events
             WHERE at < pg_catalog.clock_timestamp() - pg_catalog.make_interval(years => $1)",
            vec![AUDIT_YEARS.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .rows_affected())
}
