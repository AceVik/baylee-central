//! The label beside an AI player's name (`docs/ai-players.md` §6): the
//! administrator's label, after a small mark of the AI's kind, in the
//! pill of a state. It says in words what it is: a reader hears "AI
//! player, house AI:" before the label, and the pointer's tip names the
//! kind. Every page that names an account Central marks as an AI draws it
//! beside the name: the public profile, a mention's preview in the chat,
//! and the administration's lists.

use central_api::ranked::ai::{AiBadge, AiKind};
use leptos::prelude::*;

use super::super::chat::glyph::{self, Shape};
use crate::words::{self, Lang};

/// The mark of a kind.
#[must_use]
pub fn kind_glyph(kind: AiKind) -> &'static [Shape] {
    match kind {
        AiKind::House | AiKind::Other => glyph::BOT,
        AiKind::Model => glyph::SPARKLES,
        AiKind::Net => glyph::CPU,
    }
}

/// The words for a kind.
#[must_use]
pub fn kind_text(lang: Lang, kind: AiKind) -> &'static str {
    match kind {
        AiKind::House => words::ai_kind_house(lang),
        AiKind::Model => words::ai_kind_model(lang),
        AiKind::Net => words::ai_kind_net(lang),
        AiKind::Other => words::ai_kind_other(lang),
    }
}

/// The kind as the stylesheet and the browser tests name it.
#[must_use]
pub const fn kind_key(kind: AiKind) -> &'static str {
    match kind {
        AiKind::House => "house",
        AiKind::Model => "model",
        AiKind::Net => "net",
        AiKind::Other => "other",
    }
}

/// The badge: the kind's mark, what it is for a reader, and the label.
pub fn ai_badge(lang: Lang, badge: &AiBadge) -> AnyView {
    let kind = kind_text(lang, badge.kind);
    view! {
        <span class="pill info ai-badge" data-kind=kind_key(badge.kind) title=kind>
            {glyph::glyph(kind_glyph(badge.kind))}
            <span class="visually-hidden">{words::ai_badge_said(lang, kind)}{" "}</span>
            <bdi translate="no">{badge.label.clone()}</bdi>
        </span>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every kind has its own words in every language, and the words a
    /// reader hears name the kind.
    #[test]
    fn every_kind_has_its_own_words() {
        let kinds = [AiKind::House, AiKind::Model, AiKind::Net, AiKind::Other];
        for lang in Lang::ALL {
            let texts: Vec<&str> = kinds.iter().map(|kind| kind_text(lang, *kind)).collect();
            for (at, text) in texts.iter().enumerate() {
                assert!(!text.is_empty(), "{lang:?}");
                assert!(!texts[..at].contains(text), "{lang:?}: {text}");
                assert!(words::ai_badge_said(lang, text).contains(text), "{lang:?}");
            }
        }
        let keys: Vec<&str> = kinds.iter().map(|kind| kind_key(*kind)).collect();
        assert_eq!(keys, ["house", "model", "net", "other"]);
    }

    /// The badge's markup holds the label and the kind, also on the
    /// server, where no script runs.
    #[test]
    fn the_badge_is_plain_markup() {
        let badge = AiBadge {
            kind: AiKind::Net,
            label: "Baylee Net".into(),
        };
        let owner = Owner::new();
        let html = owner.with(|| ai_badge(Lang::En, &badge).to_html());
        assert!(html.contains("Baylee Net"), "{html}");
        assert!(html.contains(r#"data-kind="net""#), "{html}");
        assert!(html.contains(words::ai_kind_net(Lang::En)), "{html}");
    }
}
