//! Drawing messages: text with tags, paragraphs, icons; for the pages and
//! the islands alike.

use leptos::prelude::*;

use crate::words::Piece;

/// Whether a rich-text link opens in a new tab: external HTTPS links and
/// legal documents (`terms`, `privacy`, `impressum`, `fan-content`,
/// `notices.txt`).
fn opens_in_new_tab(url: &str) -> bool {
    if url.starts_with("https://") {
        return true;
    }
    let path = url.split(['#', '?']).next().unwrap_or(url);
    !path.contains("/account/")
        && [
            "/terms",
            "/privacy",
            "/impressum",
            "/fan-content",
            "/notices.txt",
        ]
        .iter()
        .any(|suffix| path.ends_with(suffix))
}

/// A message with tags: `b`, `i`, `code`, `kbd` and `glow` drawn as such, `nb`
/// as words a line break does not part (a sentence of a title); any other
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
            Some("nb") => view! { <span class="keep">{piece.text}</span> }.into_any(),
            Some(tag) => match href(tag) {
                Some(url) if opens_in_new_tab(&url) => {
                    view! { <a href=url target="_blank" rel="noopener noreferrer">{piece.text}</a> }
                        .into_any()
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

#[cfg(test)]
mod tests {
    use super::opens_in_new_tab;

    #[test]
    fn external_and_legal_links_open_in_new_tab() {
        for url in [
            "https://example.com/policy",
            "/de/terms",
            "/en/privacy",
            "/en/privacy#email",
            "/fr/impressum",
            "/ja/fan-content",
            "/notices.txt",
        ] {
            assert!(opens_in_new_tab(url), "{url} should open in a new tab");
        }
        for url in [
            "/en/download",
            "/de/gateways",
            "/en/account/privacy",
            "/site/impressum/kontakt",
            "#limits",
        ] {
            assert!(!opens_in_new_tab(url), "{url} should stay in the same tab");
        }
    }
}
