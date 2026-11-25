//! What an administrator reads of one report and decides about it, with the
//! feedback service's rules (Baylee `docs/feedback.md` §"Triage"): one
//! duplicate level, the revision against unseen overwrites, `done_at` with
//! the done statuses, notes removed by their author only, the trash, and a
//! history line for every change, never the report's words.
use super::super::admin::limit;
use super::super::{AccountStore, Error, Token, statement, uuid};
use super::list::{SUMMARY_COLUMNS, split, summary};
use super::{Actor, ReportCaller, history, release, sweep};
use central_api::report_tokens::ReportScope;
use central_api::reports::{
    BulkAction, HISTORY, MAX_BULK, MAX_HANDED_ON, MAX_NOTE, MAX_NOTES_BYTES, MAX_SEEN, MAX_VIEWS,
    NOTE_BYTES, PartState, ReportAdmin, ReportAuditRow, ReportChange, ReportDetail, ReportNote,
    ReportStatus, SavedView, valid_view,
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

/// A report's triage after a change, before it is written.
struct Decided {
    status: ReportStatus,
    issue: Option<i32>,
    assignee: Option<Uuid>,
    priority: Option<String>,
    labels: Vec<String>,
    duplicate_of: Option<Uuid>,
}

/// What `change` makes of `was`, by the rules that need no other row.
fn decide(was: &Triage, change: &ReportChange) -> Result<Decided, Error> {
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
    let assignee = match change.assignee {
        Some(target) => target.map(Uuid::from_bytes),
        None => was.assignee,
    };
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
    Ok(Decided {
        status,
        issue,
        assignee,
        priority,
        labels,
        duplicate_of,
    })
}

/// The rules that read other rows: a new original is in one level and is
/// not `report`, and a new assignee holds the administrator's role.
async fn allowed(
    tx: &DatabaseTransaction,
    report: Uuid,
    was: &Triage,
    next: &Decided,
) -> Result<(), Error> {
    if let Some(original) = next.duplicate_of.filter(|o| Some(*o) != was.duplicate_of) {
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
    if let Some(assignee) = next.assignee.filter(|a| Some(*a) != was.assignee) {
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
    Ok(())
}

/// One history line per part that changes; none when nothing does.
fn lines(was: &Triage, next: &Decided) -> Vec<(&'static str, String)> {
    let mut lines = Vec::new();
    if next.status != was.status {
        lines.push((
            "status",
            format!("{} → {}", was.status.as_str(), next.status.as_str()),
        ));
    }
    if next.issue != was.issue {
        lines.push((
            "issue",
            next.issue
                .map_or_else(|| "none".into(), |n| format!("#{n}")),
        ));
    }
    if next.assignee != was.assignee {
        lines.push(("assignee", shown(next.assignee)));
    }
    if next.priority != was.priority {
        lines.push((
            "priority",
            next.priority.clone().unwrap_or_else(|| "none".into()),
        ));
    }
    if next.labels != was.labels {
        lines.push(("labels", next.labels.join(", ")));
    }
    if next.duplicate_of != was.duplicate_of {
        lines.push(("duplicate", shown(next.duplicate_of)));
    }
    lines
}

/// Hand `report`'s duplicates on to `original`, of which it became one, each
/// with its line of history. `handed` counts those of the whole request: a
/// report token hands on at most [`MAX_HANDED_ON`], which is asked before
/// anything is written. `report`'s row is locked, and a row with a duplicate
/// is locked by whoever adds one, so none comes in between.
async fn hand_on(
    tx: &DatabaseTransaction,
    actor: &Actor,
    report: Uuid,
    original: Uuid,
    handed: &mut usize,
) -> Result<(), Error> {
    if actor.token_name.is_some() {
        let more = tx
            .query_all_raw(statement(
                "SELECT 1 AS one FROM central_identity.reports WHERE duplicate_of = $1 LIMIT $2",
                vec![report.into(), limit(MAX_HANDED_ON + 1)],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .len();
        *handed += more;
        if *handed > MAX_HANDED_ON {
            return Err(Error::Rejected);
        }
    }
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
            Some(actor),
            "duplicate",
            &original.to_string(),
        )
        .await?;
    }
    Ok(())
}

/// Apply `change` to `report` by `actor` in `tx`: the rules, the row and its
/// history; `handed` is the whole request's ([`hand_on`]). Returns whether
/// anything changed.
async fn apply(
    tx: &DatabaseTransaction,
    actor: &Actor,
    report: Uuid,
    change: &ReportChange,
    expected: Option<u32>,
    handed: &mut usize,
) -> Result<bool, Error> {
    let was = locked(tx, report).await?;
    if expected.is_some_and(|e| i64::from(e) != i64::from(was.revision)) {
        return Err(Error::Conflict);
    }
    let next = decide(&was, change)?;
    allowed(tx, report, &was, &next).await?;
    let lines = lines(&was, &next);
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
            next.status.as_str().into(),
            next.issue.into(),
            next.assignee.into(),
            next.priority.into(),
            next.labels.join(",").into(),
            next.duplicate_of.into(),
        ],
    ))
    .await
    .map_err(|_| Error::Unavailable)?;
    // a report that became a duplicate hands its own duplicates on to its original
    if let Some(original) = next.duplicate_of {
        hand_on(tx, actor, report, original, handed).await?;
    }
    for (action, detail) in lines {
        history(tx, report, Some(actor), action, &detail).await?;
    }
    Ok(true)
}

/// Into the trash, out of it, or deleted for good from it, in `tx`.
async fn bin(
    tx: &DatabaseTransaction,
    actor: &Actor,
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
            release(tx, &[report], Some(actor)).await?;
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
    history(tx, report, Some(actor), action, "").await?;
    Ok(true)
}

/// The right a bin's action needs: a purge, which cannot be undone, its own.
fn right(how: &BulkAction) -> ReportScope {
    match how {
        BulkAction::Purge => ReportScope::Purge,
        BulkAction::Change { .. } | BulkAction::Trash | BulkAction::Restore => ReportScope::Triage,
    }
}

impl AccountStore {
    /// The administrator behind `browser` in `tx`.
    async fn triager(tx: &DatabaseTransaction, browser: &Token) -> Result<Uuid, Error> {
        Ok(Self::administrator(tx, browser).await?.0)
    }

    async fn detail(
        tx: &DatabaseTransaction,
        actor: &Actor,
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
                vec![actor.id.into(), report.into()],
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
                "SELECT id, floor(extract(epoch FROM at))::bigint AS at, author, token_name, text
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
                    token: get(n, "token_name")?,
                    text: get(n, "text")?,
                })
            })
            .collect::<Result<_, Error>>()?;
        // the newest lines, and one more that says there are older ones
        let lines = tx
            .query_all_raw(statement(
                "SELECT floor(extract(epoch FROM at))::bigint AS at, actor, token_name, action, detail
                 FROM central_identity.report_audit WHERE report_id = $1 ORDER BY id DESC LIMIT $2",
                vec![report.into(), limit(HISTORY + 1)],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        let audit = lines
            .iter()
            .take(HISTORY)
            .rev()
            .map(|a| {
                Ok(ReportAuditRow {
                    at: get(a, "at")?,
                    actor: get::<Option<Uuid>>(a, "actor")?.map(|id| *id.as_bytes()),
                    token: get(a, "token_name")?,
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
            older: lines.len() > HISTORY,
        })
    }

    /// One report with its notes and history, for `caller`.
    ///
    /// # Errors
    /// `Rejected` without an administrator's live browser session or a
    /// report token that may read, or for an unknown report; storage failure.
    pub async fn report_detail(
        &self,
        caller: ReportCaller<'_>,
        report: [u8; 16],
    ) -> Result<ReportDetail, Error> {
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let reader = Actor::check(&tx, caller, ReportScope::Read).await?;
        sweep(&tx).await?;
        let detail = Self::detail(&tx, &reader, uuid(report)).await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(detail)
    }

    /// Change one report's triage; `Conflict` past `expected`. Answers the report.
    ///
    /// # Errors
    /// `Rejected` for a change out of bounds or against the rules (a second
    /// duplicate level, an assignee who is no administrator, a duplicate
    /// without its original, a report token handing on more than
    /// [`MAX_HANDED_ON`] duplicates), an unknown report, or neither an
    /// administrator nor a report token that may triage; `Conflict` on
    /// another revision; storage failure.
    pub async fn set_report(
        &self,
        caller: ReportCaller<'_>,
        report: [u8; 16],
        change: &ReportChange,
        expected: Option<u32>,
    ) -> Result<ReportDetail, Error> {
        change.validate(false)?;
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let actor = Actor::check(&tx, caller, ReportScope::Triage).await?;
        apply(&tx, &actor, uuid(report), change, expected, &mut 0).await?;
        let detail = Self::detail(&tx, &actor, uuid(report)).await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(detail)
    }

    /// Change, trash, restore or purge a selection in one transaction: all of
    /// it or none. A purge needs an administrator's fresh password sign-in,
    /// or a report token that may purge. Returns how many changed.
    ///
    /// # Errors
    /// As [`Self::set_report`] for any of them, and `Rejected` for an empty
    /// or oversized selection or a purge of a report not in the trash.
    pub async fn reports_bulk(
        &self,
        caller: ReportCaller<'_>,
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
        let actor = Actor::check(&tx, caller, right(how)).await?;
        let (mut changed, mut handed) = (0_u32, 0);
        for report in ids {
            let did = match how {
                BulkAction::Change { change } => {
                    apply(&tx, &actor, report, change, None, &mut handed).await?
                }
                bin_action => bin(&tx, &actor, report, bin_action).await?,
            };
            changed += u32::from(did);
        }
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(changed)
    }

    /// Into the trash, out of it, or deleted for good from it (an
    /// administrator with a fresh sign-in, a report token that may purge).
    ///
    /// # Errors
    /// As [`Self::reports_bulk`].
    pub async fn bin_report(
        &self,
        caller: ReportCaller<'_>,
        report: [u8; 16],
        how: &BulkAction,
    ) -> Result<(), Error> {
        if matches!(how, BulkAction::Change { .. }) {
            return Err(Error::Rejected);
        }
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let actor = Actor::check(&tx, caller, right(how)).await?;
        bin(&tx, &actor, uuid(report), how).await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(())
    }

    /// A note on a report by `caller`. Answers the report.
    ///
    /// # Errors
    /// `Rejected` for an empty or overlong note, an unknown report, or
    /// neither an administrator nor a report token that may triage; `Busy`
    /// when the report's notes, each with its [`NOTE_BYTES`], would be more
    /// than [`MAX_NOTES_BYTES`]; storage failure.
    pub async fn add_report_note(
        &self,
        caller: ReportCaller<'_>,
        report: [u8; 16],
        text: &str,
    ) -> Result<ReportDetail, Error> {
        if text.trim().is_empty() || text.chars().count() > MAX_NOTE {
            return Err(Error::Rejected);
        }
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let actor = Actor::check(&tx, caller, ReportScope::Triage).await?;
        locked(&tx, uuid(report)).await?;
        // The report's row is locked, so its notes are weighed one at a time.
        let held: i64 = tx
            .query_one_raw(statement(
                &format!(
                    "SELECT COALESCE(sum(octet_length(text) + {NOTE_BYTES}), 0)::bigint AS bytes
                     FROM central_identity.report_notes WHERE report_id = $1"
                ),
                vec![uuid(report).into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)
            .and_then(|row| get(&row, "bytes"))?;
        let held = usize::try_from(held).map_err(|_| Error::Unavailable)?;
        if held.saturating_add(text.len() + NOTE_BYTES) > MAX_NOTES_BYTES {
            return Err(Error::Busy);
        }
        tx.execute_raw(statement(
            "INSERT INTO central_identity.report_notes (report_id, author, token_name, text)
             VALUES ($1, $2, $3, $4)",
            vec![
                uuid(report).into(),
                actor.id.into(),
                actor.token_name.clone().into(),
                text.into(),
            ],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        history(&tx, uuid(report), Some(&actor), "note", "added").await?;
        let detail = Self::detail(&tx, &actor, uuid(report)).await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(detail)
    }

    /// A note removed by its author. Answers the report.
    ///
    /// # Errors
    /// `Rejected` for another's note, an unknown one, or neither an
    /// administrator nor a report token that may triage; storage failure.
    pub async fn remove_report_note(
        &self,
        caller: ReportCaller<'_>,
        report: [u8; 16],
        note: [u8; 16],
    ) -> Result<ReportDetail, Error> {
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let actor = Actor::check(&tx, caller, ReportScope::Triage).await?;
        let removed = tx
            .execute_raw(statement(
                "DELETE FROM central_identity.report_notes WHERE id = $1 AND report_id = $2 AND author = $3
                 AND (token_name IS NOT NULL) = $4",
                vec![
                    uuid(note).into(),
                    uuid(report).into(),
                    actor.id.into(),
                    actor.token_name.is_some().into(),
                ],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        if removed.rows_affected() == 0 {
            return Err(Error::Rejected);
        }
        history(&tx, uuid(report), Some(&actor), "note", "removed").await?;
        let detail = Self::detail(&tx, &actor, uuid(report)).await?;
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
        let admin = Self::triager(&tx, browser).await?;
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
        let admin = Self::triager(&tx, browser).await?;
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
        let admin = Self::triager(&tx, browser).await?;
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
        let admin = Self::triager(&tx, browser).await?;
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

    /// The administrators a report may be assigned to, for `caller`, who may
    /// assign one.
    ///
    /// # Errors
    /// `Rejected` without an administrator or a report token that may
    /// triage; storage failure.
    pub async fn report_admins(&self, caller: ReportCaller<'_>) -> Result<Vec<ReportAdmin>, Error> {
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        Actor::check(&tx, caller, ReportScope::Triage).await?;
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
