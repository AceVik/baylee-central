//! The global challenge calendar, with public task cards in the first HTML response.
use super::Built;
use crate::{
    islands::Challenges,
    site::{Context, Page, shell::Meta},
    words,
};
use leptos::prelude::*;

pub fn page(cx: &Context<'_>) -> Built {
    let lang = cx.lang;
    let mut meta = Meta::new(words::ch_title(lang), words::ch_lead(lang), "img/og.jpg");
    meta.islands = true;
    meta.trail
        .push((words::ch_title(lang).to_owned(), Page::Challenges));
    let body = view! {
        <section class="page-head wrap" aria-labelledby="challenges-title">
            <p class="eyebrow">{words::ch_eyebrow(lang)}</p>
            <h1 id="challenges-title">{words::ch_title(lang)}</h1>
            <p class="lead">{words::ch_lead(lang)}</p>
        </section>
        <div class="wrap challenge-hub">
            <Challenges lang=lang initial=cx.data.challenges.clone() />
        </div>
    }
    .into_any();
    (meta, body)
}
