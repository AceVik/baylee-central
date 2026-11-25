//! Inline handle display (`Name#tag`) with one-click copy icon and
//! optional link to the player's clean public profile URL.

use leptos::prelude::*;

use crate::words::{self, Lang};

use super::super::chat::glyph::{self, glyph};
use super::super::clip;

/// Formats a public player profile path (`/{lang}/player/{Name}-{tag}`).
#[must_use]
pub fn player_path(lang: Lang, name: &str, tag_hex: &str) -> String {
    format!("/{}/player/{name}-{tag_hex}", lang.code())
}

/// Reusable inline `Name#tag` view with a suffix copy-to-clipboard icon
/// button. Never use inside another interactive `<button>`.
pub fn handle_with_copy(
    lang: Lang,
    name: String,
    tag_hex: String,
    link_to_profile: bool,
) -> AnyView {
    let full = format!("{name}#{tag_hex}");
    let full_copy = full.clone();
    let copied = RwSignal::new(false);
    let on_copy = move |ev: leptos::ev::MouseEvent| {
        ev.stop_propagation();
        let text = full_copy.clone();
        leptos::task::spawn_local(async move {
            if clip::write(&text).await {
                copied.set(true);
            }
        });
    };
    let name_display: AnyView = if link_to_profile {
        let href = player_path(lang, &name, &tag_hex);
        let tip = words::player_open_profile(lang);
        view! {
            <a class="handle-link" href=href title=tip translate="no">
                <bdi>{name}</bdi>
                <span class="tag handle-tag">{"#"}{tag_hex}</span>
            </a>
        }
        .into_any()
    } else {
        view! {
            <span class="handle-text" translate="no">
                <bdi>{name}</bdi>
                <span class="tag handle-tag">{"#"}{tag_hex}</span>
            </span>
        }
        .into_any()
    };

    view! {
        <span class="handle-with-copy">
            {name_display}
            <button
                type="button"
                class="handle-copy-btn"
                title=move || if copied.get() { words::handle_copied_label(lang) } else { words::handle_copy_label(lang) }
                aria-label=move || if copied.get() { words::handle_copied_label(lang) } else { words::handle_copy_label(lang) }
                on:click=on_copy
            >
                {move || if copied.get() { glyph(glyph::CHECK) } else { glyph(glyph::COPY) }}
            </button>
        </span>
    }
    .into_any()
}
