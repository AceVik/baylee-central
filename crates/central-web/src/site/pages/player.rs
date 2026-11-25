//! Public player profile (`/{lang}/player/{handle}`): page wrapper around the
//! interactive `PlayerProfile` island.

use leptos::prelude::*;

use super::Built;
use crate::islands::PlayerProfile;
use crate::site::Context;
use crate::site::shell::Meta;
use crate::words;

/// Renders the public player profile page.
pub fn page(cx: &Context<'_>) -> Built {
    let lang = cx.lang;
    let mut meta = Meta::new(
        words::player_page_title(lang),
        words::player_page_lead(lang),
        "img/og.jpg",
    );
    meta.islands = true;
    let body = view! {
        <section class="page-head wrap" aria-labelledby="player-page-title">
            <h1 id="player-page-title">{words::player_page_title(lang)}</h1>
            <p class="lead">{words::player_page_lead(lang)}</p>
        </section>
        <div class="wrap">
            <PlayerProfile lang=lang />
        </div>
    }
    .into_any();
    (meta, body)
}
