//! Global public expedition, with private progression loaded only in the browser.
use super::{
    auth::problem_text,
    central::{Failure, call},
    session,
};
use crate::words::{self, Lang};
use central_api::{challenges, tower as api, wire};
use leptos::prelude::*;
mod progress;
mod ranking;
mod state;
use state::Hub;

pub(super) async fn request(request: api::Request) -> Result<api::Response, Failure> {
    match call(&wire::Request::Challenges {
        request: challenges::Request::Tower { request },
    })
    .await?
    {
        wire::Response::Challenges {
            response: challenges::Response::Tower { response },
        } => Ok(*response),
        _ => Err(Failure::Malformed),
    }
}

fn text(lang: Lang, text: &challenges::Words) -> String {
    if lang == Lang::De {
        text.de.clone()
    } else {
        text.en.clone()
    }
}

fn play(id: [u8; 16]) -> String {
    format!("baylee://tower/{}", crate::data::id_hex(&id))
}

#[island]
pub fn Tower(lang: Lang, initial: Option<api::Season>) -> impl IntoView {
    let hub = Hub::new(initial);
    Effect::new(move |_| {
        hub.busy.set(false);
        hub.refresh();
        let timer = set_interval(move || hub.refresh(), std::time::Duration::from_secs(60)).ok();
        #[cfg(target_arch = "wasm32")]
        let sessions = window_event_listener_untyped("storage", move |event| {
            let key =
                js_sys::Reflect::get(event.as_ref(), &wasm_bindgen::JsValue::from_str("key")).ok();
            if key.is_some_and(|key| {
                key.is_null() || key.as_string().as_deref() == Some("baylee-central-session")
            }) {
                hub.session_changed();
            }
        });
        on_cleanup(move || {
            #[cfg(target_arch = "wasm32")]
            sessions.remove();
            if let Some(timer) = timer {
                timer.clear();
            }
        });
    });
    view! {
        <section class="tower-hub" aria-labelledby="tower-title">
            <header class="tower-header">
                <div><p class="eyebrow">{words::ch_weekly(lang)}</p><h2 id="tower-title">{words::tower_title(lang)}</h2></div>
                <button type="button" class="button button-ghost" disabled=move || hub.busy.get()
                    on:click=move |_| hub.refresh()>{words::fb_refresh(lang)}</button>
            </header>
            <p class="error" role="alert">{move || hub.error.get().map(|error| problem_text(lang, error.into()))}</p>
            <span class="meta" role="status">{move || hub.busy.get().then(|| words::ch_loading(lang))}</span>
            {move || match hub.season.get() {
                None => view! { <p class="empty-state">{words::ch_empty(lang)}</p> }.into_any(),
                Some(season) => expedition(lang, hub, season).into_any(),
            }}
            <details class="tower-guide"><summary>{words::tower_guide(lang)}</summary><p>{words::tower_rules(lang)}</p></details>
            {move || progress::view(lang, hub)}
            {ranking::view(lang, hub)}
        </section>
    }
}

fn expedition(lang: Lang, hub: Hub, season: api::Season) -> impl IntoView {
    let id = season.id;
    let title = text(lang, &season.theme.title);
    let floor_names = season.theme.floors.map(|floor| text(lang, &floor));
    let starts = super::account::day(lang, season.theme.period.starts_at);
    let ends = super::account::day(lang, season.theme.period.ends_at);
    view! {
        <div class="tower-stage card">
            <div class="tower-stage-copy"><p class="eyebrow">{starts}{" — "}{ends}{" UTC"}</p><h3>{title}</h3>
                <p class="lead">{words::tower_lead(lang)}</p>
                {if season.withdrawn { view! { <p class="notice">{words::ch_withdrawn(lang)}</p> }.into_any() }
                    else { view! { <a class="button" href=play(id)>{words::ch_play(lang)}</a> }.into_any() }}
            </div>
            <ol class="tower-floors" aria-label=words::tower_floors(lang)>{floor_names.into_iter().enumerate().map(|(index, name)| {
                let cleared = hub.run.with(|run| run.as_ref().filter(|run| run.season.id == id).map_or(0, |run| usize::from(run.progress.cleared)));
                view! { <li class:tower-cleared={index < cleared} aria-current=(index == cleared).then_some("step")>
                    <span class="tower-floor-number">{if index < cleared { "✓".to_string() } else { (index + 1).to_string() }}</span><span>{name}</span></li> }
            }).collect_view()}</ol>
        </div>
    }
}
