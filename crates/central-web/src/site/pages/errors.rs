//! 404 and 410.

use leptos::prelude::*;

use super::Built;
use crate::site::shell::Meta;
use crate::site::{Context, Page};
use crate::words;

fn error(
    cx: &Context<'_>,
    title: &'static str,
    page_title: &'static str,
    lead: &'static str,
) -> Built {
    let lang = cx.lang;
    let meta = Meta::new(format!("{page_title} – Baylee"), lead, "img/og.jpg");
    let body = view! {
        <section class="page-head wrap narrow error-page">
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
        words::not_found_title(lang),
        words::title_not_found(lang),
        words::not_found_lead(lang),
    )
}

pub fn gone(cx: &Context<'_>) -> Built {
    let lang = cx.lang;
    error(
        cx,
        words::gone_title(lang),
        words::title_gone(lang),
        words::gone_lead(lang),
    )
}
