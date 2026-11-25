//! Public friend invitation (`/{lang}/invite/{code}`): SSR wrapper around the
//! interactive `InviteCard` island.

use leptos::prelude::*;

use super::Built;
use crate::islands::InviteCard;
use crate::site::Context;
use crate::site::shell::Meta;
use crate::words;

/// Renders the public friend invite landing page.
pub fn page(cx: &Context<'_>) -> Built {
    let lang = cx.lang;
    let mut meta = Meta::new(
        words::invite_page_title(lang),
        words::invite_page_desc(lang),
        "img/og.jpg",
    );
    meta.islands = true;
    let body = view! {
        <section class="page-head wrap" aria-labelledby="invite-page-title">
            <h1 id="invite-page-title">{words::invite_page_title(lang)}</h1>
            <p class="lead">{words::invite_page_desc(lang)}</p>
        </section>
        <div class="wrap">
            <InviteCard lang=lang />
        </div>
    }
    .into_any();
    (meta, body)
}
