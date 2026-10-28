//! What Central stores about the session's account, for the person's own
//! export (`docs/account-deletion.md`, GDPR Art. 15 and 20). The queries
//! name their columns and never read a credential, a wrapper, a digest or a
//! document; every list has a stated bound.
use super::{AccountStore, Error, Session, admin, recovery, statement};
use central_api::admin::{AccountState, AuditEvent};
use central_api::export::{
    AcceptedTerms, AccountExport, AdministeredGateway, EXPORT_LIST, ExportedGateway,
    ExportedSession, SessionKind,
};
use sea_orm::{AccessMode, ConnectionTrait, IsolationLevel, QueryResult, TransactionTrait, Value};

fn get<T: sea_orm::TryGetable>(row: &QueryResult, column: &str) -> Result<T, Error> {
    row.try_get("", column).map_err(|_| Error::Unavailable)
}

fn unsigned(row: &QueryResult, column: &str) -> Result<u64, Error> {
    u64::try_from(get::<i64>(row, column)?).map_err(|_| Error::Unavailable)
}

/// One row past the bound, to tell a full list from a cut one.
fn bound() -> Value {
    i64::try_from(EXPORT_LIST + 1).unwrap_or(i64::MAX).into()
}

/// The rows of `sql` (bound `$1` to the account and `$2` to [`bound`]), read
/// with `read`, cut at [`EXPORT_LIST`]; and whether nothing was cut.
async fn list<T>(
    db: &impl ConnectionTrait,
    sql: &str,
    account: Value,
    read: impl Fn(&QueryResult) -> Result<T, Error>,
) -> Result<(Vec<T>, bool), Error> {
    let rows = db
        .query_all_raw(statement(sql, vec![account, bound()]))
        .await
        .map_err(|_| Error::Unavailable)?;
    let items = rows
        .iter()
        .take(EXPORT_LIST)
        .map(read)
        .collect::<Result<Vec<_>, _>>()?;
    Ok((items, rows.len() <= EXPORT_LIST))
}

/// The account's sessions, at most one of each kind, without their digests.
async fn sessions(
    db: &impl ConnectionTrait,
    account: Value,
) -> Result<Vec<ExportedSession>, Error> {
    db.query_all_raw(statement(
        "SELECT kind, floor(extract(epoch FROM created_at))::bigint AS created_at,
             floor(extract(epoch FROM expires_at))::bigint AS expires_at
         FROM central_identity.sessions WHERE account_id = $1 ORDER BY kind",
        vec![account],
    ))
    .await
    .map_err(|_| Error::Unavailable)?
    .iter()
    .map(|row| {
        Ok(ExportedSession {
            kind: match get::<String>(row, "kind")?.as_str() {
                "native" => SessionKind::Native,
                "browser" => SessionKind::Browser,
                _ => return Err(Error::Unavailable),
            },
            created_at: get(row, "created_at")?,
            expires_at: get(row, "expires_at")?,
        })
    })
    .collect()
}

/// The export's bounded lists.
struct Lists {
    terms: Vec<AcceptedTerms>,
    gateways: Vec<ExportedGateway>,
    administers: Vec<AdministeredGateway>,
    audit: Vec<AuditEvent>,
    complete: bool,
}

impl Lists {
    async fn read(db: &impl ConnectionTrait, account: &Value) -> Result<Self, Error> {
        let (terms, terms_complete) = list(
            db,
            "SELECT subject, version, floor(extract(epoch FROM accepted_at))::bigint AS accepted_at
             FROM central_identity.terms_acceptances WHERE account_id = $1
             ORDER BY accepted_at, subject, version LIMIT $2",
            account.clone(),
            |row| {
                Ok(AcceptedTerms {
                    subject: get(row, "subject")?,
                    version: get(row, "version")?,
                    accepted_at: get(row, "accepted_at")?,
                })
            },
        )
        .await?;
        let (gateways, gateways_complete) = list(
            db,
            "SELECT g.id, g.origin, g.name, g.version, g.website,
                 EXISTS (SELECT 1 FROM central_identity.gateway_approvals p
                     WHERE p.gateway_id = g.id) AS approved,
                 floor(extract(epoch FROM g.created_at))::bigint AS created_at,
                 floor(extract(epoch FROM g.last_seen))::bigint AS last_seen
             FROM central_identity.gateways g WHERE g.owner_id = $1 ORDER BY g.id LIMIT $2",
            account.clone(),
            |row| {
                Ok(ExportedGateway {
                    id: admin::id(row, "id")?,
                    origin: get(row, "origin")?,
                    name: get(row, "name")?,
                    version: get(row, "version")?,
                    website: get(row, "website")?,
                    approved: get(row, "approved")?,
                    created_at: get(row, "created_at")?,
                    last_seen: get(row, "last_seen")?,
                })
            },
        )
        .await?;
        let (administers, administers_complete) = list(
            db,
            "SELECT g.id, g.origin FROM central_identity.gateway_admins m
             JOIN central_identity.gateways g ON g.id = m.gateway_id
             WHERE m.account_id = $1 ORDER BY g.id LIMIT $2",
            account.clone(),
            |row| {
                Ok(AdministeredGateway {
                    id: admin::id(row, "id")?,
                    origin: get(row, "origin")?,
                })
            },
        )
        .await?;
        // As the account acted on, and as the administrator who acted.
        let (audit, audit_complete) = list(
            db,
            &format!(
                "SELECT {} FROM central_identity.audit_events e
                 WHERE e.account_id = $1 OR e.actor_account = $1 ORDER BY e.id DESC LIMIT $2",
                admin::EVENT_COLUMNS
            ),
            account.clone(),
            admin::event,
        )
        .await?;
        Ok(Self {
            terms,
            gateways,
            administers,
            audit,
            complete: terms_complete && gateways_complete && administers_complete && audit_complete,
        })
    }
}

impl AccountStore {
    /// Everything Central stores about the account of the live session
    /// `session`, whose password sign-in must be less than 30 minutes old,
    /// apart from its secrets and ciphertext; one consistent snapshot.
    ///
    /// # Errors
    /// Rejects an expired, replaced or stale session, a session of the other
    /// kind and a sign-in 30 minutes old or older; reports storage failure.
    pub async fn account_export(&self, session: Session<'_>) -> Result<AccountExport, Error> {
        let tx = self
            .db
            .begin_with_config(
                Some(IsolationLevel::RepeatableRead),
                Some(AccessMode::ReadOnly),
            )
            .await
            .map_err(|_| Error::Unavailable)?;
        let row = tx
            .query_one_raw(statement(
                "SELECT a.id, a.vault_epoch, a.username_key, a.display_name, a.tag,
                     floor(extract(epoch FROM a.display_name_at))::bigint AS display_name_at,
                     a.source, a.legacy_id,
                     floor(extract(epoch FROM a.created_at))::bigint AS created_at,
                     floor(extract(epoch FROM a.last_sign_in_at))::bigint AS last_sign_in_at,
                     a.document_revision, a.restored, a.object_bytes,
                     a.recovery_wrapped_master IS NOT NULL AS recovery,
                     EXISTS (SELECT 1 FROM central_identity.account_roles r
                         WHERE r.account_id = a.id AND r.role = 'admin') AS admin,
                     CASE WHEN a.reset_expires > pg_catalog.clock_timestamp()
                         THEN floor(extract(epoch FROM a.reset_expires))::bigint END AS reset_until,
                     (SELECT count(*) FROM central_identity.private_objects o
                         WHERE o.account_id = a.id AND NOT o.deleted) AS objects,
                     (SELECT count(*) FROM central_identity.migration_archives m
                         WHERE m.account_id = a.id) AS archives
                 FROM central_identity.accounts a JOIN central_identity.sessions s
                 ON s.account_id = a.id AND s.credential_revision = a.credential_revision
                 WHERE s.digest = $1 AND s.kind = $2 AND s.expires_at > pg_catalog.clock_timestamp()
                 AND s.created_at > pg_catalog.clock_timestamp() - interval '30 minutes'
                 AND a.credential IS NOT NULL",
                vec![session.digest().to_vec().into(), session.kind().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        let identity = super::identity(&row)?;
        let account: Value = super::uuid(identity.account).into();
        let (display_name, tag) = super::profiles::columns(&row)?;
        let sessions = sessions(&tx, account.clone()).await?;
        let lists = Lists::read(&tx, &account).await?;
        let export = AccountExport {
            account: identity.account,
            username: get(&row, "username_key")?,
            display_name,
            display_name_at: get(&row, "display_name_at")?,
            tag,
            source: recovery::source(&get::<String>(&row, "source")?)?,
            legacy_id: get(&row, "legacy_id")?,
            created_at: get(&row, "created_at")?,
            last_sign_in_at: get(&row, "last_sign_in_at")?,
            vault_epoch: identity.epoch,
            document_revision: unsigned(&row, "document_revision")?,
            state: if get(&row, "restored")? {
                AccountState::Restored
            } else {
                AccountState::Enrolled
            },
            admin: get(&row, "admin")?,
            recovery: get(&row, "recovery")?,
            reset_until: get(&row, "reset_until")?,
            objects: unsigned(&row, "objects")?,
            object_bytes: unsigned(&row, "object_bytes")?,
            archives: unsigned(&row, "archives")?,
            sessions,
            terms: lists.terms,
            gateways: lists.gateways,
            administers: lists.administers,
            audit: lists.audit,
            complete: lists.complete,
        };
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(export)
    }
}
