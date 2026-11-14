use super::{Hub, Lang, api, play_link, words};
use crate::islands::medals;
use leptos::prelude::*;

pub(super) fn view(lang: Lang, hub: Hub) -> AnyView {
    if !hub.signed_in.get() {
        return view! { <section class="card challenge-account"><h2>{words::ch_badges(lang)}</h2>
            <p>{words::ch_badges_lead(lang)}</p><a class="button" href=format!("/{}/signin",lang.code())>{words::nav_sign_in(lang)}</a>
        </section> }.into_any();
    }
    let active = hub.active.get();
    let selected = RwSignal::new(None);
    view! {
        <section class="challenge-achievements" aria-labelledby="challenge-badges-title">
            <h2 id="challenge-badges-title">{words::ch_badges(lang)}</h2><p>{words::ch_badges_lead(lang)}</p>
            <div class="challenge-badges">{hub.badges.get().iter().map(|badge| badge_card(lang,badge,selected)).collect_view()}</div>
            {medals::gallery(lang, selected)}
            {(!active.is_empty()).then(|| view! { <div class="card"><h3>{words::ch_active(lang)}</h3>
                {active.into_iter().map(|attempt| view! { <a class="button button-ghost" href=play_link(attempt.challenge)>{words::ch_play(lang)}</a> }).collect_view()}
            </div> })}
        </section>
    }.into_any()
}

fn badge_card(
    lang: Lang,
    badge: &api::Badge,
    selected: RwSignal<Option<medals::Preview>>,
) -> impl IntoView + use<> {
    let motif = match badge.kind {
        api::BadgeKind::Daily => 0,
        api::BadgeKind::Weekly => 1,
        api::BadgeKind::Tactician => 2,
        api::BadgeKind::FirstTry => 3,
        api::BadgeKind::Regular => 4,
    };
    let title = match badge.kind {
        api::BadgeKind::Daily => words::ch_badge_daily(lang),
        api::BadgeKind::Weekly => words::ch_badge_weekly(lang),
        api::BadgeKind::Tactician => words::ch_badge_tactician(lang),
        api::BadgeKind::FirstTry => words::ch_badge_first_try(lang),
        api::BadgeKind::Regular => words::ch_badge_regular(lang),
    };
    let tier = match badge.tier {
        None => words::ch_badge_locked(lang),
        Some(api::Tier::Bronze) => words::ch_bronze(lang),
        Some(api::Tier::Silver) => words::ch_silver(lang),
        Some(api::Tier::Gold) => words::ch_gold(lang),
        Some(api::Tier::Platinum) => words::ch_platinum(lang),
        Some(api::Tier::Obsidian) => words::ch_obsidian(lang),
    };
    let progress = badge.next.map_or_else(
        || badge.count.to_string(),
        |next| format!("{} / {next}", badge.count),
    );
    view! { <article class="card challenge-badge" class:challenge-earned=badge.tier.is_some()>
        {crate::islands::medals::view(motif, badge.tier)}<h3>{title}</h3><p>{tier}</p>
        <p class="num">{progress.clone()}</p>
        <progress max=badge.next.unwrap_or(badge.count.max(1)) value=badge.count aria-label=title aria-valuetext=progress></progress>
        {medals::inspect(lang, selected, medals::Preview { motif, count:badge.count, thresholds:badge.kind.thresholds(),title })}
    </article> }
}
