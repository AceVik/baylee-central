//! Bug and crash reports (migration 000018, `docs/reports.md`): a gateway
//! files a report's metadata, then its gzip parts, which are stored as they
//! came once their length and SHA-256 equal the announcement and never
//! unpacked here; an administrator lists, reads and triages them
//! (`list.rs`, `triage.rs`). Before a list is read the sweep deletes what
//! the retention says is over. No column joins a report to an account: the
//! reporter is the gateway's pseudonym or a device's HMAC.
use super::tokens::LIVE_TOKEN;
use super::{AccountStore, Error, Token, statement, uuid};
use central_api::reports::{
    Channel, DONE_DAYS, KEEP_DAYS, PART_SECONDS, Part, RecordOrigin, ReportIntake, TRASH_DAYS,
};
use sea_orm::{ConnectionTrait, QueryResult, TransactionTrait, prelude::Uuid};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

mod list;
mod triage;

/// Who files a report: the authenticated gateway and its name in Central then.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Filer {
    /// The gateway's id.
    pub gateway: [u8; 16],
    /// Its name in Central.
    pub label: String,
    /// It is the official gateway, whose caller the bridge credential proved.
    pub official: bool,
}

fn filer(row: &QueryResult, official: bool) -> Result<Filer, Error> {
    let id: Uuid = row.try_get("", "id").map_err(|_| Error::Unavailable)?;
    Ok(Filer {
        gateway: *id.as_bytes(),
        label: row.try_get("", "name").map_err(|_| Error::Unavailable)?,
        official,
    })
}

/// One line of a report's history, in `db`'s transaction: who (none for
/// the gateway that filed it or the sweep), what, the change in short.
pub(super) async fn history(
    db: &impl ConnectionTrait,
    report: Uuid,
    actor: Option<Uuid>,
    action: &str,
    detail: &str,
) -> Result<(), Error> {
    db.execute_raw(statement(
        "INSERT INTO central_identity.report_audit (report_id, actor, action, detail) VALUES ($1, $2, $3, $4)",
        vec![report.into(), actor.into(), format!("report.{action}").into(), detail.into()],
    ))
    .await
    .map_err(|_| Error::Unavailable)?;
    Ok(())
}

/// Deletes, in `db`'s transaction, every report done for [`DONE_DAYS`], in
/// the trash for [`TRASH_DAYS`] or older than [`KEEP_DAYS`], with its parts,
/// notes and opened marks; its history stays, with a line saying so.
/// Returns how many went.
pub(super) async fn sweep(db: &impl ConnectionTrait) -> Result<u64, Error> {
    let gone = db
        .query_all_raw(statement(
            &format!(
                "DELETE FROM central_identity.reports
                 WHERE done_at < pg_catalog.clock_timestamp() - interval '{DONE_DAYS} days'
                    OR deleted_at < pg_catalog.clock_timestamp() - interval '{TRASH_DAYS} days'
                    OR created_at < pg_catalog.clock_timestamp() - interval '{KEEP_DAYS} days'
                 RETURNING id"
            ),
            vec![],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
    for row in &gone {
        let id: Uuid = row.try_get("", "id").map_err(|_| Error::Unavailable)?;
        history(db, id, None, "swept", "").await?;
    }
    Ok(gone.len() as u64)
}

impl AccountStore {
    /// The approved registered gateway behind `credential`: its credential
    /// pair, or a live token with the `sso` scope. A pending, disabled or
    /// official gateway files nothing this way (the official one files
    /// through the bridge, [`Self::official_filer`]).
    ///
    /// # Errors
    /// `Rejected` for any other credential; storage failure.
    pub async fn report_filer(&self, credential: &Token) -> Result<Filer, Error> {
        let row = self
            .db
            .query_one_raw(statement(
                &format!(
                    "SELECT g.id, g.name FROM central_identity.gateways g
                     JOIN central_identity.gateway_approvals p ON p.gateway_id = g.id AND p.enabled
                     WHERE g.credential_digest = $1 AND NOT g.official
                     UNION ALL
                     SELECT g.id, g.name FROM central_identity.gateway_tokens t
                     JOIN central_identity.gateways g ON g.id = t.gateway_id
                     JOIN central_identity.gateway_approvals p ON p.gateway_id = g.id AND p.enabled
                     WHERE t.digest = $1 AND t.sso AND {LIVE_TOKEN} AND NOT g.official
                     LIMIT 1"
                ),
                vec![credential.digest().to_vec().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        filer(&row, false)
    }

    /// The official gateway as a report's filer; its caller proved the bridge credential.
    ///
    /// # Errors
    /// `Unavailable` without an official row; storage failure.
    pub async fn official_filer(&self) -> Result<Filer, Error> {
        let row = self
            .db
            .query_one_raw(statement(
                "SELECT g.id, g.name FROM central_identity.gateways g WHERE g.official LIMIT 1",
                vec![],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)?;
        filer(&row, true)
    }

    /// File `report` for `filer`, its parts announced. `direct` only from the
    /// official gateway. Returns the report's id.
    ///
    /// # Errors
    /// `Rejected` for a report out of its bounds or a direct one from
    /// another gateway; storage failure.
    pub async fn file_report(
        &self,
        filer: &Filer,
        report: &ReportIntake,
    ) -> Result<[u8; 16], Error> {
        report.validate()?;
        if report.channel == Channel::Direct && !filer.official {
            return Err(Error::Rejected);
        }
        let record = report.record.as_ref();
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let row = tx
            .query_one_raw(statement(
                "INSERT INTO central_identity.reports (gateway_id, gateway_label, gateway_says, gateway_url,
                     gateway_version, channel, reporter, kind, text, game_id, build_version, build_commit,
                     crash_message, crash_location, signature, client_bytes, client_sha256, record_bytes,
                     record_sha256, record_complete, record_origin)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21)
                 RETURNING id",
                vec![
                    uuid(filer.gateway).into(),
                    filer.label.clone().into(),
                    report.gateway.name.clone().into(),
                    report.gateway.url.clone().into(),
                    report.gateway.version.clone().into(),
                    report.channel.as_str().into(),
                    report.reporter.clone().into(),
                    report.kind.as_str().into(),
                    report.text.clone().into(),
                    report.game_id.clone().into(),
                    report.build.as_ref().map(|b| b.version.clone()).into(),
                    report.build.as_ref().and_then(|b| b.commit.clone()).into(),
                    report.crash.as_ref().map(|c| c.message.clone()).into(),
                    report.crash.as_ref().map(|c| c.location.clone()).into(),
                    report.signature().into(),
                    report.client.as_ref().map(|p| i32::try_from(p.bytes).unwrap_or(i32::MAX)).into(),
                    report.client.as_ref().map(|p| p.sha256.to_vec()).into(),
                    record.map(|r| i32::try_from(r.bytes).unwrap_or(i32::MAX)).into(),
                    record.map(|r| r.sha256.to_vec()).into(),
                    record.map(|r| r.complete).into(),
                    record.map(|r| r.origin.as_str()).into(),
                ],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)?;
        let id: Uuid = row.try_get("", "id").map_err(|_| Error::Unavailable)?;
        history(&tx, id, None, "filed", &format!("by {}", filer.label)).await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(*id.as_bytes())
    }

    /// The announced length of a part `filer` may still upload for `report`:
    /// its own report, filed within [`PART_SECONDS`], the part announced and
    /// not stored yet. Asked before a body is read, so nothing unannounced
    /// is read at all.
    ///
    /// # Errors
    /// `Rejected` for another gateway's report, one too old or a part not
    /// announced; `Conflict` for a part stored already; storage failure.
    pub async fn report_part_expected(
        &self,
        filer: &Filer,
        report: [u8; 16],
        part: Part,
    ) -> Result<u32, Error> {
        let bytes = match part {
            Part::Client => "client_bytes",
            Part::Record => "record_bytes",
        };
        let row = self
            .db
            .query_one_raw(statement(
                &format!(
                    "SELECT r.{bytes} AS bytes, EXISTS (SELECT 1 FROM central_identity.report_parts p
                         WHERE p.report_id = r.id AND p.part = $3) AS stored
                     FROM central_identity.reports r
                     WHERE r.id = $1 AND r.gateway_id = $2
                       AND r.created_at > pg_catalog.clock_timestamp() - interval '{PART_SECONDS} seconds'"
                ),
                vec![uuid(report).into(), uuid(filer.gateway).into(), part.as_str().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        if row
            .try_get::<bool>("", "stored")
            .map_err(|_| Error::Unavailable)?
        {
            return Err(Error::Conflict);
        }
        let announced: Option<i32> = row.try_get("", "bytes").map_err(|_| Error::Unavailable)?;
        announced
            .and_then(|n| u32::try_from(n).ok())
            .ok_or(Error::Rejected)
    }

    /// Store one announced part of a report `filer` filed within
    /// [`PART_SECONDS`]: once, and only when `gzip` starts as gzip and its
    /// length and SHA-256 equal the announcement (compared in constant time).
    ///
    /// # Errors
    /// `Rejected` for another gateway's report, one too old, a part it did
    /// not announce or bytes other than announced; `Conflict` when the part
    /// is stored already; storage failure.
    pub async fn file_report_part(
        &self,
        filer: &Filer,
        report: [u8; 16],
        part: Part,
        gzip: &[u8],
    ) -> Result<(), Error> {
        let (bytes, sha) = match part {
            Part::Client => ("client_bytes", "client_sha256"),
            Part::Record => ("record_bytes", "record_sha256"),
        };
        let row = self
            .db
            .query_one_raw(statement(
                &format!(
                    "SELECT r.{bytes} AS bytes, r.{sha} AS sha256 FROM central_identity.reports r
                     WHERE r.id = $1 AND r.gateway_id = $2
                       AND r.created_at > pg_catalog.clock_timestamp() - interval '{PART_SECONDS} seconds'"
                ),
                vec![uuid(report).into(), uuid(filer.gateway).into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        let announced: Option<i32> = row.try_get("", "bytes").map_err(|_| Error::Unavailable)?;
        let digest: Option<Vec<u8>> = row.try_get("", "sha256").map_err(|_| Error::Unavailable)?;
        let (Some(announced), Some(digest)) = (announced, digest) else {
            return Err(Error::Rejected);
        };
        let same = usize::try_from(announced).is_ok_and(|n| n == gzip.len())
            && gzip.starts_with(&[0x1f, 0x8b])
            && bool::from(Sha256::digest(gzip).as_slice().ct_eq(&digest));
        if !same {
            return Err(Error::Rejected);
        }
        let stored = self
            .db
            .execute_raw(statement(
                "INSERT INTO central_identity.report_parts (report_id, part, gzip) VALUES ($1, $2, $3)
                 ON CONFLICT DO NOTHING",
                vec![uuid(report).into(), part.as_str().into(), gzip.to_vec().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        if stored.rows_affected() == 0 {
            return Err(Error::Conflict);
        }
        Ok(())
    }

    /// One part of a report as stored, for the administrator behind
    /// `browser`: the gzip and, for a record, who wrote it.
    ///
    /// # Errors
    /// `Rejected` without an administrator's live browser session, for an
    /// unknown report or a part that has not arrived; storage failure.
    pub async fn report_part(
        &self,
        browser: &Token,
        report: [u8; 16],
        part: Part,
    ) -> Result<(Vec<u8>, Option<RecordOrigin>), Error> {
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        Self::administrator(&tx, browser).await?;
        let row = tx
            .query_one_raw(statement(
                "SELECT p.gzip, r.record_origin FROM central_identity.report_parts p
                 JOIN central_identity.reports r ON r.id = p.report_id
                 WHERE p.report_id = $1 AND p.part = $2",
                vec![uuid(report).into(), part.as_str().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        let gzip: Vec<u8> = row.try_get("", "gzip").map_err(|_| Error::Unavailable)?;
        let origin: Option<String> = row
            .try_get("", "record_origin")
            .map_err(|_| Error::Unavailable)?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        let origin = match part {
            Part::Record => origin
                .as_deref()
                .map(RecordOrigin::from_stored)
                .transpose()?,
            Part::Client => None,
        };
        Ok((gzip, origin))
    }

    /// The operator's daily sweep of reports (and the one before each list).
    ///
    /// # Errors
    /// Storage failure.
    pub async fn sweep_reports(&self) -> Result<u64, Error> {
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let gone = sweep(&tx).await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(gone)
    }
}
