//! Moving the focus when an island replaces the part that held it: a form
//! by its next step, a button by what it revealed, a list by its reload.
//! Otherwise the focus falls to the page, and the next Tab goes on at the
//! page's end, past what is new.

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::wasm_bindgen::closure::Closure;
use leptos::web_sys;

/// Once the browser drew the change, focuses the first element `selector`
/// finds (one that takes no Tab stop carries `tabindex="-1"`).
///
/// The frame is asked of the window itself, not through Leptos: Leptos
/// (0.9) cancels a frame its owner asked for when that owner goes, and the
/// part that asks for the focus is often the one being replaced.
pub(crate) fn after_render(selector: impl Into<String>) {
    let selector = selector.into();
    let callback = Closure::once_into_js(move || {
        let target = document()
            .query_selector(&selector)
            .ok()
            .flatten()
            .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok());
        if let Some(element) = target {
            let _ = element.focus();
        }
    });
    let _ = window().request_animation_frame(callback.unchecked_ref());
}

/// For a part that only an action shows (never the page's first render):
/// once it is drawn, `selector` in it takes the focus. Effects run in the
/// browser only.
pub(crate) fn when_shown(selector: &'static str) {
    Effect::new(move |_| after_render(selector));
}
