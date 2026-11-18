//! Public calendar and rankings, with account-owned progress loaded only after hydration.
use super::{auth::problem_text, central::Failure, export::request, session};
use crate::{
    data::ChallengeFeed,
    words::{self, Lang},
};
use central_api::challenges as api;
use leptos::prelude::*;

mod detail;
mod progress;
mod state;
use state::Hub;

pub(in crate::islands) fn words_text(lang: Lang, words: &api::Words) -> String {
    if lang == Lang::De {
        words.de.clone()
    } else {
        words.en.clone()
    }
}

pub(in crate::islands) fn standing_handle(name: &str, tag: Option<u32>) -> String {
    tag.map_or_else(
        || name.to_owned(),
        |tag| central_api::profile::handle(name, tag),
    )
}

pub(in crate::islands) fn standing_duration(milliseconds: Option<u64>) -> String {
    milliseconds.map_or_else(
        || "—".into(),
        |ms| format!("{}:{:02}.{:03}", ms / 60_000, ms / 1000 % 60, ms % 1000),
    )
}

fn kind(lang: Lang, kind: api::Kind) -> &'static str {
    match kind {
        api::Kind::Daily => words::ch_daily(lang),
        api::Kind::Weekly => words::ch_weekly(lang),
    }
}

fn play_link(id: [u8; 16]) -> String {
    format!("baylee://challenge/{}", crate::data::id_hex(&id))
}

#[island]
pub fn Challenges(lang: Lang, initial: Option<ChallengeFeed>) -> impl IntoView {
    let hub = Hub::new(initial);
    Effect::new(move |_| {
        hub.busy.set(false);
        hub.refresh(None, false);
        let timer = set_interval(
            move || {
                if hub.confirm_hint.get_untracked().is_none() {
                    hub.refresh(hub.page_cursor(), false);
                }
            },
            std::time::Duration::from_secs(60),
        )
        .ok();
        #[cfg(target_arch = "wasm32")]
        session::on_change(move || {
            hub.session_changed();
        });
        on_cleanup(move || {
            if let Some(timer) = timer {
                timer.clear();
            }
        });
    });
    view! {
        <div class="challenge-toolbar">
            <button id="challenge-refresh" type="button" class="button button-ghost" disabled=move || hub.busy.get()
                on:click=move |_| hub.refresh(None, true)>{words::fb_refresh(lang)}</button>
            <span class="meta challenge-status" role="status">{move || hub.is_loading().then(|| words::ch_loading(lang))}</span>
        </div>
        <p id="challenge-error" class="error" role="alert">{move || hub.error.get().map(|failure| problem_text(lang, failure.into()))}</p>
        {move || match hub.selected.get() {
            Some(task) => detail::view(lang, hub, &task).into_any(),
            None => calendar(lang, hub).into_any(),
        }}
        {progress::view(lang, hub)}
        <noscript><p>{words::ch_play_help(lang)}</p></noscript>
    }
}

fn calendar(lang: Lang, hub: Hub) -> impl IntoView {
    view! {
        {move || match hub.feed.get() {
            None => view! { <p class="empty-state">{words::ch_unavailable(lang)}</p> }.into_any(),
            Some(feed) => {
                let current: Vec<_> = feed.challenges.iter().filter(|c| c.description.period.live_at(feed.now)).cloned().collect();
                let archive: Vec<_> = feed.challenges.iter().filter(|c| !c.description.period.live_at(feed.now)).cloned().collect();
                let returning_to_archive = archive.iter().any(|task| Some(task.id) == hub.return_target());
                let before = feed.before;
                view! {
                    <div class="challenge-grid">{current.into_iter().map(|task| card(lang, hub, task, true)).collect_view()}</div>
                    {feed.challenges.is_empty().then(|| view! { <p class="empty-state">{words::ch_empty(lang)}</p> })}
                    <details class="challenge-archive" open=current_is_absent(&feed) || returning_to_archive>
                        <summary>{words::ch_archive(lang)}</summary>
                        <div class="challenge-grid">{archive.into_iter().map(|task| card(lang, hub, task, false)).collect_view()}</div>
                        {before.map(|cursor| view! { <button type="button" class="button button-ghost" disabled=move || hub.busy.get()
                            on:click=move |_| hub.refresh(Some(cursor), true)>{words::ch_older(lang)}</button> })}
                    </details>
                }.into_any()
            }
        }}
    }
}

fn current_is_absent(feed: &ChallengeFeed) -> bool {
    !feed
        .challenges
        .iter()
        .any(|c| c.description.period.live_at(feed.now))
}

fn card(lang: Lang, hub: Hub, task: api::Challenge, live: bool) -> impl IntoView {
    let title = words_text(lang, &task.description.title);
    let description = words_text(lang, &task.description.instructions);
    let date = super::account::day(lang, task.description.period.starts_at);
    let cadence = kind(lang, task.description.period.kind);
    let withdrawn = task.withdrawn;
    let id = task.id;
    view! {
        <article class="card challenge-card">
            <p class="eyebrow">{cadence}<span class="num">{date}</span></p>
            <h2>{title}</h2>
            <p>{description}</p>
            <p class="meta">{if withdrawn { words::ch_withdrawn(lang) } else if live { words::ch_current(lang) } else { words::ch_archive(lang) }}</p>
            <div class="challenge-actions">
                <button id=format!("challenge-open-{}",crate::data::id_hex(&id)) type="button" class="button" disabled=move || hub.busy.get()
                    on:click=move |_| hub.open(task.clone(), api::Board::FirstWin, 0)>{words::ch_details(lang)}</button>
                {(!withdrawn).then(|| view! { <a class="button button-ghost" href=play_link(id)>{words::ch_play(lang)}</a> })}
            </div>
        </article>
    }
}
