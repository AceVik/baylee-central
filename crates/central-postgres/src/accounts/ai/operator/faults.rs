//! What is wrong with an operator's plan before the database is asked, said
//! field by field (`central-service --ai-players-stdin`). The rules are the
//! administration's own (`AiPlayerSettings::valid`, `AiSettings::valid`,
//! the runner token's `validate_new`); this only names which one a value
//! breaks. These words go to the operator's standard error and nowhere
//! else: the network-facing requests keep their one `Rejected`. A plan holds
//! no secret, so a value may be quoted.
use super::{AiPlan, AiPlayerPlan, AiSettingsPlan, RunnerTokenPlan};
use central_api::admin::AI_PLAYERS;
use central_api::ai_runner::{LISTED_TOKENS, MAX_ACCOUNTS};
use central_api::gateway::shown_char;
use central_api::owner::{MAX_TOKEN_DAYS, MAX_TOKEN_NAME};
use central_api::profile::{MAX_TAG, ai_name, valid_tag};
use central_api::ranked::ai::{
    AiKind, LABEL_CHARS, MAX_GAMES, UNFAIR_PROFILES, valid_label, valid_profile,
};
use std::collections::BTreeMap;

/// The largest daily cap the administration takes.
const MOST_DAILY: u32 = 10_000;

/// The index of the first entry with `key`, when there is one; otherwise
/// `index` is that entry from now on.
fn first<K: Ord>(seen: &mut BTreeMap<K, usize>, key: K, index: usize) -> Option<usize> {
    if let Some(before) = seen.get(&key) {
        return Some(*before);
    }
    seen.insert(key, index);
    None
}

/// What is wrong with a display name an AI player is to carry, as
/// `valid_display_name` and `ai_name` read it.
fn name_faults(name: &str) -> Vec<String> {
    let mut wrong = Vec::new();
    let length = name.chars().count();
    if length > 16 {
        wrong.push(format!("{length} characters, at most 16"));
    } else if length < 3 {
        wrong.push(format!("{length} characters, at least 3"));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        wrong.push("only ASCII letters, digits, _ and -".to_owned());
    }
    let plain = |c: Option<char>| c.is_some_and(|c| c.is_ascii_alphanumeric());
    if !plain(name.chars().next()) || !plain(name.chars().last()) {
        wrong.push("begins and ends with a letter or digit".to_owned());
    }
    if !name.starts_with("KI-") {
        wrong.push("begins with KI-, an AI player's prefix".to_owned());
    }
    wrong
}

/// What is wrong with a profile.
fn profile_fault(profile: &str) -> Option<String> {
    if !valid_profile(profile) {
        Some(
            "a family of 1 to 12 lowercase letters, a colon, and a level of 1 to 48 \
             lowercase letters, digits, _, . or -, such as house:expert"
                .to_owned(),
        )
    } else if UNFAIR_PROFILES.contains(&profile) {
        Some("sees hidden cards; every ranked AI plays fair".to_owned())
    } else {
        None
    }
}

/// What is wrong with a label.
fn label_fault(label: &str) -> Option<String> {
    if valid_label(label) {
        return None;
    }
    let count = label.chars().count();
    Some(if count == 0 || count > LABEL_CHARS {
        format!("{count} characters, 1 to {LABEL_CHARS}")
    } else if label.trim() != label {
        "no space at its start or end".to_owned()
    } else {
        "no control characters".to_owned()
    })
}

/// What is wrong with one player of the plan, at `at` (`players[3]`).
fn player_faults(at: &str, player: &AiPlayerPlan) -> Vec<String> {
    let settings = &player.settings;
    let name = &settings.display_name;
    let mut wrong: Vec<String> = name_faults(name)
        .into_iter()
        .map(|rule| format!("{at}.display_name {name:?}: {rule}"))
        .collect();
    if !valid_tag(player.tag) {
        wrong.push(format!("{at}.tag {}: 1 to {MAX_TAG}", player.tag));
    }
    if settings.kind.as_str().is_none() {
        wrong.push(format!("{at}.kind: house, model or net"));
    }
    if let Some(rule) = profile_fault(&settings.profile) {
        wrong.push(format!("{at}.profile {:?}: {rule}", settings.profile));
    }
    if let Some(rule) = label_fault(&settings.label) {
        wrong.push(format!("{at}.label {:?}: {rule}", settings.label));
    }
    if !(1..=MAX_GAMES).contains(&settings.max_games) {
        wrong.push(format!(
            "{at}.max_games {}: 1 to {MAX_GAMES}",
            settings.max_games
        ));
    }
    match settings.daily_games {
        Some(daily) if daily > MOST_DAILY => {
            wrong.push(format!("{at}.daily_games {daily}: 0 to {MOST_DAILY}"));
        }
        None if settings.kind == AiKind::Model => {
            wrong.push(format!(
                "{at}.daily_games: required for a language model (kind model)"
            ));
        }
        _ => {}
    }
    wrong
}

/// What is wrong with the settings the plan sets; those it leaves out stay.
fn settings_faults(settings: &AiSettingsPlan) -> Vec<String> {
    let mut wrong = Vec::new();
    let mut range = |field: &str, value: Option<u32>, least: u32, most: u32| {
        if let Some(value) = value
            && !(least..=most).contains(&value)
        {
            wrong.push(format!("settings.{field} {value}: {least} to {most}"));
        }
    };
    range(
        "fill_in_after",
        settings.fill_in_after.map(u32::from),
        30,
        600,
    );
    range(
        "drain_minutes",
        settings.drain_minutes.map(u32::from),
        30,
        600,
    );
    range("global_seats", settings.global_seats.map(u32::from), 0, 16);
    range("host_reserve", settings.host_reserve.map(u32::from), 0, 64);
    range(
        "carousel_games",
        settings.carousel_games.map(u32::from),
        0,
        8,
    );
    range("carousel_daily", settings.carousel_daily, 0, MOST_DAILY);
    wrong
}

/// What is wrong with the token the plan makes.
fn token_faults(token: &RunnerTokenPlan) -> Vec<String> {
    let mut wrong = Vec::new();
    let name = &token.name;
    if name.is_empty() || name.len() > MAX_TOKEN_NAME {
        wrong.push(format!(
            "token.name {name:?}: {} bytes, 1 to {MAX_TOKEN_NAME}",
            name.len()
        ));
    } else if name.trim() != name {
        wrong.push(format!("token.name {name:?}: no space at its start or end"));
    } else if !name.chars().all(shown_char) {
        wrong.push(format!(
            "token.name {name:?}: no control or invisible characters"
        ));
    }
    if !(1..=MAX_TOKEN_DAYS).contains(&token.days) {
        wrong.push(format!("token.days {}: 1 to {MAX_TOKEN_DAYS}", token.days));
    }
    if token.accounts.is_empty() || token.accounts.len() > MAX_ACCOUNTS {
        wrong.push(format!(
            "token.accounts: {} AI players, 1 to {MAX_ACCOUNTS}",
            token.accounts.len()
        ));
    }
    let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
    for (index, account) in token.accounts.iter().enumerate() {
        if let Some(before) = first(&mut seen, account.as_str(), index) {
            wrong.push(format!(
                "token.accounts[{index}] {account:?}: also token.accounts[{before}]"
            ));
        } else if !ai_name(account) {
            wrong.push(format!(
                "token.accounts[{index}] {account:?}: not an AI player's name (KI-...)"
            ));
        }
    }
    wrong
}

/// Every fault of `plan`, field by field, in the plan's order; none when it
/// fits. `secret` says whether the caller brought a token's secret, which
/// it must exactly when the plan makes a token.
#[must_use]
pub fn plan_faults(plan: &AiPlan, secret: bool) -> Vec<String> {
    let mut wrong = Vec::new();
    if plan.players.len() > AI_PLAYERS {
        wrong.push(format!(
            "players: {} AI players, at most {AI_PLAYERS}",
            plan.players.len()
        ));
    }
    let mut tags: BTreeMap<u32, usize> = BTreeMap::new();
    let mut names: BTreeMap<&str, usize> = BTreeMap::new();
    for (index, player) in plan.players.iter().enumerate() {
        let at = format!("players[{index}]");
        wrong.extend(player_faults(&at, player));
        if let Some(before) = first(&mut tags, player.tag, index) {
            wrong.push(format!(
                "{at}.tag {}: also players[{before}].tag",
                player.tag
            ));
        }
        let name = player.settings.display_name.as_str();
        if let Some(before) = first(&mut names, name, index) {
            wrong.push(format!(
                "{at}.display_name {name:?}: also players[{before}].display_name"
            ));
        }
    }
    if let Some(settings) = &plan.settings {
        wrong.extend(settings_faults(settings));
    }
    if plan.revoke.len() > LISTED_TOKENS {
        wrong.push(format!(
            "revoke: {} tokens, at most {LISTED_TOKENS}",
            plan.revoke.len()
        ));
    }
    let mut revoked: BTreeMap<[u8; 16], usize> = BTreeMap::new();
    for (index, id) in plan.revoke.iter().enumerate() {
        if let Some(before) = first(&mut revoked, *id, index) {
            wrong.push(format!("revoke[{index}]: also revoke[{before}]"));
        }
    }
    if let Some(token) = &plan.token {
        wrong.extend(token_faults(token));
    }
    if plan.token.is_some() != secret {
        // The command makes the secret for the plan's token; anything else
        // is the caller's mistake, not the operator's.
        wrong.push("token: a secret exactly when a token is made".to_owned());
    }
    wrong
}

#[cfg(test)]
mod tests {
    use super::*;
    use central_api::admin::{AiPlayerSettings, AiSettings};

    fn player(name: &str) -> AiPlayerPlan {
        AiPlayerPlan {
            settings: AiPlayerSettings {
                kind: AiKind::House,
                profile: "house:expert".into(),
                label: "Haus-KI · Expert".into(),
                max_games: 4,
                daily_games: None,
                display_name: name.into(),
            },
            tag: 0x1a2b,
            enabled: Some(true),
        }
    }

    /// The faults agree with the administration's own rules: a player the
    /// plan finds nothing wrong with is one `AiPlayerSettings::valid` takes,
    /// and each that it refuses is named.
    #[test]
    fn the_faults_follow_the_administrations_rules() {
        let good = player("KI-Haus-Expert");
        assert_eq!(player_faults("players[0]", &good), Vec::<String>::new());
        assert!(good.settings.valid());
        let mut cases: Vec<(AiPlayerPlan, &str)> = vec![
            (
                player("KI-DeepSeek-Flash"),
                "players[0].display_name \"KI-DeepSeek-Flash\": 17 characters, at most 16",
            ),
            (
                player("Haus-Expert"),
                "players[0].display_name \"Haus-Expert\": begins with KI-, an AI player's prefix",
            ),
            (
                player("KI-Haus Expert"),
                "players[0].display_name \"KI-Haus Expert\": only ASCII letters, digits, _ and -",
            ),
            (
                player("KI-Haus-"),
                "players[0].display_name \"KI-Haus-\": begins and ends with a letter or digit",
            ),
        ];
        let mut wrong = player("KI-Haus-Expert");
        wrong.tag = 0;
        cases.push((wrong, "players[0].tag 0: 1 to 2147483647"));
        let mut wrong = player("KI-Haus-Expert");
        wrong.settings.kind = AiKind::Other;
        cases.push((wrong, "players[0].kind: house, model or net"));
        let mut wrong = player("KI-Haus-Expert");
        wrong.settings.profile = "house:seer".into();
        cases.push((
            wrong,
            "players[0].profile \"house:seer\": sees hidden cards; every ranked AI plays fair",
        ));
        let mut wrong = player("KI-Haus-Expert");
        wrong.settings.profile = "House:Expert".into();
        cases.push((
            wrong,
            "players[0].profile \"House:Expert\": a family of 1 to 12 lowercase letters",
        ));
        let mut wrong = player("KI-Haus-Expert");
        wrong.settings.label = " Haus".into();
        cases.push((
            wrong,
            "players[0].label \" Haus\": no space at its start or end",
        ));
        let mut wrong = player("KI-Haus-Expert");
        wrong.settings.label = "x".repeat(41);
        cases.push((wrong, "characters, 1 to 40"));
        let mut wrong = player("KI-Haus-Expert");
        wrong.settings.max_games = 17;
        cases.push((wrong, "players[0].max_games 17: 1 to 16"));
        let mut wrong = player("KI-Haus-Expert");
        wrong.settings.daily_games = Some(10_001);
        cases.push((wrong, "players[0].daily_games 10001: 0 to 10000"));
        let mut wrong = player("KI-Claude-Haiku");
        wrong.settings.kind = AiKind::Model;
        wrong.settings.profile = "model:haiku".into();
        cases.push((
            wrong,
            "players[0].daily_games: required for a language model (kind model)",
        ));
        for (wrong, said) in cases {
            let faults = player_faults("players[0]", &wrong);
            assert!(!wrong.settings.valid() || !valid_tag(wrong.tag), "{said}");
            assert!(
                faults
                    .iter()
                    .any(|fault| fault.starts_with(said) || fault.contains(said)),
                "{said}: {faults:?}"
            );
        }
    }

    /// Every fault at once, in the plan's order, with each place named.
    #[test]
    fn a_plan_lists_every_fault_at_once() {
        let mut second = player("KI-DeepSeek-Flash");
        second.settings.max_games = 0;
        let plan = AiPlan {
            players: vec![player("KI-Haus-Expert"), second, player("KI-Haus-Expert")],
            settings: Some(AiSettingsPlan {
                fill_in_after: Some(5),
                carousel_games: Some(9),
                ..AiSettingsPlan::default()
            }),
            revoke: vec![[1; 16], [1; 16]],
            token: Some(RunnerTokenPlan {
                name: " runner".into(),
                accounts: vec![
                    "KI-Haus-Expert".into(),
                    "Bob".into(),
                    "KI-Haus-Expert".into(),
                ],
                days: 400,
            }),
        };
        let faults = plan_faults(&plan, true);
        for said in [
            "players[1].display_name \"KI-DeepSeek-Flash\": 17 characters, at most 16",
            "players[1].max_games 0: 1 to 16",
            "players[1].tag 6699: also players[0].tag",
            "players[2].tag 6699: also players[0].tag",
            "players[2].display_name \"KI-Haus-Expert\": also players[0].display_name",
            "settings.fill_in_after 5: 30 to 600",
            "settings.carousel_games 9: 0 to 8",
            "revoke[1]: also revoke[0]",
            "token.name \" runner\": no space at its start or end",
            "token.days 400: 1 to 366",
            "token.accounts[1] \"Bob\": not an AI player's name (KI-...)",
            "token.accounts[2] \"KI-Haus-Expert\": also token.accounts[0]",
        ] {
            assert!(
                faults.iter().any(|fault| fault == said),
                "{said}: {faults:?}"
            );
        }
        assert_eq!(faults.len(), 12, "{faults:?}");
        // A plan that fits has none; the secret goes with a token alone.
        let fits = AiPlan {
            players: vec![player("KI-Haus-Expert")],
            ..AiPlan::default()
        };
        assert_eq!(plan_faults(&fits, false), Vec::<String>::new());
        assert_eq!(
            plan_faults(&fits, true),
            ["token: a secret exactly when a token is made"]
        );
    }

    /// The settings' bounds are the administration's: a value the plan
    /// passes, over any held settings, makes settings `AiSettings::valid`
    /// takes.
    #[test]
    fn settings_in_bounds_are_the_administrations() {
        let held = AiSettings {
            fill_in: true,
            fill_in_after: 60,
            drain_minutes: 120,
            global_seats: 8,
            host_reserve: 2,
            carousel_games: 0,
            carousel_daily: 0,
        };
        for edge in [
            AiSettingsPlan {
                fill_in_after: Some(30),
                drain_minutes: Some(600),
                global_seats: Some(16),
                host_reserve: Some(64),
                carousel_games: Some(8),
                carousel_daily: Some(10_000),
                ..AiSettingsPlan::default()
            },
            AiSettingsPlan {
                fill_in_after: Some(600),
                drain_minutes: Some(30),
                global_seats: Some(0),
                host_reserve: Some(0),
                carousel_games: Some(0),
                carousel_daily: Some(0),
                ..AiSettingsPlan::default()
            },
        ] {
            assert_eq!(settings_faults(&edge), Vec::<String>::new());
            assert!(edge.over(held).valid());
        }
    }
}
