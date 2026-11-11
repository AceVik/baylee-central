//! Drawing messages: text with tags, paragraphs, icons; for the pages and
//! the islands alike.

use leptos::prelude::*;

use crate::words::Piece;

/// A message with tags: `b`, `i`, `code`, `kbd` and `glow` drawn as such; any other
/// tag as a link where `href` names one for it, else as its text. Nothing
/// in a message is ever read as HTML.
pub fn rich(pieces: Vec<Piece>, href: impl Fn(&str) -> Option<String>) -> AnyView {
    pieces
        .into_iter()
        .map(|piece| match piece.tag.as_deref() {
            None => piece.text.into_any(),
            Some("b") => view! { <strong>{piece.text}</strong> }.into_any(),
            Some("i") => view! { <em>{piece.text}</em> }.into_any(),
            Some("code") => view! { <code translate="no">{piece.text}</code> }.into_any(),
            Some("kbd") => view! { <kbd>{piece.text}</kbd> }.into_any(),
            Some("glow") => view! { <span class="glow">{piece.text}</span> }.into_any(),
            Some(tag) => match href(tag) {
                Some(url) if url.starts_with("https://") => {
                    view! { <a href=url target="_blank" rel="noopener noreferrer">{piece.text}</a> }.into_any()
                }
                Some(url) => view! { <a href=url>{piece.text}</a> }.into_any(),
                None => piece.text.into_any(),
            },
        })
        .collect_view()
        .into_any()
}

#[cfg(not(target_arch = "wasm32"))]
/// A message without links of its own: only the plain tags.
pub fn plain(pieces: Vec<Piece>) -> AnyView {
    rich(pieces, |_| None)
}

#[cfg(not(target_arch = "wasm32"))]
/// Text with paragraph breaks (`\n\n`) as paragraphs.
pub fn paragraphs(text: &str) -> AnyView {
    text.split("\n\n")
        .map(|paragraph| view! { <p>{paragraph.to_owned()}</p> })
        .collect_view()
        .into_any()
}

#[cfg(not(target_arch = "wasm32"))]
/// A Lucide icon, hidden from assistive technology: the words beside it
/// carry the meaning.
pub fn icon(paths: &'static str) -> impl IntoView {
    view! {
        <svg
            class="icon"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
            focusable="false"
            inner_html=paths
        ></svg>
    }
}
