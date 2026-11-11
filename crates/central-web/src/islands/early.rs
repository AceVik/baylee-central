//! What a person typed before the islands ran. The server-rendered fields
//! take text from the first paint on, while the WebAssembly comes later
//! (`js/islands.js` waits for the page's largest paint, and far down a page
//! for the reader to come near): an island starts its signals from the
//! fields as they stand, or its first render would empty them. Read through
//! `js_sys` alone, so no further `web-sys` feature is needed.

/// The text in the field with the id `id` as the page holds it now; empty
/// on the server, where nothing was typed.
pub fn typed(id: &str) -> String {
    property(id, "value")
        .and_then(|value| value.as_string())
        .unwrap_or_default()
}

/// Whether the checkbox with the id `id` is ticked now; false on the
/// server.
pub fn ticked(id: &str) -> bool {
    property(id, "checked")
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

#[cfg(target_arch = "wasm32")]
fn property(id: &str, name: &str) -> Option<wasm_bindgen::JsValue> {
    use wasm_bindgen::{JsCast, JsValue};
    let document = js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("document")).ok()?;
    let find: js_sys::Function =
        js_sys::Reflect::get(&document, &JsValue::from_str("getElementById"))
            .ok()?
            .dyn_into()
            .ok()?;
    let field = find.call1(&document, &JsValue::from_str(id)).ok()?;
    if field.is_null() || field.is_undefined() {
        return None;
    }
    js_sys::Reflect::get(&field, &JsValue::from_str(name)).ok()
}

/// On the server there is no page yet to read.
#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::unnecessary_wraps)]
fn property(_id: &str, _name: &str) -> Option<Never> {
    None
}

/// A value that never exists: the server's [`property`] finds nothing.
#[cfg(not(target_arch = "wasm32"))]
enum Never {}

#[cfg(not(target_arch = "wasm32"))]
impl Never {
    fn as_string(&self) -> Option<String> {
        match *self {}
    }

    fn as_bool(&self) -> Option<bool> {
        match *self {}
    }
}
