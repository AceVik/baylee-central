//! Moving the focus when an island replaces the part that held it: a form
//! by its next step, a button by what it revealed, a list by its reload.
//! Otherwise the focus falls to the page, and the next Tab goes on at the
//! page's end, past what is new.

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys;

/// Once the browser drew the change, focuses the first element `selector`
/// finds (one that takes no Tab stop carries `tabindex="-1"`).
pub(crate) fn after_render(selector: impl Into<String>) {
    let selector = selector.into();
    request_animation_frame(move || {
        let target = document()
            .query_selector(&selector)
            .ok()
            .flatten()
            .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok());
        if let Some(element) = target {
            let _ = element.focus();
        }
    });
}

/// For a part that only an action shows (never the page's first render):
/// once it is drawn, `selector` in it takes the focus. Effects run in the
/// browser only.
pub(crate) fn when_shown(selector: &'static str) {
    Effect::new(move |_| after_render(selector));
}
