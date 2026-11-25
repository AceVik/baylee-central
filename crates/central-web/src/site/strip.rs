//! The live status strip directly beneath the site header: gateways online,
//! live player, game, and open-table counts when reported, registered accounts,
//! and total page views.

use leptos::prelude::*;

use crate::data;
use crate::format;
use crate::rich::icon;
use crate::site::{Context, Page, icons};
use crate::words;

/// One of the strip's figures: the term is a glyph with a short label, the
/// value stands apart so that the stylesheet can set it above the label or
/// between glyph and label. `title` says the figure in full under the pointer.
fn strip_cell(title: &str, glyph: &'static str, label: &'static str, value: AnyView) -> AnyView {
    view! {
        <div class="strip-cell" title=title.to_owned()>
            <dt>{icon(glyph)}<span class="strip-label">{label}</span></dt>
            <dd class="num" translate="no">{value}</dd>
        </div>
    }
    .into_any()
}

fn gateways_cell(lang: words::Lang, list: Option<&[data::Gateway]>) -> AnyView {
    let number = |n: u64| words::number(lang, n);
    let gateways = words::strip_gateways(lang);
    match list {
        Some(all) => {
            let online = all.iter().filter(|g| g.online).count();
            let count = |n: usize| number(u64::try_from(n).unwrap_or(u64::MAX));
            let title = format!(
                "{gateways}: {} {}, {} {}",
                count(online),
                words::strip_online(lang),
                count(all.len() - online),
                words::strip_offline(lang),
            );
            strip_cell(
                &title,
                icons::SERVER,
                gateways,
                view! {
                    <a href=Page::Gateways.path(lang)>
                        <span>{count(online)}</span>
                        <span class="strip-of">{format!("/ {}", count(all.len()))}</span>
                    </a>
                }
                .into_any(),
            )
        }
        None => strip_cell(
            gateways,
            icons::SERVER,
            gateways,
            view! {
                <a class="strip-way" href=Page::Gateways.path(lang)>
                    {words::strip_directory(lang)}
                </a>
            }
            .into_any(),
        ),
    }
}

/// Renders the global telemetry strip for public and signed-in pages.
pub(crate) fn strip(cx: &Context<'_>) -> impl IntoView + use<> {
    let lang = cx.lang;
    let number = |n: u64| words::number(lang, n);
    let activity = cx.data.gateways.as_deref().and_then(data::activity_of);
    let mut cells: Vec<AnyView> = Vec::with_capacity(7);

    cells.push(gateways_cell(lang, cx.data.gateways.as_deref()));

    let live = words::strip_live(lang);
    let dot = |active: bool| {
        active.then(|| view! { <span class="dot ok live" aria-hidden="true"></span> })
    };
    if let Some((players, playing, tables, games)) = activity {
        cells.push(strip_cell(
            live,
            icons::PLAY,
            words::strip_l_playing(lang),
            view! { {dot(playing > 0)}{number(playing)} }.into_any(),
        ));
        cells.push(strip_cell(
            live,
            icons::USERS,
            words::strip_l_online(lang),
            view! { {dot(players > 0)}{number(players)} }.into_any(),
        ));
        cells.push(strip_cell(
            live,
            icons::SWORDS,
            words::strip_l_games(lang),
            view! { {dot(games > 0)}{number(games)} }.into_any(),
        ));
        cells.push(strip_cell(
            live,
            icons::LAYOUT_DASHBOARD,
            words::strip_l_tables(lang),
            number(tables).into_any(),
        ));
    }

    if let Some(accounts) = cx.data.accounts {
        cells.push(strip_cell(
            words::strip_accounts(lang),
            icons::USER_CHECK,
            words::strip_l_accounts(lang),
            number(accounts).into_any(),
        ));
    }

    if let Some(views) = cx.data.views.as_ref() {
        cells.push(strip_cell(
            &words::strip_views(lang, &format::date(lang, &views.since)),
            icons::EYE,
            words::strip_l_views(lang),
            number(views.count).into_any(),
        ));
    }

    let count = cells.len().to_string();
    view! {
        <section class="strip" aria-label=words::status_label(lang)>
            <div class="strip-inner">
                <dl class="strip-list" data-count=count>{cells}</dl>
            </div>
        </section>
    }
}
