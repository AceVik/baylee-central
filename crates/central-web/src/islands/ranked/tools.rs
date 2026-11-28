//! Choosing a board: the format above, then the ladder, who stands on it,
//! the league and the season, each a control of the same height in a grid
//! of equal columns, so nothing moves when a choice changes. The pressed
//! chips are the directory's (`.chip-btn`), the two menus its sort field
//! (`.dir-sort`).

use central_api::ranked::ai::BoardPlayers;
use central_api::ranked::{Format, League, Track};
use leptos::prelude::*;

use super::super::chat::glyph::{self, glyph};
use super::super::time;
use super::super::ui::badges::{format_name, league_name};
use super::choice::{Choice, TOP_DOWN, divisions, league_key};
use super::{Ladder, Seasons};
use crate::data::id_hex;
use crate::words;

/// A pressed or unpressed choice among few.
fn chip(
    label: &'static str,
    pressed: impl Fn() -> bool + Send + Sync + 'static,
    press: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    view! {
        <button
            type="button"
            class="chip-btn"
            aria-pressed=move || if pressed() { "true" } else { "false" }
            on:click=move |_| press()
        >
            {label}
        </button>
    }
}

fn formats(ladder: Ladder) -> impl IntoView {
    let choice = ladder.choice;
    Format::ALL
        .into_iter()
        .map(|format| {
            chip(
                format_name(format),
                move || choice.get().format == format,
                move || {
                    ladder.choose(Choice {
                        format,
                        ..choice.get_untracked()
                    });
                },
            )
        })
        .collect_view()
}

fn tracks(ladder: Ladder) -> impl IntoView {
    let lang = ladder.lang;
    let choice = ladder.choice;
    [
        (Track::Player, words::ranked_track_players(lang)),
        (Track::Deck, words::ranked_track_decks(lang)),
    ]
    .into_iter()
    .map(|(track, label)| {
        chip(
            label,
            move || choice.get().track == track,
            move || {
                ladder.choose(Choice {
                    track,
                    ..choice.get_untracked()
                });
            },
        )
    })
    .collect_view()
}

fn who(ladder: Ladder) -> impl IntoView {
    let lang = ladder.lang;
    let choice = ladder.choice;
    [
        (BoardPlayers::Everyone, words::ranked_who_everyone(lang)),
        (BoardPlayers::People, words::ranked_who_people(lang)),
        (BoardPlayers::Ai, words::ranked_who_ai(lang)),
    ]
    .into_iter()
    .map(|(players, label)| {
        chip(
            label,
            move || choice.get().players == players,
            move || {
                ladder.choose(Choice {
                    players,
                    ..choice.get_untracked()
                });
            },
        )
    })
    .collect_view()
}

/// An option of the league menu, selected while it is the choice.
fn league_option(ladder: Ladder, value: String, label: String) -> AnyView {
    let choice = ladder.choice;
    let mine = value.clone();
    view! {
        <option value=value prop:selected=move || choice.get().league_value() == mine>
            {label}
        </option>
    }
    .into_any()
}

/// The league menu: the highest league with standings first, then every
/// league from the top down; a league with divisions is a group of its
/// own, all of them first.
fn leagues(ladder: Ladder) -> impl IntoView {
    let lang = ladder.lang;
    let top = league_option(
        ladder,
        String::new(),
        words::ranked_league_top(lang).to_owned(),
    );
    let rest = TOP_DOWN
        .into_iter()
        .map(|league: League| {
            let name = league_name(lang, league);
            let key = league_key(league);
            let count = divisions(league);
            if count == 0 {
                return league_option(ladder, key.to_owned(), name.to_owned());
            }
            let all = league_option(ladder, key.to_owned(), words::ranked_league_all(lang, name));
            let each = (1..=count)
                .map(|division| {
                    let rank = central_api::ranked::Rank {
                        league,
                        division: Some(division),
                        champion_place: None,
                    };
                    league_option(
                        ladder,
                        format!("{key}-{division}"),
                        super::super::ui::badges::rank_title(lang, rank),
                    )
                })
                .collect_view();
            view! { <optgroup label=name>{all}{each}</optgroup> }.into_any()
        })
        .collect_view();
    view! { {top}{rest} }
}

/// The season menu: the running one first, then every season Central
/// lists, newest first, with its dates.
fn seasons(ladder: Ladder) -> impl IntoView {
    let lang = ladder.lang;
    let choice = ladder.choice;
    let current = view! {
        <option value="" prop:selected=move || choice.get().season.is_none()>
            {words::ranked_season_current(lang)}
        </option>
    };
    let listed = move || {
        let Seasons::Known(items) = ladder.seasons.get() else {
            return Vec::new();
        };
        items
            .into_iter()
            .map(|season| {
                let id = season.id;
                let from = time::format_unix_date_local(lang, season.starts);
                let label = match season.ends {
                    Some(ends) => words::ranked_season_span(
                        lang,
                        &season.name,
                        &from,
                        &time::format_unix_date_local(lang, ends.saturating_sub(1)),
                    ),
                    None => words::ranked_season_since(lang, &season.name, &from),
                };
                view! {
                    <option value=id_hex(&id) prop:selected=move || choice.get().season == Some(id)>
                        {label}
                    </option>
                }
            })
            .collect::<Vec<_>>()
    };
    view! { {current}{listed} }
}

/// The toolbar.
pub(super) fn tools(ladder: Ladder) -> impl IntoView {
    let lang = ladder.lang;
    let choice = ladder.choice;
    let pick_league = move |ev: leptos::ev::Event| {
        let value = event_target_value(&ev);
        ladder.choose(choice.get_untracked().with_league_value(&value));
    };
    let pick_season = move |ev: leptos::ev::Event| {
        let value = event_target_value(&ev);
        ladder.choose(Choice {
            season: crate::data::id_from_hex(&value),
            ..choice.get_untracked()
        });
    };
    view! {
        <div class="ladder-tools glass" role="group" aria-label=words::ranked_tools(lang)>
            <div class="ladder-formats" role="group" aria-label=words::ranked_format(lang)>
                {formats(ladder)}
            </div>
            <div class="ladder-fields">
                <div class="ladder-field">
                    <span class="ladder-label" id="ladder-track-label">{words::ranked_track(lang)}</span>
                    <div class="ladder-seg" role="group" aria-labelledby="ladder-track-label">
                        {tracks(ladder)}
                    </div>
                </div>
                <div class="ladder-field">
                    <span class="ladder-label" id="ladder-who-label">{words::ranked_who(lang)}</span>
                    <div class="ladder-seg ladder-seg-3" role="group" aria-labelledby="ladder-who-label">
                        {who(ladder)}
                    </div>
                </div>
                <div class="ladder-field">
                    <span class="ladder-label" aria-hidden="true">{words::ranked_league(lang)}</span>
                    <div class="dir-sort ladder-pick">
                        <label for="ladder-league">
                            {glyph(glyph::TROPHY)}
                            <span class="visually-hidden">{words::ranked_league(lang)}</span>
                        </label>
                        <select id="ladder-league" on:change=pick_league>{leagues(ladder)}</select>
                    </div>
                </div>
                <div class="ladder-field">
                    <span class="ladder-label" aria-hidden="true">{words::ranked_season(lang)}</span>
                    <div class="dir-sort ladder-pick">
                        <label for="ladder-season">
                            {glyph(glyph::CALENDAR)}
                            <span class="visually-hidden">{words::ranked_season(lang)}</span>
                        </label>
                        <select id="ladder-season" on:change=pick_season>{seasons(ladder)}</select>
                    </div>
                </div>
            </div>
        </div>
    }
}
