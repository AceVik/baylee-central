//! AI players (`docs/ai-players.md`): the accounts an administrator creates
//! as AIs, the settings for every AI, and the badge every reply naming one
//! carries.
use super::{AccountStore, Action, Error, Token, statement};
use central_api::admin::{AI_PLAYERS, AdminAiPlayer, AiPlayerSettings, AiSettings};
use central_api::ranked::ai::{AiBadge, AiKind};
use sea_orm::{ConnectionTrait, DatabaseTransaction, QueryResult, prelude::Uuid};

mod lifecycle;

/// An `accounts` row, named `accounts` in its statement, that is no AI
/// player: the condition the sessionless flows put on the account they
/// name (sign-in, discovery, every reset), so that an AI's login name and
/// credential answer as an unknown name's (`docs/ai-players.md` §2).
pub(crate) const NO_AI: &str = "NOT EXISTS (SELECT 1 FROM central_identity.ai_players ai
    WHERE ai.account_id = accounts.id)";

fn unavailable(_: impl Sized) -> Error {
    Error::Unavailable
}

fn get<T: sea_orm::TryGetable>(row: &QueryResult, name: &str) -> Result<T, Error> {
    row.try_get("", name).map_err(unavailable)
}

/// The columns a query names `ai_kind` and `ai_label` (each NULL for a
/// person) as a badge.
pub(crate) fn badge_of(row: &QueryResult) -> Result<Option<AiBadge>, Error> {
    let Some(kind) = get::<Option<String>>(row, "ai_kind")? else {
        return Ok(None);
    };
    Ok(Some(AiBadge {
        kind: AiKind::from_key(&kind).ok_or(Error::Unavailable)?,
        label: get(row, "ai_label")?,
    }))
}

/// The badge of `account`, none for a person.
pub(crate) async fn badge(
    db: &impl ConnectionTrait,
    account: Uuid,
) -> Result<Option<AiBadge>, Error> {
    match db
        .query_one_raw(statement(
            "SELECT kind AS ai_kind, label AS ai_label
             FROM central_identity.ai_players WHERE account_id = $1",
            vec![account.into()],
        ))
        .await
        .map_err(unavailable)?
    {
        Some(row) => badge_of(&row),
        None => Ok(None),
    }
}

/// Whether `account` is an AI player, switched on or off.
pub(crate) async fn flagged(db: &impl ConnectionTrait, account: Uuid) -> Result<bool, Error> {
    Ok(db
        .query_one_raw(statement(
            "SELECT 1 AS one FROM central_identity.ai_players WHERE account_id = $1",
            vec![account.into()],
        ))
        .await
        .map_err(unavailable)?
        .is_some())
}

/// The start of today, UTC, in SQL.
pub(crate) const TODAY: &str = "pg_catalog.date_trunc('day', pg_catalog.clock_timestamp(), 'UTC')";

/// A match that holds a slot: reserved or playing, its lease not run out.
pub(crate) const LIVE: &str =
    "m.state IN ('reserved','playing') AND m.lease_until > pg_catalog.clock_timestamp()";

fn settings_of(row: &QueryResult) -> Result<AiSettings, Error> {
    let small =
        |name| -> Result<u8, Error> { u8::try_from(get::<i16>(row, name)?).map_err(unavailable) };
    Ok(AiSettings {
        fill_in_after: u16::try_from(get::<i16>(row, "fill_in_after")?).map_err(unavailable)?,
        drain_minutes: u16::try_from(get::<i16>(row, "drain_minutes")?).map_err(unavailable)?,
        global_seats: small("global_seats")?,
        host_reserve: small("host_reserve")?,
        carousel_games: small("carousel_games")?,
        carousel_daily: u32::try_from(get::<i32>(row, "carousel_daily")?).map_err(unavailable)?,
    })
}

/// The settings for every AI, read in `db`.
pub(crate) async fn settings(db: &impl ConnectionTrait, lock: bool) -> Result<AiSettings, Error> {
    let row = db
        .query_one_raw(statement(
            if lock {
                "SELECT * FROM central_identity.ai_settings FOR UPDATE"
            } else {
                "SELECT * FROM central_identity.ai_settings"
            },
            vec![],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Unavailable)?;
    settings_of(&row)
}

async fn audit(
    tx: &DatabaseTransaction,
    actor: Uuid,
    action: Action,
    account: Option<Uuid>,
    detail: Option<String>,
) -> Result<(), Error> {
    tx.execute_raw(statement(
        "INSERT INTO central_identity.audit_events (actor_kind, actor_account, action, account_id, detail)
         VALUES ('admin', $1, $2, $3, $4::jsonb)",
        vec![actor.into(), action.as_str().into(), account.into(), detail.into()],
    ))
    .await
    .map_err(unavailable)?;
    Ok(())
}

/// Whether the account has ever been seated or holds a ticket that is not
/// closed: after that an AI stays for the account's life and is switched off
/// instead of removed (`docs/ai-players.md` section 1).
async fn has_played(tx: &DatabaseTransaction, account: Uuid) -> Result<bool, Error> {
    Ok(tx
        .query_one_raw(statement(
            "SELECT 1 AS one WHERE EXISTS (SELECT 1 FROM central_identity.ranked_matches
               WHERE account_a = $1 OR account_b = $1)
             OR EXISTS (SELECT 1 FROM central_identity.ranked_tickets
               WHERE account_id = $1 AND state <> 'closed')",
            vec![account.into()],
        ))
        .await
        .map_err(unavailable)?
        .is_some())
}

fn player(row: &QueryResult) -> Result<AdminAiPlayer, Error> {
    let (display_name, tag) = super::profiles::columns(row)?;
    let kind: String = get(row, "kind")?;
    Ok(AdminAiPlayer {
        account: get::<Uuid>(row, "account_id")?.into_bytes(),
        settings: AiPlayerSettings {
            kind: AiKind::from_key(&kind).ok_or(Error::Unavailable)?,
            profile: get(row, "profile")?,
            label: get(row, "label")?,
            max_games: u8::try_from(get::<i16>(row, "max_games")?).map_err(unavailable)?,
            daily_games: get::<Option<i32>>(row, "daily_games")?
                .map(u32::try_from)
                .transpose()
                .map_err(unavailable)?,
            display_name: display_name.clone().unwrap_or_default(),
        },
        display_name,
        tag,
        enabled: get(row, "enabled")?,
        live_matches: u32::try_from(get::<i64>(row, "live")?).map_err(unavailable)?,
        games_today: u32::try_from(get::<i64>(row, "today")?).map_err(unavailable)?,
        offers: get::<Vec<String>>(row, "offers")?
            .iter()
            .map(|format| format.parse().map_err(unavailable))
            .collect::<Result<_, _>>()?,
        last_sign_in: get(row, "last_sign_in")?,
        set_at: get(row, "set_at")?,
    })
}

impl AccountStore {
    /// Every AI player, oldest first, for the administrator behind `browser`.
    ///
    /// # Errors
    /// `Rejected` without an administrator's live browser session; storage failure.
    pub async fn admin_ai_players(&self, browser: &Token) -> Result<Vec<AdminAiPlayer>, Error> {
        Self::administrator(&self.db, browser).await?;
        let rows = self
            .db
            .query_all_raw(statement(
                &format!(
                    "SELECT p.*, a.display_name, a.tag,
                       floor(extract(epoch FROM p.set_at))::bigint AS set_at,
                       floor(extract(epoch FROM a.last_sign_in_at))::bigint AS last_sign_in,
                       (SELECT count(*) FROM central_identity.ranked_matches m
                         WHERE p.account_id IN (m.account_a, m.account_b) AND {LIVE}) AS live,
                       (SELECT count(*) FROM central_identity.ai_matches am
                         JOIN central_identity.ranked_matches m ON m.id = am.match_id
                         WHERE p.account_id IN (m.account_a, m.account_b)
                         AND am.created_at >= {TODAY}) AS today,
                       ARRAY(SELECT o.format FROM central_identity.ai_offers o
                         WHERE o.account_id = p.account_id
                         AND o.expires_at > pg_catalog.clock_timestamp() ORDER BY o.format) AS offers
                     FROM central_identity.ai_players p
                     JOIN central_identity.accounts a ON a.id = p.account_id
                     ORDER BY p.set_at, p.account_id LIMIT {AI_PLAYERS}"
                ),
                vec![],
            ))
            .await
            .map_err(unavailable)?;
        rows.iter().map(player).collect()
    }

    /// Switch an AI player on or off. Switching off takes it out of every new
    /// match from this commit on (fill-in locks the same row) and withdraws
    /// its offers; running matches end normally. Only switching on needs a
    /// fresh sign-in. Audited when it changes.
    ///
    /// # Errors
    /// `Rejected` without an administrator (a fresh one to switch on) or for a
    /// person; storage failure.
    pub async fn set_ai_enabled(
        &self,
        browser: &Token,
        account: [u8; 16],
        enabled: bool,
    ) -> Result<(), Error> {
        let account = Uuid::from_bytes(account);
        let (tx, actor) = self.acting_administrator_fresh(browser, enabled).await?;
        let before: bool = get(
            &tx.query_one_raw(statement(
                "SELECT enabled FROM central_identity.ai_players WHERE account_id = $1 FOR UPDATE",
                vec![account.into()],
            ))
            .await
            .map_err(unavailable)?
            .ok_or(Error::Rejected)?,
            "enabled",
        )?;
        if before == enabled {
            return tx.commit().await.map_err(unavailable);
        }
        tx.execute_raw(statement(
            "UPDATE central_identity.ai_players SET enabled = $2,
               changed_at = pg_catalog.clock_timestamp(), changed_by = $3 WHERE account_id = $1",
            vec![account.into(), enabled.into(), actor.into()],
        ))
        .await
        .map_err(unavailable)?;
        if !enabled {
            tx.execute_raw(statement(
                "DELETE FROM central_identity.ai_offers WHERE account_id = $1",
                vec![account.into()],
            ))
            .await
            .map_err(unavailable)?;
        }
        let action = if enabled {
            Action::AiEnabled
        } else {
            Action::AiDisabled
        };
        audit(&tx, actor, action, Some(account), None).await?;
        tx.commit().await.map_err(unavailable)
    }

    /// The settings for every AI and when they last changed.
    ///
    /// # Errors
    /// `Rejected` without an administrator's live browser session; storage failure.
    pub async fn admin_ai_settings(
        &self,
        browser: &Token,
    ) -> Result<(AiSettings, Option<i64>), Error> {
        Self::administrator(&self.db, browser).await?;
        let row = self
            .db
            .query_one_raw(statement(
                "SELECT *, floor(extract(epoch FROM changed_at))::bigint AS changed
                 FROM central_identity.ai_settings",
                vec![],
            ))
            .await
            .map_err(unavailable)?
            .ok_or(Error::Unavailable)?;
        Ok((settings_of(&row)?, get(&row, "changed")?))
    }

    /// Change the settings for every AI (`stop`: only `global_seats` to 0,
    /// which needs no fresh sign-in). Audited with the new numbers.
    ///
    /// # Errors
    /// `Rejected` without an administrator (a fresh one unless stopping) or
    /// for values out of range; storage failure.
    pub async fn set_ai_settings(
        &self,
        browser: &Token,
        new: Option<&AiSettings>,
    ) -> Result<(), Error> {
        if new.is_some_and(|s| !s.valid()) {
            return Err(Error::Rejected);
        }
        let (tx, actor) = self
            .acting_administrator_fresh(browser, new.is_some())
            .await?;
        let mut next = settings(&tx, true).await?;
        if let Some(new) = new {
            next = *new;
        } else {
            next.global_seats = 0;
        }
        tx.execute_raw(statement(
            "UPDATE central_identity.ai_settings SET fill_in_after = $1, drain_minutes = $2,
               global_seats = $3, host_reserve = $4, carousel_games = $5, carousel_daily = $6,
               changed_at = pg_catalog.clock_timestamp(), changed_by = $7",
            vec![
                i16::try_from(next.fill_in_after)
                    .map_err(|_| Error::Rejected)?
                    .into(),
                i16::try_from(next.drain_minutes)
                    .map_err(|_| Error::Rejected)?
                    .into(),
                i16::from(next.global_seats).into(),
                i16::from(next.host_reserve).into(),
                i16::from(next.carousel_games).into(),
                i32::try_from(next.carousel_daily)
                    .map_err(|_| Error::Rejected)?
                    .into(),
                actor.into(),
            ],
        ))
        .await
        .map_err(unavailable)?;
        let detail = format!(
            "{{\"fill_in_after\":{},\"drain_minutes\":{},\"global_seats\":{},\"host_reserve\":{},\"carousel_games\":{},\"carousel_daily\":{}}}",
            next.fill_in_after,
            next.drain_minutes,
            next.global_seats,
            next.host_reserve,
            next.carousel_games,
            next.carousel_daily
        );
        audit(&tx, actor, Action::AiSettingsChanged, None, Some(detail)).await?;
        tx.commit().await.map_err(unavailable)
    }
}
