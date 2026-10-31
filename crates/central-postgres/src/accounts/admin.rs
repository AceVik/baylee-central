//! Central administration (`docs/administration.md`, migration 000012): an
//! administrator's reads and changes, each checking the role in the statement
//! or transaction that acts, and the operator's appointment and sweep. The
//! queries name their columns and never read a credential, a wrapper or a
//! document.
use super::tokens::{LIVE_TOKEN, STATE, TOKEN_COLUMNS, metadata, state, token_info};
use super::{AccountStore, Error, Session, Token, statement, username};
use central_api::admin::{
    ACCOUNT_LIST, AccountQuery, AccountState, AdminAccount, AdminGateway, AuditEvent, NamedAccount,
    NamedGateway, OwnedGateway, PAGE,
};
use central_api::gateway::GatewayState;
use central_api::owner::TokenInfo;
use central_api::profile::valid_tag;
use sea_orm::{
    ConnectionTrait, DatabaseConnection, QueryResult, TransactionTrait, Value, prelude::Uuid,
};
use std::collections::BTreeMap;

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
    /// An account was given another tag; the detail holds both.
    TagChanged,
    /// A gateway was approved, or enabled again.
    GatewayApproved,
    /// A gateway was disabled.
    GatewayDisabled,
    /// A gateway token was revoked by an administrator.
    GatewayTokenRevoked,
}

impl Action {
    /// The stored name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AdminGranted => "account.admin_granted",
            Self::AdminRemoved => "account.admin_removed",
            Self::ResetIssued => "account.reset_issued",
            Self::TagChanged => "account.tag_changed",
            Self::GatewayApproved => "gateway.approved",
            Self::GatewayDisabled => "gateway.disabled",
            Self::GatewayTokenRevoked => "gateway.token_revoked",
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
pub(super) const EVENT_COLUMNS: &str = "e.id, floor(extract(epoch FROM e.at))::bigint AS at,
    e.actor_account, e.action, e.account_id, e.gateway_id, e.beta_key_id";

/// The live browser session `browser` of an administrator, as a condition on
/// `accounts a` joined to `sessions s`.
const ADMIN_SESSION: &str = "s.digest = $1 AND s.kind = 'browser'
    AND s.expires_at > pg_catalog.clock_timestamp() AND a.credential IS NOT NULL
    AND EXISTS (SELECT 1 FROM central_identity.account_roles r
        WHERE r.account_id = a.id AND r.role = 'admin')";

pub(super) fn id(row: &QueryResult, column: &str) -> Result<[u8; 16], Error> {
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

pub(super) fn event(row: &QueryResult) -> Result<AuditEvent, Error> {
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

/// An [`OwnedGateway`] from `id`, `origin` and `approved`.
pub(super) fn owned_gateway(row: &QueryResult) -> Result<OwnedGateway, Error> {
    Ok(OwnedGateway {
        id: id(row, "id")?,
        origin: row.try_get("", "origin").map_err(|_| Error::Unavailable)?,
        approved: row
            .try_get("", "approved")
            .map_err(|_| Error::Unavailable)?,
    })
}

/// A page of the audit log, with today's names of what it names.
pub struct AuditPage {
    /// At most [`PAGE`] events, newest first.
    pub events: Vec<AuditEvent>,
    /// Another page follows.
    pub more: bool,
    /// The accounts the events name that still exist, each once.
    pub accounts: Vec<NamedAccount>,
    /// The gateways the events name that still exist, each once.
    pub gateways: Vec<NamedGateway>,
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

/// Write one audit event about a gateway in `db`'s transaction.
async fn audit_gateway(
    db: &impl ConnectionTrait,
    actor: Uuid,
    action: Action,
    gateway: Uuid,
) -> Result<(), Error> {
    db.execute_raw(statement(
        "INSERT INTO central_identity.audit_events (actor_kind, actor_account, action, gateway_id)
         VALUES ('admin', $1, $2, $3)",
        vec![actor.into(), action.as_str().into(), gateway.into()],
    ))
    .await
    .map_err(|_| Error::Unavailable)?;
    Ok(())
}

/// The columns of an [`AdminGateway`], from `gateways g` with `gateway_approvals p`
/// left-joined and the owner `accounts o` left-joined.
const GATEWAY_COLUMNS: &str = "g.id, g.origin, g.name, g.version, g.website, g.official, g.owner_id,
    o.display_name AS owner_name, o.tag AS owner_tag,
    CASE WHEN p.gateway_id IS NULL THEN 'pending' WHEN p.enabled THEN 'approved' ELSE 'disabled' END AS state,
    COALESCE(g.last_seen > pg_catalog.clock_timestamp() - interval '90 seconds', false) AS online,
    floor(extract(epoch FROM g.last_seen))::bigint AS last_seen,
    floor(extract(epoch FROM g.created_at))::bigint AS created_at,
    g.credential_digest IS NOT NULL AS credential_pair";

/// The joins under [`GATEWAY_COLUMNS`].
const GATEWAY_FROM: &str = "central_identity.gateways g
    LEFT JOIN central_identity.gateway_approvals p ON p.gateway_id = g.id
    LEFT JOIN central_identity.accounts o ON o.id = g.owner_id";

fn admin_gateway(row: &QueryResult) -> Result<AdminGateway, Error> {
    let owner_tag: Option<i32> = row
        .try_get("", "owner_tag")
        .map_err(|_| Error::Unavailable)?;
    let live: i64 = row
        .try_get("", "live_tokens")
        .map_err(|_| Error::Unavailable)?;
    Ok(AdminGateway {
        id: id(row, "id")?,
        origin: row.try_get("", "origin").map_err(|_| Error::Unavailable)?,
        metadata: metadata(row)?,
        official: row
            .try_get("", "official")
            .map_err(|_| Error::Unavailable)?,
        owner: optional_id(row, "owner_id")?,
        owner_name: row
            .try_get("", "owner_name")
            .map_err(|_| Error::Unavailable)?,
        owner_tag: owner_tag
            .map(u32::try_from)
            .transpose()
            .map_err(|_| Error::Unavailable)?,
        state: state(row)?,
        online: row.try_get("", "online").map_err(|_| Error::Unavailable)?,
        last_seen: row
            .try_get("", "last_seen")
            .map_err(|_| Error::Unavailable)?,
        created_at: row
            .try_get("", "created_at")
            .map_err(|_| Error::Unavailable)?,
        live_tokens: u32::try_from(live).map_err(|_| Error::Unavailable)?,
        credential_pair: row
            .try_get("", "credential_pair")
            .map_err(|_| Error::Unavailable)?,
    })
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
                    "SELECT a.id, COALESCE(s.proven_at, s.created_at) > pg_catalog.clock_timestamp() - interval '30 minutes' AS fresh
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
            .map(owned_gateway)
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
    ) -> Result<AuditPage, Error> {
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        Self::administrator(&tx, browser).await?;
        // The record names ids; the same statement reads what those accounts
        // and gateways are called today, so names and events agree.
        let rows = tx
            .query_all_raw(statement(
                &format!(
                    "SELECT {EVENT_COLUMNS},
                         actor.id IS NOT NULL AS actor_known, actor.display_name AS actor_name, actor.tag AS actor_tag,
                         subject.id IS NOT NULL AS subject_known, subject.display_name AS subject_name, subject.tag AS subject_tag,
                         gw.name AS gateway_name, gw.origin AS gateway_origin
                     FROM central_identity.audit_events e
                     LEFT JOIN central_identity.accounts actor ON actor.id = e.actor_account
                     LEFT JOIN central_identity.accounts subject ON subject.id = e.account_id
                     LEFT JOIN central_identity.gateways gw ON gw.id = e.gateway_id
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
        let (events, more) = page(&rows, event)?;
        let mut accounts = BTreeMap::new();
        let mut gateways = BTreeMap::new();
        for (row, event) in rows.iter().zip(&events) {
            for (id, prefix) in [(event.actor, "actor"), (event.account, "subject")] {
                let Some(id) = id else { continue };
                let known: bool = row
                    .try_get("", &format!("{prefix}_known"))
                    .map_err(|_| Error::Unavailable)?;
                if known && !accounts.contains_key(&id) {
                    let name: Option<String> = row
                        .try_get("", &format!("{prefix}_name"))
                        .map_err(|_| Error::Unavailable)?;
                    let tag: Option<i32> = row
                        .try_get("", &format!("{prefix}_tag"))
                        .map_err(|_| Error::Unavailable)?;
                    let tag = tag
                        .map(|t| u32::try_from(t).map_err(|_| Error::Unavailable))
                        .transpose()?;
                    accounts.insert(
                        id,
                        NamedAccount {
                            id,
                            display_name: name,
                            tag,
                        },
                    );
                }
            }
            if let Some(id) = event.gateway {
                let name: Option<String> = row
                    .try_get("", "gateway_name")
                    .map_err(|_| Error::Unavailable)?;
                let origin: Option<String> = row
                    .try_get("", "gateway_origin")
                    .map_err(|_| Error::Unavailable)?;
                if let (Some(name), Some(origin)) = (name, origin) {
                    gateways
                        .entry(id)
                        .or_insert(NamedGateway { id, name, origin });
                }
            }
        }
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(AuditPage {
            events,
            more,
            accounts: accounts.into_values().collect(),
            gateways: gateways.into_values().collect(),
        })
    }

    /// The administrator behind `browser` for a change in `tx`: anyone else
    /// is refused before any lock is taken; then every administrator's
    /// account row is locked, in one order so two changes cannot deadlock
    /// (the role rows themselves need an UPDATE grant to lock), and the role
    /// and a password sign-in less than 30 minutes old are checked again in a
    /// fresh snapshot, so a removal that committed meanwhile is seen.
    async fn acting_administrator(
        &self,
        tx: &sea_orm::DatabaseTransaction,
        browser: &Token,
    ) -> Result<Uuid, Error> {
        Self::administrator(&self.db, browser).await?;
        tx.query_all_raw(statement(
            "SELECT a.id FROM central_identity.accounts a JOIN central_identity.account_roles r
             ON r.account_id = a.id WHERE r.role = 'admin' ORDER BY a.id FOR UPDATE OF a",
            vec![],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        let (actor, fresh) = Self::administrator(tx, browser).await?;
        if !fresh {
            return Err(Error::Rejected);
        }
        Ok(actor)
    }

    /// A page of registered gateways for the administrator behind
    /// `browser`, newest first, all or those in `filter`'s state.
    ///
    /// # Errors
    /// `Rejected` without an administrator's live browser session; storage
    /// failure.
    pub async fn admin_gateways(
        &self,
        browser: &Token,
        filter: Option<GatewayState>,
        before: Option<[u8; 16]>,
    ) -> Result<(Vec<AdminGateway>, bool), Error> {
        let filter = filter.map(|state| match state {
            GatewayState::Pending => "pending",
            GatewayState::Approved => "approved",
            GatewayState::Disabled => "disabled",
        });
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        Self::administrator(&tx, browser).await?;
        let rows = tx
            .query_all_raw(statement(
                &format!(
                    "SELECT {GATEWAY_COLUMNS},
                         (SELECT count(*) FROM central_identity.gateway_tokens t
                          WHERE t.gateway_id = g.id AND {LIVE_TOKEN})::bigint AS live_tokens
                     FROM {GATEWAY_FROM}
                     WHERE ($1::text IS NULL OR {STATE} = $1) AND ($2::uuid IS NULL OR g.id < $2)
                     ORDER BY g.id DESC LIMIT $3"
                ),
                vec![
                    filter.into(),
                    before.map(Uuid::from_bytes).into(),
                    limit(PAGE + 1),
                ],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        let page = page(&rows, admin_gateway)?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(page)
    }

    /// One gateway for the administrator behind `browser`, with its latest
    /// tokens.
    ///
    /// # Errors
    /// `Rejected` without an administrator's live browser session or for an
    /// unknown gateway; storage failure.
    pub async fn admin_gateway(
        &self,
        browser: &Token,
        gateway: [u8; 16],
    ) -> Result<(AdminGateway, Vec<TokenInfo>), Error> {
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        Self::administrator(&tx, browser).await?;
        let gateway = Uuid::from_bytes(gateway);
        let row = tx
            .query_one_raw(statement(
                &format!(
                    "SELECT {GATEWAY_COLUMNS},
                         (SELECT count(*) FROM central_identity.gateway_tokens t
                          WHERE t.gateway_id = g.id AND {LIVE_TOKEN})::bigint AS live_tokens
                     FROM {GATEWAY_FROM} WHERE g.id = $1"
                ),
                vec![gateway.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        let tokens = tx
            .query_all_raw(statement(
                &format!(
                    "SELECT {TOKEN_COLUMNS} FROM central_identity.gateway_tokens t
                     WHERE t.gateway_id = $1 ORDER BY t.id DESC LIMIT $2"
                ),
                vec![gateway.into(), limit(ACCOUNT_LIST)],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .iter()
            .map(token_info)
            .collect::<Result<Vec<_>, Error>>()?;
        let gateway = admin_gateway(&row)?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok((gateway, tokens))
    }

    /// Approve or enable `gateway` (`enabled`), or disable it, for the
    /// administrator behind `browser`, whose password sign-in must be less
    /// than 30 minutes old. Disabling ends every lease at that gateway, so
    /// each of its sessions fails at its next check; its credentials and
    /// tokens stop authenticating with the approval. The same state again
    /// changes nothing; a change writes its audit event in the same
    /// transaction.
    ///
    /// # Errors
    /// `Rejected` without a fresh administrator's browser session or for an
    /// unknown gateway; storage failure.
    pub async fn set_gateway_approval(
        &self,
        browser: &Token,
        gateway: [u8; 16],
        enabled: bool,
    ) -> Result<(), Error> {
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let actor = self.acting_administrator(&tx, browser).await?;
        let gateway = Uuid::from_bytes(gateway);
        tx.query_one_raw(statement(
            "SELECT id FROM central_identity.gateways WHERE id = $1 FOR UPDATE",
            vec![gateway.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::Rejected)?;
        let changed = tx
            .query_one_raw(statement(
                "INSERT INTO central_identity.gateway_approvals (gateway_id, enabled) VALUES ($1, $2)
                 ON CONFLICT (gateway_id) DO UPDATE SET enabled = EXCLUDED.enabled
                 WHERE central_identity.gateway_approvals.enabled IS DISTINCT FROM EXCLUDED.enabled
                 RETURNING gateway_id",
                vec![gateway.into(), enabled.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .is_some();
        if changed {
            if !enabled {
                tx.execute_raw(statement(
                    "DELETE FROM central_identity.gateway_leases WHERE gateway_id = $1",
                    vec![gateway.into()],
                ))
                .await
                .map_err(|_| Error::Unavailable)?;
            }
            let action = if enabled {
                Action::GatewayApproved
            } else {
                Action::GatewayDisabled
            };
            audit_gateway(&tx, actor, action, gateway).await?;
        }
        tx.commit().await.map_err(|_| Error::Unavailable)
    }

    /// Revoke the gateway token `token` for the administrator behind
    /// `browser`, whose password sign-in must be less than 30 minutes old.
    /// One revoked already changes nothing.
    ///
    /// # Errors
    /// `Rejected` without a fresh administrator's browser session or for an
    /// unknown token; storage failure.
    pub async fn admin_revoke_gateway_token(
        &self,
        browser: &Token,
        token: [u8; 16],
    ) -> Result<(), Error> {
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let actor = self.acting_administrator(&tx, browser).await?;
        let token = Uuid::from_bytes(token);
        let row = tx
            .query_one_raw(statement(
                "SELECT gateway_id, revoked_at IS NULL AS live FROM central_identity.gateway_tokens
                 WHERE id = $1",
                vec![token.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        let gateway: Uuid = row
            .try_get("", "gateway_id")
            .map_err(|_| Error::Unavailable)?;
        if row
            .try_get::<bool>("", "live")
            .map_err(|_| Error::Unavailable)?
        {
            tx.execute_raw(statement(
                "UPDATE central_identity.gateway_tokens SET revoked_at = pg_catalog.clock_timestamp()
                 WHERE id = $1 AND revoked_at IS NULL",
                vec![token.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
            audit_gateway(&tx, actor, Action::GatewayTokenRevoked, gateway).await?;
        }
        tx.commit().await.map_err(|_| Error::Unavailable)
    }

    /// Give the enrolled account `target` the tag `tag`, for the
    /// administrator behind `browser`, whose password sign-in must be less
    /// than 30 minutes old. Tags stay unique, so the handle `Name#tag` does.
    /// The same tag again changes nothing; a change writes its audit event,
    /// with the old and the new tag, in the same transaction.
    ///
    /// # Errors
    /// `Rejected` without a fresh administrator's browser session, for a tag
    /// out of range and for an unknown or unenrolled account; `Conflict` for a
    /// tag another account holds; storage failure.
    pub async fn set_tag(&self, browser: &Token, target: [u8; 16], tag: u32) -> Result<(), Error> {
        if !valid_tag(tag) {
            return Err(Error::Rejected);
        }
        let tag = i32::try_from(tag).map_err(|_| Error::Rejected)?;
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let actor = self.acting_administrator(&tx, browser).await?;
        let target = Uuid::from_bytes(target);
        let before: Option<i32> = tx
            .query_one_raw(statement(
                "SELECT tag FROM central_identity.accounts WHERE id = $1 AND credential IS NOT NULL
                 FOR UPDATE",
                vec![target.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?
            .try_get("", "tag")
            .map_err(|_| Error::Unavailable)?;
        if before == Some(tag) {
            return tx.commit().await.map_err(|_| Error::Unavailable);
        }
        let taken = tx
            .query_one_raw(statement(
                "SELECT 1 AS taken FROM central_identity.accounts WHERE tag = $1",
                vec![tag.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .is_some();
        if taken {
            return Err(Error::Conflict);
        }
        // A tag taken by a concurrent change is the unique index's to refuse.
        tx.execute_raw(statement(
            "UPDATE central_identity.accounts SET tag = $2 WHERE id = $1",
            vec![target.into(), tag.into()],
        ))
        .await
        .map_err(|_| Error::Conflict)?;
        tx.execute_raw(statement(
            "INSERT INTO central_identity.audit_events (actor_kind, actor_account, action, account_id, detail)
             VALUES ('admin', $1, $2, $3, pg_catalog.jsonb_build_object('from', $4::integer, 'to', $5::integer))",
            vec![
                actor.into(),
                Action::TagChanged.as_str().into(),
                target.into(),
                before.into(),
                tag.into(),
            ],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        tx.commit().await.map_err(|_| Error::Unavailable)
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
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let actor = self.acting_administrator(&tx, browser).await?;
        let target = Uuid::from_bytes(target);
        if !admin && actor == target {
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

/// What the operator's [`sweep`] deleted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Swept {
    /// Audit events older than [`AUDIT_YEARS`].
    pub events: u64,
    /// Reset codes past their expiry, whose digests no longer work.
    pub codes: u64,
    /// Pairings past their five minutes, with what the client and the page
    /// left in them.
    pub pairings: u64,
    /// Reports the retention says are over (`docs/reports.md`), their
    /// history kept.
    pub reports: u64,
}

/// Delete audit events older than [`AUDIT_YEARS`], the digests of expired
/// reset codes, expired pairings and the reports whose retention is over,
/// under the operator's (migrator's) login, as the privacy notice allows;
/// all or none.
///
/// # Errors
/// Storage failure.
pub async fn sweep(db: &DatabaseConnection) -> Result<Swept, Error> {
    let tx = db.begin().await.map_err(|_| Error::Unavailable)?;
    let events = tx
        .execute_raw(statement(
            "DELETE FROM central_identity.audit_events
             WHERE at < pg_catalog.clock_timestamp() - pg_catalog.make_interval(years => $1)",
            vec![AUDIT_YEARS.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .rows_affected();
    let codes = tx
        .execute_raw(statement(
            "UPDATE central_identity.accounts SET reset_digest = NULL, reset_expires = NULL
             WHERE reset_expires <= pg_catalog.clock_timestamp()",
            vec![],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .rows_affected();
    let pairings = tx
        .execute_raw(statement(
            "DELETE FROM central_identity.pairings WHERE expires_at <= pg_catalog.clock_timestamp()",
            vec![],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .rows_affected();
    let reports = super::reports::sweep(&tx).await?;
    tx.commit().await.map_err(|_| Error::Unavailable)?;
    Ok(Swept {
        events,
        codes,
        pairings,
        reports,
    })
}
