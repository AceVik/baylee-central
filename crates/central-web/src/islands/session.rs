//! What the browser remembers of a sign-in: its expiry and a random, non-secret
//! change marker, so pages know to ask Central whose session it is. The marker
//! distinguishes even two sign-ins with the same expiry second; it carries no
//! identity and grants no authority. The session itself is the `__Host-central`
//! cookie, which no script reads. Old expiry-only records remain readable.

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
        let mut change = [0_u8; 16];
        if getrandom::fill(&mut change).is_err() {
            // Without a distinct marker do not leave another tab believing its
            // previous account is still current. The cookie remains server-owned.
            let _ = storage.remove_item(KEY);
            return;
        }
        if storage
            .set_item(
                KEY,
                &serde_json::json!({ "expiresAt": expires_at, "change": change }).to_string(),
            )
            .is_err()
        {
            let _ = storage.remove_item(KEY);
        }
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

/// A local snapshot for fencing private UI work, never for authorization. The
/// full record distinguishes different sign-ins even when their expiry is equal.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn marker() -> Option<String> {
    remembered()?;
    read(KEY)
}

/// Invalidate account-owned page state after another tab changes its sign-in.
/// The listener is removed with the current Leptos owner.
#[cfg(target_arch = "wasm32")]
pub fn on_change(callback: impl Fn() + 'static) {
    let listener = leptos::prelude::window_event_listener_untyped("storage", move |event| {
        let key =
            js_sys::Reflect::get(event.as_ref(), &wasm_bindgen::JsValue::from_str("key")).ok();
        if key.is_some_and(|key| key.is_null() || key.as_string().as_deref() == Some(KEY)) {
            callback();
        }
    });
    leptos::prelude::on_cleanup(move || listener.remove());
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

/// Whether `path` names a place on this site and nowhere else: it starts
/// with one slash, what follows is neither a slash nor a backslash
/// (browsers read `/\host` as they read `//host`, another site), and it
/// holds no control character (a browser drops a tab or a line break
/// from an address before it reads it, so `/<tab>/host` is `//host` too).
#[cfg(any(target_arch = "wasm32", test))]
#[must_use]
pub fn local(path: &str) -> bool {
    let mut bytes = path.bytes();
    bytes.next() == Some(b'/')
        && !matches!(bytes.next(), Some(b'/' | b'\\'))
        && !path.bytes().any(|b| b.is_ascii_control())
}

/// Whether `path` is a form's own address, `/{lang}/signin`, `/register`
/// or `/reset`: the entry in the history a dialog stands at (`boot.js`).
#[cfg(any(target_arch = "wasm32", test))]
#[must_use]
pub fn form_address(path: &str) -> bool {
    let mut parts = path.trim_end_matches('/').split('/');
    parts.next() == Some("")
        && parts.next().is_some_and(|lang| !lang.is_empty())
        && matches!(parts.next(), Some("signin" | "register" | "reset"))
        && parts.next().is_none()
}

/// Sends the browser to `path` on this site. From a form's own address the
/// new page takes that entry's place in the history: the form is done
/// with, and Back from where it led does not open it again.
#[cfg(target_arch = "wasm32")]
pub fn go(path: &str) {
    if local(path)
        && let Some(window) = web_sys::window()
    {
        let location = window.location();
        let _ = if location.pathname().is_ok_and(|at| form_address(&at)) {
            location.replace(path)
        } else {
            location.assign(path)
        };
    }
}

/// Opens `address` (`mailto:…`, `tel:…`) as a link would: the browser
/// hands it to the program that takes it, and the page stays. The address
/// exists from the click on and is never in the page.
#[cfg(target_arch = "wasm32")]
pub fn open(address: &str) {
    if let Some(window) = web_sys::window() {
        let _ = window.location().assign(address);
    }
}

/// On the server nothing is remembered.
#[cfg(not(target_arch = "wasm32"))]
pub fn remember(_expires_at: i64) {}

/// On the server nothing is opened.
#[cfg(not(target_arch = "wasm32"))]
pub fn open(_address: &str) {}

/// On the server nothing is remembered.
#[cfg(not(target_arch = "wasm32"))]
pub fn forget() {}

/// On the server nothing is remembered.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn remembered() -> Option<i64> {
    None
}

/// The server retains no browser session marker.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn marker() -> Option<String> {
    None
}

/// The server has no cross-tab browser events.
#[cfg(not(target_arch = "wasm32"))]
pub fn on_change(_callback: impl Fn() + 'static) {}

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

#[cfg(test)]
mod tests {
    use super::{form_address, local};

    #[test]
    fn only_a_path_on_this_site_is_gone_to() {
        for path in ["/", "/de/account", "/en/account/profile", "/de/#cards"] {
            assert!(local(path), "{path}");
        }
        for path in [
            "",
            "de/account",
            "//example.org",
            "/\\example.org",
            "https://example.org/",
            "\\/example.org",
            "javascript:alert(1)",
            "/\t/example.org",
            "/\n/example.org",
            "/\r\\example.org",
            "/de/account\t",
        ] {
            assert!(!local(path), "{path}");
        }
    }

    #[test]
    fn a_form_has_three_addresses_in_each_language() {
        for path in ["/de/signin", "/en/register", "/pt-br/reset", "/de/signin/"] {
            assert!(form_address(path), "{path}");
        }
        for path in [
            "/de/",
            "/de",
            "/signin",
            "//signin",
            "/de/account",
            "/de/signin/more",
            "/de/account/signin",
            "de/signin",
        ] {
            assert!(!form_address(path), "{path}");
        }
    }
}
