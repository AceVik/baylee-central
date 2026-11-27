//! `--ai-players-stdin` (`docs/operations.md` §"AI players"): the operator
//! creates and changes AI players, switches them, sets the settings for
//! every AI, revokes runner tokens and makes one for a runner machine, or
//! lists the tokens, under the migrator's login, which arrives on standard
//! input as every secret does (`--appoint-admin-stdin`'s reading and
//! refusal). A plan is applied whole or not at all, through the functions
//! the administration's page takes, with the operator as the audit's actor.
//!
//! Standard output carries a new token's text and nothing else, exactly
//! once, after the plan committed; or, for a listing, the listing. What was
//! done goes to standard error, without a secret, and so does what is wrong
//! with a plan: every fault at once, each in its place
//! (`players[3].display_name "KI-DeepSeek-Flash": 17 characters, at most
//! 16`), found before the database is asked where the plan alone shows it,
//! and never a word of the login.
use super::production::{Database, parse, read_settings};
use central_api::Token;
use central_api::admin::AiPlayerSettings;
use central_api::ai_runner::{AiRunnerTokenInfo, token_text};
use central_api::ranked::ai::AiKind;
use central_postgres::{
    AiApplied, AiPlan, AiPlayerPlan, AiSettingsPlan, PlanFault, PlayerOutcome, RunnerTokenPlan,
    ai_runner_tokens, apply_ai_plan, connect_server, plan_faults,
};
use serde::Deserialize;
use std::process::ExitCode;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// What the operator sends. Every field but the login may be left out; a
/// listing goes alone.
#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
struct Input {
    database: Database,
    #[serde(default)]
    #[zeroize(skip)]
    players: Vec<Player>,
    #[serde(default)]
    #[zeroize(skip)]
    settings: Option<Settings>,
    #[serde(default)]
    #[zeroize(skip)]
    revoke: Vec<String>,
    #[serde(default)]
    #[zeroize(skip)]
    token: Option<NewToken>,
    #[serde(default)]
    #[zeroize(skip)]
    list: bool,
}

/// One AI player: its public name and tag name it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Player {
    display_name: String,
    tag: u32,
    kind: AiKind,
    profile: String,
    label: String,
    max_games: u8,
    #[serde(default)]
    daily_games: Option<u32>,
    #[serde(default)]
    enabled: Option<bool>,
}

/// The settings for every AI to change; the rest stay.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Settings {
    fill_in: Option<bool>,
    fill_in_after: Option<u16>,
    drain_minutes: Option<u16>,
    global_seats: Option<u8>,
    host_reserve: Option<u8>,
    carousel_games: Option<u8>,
    carousel_daily: Option<u32>,
}

/// The runner token to make for one machine.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NewToken {
    name: String,
    accounts: Vec<String>,
    #[serde(default = "ninety")]
    days: u16,
}

/// The lifetime the administration's page offers too.
const fn ninety() -> u16 {
    90
}

/// The refusal for everything that is not done: what it read is never echoed.
const REFUSED: &str =
    "central: AI players rejected the input, found it ambiguous, or failed; nothing was changed";

/// A token id as the listing prints it: the UUID's hyphenated form.
fn uuid_text(id: &[u8; 16]) -> String {
    let hex = central_api::reports::hex(id);
    format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
}

/// A token id as the operator writes it, hyphenated or not.
fn uuid_bytes(text: &str) -> Result<[u8; 16], ()> {
    let digits: Vec<u8> = text.bytes().filter(|byte| *byte != b'-').collect();
    if digits.len() != 32 || (text.len() != 32 && text.len() != 36) {
        return Err(());
    }
    let mut id = [0; 16];
    for (slot, pair) in id.iter_mut().zip(digits.chunks(2)) {
        let pair = std::str::from_utf8(pair).map_err(|_| ())?;
        if !pair.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(());
        }
        *slot = u8::from_str_radix(pair, 16).map_err(|_| ())?;
    }
    if uuid_text(&id) != text.to_ascii_lowercase() && text.len() == 36 {
        return Err(());
    }
    Ok(id)
}

/// What the operator asks, whichever way the input was read.
struct Parts<'a> {
    players: &'a [Player],
    settings: Option<&'a Settings>,
    revoke: &'a [String],
    token: Option<&'a NewToken>,
    list: bool,
}

impl Input {
    fn parts(&self) -> Parts<'_> {
        Parts {
            players: &self.players,
            settings: self.settings.as_ref(),
            revoke: &self.revoke,
            token: self.token.as_ref(),
            list: self.list,
        }
    }
}

/// The plan the input asks for, or every fault of its shape: a listing
/// beside anything else, nothing asked, a token id that is none.
fn plan(input: &Parts<'_>) -> Result<AiPlan, Vec<String>> {
    let changes = !input.players.is_empty()
        || input.settings.is_some()
        || !input.revoke.is_empty()
        || input.token.is_some();
    let mut wrong = Vec::new();
    if input.list && changes {
        wrong.push(
            "list: a listing goes alone, without players, settings, revoke or token".to_owned(),
        );
    } else if !input.list && !changes {
        wrong.push("nothing asked: give players, settings, revoke, token, or list".to_owned());
    }
    let mut revoke = Vec::with_capacity(input.revoke.len());
    for (index, text) in input.revoke.iter().enumerate() {
        match uuid_bytes(text) {
            Ok(id) => revoke.push(id),
            Err(()) => wrong.push(format!(
                "revoke[{index}] {text:?}: a runner token's id, such as 0192f0c4-7b1a-7cde-8f00-000000000001"
            )),
        }
    }
    if !wrong.is_empty() {
        return Err(wrong);
    }
    Ok(AiPlan {
        players: input
            .players
            .iter()
            .map(|player| AiPlayerPlan {
                settings: AiPlayerSettings {
                    kind: player.kind,
                    profile: player.profile.clone(),
                    label: player.label.clone(),
                    max_games: player.max_games,
                    daily_games: player.daily_games,
                    display_name: player.display_name.clone(),
                },
                tag: player.tag,
                enabled: player.enabled,
            })
            .collect(),
        settings: input.settings.map(|settings| AiSettingsPlan {
            fill_in: settings.fill_in,
            fill_in_after: settings.fill_in_after,
            drain_minutes: settings.drain_minutes,
            global_seats: settings.global_seats,
            host_reserve: settings.host_reserve,
            carousel_games: settings.carousel_games,
            carousel_daily: settings.carousel_daily,
        }),
        revoke,
        token: input.token.map(|token| RunnerTokenPlan {
            name: token.name.clone(),
            accounts: token.accounts.clone(),
            days: token.days,
        }),
    })
}

/// The plan of `parts`, or every fault it has before the database is
/// asked: its shape's ([`plan`]) and then its fields' (`plan_faults`). The
/// command makes a token's secret exactly when the plan makes a token.
fn checked(parts: &Parts<'_>) -> Result<AiPlan, Vec<String>> {
    let plan = plan(parts)?;
    let faults = plan_faults(&plan, plan.token.is_some());
    if faults.is_empty() {
        Ok(plan)
    } else {
        Err(faults)
    }
}

/// The input's shape without the login: read when [`Input`] could not be,
/// to say what is wrong without quoting the login. Its `database` is read
/// past unseen, so no word of it reaches a message.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Shape {
    #[serde(default)]
    database: Option<serde::de::IgnoredAny>,
    #[serde(default)]
    players: Vec<Player>,
    #[serde(default)]
    settings: Option<Settings>,
    #[serde(default)]
    revoke: Vec<String>,
    #[serde(default)]
    token: Option<NewToken>,
    #[serde(default)]
    list: bool,
}

/// Why `bytes` is no input, every fault at once, for standard error: the
/// parser's words on the plan's part, or that the login is missing or is
/// none with every fault of the plan beside it; never what the login holds.
fn unread(bytes: &[u8]) -> Vec<String> {
    let shape = match serde_json::from_slice::<Shape>(bytes) {
        Ok(shape) => shape,
        Err(error) => return vec![format!("the input: {error}")],
    };
    let login = if shape.database.is_some() {
        "database: the migrator login, with host, port, name, user, password and ca_certificate, and nothing else"
    } else {
        "database: missing; the migrator login comes in it"
    };
    let parts = Parts {
        players: &shape.players,
        settings: shape.settings.as_ref(),
        revoke: &shape.revoke,
        token: shape.token.as_ref(),
        list: shape.list,
    };
    let mut wrong = vec![login.to_owned()];
    if let Err(faults) = checked(&parts) {
        wrong.extend(faults);
    }
    wrong
}

/// Each fault on its own line of standard error, before the refusal.
fn say(faults: &[String]) {
    for fault in faults {
        eprintln!("central: {fault}");
    }
}

/// What the plan did, for standard error: names, tags, ids and counts.
fn report(applied: &AiApplied) {
    for player in &applied.players {
        let outcome = match player.outcome {
            PlayerOutcome::Created => "created",
            PlayerOutcome::Unchanged => "unchanged",
            PlayerOutcome::Changed => "changed",
        };
        let switched = if player.switched { ", switched" } else { "" };
        eprintln!(
            "central: AI player {}#{:x} {outcome}{switched}",
            player.display_name, player.tag
        );
    }
    if let Some(settings) = applied.settings {
        eprintln!(
            "central: AI settings: fill-in {}, after {} s, drain {} min, {} seats, host reserve {}, carousel {} at once and {} a day",
            if settings.fill_in { "on" } else { "off" },
            settings.fill_in_after,
            settings.drain_minutes,
            settings.global_seats,
            settings.host_reserve,
            settings.carousel_games,
            settings.carousel_daily
        );
    }
    for (id, sessions) in &applied.revoked {
        eprintln!(
            "central: runner token {} revoked, {sessions} sessions ended",
            uuid_text(id)
        );
    }
    if let Some((id, expires_at)) = applied.token {
        eprintln!(
            "central: runner token {} made, expires at {expires_at} (Unix seconds); its text is on standard output, shown this once",
            uuid_text(&id)
        );
    }
}

/// One line for a token of the listing: never a digest or a text.
fn listed(token: &AiRunnerTokenInfo) -> String {
    let accounts: Vec<String> = token
        .accounts
        .iter()
        .map(|account| {
            format!(
                "{}#{:x}",
                account.display_name.as_deref().unwrap_or("?"),
                account.tag.unwrap_or(0)
            )
        })
        .collect();
    let time = |at: Option<i64>| at.map_or_else(|| "never".to_owned(), |at| at.to_string());
    format!(
        "{} name={:?} accounts={} expires={} last_used={} revoked={} made_by={}",
        uuid_text(&token.id),
        token.name,
        accounts.join(","),
        token.expires_at,
        time(token.last_used_at),
        time(token.revoked_at),
        if token.created_by.is_some() {
            "administrator"
        } else {
            "operator"
        }
    )
}

pub(super) async fn run() -> ExitCode {
    if apply().await.is_ok() {
        ExitCode::SUCCESS
    } else {
        eprintln!("{REFUSED}");
        ExitCode::FAILURE
    }
}

async fn apply() -> Result<(), ()> {
    let Ok(bytes) = read_settings() else {
        say(&["the input could not be read, or is larger than the launcher takes".to_owned()]);
        return Err(());
    };
    let Ok(mut input) = parse::<Input>(&bytes) else {
        say(&unread(&bytes));
        return Err(());
    };
    drop(bytes);
    // Every fault the plan has on its own, all at once, before the database.
    let plan = checked(&input.parts()).map_err(|faults| say(&faults))?;
    let Ok(database) = input.database.server() else {
        say(&["database: the login is not one this command can use (host, port, name, user, password, ca_certificate)".to_owned()]);
        return Err(());
    };
    let list = input.list;
    drop(input);
    let Ok(db) = connect_server(database).await else {
        say(&["database: could not connect with that login".to_owned()]);
        return Err(());
    };
    if list {
        let tokens = ai_runner_tokens(&db).await;
        let closed = db.close().await;
        let Ok(tokens) = tokens else {
            say(&["the database failed while listing".to_owned()]);
            return Err(());
        };
        for token in &tokens {
            println!("{}", listed(token));
        }
        eprintln!("central: {} runner tokens listed", tokens.len());
        return closed.map_err(|_| ());
    }
    let secret = plan
        .token
        .as_ref()
        .map(|_| Token::generate())
        .transpose()
        .map_err(|_| say(&["no randomness for a token's secret".to_owned()]))?;
    let applied = apply_ai_plan(&db, &plan, secret.as_ref()).await;
    let closed = db.close().await;
    let applied = applied.map_err(|fault: PlanFault| say(&fault.faults))?;
    report(&applied);
    // Committed: the token works now, so its text is shown, this once. A
    // connection that fails to close after the commit changes nothing of it.
    if let (Some(secret), Some(_)) = (secret, applied.token) {
        println!("{}", *token_text(&secret));
    }
    if closed.is_err() {
        eprintln!("central: the plan was applied; closing the connection failed");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(json: &str) -> Input {
        serde_json::from_str(json).unwrap()
    }

    const LOGIN: &str = r#""database":{"host":"127.0.0.1","port":5432,"name":"central","user":"central_migrator","password":"synthetic","ca_certificate":"synthetic"}"#;

    #[test]
    fn a_listing_goes_alone_and_something_is_asked() {
        let planned = |json: String| plan(&input(&json).parts());
        assert!(planned(format!(r#"{{{LOGIN},"list":true}}"#)).is_ok());
        assert_eq!(
            planned(format!(r"{{{LOGIN}}}")).unwrap_err(),
            ["nothing asked: give players, settings, revoke, token, or list"]
        );
        assert!(
            planned(format!(
                r#"{{{LOGIN},"list":true,"revoke":["0192f0c4-7b1a-7cde-8f00-000000000001"]}}"#
            ))
            .is_err()
        );
    }

    #[test]
    fn a_whole_plan_reads_as_its_parts() {
        let read = checked(&input(&format!(
            r#"{{{LOGIN},
              "players":[{{"display_name":"KI-Haus-Expert","tag":6699,"kind":"house",
                "profile":"house:expert","label":"Haus-KI · Expert","max_games":4,"enabled":true}}],
              "settings":{{"fill_in":false,"carousel_games":8,"carousel_daily":10000,"global_seats":16}},
              "revoke":["0192f0c47b1a7cde8f00000000000001"],
              "token":{{"name":"runner-1","accounts":["KI-Haus-Expert"]}}}}"#
        ))
        .parts())
        .unwrap();
        assert_eq!(read.players.len(), 1);
        assert_eq!(read.players[0].tag, 6699);
        assert_eq!(read.players[0].enabled, Some(true));
        assert_eq!(read.players[0].settings.kind, AiKind::House);
        let settings = read.settings.unwrap();
        assert_eq!(settings.fill_in, Some(false));
        assert_eq!(settings.fill_in_after, None);
        assert_eq!(read.revoke[0][..2], [0x01, 0x92]);
        assert_eq!(read.token.unwrap().days, 90);
    }

    #[test]
    fn a_token_id_is_a_uuid_or_refused() {
        let id = [
            0x01, 0x92, 0xf0, 0xc4, 0x7b, 0x1a, 0x7c, 0xde, 0x8f, 0, 0, 0, 0, 0, 0, 1,
        ];
        assert_eq!(uuid_text(&id), "0192f0c4-7b1a-7cde-8f00-000000000001");
        assert_eq!(uuid_bytes("0192f0c4-7b1a-7cde-8f00-000000000001"), Ok(id));
        assert_eq!(uuid_bytes("0192F0C4-7B1A-7CDE-8F00-000000000001"), Ok(id));
        assert_eq!(uuid_bytes("0192f0c47b1a7cde8f00000000000001"), Ok(id));
        for wrong in [
            "",
            "0192f0c4-7b1a-7cde-8f00-00000000000",
            "0192f0c4-7b1a-7cde-8f00-0000000000001",
            "0192f0c47b1a-7cde-8f00-0000-00000001",
            "0192f0c4-7b1a-7cde-8f00-00000000000g",
            "+192f0c4-7b1a-7cde-8f00-000000000001",
        ] {
            assert_eq!(uuid_bytes(wrong), Err(()), "{wrong}");
        }
    }

    #[test]
    fn unknown_fields_are_refused() {
        for json in [
            format!(r#"{{{LOGIN},"list":true,"secret":"x"}}"#),
            format!(
                r#"{{{LOGIN},"players":[{{"display_name":"KI-A1","tag":1,"kind":"house","profile":"house:expert","label":"x","max_games":1,"password":"x"}}]}}"#
            ),
            format!(r#"{{{LOGIN},"token":{{"name":"r","accounts":["KI-A1"],"text":"bat_"}}}}"#),
        ] {
            assert!(serde_json::from_str::<Input>(&json).is_err(), "{json}");
        }
    }

    #[test]
    fn a_listed_token_shows_no_secret() {
        let token = AiRunnerTokenInfo {
            id: [1; 16],
            name: "runner-1".into(),
            accounts: vec![central_api::ai_runner::AiRunnerAccount {
                id: [2; 16],
                display_name: Some("KI-Haus-Expert".into()),
                tag: Some(0x1a2b),
            }],
            created_by: None,
            created_by_name: None,
            created_by_tag: None,
            created_at: 1,
            expires_at: 2,
            revoked_at: None,
            revoked_by: None,
            revoked_by_name: None,
            revoked_by_tag: None,
            last_used_at: None,
        };
        let line = listed(&token);
        assert!(line.contains("KI-Haus-Expert#1a2b"), "{line}");
        assert!(line.contains("made_by=operator"), "{line}");
        assert!(!line.contains("bat_"), "{line}");
    }
}
