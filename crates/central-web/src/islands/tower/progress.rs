use super::{Hub, Lang, api, play, text, words};
use crate::islands::medals;
use leptos::prelude::*;

pub(super) fn view(lang: Lang, hub: Hub) -> AnyView {
    if !hub.signed_in.get() {
        return view! { <div class="card tower-account"><h3>{words::tower_rewards(lang)}</h3>
            <p>{words::ch_badges_lead(lang)}</p><a class="button button-ghost" href=format!("/{}/signin",lang.code())>{words::nav_sign_in(lang)}</a>
        </div> }.into_any();
    }
    let run = hub.active.get().or_else(|| hub.run.get());
    let selected = RwSignal::new(None);
    view! {
        {run.map(|run| owned(lang, hub, run))}
        <section class="tower-rewards" aria-labelledby="tower-rewards-title">
            <h3 id="tower-rewards-title">{words::tower_rewards(lang)}</h3><p>{words::tower_reward_rules(lang)}</p>
            <div class="tower-relics">{api::BadgeKind::ALL.into_iter().map(|kind| {
                let badge = hub.badges.with(|badges| badges.iter().find(|badge| badge.kind == kind).cloned()).unwrap_or_else(|| kind.progress(0));
                relic(lang, &badge, selected)
            }).collect_view()}</div>
            {medals::gallery(lang, selected)}
        </section>
    }.into_any()
}

fn owned(lang: Lang, hub: Hub, run: api::Run) -> impl IntoView {
    let progress = &run.progress;
    let id = run.season.id;
    let status = match progress.phase {
        api::Phase::Conquered => words::tower_conquered(lang),
        api::Phase::Choosing => words::tower_choosing(lang),
        api::Phase::Abandoned | api::Phase::Exhausted => words::tower_ended(lang),
        _ => words::ch_active(lang),
    };
    let offers: Vec<_> = if progress.phase == api::Phase::Choosing {
        let counts = progress.counts();
        api::Track::ALL
            .into_iter()
            .filter_map(|track| {
                run.season.theme.upgrades[track.index()]
                    .get(usize::from(counts[track.index()]))
                    .cloned()
            })
            .collect()
    } else {
        Vec::new()
    };
    let listed = run.listed;
    let checked = RwSignal::new(listed);
    view! {
        <article class="card tower-owned">
            <p class="eyebrow">{status}</p><h3>{text(lang,&run.season.theme.title)}</h3>
            <dl class="challenge-stats">
                <div><dt>{words::tower_floors(lang)}</dt><dd>{progress.cleared}{" / 5"}</dd></div>
                <div><dt>{words::tower_lives(lang)}</dt><dd>{progress.lives}{" / 3"}</dd></div>
                <div><dt>{words::ch_turn(lang)}</dt><dd>{progress.turns}</dd></div>
            </dl>
            <div class="challenge-actions"><a class="button" href=play(id)>{words::ch_play(lang)}</a></div>
            <label class="challenge-consent" for="tower-consent"><input id="tower-consent" type="checkbox" prop:checked=move || checked.get() disabled=move || hub.busy.get()
                on:change=move |event| { let listed = event_target_checked(&event); checked.set(listed); hub.visibility(&run, listed, checked); }/><span>{words::ch_consent(lang)}</span></label>
            {(!offers.is_empty()).then(|| view! { <div class="tower-upgrades">{offers.into_iter().map(|offer| view! {
                <article><h4>{text(lang,&offer.title)}</h4><p>{text(lang,&offer.summary)}</p>
                    <p class="tower-replacement"><span>{offer.copies}{" × "}{offer.remove}</span><span aria-hidden="true">{"↓"}</span><strong>{offer.copies}{" × "}{offer.add}</strong></p>
                </article>
            }).collect_view()}</div> })}
        </article>
    }
}

fn relic(
    lang: Lang,
    badge: &api::Badge,
    selected: RwSignal<Option<medals::Preview>>,
) -> impl IntoView + use<> {
    let (motif, title) = match badge.kind {
        api::BadgeKind::Pathfinder => (5, words::tower_pathfinder(lang)),
        api::BadgeKind::Summit => (6, words::tower_summit(lang)),
        api::BadgeKind::Unbroken => (7, words::tower_unbroken(lang)),
    };
    let next = badge.next.unwrap_or(badge.count.max(1));
    view! {
        <article class="card challenge-badge">
            {medals::view(motif,badge.tier)}<h4>{title}</h4><p>{medals::tier(lang,badge.tier)}</p>
            <p class="num">{badge.count}{" / "}{next}</p><progress value=badge.count max=next aria-label=title></progress>
            {medals::inspect(lang, selected, medals::Preview { motif, count: badge.count, thresholds: badge.kind.thresholds(), title })}
        </article>
    }
}
