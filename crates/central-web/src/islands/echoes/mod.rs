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
        hub.refresh();
        let timer = set_interval(move || hub.refresh(), std::time::Duration::from_secs(60)).ok();
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
            <header class="tower-header"><div><p class="eyebrow">{words::echoes_permanent(lang)}</p><h2 id="echoes-title">{words::echoes_title(lang)}</h2></div>
                <button type="button" class="button button-ghost" disabled=move || hub.busy.get() on:click=move |_| hub.refresh()>{words::fb_refresh(lang)}</button>
            </header>
            <p class="lead">{words::echoes_lead(lang)}</p>
            <p class="error" role="alert">{move || hub.error.get().map(|error|problem_text(lang,error.into()))}</p>
            <span class="meta" role="status">{move || hub.busy.get().then(||words::ch_loading(lang))}</span>
            {move || match hub.progress.get() {
                Some(progress) => view! { <div class="tower-relics">{progress.badges().into_iter().map(|badge|relic(lang,badge,selected)).collect_view()}</div>
                    <p class="meta">{words::echoes_confirmed(lang)}</p>
                    {medals::gallery(lang,selected)}
                }.into_any(),
                None if !hub.signed_in.get() => view! { <div class="card tower-account"><p>{words::echoes_confirmed(lang)}</p><a class="button button-ghost" href=format!("/{}/signin",lang.code())>{words::nav_sign_in(lang)}</a></div> }.into_any(),
                None => view! { <p class="empty-state">{words::echoes_pending(lang)}</p> }.into_any(),
            }}
        </section>
    }
}

fn relic(
    lang: Lang,
    badge: api::Badge,
    selected: RwSignal<Option<medals::Preview>>,
) -> impl IntoView {
    let (motif, title, goal) = match badge.kind {
        api::BadgeKind::Victories => (
            8,
            words::echoes_victories(lang),
            words::echoes_victories_goal(lang),
        ),
        api::BadgeKind::Ascents => (
            9,
            words::echoes_ascents(lang),
            words::echoes_ascents_goal(lang),
        ),
        api::BadgeKind::Unbroken => (
            10,
            words::echoes_unbroken(lang),
            words::echoes_unbroken_goal(lang),
        ),
    };
    let count = badge.count;
    let next = badge.next.unwrap_or(count.max(1));
    view! { <article class="card challenge-badge" data-echoes-badge=motif>
        {medals::view(motif,badge.tier)}<h3>{title}</h3><p class="echoes-badge-goal">{goal}</p><p>{medals::tier(lang,badge.tier)}</p>
        <p class="num">{words::number(lang,count)}{badge.next.map(|next|format!(" / {}",words::number(lang,next)))}</p>
        <progress value=count max=next aria-label=title></progress>
        {medals::inspect(lang,selected,medals::Preview {motif,count:u32::try_from(count).unwrap_or(u32::MAX),thresholds:badge.kind.thresholds().map(|value|u32::try_from(value).unwrap_or(u32::MAX)),title})}
    </article> }
}
