//! What the browser remembers of a sign-in: only when the session ends
//! (localStorage `baylee-central-session`, `{"expiresAt": <unix seconds>}`,
//! as before), so a page knows to ask Central whose session it is. The
//! session itself is the `__Host-central` cookie, which no script reads.

/// The key, unchanged from the previous site.
#[cfg(target_arch = "wasm32")]
pub const KEY: &str = "baylee-central-session";

#[cfg(target_arch = "wasm32")]
fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

/// Remembers that a session lasts until `expires_at` (Unix seconds).
#[cfg(target_arch = "wasm32")]
pub fn remember(expires_at: i64) {
    if let Some(storage) = storage() {
        let _ = storage.set_item(
            KEY,
            &serde_json::json!({ "expiresAt": expires_at }).to_string(),
        );
    }
}

/// Forgets the session.
#[cfg(target_arch = "wasm32")]
pub fn forget() {
    if let Some(storage) = storage() {
        let _ = storage.remove_item(KEY);
    }
}

/// When the remembered session ends, if one is remembered and has not.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn remembered() -> Option<i64> {
    let text = storage()?.get_item(KEY).ok().flatten()?;
    let ends = serde_json::from_str::<serde_json::Value>(&text)
        .ok()?
        .get("expiresAt")?
        .as_i64()?;
    let now: i64 = format!("{:.0}", (js_sys::Date::now() / 1000.0).trunc())
        .parse()
        .ok()?;
    (ends > now).then_some(ends)
}

/// The text localStorage keeps under `key`.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn read(key: &str) -> Option<String> {
    storage()?.get_item(key).ok().flatten()
}

/// Keeps `value` in localStorage under `key`, where the browser allows it.
#[cfg(target_arch = "wasm32")]
pub fn write(key: &str, value: &str) {
    if let Some(storage) = storage() {
        let _ = storage.set_item(key, value);
    }
}

/// Forgets what localStorage keeps under `key`.
#[cfg(target_arch = "wasm32")]
pub fn remove(key: &str) {
    if let Some(storage) = storage() {
        let _ = storage.remove_item(key);
    }
}

/// Sends the browser to `path` on this site.
#[cfg(target_arch = "wasm32")]
pub fn go(path: &str) {
    if path.starts_with('/')
        && !path.starts_with("//")
        && let Some(window) = web_sys::window()
    {
        let _ = window.location().assign(path);
    }
}

/// On the server nothing is remembered.
#[cfg(not(target_arch = "wasm32"))]
pub fn remember(_expires_at: i64) {}

/// On the server nothing is remembered.
#[cfg(not(target_arch = "wasm32"))]
pub fn forget() {}

/// On the server nothing is remembered.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn remembered() -> Option<i64> {
    None
}

/// On the server nothing is kept.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn read(_key: &str) -> Option<String> {
    None
}

/// On the server nothing is kept.
#[cfg(not(target_arch = "wasm32"))]
pub fn write(_key: &str, _value: &str) {}

/// On the server nothing is kept.
#[cfg(not(target_arch = "wasm32"))]
pub fn remove(_key: &str) {}

/// On the server nobody is sent anywhere.
#[cfg(not(target_arch = "wasm32"))]
pub fn go(_path: &str) {}
