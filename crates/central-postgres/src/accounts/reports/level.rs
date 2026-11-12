//! What reports may take in the store (`docs/reports.md` §"What the
//! reports may take").
//!
//! A gateway may file 120 reports an hour, each announcing up to 66 MiB of
//! parts, and the hour's count is in the service's memory. So the store
//! itself weighs every filing: against what its gateway's reports take,
//! and against what everyone's take. Past a bound the filing is `Busy`;
//! the report is not filed, so no part of it can follow.
//!
//! No count is kept beside the reports. A level is asked of the reports
//! themselves in the filing's transaction, so it cannot drift, a restart
//! changes nothing, and whatever deletes a report (a purge, the sweep)
//! frees its room without knowing of this file. A report counts as
//! [`REPORT_BYTES`], its body's bound, whatever its length. A part counts
//! as its announced length while it is stored or may still come (its
//! report is younger than [`PART_SECONDS`]); stored bytes are never read
//! for it, since a part is stored only at its announced length. A part
//! that never came stops counting when it can no longer be stored.
//!
//! One filing is weighed at a time, under a transaction's advisory lock
//! that is tried and never waited for: two filings cannot both take the
//! last room, and none holds one of the store's connections waiting for
//! another. The one that finds the lock taken is `Busy`, which a gateway
//! sends again. The operator's one-time copy (`import.rs`) is not weighed
//! and is counted afterwards like any report.
use super::super::{Error, statement, uuid};
use super::Filer;
use central_api::reports::{MAX_INTAKE_BYTES, PART_SECONDS, ReportIntake};
use sea_orm::ConnectionTrait;

/// What one report counts as besides its parts: its body's bound, so the
/// level is never under what the report's row takes.
pub const REPORT_BYTES: u64 = 65_536;
const _: () = assert!(MAX_INTAKE_BYTES == 65_536);

/// What one report takes at the most: its body's bound, 2 MiB of `client`
/// and 64 MiB of record.
const LARGEST: u64 = REPORT_BYTES + 2 * 1024 * 1024 + 64 * 1024 * 1024;

/// What a registered gateway's reports may take: 2 GiB. That is 31 reports
/// with both parts at their largest, about 2,000 with a megabyte of parts
/// each, or 32,768 without parts; a gateway that announces all it may
/// reaches it in a quarter of an hour and files nothing more until an
/// administrator purges its reports or the sweep takes them.
pub const GATEWAY_STORED: u64 = 2 * 1024 * 1024 * 1024;

/// What the official gateway's reports may take: 8 GiB, four times a
/// registered gateway's, since most players play through it. What the
/// one-time copy brought is counted in it.
pub const OFFICIAL_STORED: u64 = 4 * GATEWAY_STORED;

/// What all reports together may take: 16 GiB. The official gateway at its
/// bound leaves four registered gateways theirs.
pub const ALL_STORED: u64 = 8 * GATEWAY_STORED;

const _: () = assert!(
    30 * LARGEST <= GATEWAY_STORED
        && OFFICIAL_STORED + 4 * GATEWAY_STORED <= ALL_STORED
        && ALL_STORED < 1 << 40
);

/// The advisory lock's two keys. The first is the one the migrations use
/// with `1` (`central-migration`); `2` is the weighing of a report.
const LOCK: (i32, i32) = (1_862_384_219, 2);

/// What the reports of `gateway` count as, and everyone's, in `db`'s
/// transaction.
async fn levels(db: &impl ConnectionTrait, gateway: [u8; 16]) -> Result<(u64, u64), Error> {
    let row = db
        .query_one_raw(statement(
            &format!(
                "SELECT COALESCE(sum(w.bytes) FILTER (WHERE w.gateway_id = $1), 0)::bigint AS theirs,
                        COALESCE(sum(w.bytes), 0)::bigint AS everyone
                 FROM (SELECT r.gateway_id, {REPORT_BYTES}::bigint
                           + CASE WHEN COALESCE(p.client, false) OR r.created_at
                                 > pg_catalog.clock_timestamp() - interval '{PART_SECONDS} seconds'
                             THEN COALESCE(r.client_bytes, 0) ELSE 0 END
                           + CASE WHEN COALESCE(p.record, false) OR r.created_at
                                 > pg_catalog.clock_timestamp() - interval '{PART_SECONDS} seconds'
                             THEN COALESCE(r.record_bytes, 0) ELSE 0 END AS bytes
                       FROM central_identity.reports r
                       LEFT JOIN (SELECT s.report_id, bool_or(s.part = 'client') AS client,
                                         bool_or(s.part = 'record') AS record
                                  FROM central_identity.report_parts s GROUP BY s.report_id) p
                         ON p.report_id = r.id) w"
            ),
            vec![uuid(gateway).into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::Unavailable)?;
    let level = |column| {
        row.try_get::<i64>("", column)
            .ok()
            .and_then(|bytes| u64::try_from(bytes).ok())
            .ok_or(Error::Unavailable)
    };
    Ok((level("theirs")?, level("everyone")?))
}

/// Admit `report` for `filer` in `db`'s transaction, before it is written:
/// alone (the advisory lock), and only while its gateway's level and
/// everyone's stay within their bounds with it.
///
/// # Errors
/// `Busy` past a bound, or while another filing is weighed; storage failure.
pub(super) async fn admit(
    db: &impl ConnectionTrait,
    filer: &Filer,
    report: &ReportIntake,
) -> Result<(), Error> {
    let alone = db
        .query_one_raw(statement(
            &format!(
                "SELECT pg_catalog.pg_try_advisory_xact_lock({}, {}) AS alone",
                LOCK.0, LOCK.1
            ),
            vec![],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::Unavailable)?
        .try_get::<bool>("", "alone")
        .map_err(|_| Error::Unavailable)?;
    if !alone {
        return Err(Error::Busy);
    }
    let (theirs, everyone) = levels(db, filer.gateway).await?;
    let new = REPORT_BYTES
        + report
            .client
            .as_ref()
            .map_or(0, |part| u64::from(part.bytes))
        + report
            .record
            .as_ref()
            .map_or(0, |part| u64::from(part.bytes));
    let bound = if filer.official {
        OFFICIAL_STORED
    } else {
        GATEWAY_STORED
    };
    if theirs.saturating_add(new) > bound || everyone.saturating_add(new) > ALL_STORED {
        return Err(Error::Busy);
    }
    Ok(())
}

impl super::AccountStore {
    /// What the reports of `gateway` count as towards its bound, and what
    /// everyone's count as towards [`ALL_STORED`]: the measure a filing is
    /// weighed by.
    ///
    /// # Errors
    /// Storage failure.
    pub async fn report_levels(&self, gateway: [u8; 16]) -> Result<(u64, u64), Error> {
        levels(&self.db, gateway).await
    }
}
