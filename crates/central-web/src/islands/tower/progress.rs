use super::{Hub, Lang, api, play, words, words_text};
use crate::islands::medals;
use leptos::prelude::*;

pub(super) fn view(lang: Lang, hub: Hub) -> impl IntoView {
    let selected = hub.selected;
    view! {
        <Show
            when=move || hub.signed_in.get()
            fallback=move || view! {
                <div class="card tower-account">
                    <h3>{words::tower_rewards(lang)}</h3>
                    <p>{words::tower_reward_rules(lang)}</p>
                    <a class="button button-ghost" href=format!("/{}/signin", lang.code())>{words::nav_sign_in(lang)}</a>
                </div>
            }
        >
            {move || hub.active.get().or_else(|| hub.run.get()).map(|run| owned(lang, hub, &run))}
            <section class="tower-rewards" aria-labelledby="tower-rewards-title">
                <h3 id="tower-rewards-title">{words::tower_rewards(lang)}</h3>
                <p>{words::tower_reward_rules(lang)}</p>
                <div class="tower-relics">
                    {api::BadgeKind::ALL.into_iter().map(|kind| relic(lang, hub, kind, selected)).collect_view()}
                </div>
                {medals::gallery(lang, selected)}
            </section>
        </Show>
    }
}

pub(super) fn motif_of(kind: api::BadgeKind) -> u8 {
    match kind {
        api::BadgeKind::Pathfinder => 5,
        api::BadgeKind::Summit => 6,
        api::BadgeKind::Unbroken => 7,
    }
}

fn owned(lang: Lang, hub: Hub, run: &api::Run) -> impl IntoView + use<> {
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
            <p class="eyebrow">{status}</p><h3>{words_text(lang, &run.season.theme.title)}</h3>
            <dl class="challenge-stats">
                <div><dt>{words::tower_floors(lang)}</dt><dd>{progress.cleared}{" / 5"}</dd></div>
                <div><dt>{words::tower_lives(lang)}</dt><dd>{progress.lives}{" / 3"}</dd></div>
                <div><dt>{words::ch_turn(lang)}</dt><dd>{words::number(lang, u64::from(progress.turns))}</dd></div>
            </dl>
            <div class="challenge-actions"><a class="button" href=play(id)>{words::ch_play(lang)}</a></div>
            <label class="challenge-consent" for="tower-consent"><input id="tower-consent" type="checkbox" prop:checked=move || checked.get() disabled=move || hub.busy.get()
                on:change=move |event| { let listed = event_target_checked(&event); checked.set(listed); hub.visibility(id, listed, checked); }/><span>{words::ch_consent(lang)}</span></label>
            {(!offers.is_empty()).then(|| view! { <div class="tower-upgrades">{offers.into_iter().map(|offer| view! {
                <article><h4>{words_text(lang, &offer.title)}</h4><p>{words_text(lang, &offer.summary)}</p>
                    <p class="tower-replacement"><span>{offer.copies}{" × "}{offer.remove}</span><span aria-hidden="true">{"↓"}</span><strong>{offer.copies}{" × "}{offer.add}</strong></p>
                </article>
            }).collect_view()}</div> })}
        </article>
    }
}

fn relic(
    lang: Lang,
    hub: Hub,
    kind: api::BadgeKind,
    selected: RwSignal<Option<medals::Preview>>,
) -> impl IntoView {
    let motif = motif_of(kind);
    let title = match kind {
        api::BadgeKind::Pathfinder => words::tower_pathfinder(lang),
        api::BadgeKind::Summit => words::tower_summit(lang),
        api::BadgeKind::Unbroken => words::tower_unbroken(lang),
    };
    let thresholds = kind.thresholds();
    let badge = Memo::new(move |_| {
        hub.badges
            .with(|badges| badges.iter().find(|b| b.kind == kind).cloned())
            .unwrap_or_else(|| kind.progress(0))
    });
    let progress_label = Memo::new(move |_| {
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
            <p class="tower-badge-tier">{move || medals::tier(lang, badge.get().tier)}</p>
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
