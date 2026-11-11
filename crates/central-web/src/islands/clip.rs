//! The clipboard, for a code a person copies: `navigator.clipboard`, reached
//! through `js_sys` alone, so no further `web-sys` feature is needed. A
//! browser without it, or one that refuses, says so and the page tells the
//! person to copy by hand.

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
