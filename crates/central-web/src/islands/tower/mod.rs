//! Global public expedition, with private progression loaded only in the browser.
use super::{
    auth::problem_text,
    central::Failure,
    challenges::{standing_duration, standing_handle, words_text},
    export::tower::request,
    session,
};
use crate::words::{self, Lang};
use central_api::tower as api;
use leptos::prelude::*;
mod progress;
mod ranking;
mod state;
use state::Hub;

fn play(id: [u8; 16]) -> String {
    format!("baylee://tower/{}", crate::data::id_hex(&id))
}

#[island]
pub fn Tower(
    lang: Lang,
    initial: Option<api::Season>,
    standings: Vec<api::Standing>,
) -> impl IntoView {
    let hub = Hub::new(initial, standings);
    Effect::new(move |_| {
        hub.busy.set(false);
        hub.refresh(false);
        let timer = set_interval(
            move || hub.refresh(false),
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
        <section class="tower-hub" aria-labelledby="tower-title">
            <header class="tower-header">
                <div><p class="eyebrow">{words::ch_weekly(lang)}</p><h2 id="tower-title">{words::tower_title(lang)}</h2></div>
                <div class="tower-header-actions">
                    <span class="meta" role="status">{move || hub.loading.get().then(|| words::ch_loading(lang))}</span>
                    <button type="button" class="button button-ghost" disabled=move || hub.busy.get()
                        on:click=move |_| hub.refresh(true)>{words::fb_refresh(lang)}</button>
                </div>
            </header>
            <p class="error" role="alert">{move || hub.error.get().map(|error| problem_text(lang, error.into()))}</p>
            {move || match hub.season.get() {
                None => view! { <p class="empty-state">{words::ch_empty(lang)}</p> }.into_any(),
                Some(season) => expedition(lang, hub, season).into_any(),
            }}
            <details class="tower-guide"><summary>{words::tower_guide(lang)}</summary><p>{words::tower_rules(lang)}</p></details>
            {progress::view(lang, hub)}
            {ranking::view(lang, hub)}
        </section>
    }
}

fn expedition(lang: Lang, hub: Hub, season: api::Season) -> impl IntoView {
    let id = season.id;
    let title = words_text(lang, &season.theme.title);
    let floor_names = season.theme.floors.map(|floor| words_text(lang, &floor));
    let starts = super::account::day(lang, season.theme.period.starts_at);
    let ends = super::account::day(lang, season.theme.period.ends_at);
    let cleared = Memo::new(move |_| {
        hub.run.with(|run| {
            run.as_ref()
                .filter(|run| run.season.id == id)
                .map_or(0, |run| usize::from(run.progress.cleared))
        })
    });
    view! {
        <div class="tower-stage card">
            <div class="tower-stage-copy"><p class="eyebrow">{starts}{" — "}{ends}{" UTC"}</p><h3>{title}</h3>
                <p class="lead">{words::tower_lead(lang)}</p>
                {if season.withdrawn { view! { <p class="notice">{words::ch_withdrawn(lang)}</p> }.into_any() }
                    else { view! { <a class="button" href=play(id)>{words::ch_play(lang)}</a> }.into_any() }}
            </div>
            <ol class="tower-floors" aria-label=words::tower_floors(lang)>{floor_names.into_iter().enumerate().map(|(index, name)| {
                view! { <li class:tower-cleared=move || { index < cleared.get() } aria-current=move || (index == cleared.get()).then_some("step")>
                    <span class="tower-floor-number">{move || if index < cleared.get() { "✓".to_string() } else { (index + 1).to_string() }}</span><span>{name}</span></li> }
            }).collect_view()}</ol>
        </div>
    }
}
