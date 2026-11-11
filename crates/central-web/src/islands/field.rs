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

/// What a labelled field is, apart from what it holds.
#[derive(Clone, Copy)]
pub struct Labelled {
    /// The input's id; its hint's is `<id>-hint`.
    pub id: &'static str,
    /// The label's words.
    pub label: &'static str,
    /// A line under the field that says what belongs in it.
    pub hint: Option<&'static str>,
    /// The glyph inside its start.
    pub mark: &'static [Shape],
    /// `autocomplete`: a field for a password (`…-password`) hides what is
    /// typed and has the eye.
    pub auto: &'static str,
    /// The most characters it takes.
    pub max: &'static str,
}

/// A labelled field of a form: its glyph, its input bound both ways (a form
/// drawn again shows what it held), a password's eye and the hint.
pub fn labelled(
    lang: Lang,
    what: Labelled,
    value: impl Fn() -> String + Send + Sync + 'static,
    set: impl Fn(String) + 'static,
) -> impl IntoView {
    let Labelled {
        id,
        label,
        hint,
        mark: shapes,
        auto,
        max,
    } = what;
    let secret = auto.ends_with("-password");
    let shown = RwSignal::new(!secret);
    let eye = secret.then(|| eye(lang, shown, id));
    let hint_id = format!("{id}-hint");
    let described = hint.map(|_| hint_id.clone());
    let hint = hint.map(|hint| view! { <p class="hint" id=hint_id>{hint}</p> });
    view! {
        <div class="field">
            <label for=id>{label}</label>
            <div class="inset">
                {mark(shapes)}
                <input id=id autocomplete=auto autocapitalize="none" spellcheck="false" maxlength=max aria-describedby=described
                    type=move || if shown.get() { "text" } else { "password" }
                    on:input=move |ev| set(event_target_value(&ev)) prop:value=value />
                {eye}
            </div>
            {hint}
        </div>
    }
}
