//! The clipboard, for a code a person copies: `navigator.clipboard`, reached
//! through `js_sys` alone, so no further `web-sys` feature is needed. A
//! browser without it, or one that refuses, says so and the page tells the
//! person to copy by hand.

use leptos::prelude::*;

use super::chat::glyph;
use crate::words::{self, Lang};

/// A button that copies `code` and the line that says whether the browser
/// took it. The two go into the page apart: the button among a step's
/// actions, the line (a status region) under them.
pub fn copier(lang: Lang, code: String) -> (impl IntoView, impl IntoView) {
    let said = RwSignal::new(None::<&'static str>);
    let copy = move |_| {
        let code = code.clone();
        leptos::task::spawn_local(async move {
            said.set(Some(if write(&code).await {
                words::code_copied(lang)
            } else {
                words::code_copy_failed(lang)
            }));
        });
    };
    (
        view! {
            <button type="button" class="button" on:click=copy>
                {glyph::glyph(glyph::COPY)}
                <span>{words::code_copy(lang)}</span>
            </button>
        },
        view! { <p class="meta" role="status">{move || said.get()}</p> },
    )
}

/// Writes `text` to the clipboard; whether the browser took it.
#[cfg(target_arch = "wasm32")]
pub async fn write(text: &str) -> bool {
    use wasm_bindgen::{JsCast, JsValue};

    let asked = || {
        let navigator =
            js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("navigator")).ok()?;
        let clipboard = js_sys::Reflect::get(&navigator, &JsValue::from_str("clipboard")).ok()?;
        let write: js_sys::Function =
            js_sys::Reflect::get(&clipboard, &JsValue::from_str("writeText"))
                .ok()?
                .dyn_into()
                .ok()?;
        write
            .call1(&clipboard, &JsValue::from_str(text))
            .ok()?
            .dyn_into::<js_sys::Promise>()
            .ok()
    };
    match asked() {
        Some(promise) => wasm_bindgen_futures::JsFuture::from(promise).await.is_ok(),
        None => false,
    }
}

/// On the server there is no clipboard.
#[cfg(not(target_arch = "wasm32"))]
pub fn write(_text: &str) -> std::future::Ready<bool> {
    std::future::ready(false)
}
