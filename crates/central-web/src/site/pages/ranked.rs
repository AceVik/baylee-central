//! The leaderboards of ranked play (`/{lang}/ranked`): the page's head and
//! notes around the `RankedLadder` island, which asks Central for the
//! boards (docs/website-design.md §"The leaderboards").

use leptos::prelude::*;
use serde_json::json;

use super::Built;
use crate::islands::RankedLadder;
use crate::rich::rich;
use crate::site::shell::Meta;
use crate::site::{Context, Page};
use crate::words;

/// Renders the leaderboards' page.
pub fn page(cx: &Context<'_>) -> Built {
    let lang = cx.lang;
    let mut meta = Meta::new(
        words::title_ranked(lang),
        words::desc_ranked(lang),
        "img/og.jpg",
    );
    meta.islands = true;
    meta.trail
        .push((words::ranked_title(lang).to_owned(), Page::Ranked));
    meta.data.push(json!({
        "@context": "https://schema.org",
        "@type": "CollectionPage",
        "name": words::title_ranked(lang),
        "description": words::desc_ranked(lang),
        "url": format!("{}{}", cx.origin, Page::Ranked.path(lang)),
        "inLanguage": lang.tag(),
        "isPartOf": {
            "@type": "WebSite",
            "name": "Baylee",
            "url": format!("{}{}", cx.origin, Page::Home.path(lang)),
        },
    }));
    let download = Page::Download.path(lang);
    let help = rich(words::ranked_help(lang), move |tag| {
        (tag == "download").then(|| download.clone())
    });
    let body = view! {
        <section class="page-head wrap" aria-labelledby="ranked-title">
            <p class="eyebrow">{words::ranked_eyebrow(lang)}</p>
            <h1 id="ranked-title">{words::ranked_title(lang)}</h1>
            <p class="lead">{words::ranked_lead(lang)}</p>
        </section>
        <div class="wrap ladder-wrap">
            <noscript><p class="notice">{words::ranked_script(lang)}</p></noscript>
            <RankedLadder lang=lang />
            <div class="ladder-notes">
                <p class="small">{help}</p>
                <p class="small">{words::ranked_ai_note(lang)}</p>
            </div>
        </div>
    }
    .into_any();
    (meta, body)
}
