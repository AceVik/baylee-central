//! Original vector counterparts of the client's eight procedural insignia.
use crate::words::{self, Lang};
use central_api::challenges::Tier;
use leptos::prelude::*;

pub(super) fn tier(lang: Lang, tier: Option<Tier>) -> &'static str {
    match tier {
        None => words::ch_badge_locked(lang),
        Some(Tier::Bronze) => words::ch_bronze(lang),
        Some(Tier::Silver) => words::ch_silver(lang),
        Some(Tier::Gold) => words::ch_gold(lang),
        Some(Tier::Platinum) => words::ch_platinum(lang),
        Some(Tier::Obsidian) => words::ch_obsidian(lang),
    }
}

#[derive(Clone, Copy)]
pub(super) struct Preview {
    pub motif: u8,
    pub count: u32,
    pub thresholds: [u32; 5],
    pub title: &'static str,
}

pub(super) fn inspect(
    lang: Lang,
    selected: RwSignal<Option<Preview>>,
    badge: Preview,
) -> impl IntoView {
    view! { <button id=format!("medal-preview-{}",badge.motif) class="button button-ghost" type="button"
        aria-label=format!("{} · {}", badge.title, words::tower_tiers(lang))
        aria-pressed=move || selected.with(|value| if value.is_some_and(|value| value.motif == badge.motif) { "true" } else { "false" })
        on:click=move |_| selected.update(|value| *value = if value.is_some_and(|value| value.motif == badge.motif) { None } else { Some(badge) })>
        {words::tower_tiers(lang)}
    </button> }
}

pub(super) fn gallery(lang: Lang, selected: RwSignal<Option<Preview>>) -> impl IntoView {
    move || {
        selected.get().map(|badge| {
        let Preview { motif, count, thresholds, title } = badge;
        view! { <section class="card medal-preview" aria-label=format!("{} · {}",title,words::tower_tiers(lang))>
        <header class="tower-header"><h4>{title}</h4><button type="button" class="button button-ghost"
            on:click=move |_| { selected.set(None); super::focus::after_render(format!("#medal-preview-{motif}")); }>{words::dialog_close(lang)}</button></header>
        <div class="tower-tier-gallery">{
        [Tier::Bronze,Tier::Silver,Tier::Gold,Tier::Platinum,Tier::Obsidian]
            .into_iter().zip(thresholds).map(|(material,target)| view! {
                <div class:relic-earned={count >= target}>{view(motif,Some(material))}
                    <p>{tier(lang,Some(material))}</p><p class="num">{target}</p>
                    {(count >= target).then(|| view! { <span aria-label=words::ch_completed(lang)>{"✓"}</span> })}
                </div>
            }).collect_view()
    }</div></section> }
    })
    }
}

pub(super) fn view(motif: u8, tier: Option<Tier>) -> impl IntoView {
    let tier = match tier {
        None => 0,
        Some(Tier::Bronze) => 1,
        Some(Tier::Silver) => 2,
        Some(Tier::Gold) => 3,
        Some(Tier::Platinum) => 4,
        Some(Tier::Obsidian) => 5,
    };
    let path = match motif {
        0 => {
            "M0-12A12 12 0 1 0 0 12A12 12 0 1 0 0-12 M0-25V-19M0 19V25M-25 0H-19M19 0H25M-18-18L-14-14M14 14L18 18M18-18L14-14M-14 14L-18 18"
        }
        1 => {
            "M8-20A21 21 0 1 0 20 12A19 19 0 0 1 8-20 M18-22L21-14L28-11L21-8L18 0L15-8L8-11L15-14Z"
        }
        2 => "M0-27L7-7L24 0L7 7L0 27L-7 7L-24 0L-7-7Z M0-16L13 0L0 16L-13 0Z",
        3 => "M0-17A17 17 0 1 0 0 17A17 17 0 1 0 0-17 M-23 25L19-21M7-20L19-21L19-9",
        4 => {
            "M0 24V-8M0 16L-14 25M0 16L14 25 M-10 6C-30 9-26-15-12-13C-13-31 15-31 13-13C28-14 29 9 10 6Z"
        }
        5 => "M-26 22L0-23L26 22M6 24L-7 9L5-3M18-27L21-20L27-17L21-14L18-7L15-14L9-17L15-20Z",
        6 => "M-12 24V-10H12V24M-22-11V-25L-11-18L0-29L11-18L22-25V-11ZM-4 24V10A4 4 0 0 1 4 10V24",
        _ => "M-23-23H23V0Q21 15 0 28Q-21 15-23 0ZM0-15L12 2L0 19L-12 2Z",
    };
    view! {
        <span class=format!("retention-medal tier-{tier}") data-motif=motif aria-hidden="true">
            <svg viewBox="0 0 128 128" focusable="false">
                <g transform="translate(64 64)">
                    <circle class={"medal-disc"} r="38"/><circle class={"medal-bezel"} r="35"/>
                    <circle class={"medal-enamel"} r="31"/>
                    <path class="medal-emblem" d=path/>
                    {(tier >= 2).then(|| view! { <circle class={"medal-detail"} r="42"/> })}
                    {(tier >= 3).then(|| view! { <g class="medal-wreath">
                        {(0..7).flat_map(|step| [-1, 1].map(move |side| view! {
                            <path d="M42-5Q57-12 48-21Q36-15 42-5Z" transform=format!("scale({side} 1) rotate({})", step * 19 - 50)/>
                        })).collect_view()}
                    </g> })}
                    {(tier >= 4).then(|| view! { <g class="medal-gems">
                        {(0..5).map(|step| view! { <path d="M0-49L4-42L0-35L-4-42Z" transform=format!("rotate({})", step * 72)/> }).collect_view()}
                    </g> })}
                    {(tier >= 5).then(|| view! { <path class={"medal-crown"} d="M-18-49L-21-62L-10-56L0-67L10-56L21-62L18-49Z" transform="translate(0 7)"/> })}
                </g>
            </svg>
        </span>
    }
}
