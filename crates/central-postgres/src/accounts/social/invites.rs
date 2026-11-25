//! Single-use friend invites: storage, atomic redemption and invite-only registration.

use super::super::email::MAIL_CODE_MINUTES;
use super::super::{AccountStore, Error, Session, live::Concerns, statement};
use super::{caller, lock_accounts, unavailable};
use central_api::email as api_email;
use central_api::social::{
    FriendInvitePreview, FriendInviteStatus, FriendInviteSummary, MAX_ACTIVE_FRIEND_INVITES,
    MAX_CONTACTS, MAX_INVITE_LABEL, MAX_INVITE_NOTE, Person, SocialBadge, SocialBadgeKind,
    SocialRequest, SocialResponse, clean_invite_text, generate_friend_invite, invite_code_digest,
    invite_expiry_days,
};
use sea_orm::{ConnectionTrait, QueryResult, TransactionTrait, prelude::Uuid};
use secrecy::zeroize::Zeroizing;

const ACCEPTED_FRIENDS_SQL: &str = "SELECT count(*) AS count FROM central_identity.social_contacts WHERE accepted AND (low = $1 OR high = $1)";

struct InviteInput<'a> {
    label: Option<&'a str>,
    note: Option<&'a str>,
    email: Option<&'a str>,
    lang: Option<&'a str>,
    expires_days: Option<u8>,
}

fn status_from_flags(redeemed: bool, revoked: bool, expired: bool) -> FriendInviteStatus {
    if redeemed {
        FriendInviteStatus::Redeemed
    } else if revoked {
        FriendInviteStatus::Revoked
    } else if expired {
        FriendInviteStatus::Expired
    } else {
        FriendInviteStatus::Active
    }
}

fn clean_optional_email(raw: Option<&str>) -> Result<Option<String>, Error> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    api_email::address(trimmed)?;
    Ok(Some(trimmed.to_owned()))
}

fn clean_lang(raw: Option<&str>) -> Result<String, Error> {
    let Some(raw) = raw else {
        return Ok("en".to_owned());
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok("en".to_owned());
    }
    api_email::language(trimmed)?;
    Ok(trimmed.to_owned())
}

fn optional_person(name: Option<String>, tag: Option<i32>) -> Result<Option<Person>, Error> {
    match (name, tag) {
        (Some(display_name), Some(raw_tag)) => Ok(Some(Person {
            tag: u32::try_from(raw_tag).map_err(unavailable)?,
            display_name,
        })),
        _ => Ok(None),
    }
}

async fn has_block(db: &impl ConnectionTrait, a: Uuid, b: Uuid) -> Result<bool, Error> {
    db.query_one_raw(statement(
        "SELECT 1 FROM central_identity.social_blocks
         WHERE (owner = $1 AND blocked = $2) OR (owner = $2 AND blocked = $1)
         LIMIT 1",
        vec![a.into(), b.into()],
    ))
    .await
    .map_err(unavailable)
    .map(|row| row.is_some())
}

async fn accepted_friends_count(db: &impl ConnectionTrait, account: Uuid) -> Result<i64, Error> {
    let row = db
        .query_one_raw(statement(ACCEPTED_FRIENDS_SQL, vec![account.into()]))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Unavailable)?;
    row.try_get("", "count").map_err(unavailable)
}

async fn ensure_friend_room(db: &impl ConnectionTrait, account: Uuid) -> Result<(), Error> {
    let limit = i64::try_from(MAX_CONTACTS).map_err(unavailable)?;
    if accepted_friends_count(db, account).await? >= limit {
        return Err(Error::Busy);
    }
    Ok(())
}

async fn make_mutual_friends(
    tx: &impl ConnectionTrait,
    inviter_id: Uuid,
    peer_id: Uuid,
) -> Result<(), Error> {
    if inviter_id == peer_id {
        return Err(Error::Rejected);
    }
    if has_block(tx, inviter_id, peer_id).await? {
        return Err(Error::Rejected);
    }
    let (low, high) = if inviter_id < peer_id {
        (inviter_id, peer_id)
    } else {
        (peer_id, inviter_id)
    };
    let existing = tx
        .query_one_raw(statement(
            "SELECT accepted FROM central_identity.social_contacts WHERE low = $1 AND high = $2 FOR UPDATE",
            vec![low.into(), high.into()],
        ))
        .await
        .map_err(unavailable)?;
    let already_friends = existing
        .as_ref()
        .map(|row| row.try_get::<bool>("", "accepted"))
        .transpose()
        .map_err(unavailable)?
        == Some(true);
    if !already_friends {
        ensure_friend_room(tx, inviter_id).await?;
        ensure_friend_room(tx, peer_id).await?;
    }
    tx.execute_raw(statement(
        "INSERT INTO central_identity.social_contacts (low, high, requested_by, accepted)
         VALUES ($1, $2, $3, true)
         ON CONFLICT (low, high) DO UPDATE SET accepted = true",
        vec![low.into(), high.into(), inviter_id.into()],
    ))
    .await
    .map_err(unavailable)?;
    Ok(())
}

fn decode_summary(row: &QueryResult) -> Result<FriendInviteSummary, Error> {
    let id: Uuid = row.try_get("", "id").map_err(unavailable)?;
    let is_redeemed: bool = row.try_get("", "is_redeemed").map_err(unavailable)?;
    let is_revoked: bool = row.try_get("", "is_revoked").map_err(unavailable)?;
    let is_expired: bool = row.try_get("", "is_expired").map_err(unavailable)?;
    let status = status_from_flags(is_redeemed, is_revoked, is_expired);
    let redeemed_by = if status == FriendInviteStatus::Redeemed {
        let name: Option<String> = row.try_get("", "redeemer_name").map_err(unavailable)?;
        let tag: Option<i32> = row.try_get("", "redeemer_tag").map_err(unavailable)?;
        optional_person(name, tag)?
    } else {
        None
    };
    Ok(FriendInviteSummary {
        id: id.into_bytes(),
        label: row.try_get("", "label").map_err(unavailable)?,
        note: row.try_get("", "note").map_err(unavailable)?,
        invited_email: row.try_get("", "invited_email").map_err(unavailable)?,
        status,
        created_at: row.try_get("", "created_at").map_err(unavailable)?,
        expires_at: row.try_get("", "expires_at").map_err(unavailable)?,
        redeemed_at: row.try_get("", "redeemed_at").map_err(unavailable)?,
        redeemed_by,
    })
}

async fn account_person(db: &impl ConnectionTrait, id: Uuid) -> Result<Person, Error> {
    let row = db
        .query_one_raw(statement(
            "SELECT display_name, tag FROM central_identity.accounts WHERE id = $1",
            vec![id.into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Rejected)?;
    let display_name: Option<String> = row.try_get("", "display_name").map_err(unavailable)?;
    let raw_tag: Option<i32> = row.try_get("", "tag").map_err(unavailable)?;
    optional_person(display_name, raw_tag)?.ok_or(Error::Rejected)
}

async fn list_tx(db: &impl ConnectionTrait, me: Uuid) -> Result<Vec<FriendInviteSummary>, Error> {
    let rows = db
        .query_all_raw(statement(
            "SELECT f.id, f.label, f.note, f.invited_email,
                    floor(extract(epoch FROM f.created_at))::bigint AS created_at,
                    floor(extract(epoch FROM f.expires_at))::bigint AS expires_at,
                    floor(extract(epoch FROM f.redeemed_at))::bigint AS redeemed_at,
                    f.redeemed_at IS NOT NULL AS is_redeemed,
                    f.revoked_at IS NOT NULL AS is_revoked,
                    (f.expires_at <= pg_catalog.clock_timestamp()) AS is_expired,
                    r.display_name AS redeemer_name,
                    r.tag AS redeemer_tag
             FROM central_identity.friend_invites f
             LEFT JOIN central_identity.accounts r ON r.id = f.redeemed_by
             WHERE f.inviter_id = $1
             ORDER BY f.created_at DESC, f.id DESC
             LIMIT 100",
            vec![me.into()],
        ))
        .await
        .map_err(unavailable)?;
    rows.iter().map(decode_summary).collect()
}

pub(crate) async fn account_social_badges(
    db: &impl ConnectionTrait,
    account: Uuid,
) -> Result<Vec<SocialBadge>, Error> {
    let row = db
        .query_one_raw(statement(
            "SELECT COUNT(DISTINCT COALESCE(redeemed_by, id))::bigint AS invited
             FROM central_identity.friend_invites
             WHERE inviter_id = $1 AND redeemed_at IS NOT NULL",
            vec![account.into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Unavailable)?;
    let raw: i64 = row.try_get("", "invited").map_err(unavailable)?;
    let count = u32::try_from(raw.max(0)).unwrap_or(u32::MAX);
    Ok(vec![SocialBadgeKind::Recruiter.progress(count)])
}

async fn create_tx(
    tx: &impl ConnectionTrait,
    session: Session<'_>,
    me: Uuid,
    input: InviteInput<'_>,
) -> Result<(FriendInviteSummary, Zeroizing<String>, Concerns), Error> {
    let label = clean_invite_text(input.label, MAX_INVITE_LABEL)?;
    let note = clean_invite_text(input.note, MAX_INVITE_NOTE)?;
    let email = clean_optional_email(input.email)?;
    let lang = clean_lang(input.lang)?;
    let days = invite_expiry_days(input.expires_days)?;

    lock_accounts(tx, vec![me]).await?;
    caller(tx, session, true).await?;
    super::limits::charge(tx, me, super::limits::Budget::Request).await?;
    ensure_friend_room(tx, me).await?;

    let active_row = tx
        .query_one_raw(statement(
            "SELECT count(*)::bigint AS count
             FROM central_identity.friend_invites
             WHERE inviter_id = $1 AND redeemed_at IS NULL AND revoked_at IS NULL
               AND expires_at > pg_catalog.clock_timestamp()",
            vec![me.into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Unavailable)?;
    let active_count: i64 = active_row.try_get("", "count").map_err(unavailable)?;
    let active_limit = i64::try_from(MAX_ACTIVE_FRIEND_INVITES).map_err(unavailable)?;
    if active_count >= active_limit {
        return Err(Error::Busy);
    }

    let (code, code_hash) = generate_friend_invite()?;
    let row = tx
        .query_one_raw(statement(
            "INSERT INTO central_identity.friend_invites
                 (inviter_id, code_hash, label, note, invited_email, lang, expires_at)
             VALUES ($1, $2, $3, $4, $5, $6,
                 pg_catalog.clock_timestamp() + pg_catalog.make_interval(days => $7))
             RETURNING id,
                       floor(extract(epoch FROM created_at))::bigint AS created_at,
                       floor(extract(epoch FROM expires_at))::bigint AS expires_at",
            vec![
                me.into(),
                code_hash.to_vec().into(),
                label.clone().into(),
                note.clone().into(),
                email.clone().into(),
                lang.into(),
                i32::from(days).into(),
            ],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Unavailable)?;
    let id: Uuid = row.try_get("", "id").map_err(unavailable)?;
    let created_at: i64 = row.try_get("", "created_at").map_err(unavailable)?;
    let expires_at: i64 = row.try_get("", "expires_at").map_err(unavailable)?;
    let summary = FriendInviteSummary {
        id: id.into_bytes(),
        label,
        note,
        invited_email: email,
        status: FriendInviteStatus::Active,
        created_at,
        expires_at,
        redeemed_at: None,
        redeemed_by: None,
    };
    Ok((summary, code, Concerns::social_of([me])))
}

async fn revoke_tx(
    tx: &impl ConnectionTrait,
    session: Session<'_>,
    me: Uuid,
    invite: [u8; 16],
) -> Result<Concerns, Error> {
    lock_accounts(tx, vec![me]).await?;
    caller(tx, session, true).await?;
    let invite_id = Uuid::from_bytes(invite);
    let result = tx
        .execute_raw(statement(
            "UPDATE central_identity.friend_invites
             SET revoked_at = pg_catalog.clock_timestamp(),
                 pending_account_id = NULL,
                 pending_until = NULL
             WHERE id = $1 AND inviter_id = $2
               AND redeemed_at IS NULL AND revoked_at IS NULL
               AND expires_at > pg_catalog.clock_timestamp()",
            vec![invite_id.into(), me.into()],
        ))
        .await
        .map_err(unavailable)?;
    if result.rows_affected() != 1 {
        return Err(Error::Rejected);
    }
    Ok(Concerns::social_of([me]))
}

async fn pending_invite_email(
    tx: &impl ConnectionTrait,
    session: Session<'_>,
    me: Uuid,
    link_email: bool,
    invited_email: Option<String>,
    invite_lang: Option<String>,
    requested_lang: Option<String>,
) -> Result<Option<[String; 2]>, Error> {
    if !link_email {
        return Ok(None);
    }
    let Some(addr) = invited_email.filter(|raw| api_email::address(raw).is_ok()) else {
        return Ok(None);
    };
    let (holder_id, fresh) = super::super::email::holder(tx, session, false).await?;
    if holder_id != me || !fresh {
        return Ok(None);
    }
    let has_confirmed = tx
        .query_one_raw(statement(
            "SELECT 1 FROM central_identity.account_emails WHERE account_id = $1",
            vec![me.into()],
        ))
        .await
        .map_err(unavailable)?
        .is_some();
    if has_confirmed {
        return Ok(None);
    }
    let effective_lang = requested_lang
        .or_else(|| invite_lang.filter(|l| api_email::language(l).is_ok()))
        .unwrap_or_else(|| "en".to_owned());
    Ok(Some([addr, effective_lang]))
}

async fn redeem_tx(
    tx: &impl ConnectionTrait,
    session: Session<'_>,
    me: Uuid,
    code: &str,
    link_email: bool,
    lang: Option<&str>,
) -> Result<(Person, Option<[String; 2]>, Concerns), Error> {
    let code_hash = invite_code_digest(code.trim())?;
    let requested_lang = lang.map(|raw| clean_lang(Some(raw))).transpose()?;

    let inviter_lookup = tx
        .query_one_raw(statement(
            "SELECT inviter_id FROM central_identity.friend_invites WHERE code_hash = $1",
            vec![code_hash.to_vec().into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Rejected)?;
    let inviter_id: Uuid = inviter_lookup
        .try_get("", "inviter_id")
        .map_err(unavailable)?;
    if inviter_id == me {
        return Err(Error::Rejected);
    }

    lock_accounts(tx, vec![inviter_id, me]).await?;
    caller(tx, session, true).await?;
    super::limits::charge(tx, me, super::limits::Budget::Request).await?;
    let me_person = account_person(tx, me).await?;

    let row = tx
        .query_one_raw(statement(
            "SELECT id, invited_email, lang,
                    redeemed_at IS NOT NULL AS is_redeemed,
                    revoked_at IS NOT NULL AS is_revoked,
                    (expires_at <= pg_catalog.clock_timestamp()) AS is_expired,
                    (pending_account_id IS NOT NULL AND pending_until > pg_catalog.clock_timestamp()) AS is_pending
             FROM central_identity.friend_invites
             WHERE code_hash = $1 AND inviter_id = $2
             FOR UPDATE",
            vec![code_hash.to_vec().into(), inviter_id.into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Rejected)?;
    let invite_uuid: Uuid = row.try_get("", "id").map_err(unavailable)?;
    let invited_email: Option<String> = row.try_get("", "invited_email").map_err(unavailable)?;
    let invite_lang: Option<String> = row.try_get("", "lang").map_err(unavailable)?;
    let is_redeemed: bool = row.try_get("", "is_redeemed").map_err(unavailable)?;
    let is_revoked: bool = row.try_get("", "is_revoked").map_err(unavailable)?;
    let is_expired: bool = row.try_get("", "is_expired").map_err(unavailable)?;
    let is_pending: bool = row.try_get("", "is_pending").map_err(unavailable)?;
    if is_redeemed || is_revoked || is_expired || is_pending {
        return Err(Error::Rejected);
    }

    let inviter = account_person(tx, inviter_id).await?;
    make_mutual_friends(tx, inviter_id, me).await?;

    tx.execute_raw(statement(
        "UPDATE central_identity.friend_invites
         SET redeemed_by = $2,
             redeemed_at = pg_catalog.clock_timestamp(),
             pending_account_id = NULL,
             pending_until = NULL
         WHERE id = $1",
        vec![invite_uuid.into(), me.into()],
    ))
    .await
    .map_err(unavailable)?;

    let pending_email = pending_invite_email(
        tx,
        session,
        me,
        link_email,
        invited_email,
        invite_lang,
        requested_lang,
    )
    .await?;
    let concerns = Concerns::friend_accept_invite(
        (inviter_id.into_bytes(), inviter.tag),
        (me.into_bytes(), me_person.tag),
    );
    Ok((inviter, pending_email, concerns))
}

pub(super) async fn dispatch_tx(
    tx: &impl ConnectionTrait,
    session: Session<'_>,
    me: Uuid,
    request: SocialRequest,
) -> Result<(SocialResponse, Concerns), Error> {
    match request {
        SocialRequest::FriendInvites => Ok((
            SocialResponse::FriendInvites {
                invites: list_tx(tx, me).await?,
                active_limit: u32::try_from(MAX_ACTIVE_FRIEND_INVITES).unwrap_or(10),
                mail_available: false,
                recruiter_badge: account_social_badges(tx, me).await?.into_iter().next(),
            },
            Concerns::default(),
        )),
        SocialRequest::CreateFriendInvite {
            label,
            note,
            email,
            lang,
            expires_days,
        } => {
            let (invite, code, concerns) = create_tx(
                tx,
                session,
                me,
                InviteInput {
                    label: label.as_deref(),
                    note: note.as_deref(),
                    email: email.as_deref(),
                    lang: lang.as_deref(),
                    expires_days,
                },
            )
            .await?;
            Ok((
                SocialResponse::FriendInviteCreated {
                    invite,
                    code: code.to_string(),
                    mail_sent: false,
                },
                concerns,
            ))
        }
        SocialRequest::RevokeFriendInvite { invite } => {
            let concerns = revoke_tx(tx, session, me, invite).await?;
            Ok((SocialResponse::Done, concerns))
        }
        SocialRequest::RedeemFriendInvite {
            code,
            link_email,
            lang,
        } => {
            let (inviter, _, concerns) =
                redeem_tx(tx, session, me, &code, link_email, lang.as_deref()).await?;
            Ok((
                SocialResponse::FriendInviteRedeemed {
                    inviter,
                    email_pending: false,
                },
                concerns,
            ))
        }
        _ => Err(Error::Rejected),
    }
}

pub(crate) async fn lock_valid_registration_invite(
    tx: &impl ConnectionTrait,
    code: &str,
) -> Result<Uuid, Error> {
    let code_hash = invite_code_digest(code.trim())?;
    let row = tx
        .query_one_raw(statement(
            "SELECT id
             FROM central_identity.friend_invites
             WHERE code_hash = $1
               AND redeemed_at IS NULL
               AND revoked_at IS NULL
               AND expires_at > pg_catalog.clock_timestamp()
               AND (pending_account_id IS NULL OR pending_until <= pg_catalog.clock_timestamp())
             FOR UPDATE",
            vec![code_hash.to_vec().into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Rejected)?;
    row.try_get("", "id").map_err(unavailable)
}

pub(crate) async fn claim_for_registration(
    tx: &impl ConnectionTrait,
    invite_id: Uuid,
    account_id: Uuid,
) -> Result<(), Error> {
    let result = tx
        .execute_raw(statement(
            "UPDATE central_identity.friend_invites
             SET pending_account_id = $2,
                 pending_until = pg_catalog.clock_timestamp() + interval '10 minutes'
             WHERE id = $1",
            vec![invite_id.into(), account_id.into()],
        ))
        .await
        .map_err(unavailable)?;
    if result.rows_affected() != 1 {
        return Err(Error::Rejected);
    }
    Ok(())
}

pub(crate) async fn finalize_on_enroll(
    tx: &impl ConnectionTrait,
    account_id: Uuid,
    account_tag: Option<i32>,
) -> Result<Concerns, Error> {
    let Some(invite_row) = tx
        .query_one_raw(statement(
            "SELECT id, inviter_id
             FROM central_identity.friend_invites
             WHERE pending_account_id = $1
               AND pending_until > pg_catalog.clock_timestamp()
               AND redeemed_at IS NULL
               AND revoked_at IS NULL
               AND expires_at > pg_catalog.clock_timestamp()
             FOR UPDATE",
            vec![account_id.into()],
        ))
        .await
        .map_err(unavailable)?
    else {
        return Ok(Concerns::default());
    };
    let Some(raw_tag) = account_tag else {
        return Err(Error::Rejected);
    };
    let account_tag = u32::try_from(raw_tag).map_err(unavailable)?;
    let invite_uuid: Uuid = invite_row.try_get("", "id").map_err(unavailable)?;
    let inviter_id: Uuid = invite_row.try_get("", "inviter_id").map_err(unavailable)?;

    let inviter = account_person(tx, inviter_id).await?;
    make_mutual_friends(tx, inviter_id, account_id).await?;
    let updated = tx
        .execute_raw(statement(
            "UPDATE central_identity.friend_invites
             SET redeemed_by = $2,
                 redeemed_at = pg_catalog.clock_timestamp(),
                 pending_account_id = NULL,
                 pending_until = NULL
             WHERE id = $1",
            vec![invite_uuid.into(), account_id.into()],
        ))
        .await
        .map_err(unavailable)?;
    if updated.rows_affected() != 1 {
        return Err(Error::Rejected);
    }
    Ok(Concerns::friend_accept_invite(
        (inviter_id.into_bytes(), inviter.tag),
        (account_id.into_bytes(), account_tag),
    ))
}

impl AccountStore {
    /// Return the signed-in caller's account UUID bytes and public handle.
    ///
    /// # Errors
    /// Rejects invalid sessions or accounts missing a display name / tag.
    pub async fn social_caller(&self, session: Session<'_>) -> Result<([u8; 16], Person), Error> {
        let me = caller(&self.db, session, false).await?;
        let person = account_person(&self.db, me).await?;
        Ok((me.into_bytes(), person))
    }

    /// List friend invites owned by `session` (most recent first).
    ///
    /// # Errors
    /// Rejects invalid sessions or accounts without a public handle; storage failure returns `Unavailable`.
    pub async fn list_friend_invites(
        &self,
        session: Session<'_>,
    ) -> Result<Vec<FriendInviteSummary>, Error> {
        let me = caller(&self.db, session, false).await?;
        list_tx(&self.db, me).await
    }

    /// Current Ingress-style tiered recruiter medal progress for `session`.
    ///
    /// # Errors
    /// Rejects invalid sessions or accounts without a public handle; storage failure returns `Unavailable`.
    pub async fn recruiter_badge(&self, session: Session<'_>) -> Result<SocialBadge, Error> {
        let me = caller(&self.db, session, false).await?;
        let mut badges = account_social_badges(&self.db, me).await?;
        Ok(badges
            .pop()
            .unwrap_or_else(|| SocialBadgeKind::Recruiter.progress(0)))
    }

    /// Create a single-use 128-bit friend invite owned by `session`.
    ///
    /// # Errors
    /// Rejects invalid input or sessions; returns `Busy` above 10 active invites or rate budget.
    pub async fn create_friend_invite(
        &self,
        session: Session<'_>,
        label: Option<&str>,
        note: Option<&str>,
        email: Option<&str>,
        lang: Option<&str>,
        expires_days: Option<u8>,
    ) -> Result<(FriendInviteSummary, Zeroizing<String>, Concerns), Error> {
        let tx = self.db.begin().await.map_err(unavailable)?;
        let me = caller(&tx, session, false).await?;
        let out = create_tx(
            &tx,
            session,
            me,
            InviteInput {
                label,
                note,
                email,
                lang,
                expires_days,
            },
        )
        .await?;
        tx.commit().await.map_err(unavailable)?;
        Ok(out)
    }

    /// Revoke an active friend invite owned by `session`.
    ///
    /// # Errors
    /// Rejects unknown, already-used, expired, or non-owned invites.
    pub async fn revoke_friend_invite(
        &self,
        session: Session<'_>,
        invite: [u8; 16],
    ) -> Result<Concerns, Error> {
        let tx = self.db.begin().await.map_err(unavailable)?;
        let me = caller(&tx, session, false).await?;
        let concerns = revoke_tx(&tx, session, me, invite).await?;
        tx.commit().await.map_err(unavailable)?;
        Ok(concerns)
    }

    /// Redeem an active friend invite atomically, making inviter and caller mutual friends.
    /// Returns `(inviter, optional [pending_email, effective_lang], concerns)`.
    ///
    /// # Errors
    /// Rejects invalid, used, expired, revoked, self-owned, or blocked invites; returns `Busy` at friend limit.
    pub async fn redeem_friend_invite(
        &self,
        session: Session<'_>,
        code: &str,
        link_email: bool,
        lang: Option<&str>,
    ) -> Result<(Person, Option<[String; 2]>, Concerns), Error> {
        let tx = self.db.begin().await.map_err(unavailable)?;
        let me = caller(&tx, session, false).await?;
        let out = redeem_tx(&tx, session, me, code, link_email, lang).await?;
        tx.commit().await.map_err(unavailable)?;
        Ok(out)
    }

    /// Public capability-scoped preview for a friend invite code.
    ///
    /// Omits `invited_email` and `redeemed_by` to avoid exposing recipient PII
    /// or private accounts unauthenticated, and rejects active invites with an
    /// in-flight registration reservation (`pending_account_id` before expiry).
    ///
    /// # Errors
    /// Rejects malformed, unknown, or reserved codes; storage failures return `Unavailable`.
    pub async fn friend_invite_info(&self, code: &str) -> Result<FriendInvitePreview, Error> {
        let code_hash = invite_code_digest(code.trim())?;
        let row = self
            .db
            .query_one_raw(statement(
                "SELECT
                     (SELECT open FROM central_identity.registration LIMIT 1) AS registration_open,
                     a.display_name AS inviter_name,
                     a.tag AS inviter_tag,
                     f.note,
                     floor(extract(epoch FROM f.created_at))::bigint AS created_at,
                     floor(extract(epoch FROM f.expires_at))::bigint AS expires_at,
                     f.redeemed_at IS NOT NULL AS is_redeemed,
                     f.revoked_at IS NOT NULL AS is_revoked,
                     (f.expires_at <= pg_catalog.clock_timestamp()) AS is_expired,
                     (f.pending_account_id IS NOT NULL AND f.pending_until > pg_catalog.clock_timestamp()) AS is_pending
                 FROM central_identity.friend_invites f
                 JOIN central_identity.accounts a ON a.id = f.inviter_id
                 WHERE f.code_hash = $1",
                vec![code_hash.to_vec().into()],
            ))
            .await
            .map_err(unavailable)?
            .ok_or(Error::Rejected)?;

        let registration_open: bool = row.try_get("", "registration_open").map_err(unavailable)?;
        let inviter_name: Option<String> = row.try_get("", "inviter_name").map_err(unavailable)?;
        let inviter_tag: Option<i32> = row.try_get("", "inviter_tag").map_err(unavailable)?;
        let inviter = optional_person(inviter_name, inviter_tag)?.ok_or(Error::Rejected)?;
        let is_redeemed: bool = row.try_get("", "is_redeemed").map_err(unavailable)?;
        let is_revoked: bool = row.try_get("", "is_revoked").map_err(unavailable)?;
        let is_expired: bool = row.try_get("", "is_expired").map_err(unavailable)?;
        let is_pending: bool = row.try_get("", "is_pending").map_err(unavailable)?;
        let status = status_from_flags(is_redeemed, is_revoked, is_expired);
        if status == FriendInviteStatus::Active && is_pending {
            return Err(Error::Rejected);
        }

        let note = if status == FriendInviteStatus::Active {
            row.try_get("", "note").map_err(unavailable)?
        } else {
            None
        };

        Ok(FriendInvitePreview {
            registration_open,
            invite_only: !registration_open,
            inviter,
            note,
            invited_email: None,
            status,
            created_at: row.try_get("", "created_at").map_err(unavailable)?,
            expires_at: row.try_get("", "expires_at").map_err(unavailable)?,
            redeemed_by: None,
        })
    }

    /// Returns `true` when `code` identifies a currently active, unreserved,
    /// unexpired friend invite eligible for invite-only registration.
    ///
    /// # Errors
    /// Storage failures return `Unavailable`. Invalid/unknown codes return `Ok(false)`.
    pub async fn invite_can_register(&self, code: &str) -> Result<bool, Error> {
        let Ok(code_hash) = invite_code_digest(code.trim()) else {
            return Ok(false);
        };
        self.db
            .query_one_raw(statement(
                "SELECT 1
                 FROM central_identity.friend_invites
                 WHERE code_hash = $1
                   AND redeemed_at IS NULL
                   AND revoked_at IS NULL
                   AND expires_at > pg_catalog.clock_timestamp()
                   AND (pending_account_id IS NULL OR pending_until <= pg_catalog.clock_timestamp())",
                vec![code_hash.to_vec().into()],
            ))
            .await
            .map_err(unavailable)
            .map(|row| row.is_some())
    }

    /// Store a pending email confirmation digest for `session`'s account ONLY
    /// when the session's password sign-in is still fresh
    /// ([`super::super::email::EMAIL_FRESH_MINUTES`]) and `account_emails` still
    /// has no confirmed address (safe against races).
    ///
    /// # Errors
    /// Storage failures or invalid inputs.
    pub async fn save_invite_email_pending(
        &self,
        session: Session<'_>,
        address: &str,
        lang: &str,
        code_digest: [u8; 32],
    ) -> Result<bool, Error> {
        api_email::address(address)?;
        api_email::language(lang)?;
        let tx = self.db.begin().await.map_err(unavailable)?;
        let (account, fresh) = super::super::email::holder(&tx, session, true).await?;
        if !fresh {
            return Ok(false);
        }
        let res = tx
            .execute_raw(statement(
                "INSERT INTO central_identity.account_email_pending
                     (account_id, address, lang, code_digest, expires_at)
                 SELECT $1, $2, $3, $4,
                     pg_catalog.clock_timestamp() + pg_catalog.make_interval(mins => $5)
                 WHERE NOT EXISTS (
                     SELECT 1 FROM central_identity.account_emails WHERE account_id = $1
                 )
                 ON CONFLICT (account_id) DO UPDATE
                 SET address = EXCLUDED.address,
                     lang = EXCLUDED.lang,
                     code_digest = EXCLUDED.code_digest,
                     expires_at = EXCLUDED.expires_at,
                     tries = 0",
                vec![
                    account.into(),
                    address.into(),
                    lang.into(),
                    code_digest.to_vec().into(),
                    MAIL_CODE_MINUTES.into(),
                ],
            ))
            .await
            .map_err(unavailable)?;
        tx.commit().await.map_err(unavailable)?;
        Ok(res.rows_affected() == 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_optional_email_and_lang_validate_inputs() {
        assert_eq!(clean_optional_email(None), Ok(None));
        assert_eq!(clean_optional_email(Some("   ")), Ok(None));
        assert_eq!(
            clean_optional_email(Some(" player@example.org ")),
            Ok(Some("player@example.org".to_owned()))
        );
        assert_eq!(
            clean_optional_email(Some("not-an-email")),
            Err(Error::Rejected)
        );
        assert_eq!(clean_lang(None), Ok("en".to_owned()));
        assert_eq!(clean_lang(Some(" pt-br ")), Ok("pt-br".to_owned()));
        assert_eq!(clean_lang(Some("INVALID")), Err(Error::Rejected));
    }

    #[test]
    fn status_precedence_and_optional_person_hold() {
        assert_eq!(
            status_from_flags(true, false, true),
            FriendInviteStatus::Redeemed
        );
        assert_eq!(
            status_from_flags(false, true, true),
            FriendInviteStatus::Revoked
        );
        assert_eq!(
            status_from_flags(false, false, true),
            FriendInviteStatus::Expired
        );
        assert_eq!(
            status_from_flags(false, false, false),
            FriendInviteStatus::Active
        );
        assert_eq!(
            optional_person(Some("Alice".into()), Some(42)).unwrap(),
            Some(Person {
                tag: 42,
                display_name: "Alice".into()
            })
        );
        assert_eq!(optional_person(None, Some(42)).unwrap(), None);
        assert_eq!(optional_person(Some("Alice".into()), None).unwrap(), None);
    }
}
