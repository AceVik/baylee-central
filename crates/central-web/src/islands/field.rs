//! What a form's field carries inside it: a glyph at its start that says
//! what belongs there, and in a password's field the eye at its end that
//! shows what was typed.

use leptos::prelude::*;

use super::chat::glyph::{self, Shape};
use crate::words::{self, Lang};

/// The glyph inside a field's start. Decoration: the label names the field.
pub fn mark(shapes: &'static [Shape]) -> impl IntoView {
    view! { <span class="inset-mark">{glyph::glyph(shapes)}</span> }
}

/// The eye inside a password field's end: a button of its own with a
/// 44 px target, pressed while `shown` (the field `field` then stands in
/// plain text).
pub fn eye(lang: Lang, shown: RwSignal<bool>, field: &'static str) -> impl IntoView {
    view! {
        <button type="button" class="eye" aria-controls=field
            aria-label=words::password_show_label(lang) title=words::password_show_label(lang)
            aria-pressed=move || if shown.get() { "true" } else { "false" }
            on:click=move |_| shown.update(|s| *s = !*s)>
            <span class="eye-open">{glyph::glyph(glyph::EYE)}</span>
            <span class="eye-shut">{glyph::glyph(glyph::EYE_OFF)}</span>
        </button>
    }
}
