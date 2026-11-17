//! Private permanent Tower accomplishments, independent of the live gameplay service.
use super::{
    auth::problem_text,
    central::{Failure, call},
    medals, session,
};
use crate::words::{self, Lang};
use central_api::{tower_awards as api, wire};
use leptos::prelude::*;
mod state;
use state::Hub;

#[island]
pub fn Echoes(lang: Lang) -> impl IntoView {
    let hub = Hub::new();
    let selected = hub.selected;
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
        <section class="echoes-accomplishments" aria-labelledby="echoes-title">
            <header class="tower-header">
                <div><p class="eyebrow">{words::echoes_permanent(lang)}</p><h2 id="echoes-title">{words::echoes_title(lang)}</h2></div>
                <div class="tower-header-actions">
                    <span class="meta" role="status">{move || hub.loading.get().then(|| words::ch_loading(lang))}</span>
                    <button type="button" class="button button-ghost" disabled=move || hub.busy.get() on:click=move |_| hub.refresh(true)>{words::fb_refresh(lang)}</button>
                </div>
            </header>
            <p class="lead">{words::echoes_lead(lang)}</p>
            <p class="error" role="alert">{move || hub.error.get().map(|error| problem_text(lang, error.into()))}</p>
            <Show
                when=move || hub.progress.with(Option::is_some)
                fallback=move || {
                    if hub.signed_in.get() {
                        view! {
                            <div class="card tower-account">
                                <p role="status">{words::echoes_pending(lang)}</p>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div class="card tower-account">
                                <h3>{words::ch_progress(lang)}</h3>
                                <p>{words::echoes_confirmed(lang)}</p>
                                <a class="button button-ghost" href=format!("/{}/signin", lang.code())>{words::nav_sign_in(lang)}</a>
                            </div>
                        }.into_any()
                    }
                }
            >
                <div class="tower-relics">
                    {[api::BadgeKind::Victories, api::BadgeKind::Ascents, api::BadgeKind::Unbroken]
                        .into_iter()
                        .map(|kind| relic(lang, hub, kind, selected))
                        .collect_view()}
                </div>
                <p class="meta">{words::echoes_confirmed(lang)}</p>
                {medals::gallery(lang, selected)}
            </Show>
        </section>
    }
}

pub(super) fn motif_of(kind: api::BadgeKind) -> u8 {
    match kind {
        api::BadgeKind::Victories => 8,
        api::BadgeKind::Ascents => 9,
        api::BadgeKind::Unbroken => 10,
    }
}

fn relic(
    lang: Lang,
    hub: Hub,
    kind: api::BadgeKind,
    selected: RwSignal<Option<medals::Preview>>,
) -> impl IntoView {
    let motif = motif_of(kind);
    let (title, goal) = match kind {
        api::BadgeKind::Victories => (
            words::echoes_victories(lang),
            words::echoes_victories_goal(lang),
        ),
        api::BadgeKind::Ascents => (
            words::echoes_ascents(lang),
            words::echoes_ascents_goal(lang),
        ),
        api::BadgeKind::Unbroken => (
            words::echoes_unbroken(lang),
            words::echoes_unbroken_goal(lang),
        ),
    };
    let thresholds = kind
        .thresholds()
        .map(|value| u32::try_from(value).unwrap_or(u32::MAX));
    let badge = Memo::new(move |_| {
        hub.progress
            .get()
            .and_then(|progress| progress.badges().into_iter().find(|b| b.kind == kind))
            .unwrap_or_else(|| kind.progress(0))
    });
    let progress_label = Memo::new(move |_| {
        let b = badge.get();
        medals::progress_text(lang, b.count, b.next)
    });
    let preview = Signal::derive(move || medals::Preview {
        motif,
        count: u32::try_from(badge.get().count).unwrap_or(u32::MAX),
        thresholds,
        title,
    });
    view! {
        <article class="card challenge-badge" data-echoes-badge=motif>
            {move || medals::view(motif, badge.get().tier)}
            <h3>{title}</h3>
            <p class="echoes-badge-goal">{goal}</p>
            <p>{move || medals::tier(lang, badge.get().tier)}</p>
            <p class="num">{move || progress_label.get()}</p>
            <progress
                value=move || badge.get().count
                max=move || badge.get().next.unwrap_or_else(|| badge.get().count.max(1))
                aria-label=title
                aria-valuetext=move || progress_label.get()
            ></progress>
            {medals::inspect(lang, selected, preview)}
        </article>
    }
}
