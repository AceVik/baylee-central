use super::{Hub, Lang, api, play_link, words};
use crate::islands::medals;
use leptos::prelude::*;

pub(super) fn view(lang: Lang, hub: Hub) -> impl IntoView {
    let selected = hub.selected_medal;
    view! {
        <Show when=move || hub.signed_in.get() fallback=move || view! {
            <section class="card challenge-account">
                <h2>{words::ch_badges(lang)}</h2>
                <p>{words::ch_badges_lead(lang)}</p>
                <a class="button button-ghost" href=format!("/{}/signin", lang.code())>{words::nav_sign_in(lang)}</a>
            </section>
        }>
            <section class="challenge-achievements" aria-labelledby="challenge-badges-title">
                <h2 id="challenge-badges-title">{words::ch_badges(lang)}</h2>
                <p>{words::ch_badges_lead(lang)}</p>
                <div class="challenge-badges">
                    {api::BadgeKind::ALL.into_iter().map(|kind| badge_card(lang, hub, kind, selected)).collect_view()}
                </div>
                {medals::gallery(lang, selected)}
                {move || {
                    let active = hub.active.get();
                    (!active.is_empty()).then(|| view! {
                        <div class="card">
                            <h3>{words::ch_active(lang)}</h3>
                            <div class="challenge-actions">
                                {active.into_iter().map(|attempt| view! {
                                    <a class="button button-ghost" href=play_link(attempt.challenge)>{words::ch_play(lang)}</a>
                                }).collect_view()}
                            </div>
                        </div>
                    })
                }}
            </section>
        </Show>
    }
}

pub(super) fn motif_of(kind: api::BadgeKind) -> u8 {
    match kind {
        api::BadgeKind::Daily => 0,
        api::BadgeKind::Weekly => 1,
        api::BadgeKind::Tactician => 2,
        api::BadgeKind::FirstTry => 3,
        api::BadgeKind::Regular => 4,
    }
}

fn badge_card(
    lang: Lang,
    hub: Hub,
    kind: api::BadgeKind,
    selected: RwSignal<Option<medals::Preview>>,
) -> impl IntoView {
    let motif = motif_of(kind);
    let title = match kind {
        api::BadgeKind::Daily => words::ch_badge_daily(lang),
        api::BadgeKind::Weekly => words::ch_badge_weekly(lang),
        api::BadgeKind::Tactician => words::ch_badge_tactician(lang),
        api::BadgeKind::FirstTry => words::ch_badge_first_try(lang),
        api::BadgeKind::Regular => words::ch_badge_regular(lang),
    };
    let thresholds = kind.thresholds();
    let badge = Memo::new(move |_| {
        hub.badges
            .with(|badges| badges.iter().find(|b| b.kind == kind).cloned())
            .unwrap_or_else(|| kind.progress(0))
    });
    let progress = Memo::new(move |_| {
        let b = badge.get();
        medals::progress_text(lang, u64::from(b.count), b.next.map(u64::from))
    });
    let preview = Signal::derive(move || medals::Preview {
        motif,
        count: badge.get().count,
        thresholds,
        title,
    });
    view! {
        <article class="card challenge-badge">
            {move || medals::view(motif, badge.get().tier)}
            <h3>{title}</h3>
            <p>{move || medals::tier(lang, badge.get().tier)}</p>
            <p class="num">{move || progress.get()}</p>
            <progress
                max=move || badge.get().next.unwrap_or_else(|| badge.get().count.max(1))
                value=move || badge.get().count
                aria-label=title
                aria-valuetext=move || progress.get()
            ></progress>
            {medals::inspect(lang, selected, preview)}
        </article>
    }
}
