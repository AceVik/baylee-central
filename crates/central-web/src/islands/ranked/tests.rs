//! What the leaderboards ask, what they accept, and what they draw. The
//! browser suite's stand-in for Central (`browser/e2e-site/ranked.spec.ts`)
//! builds its answers from `ranked.json`; the shapes are checked here
//! against `central-api`'s own types, so a stand-in that drifted from
//! Central would fail here and not pass in the browser.

use central_api::ranked::ai::BoardPlayers;
use central_api::ranked::{self as api, Format, League, Row, Season, Track};
use central_api::wire;
use leptos::prelude::*;
use serde_json::{Value, json};

use super::choice::Choice;
use super::{Board, PAGE, Shown, board_request, fits, refused};
use crate::islands::central::Failure;
use crate::words::{self, Lang};

const FIXTURE: &str = include_str!("../../../browser/e2e-site/ranked.json");

fn fixture() -> Value {
    serde_json::from_str(FIXTURE).expect("ranked.json is JSON")
}

/// A row of the fixture, read strictly as Central's own type.
fn row(name: &str) -> Row {
    serde_json::from_value(fixture()[name].clone()).expect("a row of Central's shape")
}

fn season(name: &str) -> Season {
    serde_json::from_value(fixture()[name].clone()).expect("a season of Central's shape")
}

/// Every part of the fixture is exactly what Central writes: read as
/// `central-api`'s types and written again, it is the same JSON, so no
/// field is missing, extra or of another shape. The answers wrap them as
/// the stand-in does.
#[test]
fn the_stand_in_s_fixture_is_central_s_own_shape() {
    let all = fixture();
    for name in ["person", "ai", "deck", "placing"] {
        let again = serde_json::to_value(row(name)).expect("a row writes");
        assert_eq!(again, all[name], "{name}");
    }
    for name in ["season", "older"] {
        let again = serde_json::to_value(season(name)).expect("a season writes");
        assert_eq!(again, all[name], "{name}");
    }
    let person = row("person");
    let board = api::Response::Board {
        season: season("season"),
        entries: vec![person.clone()],
        next: Some(api::Cursor {
            elo: person.standing.elo,
            account: person.account,
            deck: None,
        }),
    };
    let answer = wire::Response::Ranked {
        response: board.clone(),
    };
    let written = json!({
        "result": "ranked",
        "response": { "board": {
            "season": all["season"],
            "entries": [all["person"]],
            "next": { "elo": 2310, "account": all["person"]["account"], "deck": null },
        } },
    });
    assert_eq!(serde_json::to_value(&answer).expect("writes"), written);
    let read: wire::Response = serde_json::from_value(written).expect("reads");
    let wire::Response::Ranked { response } = read else {
        panic!("a ranked answer");
    };
    assert_eq!(response, board);
    let seasons = json!({
        "result": "ranked",
        "response": { "seasons": { "items": [all["season"], all["older"]], "next": null } },
    });
    let read: wire::Response = serde_json::from_value(seasons).expect("seasons read");
    assert!(matches!(
        read,
        wire::Response::Ranked {
            response: api::Response::Seasons { ref items, next: None },
        } if items.len() == 2
    ));
}

/// The question is Central's public board read, as the stand-in reads it:
/// no `players` for everyone, the division only for the league named.
#[test]
fn the_question_is_the_public_board_read() {
    let asked = board_request(Choice::default(), League::Champion, None);
    assert_eq!(
        serde_json::to_value(&asked).expect("writes"),
        json!({ "operation": "ranked", "request": { "board": {
            "season": null, "format": "standard", "track": "player",
            "league": "champion", "division": null, "after": null,
        } } })
    );
    let wire::Request::Ranked { request } = &asked else {
        panic!("a ranked request");
    };
    assert!(request.is_public(), "the board needs no session");
    let named = Choice {
        format: Format::Commander,
        track: Track::Deck,
        players: BoardPlayers::Ai,
        league: Some(League::Gold),
        division: Some(3),
        season: Some([7; 16]),
    };
    let value = serde_json::to_value(board_request(named, League::Gold, None)).expect("writes");
    let board = &value["request"]["board"];
    assert_eq!(board["players"], "ai");
    assert_eq!(board["division"], 3);
    assert_eq!(board["track"], "deck");
    // searching from the top asks whole leagues: a division belongs to the named league
    let other = serde_json::to_value(board_request(named, League::Champion, None)).expect("w");
    assert_eq!(other["request"]["board"]["division"], Value::Null);
}

/// An answer is taken only when it is the board asked for.
#[test]
fn an_answer_of_another_board_is_refused() {
    let choice = Choice::default();
    let person = row("person");
    let ai = row("ai");
    assert!(fits(
        &choice,
        League::Master,
        None,
        &[ai.clone(), person.clone()]
    ));
    assert!(
        !fits(&choice, League::Gold, None, std::slice::from_ref(&person)),
        "another league"
    );
    let highlander = Choice {
        format: Format::Highlander,
        ..choice
    };
    assert!(!fits(
        &highlander,
        League::Master,
        None,
        std::slice::from_ref(&person)
    ));
    let people = Choice {
        players: BoardPlayers::People,
        ..choice
    };
    assert!(
        !fits(&people, League::Master, None, std::slice::from_ref(&ai)),
        "an AI among people"
    );
    let ais = Choice {
        players: BoardPlayers::Ai,
        ..choice
    };
    assert!(
        !fits(&ais, League::Master, None, std::slice::from_ref(&person)),
        "a person among AIs"
    );
    let decks = Choice {
        track: Track::Deck,
        ..choice
    };
    let deck = row("deck");
    assert!(fits(
        &decks,
        League::Gold,
        Some(2),
        std::slice::from_ref(&deck)
    ));
    assert!(
        !fits(&decks, League::Gold, Some(1), std::slice::from_ref(&deck)),
        "another division"
    );
    assert!(
        !fits(&choice, League::Gold, None, std::slice::from_ref(&deck)),
        "a deck on the players' board"
    );
    let too_many = vec![person; PAGE + 1];
    assert!(!fits(&choice, League::Master, None, &too_many));
}

/// No running season is a state of its own; a refusal of a season the
/// visitor named is a failure.
#[test]
fn no_running_season_is_no_failure() {
    let choice = Choice::default();
    assert_eq!(refused(choice, Failure::Rejected), Shown::NoSeason);
    assert_eq!(refused(choice, Failure::Busy), Shown::Failed(Failure::Busy));
    let named = Choice {
        season: Some([1; 16]),
        ..choice
    };
    assert_eq!(
        refused(named, Failure::Rejected),
        Shown::Failed(Failure::Rejected)
    );
}

fn drawn(board: Board) -> String {
    let owner = Owner::new();
    owner.with(|| {
        let ladder = super::Ladder::new(Lang::En);
        ladder.board.set(board);
        ladder.busy.set(false);
        super::rows::board(ladder).to_html()
    })
}

/// A row says who stands there and links the public profile; an AI
/// player's carries its badge, a person's none; a deck shows its own id;
/// a player in placement shows the games of the ten.
#[test]
fn a_row_says_who_and_marks_an_ai() {
    let html = drawn(Board {
        choice: Choice::default(),
        page: 0,
        shown: Shown::Page {
            season: season("season"),
            league: League::Master,
            entries: vec![row("ai"), row("person")],
            next: None,
        },
    });
    assert!(html.contains("/en/player/KI-Haus-Expert-0a1a"), "{html}");
    assert!(html.contains("/en/player/Erika-1000"), "{html}");
    assert_eq!(html.matches("ai-badge").count(), 1, "{html}");
    assert!(html.contains("Haus-KI · Expert"), "{html}");
    assert!(html.contains("2,420"), "{html}");
    assert!(
        html.contains(&words::ranked_page_count(Lang::En, 2, "1")),
        "{html}"
    );
    assert!(html.contains(r#"data-rank-kind="badge""#), "{html}");
    let decks = drawn(Board {
        choice: Choice {
            track: Track::Deck,
            ..Choice::default()
        },
        page: 1,
        shown: Shown::Page {
            season: season("season"),
            league: League::Gold,
            entries: vec![row("deck")],
            next: None,
        },
    });
    assert!(
        decks.contains(&words::ranked_deck(Lang::En, "AB12CD34")),
        "{decks}"
    );
    assert!(
        decks.contains(r#"data-place="41""#),
        "the second page counts on: {decks}"
    );
    let placing = drawn(Board {
        choice: Choice::default(),
        page: 0,
        shown: Shown::Page {
            season: season("season"),
            league: League::Unranked,
            entries: vec![row("placing")],
            next: None,
        },
    });
    assert!(
        placing.contains(&words::ranked_placements(Lang::En, "7")),
        "{placing}"
    );
    assert!(
        placing.contains(words::ranked_unnamed(Lang::En)),
        "{placing}"
    );
    assert!(
        !placing.contains("/en/player/"),
        "no handle, no profile: {placing}"
    );
}

/// Waiting, empty, seasonless, busy and failed each say so in words of
/// their own, and nothing of one is drawn for another.
#[test]
fn every_state_without_rows_says_its_own_words() {
    let lang = Lang::En;
    let choice = Choice::default();
    let state = |shown: Shown| {
        drawn(Board {
            choice,
            page: 0,
            shown,
        })
    };
    let waiting = state(Shown::Waiting);
    assert!(waiting.contains("is-skeleton"), "{waiting}");
    assert!(waiting.contains(words::ranked_loading(lang)), "{waiting}");
    let sentences = [
        (
            state(Shown::Empty {
                season: season("season"),
                league: None,
            }),
            words::ranked_empty_season(lang, "Standard"),
        ),
        (
            state(Shown::Empty {
                season: season("season"),
                league: Some(League::Gold),
            }),
            words::ranked_empty(lang).to_owned(),
        ),
        (
            state(Shown::NoSeason),
            words::ranked_no_season(lang).to_owned(),
        ),
        (
            state(Shown::Failed(Failure::Busy)),
            words::ranked_busy(lang).to_owned(),
        ),
        (
            state(Shown::Failed(Failure::Unavailable)),
            words::ranked_failed(lang).to_owned(),
        ),
    ];
    for (at, (html, sentence)) in sentences.iter().enumerate() {
        assert!(html.contains(sentence.as_str()), "{sentence}: {html}");
        assert!(!html.contains("is-skeleton"), "{html}");
        for (other, (_, said)) in sentences.iter().enumerate() {
            if other != at && said != sentence {
                assert!(
                    !html.contains(&format!(">{said}<")),
                    "{sentence} also says {said}"
                );
            }
        }
    }
    let failed = &sentences[4].0;
    assert!(failed.contains(words::ranked_again(lang)), "{failed}");
    let empty_everywhere = &sentences[0].0;
    assert!(
        empty_everywhere.contains("/en/download"),
        "{empty_everywhere}"
    );
}
