//! The operator's command for AI players (`central-service
//! --ai-players-stdin`, `docs/operations.md` §"AI players"): what the
//! administration's AI page does, for an operator who cannot use the
//! website as an administrator, under the migrator's login. Every change
//! takes the same functions the administration takes ([`super::lifecycle`],
//! [`super::switch`], [`super::write_settings`], the runner tokens' `make`
//! and `revoke`), with the operator as the audit's actor, in one
//! transaction: a plan is applied whole or not at all.
//!
//! The order of the locks is the administration's and fill-in's: every
//! administrator's row (so a token's caps are counted as an administrator
//! counts them), the settings for every AI, the tokens to revoke (as an
//! exchange takes its token first), the AI accounts and their rows, the
//! revoked tokens' sessions, and a new token last.
use super::lifecycle::{change, create, remove};
use super::{get, settings, switch, unavailable, write_settings};
use crate::accounts::ai_runner::{listed, make, revoke};
use crate::accounts::{Error, Token, statement, uuid};
use central_api::admin::{AiPlayerSettings, AiSettings};
use central_api::ai_runner::{AiRunnerTokenInfo, LIVE_TOKENS, TOKENS_PER_DAY};
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DatabaseTransaction, QueryResult, TransactionTrait,
    prelude::Uuid,
};

mod faults;
pub use faults::plan_faults;

/// One AI player as the operator wants it. Its tag and display name name
/// it: an AI with both is the one meant, and none with either is made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiPlayerPlan {
    /// Its settings, the display name among them.
    pub settings: AiPlayerSettings,
    /// Its tag.
    pub tag: u32,
    /// Switch it on or off, or leave it as it is.
    pub enabled: Option<bool>,
}

/// The settings for every AI the operator changes; the rest stay.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AiSettingsPlan {
    /// Whether fill-in runs (off for calibration).
    pub fill_in: Option<bool>,
    /// Seconds before a fill-in.
    pub fill_in_after: Option<u16>,
    /// Minutes a runner session must have left.
    pub drain_minutes: Option<u16>,
    /// AI seats at once.
    pub global_seats: Option<u8>,
    /// Slots each host keeps for people.
    pub host_reserve: Option<u8>,
    /// AI-against-AI matches at once.
    pub carousel_games: Option<u8>,
    /// AI-against-AI matches a UTC day.
    pub carousel_daily: Option<u32>,
}

impl AiSettingsPlan {
    fn over(self, held: AiSettings) -> AiSettings {
        AiSettings {
            fill_in: self.fill_in.unwrap_or(held.fill_in),
            fill_in_after: self.fill_in_after.unwrap_or(held.fill_in_after),
            drain_minutes: self.drain_minutes.unwrap_or(held.drain_minutes),
            global_seats: self.global_seats.unwrap_or(held.global_seats),
            host_reserve: self.host_reserve.unwrap_or(held.host_reserve),
            carousel_games: self.carousel_games.unwrap_or(held.carousel_games),
            carousel_daily: self.carousel_daily.unwrap_or(held.carousel_daily),
        }
    }
}

/// A runner token to make for one runner machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunnerTokenPlan {
    /// Its name: the machine's.
    pub name: String,
    /// The AI players it lists, by display name; each must name exactly one.
    pub accounts: Vec<String>,
    /// How many days it lives.
    pub days: u16,
}

/// What the operator asks, applied whole or not at all.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AiPlan {
    /// The AI players, made or brought to these settings.
    pub players: Vec<AiPlayerPlan>,
    /// The settings for every AI.
    pub settings: Option<AiSettingsPlan>,
    /// Runner tokens to revoke, by id, before a new one is made.
    pub revoke: Vec<[u8; 16]>,
    /// AI players to remove, which must never have played.
    pub remove: Vec<RemoveTarget>,
    /// A runner token to make.
    pub token: Option<RunnerTokenPlan>,
}

/// An AI player the plan removes: by its id, or by its display name and
/// tag (`KI-GPT-Astra#1b66`).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RemoveTarget {
    /// Its account id.
    Id([u8; 16]),
    /// Its display name and tag.
    Named {
        /// The display name.
        display_name: String,
        /// The tag.
        tag: u32,
    },
}

impl std::fmt::Display for RemoveTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Id(id) => write!(f, "{}", Uuid::from_bytes(*id)),
            Self::Named { display_name, tag } => write!(f, "{display_name}#{tag:x}"),
        }
    }
}

/// An AI player the plan removed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemovedApplied {
    /// Its account, gone now.
    pub account: [u8; 16],
    /// Its display name, as it was.
    pub display_name: String,
    /// Its tag, as it was.
    pub tag: u32,
    /// How many of its sessions ended with it.
    pub sessions: usize,
}

/// What became of one AI player of the plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerOutcome {
    /// Made now, switched off unless the plan switched it on.
    Created,
    /// It was there with these settings.
    Unchanged,
    /// It was there; its label or limits changed.
    Changed,
}

/// One AI player as the plan left it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerApplied {
    /// Its account.
    pub account: [u8; 16],
    /// Its display name.
    pub display_name: String,
    /// Its tag.
    pub tag: u32,
    /// What became of it.
    pub outcome: PlayerOutcome,
    /// Whether it was switched on or off now.
    pub switched: bool,
}

/// What the plan did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AiApplied {
    /// The AI players, in the plan's order.
    pub players: Vec<PlayerApplied>,
    /// The settings for every AI as written, when the plan changed them.
    pub settings: Option<AiSettings>,
    /// The tokens revoked now, each with how many sessions ended with it.
    pub revoked: Vec<([u8; 16], usize)>,
    /// The AI players removed now, in the plan's order.
    pub removed: Vec<RemovedApplied>,
    /// The new token's id and expiry, in Unix seconds.
    pub token: Option<([u8; 16], i64)>,
}

/// Why the operator's plan was refused: the error the store would answer,
/// and, for the operator's standard error alone, what was wrong and where
/// (`players[3].display_name "KI-DeepSeek-Flash": 17 characters, at most
/// 16`). It names no secret; the network-facing requests keep their one
/// `Rejected`.
#[derive(Debug, PartialEq, Eq)]
pub struct PlanFault {
    /// The error.
    pub error: Error,
    /// What was wrong, each in its place; empty for a storage failure.
    pub faults: Vec<String>,
}

impl PlanFault {
    fn at(error: Error, fault: String) -> Self {
        Self {
            error,
            faults: vec![fault],
        }
    }
}

/// A store failure, which says nothing of the plan.
impl From<Error> for PlanFault {
    fn from(error: Error) -> Self {
        let fault = match error {
            Error::Unavailable => "the database failed or could not be reached".to_owned(),
            other => format!("refused ({other:?})"),
        };
        Self::at(error, fault)
    }
}

/// The settings an AI holds now, from `ai_players p` joined to `accounts a`.
fn held(row: &QueryResult) -> Result<(String, String, String, i16, Option<i32>), Error> {
    Ok((
        get(row, "kind")?,
        get(row, "profile")?,
        get(row, "label")?,
        get(row, "max_games")?,
        get(row, "daily_games")?,
    ))
}

/// Bring one AI player, the plan's `index`th, to the plan: made when
/// neither its tag nor its name is taken, changed when an AI holds both and
/// its label or limits differ. Anything else is ambiguous and refused,
/// saying which: a tag a person holds or an AI of another name, a name
/// another AI holds under another tag, another kind or profile.
async fn player(
    tx: &DatabaseTransaction,
    index: usize,
    wanted: &AiPlayerPlan,
) -> Result<PlayerApplied, PlanFault> {
    let settings = &wanted.settings;
    let at = format!(
        "players[{index}] {:?}#{:x}",
        settings.display_name, wanted.tag
    );
    let refused = |error, why: &str| PlanFault::at(error, format!("{at}: {why}"));
    let tag =
        i32::try_from(wanted.tag).map_err(|_| refused(Error::Rejected, "tag out of range"))?;
    let found = tx
        .query_all_raw(statement(
            "SELECT a.id, a.display_name, a.tag, p.kind, p.profile, p.label, p.max_games,
                 p.daily_games, p.account_id IS NOT NULL AS ai
             FROM central_identity.accounts a
             LEFT JOIN central_identity.ai_players p ON p.account_id = a.id
             WHERE a.tag = $1 OR (p.account_id IS NOT NULL AND a.display_name = $2)
             ORDER BY a.id",
            vec![tag.into(), settings.display_name.clone().into()],
        ))
        .await
        .map_err(unavailable)?;
    let (account, outcome) = match found.as_slice() {
        [] => {
            let (account, _) = create(tx, None, settings, wanted.tag)
                .await
                .map_err(|error| match error {
                    Error::Conflict => refused(error, "another account took the tag meanwhile"),
                    other => other.into(),
                })?;
            (Uuid::from_bytes(account), PlayerOutcome::Created)
        }
        [row] => {
            let ai: bool = get(row, "ai")?;
            let held_tag: Option<i32> = get(row, "tag")?;
            let held_name: Option<String> = get(row, "display_name")?;
            if !ai {
                return Err(refused(Error::Conflict, "the tag is a person's"));
            }
            if held_tag != Some(tag) {
                let held_tag = held_tag.map_or_else(|| "none".to_owned(), |t| format!("#{t:x}"));
                return Err(refused(
                    Error::Conflict,
                    &format!("an AI player of that name has the tag {held_tag}"),
                ));
            }
            if held_name.as_deref() != Some(settings.display_name.as_str()) {
                return Err(refused(
                    Error::Conflict,
                    &format!(
                        "the tag is AI player {:?}'s; a name is changed on the website",
                        held_name.unwrap_or_default()
                    ),
                ));
            }
            let account: Uuid = get(row, "id")?;
            let (kind, profile, label, max_games, daily) = held(row)?;
            if Some(kind.as_str()) != settings.kind.as_str() || profile != settings.profile {
                return Err(refused(
                    Error::Conflict,
                    &format!(
                        "it plays as {kind} {profile:?}; a mind is fixed, a new one is a new AI player"
                    ),
                ));
            }
            let daily = daily.map(u32::try_from).transpose().map_err(unavailable)?;
            if label == settings.label
                && max_games == i16::from(settings.max_games)
                && daily == settings.daily_games
            {
                (account, PlayerOutcome::Unchanged)
            } else {
                change(tx, None, account, settings).await?;
                (account, PlayerOutcome::Changed)
            }
        }
        _ => {
            return Err(refused(
                Error::Conflict,
                "the tag and the name belong to two accounts",
            ));
        }
    };
    let switched = match wanted.enabled {
        Some(enabled) => switch(tx, None, account, enabled).await?,
        None => false,
    };
    Ok(PlayerApplied {
        account: account.into_bytes(),
        display_name: settings.display_name.clone(),
        tag: wanted.tag,
        outcome,
        switched,
    })
}

/// The AI player a removal names, with its display name and tag as they
/// are; none when no AI player is it.
async fn target(
    tx: &DatabaseTransaction,
    wanted: &RemoveTarget,
) -> Result<Option<(Uuid, String, u32)>, PlanFault> {
    let row = match wanted {
        RemoveTarget::Id(id) => {
            tx.query_one_raw(statement(
                "SELECT a.id, a.display_name, a.tag FROM central_identity.accounts a
                 JOIN central_identity.ai_players p ON p.account_id = a.id WHERE a.id = $1",
                vec![Uuid::from_bytes(*id).into()],
            ))
            .await
        }
        RemoveTarget::Named { display_name, tag } => {
            let Ok(tag) = i32::try_from(*tag) else {
                return Ok(None);
            };
            tx.query_one_raw(statement(
                "SELECT a.id, a.display_name, a.tag FROM central_identity.accounts a
                 JOIN central_identity.ai_players p ON p.account_id = a.id
                 WHERE a.display_name = $1 AND a.tag = $2",
                vec![display_name.clone().into(), tag.into()],
            ))
            .await
        }
    }
    .map_err(unavailable)?;
    let Some(row) = row else {
        return Ok(None);
    };
    let name: Option<String> = get(&row, "display_name")?;
    let tag: Option<i32> = get(&row, "tag")?;
    Ok(Some((
        get(&row, "id")?,
        name.unwrap_or_default(),
        tag.and_then(|tag| u32::try_from(tag).ok()).unwrap_or(0),
    )))
}

/// How many ranked matches `account` has been seated in, and whether it
/// holds a ticket that is not closed: what keeps an AI from going.
async fn played(tx: &DatabaseTransaction, account: Uuid) -> Result<(i64, bool), PlanFault> {
    let row = tx
        .query_one_raw(statement(
            "SELECT (SELECT count(*) FROM central_identity.ranked_matches
                     WHERE account_a = $1 OR account_b = $1)::bigint AS matches,
                    EXISTS (SELECT 1 FROM central_identity.ranked_tickets
                     WHERE account_id = $1 AND state <> 'closed') AS ticket",
            vec![account.into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Unavailable)?;
    Ok((get(&row, "matches")?, get(&row, "ticket")?))
}

/// Remove the plan's AI players, after every removal's faults are known and
/// said at once: one no AI player is, one named twice, one that has played.
/// Each goes through the administration's own removal ([`remove`]), with
/// the operator as the audit's actor.
async fn removals(
    tx: &DatabaseTransaction,
    plan: &AiPlan,
) -> Result<Vec<RemovedApplied>, PlanFault> {
    let mut faults = Vec::new();
    let mut error = Error::Rejected;
    let mut found: Vec<(usize, Uuid, String, u32)> = Vec::new();
    for (index, wanted) in plan.remove.iter().enumerate() {
        let Some((account, name, tag)) = target(tx, wanted).await? else {
            faults.push(match wanted {
                RemoveTarget::Id(_) => {
                    format!("remove[{index}] {wanted}: no AI player has that id")
                }
                RemoveTarget::Named { .. } => {
                    format!("remove[{index}] \"{wanted}\": no AI player has that name and tag")
                }
            });
            continue;
        };
        if let Some((before, ..)) = found.iter().find(|(_, held, ..)| *held == account) {
            faults.push(format!(
                "remove[{index}] \"{name}#{tag:x}\": the same AI player as remove[{before}]"
            ));
            continue;
        }
        let (matches, ticket) = played(tx, account).await?;
        if matches > 0 || ticket {
            error = Error::Conflict;
            faults.push(if matches > 0 {
                format!(
                    "remove[{index}] \"{name}#{tag:x}\": has played {matches} matches, cannot be removed; switch it off instead"
                )
            } else {
                format!(
                    "remove[{index}] \"{name}#{tag:x}\": holds an open ranked ticket, cannot be removed; switch it off instead"
                )
            });
        }
        found.push((index, account, name, tag));
    }
    if !faults.is_empty() {
        return Err(PlanFault { error, faults });
    }
    let mut removed = Vec::with_capacity(found.len());
    for (index, account, display_name, tag) in found {
        let ended = remove(tx, None, account)
            .await
            .map_err(|error| match error {
                Error::Conflict => PlanFault::at(
                    error,
                    format!(
                        "remove[{index}] \"{display_name}#{tag:x}\": was seated meanwhile, cannot be removed; switch it off instead"
                    ),
                ),
                other => other.into(),
            })?;
        removed.push(RemovedApplied {
            account: account.into_bytes(),
            display_name,
            tag,
            sessions: ended.len(),
        });
    }
    Ok(removed)
}

/// The one AI player named `name`, the token's `index`th account; none or
/// several is refused, saying which.
async fn named(tx: &DatabaseTransaction, index: usize, name: &str) -> Result<[u8; 16], PlanFault> {
    let rows = tx
        .query_all_raw(statement(
            "SELECT a.id FROM central_identity.accounts a
             JOIN central_identity.ai_players p ON p.account_id = a.id
             WHERE a.display_name = $1 LIMIT 2",
            vec![name.into()],
        ))
        .await
        .map_err(unavailable)?;
    match rows.as_slice() {
        [row] => Ok(get::<Uuid>(row, "id")?.into_bytes()),
        [] => Err(PlanFault::at(
            Error::Rejected,
            format!("token.accounts[{index}] {name:?}: no AI player has that name"),
        )),
        _ => Err(PlanFault::at(
            Error::Rejected,
            format!("token.accounts[{index}] {name:?}: two AI players have that name"),
        )),
    }
}

/// Make the plan's token: every account named once, and no live token of
/// that name left, which would make two machines' tokens alike.
async fn token(
    tx: &DatabaseTransaction,
    wanted: &RunnerTokenPlan,
    token: &Token,
) -> Result<([u8; 16], i64), PlanFault> {
    let mut accounts = Vec::with_capacity(wanted.accounts.len());
    for (index, name) in wanted.accounts.iter().enumerate() {
        accounts.push(named(tx, index, name).await?);
    }
    let twin = tx
        .query_one_raw(statement(
            &format!(
                "SELECT 1 AS one FROM central_identity.ai_runner_tokens t
                 WHERE t.name = $1 AND {}",
                crate::accounts::tokens::LIVE_TOKEN
            ),
            vec![wanted.name.clone().into()],
        ))
        .await
        .map_err(unavailable)?;
    if twin.is_some() {
        return Err(PlanFault::at(
            Error::Conflict,
            format!(
                "token.name {:?}: a live token has that name; revoke it in the same plan to rotate",
                wanted.name
            ),
        ));
    }
    make(tx, None, &wanted.name, &accounts, wanted.days, token)
        .await
        .map_err(|error| match error {
            Error::Busy => PlanFault::at(
                error,
                format!(
                    "token: {LIVE_TOKENS} runner tokens are live, or {TOKENS_PER_DAY} were made today"
                ),
            ),
            other => other.into(),
        })
}

/// Apply `plan` in one transaction under the migrator's login `db`, as the
/// operator, and answer what it did. `secret` is the new runner token's,
/// present exactly when the plan makes one; only its digest is stored, and
/// the caller shows it once after this returns. Every fault the plan has
/// on its own ([`plan_faults`]) is found before the database is asked.
///
/// # Errors
/// `Rejected` for a plan that does not fit ([`plan_faults`], every fault
/// listed), an unknown token to revoke, a token account that names no AI
/// player or several; `Conflict` for an ambiguous player (a tag or name
/// held otherwise, another kind or profile) and a live token of the same
/// name; `Busy` past the tokens' caps; storage failure. Each says where;
/// nothing is written then.
pub async fn apply_ai_plan(
    db: &DatabaseConnection,
    plan: &AiPlan,
    secret: Option<&Token>,
) -> Result<AiApplied, PlanFault> {
    let faults = plan_faults(plan, secret.is_some());
    if !faults.is_empty() {
        return Err(PlanFault {
            error: Error::Rejected,
            faults,
        });
    }
    let tx = db.begin().await.map_err(unavailable)?;
    tx.query_all_raw(statement(
        "SELECT a.id FROM central_identity.accounts a JOIN central_identity.account_roles r
         ON r.account_id = a.id WHERE r.role = 'admin' ORDER BY a.id FOR UPDATE OF a",
        vec![],
    ))
    .await
    .map_err(unavailable)?;
    let mut applied = AiApplied::default();
    if let Some(wanted) = plan.settings {
        let next = wanted.over(settings(&tx, true).await?);
        write_settings(&tx, None, &next).await?;
        applied.settings = Some(next);
    }
    // The tokens to revoke are held before any AI account, as an exchange
    // takes its token before its account, and their sessions are ended
    // after the accounts: an exchange then never holds what this waits for.
    let revoked: Vec<Uuid> = plan.revoke.iter().map(|id| uuid(*id)).collect();
    tx.query_all_raw(statement(
        "SELECT id FROM central_identity.ai_runner_tokens WHERE id = ANY($1) ORDER BY id
         FOR UPDATE",
        vec![revoked.into()],
    ))
    .await
    .map_err(unavailable)?;
    // Removals before the players, in the plan's order, each AI's account
    // row before its own row, as a change takes them.
    applied.removed = removals(&tx, plan).await?;
    for (index, wanted) in plan.players.iter().enumerate() {
        applied.players.push(player(&tx, index, wanted).await?);
    }
    for (index, id) in plan.revoke.iter().enumerate() {
        let ended = revoke(&tx, None, uuid(*id))
            .await
            .map_err(|error| match error {
                Error::Rejected => PlanFault::at(
                    error,
                    format!("revoke[{index}] {}: no runner token has that id", uuid(*id)),
                ),
                other => other.into(),
            })?;
        applied.revoked.push((*id, ended.len()));
    }
    if let (Some(wanted), Some(secret)) = (&plan.token, secret) {
        applied.token = Some(token(&tx, wanted, secret).await?);
    }
    tx.commit().await.map_err(unavailable)?;
    Ok(applied)
}

/// The runner tokens, as the administration lists them (never a digest),
/// under the migrator's login `db`.
///
/// # Errors
/// Storage failure.
pub async fn ai_runner_tokens(db: &DatabaseConnection) -> Result<Vec<AiRunnerTokenInfo>, Error> {
    let tx = db.begin().await.map_err(unavailable)?;
    let tokens = listed(&tx).await?;
    tx.commit().await.map_err(unavailable)?;
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The settings the plan leaves out stay as they are.
    #[test]
    fn the_settings_left_out_stay() {
        let held = AiSettings {
            fill_in: true,
            fill_in_after: 60,
            drain_minutes: 120,
            global_seats: 8,
            host_reserve: 2,
            carousel_games: 0,
            carousel_daily: 0,
        };
        let calibration = AiSettingsPlan {
            fill_in: Some(false),
            carousel_games: Some(8),
            carousel_daily: Some(10_000),
            ..AiSettingsPlan::default()
        };
        assert_eq!(
            calibration.over(held),
            AiSettings {
                fill_in: false,
                carousel_games: 8,
                carousel_daily: 10_000,
                ..held
            }
        );
        assert_eq!(AiSettingsPlan::default().over(held), held);
    }
}
