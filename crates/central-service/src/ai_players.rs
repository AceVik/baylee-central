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
//! done goes to standard error, without a secret.
use super::production::{Database, parse, read_settings};
use central_api::Token;
use central_api::admin::AiPlayerSettings;
use central_api::ai_runner::{AiRunnerTokenInfo, token_text};
use central_api::ranked::ai::AiKind;
use central_postgres::{
    AiApplied, AiPlan, AiPlayerPlan, AiSettingsPlan, PlayerOutcome, RunnerTokenPlan,
    ai_runner_tokens, apply_ai_plan, connect_server,
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

/// The plan the input asks for; a listing beside anything else, and a
/// token id that is none, are refused.
fn plan(input: &Input) -> Result<AiPlan, ()> {
    let changes = !input.players.is_empty()
        || input.settings.is_some()
        || !input.revoke.is_empty()
        || input.token.is_some();
    if input.list == changes {
        // a listing alone, or changes alone; neither is no request
        return Err(());
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
        settings: input.settings.as_ref().map(|settings| AiSettingsPlan {
            fill_in: settings.fill_in,
            fill_in_after: settings.fill_in_after,
            drain_minutes: settings.drain_minutes,
            global_seats: settings.global_seats,
            host_reserve: settings.host_reserve,
            carousel_games: settings.carousel_games,
            carousel_daily: settings.carousel_daily,
        }),
        revoke: input
            .revoke
            .iter()
            .map(|id| uuid_bytes(id))
            .collect::<Result<_, _>>()?,
        token: input.token.as_ref().map(|token| RunnerTokenPlan {
            name: token.name.clone(),
            accounts: token.accounts.clone(),
            days: token.days,
        }),
    })
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
    let bytes = read_settings()?;
    let mut input: Input = parse(&bytes)?;
    drop(bytes);
    let plan = plan(&input)?;
    let database = input.database.server()?;
    let list = input.list;
    drop(input);
    let db = connect_server(database).await.map_err(|_| ())?;
    if list {
        let tokens = ai_runner_tokens(&db).await.map_err(|_| ());
        let closed = db.close().await.map_err(|_| ());
        let tokens = tokens?;
        closed?;
        for token in &tokens {
            println!("{}", listed(token));
        }
        eprintln!("central: {} runner tokens listed", tokens.len());
        return Ok(());
    }
    let secret = plan
        .token
        .as_ref()
        .map(|_| Token::generate())
        .transpose()
        .map_err(|_| ())?;
    let applied = apply_ai_plan(&db, &plan, secret.as_ref())
        .await
        .map_err(|_| ());
    let closed = db.close().await.map_err(|_| ());
    let applied = applied?;
    report(&applied);
    // Committed: the token works now, so its text is shown, this once.
    if let (Some(secret), Some(_)) = (secret, applied.token) {
        println!("{}", *token_text(&secret));
    }
    closed
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
        assert!(plan(&input(&format!(r#"{{{LOGIN},"list":true}}"#))).is_ok());
        assert!(plan(&input(&format!(r"{{{LOGIN}}}"))).is_err());
        assert!(
            plan(&input(&format!(
                r#"{{{LOGIN},"list":true,"revoke":["0192f0c4-7b1a-7cde-8f00-000000000001"]}}"#
            )))
            .is_err()
        );
    }

    #[test]
    fn a_whole_plan_reads_as_its_parts() {
        let read = plan(&input(&format!(
            r#"{{{LOGIN},
              "players":[{{"display_name":"KI-Haus-Expert","tag":6699,"kind":"house",
                "profile":"house:expert","label":"Haus-KI · Expert","max_games":4,"enabled":true}}],
              "settings":{{"fill_in":false,"carousel_games":8,"carousel_daily":10000,"global_seats":16}},
              "revoke":["0192f0c47b1a7cde8f00000000000001"],
              "token":{{"name":"runner-1","accounts":["KI-Haus-Expert"]}}}}"#
        )))
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
