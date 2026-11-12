//! Profile pictures (`docs/profile-pictures.md`, migration 000027): an
//! account's picture, its upload budget, the public read by key, picture
//! reports, and the administrators' moderation. Every statement names its
//! columns; no statement reads a picture's bytes but the public read and the
//! owner's own.
use super::admin::{Action, id};
use super::{AccountStore, Error, Session, Token, statement};
use central_api::picture::{
    AdminPictureReport, BlockChoice, FiledReport, MAX_BLOCK_DAYS, MAX_NOTICE_NOTE,
    MAX_PICTURE_TAGS, MAX_REPORT_NOTE, Named, Picture, PictureDecision, PictureDigest, PictureKey,
    PictureNotice, PictureReason, PictureReportState, REPORT_PAGE, TaggedPicture, UploadBlock,
    valid_note,
};
use sea_orm::{ConnectionTrait, DatabaseTransaction, QueryResult, TransactionTrait, prelude::Uuid};

/// Uploads an account may make in an hour, and in a day.
pub const UPLOADS_PER_HOUR: i16 = 10;
/// See [`UPLOADS_PER_HOUR`].
pub const UPLOADS_PER_DAY: i16 = 30;
/// Reports one account may file in an hour, and have open at once.
pub const REPORTS_PER_HOUR: i64 = 20;
/// See [`REPORTS_PER_HOUR`].
pub const REPORTS_OPEN: i64 = 100;
/// How long a decided report is kept before the operator's sweep deletes
/// it (owner, 12.11.2026).
pub const DECIDED_REPORT_DAYS: i32 = 365;

/// Which of a picture's files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    /// The still's AVIF.
    Avif,
    /// The still's PNG master and fallback.
    Png,
    /// The AVIF image sequence of a picture that moves.
    Sequence,
}

/// A converted picture's files and digest, as the service checked them.
#[derive(Clone, Copy, Debug)]
pub struct PictureFiles<'a> {
    /// The still's AVIF.
    pub avif: &'a [u8],
    /// The still's PNG master.
    pub png: &'a [u8],
    /// The image sequence, when the picture moves.
    pub sequence: Option<&'a [u8]>,
    /// The digest of the master and the sequence.
    pub sha256: PictureDigest,
}

/// What an upload may do, read before its body is: the account, a block in
/// force, and whether its budget has room.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Admission {
    /// The session's account.
    pub account: [u8; 16],
    /// An administrator's block in force, if any.
    pub block: Option<UploadBlock>,
    /// The hour's and the day's budget both have room.
    pub room: bool,
}

/// An account's own picture, notice and block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnPicture {
    /// The current picture.
    pub picture: Option<Picture>,
    /// The last removal's notice.
    pub notice: Option<PictureNotice>,
    /// A block in force.
    pub block: Option<UploadBlock>,
}

/// One administrator's view of an account's picture case.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PictureCase {
    /// The account.
    pub account: Named,
    /// Its own picture, notice and block.
    pub own: OwnPicture,
    /// The reports about its pictures, newest first.
    pub reports: Vec<AdminPictureReport>,
}

fn unavailable(_: impl Sized) -> Error {
    Error::Unavailable
}

/// The account behind a live browser session, whatever its profile.
async fn browser_account(
    db: &impl ConnectionTrait,
    cookie: &Token,
    lock: bool,
) -> Result<Uuid, Error> {
    let session = Session::Browser(cookie);
    db.query_one_raw(statement(
        &format!(
            "SELECT a.id FROM central_identity.accounts a JOIN central_identity.sessions s
             ON s.account_id = a.id AND s.credential_revision = a.credential_revision
             WHERE s.digest = $1 AND s.kind = 'browser' AND s.expires_at > pg_catalog.clock_timestamp()
             AND a.credential IS NOT NULL{}",
            if lock { " FOR SHARE OF s" } else { "" }
        ),
        vec![session.digest().to_vec().into()],
    ))
    .await
    .map_err(unavailable)?
    .ok_or(Error::Rejected)?
    .try_get("", "id")
    .map_err(unavailable)
}

const PICTURE_COLUMNS: &str = "p.key, octet_length(p.avif) AS avif_bytes,
    octet_length(p.png) AS png_bytes, octet_length(p.sequence) AS sequence_bytes, p.sha256, p.side,
    floor(extract(epoch FROM p.set_at))::bigint AS set_at";

fn bytes16(row: &QueryResult, column: &str) -> Result<[u8; 16], Error> {
    let value: Vec<u8> = row.try_get("", column).map_err(unavailable)?;
    value.try_into().map_err(unavailable)
}

fn bytes32(row: &QueryResult, column: &str) -> Result<[u8; 32], Error> {
    let value: Vec<u8> = row.try_get("", column).map_err(unavailable)?;
    value.try_into().map_err(unavailable)
}

fn picture(row: &QueryResult) -> Result<Picture, Error> {
    let get = |column| row.try_get::<i32>("", column).map_err(unavailable);
    Ok(Picture {
        key: bytes16(row, "key")?,
        side: u16::try_from(row.try_get::<i16>("", "side").map_err(unavailable)?)
            .map_err(unavailable)?,
        avif_bytes: u32::try_from(get("avif_bytes")?).map_err(unavailable)?,
        png_bytes: u32::try_from(get("png_bytes")?).map_err(unavailable)?,
        sequence_bytes: row
            .try_get::<Option<i32>>("", "sequence_bytes")
            .map_err(unavailable)?
            .map(|bytes| u32::try_from(bytes).map_err(unavailable))
            .transpose()?,
        sha256: bytes32(row, "sha256")?,
        set_at: row.try_get("", "set_at").map_err(unavailable)?,
    })
}

fn reason(row: &QueryResult) -> Result<PictureReason, Error> {
    PictureReason::parse(&row.try_get::<String>("", "reason").map_err(unavailable)?)
        .ok_or(Error::Unavailable)
}

/// The notice and block columns of `picture_notices n`.
const NOTICE_COLUMNS: &str = "floor(extract(epoch FROM n.removed_at))::bigint AS removed_at,
    n.reason, n.note, COALESCE(n.blocked_until > pg_catalog.clock_timestamp(), false) AS blocked,
    n.blocked_until = 'infinity'::timestamptz AS forever,
    CASE WHEN n.blocked_until <> 'infinity'::timestamptz
        THEN floor(extract(epoch FROM n.blocked_until))::bigint END AS until";

fn notice(row: &QueryResult) -> Result<(PictureNotice, Option<UploadBlock>), Error> {
    let notice = PictureNotice {
        at: row.try_get("", "removed_at").map_err(unavailable)?,
        reason: reason(row)?,
        note: row.try_get("", "note").map_err(unavailable)?,
    };
    let blocked: bool = row.try_get("", "blocked").map_err(unavailable)?;
    let forever: Option<bool> = row.try_get("", "forever").map_err(unavailable)?;
    let until: Option<i64> = row.try_get("", "until").map_err(unavailable)?;
    let block = match (blocked, forever, until) {
        (false, _, _) => None,
        (true, Some(true), _) => Some(UploadBlock::UntilLifted),
        (true, _, Some(until)) => Some(UploadBlock::Until { until }),
        (true, _, None) => return Err(Error::Unavailable),
    };
    Ok((notice, block))
}

async fn own(db: &impl ConnectionTrait, account: Uuid) -> Result<OwnPicture, Error> {
    let picture = db
        .query_one_raw(statement(
            &format!(
                "SELECT {PICTURE_COLUMNS} FROM central_identity.pictures p WHERE p.account_id = $1"
            ),
            vec![account.into()],
        ))
        .await
        .map_err(unavailable)?
        .as_ref()
        .map(picture)
        .transpose()?;
    let found = db
        .query_one_raw(statement(
            &format!("SELECT {NOTICE_COLUMNS} FROM central_identity.picture_notices n WHERE n.account_id = $1"),
            vec![account.into()],
        ))
        .await
        .map_err(unavailable)?
        .as_ref()
        .map(notice)
        .transpose()?;
    let (notice, block) = found.map_or((None, None), |(notice, block)| (Some(notice), block));
    Ok(OwnPicture {
        picture,
        notice,
        block,
    })
}

/// The block in force for `account`, read in `db`'s snapshot.
async fn block(db: &impl ConnectionTrait, account: Uuid) -> Result<Option<UploadBlock>, Error> {
    Ok(own(db, account).await?.block)
}

impl AccountStore {
    /// A picture's bytes by its key, for anyone: the public read behind the
    /// picture's URLs, with the PNG master's digest as its entity tag. A
    /// removed or replaced picture's key finds nothing.
    ///
    /// # Errors
    /// Storage failure.
    pub async fn picture_bytes(
        &self,
        key: PictureKey,
        encoding: Encoding,
    ) -> Result<Option<(Vec<u8>, PictureDigest)>, Error> {
        let column = match encoding {
            Encoding::Avif => "avif",
            Encoding::Png => "png",
            Encoding::Sequence => "sequence",
        };
        let Some(row) = self
            .db
            .query_one_raw(statement(
                &format!(
                    "SELECT {column} AS bytes, sha256 FROM central_identity.pictures
                     WHERE key = $1 AND {column} IS NOT NULL"
                ),
                vec![key.to_vec().into()],
            ))
            .await
            .map_err(unavailable)?
        else {
            return Ok(None);
        };
        Ok(Some((
            row.try_get("", "bytes").map_err(unavailable)?,
            bytes32(&row, "sha256")?,
        )))
    }

    /// What an upload by the browser session `cookie` may do, before its
    /// body is read: its account, a block in force and the budget's room.
    ///
    /// # Errors
    /// `Rejected` without a live browser session; storage failure.
    pub async fn picture_admission(&self, cookie: &Token) -> Result<Admission, Error> {
        let account = browser_account(&self.db, cookie, false).await?;
        let block = block(&self.db, account).await?;
        let room = self
            .db
            .query_one_raw(statement(
                "SELECT (hour_start <= pg_catalog.clock_timestamp() - interval '1 hour' OR hour_count < $2)
                    AND (day_start <= pg_catalog.clock_timestamp() - interval '1 day' OR day_count < $3) AS room
                 FROM central_identity.picture_budgets WHERE account_id = $1",
                vec![account.into(), UPLOADS_PER_HOUR.into(), UPLOADS_PER_DAY.into()],
            ))
            .await
            .map_err(unavailable)?
            .map_or(Ok(true), |row| row.try_get("", "room").map_err(unavailable))?;
        Ok(Admission {
            account: *account.as_bytes(),
            block,
            room,
        })
    }

    /// Spend one upload of `account`'s budget, once its body was read.
    ///
    /// # Errors
    /// `Busy` when the hour's or the day's uploads are spent; storage failure.
    pub async fn charge_picture(&self, account: [u8; 16]) -> Result<(), Error> {
        let account = Uuid::from_bytes(account);
        let tx = self.db.begin().await.map_err(unavailable)?;
        for sql in [
            "INSERT INTO central_identity.picture_budgets (account_id) VALUES ($1) ON CONFLICT DO NOTHING",
            "UPDATE central_identity.picture_budgets SET hour_start = pg_catalog.clock_timestamp(), hour_count = 0
             WHERE account_id = $1 AND hour_start <= pg_catalog.clock_timestamp() - interval '1 hour'",
            "UPDATE central_identity.picture_budgets SET day_start = pg_catalog.clock_timestamp(), day_count = 0
             WHERE account_id = $1 AND day_start <= pg_catalog.clock_timestamp() - interval '1 day'",
        ] {
            tx.execute_raw(statement(sql, vec![account.into()]))
                .await
                .map_err(unavailable)?;
        }
        let charged = tx
            .execute_raw(statement(
                "UPDATE central_identity.picture_budgets SET hour_count = hour_count + 1, day_count = day_count + 1
                 WHERE account_id = $1 AND hour_count < $2 AND day_count < $3",
                vec![account.into(), UPLOADS_PER_HOUR.into(), UPLOADS_PER_DAY.into()],
            ))
            .await
            .map_err(unavailable)?
            .rows_affected();
        tx.commit().await.map_err(unavailable)?;
        if charged == 1 {
            Ok(())
        } else {
            Err(Error::Busy)
        }
    }

    /// Set the browser session's picture to a converted pair under a new
    /// `key`, in one transaction that locks the account and checks the
    /// session and the block again. Open reports about the replaced picture
    /// become `gone`, and an old notice without a block in force goes.
    ///
    /// # Errors
    /// `Rejected` without the session, or for another account than
    /// `account`; `Conflict` when a block is in force now or the key is
    /// taken; storage failure.
    pub async fn set_picture(
        &self,
        cookie: &Token,
        account: [u8; 16],
        key: PictureKey,
        files: PictureFiles<'_>,
    ) -> Result<OwnPicture, Error> {
        let PictureFiles {
            avif,
            png,
            sequence,
            sha256,
        } = files;
        let account = Uuid::from_bytes(account);
        let tx = self.db.begin().await.map_err(unavailable)?;
        tx.query_one_raw(statement(
            "SELECT id FROM central_identity.accounts WHERE id = $1 FOR UPDATE",
            vec![account.into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Rejected)?;
        if browser_account(&tx, cookie, true).await? != account {
            return Err(Error::Rejected);
        }
        if block(&tx, account).await?.is_some() {
            return Err(Error::Conflict);
        }
        tx.execute_raw(statement(
            "UPDATE central_identity.picture_reports SET state = 'gone', decided_at = pg_catalog.clock_timestamp()
             WHERE account_id = $1 AND state = 'open' AND sha256 <> $2
             AND sha256 = (SELECT sha256 FROM central_identity.pictures WHERE account_id = $1)",
            vec![account.into(), sha256.to_vec().into()],
        ))
        .await
        .map_err(unavailable)?;
        tx.execute_raw(statement(
            "INSERT INTO central_identity.pictures (account_id, key, avif, png, sequence, sha256, side)
             VALUES ($1, $2, $3, $4, $5, $6, 256)
             ON CONFLICT (account_id) DO UPDATE SET key = EXCLUDED.key, avif = EXCLUDED.avif,
                png = EXCLUDED.png, sequence = EXCLUDED.sequence, sha256 = EXCLUDED.sha256,
                side = EXCLUDED.side, set_at = pg_catalog.clock_timestamp()",
            vec![
                account.into(),
                key.to_vec().into(),
                avif.to_vec().into(),
                png.to_vec().into(),
                sequence.map(<[u8]>::to_vec).into(),
                sha256.to_vec().into(),
            ],
        ))
        .await
        .map_err(|_| Error::Conflict)?;
        tx.execute_raw(statement(
            "DELETE FROM central_identity.picture_notices WHERE account_id = $1",
            vec![account.into()],
        ))
        .await
        .map_err(unavailable)?;
        let own = own(&tx, account).await?;
        tx.commit().await.map_err(unavailable)?;
        Ok(own)
    }

    /// The browser session's own picture, notice and block.
    ///
    /// # Errors
    /// `Rejected` without a live browser session; storage failure.
    pub async fn my_picture(&self, cookie: &Token) -> Result<OwnPicture, Error> {
        let account = browser_account(&self.db, cookie, false).await?;
        own(&self.db, account).await
    }

    /// Delete the browser session's picture; both its URLs answer 404 at
    /// once. Open reports about it become `gone`.
    ///
    /// # Errors
    /// `Rejected` without a live browser session; storage failure.
    pub async fn remove_picture(&self, cookie: &Token) -> Result<(), Error> {
        let tx = self.db.begin().await.map_err(unavailable)?;
        let account = browser_account(&tx, cookie, true).await?;
        tx.execute_raw(statement(
            "UPDATE central_identity.picture_reports SET state = 'gone', decided_at = pg_catalog.clock_timestamp()
             WHERE account_id = $1 AND state = 'open'",
            vec![account.into()],
        ))
        .await
        .map_err(unavailable)?;
        tx.execute_raw(statement(
            "DELETE FROM central_identity.pictures WHERE account_id = $1",
            vec![account.into()],
        ))
        .await
        .map_err(unavailable)?;
        tx.commit().await.map_err(unavailable)
    }

    /// The picture reports the browser session's account filed, newest
    /// first, for its export.
    ///
    /// # Errors
    /// `Rejected` without a live browser session; storage failure.
    pub async fn my_picture_reports(&self, cookie: &Token) -> Result<Vec<FiledReport>, Error> {
        let account = browser_account(&self.db, cookie, false).await?;
        let rows = self
            .db
            .query_all_raw(statement(
                "SELECT reason, note, floor(extract(epoch FROM created_at))::bigint AS created_at, state
                 FROM central_identity.picture_reports WHERE reporter_id = $1 ORDER BY id DESC LIMIT 1000",
                vec![account.into()],
            ))
            .await
            .map_err(unavailable)?;
        rows.iter()
            .map(|row| {
                Ok(FiledReport {
                    reason: reason(row)?,
                    note: row.try_get("", "note").map_err(unavailable)?,
                    created_at: row.try_get("", "created_at").map_err(unavailable)?,
                    state: report_state(row)?,
                })
            })
            .collect()
    }
}

fn report_state(row: &QueryResult) -> Result<PictureReportState, Error> {
    PictureReportState::parse(&row.try_get::<String>("", "state").map_err(unavailable)?)
        .ok_or(Error::Unavailable)
}

/// The current picture keys of `tags` for `me` (`SocialRequest::Pictures`),
/// with `me`'s row locked by the caller. Tags outside `me`'s contacts and
/// groups (and other than its own) spend the lookup budget, all at once.
pub(super) async fn pictures(
    tx: &DatabaseTransaction,
    me: Uuid,
    tags: &[u32],
) -> Result<Vec<TaggedPicture>, Error> {
    if tags.len() > MAX_PICTURE_TAGS {
        return Err(Error::Rejected);
    }
    let mut asked: Vec<i32> = tags
        .iter()
        .map(|&tag| i32::try_from(tag).map_err(|_| Error::Rejected))
        .collect::<Result<_, _>>()?;
    asked.sort_unstable();
    asked.dedup();
    let near: i64 = tx
        .query_one_raw(statement(
            "SELECT count(*)::bigint AS near FROM central_identity.accounts a
             WHERE a.tag = ANY($2) AND (a.id = $1
                OR EXISTS (SELECT 1 FROM central_identity.social_contacts c
                    WHERE (c.low = $1 AND c.high = a.id) OR (c.high = $1 AND c.low = a.id))
                OR EXISTS (SELECT 1 FROM central_identity.chat_members m1
                    JOIN central_identity.chat_members m2 ON m2.group_id = m1.group_id
                    WHERE m1.account_id = $1 AND m2.account_id = a.id))",
            vec![me.into(), asked.clone().into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Unavailable)?
        .try_get("", "near")
        .map_err(unavailable)?;
    let outside = i64::try_from(asked.len()).map_err(unavailable)? - near;
    super::social::charge_lookups(tx, me, outside).await?;
    let rows = tx
        .query_all_raw(statement(
            "SELECT a.tag, p.key, p.sequence IS NOT NULL AS moving FROM central_identity.accounts a
             JOIN central_identity.pictures p ON p.account_id = a.id
             WHERE a.tag = ANY($2) AND a.credential IS NOT NULL
             AND NOT EXISTS (SELECT 1 FROM central_identity.social_blocks b
                WHERE (b.owner = $1 AND b.blocked = a.id) OR (b.owner = a.id AND b.blocked = $1))",
            vec![me.into(), asked.into()],
        ))
        .await
        .map_err(unavailable)?;
    let mut found = Vec::with_capacity(rows.len());
    for row in &rows {
        let tag: i32 = row.try_get("", "tag").map_err(unavailable)?;
        found.push(TaggedPicture {
            tag: u32::try_from(tag).map_err(unavailable)?,
            key: bytes16(row, "key")?,
            moving: row.try_get("", "moving").map_err(unavailable)?,
        });
    }
    // In the order the tags were asked for.
    let mut ordered = Vec::with_capacity(found.len());
    for tag in tags {
        if let Some(at) = found.iter().position(|p| p.tag == *tag) {
            ordered.push(found.swap_remove(at));
        }
    }
    Ok(ordered)
}

/// File `me`'s report of `other`'s current picture `key`, with `me`'s row
/// locked by the caller.
pub(super) async fn report(
    tx: &DatabaseTransaction,
    me: Uuid,
    other: Uuid,
    key: PictureKey,
    reason: PictureReason,
    note: Option<&str>,
) -> Result<(), Error> {
    if me == other || note.is_some_and(|note| !valid_note(note, MAX_REPORT_NOTE)) {
        return Err(Error::Rejected);
    }
    // The picture the caller saw, still current, and visible to it.
    let sha256 = tx
        .query_one_raw(statement(
            "SELECT p.sha256 FROM central_identity.pictures p WHERE p.account_id = $2 AND p.key = $3
             AND NOT EXISTS (SELECT 1 FROM central_identity.social_blocks b
                WHERE (b.owner = $1 AND b.blocked = $2) OR (b.owner = $2 AND b.blocked = $1))",
            vec![me.into(), other.into(), key.to_vec().into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Rejected)
        .and_then(|row| bytes32(&row, "sha256"))?;
    let counts = tx
        .query_one_raw(statement(
            "SELECT count(*) FILTER (WHERE created_at > pg_catalog.clock_timestamp() - interval '1 hour') AS hour,
                    count(*) FILTER (WHERE state = 'open') AS open
             FROM central_identity.picture_reports WHERE reporter_id = $1",
            vec![me.into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Unavailable)?;
    let hour: i64 = counts.try_get("", "hour").map_err(unavailable)?;
    let open: i64 = counts.try_get("", "open").map_err(unavailable)?;
    if hour >= REPORTS_PER_HOUR || open >= REPORTS_OPEN {
        return Err(Error::Busy);
    }
    let filed = tx
        .execute_raw(statement(
            "INSERT INTO central_identity.picture_reports (account_id, sha256, reporter_id, reason, note)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (account_id, sha256, reporter_id) WHERE state = 'open' DO NOTHING",
            vec![
                other.into(),
                sha256.to_vec().into(),
                me.into(),
                reason.as_str().into(),
                note.map(str::to_owned).into(),
            ],
        ))
        .await
        .map_err(unavailable)?
        .rows_affected();
    if filed == 1 {
        Ok(())
    } else {
        Err(Error::Conflict)
    }
}

/// The columns of an [`AdminPictureReport`], from `picture_reports r` with
/// the account `a`, the reporter `rep` and the current picture `p` joined.
const REPORT_COLUMNS: &str = "r.id, r.account_id, a.display_name, a.tag, r.sha256,
    p.key AS current, coalesce(p.sequence IS NOT NULL, false) AS moving, r.reporter_id, rep.display_name AS reporter_name, rep.tag AS reporter_tag,
    r.reason, r.note, floor(extract(epoch FROM r.created_at))::bigint AS created_at, r.state,
    floor(extract(epoch FROM r.decided_at))::bigint AS decided_at";
const REPORT_FROM: &str = "central_identity.picture_reports r
    JOIN central_identity.accounts a ON a.id = r.account_id
    LEFT JOIN central_identity.accounts rep ON rep.id = r.reporter_id
    LEFT JOIN central_identity.pictures p ON p.account_id = r.account_id AND p.sha256 = r.sha256";

fn tag(row: &QueryResult, column: &str) -> Result<Option<u32>, Error> {
    let tag: Option<i32> = row.try_get("", column).map_err(unavailable)?;
    tag.map(|tag| u32::try_from(tag).map_err(unavailable))
        .transpose()
}

fn admin_report(row: &QueryResult) -> Result<AdminPictureReport, Error> {
    let reporter: Option<Uuid> = row.try_get("", "reporter_id").map_err(unavailable)?;
    let current: Option<Vec<u8>> = row.try_get("", "current").map_err(unavailable)?;
    Ok(AdminPictureReport {
        id: id(row, "id")?,
        account: Named {
            id: id(row, "account_id")?,
            display_name: row.try_get("", "display_name").map_err(unavailable)?,
            tag: tag(row, "tag")?,
        },
        sha256: bytes32(row, "sha256")?,
        current: current
            .map(|key| key.try_into().map_err(unavailable))
            .transpose()?,
        moving: row.try_get("", "moving").map_err(unavailable)?,
        reporter: reporter
            .map(|reporter| {
                Ok::<_, Error>(Named {
                    id: *reporter.as_bytes(),
                    display_name: row.try_get("", "reporter_name").map_err(unavailable)?,
                    tag: tag(row, "reporter_tag")?,
                })
            })
            .transpose()?,
        reason: reason(row)?,
        note: row.try_get("", "note").map_err(unavailable)?,
        created_at: row.try_get("", "created_at").map_err(unavailable)?,
        state: report_state(row)?,
        decided_at: row.try_get("", "decided_at").map_err(unavailable)?,
    })
}

impl AccountStore {
    /// A page of picture reports for the administrator behind `browser`,
    /// newest first: all, or those in `state`.
    ///
    /// # Errors
    /// `Rejected` without an administrator's live browser session; storage
    /// failure.
    pub async fn admin_picture_reports(
        &self,
        browser: &Token,
        state: Option<PictureReportState>,
        before: Option<[u8; 16]>,
    ) -> Result<(Vec<AdminPictureReport>, bool), Error> {
        Self::administrator(&self.db, browser).await?;
        let rows = self
            .db
            .query_all_raw(statement(
                &format!(
                    "SELECT {REPORT_COLUMNS} FROM {REPORT_FROM}
                     WHERE ($1::text IS NULL OR r.state = $1) AND ($2::uuid IS NULL OR r.id < $2)
                     ORDER BY r.id DESC LIMIT $3"
                ),
                vec![
                    state.map(|s| s.as_str().to_owned()).into(),
                    before.map(Uuid::from_bytes).into(),
                    i64::try_from(REPORT_PAGE + 1).map_err(unavailable)?.into(),
                ],
            ))
            .await
            .map_err(unavailable)?;
        let reports = rows
            .iter()
            .take(REPORT_PAGE)
            .map(admin_report)
            .collect::<Result<Vec<_>, _>>()?;
        Ok((reports, rows.len() > REPORT_PAGE))
    }

    /// One account's picture case for the administrator behind `browser`.
    ///
    /// # Errors
    /// `Rejected` without an administrator's live browser session and for an
    /// unknown account; storage failure.
    pub async fn admin_picture_case(
        &self,
        browser: &Token,
        account: [u8; 16],
    ) -> Result<PictureCase, Error> {
        Self::administrator(&self.db, browser).await?;
        let account = Uuid::from_bytes(account);
        let row = self
            .db
            .query_one_raw(statement(
                "SELECT id, display_name, tag FROM central_identity.accounts WHERE id = $1",
                vec![account.into()],
            ))
            .await
            .map_err(unavailable)?
            .ok_or(Error::Rejected)?;
        let named = Named {
            id: id(&row, "id")?,
            display_name: row.try_get("", "display_name").map_err(unavailable)?,
            tag: tag(&row, "tag")?,
        };
        let reports = self
            .db
            .query_all_raw(statement(
                &format!(
                    "SELECT {REPORT_COLUMNS} FROM {REPORT_FROM} WHERE r.account_id = $1
                     ORDER BY r.id DESC LIMIT $2"
                ),
                vec![
                    account.into(),
                    i64::try_from(REPORT_PAGE).map_err(unavailable)?.into(),
                ],
            ))
            .await
            .map_err(unavailable)?
            .iter()
            .map(admin_report)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(PictureCase {
            account: named,
            own: own(&self.db, account).await?,
            reports,
        })
    }

    /// Keep or remove `account`'s picture with digest `sha256`, for the
    /// administrator behind `browser`, whose password sign-in must be less
    /// than 30 minutes old; audited in the same transaction, with the
    /// reason code only.
    ///
    /// # Errors
    /// `Rejected` without a fresh administrator's session, for a note out
    /// of bounds or a block of 0 or more than [`MAX_BLOCK_DAYS`] days;
    /// `Conflict` when the account's picture is no longer the one decided
    /// on, or there is nothing open to keep; storage failure.
    pub async fn decide_picture(
        &self,
        browser: &Token,
        account: [u8; 16],
        sha256: PictureDigest,
        decision: &PictureDecision,
    ) -> Result<(), Error> {
        if let PictureDecision::Remove { note, block, .. } = decision
            && (note
                .as_deref()
                .is_some_and(|n| !valid_note(n, MAX_NOTICE_NOTE))
                || matches!(block, BlockChoice::Days { days } if *days == 0 || *days > MAX_BLOCK_DAYS))
        {
            return Err(Error::Rejected);
        }
        let (tx, actor) = self.acting_administrator(browser).await?;
        let account = Uuid::from_bytes(account);
        tx.query_one_raw(statement(
            "SELECT id FROM central_identity.accounts WHERE id = $1 FOR UPDATE",
            vec![account.into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Rejected)?;
        let current = tx
            .query_one_raw(statement(
                "SELECT sha256 FROM central_identity.pictures WHERE account_id = $1",
                vec![account.into()],
            ))
            .await
            .map_err(unavailable)?
            .map(|row| bytes32(&row, "sha256"))
            .transpose()?;
        let state = match decision {
            PictureDecision::Keep => "kept",
            PictureDecision::Remove { .. } => "removed",
        };
        let decided = tx
            .execute_raw(statement(
                "UPDATE central_identity.picture_reports
                 SET state = $3, decided_at = pg_catalog.clock_timestamp(), decided_by = $4
                 WHERE account_id = $1 AND sha256 = $2 AND state = 'open'",
                vec![
                    account.into(),
                    sha256.to_vec().into(),
                    state.into(),
                    actor.into(),
                ],
            ))
            .await
            .map_err(unavailable)?
            .rows_affected();
        match decision {
            PictureDecision::Keep => {
                if decided == 0 {
                    return Err(Error::Conflict);
                }
                audit(&tx, actor, Action::PictureKept, account, None).await?;
            }
            PictureDecision::Remove {
                reason,
                note,
                block,
            } => {
                if current != Some(sha256) {
                    return Err(Error::Conflict);
                }
                remove(&tx, account, *reason, note.as_deref(), *block).await?;
                audit(&tx, actor, Action::PictureRemoved, account, Some(*reason)).await?;
                if !matches!(block, BlockChoice::None) {
                    audit(&tx, actor, Action::PictureBlocked, account, None).await?;
                }
            }
        }
        tx.commit().await.map_err(unavailable)
    }

    /// Lift `account`'s upload block, for the administrator behind
    /// `browser`, whose password sign-in must be less than 30 minutes old;
    /// audited when there was one.
    ///
    /// # Errors
    /// `Rejected` without a fresh administrator's session; storage failure.
    pub async fn lift_picture_block(
        &self,
        browser: &Token,
        account: [u8; 16],
    ) -> Result<(), Error> {
        let (tx, actor) = self.acting_administrator(browser).await?;
        let account = Uuid::from_bytes(account);
        let lifted = tx
            .execute_raw(statement(
                "UPDATE central_identity.picture_notices SET blocked_until = NULL
                 WHERE account_id = $1 AND blocked_until > pg_catalog.clock_timestamp()",
                vec![account.into()],
            ))
            .await
            .map_err(unavailable)?
            .rows_affected();
        if lifted > 0 {
            audit(&tx, actor, Action::PictureUnblocked, account, None).await?;
        }
        tx.commit().await.map_err(unavailable)
    }
}

/// Delete the picture and write the person's notice and block.
async fn remove(
    tx: &DatabaseTransaction,
    account: Uuid,
    reason: PictureReason,
    note: Option<&str>,
    block: BlockChoice,
) -> Result<(), Error> {
    tx.execute_raw(statement(
        "DELETE FROM central_identity.pictures WHERE account_id = $1",
        vec![account.into()],
    ))
    .await
    .map_err(unavailable)?;
    let (days, forever): (Option<i32>, bool) = match block {
        BlockChoice::None => (None, false),
        BlockChoice::Days { days } => (Some(i32::from(days)), false),
        BlockChoice::UntilLifted => (None, true),
    };
    tx.execute_raw(statement(
        "INSERT INTO central_identity.picture_notices (account_id, removed_at, reason, note, blocked_until)
         VALUES ($1, pg_catalog.clock_timestamp(), $2, $3,
            CASE WHEN $5 THEN 'infinity'::timestamptz
                 WHEN $4::integer IS NOT NULL THEN pg_catalog.clock_timestamp() + pg_catalog.make_interval(days => $4)
            END)
         ON CONFLICT (account_id) DO UPDATE SET removed_at = EXCLUDED.removed_at, reason = EXCLUDED.reason,
            note = EXCLUDED.note, blocked_until = EXCLUDED.blocked_until",
        vec![
            account.into(),
            reason.as_str().into(),
            note.map(str::to_owned).into(),
            days.into(),
            forever.into(),
        ],
    ))
    .await
    .map_err(unavailable)?;
    Ok(())
}

/// One audit event about `account`'s picture; the detail holds the reason
/// code only, never a note.
async fn audit(
    tx: &DatabaseTransaction,
    actor: Uuid,
    action: Action,
    account: Uuid,
    reason: Option<PictureReason>,
) -> Result<(), Error> {
    tx.execute_raw(statement(
        "INSERT INTO central_identity.audit_events (actor_kind, actor_account, action, account_id, detail)
         VALUES ('admin', $1, $2, $3,
            CASE WHEN $4::text IS NULL THEN NULL ELSE pg_catalog.jsonb_build_object('reason', $4::text) END)",
        vec![
            actor.into(),
            action.as_str().into(),
            account.into(),
            reason.map(|r| r.as_str().to_owned()).into(),
        ],
    ))
    .await
    .map_err(unavailable)?;
    Ok(())
}

/// Delete decided picture reports past [`DECIDED_REPORT_DAYS`], in the
/// operator's sweep.
pub(super) async fn sweep(tx: &impl ConnectionTrait) -> Result<u64, Error> {
    Ok(tx
        .execute_raw(statement(
            "DELETE FROM central_identity.picture_reports
             WHERE decided_at < pg_catalog.clock_timestamp() - pg_catalog.make_interval(days => $1)",
            vec![DECIDED_REPORT_DAYS.into()],
        ))
        .await
        .map_err(unavailable)?
        .rows_affected())
}
