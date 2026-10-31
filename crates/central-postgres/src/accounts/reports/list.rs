//! What an administrator lists: pages, counts per dimension, crash groups,
//! the month's numbers and a report's neighbours. A filter becomes fixed SQL
//! with numbered values (`Where`); no text of a filter is ever SQL.
use super::super::{AccountStore, Error, Token, statement};
use super::sweep;
use central_api::reports::{
    Channel, Count, CrashGroup, DayCount, EXCERPT, FACET_REPORTERS, MAX_CRASH_GROUPS, MAX_PAGE,
    Neighbours, Priority, RecordOrigin, ReportFacets, ReportFilter, ReportKind, ReportSort,
    ReportStatus, ReportSummary, STATS_DAYS,
};
use sea_orm::{
    ConnectionTrait, DatabaseTransaction, QueryResult, TransactionTrait, Value, prelude::Uuid,
};

/// The columns of a [`ReportSummary`] from `reports r`; `$1` is the reading administrator.
pub(super) const SUMMARY_COLUMNS: &str = "r.id,
    floor(extract(epoch FROM r.created_at))::bigint AS created_at,
    floor(extract(epoch FROM r.updated_at))::bigint AS updated_at,
    r.gateway_label, r.gateway_says, r.gateway_version, r.channel, r.reporter, r.kind, r.status,
    left(r.text, 300) AS excerpt, r.build_version, r.crash_message,
    r.record_bytes, r.record_complete, r.record_origin, r.issue_number, r.assignee, r.priority,
    pg_catalog.array_to_string(r.labels, ',') AS labels, r.duplicate_of, r.signature, r.revision,
    floor(extract(epoch FROM r.done_at))::bigint AS done_at,
    floor(extract(epoch FROM r.deleted_at))::bigint AS deleted_at,
    EXISTS (SELECT 1 FROM central_identity.report_seen s WHERE s.admin = $1 AND s.report_id = r.id) AS seen,
    (SELECT count(*) FROM central_identity.report_notes n WHERE n.report_id = r.id)::bigint AS notes";

const _: () = assert!(EXCERPT == 300, "SUMMARY_COLUMNS cuts the excerpt at 300");

fn get<T: sea_orm::TryGetable>(row: &QueryResult, column: &str) -> Result<T, Error> {
    row.try_get("", column).map_err(|_| Error::Unavailable)
}

fn id(row: &QueryResult, column: &str) -> Result<[u8; 16], Error> {
    get::<Uuid>(row, column).map(|id| *id.as_bytes())
}

fn optional_id(row: &QueryResult, column: &str) -> Result<Option<[u8; 16]>, Error> {
    get::<Option<Uuid>>(row, column).map(|id| id.map(|id| *id.as_bytes()))
}

/// Labels as stored, comma-joined (a label holds no comma).
pub(super) fn split(labels: &str) -> Vec<String> {
    labels
        .split(',')
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect()
}

fn small(row: &QueryResult, column: &str) -> Result<Option<u32>, Error> {
    get::<Option<i32>>(row, column)?
        .map(|n| u32::try_from(n).map_err(|_| Error::Unavailable))
        .transpose()
}

pub(super) fn summary(row: &QueryResult) -> Result<ReportSummary, Error> {
    let record_origin: Option<String> = get(row, "record_origin")?;
    let priority: Option<String> = get(row, "priority")?;
    Ok(ReportSummary {
        id: id(row, "id")?,
        created_at: get(row, "created_at")?,
        updated_at: get(row, "updated_at")?,
        gateway: get(row, "gateway_label")?,
        gateway_says: get(row, "gateway_says")?,
        gateway_version: get(row, "gateway_version")?,
        channel: Channel::from_stored(&get::<String>(row, "channel")?)?,
        reporter: get(row, "reporter")?,
        kind: ReportKind::from_stored(&get::<String>(row, "kind")?)?,
        status: ReportStatus::from_stored(&get::<String>(row, "status")?)?,
        excerpt: get(row, "excerpt")?,
        build: get(row, "build_version")?,
        crash: get(row, "crash_message")?,
        has_record: record_origin.is_some(),
        record_complete: get::<Option<bool>>(row, "record_complete")?.unwrap_or(false),
        record_origin: record_origin
            .as_deref()
            .map(RecordOrigin::from_stored)
            .transpose()?,
        record_bytes: small(row, "record_bytes")?,
        issue: small(row, "issue_number")?,
        assignee: optional_id(row, "assignee")?,
        priority: priority.as_deref().map(Priority::from_stored).transpose()?,
        labels: split(&get::<String>(row, "labels")?),
        duplicate_of: optional_id(row, "duplicate_of")?,
        signature: get(row, "signature")?,
        revision: u32::try_from(get::<i32>(row, "revision")?).map_err(|_| Error::Unavailable)?,
        done_at: get(row, "done_at")?,
        deleted_at: get(row, "deleted_at")?,
        seen: get(row, "seen")?,
        notes: u32::try_from(get::<i64>(row, "notes")?).map_err(|_| Error::Unavailable)?,
    })
}

/// One facet's rows: `column`'s values and their counts under `filter`
/// but `skip`'s dimension, the most frequent first.
async fn facet(
    tx: &DatabaseTransaction,
    admin: Uuid,
    filter: &ReportFilter,
    column: &str,
    skip: Dimension,
    limit: Option<usize>,
) -> Result<Vec<QueryResult>, Error> {
    let at = Where::new(admin).filter(filter, Some(skip));
    let limit = limit.map_or_else(String::new, |n| format!(" LIMIT {n}"));
    let sql = format!(
        "SELECT {column} AS value, count(*)::bigint AS n FROM central_identity.reports r {}
         GROUP BY 1 ORDER BY 2 DESC, 1{limit}",
        at.clause()
    );
    tx.query_all_raw(statement(&sql, at.into_values()))
        .await
        .map_err(|_| Error::Unavailable)
}

/// A facet of words, each read by `read`.
fn counted<T>(
    rows: &[QueryResult],
    read: impl Fn(&str) -> Result<T, Error>,
) -> Result<Vec<Count<T>>, Error> {
    rows.iter()
        .map(|row| {
            Ok(Count {
                value: read(&get::<String>(row, "value")?)?,
                count: n(row)?,
            })
        })
        .collect()
}

/// A dimension a facet counts, left out of its own condition.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Dimension {
    Status,
    Kind,
    Priority,
    Assignee,
    Label,
    Gateway,
    Channel,
    Reporter,
}

/// Conditions over `reports r` and their values; `$1` is always the reading administrator.
pub(super) struct Where {
    conditions: Vec<String>,
    values: Vec<Value>,
}

impl Where {
    pub(super) fn new(admin: Uuid) -> Self {
        Self {
            conditions: Vec::new(),
            values: vec![admin.into()],
        }
    }

    /// The next value's placeholder.
    pub(super) fn value(&mut self, value: impl Into<Value>) -> String {
        self.values.push(value.into());
        format!("${}", self.values.len())
    }

    fn when(&mut self, condition: String) {
        self.conditions.push(condition);
    }

    /// `column` is one of the comma-joined `words`.
    fn one_of(&mut self, column: &str, words: String) {
        let n = self.value(words);
        self.when(format!(
            "{column} = ANY(pg_catalog.string_to_array({n}::text, ','))"
        ));
    }

    /// `column` equals `value`.
    fn equals(&mut self, column: &str, value: impl Into<Value>) {
        let n = self.value(value);
        self.when(format!("{column} = {n}"));
    }

    /// Every part of `filter` but `skip`'s dimension.
    fn filter(mut self, filter: &ReportFilter, skip: Option<Dimension>) -> Self {
        self.when(
            if filter.trash {
                "r.deleted_at IS NOT NULL"
            } else {
                "r.deleted_at IS NULL"
            }
            .into(),
        );
        self.faceted(filter, skip);
        self.narrowed(filter);
        self
    }

    /// The dimensions a facet counts, each but `skip`.
    fn faceted(&mut self, filter: &ReportFilter, skip: Option<Dimension>) {
        let keep = |d: Dimension| skip != Some(d);
        if keep(Dimension::Status) && !filter.statuses.is_empty() {
            self.one_of(
                "r.status",
                words(filter.statuses.iter().map(|s| s.as_str())),
            );
        }
        if keep(Dimension::Kind) && !filter.kinds.is_empty() {
            self.one_of("r.kind", words(filter.kinds.iter().map(|k| k.as_str())));
        }
        if keep(Dimension::Priority) && !filter.priorities.is_empty() {
            let none = filter.priorities.contains(&None);
            let n = self.value(words(
                filter.priorities.iter().flatten().map(|p| p.as_str()),
            ));
            self.when(format!(
                "(r.priority = ANY(pg_catalog.string_to_array({n}::text, ',')){})",
                if none { " OR r.priority IS NULL" } else { "" }
            ));
        }
        if keep(Dimension::Channel)
            && let Some(channel) = filter.channel
        {
            self.equals("r.channel", channel.as_str());
        }
        if keep(Dimension::Gateway)
            && let Some(gateway) = &filter.gateway
        {
            self.equals("r.gateway_label", gateway.clone());
        }
        if keep(Dimension::Assignee) {
            if filter.unassigned {
                self.when("r.assignee IS NULL".into());
            } else if let Some(assignee) = filter.assignee {
                self.equals("r.assignee", Uuid::from_bytes(assignee));
            }
        }
        if keep(Dimension::Label)
            && let Some(label) = &filter.label
        {
            let n = self.value(label.clone());
            self.when(format!("{n} = ANY(r.labels)"));
        }
        if keep(Dimension::Reporter)
            && let Some(reporter) = &filter.reporter
        {
            self.equals("r.reporter", reporter.clone());
        }
    }

    /// The parts of a filter no facet counts.
    fn narrowed(&mut self, filter: &ReportFilter) {
        if let Some(q) = &filter.q {
            let n = self.value(q.trim().to_lowercase());
            self.when(format!(
                "(strpos(lower(r.text), {n}) > 0 OR strpos(lower(COALESCE(r.crash_message, '')), {n}) > 0
                  OR strpos(r.id::text, {n}) > 0)"
            ));
        }
        if let Some(from) = &filter.from {
            let n = self.value(from.clone());
            self.when(format!(
                "r.created_at >= ({n}::date::timestamp AT TIME ZONE 'UTC')"
            ));
        }
        if let Some(to) = &filter.to {
            let n = self.value(to.clone());
            self.when(format!(
                "r.created_at < (({n}::date + 1)::timestamp AT TIME ZONE 'UTC')"
            ));
        }
        if let Some(has) = filter.has_record {
            self.when(
                if has {
                    "r.record_origin IS NOT NULL"
                } else {
                    "r.record_origin IS NULL"
                }
                .into(),
            );
        }
        if let Some(signature) = &filter.signature {
            self.equals("r.signature", signature.clone());
        }
        if let Some(original) = filter.duplicate_of {
            self.equals("r.duplicate_of", Uuid::from_bytes(original));
        }
        if filter.unseen {
            self.when(
                "NOT EXISTS (SELECT 1 FROM central_identity.report_seen s WHERE s.admin = $1 AND s.report_id = r.id)".into(),
            );
        }
    }

    /// `WHERE …`, never empty: the trash's condition is always there.
    pub(super) fn clause(&self) -> String {
        format!("WHERE {}", self.conditions.join(" AND "))
    }

    pub(super) fn into_values(self) -> Vec<Value> {
        self.values
    }
}

fn words<'a>(words: impl Iterator<Item = &'a str>) -> String {
    words.collect::<Vec<_>>().join(",")
}

/// The `ORDER BY` of a list: the sort's key, then the id, both in its direction.
pub(super) fn order(sort: ReportSort, descending: bool) -> String {
    let key = match sort {
        ReportSort::Received => "r.id",
        ReportSort::Changed => "r.updated_at",
        ReportSort::Priority => {
            "CASE r.priority WHEN 'critical' THEN 0 WHEN 'high' THEN 1 WHEN 'normal' THEN 2 WHEN 'low' THEN 3 ELSE 4 END"
        }
        ReportSort::Status => {
            "CASE r.status WHEN 'new' THEN 0 WHEN 'triaged' THEN 1 WHEN 'in_progress' THEN 2 WHEN 'resolved' THEN 3 WHEN 'wont_fix' THEN 4 ELSE 5 END"
        }
        ReportSort::Kind => "r.kind",
        ReportSort::Reporter => "r.reporter",
        ReportSort::Gateway => "r.gateway_label",
    };
    let direction = if descending { "DESC" } else { "ASC" };
    format!("ORDER BY {key} {direction}, r.id {direction}")
}

/// The administrator behind `browser`, and the sweep before anything is read, in `tx`.
async fn reader(tx: &sea_orm::DatabaseTransaction, browser: &Token) -> Result<Uuid, Error> {
    let (admin, _) = AccountStore::administrator(tx, browser).await?;
    sweep(tx).await?;
    Ok(admin)
}

impl AccountStore {
    /// A page of reports for the administrator behind `browser`, after the
    /// sweep, and how many the filter matches.
    ///
    /// # Errors
    /// `Rejected` without an administrator's live browser session or for a
    /// filter or page out of bounds; storage failure.
    pub async fn reports(
        &self,
        browser: &Token,
        filter: &ReportFilter,
        sort: ReportSort,
        descending: bool,
        offset: u32,
        limit: u32,
    ) -> Result<(Vec<ReportSummary>, u64), Error> {
        filter.validate()?;
        if limit == 0 || limit > MAX_PAGE || offset > 1_000_000 {
            return Err(Error::Rejected);
        }
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let admin = reader(&tx, browser).await?;
        let mut at = Where::new(admin).filter(filter, None);
        let (l, o) = (at.value(i64::from(limit)), at.value(i64::from(offset)));
        let sql = format!(
            "SELECT {SUMMARY_COLUMNS} FROM central_identity.reports r {} {} LIMIT {l} OFFSET {o}",
            at.clause(),
            order(sort, descending),
        );
        let rows = tx
            .query_all_raw(statement(&sql, at.into_values()))
            .await
            .map_err(|_| Error::Unavailable)?;
        let page = rows.iter().map(summary).collect::<Result<Vec<_>, _>>()?;
        let total = matching(&tx, admin, filter).await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok((page, total))
    }

    /// Counts per dimension, each under the filter but its own.
    ///
    /// # Errors
    /// As [`Self::reports`].
    pub async fn report_facets(
        &self,
        browser: &Token,
        filter: &ReportFilter,
    ) -> Result<ReportFacets, Error> {
        filter.validate()?;
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let admin = reader(&tx, browser).await?;
        let facet = |column, skip, limit| facet(&tx, admin, filter, column, skip, limit);
        let words = |s: &str| Ok(s.to_owned());
        let statuses = counted(
            &facet("r.status", Dimension::Status, None).await?,
            ReportStatus::from_stored,
        )?;
        let kinds = counted(
            &facet("r.kind", Dimension::Kind, None).await?,
            ReportKind::from_stored,
        )?;
        let channels = counted(
            &facet("r.channel", Dimension::Channel, None).await?,
            Channel::from_stored,
        )?;
        let priorities = facet("r.priority", Dimension::Priority, None)
            .await?
            .iter()
            .map(|row| {
                let value: Option<String> = get(row, "value")?;
                Ok(Count {
                    value: value.as_deref().map(Priority::from_stored).transpose()?,
                    count: n(row)?,
                })
            })
            .collect::<Result<_, Error>>()?;
        let assignees = facet("r.assignee", Dimension::Assignee, None)
            .await?
            .iter()
            .map(|row| {
                Ok(Count {
                    value: optional_id(row, "value")?,
                    count: n(row)?,
                })
            })
            .collect::<Result<_, Error>>()?;
        let labels = counted(
            &facet("pg_catalog.unnest(r.labels)", Dimension::Label, None).await?,
            words,
        )?;
        let gateways = counted(
            &facet("r.gateway_label", Dimension::Gateway, None).await?,
            words,
        )?;
        let reporters = counted(
            &facet("r.reporter", Dimension::Reporter, Some(FACET_REPORTERS)).await?,
            words,
        )?;
        let total = matching(&tx, admin, filter).await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(ReportFacets {
            statuses,
            kinds,
            priorities,
            assignees,
            labels,
            gateways,
            channels,
            reporters,
            total,
        })
    }

    /// The crashes the filter matches, grouped by signature, the latest group first.
    ///
    /// # Errors
    /// As [`Self::reports`].
    pub async fn crash_groups(
        &self,
        browser: &Token,
        filter: &ReportFilter,
    ) -> Result<Vec<CrashGroup>, Error> {
        filter.validate()?;
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let admin = reader(&tx, browser).await?;
        let at = Where::new(admin).filter(filter, None);
        let rows = tx
            .query_all_raw(statement(
                &format!(
                    "SELECT r.signature, count(*)::bigint AS n, count(*) FILTER (WHERE r.done_at IS NULL)::bigint AS open,
                         floor(extract(epoch FROM min(r.created_at)))::bigint AS first,
                         floor(extract(epoch FROM max(r.created_at)))::bigint AS last,
                         (array_agg(r.id ORDER BY r.id DESC))[1] AS latest,
                         (array_agg(COALESCE(r.crash_message, '') ORDER BY r.id DESC))[1] AS message,
                         (array_agg(COALESCE(r.crash_location, '') ORDER BY r.id DESC))[1] AS location
                     FROM central_identity.reports r {} AND r.signature IS NOT NULL
                     GROUP BY r.signature ORDER BY max(r.created_at) DESC LIMIT {MAX_CRASH_GROUPS}",
                    at.clause()
                ),
                at.into_values(),
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        let groups = rows
            .iter()
            .map(|row| {
                Ok(CrashGroup {
                    signature: get(row, "signature")?,
                    count: n(row)?,
                    open: u64::try_from(get::<i64>(row, "open")?)
                        .map_err(|_| Error::Unavailable)?,
                    first: get(row, "first")?,
                    last: get(row, "last")?,
                    latest: id(row, "latest")?,
                    message: get(row, "message")?,
                    location: get(row, "location")?,
                })
            })
            .collect::<Result<_, Error>>()?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(groups)
    }

    /// Reports per UTC day and kind over [`STATS_DAYS`] days, out of the trash: counts only.
    ///
    /// # Errors
    /// `Rejected` without an administrator's live browser session; storage failure.
    pub async fn report_stats(&self, browser: &Token) -> Result<Vec<DayCount>, Error> {
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        reader(&tx, browser).await?;
        let rows = tx
            .query_all_raw(statement(
                &format!(
                    "SELECT to_char(r.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD') AS day, r.kind, count(*)::bigint AS n
                     FROM central_identity.reports r
                     WHERE r.deleted_at IS NULL
                       AND r.created_at > pg_catalog.clock_timestamp() - interval '{STATS_DAYS} days'
                     GROUP BY 1, 2 ORDER BY 1, 2"
                ),
                vec![],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        let days = rows
            .iter()
            .map(|row| {
                Ok(DayCount {
                    day: get(row, "day")?,
                    kind: ReportKind::from_stored(&get::<String>(row, "kind")?)?,
                    count: n(row)?,
                })
            })
            .collect::<Result<_, Error>>()?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(days)
    }

    /// `report`'s place in the list the filter and order make.
    ///
    /// # Errors
    /// As [`Self::reports`].
    pub async fn report_neighbours(
        &self,
        browser: &Token,
        report: [u8; 16],
        filter: &ReportFilter,
        sort: ReportSort,
        descending: bool,
    ) -> Result<Neighbours, Error> {
        filter.validate()?;
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let admin = reader(&tx, browser).await?;
        let mut at = Where::new(admin).filter(filter, None);
        let wanted = at.value(Uuid::from_bytes(report));
        let order = order(sort, descending);
        let row = tx
            .query_one_raw(statement(
                &format!(
                    "WITH list AS (SELECT r.id, lag(r.id) OVER w AS previous, lead(r.id) OVER w AS next,
                         row_number() OVER w AS position, count(*) OVER () AS total
                       FROM central_identity.reports r {} WINDOW w AS ({order}))
                     SELECT previous, next, position, total FROM list WHERE id = {wanted}",
                    at.clause()
                ),
                at.into_values(),
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        let neighbours = if let Some(row) = row {
            Neighbours {
                previous: optional_id(&row, "previous")?,
                next: optional_id(&row, "next")?,
                position: Some(
                    u64::try_from(get::<i64>(&row, "position")?).map_err(|_| Error::Unavailable)?,
                ),
                total: u64::try_from(get::<i64>(&row, "total")?).map_err(|_| Error::Unavailable)?,
            }
        } else {
            // not in the list: no place, and the list's length still
            Neighbours {
                previous: None,
                next: None,
                position: None,
                total: matching(&tx, admin, filter).await?,
            }
        };
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(neighbours)
    }
}

fn n(row: &QueryResult) -> Result<u64, Error> {
    u64::try_from(get::<i64>(row, "n")?).map_err(|_| Error::Unavailable)
}

/// How many reports `filter` matches.
async fn matching(
    tx: &DatabaseTransaction,
    admin: Uuid,
    filter: &ReportFilter,
) -> Result<u64, Error> {
    let at = Where::new(admin).filter(filter, None);
    let sql = format!(
        "SELECT count(*) AS n FROM central_identity.reports r {}",
        at.clause()
    );
    count(tx, &sql, at.into_values()).await
}

async fn count(db: &impl ConnectionTrait, sql: &str, values: Vec<Value>) -> Result<u64, Error> {
    let row = db
        .query_one_raw(statement(sql, values))
        .await
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::Unavailable)?;
    n(&row)
}
