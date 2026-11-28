//! The board: its head (the league's badge, its name, the format, ladder
//! and season), its rows, and the pages. Each row has the same height and
//! the same columns, figures in tabular digits; while the next answer is
//! on its way the rows on screen stay, dimmed, and the first answer has
//! rows of the same shape to wait in. Empty, failed, busy and seasonless
//! boards each say so in words of their own, with the way on.

use central_api::profile::tag_text;
use central_api::ranked::ai::BoardPlayers;
use central_api::ranked::{League, Rank, Row, Season, Track};
use leptos::prelude::*;

use super::super::central::Failure;
use super::super::chat::glyph::{self, glyph};
use super::super::time;
use super::super::ui::ai_badge::ai_badge;
use super::super::ui::badges::{format_name, rank_title};
use super::super::ui::copy_handle::player_path;
use super::super::ui::rank_avatar::{AvatarSize, rank_avatar, rank_canvas};
use super::choice::Choice;
use super::{Board, Ladder, PAGE, Seasons, Shown};
use crate::words::{self, Lang};

/// Rows the first answer waits in.
const WAITING_ROWS: usize = 8;

/// The rank a board of `league` is drawn with: the division the visitor
/// named, if any.
fn board_rank(choice: &Choice, league: League) -> Rank {
    Rank {
        league,
        division: super::division_of(choice, league),
        champion_place: None,
    }
}

/// "2026-11-25 · since 25 November 2026" or with its end. A name that is
/// a date keeps together (non-breaking hyphens, docs/website-design.md
/// §"Typography").
fn season_line(lang: Lang, season: &Season) -> String {
    let from = time::format_unix_date_local(lang, season.starts);
    let name = season.name.replace('-', "\u{2011}");
    match season.ends {
        Some(ends) => words::ranked_season_span(
            lang,
            &name,
            &from,
            &time::format_unix_date_local(lang, ends.saturating_sub(1)),
        ),
        None => words::ranked_season_since(lang, &name, &from),
    }
}

fn track_word(lang: Lang, track: Track) -> &'static str {
    match track {
        Track::Player => words::ranked_track_players(lang),
        Track::Deck => words::ranked_track_decks(lang),
    }
}

/// The board's head: the league's badge over its name, and what board it
/// is. Before a league is known, the trophy and the format.
fn head(lang: Lang, board: &Board) -> AnyView {
    let choice = &board.choice;
    let (league, season) = match &board.shown {
        Shown::Page { league, season, .. } => (Some(*league), Some(season)),
        Shown::Empty { league, season } => (*league, Some(season)),
        Shown::Waiting | Shown::NoSeason | Shown::Failed(_) => (choice.league, None),
    };
    let deck = choice.track == Track::Deck;
    let rank = league.map(|league| board_rank(choice, league));
    let crest = rank.map_or_else(
        || glyph(glyph::TROPHY),
        |rank| rank_canvas("badge", rank, deck, 160),
    );
    let title = rank.map_or_else(
        || format_name(choice.format).to_owned(),
        |rank| rank_title(lang, rank),
    );
    let mut meta = vec![
        format_name(choice.format).to_owned(),
        track_word(lang, choice.track).to_owned(),
    ];
    match choice.players {
        BoardPlayers::Everyone => {}
        BoardPlayers::People => meta.push(words::ranked_who_people(lang).to_owned()),
        BoardPlayers::Ai => meta.push(words::ranked_who_ai(lang).to_owned()),
    }
    if let Some(season) = season {
        meta.push(season_line(lang, season));
    }
    view! {
        <header class="ladder-head">
            <div class="ladder-crest" class:is-trophy=rank.is_none() aria-hidden="true">{crest}</div>
            <h2 id="ladder-title" tabindex="-1">{title}</h2>
            <p class="ladder-meta">{meta.join(" · ")}</p>
        </header>
    }
    .into_any()
}

/// The place: Champion's published place, else the row's place on this
/// board; a medal before the first three.
fn place(lang: Lang, row: &Row, ordinal: usize) -> AnyView {
    let place = row
        .standing
        .rank
        .champion_place
        .map_or(ordinal, usize::from);
    let medal = (place <= 3).then(|| glyph(glyph::MEDAL));
    let said = words::ranked_place(lang, &place.to_string());
    view! {
        <span class="ladder-place num" class:is-podium=place <= 3 data-place=place.to_string()>
            {medal}
            <span aria-hidden="true">{place.to_string()}</span>
            <span class="visually-hidden">{said}</span>
        </span>
    }
    .into_any()
}

/// Who stands there: the handle, a link to the public profile, with the
/// AI badge of an AI player; under it the rank, and a deck's own id.
fn who(lang: Lang, row: &Row) -> AnyView {
    let standing = &row.standing;
    let name = match (&row.name, row.tag) {
        (Some(name), Some(tag)) => {
            let tag = tag_text(tag);
            let href = player_path(lang, name, &tag);
            view! {
                <a class="handle-link ladder-handle" href=href translate="no">
                    <bdi>{name.clone()}</bdi>
                    <span class="tag handle-tag">{"#"}{tag}</span>
                </a>
            }
            .into_any()
        }
        (Some(name), None) => {
            view! { <span class="handle-text" translate="no"><bdi>{name.clone()}</bdi></span> }
                .into_any()
        }
        (None, _) => {
            view! { <span class="ladder-unnamed">{words::ranked_unnamed(lang)}</span> }.into_any()
        }
    };
    // an AI account is never shown without its label (docs/ai-players.md §6)
    let ai = row.ai.as_ref().map(|badge| ai_badge(lang, badge));
    let mut sub = rank_title(lang, standing.rank);
    if let Some(deck) = standing.deck {
        let id = format!(
            "{:02X}{:02X}{:02X}{:02X}",
            deck[0], deck[1], deck[2], deck[3]
        );
        sub = format!("{sub} · {}", words::ranked_deck(lang, &id));
    }
    view! {
        <span class="ladder-who">
            <span class="ladder-name">{name}{ai}</span>
            <span class="ladder-sub">{sub}</span>
        </span>
    }
    .into_any()
}

/// Wins, losses and draws of the season, and its games; in placement the
/// games of the ten.
fn record(lang: Lang, row: &Row) -> AnyView {
    let standing = &row.standing;
    if standing.rank.league == League::Unranked {
        let placed = words::ranked_placements(lang, &standing.placement_games.to_string());
        return view! {
            <span class="ladder-record num">
                <span class="ladder-tally">{placed}</span>
            </span>
        }
        .into_any();
    }
    let n = |count: u32| words::number(lang, u64::from(count));
    let said = words::ranked_record_said(
        lang,
        &n(standing.wins),
        &n(standing.losses),
        &n(standing.draws),
    );
    let games = words::ranked_games(lang, u64::from(standing.games));
    view! {
        <span class="ladder-record num">
            <span class="ladder-tally" aria-hidden="true">
                {n(standing.wins)}<abbr>{words::ranked_wins_short(lang)}</abbr>" · "
                {n(standing.losses)}<abbr>{words::ranked_losses_short(lang)}</abbr>" · "
                {n(standing.draws)}<abbr>{words::ranked_draws_short(lang)}</abbr>
            </span>
            <span class="visually-hidden">{said}</span>
            <span class="ladder-games">{games}</span>
        </span>
    }
    .into_any()
}

fn row(lang: Lang, entry: &Row, ordinal: usize) -> AnyView {
    let standing = &entry.standing;
    let deck = standing.track == Track::Deck;
    let mark = match (&entry.name, entry.tag, deck) {
        (_, _, true) => view! {
            <span class="rank-avatar rank-avatar-sm ladder-deck" aria-hidden="true">
                <span class="rank-avatar-plate">{glyph(glyph::LAYERS)}</span>
            </span>
        }
        .into_any(),
        (name, tag, false) => {
            let shown = name.clone().unwrap_or_default();
            let seed = tag.map_or_else(
                || crate::data::id_hex(&entry.account),
                |tag| format!("{shown}#{}", tag_text(tag)),
            );
            rank_avatar(shown, &seed, None, Some(standing.rank), AvatarSize::Small)
        }
    };
    let magnitude = words::number(lang, u64::from(standing.elo.unsigned_abs()));
    let elo = if standing.elo < 0 {
        format!("−{magnitude}")
    } else {
        magnitude
    };
    view! {
        <li class="ladder-row" class:is-ai=entry.ai.is_some()>
            {place(lang, entry, ordinal)}
            <span class="ladder-badge" aria-hidden="true">
                {rank_canvas("badge", standing.rank, deck, 88)}
            </span>
            {mark}
            {who(lang, entry)}
            {record(lang, entry)}
            <span class="ladder-elo num">
                <strong>{elo}</strong>
                <span>{words::ranked_elo(lang)}</span>
            </span>
        </li>
    }
    .into_any()
}

/// A grey stand-in for a part of a row (the classes are expressions: an
/// element of literals alone would be built through `innerHTML`, which
/// the site's Trusted Types refuse, `tests/inert.rs`).
fn bone(class: &'static str) -> impl IntoView {
    view! { <span class=class></span> }
}

/// Rows of the list's shape, for the first answer to arrive in.
fn waiting(lang: Lang) -> AnyView {
    let rows = (0..WAITING_ROWS)
        .map(|_| {
            view! {
                <li class="ladder-row is-skeleton">
                    <span class="ladder-place">{bone("bone")}</span>
                    <span class="ladder-badge">{bone("bone round")}</span>
                    <span class="rank-avatar rank-avatar-sm">{bone("bone round")}</span>
                    <span class="ladder-who">{bone("bone wide")}{bone("bone")}</span>
                    <span class="ladder-record">{bone("bone")}</span>
                    <span class="ladder-elo">{bone("bone")}</span>
                </li>
            }
        })
        .collect_view();
    view! {
        <p class="visually-hidden">{words::ranked_loading(lang)}</p>
        <ol class="ladder-rows" aria-hidden="true">{rows}</ol>
    }
    .into_any()
}

/// A board without rows: the trophy, what it means, and the ways on.
fn quiet(
    sentence: String,
    lead: Option<String>,
    ways: Vec<AnyView>,
    kind: &'static str,
) -> AnyView {
    view! {
        <div class="ladder-quiet" data-state=kind>
            <span class="ladder-quiet-mark" aria-hidden="true">
                {if matches!(kind, "failed" | "busy") { glyph(glyph::INFO) } else { glyph(glyph::TROPHY) }}
            </span>
            <p class="ladder-quiet-title">{sentence}</p>
            {lead.map(|lead| view! { <p class="ladder-quiet-lead">{lead}</p> })}
            {(!ways.is_empty()).then(|| view! { <div class="ladder-ways">{ways}</div> })}
        </div>
    }
    .into_any()
}

fn button(label: &'static str, press: impl Fn() + Send + Sync + 'static) -> AnyView {
    view! {
        <button type="button" class="button button-ghost button-sm" on:click=move |_| press()>
            {label}
        </button>
    }
    .into_any()
}

/// The ways on from an empty board: everyone where a filter narrowed it,
/// the highest league with standings where a league was named, and where
/// nobody stands anywhere, the game.
fn empty_ways(ladder: Ladder, choice: Choice, every_league: bool) -> Vec<AnyView> {
    let lang = ladder.lang;
    let mut ways = Vec::new();
    if choice.players != BoardPlayers::Everyone {
        ways.push(button(words::ranked_show_everyone(lang), move || {
            ladder.choose(Choice {
                players: BoardPlayers::Everyone,
                ..ladder.choice.get_untracked()
            });
        }));
    }
    if choice.league.is_some() {
        ways.push(button(words::ranked_show_top(lang), move || {
            ladder.choose(Choice {
                league: None,
                division: None,
                ..ladder.choice.get_untracked()
            });
        }));
    }
    if every_league && choice.players == BoardPlayers::Everyone {
        let href = format!("/{}/download", lang.code());
        ways.push(
            view! { <a class="button button-ghost button-sm" href=href>{words::ranked_play(lang)}</a> }
                .into_any(),
        );
    }
    ways
}

fn body(ladder: Ladder, board: &Board) -> AnyView {
    let lang = ladder.lang;
    let choice = board.choice;
    match &board.shown {
        Shown::Waiting => waiting(lang),
        Shown::Page {
            league, entries, ..
        } if !entries.is_empty() => {
            let first = board.page * PAGE;
            let label = words::ranked_list(
                lang,
                format_name(choice.format),
                &rank_title(lang, board_rank(&choice, *league)),
            );
            let rows = entries
                .iter()
                .enumerate()
                .map(|(at, entry)| row(lang, entry, first + at + 1))
                .collect_view();
            view! { <ol class="ladder-rows" aria-label=label>{rows}</ol> }.into_any()
        }
        Shown::Page { .. }
        | Shown::Empty {
            league: Some(_), ..
        } => quiet(
            words::ranked_empty(lang).to_owned(),
            None,
            empty_ways(ladder, choice, false),
            "empty",
        ),
        Shown::Empty { league: None, .. } => quiet(
            words::ranked_empty_season(lang, format_name(choice.format)),
            None,
            empty_ways(ladder, choice, true),
            "empty",
        ),
        Shown::NoSeason => {
            let lead = ladder
                .seasons
                .with(|seasons| matches!(seasons, Seasons::Known(items) if !items.is_empty()))
                .then(|| words::ranked_no_season_lead(lang).to_owned());
            quiet(
                words::ranked_no_season(lang).to_owned(),
                lead,
                Vec::new(),
                "no-season",
            )
        }
        Shown::Failed(failure) => {
            let sentence = if *failure == Failure::Busy {
                words::ranked_busy(lang)
            } else {
                words::ranked_failed(lang)
            };
            quiet(
                sentence.to_owned(),
                None,
                vec![button(words::ranked_again(lang), move || ladder.reload())],
                if *failure == Failure::Busy {
                    "busy"
                } else {
                    "failed"
                },
            )
        }
    }
}

/// What the live region says once a board stands: its page and places, or
/// the sentence of a board without rows.
fn said(lang: Lang, board: &Board) -> String {
    match &board.shown {
        Shown::Waiting => String::new(),
        Shown::Page { entries, .. } if !entries.is_empty() => words::ranked_page_count(
            lang,
            u64::try_from(entries.len()).unwrap_or(0),
            &(board.page + 1).to_string(),
        ),
        Shown::Page { .. }
        | Shown::Empty {
            league: Some(_), ..
        } => words::ranked_empty(lang).to_owned(),
        Shown::Empty { league: None, .. } => {
            words::ranked_empty_season(lang, format_name(board.choice.format))
        }
        Shown::NoSeason => words::ranked_no_season(lang).to_owned(),
        Shown::Failed(Failure::Busy) => words::ranked_busy(lang).to_owned(),
        Shown::Failed(_) => words::ranked_failed(lang).to_owned(),
    }
}

/// The pages: the one before, which page this is with its places, the
/// next. Both buttons always stand where they stand; one that has no page
/// to go to says so (`aria-disabled`) and keeps its place.
fn pager(ladder: Ladder) -> impl IntoView {
    let lang = ladder.lang;
    let before = move || !ladder.busy.get() && ladder.board.with(|board| board.page > 0);
    let after = move || {
        !ladder.busy.get()
            && ladder
                .board
                .with(|board| matches!(&board.shown, Shown::Page { next: Some(_), .. }))
    };
    let count = move || {
        ladder.board.with(|board| match &board.shown {
            Shown::Page { entries, .. } if !entries.is_empty() => words::ranked_page_count(
                lang,
                u64::try_from(entries.len()).unwrap_or(0),
                &(board.page + 1).to_string(),
            ),
            _ => words::ranked_page(lang, &(board.page + 1).to_string()),
        })
    };
    view! {
        <nav class="ladder-pager" aria-label=words::ranked_pages(lang)>
            <button
                type="button"
                class="button button-ghost button-sm ladder-turn"
                aria-disabled=move || if before() { "false" } else { "true" }
                on:click=move |_| ladder.turn(false)
            >
                {glyph(glyph::CHEVRON_LEFT)}
                <span>{words::ranked_previous(lang)}</span>
            </button>
            <span class="ladder-count num">{count}</span>
            <button
                type="button"
                class="button button-ghost button-sm ladder-turn"
                aria-disabled=move || if after() { "false" } else { "true" }
                on:click=move |_| ladder.turn(true)
            >
                <span>{words::ranked_next(lang)}</span>
                {glyph(glyph::CHEVRON_RIGHT)}
            </button>
        </nav>
    }
}

/// The board, with its reload in the corner.
pub(super) fn board(ladder: Ladder) -> AnyView {
    let lang = ladder.lang;
    view! {
        <section
            class="ladder glass"
            class:is-busy=move || ladder.busy.get()
            aria-labelledby="ladder-title"
            aria-busy=move || if ladder.busy.get() { "true" } else { "false" }
        >
            <button
                type="button"
                class="round-btn ladder-reload"
                aria-label=words::ranked_reload(lang)
                title=words::ranked_reload(lang)
                on:click=move |_| ladder.reload()
            >
                {glyph(glyph::REFRESH_CW)}
            </button>
            {move || ladder.board.with(|board| head(lang, board))}
            <div class="ladder-body">{move || ladder.board.with(|board| body(ladder, board))}</div>
            {pager(ladder)}
            <p class="visually-hidden" role="status">
                {move || ladder.board.with(|board| said(lang, board))}
            </p>
        </section>
    }
    .into_any()
}
