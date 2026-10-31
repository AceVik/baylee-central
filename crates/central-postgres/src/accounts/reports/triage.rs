//! What an administrator reads of one report and decides about it, with the
//! feedback service's rules (Baylee `docs/feedback.md` §"Triage"): one
//! duplicate level, the revision against unseen overwrites, `done_at` with
//! the done statuses, notes removed by their author only, the trash, and a
//! history line for every change, never the report's words.
use super::super::{AccountStore, Error, Token, statement, uuid};
use super::list::{SUMMARY_COLUMNS, split, summary};
use super::{history, release, sweep};
use central_api::reports::{
    BulkAction, MAX_BULK, MAX_NOTE, MAX_SEEN, MAX_VIEWS, PartState, ReportAdmin, ReportAuditRow,
    ReportChange, ReportDetail, ReportNote, ReportStatus, SavedView, valid_view,
};
use sea_orm::{ConnectionTrait, DatabaseTransaction, QueryResult, TransactionTrait, prelude::Uuid};

fn get<T: sea_orm::TryGetable>(row: &QueryResult, column: &str) -> Result<T, Error> {
    row.try_get("", column).map_err(|_| Error::Unavailable)
}

/// A report's triage as it stands, read for update.
struct Triage {
    status: ReportStatus,
    issue: Option<i32>,
    assignee: Option<Uuid>,
    priority: Option<String>,
    labels: Vec<String>,
    duplicate_of: Option<Uuid>,
    revision: i32,
    trashed: bool,
}

async fn locked(tx: &DatabaseTransaction, report: Uuid) -> Result<Triage, Error> {
    let row = tx
        .query_one_raw(statement(
            "SELECT status, issue_number, assignee, priority, pg_catalog.array_to_string(labels, ',') AS labels,
                 duplicate_of, revision,
                 deleted_at IS NOT NULL AS trashed
             FROM central_identity.reports WHERE id = $1 FOR UPDATE",
            vec![report.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::Rejected)?;
    Ok(Triage {
        status: ReportStatus::from_stored(&get::<String>(&row, "status")?)?,
        issue: get(&row, "issue_number")?,
        assignee: get(&row, "assignee")?,
        priority: get(&row, "priority")?,
        labels: split(&get::<String>(&row, "labels")?),
        duplicate_of: get(&row, "duplicate_of")?,
        revision: get(&row, "revision")?,
        trashed: get(&row, "trashed")?,
    })
}

fn shown(id: Option<Uuid>) -> String {
    id.map_or_else(|| "none".into(), |id| id.to_string())
}

/// Apply `change` to `report` by `admin` in `tx`: the rules, the row and its
/// history. Returns whether anything changed.
async fn apply(
    tx: &DatabaseTransaction,
    admin: Uuid,
    report: Uuid,
    change: &ReportChange,
    expected: Option<u32>,
) -> Result<bool, Error> {
    let was = locked(tx, report).await?;
    if expected.is_some_and(|e| i64::from(e) != i64::from(was.revision)) {
        return Err(Error::Conflict);
    }
    let mut status = change.status.unwrap_or(was.status);
    let mut duplicate_of = match change.duplicate_of {
        Some(target) => target.map(Uuid::from_bytes),
        None => was.duplicate_of,
    };
    // marking a duplicate sets its status, unmarking one sets `triaged`
    // unless the change names another; leaving the status clears the mark
    if change.duplicate_of.is_some_and(|t| t.is_some()) {
        status = ReportStatus::Duplicate;
    } else if change.duplicate_of == Some(None)
        && change.status.is_none()
        && was.status == ReportStatus::Duplicate
    {
        status = ReportStatus::Triaged;
    }
    if status != ReportStatus::Duplicate {
        duplicate_of = None;
    } else if duplicate_of.is_none() {
        return Err(Error::Rejected);
    }
    if let Some(original) = duplicate_of.filter(|o| Some(*o) != was.duplicate_of) {
        // one level: the original is no duplicate itself, and not this one
        let row = tx
            .query_one_raw(statement(
                "SELECT duplicate_of IS NULL AS free FROM central_identity.reports WHERE id = $1 AND id <> $2",
                vec![original.into(), report.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        if !get::<bool>(&row, "free")? {
            return Err(Error::Rejected);
        }
    }
    let assignee = match change.assignee {
        Some(target) => target.map(Uuid::from_bytes),
        None => was.assignee,
    };
    if let Some(assignee) = assignee.filter(|a| Some(*a) != was.assignee) {
        let row = tx
            .query_one_raw(statement(
                "SELECT 1 AS yes FROM central_identity.account_roles WHERE account_id = $1 AND role = 'admin'",
                vec![assignee.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        if row.is_none() {
            return Err(Error::Rejected);
        }
    }
    let issue = change.issue.map_or(was.issue, |i| {
        i.map(|n| i32::try_from(n).unwrap_or(i32::MAX))
    });
    let priority = change
        .priority
        .map_or(was.priority.clone(), |p| p.map(|p| p.as_str().to_owned()));
    let mut labels = change.labels.clone().unwrap_or_else(|| was.labels.clone());
    labels.extend(change.labels_add.iter().cloned());
    labels.retain(|l| !change.labels_remove.contains(l));
    labels.sort();
    labels.dedup();
    if labels.len() > central_api::reports::MAX_LABELS {
        return Err(Error::Rejected);
    }
    let mut lines: Vec<(&str, String)> = Vec::new();
    if status != was.status {
        lines.push((
            "status",
            format!("{} → {}", was.status.as_str(), status.as_str()),
        ));
    }
    if issue != was.issue {
        lines.push((
            "issue",
            issue.map_or_else(|| "none".into(), |n| format!("#{n}")),
        ));
    }
    if assignee != was.assignee {
        lines.push(("assignee", shown(assignee)));
    }
    if priority != was.priority {
        lines.push((
            "priority",
            priority.clone().unwrap_or_else(|| "none".into()),
        ));
    }
    if labels != was.labels {
        lines.push(("labels", labels.join(", ")));
    }
    if duplicate_of != was.duplicate_of {
        lines.push(("duplicate", shown(duplicate_of)));
    }
    if lines.is_empty() {
        return Ok(false);
    }
    tx.execute_raw(statement(
        "UPDATE central_identity.reports SET status = $2, issue_number = $3, assignee = $4, priority = $5,
             labels = COALESCE(pg_catalog.string_to_array(NULLIF($6::text, ''), ','), '{}'), duplicate_of = $7, revision = revision + 1, updated_at = pg_catalog.clock_timestamp(),
             done_at = CASE WHEN $2 IN ('resolved', 'wont_fix', 'duplicate')
                 THEN COALESCE(done_at, pg_catalog.clock_timestamp()) END
         WHERE id = $1",
        vec![
            report.into(),
            status.as_str().into(),
            issue.into(),
            assignee.into(),
            priority.into(),
            labels.join(",").into(),
            duplicate_of.into(),
        ],
    ))
    .await
    .map_err(|_| Error::Unavailable)?;
    // a report that became a duplicate hands its own duplicates on to its original
    if let Some(original) = duplicate_of.filter(|_| status == ReportStatus::Duplicate) {
        let moved = tx
            .query_all_raw(statement(
                "UPDATE central_identity.reports SET duplicate_of = $2, revision = revision + 1,
                     updated_at = pg_catalog.clock_timestamp()
                 WHERE duplicate_of = $1 RETURNING id",
                vec![report.into(), original.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        for row in &moved {
            history(
                tx,
                get(row, "id")?,
                Some(admin),
                "duplicate",
                &original.to_string(),
            )
            .await?;
        }
    }
    for (action, detail) in lines {
        history(tx, report, Some(admin), action, &detail).await?;
    }
    Ok(true)
}

/// Into the trash, out of it, or deleted for good from it, in `tx`.
async fn bin(
    tx: &DatabaseTransaction,
    admin: Uuid,
    report: Uuid,
    how: &BulkAction,
) -> Result<bool, Error> {
    let was = locked(tx, report).await?;
    let (sql, action) = match how {
        BulkAction::Trash if !was.trashed => (
            "UPDATE central_identity.reports SET deleted_at = pg_catalog.clock_timestamp(),
                 updated_at = pg_catalog.clock_timestamp(), revision = revision + 1 WHERE id = $1",
            "trashed",
        ),
        BulkAction::Restore if was.trashed => (
            "UPDATE central_identity.reports SET deleted_at = NULL,
                 updated_at = pg_catalog.clock_timestamp(), revision = revision + 1 WHERE id = $1",
            "restored",
        ),
        BulkAction::Purge if was.trashed => {
            // its duplicates keep no original: they are opened again first
            release(tx, &[report], Some(admin)).await?;
            (
                "DELETE FROM central_identity.reports WHERE id = $1",
                "purged",
            )
        }
        BulkAction::Purge => return Err(Error::Rejected),
        _ => return Ok(false),
    };
    tx.execute_raw(statement(sql, vec![report.into()]))
        .await
        .map_err(|_| Error::Unavailable)?;
    history(tx, report, Some(admin), action, "").await?;
    Ok(true)
}

impl AccountStore {
    /// The administrator behind `browser` in `tx`, refused unless their
    /// password sign-in is fresh when `fresh` (a purge).
    async fn triager(
        tx: &DatabaseTransaction,
        browser: &Token,
        fresh: bool,
    ) -> Result<Uuid, Error> {
        let (admin, recent) = Self::administrator(tx, browser).await?;
        if fresh && !recent {
            return Err(Error::Rejected);
        }
        Ok(admin)
    }

    async fn detail(
        tx: &DatabaseTransaction,
        admin: Uuid,
        report: Uuid,
    ) -> Result<ReportDetail, Error> {
        let row = tx
            .query_one_raw(statement(
                &format!(
                    "SELECT {SUMMARY_COLUMNS}, r.text, r.game_id, r.gateway_url, r.build_commit, r.crash_location,
                         r.client_bytes,
                         EXISTS (SELECT 1 FROM central_identity.report_parts p WHERE p.report_id = r.id AND p.part = 'client') AS client_in,
                         EXISTS (SELECT 1 FROM central_identity.report_parts p WHERE p.report_id = r.id AND p.part = 'record') AS record_in
                     FROM central_identity.reports r WHERE r.id = $2"
                ),
                vec![admin.into(), report.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        let summary = summary(&row)?;
        let part = |bytes: Option<i32>, received: bool| {
            bytes
                .map(|b| {
                    u32::try_from(b)
                        .map(|bytes| PartState { bytes, received })
                        .map_err(|_| Error::Unavailable)
                })
                .transpose()
        };
        let client = part(get(&row, "client_bytes")?, get(&row, "client_in")?)?;
        let record = part(get(&row, "record_bytes")?, get(&row, "record_in")?)?;
        let notes = tx
            .query_all_raw(statement(
                "SELECT id, floor(extract(epoch FROM at))::bigint AS at, author, text
                 FROM central_identity.report_notes WHERE report_id = $1 ORDER BY at, id",
                vec![report.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .iter()
            .map(|n| {
                Ok(ReportNote {
                    id: *get::<Uuid>(n, "id")?.as_bytes(),
                    at: get(n, "at")?,
                    author: *get::<Uuid>(n, "author")?.as_bytes(),
                    text: get(n, "text")?,
                })
            })
            .collect::<Result<_, Error>>()?;
        let audit = tx
            .query_all_raw(statement(
                "SELECT floor(extract(epoch FROM at))::bigint AS at, actor, action, detail
                 FROM central_identity.report_audit WHERE report_id = $1 ORDER BY id",
                vec![report.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .iter()
            .map(|a| {
                Ok(ReportAuditRow {
                    at: get(a, "at")?,
                    actor: get::<Option<Uuid>>(a, "actor")?.map(|id| *id.as_bytes()),
                    action: get(a, "action")?,
                    detail: get(a, "detail")?,
                })
            })
            .collect::<Result<_, Error>>()?;
        Ok(ReportDetail {
            summary,
            text: get(&row, "text")?,
            game_id: get(&row, "game_id")?,
            gateway_url: get(&row, "gateway_url")?,
            commit: get(&row, "build_commit")?,
            crash_location: get(&row, "crash_location")?,
            client,
            record,
            notes,
            audit,
        })
    }

    /// One report with its notes and history, for the administrator behind `browser`.
    ///
    /// # Errors
    /// `Rejected` without an administrator's live browser session or for an
    /// unknown report; storage failure.
    pub async fn report_detail(
        &self,
        browser: &Token,
        report: [u8; 16],
    ) -> Result<ReportDetail, Error> {
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let admin = Self::triager(&tx, browser, false).await?;
        sweep(&tx).await?;
        let detail = Self::detail(&tx, admin, uuid(report)).await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(detail)
    }

    /// Change one report's triage; `Conflict` past `expected`. Answers the report.
    ///
    /// # Errors
    /// `Rejected` for a change out of bounds or against the rules (a second
    /// duplicate level, an assignee who is no administrator, a duplicate
    /// without its original), an unknown report or no administrator;
    /// `Conflict` on another revision; storage failure.
    pub async fn set_report(
        &self,
        browser: &Token,
        report: [u8; 16],
        change: &ReportChange,
        expected: Option<u32>,
    ) -> Result<ReportDetail, Error> {
        change.validate(false)?;
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let admin = Self::triager(&tx, browser, false).await?;
        apply(&tx, admin, uuid(report), change, expected).await?;
        let detail = Self::detail(&tx, admin, uuid(report)).await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(detail)
    }

    /// Change, trash, restore or purge a selection in one transaction: all of
    /// it or none. A purge needs a fresh password sign-in. Returns how many changed.
    ///
    /// # Errors
    /// As [`Self::set_report`] for any of them, and `Rejected` for an empty
    /// or oversized selection or a purge of a report not in the trash.
    pub async fn reports_bulk(
        &self,
        browser: &Token,
        reports: &[[u8; 16]],
        how: &BulkAction,
    ) -> Result<u32, Error> {
        if reports.is_empty() || reports.len() > MAX_BULK {
            return Err(Error::Rejected);
        }
        if let BulkAction::Change { change } = how {
            change.validate(true)?;
        }
        let mut ids: Vec<Uuid> = reports.iter().map(|r| uuid(*r)).collect();
        // one lock order, so two selections never deadlock
        ids.sort_unstable();
        ids.dedup();
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let admin = Self::triager(&tx, browser, matches!(how, BulkAction::Purge)).await?;
        let mut changed = 0_u32;
        for report in ids {
            let did = match how {
                BulkAction::Change { change } => apply(&tx, admin, report, change, None).await?,
                bin_action => bin(&tx, admin, report, bin_action).await?,
            };
            changed += u32::from(did);
        }
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(changed)
    }

    /// Into the trash, out of it, or (with a fresh sign-in) deleted for good from it.
    ///
    /// # Errors
    /// As [`Self::reports_bulk`].
    pub async fn bin_report(
        &self,
        browser: &Token,
        report: [u8; 16],
        how: &BulkAction,
    ) -> Result<(), Error> {
        if matches!(how, BulkAction::Change { .. }) {
            return Err(Error::Rejected);
        }
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let admin = Self::triager(&tx, browser, matches!(how, BulkAction::Purge)).await?;
        bin(&tx, admin, uuid(report), how).await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(())
    }

    /// A note on a report by the administrator behind `browser`. Answers the report.
    ///
    /// # Errors
    /// `Rejected` for an empty or overlong note, an unknown report or no
    /// administrator; storage failure.
    pub async fn add_report_note(
        &self,
        browser: &Token,
        report: [u8; 16],
        text: &str,
    ) -> Result<ReportDetail, Error> {
        if text.trim().is_empty() || text.chars().count() > MAX_NOTE {
            return Err(Error::Rejected);
        }
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let admin = Self::triager(&tx, browser, false).await?;
        locked(&tx, uuid(report)).await?;
        tx.execute_raw(statement(
            "INSERT INTO central_identity.report_notes (report_id, author, text) VALUES ($1, $2, $3)",
            vec![uuid(report).into(), admin.into(), text.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        history(&tx, uuid(report), Some(admin), "note", "added").await?;
        let detail = Self::detail(&tx, admin, uuid(report)).await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(detail)
    }

    /// A note removed by its author. Answers the report.
    ///
    /// # Errors
    /// `Rejected` for another's note, an unknown one or no administrator; storage failure.
    pub async fn remove_report_note(
        &self,
        browser: &Token,
        report: [u8; 16],
        note: [u8; 16],
    ) -> Result<ReportDetail, Error> {
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let admin = Self::triager(&tx, browser, false).await?;
        let removed = tx
            .execute_raw(statement(
                "DELETE FROM central_identity.report_notes WHERE id = $1 AND report_id = $2 AND author = $3",
                vec![uuid(note).into(), uuid(report).into(), admin.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        if removed.rows_affected() == 0 {
            return Err(Error::Rejected);
        }
        history(&tx, uuid(report), Some(admin), "note", "removed").await?;
        let detail = Self::detail(&tx, admin, uuid(report)).await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(detail)
    }

    async fn views(db: &impl ConnectionTrait, admin: Uuid) -> Result<Vec<SavedView>, Error> {
        db.query_all_raw(statement(
            "SELECT id, name, query FROM central_identity.report_views WHERE admin = $1 ORDER BY created_at, id",
            vec![admin.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .iter()
        .map(|v| Ok(SavedView { id: *get::<Uuid>(v, "id")?.as_bytes(), name: get(v, "name")?, query: get(v, "query")? }))
        .collect()
    }

    /// The saved lists of the administrator behind `browser`.
    ///
    /// # Errors
    /// `Rejected` without an administrator; storage failure.
    pub async fn report_views(&self, browser: &Token) -> Result<Vec<SavedView>, Error> {
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let admin = Self::triager(&tx, browser, false).await?;
        let views = Self::views(&tx, admin).await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(views)
    }

    /// Save a list; a name saved again replaces its query. At most [`MAX_VIEWS`].
    ///
    /// # Errors
    /// `Rejected` for a name or query out of bounds; `Busy` past the most; storage failure.
    pub async fn save_report_view(
        &self,
        browser: &Token,
        name: &str,
        query: &str,
    ) -> Result<Vec<SavedView>, Error> {
        if !valid_view(name, query) {
            return Err(Error::Rejected);
        }
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let admin = Self::triager(&tx, browser, false).await?;
        let replaced = tx
            .execute_raw(statement(
                "UPDATE central_identity.report_views SET query = $3 WHERE admin = $1 AND name = $2",
                vec![admin.into(), name.trim().into(), query.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        if replaced.rows_affected() == 0 {
            if Self::views(&tx, admin).await?.len() >= MAX_VIEWS {
                return Err(Error::Busy);
            }
            tx.execute_raw(statement(
                "INSERT INTO central_identity.report_views (admin, name, query) VALUES ($1, $2, $3)",
                vec![admin.into(), name.trim().into(), query.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        }
        let views = Self::views(&tx, admin).await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(views)
    }

    /// Forget a saved list of the administrator behind `browser`.
    ///
    /// # Errors
    /// `Rejected` without an administrator; storage failure.
    pub async fn delete_report_view(
        &self,
        browser: &Token,
        view: [u8; 16],
    ) -> Result<Vec<SavedView>, Error> {
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let admin = Self::triager(&tx, browser, false).await?;
        tx.execute_raw(statement(
            "DELETE FROM central_identity.report_views WHERE id = $1 AND admin = $2",
            vec![uuid(view).into(), admin.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        let views = Self::views(&tx, admin).await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(views)
    }

    /// Mark reports opened, or not, for the administrator behind `browser`.
    ///
    /// # Errors
    /// `Rejected` for an empty or oversized selection or no administrator; storage failure.
    pub async fn set_reports_seen(
        &self,
        browser: &Token,
        reports: &[[u8; 16]],
        seen: bool,
    ) -> Result<(), Error> {
        if reports.is_empty() || reports.len() > MAX_SEEN {
            return Err(Error::Rejected);
        }
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let admin = Self::triager(&tx, browser, false).await?;
        let ids = reports
            .iter()
            .map(|r| uuid(*r).to_string())
            .collect::<Vec<_>>()
            .join(",");
        let sql = if seen {
            "INSERT INTO central_identity.report_seen (admin, report_id)
             SELECT $1, r.id FROM central_identity.reports r
             WHERE r.id = ANY(pg_catalog.string_to_array($2::text, ',')::uuid[])
             ON CONFLICT DO NOTHING"
        } else {
            "DELETE FROM central_identity.report_seen
             WHERE admin = $1 AND report_id = ANY(pg_catalog.string_to_array($2::text, ',')::uuid[])"
        };
        tx.execute_raw(statement(sql, vec![admin.into(), ids.into()]))
            .await
            .map_err(|_| Error::Unavailable)?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(())
    }

    /// The administrators a report may be assigned to, for the administrator behind `browser`.
    ///
    /// # Errors
    /// `Rejected` without an administrator; storage failure.
    pub async fn report_admins(&self, browser: &Token) -> Result<Vec<ReportAdmin>, Error> {
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        Self::triager(&tx, browser, false).await?;
        let admins = tx
            .query_all_raw(statement(
                "SELECT a.id, a.display_name, a.tag FROM central_identity.accounts a
                 JOIN central_identity.account_roles r ON r.account_id = a.id AND r.role = 'admin'
                 ORDER BY lower(COALESCE(a.display_name, '')), a.id",
                vec![],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .iter()
            .map(|row| {
                let (display_name, tag) = super::super::profiles::columns(row)?;
                Ok(ReportAdmin {
                    id: *get::<Uuid>(row, "id")?.as_bytes(),
                    display_name,
                    tag,
                })
            })
            .collect::<Result<_, Error>>()?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(admins)
    }
}
