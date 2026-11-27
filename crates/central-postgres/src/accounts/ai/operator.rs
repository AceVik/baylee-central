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
//! counts them), the settings for every AI, the tokens revoked with their
//! sessions, the AI accounts and their rows, and a new token last.
use super::lifecycle::{change, create};
use super::{get, settings, switch, unavailable, write_settings};
use crate::accounts::ai_runner::{listed, make, revoke};
use crate::accounts::{Error, Token, statement, uuid};
use central_api::admin::{AI_PLAYERS, AiPlayerSettings, AiSettings};
use central_api::ai_runner::{AiRunnerTokenInfo, LISTED_TOKENS, MAX_ACCOUNTS};
use central_api::owner::{MAX_TOKEN_DAYS, token_name};
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DatabaseTransaction, QueryResult, TransactionTrait,
    prelude::Uuid,
};
use std::collections::BTreeSet;

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
    /// A runner token to make.
    pub token: Option<RunnerTokenPlan>,
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
    /// The new token's id and expiry, in Unix seconds.
    pub token: Option<([u8; 16], i64)>,
}

/// The plan's own consistency, before anything is asked: settings in their
/// bounds, distinct tags and names, at most [`AI_PLAYERS`] players, distinct
/// revocations, and a token whose name, lifetime and list fit, presented
/// exactly when one is to be made.
fn checked(plan: &AiPlan, token: Option<&Token>) -> Result<(), Error> {
    let mut tags = BTreeSet::new();
    let mut names = BTreeSet::new();
    let players_fit = plan.players.len() <= AI_PLAYERS
        && plan.players.iter().all(|player| {
            player.settings.valid()
                && central_api::profile::valid_tag(player.tag)
                && tags.insert(player.tag)
                && names.insert(player.settings.display_name.as_str())
        });
    let revoked: BTreeSet<_> = plan.revoke.iter().collect();
    let revocations_fit = revoked.len() == plan.revoke.len() && plan.revoke.len() <= LISTED_TOKENS;
    let token_fits = match (&plan.token, token) {
        (None, None) => true,
        (Some(wanted), Some(_)) => {
            let listed: BTreeSet<_> = wanted.accounts.iter().collect();
            token_name(&wanted.name).is_ok()
                && (1..=MAX_TOKEN_DAYS).contains(&wanted.days)
                && !wanted.accounts.is_empty()
                && wanted.accounts.len() <= MAX_ACCOUNTS
                && listed.len() == wanted.accounts.len()
        }
        _ => false,
    };
    if players_fit && revocations_fit && token_fits {
        Ok(())
    } else {
        Err(Error::Rejected)
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

/// Bring one AI player to the plan: made when neither its tag nor its name
/// is taken, changed when an AI holds both and its label or limits differ.
/// Anything else is ambiguous and refused: a tag a person holds or an AI of
/// another name, a name another AI holds under another tag, another kind or
/// profile.
async fn player(tx: &DatabaseTransaction, wanted: &AiPlayerPlan) -> Result<PlayerApplied, Error> {
    let tag = i32::try_from(wanted.tag).map_err(|_| Error::Rejected)?;
    let found = tx
        .query_all_raw(statement(
            "SELECT a.id, a.display_name, a.tag, p.kind, p.profile, p.label, p.max_games,
                 p.daily_games, p.account_id IS NOT NULL AS ai
             FROM central_identity.accounts a
             LEFT JOIN central_identity.ai_players p ON p.account_id = a.id
             WHERE a.tag = $1 OR (p.account_id IS NOT NULL AND a.display_name = $2)
             ORDER BY a.id",
            vec![tag.into(), wanted.settings.display_name.clone().into()],
        ))
        .await
        .map_err(unavailable)?;
    let settings = &wanted.settings;
    let (account, outcome) = match found.as_slice() {
        [] => {
            let (account, _) = create(tx, None, settings, wanted.tag).await?;
            (Uuid::from_bytes(account), PlayerOutcome::Created)
        }
        [row] => {
            let same = get::<bool>(row, "ai")?
                && get::<Option<i32>>(row, "tag")? == Some(tag)
                && get::<Option<String>>(row, "display_name")?.as_deref()
                    == Some(settings.display_name.as_str());
            if !same {
                return Err(Error::Conflict);
            }
            let account: Uuid = get(row, "id")?;
            let (kind, profile, label, max_games, daily) = held(row)?;
            if Some(kind.as_str()) != settings.kind.as_str() || profile != settings.profile {
                return Err(Error::Conflict);
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
        _ => return Err(Error::Conflict),
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

/// The one AI player named `name`; none or several is refused.
async fn named(tx: &DatabaseTransaction, name: &str) -> Result<[u8; 16], Error> {
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
        _ => Err(Error::Rejected),
    }
}

/// Make the plan's token: every account named once, and no live token of
/// that name left, which would make two machines' tokens alike.
async fn token(
    tx: &DatabaseTransaction,
    wanted: &RunnerTokenPlan,
    token: &Token,
) -> Result<([u8; 16], i64), Error> {
    let mut accounts = Vec::with_capacity(wanted.accounts.len());
    for name in &wanted.accounts {
        accounts.push(named(tx, name).await?);
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
        return Err(Error::Conflict);
    }
    make(tx, None, &wanted.name, &accounts, wanted.days, token).await
}

/// Apply `plan` in one transaction under the migrator's login `db`, as the
/// operator, and answer what it did. `token` is the new runner token's
/// secret, present exactly when the plan makes one; only its digest is
/// stored, and the caller shows it once after this returns.
///
/// # Errors
/// `Rejected` for a plan that does not fit ([`checked`]), settings out of
/// bounds, an unknown token to revoke, a token account that names no AI
/// player or several; `Conflict` for an ambiguous player (a tag or name
/// held otherwise, another kind or profile) and a live token of the same
/// name; `Busy` past the tokens' caps; storage failure. Nothing is written
/// then.
pub async fn apply_ai_plan(
    db: &DatabaseConnection,
    plan: &AiPlan,
    secret: Option<&Token>,
) -> Result<AiApplied, Error> {
    checked(plan, secret)?;
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
    for id in &plan.revoke {
        let ended = revoke(&tx, None, uuid(*id)).await?;
        applied.revoked.push((*id, ended.len()));
    }
    for wanted in &plan.players {
        applied.players.push(player(&tx, wanted).await?);
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
    use central_api::ranked::ai::AiKind;

    fn house(name: &str) -> AiPlayerSettings {
        AiPlayerSettings {
            kind: AiKind::House,
            profile: "house:expert".into(),
            label: "Haus-KI · Expert".into(),
            max_games: 4,
            daily_games: None,
            display_name: name.into(),
        }
    }

    fn plan(players: Vec<AiPlayerPlan>, token: Option<RunnerTokenPlan>) -> AiPlan {
        AiPlan {
            players,
            settings: None,
            revoke: Vec::new(),
            token,
        }
    }

    fn one(name: &str, tag: u32) -> AiPlayerPlan {
        AiPlayerPlan {
            settings: house(name),
            tag,
            enabled: Some(true),
        }
    }

    /// A plan that does not fit is refused before anything is asked: a
    /// repeated tag or name, a person's name, a token without its secret
    /// or a secret without a token, a token's name, lifetime or list out
    /// of bounds, a revocation named twice.
    #[test]
    fn a_plan_that_does_not_fit_is_refused_whole() {
        let secret = Token::generate().unwrap();
        let runner = |accounts: Vec<&str>, days| RunnerTokenPlan {
            name: "runner-1".into(),
            accounts: accounts.into_iter().map(str::to_owned).collect(),
            days,
        };
        let fits = plan(
            vec![one("KI-Haus-Expert", 0x1a2b)],
            Some(runner(vec!["KI-Haus-Expert"], 90)),
        );
        assert_eq!(checked(&fits, Some(&secret)), Ok(()));
        assert_eq!(checked(&plan(vec![], None), None), Ok(()));
        for (wrong, with) in [
            (plan(vec![one("KI-A1", 7), one("KI-B1", 7)], None), false),
            (plan(vec![one("KI-A1", 7), one("KI-A1", 8)], None), false),
            (plan(vec![one("Person", 7)], None), false),
            (plan(vec![one("AI-Bot", 7)], None), false),
            (plan(vec![], Some(runner(vec!["KI-A1"], 90))), false),
            (plan(vec![], None), true),
            (plan(vec![], Some(runner(vec!["KI-A1"], 0))), true),
            (plan(vec![], Some(runner(vec!["KI-A1"], 367))), true),
            (plan(vec![], Some(runner(vec![], 90))), true),
            (plan(vec![], Some(runner(vec!["KI-A1", "KI-A1"], 90))), true),
        ] {
            let secret = with.then_some(&secret);
            assert_eq!(checked(&wrong, secret), Err(Error::Rejected), "{wrong:?}");
        }
        let mut twice = plan(vec![], None);
        twice.revoke = vec![[1; 16], [1; 16]];
        assert_eq!(checked(&twice, None), Err(Error::Rejected));
        let mut unfair = plan(vec![one("KI-Seer", 9)], None);
        unfair.players[0].settings.profile = "house:seer".into();
        assert_eq!(checked(&unfair, None), Err(Error::Rejected));
    }

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
