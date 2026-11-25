//! Display names and tags (migration 000011, `docs/web-accounts.md`
//! §"Display name and tag"): the tag a registered account draws, the name it
//! chooses and the name and tag the bridge carries from the official gateway.
use super::{AccountStore, Error, Session, Token, statement};
use central_api::profile::{
    self, ProfileDetails, PublicBadges, PublicPicture, PublicProfile, REGISTERED_TAG_FLOOR,
};
use sea_orm::{
    ConnectionTrait, DatabaseTransaction, QueryResult, SqlErr, Statement, TransactionTrait,
    prelude::Uuid,
};
use std::{ops::RangeInclusive, sync::Arc};

/// Draws of a registered account's tag before a request is `Busy`.
pub const TAG_DRAWS: usize = 8;

/// Draws one tag from a range; [`profile::draw_tag`] unless a test injects another.
pub type TagDraw = Arc<dyn Fn(&RangeInclusive<u32>) -> Result<u32, Error> + Send + Sync>;

/// How a registered account's tag is drawn: from the first of
/// [`profile::tag_ranges`] of which fewer than half are taken.
#[derive(Clone)]
pub(super) struct Tags {
    floor: u32,
    draw: TagDraw,
}

impl Default for Tags {
    fn default() -> Self {
        Self {
            floor: REGISTERED_TAG_FLOOR,
            draw: Arc::new(profile::draw_tag),
        }
    }
}

/// The display name and tag columns of `row`.
pub(super) fn columns(row: &QueryResult) -> Result<(Option<String>, Option<u32>), Error> {
    let name: Option<String> = row
        .try_get("", "display_name")
        .map_err(|_| Error::Unavailable)?;
    let tag: Option<i32> = row.try_get("", "tag").map_err(|_| Error::Unavailable)?;
    let tag = tag
        .map(|t| u32::try_from(t).map_err(|_| Error::Unavailable))
        .transpose()?;
    Ok((name, tag))
}

fn wire_tag(tag: u32) -> Result<i32, Error> {
    if !profile::valid_tag(tag) {
        return Err(Error::Rejected);
    }
    i32::try_from(tag).map_err(|_| Error::Rejected)
}

/// A carried name: the rule's shape only; reserved names are kept.
pub(super) fn carried_name(name: Option<&str>) -> Result<Option<&str>, Error> {
    match name {
        Some(name) if !profile::valid_display_name(name) => Err(Error::Rejected),
        name => Ok(name),
    }
}

/// A carried tag, in a gateway's column.
pub(super) fn carried_tag(tag: Option<u32>) -> Result<Option<i32>, Error> {
    tag.map(wire_tag).transpose()
}

impl AccountStore {
    /// Draw registered accounts' tags from `floor` with `draw`, for a test
    /// that needs a small range or a known sequence. The service keeps
    /// [`REGISTERED_TAG_FLOOR`] and operating-system randomness.
    #[must_use]
    pub fn with_tag_draw(mut self, floor: u32, draw: TagDraw) -> Self {
        self.tags = Tags { floor, draw };
        self
    }

    /// [`TAG_DRAWS`] candidate tags from the first range of which fewer than
    /// half are taken.
    ///
    /// # Errors
    /// `Busy` when every range is half taken; `Unavailable` for storage or
    /// randomness failure.
    pub(super) async fn draws(&self, db: &impl ConnectionTrait) -> Result<Vec<i32>, Error> {
        for range in profile::tag_ranges(self.tags.floor) {
            let taken: i64 = db
                .query_one_raw(statement(
                    "SELECT count(*) AS taken FROM central_identity.accounts WHERE tag BETWEEN $1 AND $2",
                    vec![wire_tag(*range.start())?.into(), wire_tag(*range.end())?.into()],
                ))
                .await
                .map_err(|_| Error::Unavailable)?
                .ok_or(Error::Unavailable)?
                .try_get("", "taken")
                .map_err(|_| Error::Unavailable)?;
            let size = u64::from(range.end() - range.start()) + 1;
            if u64::try_from(taken).map_err(|_| Error::Unavailable)? * 2 < size {
                return (0..TAG_DRAWS)
                    .map(|_| {
                        let tag = (self.tags.draw)(&range)?;
                        if !range.contains(&tag) {
                            return Err(Error::Unavailable);
                        }
                        wire_tag(tag)
                    })
                    .collect();
            }
        }
        Err(Error::Busy)
    }

    /// The live browser session's account chooses its display name; see
    /// [`central_api::wire::Request::SetDisplayName`]. Renaming is not
    /// allowed yet: the same name again is `Done`, another `Conflict`.
    ///
    /// # Errors
    /// `Rejected` for a name a person may not choose, no live browser session
    /// or an account without a tag (a moved account whose name and tag the
    /// bridge has not carried yet); `Conflict` for another name while one is
    /// set; `Unavailable` for storage failure.
    pub async fn set_display_name(&self, browser: &Token, name: &str) -> Result<(), Error> {
        if !profile::choosable(name) {
            return Err(Error::Rejected);
        }
        let session = Session::Browser(browser);
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let row = tx
            .query_one_raw(statement(
                "SELECT a.id, a.display_name, a.tag FROM central_identity.accounts a
                 JOIN central_identity.sessions s
                 ON s.account_id = a.id AND s.credential_revision = a.credential_revision
                 WHERE s.digest = $1 AND s.kind = $2 AND a.credential IS NOT NULL
                 AND s.expires_at > pg_catalog.clock_timestamp() FOR UPDATE OF a",
                vec![session.digest().to_vec().into(), session.kind().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        let account: Uuid = row.try_get("", "id").map_err(|_| Error::Unavailable)?;
        match columns(&row)? {
            // A retry of the request that set it, or a rename (step 9).
            (Some(current), _) => {
                return if current == name {
                    Ok(())
                } else {
                    Err(Error::Conflict)
                };
            }
            // A moved account keeps its gateway's name: it waits for the bridge.
            (None, None) => return Err(Error::Rejected),
            (None, Some(_)) => {}
        }
        let set = tx
            .execute_raw(statement(
                "UPDATE central_identity.accounts SET display_name = $2,
                 display_name_at = pg_catalog.clock_timestamp()
                 WHERE id = $1 AND display_name IS NULL AND tag IS NOT NULL",
                vec![account.into(), name.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        if set.rows_affected() != 1 {
            return Err(Error::Unavailable);
        }
        tx.commit().await.map_err(|_| Error::Unavailable)
    }

    /// Carry a legacy account's display name and tag once, for an account
    /// reserved before Central kept them; see
    /// [`central_api::wire::Request::LegacyProfile`]. A tag another account
    /// holds already is replaced by a drawn one, and the name is kept.
    ///
    /// # Errors
    /// `Rejected` for an unknown legacy account, a malformed name or tag;
    /// `Conflict` when the account has its tag already; `Busy` when no tag
    /// can be drawn; `Unavailable` for storage failure.
    pub async fn legacy_profile(
        &self,
        legacy: &str,
        name: Option<&str>,
        tag: u32,
    ) -> Result<(), Error> {
        let name = carried_name(name)?;
        let carried = wire_tag(tag)?;
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let row = tx
            .query_one_raw(statement(
                "SELECT id, display_name, tag FROM central_identity.accounts
                 WHERE legacy_id = $1 AND source = 'legacy' FOR UPDATE",
                vec![legacy.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        let account: Uuid = row.try_get("", "id").map_err(|_| Error::Unavailable)?;
        if columns(&row)?.1.is_some() {
            return Err(Error::Conflict);
        }
        let written = self
            .first_free_tag(
                &tx,
                Some(account),
                Some(carried),
                Error::Unavailable,
                |tag| {
                    statement(
                        "UPDATE central_identity.accounts SET tag = $2,
                     display_name = COALESCE(display_name, $3),
                     display_name_at = CASE WHEN display_name IS NULL AND $3::text IS NOT NULL
                         THEN pg_catalog.clock_timestamp() ELSE display_name_at END
                     WHERE id = $1 AND tag IS NULL RETURNING id",
                        vec![account.into(), tag.into(), name.into()],
                    )
                },
            )
            .await?;
        if written.is_none() {
            return Err(Error::Unavailable);
        }
        tx.commit().await.map_err(|_| Error::Unavailable)
    }

    /// Owner-only profile details (given name, family name, ISO birth date)
    /// and the account's public profile visibility flag.
    ///
    /// # Errors
    /// `Rejected` without a live browser session; storage failure.
    pub async fn profile_details(&self, browser: &Token) -> Result<ProfileDetails, Error> {
        read_profile_details(&self.db, browser, false).await
    }

    /// Owner-only export of personal profile details, requiring a password
    /// sign-in less than 30 minutes old.
    ///
    /// # Errors
    /// `Rejected` without a fresh live browser session; storage failure.
    pub async fn export_profile_details(&self, browser: &Token) -> Result<ProfileDetails, Error> {
        read_profile_details(&self.db, browser, true).await
    }

    /// Update owner-only profile details and public profile visibility for the
    /// account behind `browser`. Optional personal names are trimmed and
    /// checked with `shown_char`; optional `birth_date` (`YYYY-MM-DD`) must be
    /// valid and at least 16 years old relative to the database's `CURRENT_DATE`.
    ///
    /// # Errors
    /// `Rejected` for invalid names, malformed birth date, age under 16, or
    /// missing live browser session; storage failure.
    pub async fn set_profile_details(
        &self,
        browser: &Token,
        details: &ProfileDetails,
    ) -> Result<(), Error> {
        let first_name = profile::validate_person_name(details.first_name.as_deref())?;
        let last_name = profile::validate_person_name(details.last_name.as_deref())?;
        let birth_date = match details.birth_date.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(date) => {
                let (y, m, d) = profile::parse_iso_date(date)?;
                Some(format!("{y:04}-{m:02}-{d:02}"))
            }
        };
        let session = Session::Browser(browser);
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let row = tx
            .query_one_raw(statement(
                "SELECT a.id FROM central_identity.accounts a
                 JOIN central_identity.sessions s
                     ON s.account_id = a.id AND s.credential_revision = a.credential_revision
                 WHERE s.digest = $1 AND s.kind = 'browser' AND a.credential IS NOT NULL
                 AND s.expires_at > pg_catalog.clock_timestamp() FOR UPDATE OF a",
                vec![session.digest().to_vec().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        let account: Uuid = row.try_get("", "id").map_err(|_| Error::Unavailable)?;
        let updated = tx
            .query_one_raw(statement(
                "UPDATE central_identity.accounts
                 SET first_name = $2,
                     last_name = $3,
                     birth_date = $4::date,
                     public_profile = $5
                 WHERE id = $1
                 AND ($4::date IS NULL OR $4::date <= (CURRENT_DATE - INTERVAL '16 years')::date)
                 RETURNING id",
                vec![
                    account.into(),
                    first_name.into(),
                    last_name.into(),
                    birth_date.into(),
                    details.public_profile.into(),
                ],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        if updated.is_none() {
            return Err(Error::Rejected);
        }
        tx.commit().await.map_err(|_| Error::Unavailable)
    }

    /// Current ranked standings, awards, challenge medals and Tower badges for
    /// the account behind `browser`.
    ///
    /// # Errors
    /// `Rejected` without a live browser session; storage failure.
    pub async fn own_badges(&self, browser: &Token) -> Result<PublicBadges, Error> {
        let session = Session::Browser(browser);
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let row = tx
            .query_one_raw(statement(
                "SELECT a.id FROM central_identity.accounts a
                 JOIN central_identity.sessions s
                     ON s.account_id = a.id AND s.credential_revision = a.credential_revision
                 WHERE s.digest = $1 AND s.kind = 'browser' AND a.credential IS NOT NULL
                 AND s.expires_at > pg_catalog.clock_timestamp()",
                vec![session.digest().to_vec().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        let account: Uuid = row.try_get("", "id").map_err(|_| Error::Unavailable)?;
        let badges = collect_badges(&tx, account).await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(badges)
    }

    /// Public showcase for a player handle (`Name#tag` or `Name-tag`), served
    /// when `public_profile = true`. Private or missing handles both return
    /// `Rejected` so opt-out is indistinguishable from absence.
    ///
    /// # Errors
    /// `Rejected` for invalid handle, private profile, or unknown account;
    /// storage failure.
    pub async fn public_profile(&self, raw_handle: &str) -> Result<PublicProfile, Error> {
        let (name, tag) = profile::parse_handle(raw_handle)?;
        let tag_num = wire_tag(tag)?;
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let row = tx
            .query_one_raw(statement(
                "SELECT id, display_name, tag FROM central_identity.accounts
                 WHERE display_name = $1 AND tag = $2
                 AND credential IS NOT NULL AND public_profile",
                vec![name.into(), tag_num.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        let account: Uuid = row.try_get("", "id").map_err(|_| Error::Unavailable)?;
        let (display_name, tag) = columns(&row)?;
        let (Some(display_name), Some(tag)) = (display_name, tag) else {
            return Err(Error::Rejected);
        };
        let pic = tx
            .query_one_raw(statement(
                "SELECT key, sequence IS NOT NULL AS moving
                 FROM central_identity.pictures WHERE account_id = $1",
                vec![account.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        let picture = pic
            .map(|p| {
                let bytes: Vec<u8> = p.try_get("", "key").map_err(|_| Error::Unavailable)?;
                let key = <[u8; 16]>::try_from(bytes).map_err(|_| Error::Unavailable)?;
                Ok::<_, Error>(PublicPicture {
                    key,
                    moving: p.try_get("", "moving").map_err(|_| Error::Unavailable)?,
                })
            })
            .transpose()?;
        let badges = collect_badges(&tx, account).await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(PublicProfile {
            display_name,
            tag,
            picture,
            badges,
        })
    }
}

async fn read_profile_details(
    db: &impl ConnectionTrait,
    browser: &Token,
    require_fresh: bool,
) -> Result<ProfileDetails, Error> {
    let session = Session::Browser(browser);
    let fresh_clause = if require_fresh {
        " AND COALESCE(s.proven_at, s.created_at) > pg_catalog.clock_timestamp() - interval '30 minutes'"
    } else {
        ""
    };
    let row = db
        .query_one_raw(statement(
            &format!(
                "SELECT a.first_name, a.last_name,
                     to_char(a.birth_date, 'YYYY-MM-DD') AS birth_date,
                     a.public_profile
                 FROM central_identity.accounts a
                 JOIN central_identity.sessions s
                     ON s.account_id = a.id AND s.credential_revision = a.credential_revision
                 WHERE s.digest = $1 AND s.kind = 'browser' AND a.credential IS NOT NULL
                 AND s.expires_at > pg_catalog.clock_timestamp(){fresh_clause}"
            ),
            vec![session.digest().to_vec().into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::Rejected)?;
    Ok(ProfileDetails {
        first_name: row
            .try_get("", "first_name")
            .map_err(|_| Error::Unavailable)?,
        last_name: row
            .try_get("", "last_name")
            .map_err(|_| Error::Unavailable)?,
        birth_date: row
            .try_get("", "birth_date")
            .map_err(|_| Error::Unavailable)?,
        public_profile: row
            .try_get("", "public_profile")
            .map_err(|_| Error::Unavailable)?,
    })
}

async fn collect_badges(tx: &DatabaseTransaction, account: Uuid) -> Result<PublicBadges, Error> {
    let (ranked_season, ranked_standings, ranked_awards) =
        super::ranked::boards::account_ranked_badges(tx, account).await?;
    let best_rank = central_api::ranked::best_player_rank(&ranked_standings);
    let challenge_badges = super::challenges::account_badges(tx, account).await?;
    let tower_badges = super::challenges::tower_badges(tx, account).await?;
    let echoes_badges = super::tower_awards::totals(tx, account)
        .await?
        .badges()
        .to_vec();
    let social_badges = super::social::invites::account_social_badges(tx, account).await?;
    Ok(PublicBadges {
        ranked_season,
        ranked_standings,
        ranked_awards,
        best_rank,
        challenge_badges,
        tower_badges,
        echoes_badges,
        social_badges,
    })
}

/// What one write with one tag came to.
enum Attempt {
    /// It ran: its row, if it returned one.
    Written(Option<QueryResult>),
    /// Another account holds the tag.
    Taken,
}

impl AccountStore {
    /// Runs `write` with `carried` first, when given, then with [`draws`]
    /// once it is taken (or at once without it), each in a savepoint of its
    /// own, until a tag is held by no account other than `account`: a unique
    /// violation on the tag rolls back only its savepoint. Any other failure
    /// of `write` is `failed`.
    ///
    /// [`draws`]: AccountStore::draws
    ///
    /// # Errors
    /// `Busy` when every draw was taken or no range is left; `failed` for
    /// another failure of the write; `Unavailable` for storage failure
    /// around it.
    pub(super) async fn first_free_tag(
        &self,
        tx: &DatabaseTransaction,
        account: Option<Uuid>,
        carried: Option<i32>,
        failed: Error,
        write: impl Fn(i32) -> Statement,
    ) -> Result<Option<QueryResult>, Error> {
        if let Some(tag) = carried
            && let Attempt::Written(row) = attempt(tx, account, tag, failed, &write).await?
        {
            return Ok(row);
        }
        for tag in self.draws(tx).await? {
            if let Attempt::Written(row) = attempt(tx, account, tag, failed, &write).await? {
                return Ok(row);
            }
        }
        Err(Error::Busy)
    }
}

async fn attempt(
    tx: &DatabaseTransaction,
    account: Option<Uuid>,
    tag: i32,
    failed: Error,
    write: &impl Fn(i32) -> Statement,
) -> Result<Attempt, Error> {
    let savepoint = tx.begin().await.map_err(|_| Error::Unavailable)?;
    match savepoint.query_one_raw(write(tag)).await {
        Ok(row) => {
            savepoint.commit().await.map_err(|_| Error::Unavailable)?;
            Ok(Attempt::Written(row))
        }
        Err(error) => {
            savepoint.rollback().await.map_err(|_| Error::Unavailable)?;
            let unique = matches!(error.sql_err(), Some(SqlErr::UniqueConstraintViolation(_)));
            if unique && held(tx, tag, account).await? {
                Ok(Attempt::Taken)
            } else {
                Err(failed)
            }
        }
    }
}

/// Whether an account other than `account` holds `tag`.
async fn held(tx: &DatabaseTransaction, tag: i32, account: Option<Uuid>) -> Result<bool, Error> {
    tx.query_one_raw(statement(
        "SELECT EXISTS (SELECT 1 FROM central_identity.accounts
         WHERE tag = $1 AND id IS DISTINCT FROM $2) AS held",
        vec![tag.into(), account.into()],
    ))
    .await
    .map_err(|_| Error::Unavailable)?
    .ok_or(Error::Unavailable)?
    .try_get("", "held")
    .map_err(|_| Error::Unavailable)
}
