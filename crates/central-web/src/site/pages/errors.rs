//! 404, 410, and the page the service worker shows offline.

use leptos::prelude::*;

use super::Built;
use crate::site::shell::Meta;
use crate::site::{Context, Page};
use crate::words;

fn error(
    cx: &Context<'_>,
    code: &'static str,
    title: &'static str,
    page_title: &'static str,
    lead: &'static str,
) -> Built {
    let lang = cx.lang;
    let meta = Meta::new(format!("{page_title} – Baylee"), lead, "img/og.jpg");
    let body = view! {
        <section class="page-head wrap narrow error-page">
            <p class="eyebrow">{code}</p>
            <h1>{title}</h1>
            <p class="lead">{lead}</p>
            <p class="actions">
                <a class="button primary" href=Page::Home.path(lang)>{words::error_home(lang)}</a>
            </p>
        </section>
    }
    .into_any();
    (meta, body)
}

pub fn not_found(cx: &Context<'_>) -> Built {
    let lang = cx.lang;
    error(
        cx,
        "404",
        words::not_found_title(lang),
        words::title_not_found(lang),
        words::not_found_lead(lang),
    )
}

pub fn gone(cx: &Context<'_>) -> Built {
    let lang = cx.lang;
    error(
        cx,
        "410",
        words::gone_title(lang),
        words::title_gone(lang),
        words::gone_lead(lang),
    )
}

/// Without a network: the service worker answers a page it cannot fetch
/// with this one, at the address that was asked for, so "Try again" asks
/// for that address again. No script needed.
pub fn offline(cx: &Context<'_>) -> Built {
    let lang = cx.lang;
    let meta = Meta::new(
        format!("{} – Baylee", words::title_offline(lang)),
        words::offline_lead(lang),
        "img/og.jpg",
    );
    let body = view! {
        <section class="page-head wrap narrow error-page">
            <h1>{words::offline_title(lang)}</h1>
            <p class="lead">{words::offline_lead(lang)}</p>
            <p class="actions">
                <a class="button primary" href="">{words::offline_again(lang)}</a>
            </p>
        </section>
    }
    .into_any();
    (meta, body)
}
