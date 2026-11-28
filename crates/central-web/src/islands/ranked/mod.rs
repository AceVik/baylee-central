//! The public leaderboards of ranked play (`/{lang}/ranked`, docs/ranked.md
//! §"Publication and storage", docs/website-design.md §"The leaderboards").
//! It asks Central's public `Board` and `Seasons`, the reads the Baylee
//! client's leaderboards make, through this origin's `/api/v1`: no session
//! and no new endpoint, under the directory's budget of sixty a minute.
//! Central serves one league (or one division) a page of forty at a time;
//! where the visitor named no league, the page asks from Champion down and
//! shows the first league anybody stands in, and remembers which it was.

mod choice;
mod rows;
mod tools;

use std::collections::HashMap;

use central_api::ranked::ai::BoardPlayers;
use central_api::ranked::{self as api, Cursor, League, Row, Season, Track};
use central_api::wire;
use leptos::prelude::*;

use self::choice::{Choice, Key, TOP_DOWN};
use super::central::{Failure, call};
use crate::words::Lang;

/// Rows Central sends in a page at most (`boards.rs`'s `PAGE`).
const PAGE: usize = 40;

/// What Central answered for the board on screen.
#[derive(Clone, Debug, PartialEq)]
enum Shown {
    /// Nothing yet: the list's shape stands in for it.
    Waiting,
    /// A page of a league.
    Page {
        /// The season Central resolved.
        season: Season,
        /// The league the page is of.
        league: League,
        /// Its rows, best first.
        entries: Vec<Row>,
        /// The next page, where there is one.
        next: Option<Cursor>,
    },
    /// Nobody stands in `league`, or with no league named, in any league
    /// of this season, format and ladder.
    Empty {
        /// The season Central resolved.
        season: Season,
        /// The league asked, or none when every league was.
        league: Option<League>,
    },
    /// No season runs now, and the visitor named none.
    NoSeason,
    /// Central gave no board: busy (429) or anything else.
    Failed(Failure),
}

/// The board on screen: what it was asked for, which page, and the answer.
#[derive(Clone, Debug, PartialEq)]
struct Board {
    /// The choice it answers.
    choice: Choice,
    /// Its page, from 0.
    page: usize,
    /// The answer.
    shown: Shown,
}

/// The seasons Central lists, newest first.
#[derive(Clone, Debug, PartialEq)]
enum Seasons {
    /// Asked for.
    Asking,
    /// The first page (forty seasons, years of play).
    Known(Vec<Season>),
    /// Not to be had; the running season still is.
    Failed,
}

/// The island's state, every part a signal or a value kept beside them.
#[derive(Clone, Copy)]
struct Ladder {
    lang: Lang,
    /// What the toolbar shows: the visitor's choice, at once.
    choice: RwSignal<Choice>,
    /// The cursors of the pages after the first one, in order.
    pages: RwSignal<Vec<Cursor>>,
    /// The board on screen, which stays while the next one loads.
    board: RwSignal<Board>,
    /// A board is being asked for.
    busy: RwSignal<bool>,
    seasons: RwSignal<Seasons>,
    /// The highest league with standings, for each board without its league.
    top: StoredValue<HashMap<Key, League>>,
    /// The number of the newest question: only its answer is shown.
    asked: StoredValue<u64>,
}

impl Ladder {
    fn new(lang: Lang) -> Self {
        Self {
            lang,
            choice: RwSignal::new(Choice::default()),
            pages: RwSignal::new(Vec::new()),
            board: RwSignal::new(Board {
                choice: Choice::default(),
                page: 0,
                shown: Shown::Waiting,
            }),
            busy: RwSignal::new(true),
            seasons: RwSignal::new(Seasons::Asking),
            top: StoredValue::new(HashMap::new()),
            asked: StoredValue::new(0),
        }
    }

    /// The address's choice, its board, and the seasons.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    fn start(self) {
        let search = leptos::web_sys::window()
            .and_then(|window| window.location().search().ok())
            .unwrap_or_default();
        self.choice.set(Choice::from_search(&search));
        self.load(false);
        let seasons = self.seasons;
        leptos::task::spawn_local(async move {
            seasons.set(match list_seasons().await {
                Ok(items) => Seasons::Known(items),
                Err(_) => Seasons::Failed,
            });
        });
    }

    /// Shows `next` from its first page, and puts it in the address.
    fn choose(self, next: Choice) {
        if next == self.choice.get_untracked() {
            return;
        }
        self.choice.set(next);
        self.pages.set(Vec::new());
        remember(&next);
        self.load(false);
    }

    /// The next page or the one before, when there is one.
    fn turn(self, forward: bool) {
        if self.busy.get_untracked() {
            return;
        }
        let next = self.board.with_untracked(|board| match &board.shown {
            Shown::Page { next, .. } => *next,
            _ => None,
        });
        match (forward, next) {
            (true, Some(cursor)) => self.pages.update(|pages| pages.push(cursor)),
            (false, _) if !self.pages.with_untracked(Vec::is_empty) => {
                self.pages.update(|pages| {
                    pages.pop();
                });
            }
            _ => return,
        }
        self.load(true);
    }

    /// Asks for the board of the current choice and page; the newest
    /// question's answer replaces the board, and after a page's turn the
    /// board's title takes the focus.
    fn load(self, turned: bool) {
        let ticket = self.asked.get_value() + 1;
        self.asked.set_value(ticket);
        self.busy.set(true);
        let choice = self.choice.get_untracked();
        let pages = self.pages.get_untracked();
        let after = pages.last().copied();
        let known = self.top.with_value(|top| top.get(&choice.key()).copied());
        leptos::task::spawn_local(async move {
            let shown = find(choice, after, known).await;
            if self.asked.get_value() != ticket {
                return;
            }
            if choice.league.is_none()
                && let Shown::Page { league, .. } = &shown
            {
                let league = *league;
                self.top.update_value(|top| {
                    top.insert(choice.key(), league);
                });
            }
            self.board.set(Board {
                choice,
                page: pages.len(),
                shown,
            });
            self.busy.set(false);
            if turned {
                #[cfg(target_arch = "wasm32")]
                super::focus::after_render("#ladder-title");
            }
        });
    }

    /// Asks again for what is on screen, from its first page.
    fn reload(self) {
        let choice = self.choice.get_untracked();
        self.top.update_value(|top| {
            top.remove(&choice.key());
        });
        self.pages.set(Vec::new());
        self.load(false);
    }
}

/// Puts `choice` in the address in place of the one there, so that a
/// reload or a shared link shows the same board.
fn remember(choice: &Choice) {
    #[cfg(target_arch = "wasm32")]
    if let Some(window) = leptos::web_sys::window()
        && let (Ok(history), Ok(path)) = (window.history(), window.location().pathname())
    {
        let address = format!("{path}{}", choice.to_search());
        let _ = history.replace_state_with_url(
            &leptos::wasm_bindgen::JsValue::NULL,
            "",
            Some(&address),
        );
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = choice;
}

/// The board for `choice`: its league, the league remembered as the
/// highest with standings, or else the first league from the top down
/// that has any.
async fn find(choice: Choice, after: Option<Cursor>, known: Option<League>) -> Shown {
    let Some(league) = choice.league.or(known) else {
        return search(choice).await;
    };
    match board(choice, league, after).await {
        // the remembered league has emptied since: search again
        Ok((_, entries, _)) if entries.is_empty() && choice.league.is_none() => {
            search(choice).await
        }
        Ok((season, entries, _)) if entries.is_empty() => Shown::Empty {
            season,
            league: Some(league),
        },
        Ok((season, entries, next)) => Shown::Page {
            season,
            league,
            entries,
            next,
        },
        Err(failure) => refused(choice, failure),
    }
}

/// The first league from the top down with anybody in it; at most one
/// question for each of the twelve leagues.
async fn search(choice: Choice) -> Shown {
    let mut resolved = None;
    for league in TOP_DOWN {
        match board(choice, league, None).await {
            Ok((season, entries, next)) if !entries.is_empty() => {
                return Shown::Page {
                    season,
                    league,
                    entries,
                    next,
                };
            }
            Ok((season, ..)) => resolved = Some(season),
            Err(failure) => return refused(choice, failure),
        }
    }
    resolved.map_or(Shown::Failed(Failure::Malformed), |season| Shown::Empty {
        season,
        league: None,
    })
}

/// What a refusal means here: Central refuses the running season's board
/// while no season runs (`boards.rs`'s `read`), which is no failure.
fn refused(choice: Choice, failure: Failure) -> Shown {
    if failure == Failure::Rejected && choice.season.is_none() {
        Shown::NoSeason
    } else {
        Shown::Failed(failure)
    }
}

/// The division asked of `league`: only the league the visitor named has
/// one.
fn division_of(choice: &Choice, league: League) -> Option<u8> {
    if choice.league == Some(league) {
        choice.division
    } else {
        None
    }
}

/// The request for one page of `league` as `choice` asks it.
fn board_request(choice: Choice, league: League, after: Option<Cursor>) -> wire::Request {
    let division = division_of(&choice, league);
    wire::Request::Ranked {
        request: api::Request::Board {
            season: choice.season,
            format: choice.format,
            track: choice.track,
            league,
            division,
            after,
            players: choice.players,
        },
    }
}

/// One page of `league`, checked against what was asked.
async fn board(
    choice: Choice,
    league: League,
    after: Option<Cursor>,
) -> Result<(Season, Vec<Row>, Option<Cursor>), Failure> {
    let division = division_of(&choice, league);
    match call(&board_request(choice, league, after)).await? {
        wire::Response::Ranked {
            response:
                api::Response::Board {
                    season,
                    entries,
                    next,
                },
        } if fits(&choice, league, division, &entries) => Ok((season, entries, next)),
        _ => Err(Failure::Malformed),
    }
}

/// Whether a page is one of the board asked for: at most a page of rows,
/// each of the format, ladder, league and division asked, a deck's row
/// with its deck, and only people or only AI players where that was asked.
fn fits(choice: &Choice, league: League, division: Option<u8>, entries: &[Row]) -> bool {
    entries.len() <= PAGE
        && entries.iter().all(|row| {
            let standing = &row.standing;
            standing.format == choice.format
                && standing.track == choice.track
                && standing.rank.league == league
                && division.is_none_or(|d| standing.rank.division == Some(d))
                && standing.deck.is_some() == (choice.track == Track::Deck)
                && match choice.players {
                    BoardPlayers::Everyone => true,
                    BoardPlayers::People => row.ai.is_none(),
                    BoardPlayers::Ai => row.ai.is_some(),
                }
        })
}

/// The seasons, newest first: the first page, forty of them.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
async fn list_seasons() -> Result<Vec<Season>, Failure> {
    match call(&wire::Request::Ranked {
        request: api::Request::Seasons { before: None },
    })
    .await?
    {
        wire::Response::Ranked {
            response: api::Response::Seasons { items, .. },
        } if items.len() <= PAGE => Ok(items),
        _ => Err(Failure::Malformed),
    }
}

/// The leaderboards: the toolbar that chooses a board, and the board.
#[island]
pub fn RankedLadder(lang: Lang) -> impl IntoView {
    let ladder = Ladder::new(lang);
    #[cfg(target_arch = "wasm32")]
    Effect::new(move |_| ladder.start());
    view! {
        <div class="ladder-page">
            {tools::tools(ladder)}
            {rows::board(ladder)}
        </div>
    }
}

#[cfg(test)]
mod tests;
