//! What the browser remembers of a sign-in: its expiry and a random, non-secret
//! change marker, so pages know to ask Central whose session it is. The marker
//! distinguishes even two sign-ins with the same expiry second; it carries no
//! identity and grants no authority. The session itself is the `__Host-central`
//! cookie, which no script reads. Old expiry-only records remain readable.
//!
//! Beside it stands the header's note of who is signed in (`OWN`): the
//! display name, the tag and the own picture's key, each what other people
//! see of the account, and the session record the note belongs to. The
//! pages that learn them write it, `boot.js` draws the account link from
//! it without asking anyone, and it goes with the session: at signing out,
//! at another sign-in, and when a page finds no live session.

/// The key, unchanged from the previous site.
#[cfg(target_arch = "wasm32")]
pub const KEY: &str = "baylee-central-session";

/// Grace window (in seconds) beyond the 15-minute access cookie expiry during
/// which the 3-day sliding refresh cookie can still silently rotate the session.
#[cfg(any(target_arch = "wasm32", test))]
const REFRESH_GRACE_SECS: i64 = 3 * 86_400 - 15 * 60;

#[cfg(any(target_arch = "wasm32", test))]
const fn within_refresh_grace(expires_at: i64, now: i64) -> bool {
    expires_at.saturating_add(REFRESH_GRACE_SECS) > now
}

#[cfg(any(target_arch = "wasm32", test))]
fn renewed_record(
    old_text: Option<&str>,
    expires_at: i64,
    fresh_change: impl FnOnce() -> Option<[u8; 16]>,
) -> Option<String> {
    let existing = old_text
        .and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok())
        .and_then(|mut value| value.as_object_mut()?.remove("change"))
        .filter(|value| !value.is_null());
    let change = match existing {
        Some(value) => value,
        None => serde_json::json!(fresh_change()?),
    };
    Some(serde_json::json!({ "expiresAt": expires_at, "change": change }).to_string())
}

#[cfg(any(target_arch = "wasm32", test))]
fn extract_login_stamp(raw: &str) -> String {
    serde_json::from_str::<serde_json::Value>(raw)
        .ok()
        .and_then(|value| value.get("change").filter(|v| !v.is_null()).cloned())
        .and_then(|change| serde_json::to_string(&change).ok())
        .unwrap_or_else(|| raw.to_owned())
}

#[cfg(any(target_arch = "wasm32", test))]
fn same_login_refresh(old: Option<&str>, new: Option<&str>) -> bool {
    let (Some(old), Some(new)) = (old, new) else {
        return false;
    };
    let parse_change = |raw: &str| {
        serde_json::from_str::<serde_json::Value>(raw)
            .ok()?
            .as_object()?
            .get("change")
            .filter(|v| !v.is_null())
            .cloned()
    };
    matches!((parse_change(old), parse_change(new)), (Some(a), Some(b)) if a == b)
}

#[cfg(target_arch = "wasm32")]
fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

/// Remembers that a session lasts until `expires_at` (Unix seconds).
#[cfg(target_arch = "wasm32")]
pub fn remember(expires_at: i64) {
    if let Some(storage) = storage() {
        // another sign-in: what the header noted of the one before goes
        let _ = storage.remove_item(OWN);
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

/// Renews the remembered session expiry while preserving the existing
/// `"change"` marker (so silent refreshes do not look like a new sign-in) and
/// keeping the header's local note (`OWN`: display name, tag, picture key, and
/// highest rank) whenever it belonged to the previous session record.
#[cfg(target_arch = "wasm32")]
pub fn renew(expires_at: i64) {
    use serde_json::Value;
    let Some(storage) = storage() else {
        return;
    };
    let old_text = storage.get_item(KEY).ok().flatten();
    let kept_own = old_text.as_deref().and_then(|old| {
        storage
            .get_item(OWN)
            .ok()
            .flatten()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .and_then(|value| match value {
                Value::Object(held) => Some(held),
                _ => None,
            })
            .filter(|held| held.get("of").and_then(Value::as_str) == Some(old))
    });
    let Some(new_key) = renewed_record(old_text.as_deref(), expires_at, || {
        let mut bytes = [0_u8; 16];
        getrandom::fill(&mut bytes).is_ok().then_some(bytes)
    }) else {
        let _ = storage.remove_item(KEY);
        let _ = storage.remove_item(OWN);
        return;
    };
    if storage.set_item(KEY, &new_key).is_err() {
        let _ = storage.remove_item(KEY);
        let _ = storage.remove_item(OWN);
        return;
    }
    if let Some(mut held) = kept_own {
        held.insert("of".to_owned(), Value::String(new_key));
        if storage
            .set_item(OWN, &Value::Object(held).to_string())
            .is_err()
        {
            let _ = storage.remove_item(OWN);
        }
    } else {
        let _ = storage.remove_item(OWN);
    }
    if let Some(window) = web_sys::window()
        && let Ok(event) = web_sys::Event::new("baylee:own")
    {
        let _ = window.dispatch_event(&event);
    }
}

/// Forgets the session.
#[cfg(target_arch = "wasm32")]
pub fn forget() {
    if let Some(storage) = storage() {
        let _ = storage.remove_item(KEY);
        let _ = storage.remove_item(OWN);
    }
}

/// The key of the header's note of who is signed in (`boot.js`).
#[cfg(target_arch = "wasm32")]
pub const OWN: &str = "baylee-central-own";

/// Changes the header's note of who is signed in and tells the page
/// (`baylee:own`, at which `boot.js` draws the account link again). The
/// note holds the session record it belongs to (`of`), so one of another
/// sign-in is started afresh here and never shown there; without a session
/// there is no note.
#[cfg(target_arch = "wasm32")]
fn note(change: impl FnOnce(&mut serde_json::Map<String, serde_json::Value>)) {
    use serde_json::Value;
    let Some(storage) = storage() else {
        return;
    };
    if let Some(of) = marker() {
        let mut held = storage
            .get_item(OWN)
            .ok()
            .flatten()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .and_then(|value| match value {
                Value::Object(held) => Some(held),
                _ => None,
            })
            .filter(|held| held.get("of").and_then(Value::as_str) == Some(of.as_str()))
            .unwrap_or_default();
        held.insert("of".to_owned(), Value::String(of));
        change(&mut held);
        if storage
            .set_item(OWN, &Value::Object(held).to_string())
            .is_err()
        {
            let _ = storage.remove_item(OWN);
        }
    } else {
        let _ = storage.remove_item(OWN);
    }
    if let Some(window) = web_sys::window()
        && let Ok(event) = web_sys::Event::new("baylee:own")
    {
        let _ = window.dispatch_event(&event);
    }
}

/// Notes the signed-in account's display name and tag for the header, or
/// that it has none yet. Both are what other people see of the account.
#[cfg(target_arch = "wasm32")]
pub fn note_name(name: Option<&str>, tag: Option<u32>) {
    note(|held| {
        if let Some((name, tag)) = name.zip(tag) {
            held.insert("name".to_owned(), name.into());
            held.insert("tag".to_owned(), central_api::profile::tag_text(tag).into());
        } else {
            held.remove("name");
            held.remove("tag");
        }
    });
}

/// Maps a ranked tier into `(tier_index, division, champion_podium)`.
#[must_use]
pub fn rank_parts(rank: central_api::ranked::Rank) -> (u8, u8, u8) {
    use central_api::ranked::League;
    let tier = match rank.league {
        League::Paper => 0,
        League::Wood => 1,
        League::Iron => 2,
        League::Bronze => 3,
        League::Silver => 4,
        League::Gold => 5,
        League::Platinum => 6,
        League::Diamond => 7,
        League::Master => 8,
        League::Grandmaster => 9,
        League::Champion => 10,
        League::Unranked => 11,
    };
    let div = rank.division.unwrap_or(0);
    let podium = rank.champion_place.filter(|&p| p <= 3).unwrap_or(0);
    (tier, div, podium)
}

/// Notes the signed-in account's own picture for the header: its key in
/// hexadecimal and whether it moves, or that there is none.
#[cfg(target_arch = "wasm32")]
pub fn note_picture(key: Option<&str>, moving: bool) {
    note(|held| {
        held.insert(
            "key".to_owned(),
            key.map_or(serde_json::Value::Null, Into::into),
        );
        held.insert("moving".to_owned(), moving.into());
    });
}

/// Notes the signed-in account's highest current-season rank for the header
/// avatar border.
#[cfg(target_arch = "wasm32")]
pub fn note_rank(rank: Option<&central_api::ranked::Rank>) {
    note(|held| {
        if let Some(rank) = rank {
            let (tier, div, podium) = rank_parts(*rank);
            held.insert(
                "rank".to_owned(),
                serde_json::json!({ "tier": tier, "div": div, "podium": podium }),
            );
        } else {
            held.remove("rank");
        }
    });
}

/// Whether the header's note of this sign-in holds a picture's key.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn noted_picture() -> bool {
    let Some(of) = marker() else {
        return false;
    };
    read(OWN)
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .filter(|held| held.get("of").and_then(serde_json::Value::as_str) == Some(of.as_str()))
        .and_then(|held| held.get("key")?.as_str().map(str::len))
        .is_some_and(|len| len == 32)
}

#[cfg(target_arch = "wasm32")]
fn now_secs() -> Option<i64> {
    format!("{:.0}", (js_sys::Date::now() / 1000.0).trunc())
        .parse()
        .ok()
}

/// When the remembered access session ends, if its 3-day sliding refresh
/// window is still live.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn remembered() -> Option<i64> {
    let text = storage()?.get_item(KEY).ok().flatten()?;
    let ends = serde_json::from_str::<serde_json::Value>(&text)
        .ok()?
        .get("expiresAt")?
        .as_i64()?;
    let now = now_secs()?;
    within_refresh_grace(ends, now).then_some(ends)
}

/// Whether the short-lived access cookie has expired while the 3-day sliding
/// refresh window is still live.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn access_expired() -> bool {
    let Some(ends) = remembered() else {
        return false;
    };
    let Some(now) = now_secs() else {
        return false;
    };
    ends <= now
}

/// A local snapshot for matching the header's note (`OWN["of"]`) to the exact
/// session record, never for authorization.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn marker() -> Option<String> {
    remembered()?;
    read(KEY)
}

/// A stable sign-in stamp for fencing async private UI work (the serialized
/// `"change"` field of `KEY`, falling back to raw `KEY` text for legacy
/// expiry-only records). Survives silent session refreshes while changing on
/// sign-out or sign-in as another account.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn login_stamp() -> Option<String> {
    remembered()?;
    let raw = read(KEY)?;
    Some(extract_login_stamp(&raw))
}

/// Invalidate account-owned page state after another tab changes its sign-in.
/// Silent refreshes of the same sign-in (`oldValue` and `newValue` share an
/// identical non-null `"change"` marker) are ignored. The listener is removed
/// with the current Leptos owner.
#[cfg(target_arch = "wasm32")]
pub fn on_change(callback: impl Fn() + 'static) {
    let listener = leptos::prelude::window_event_listener_untyped("storage", move |event| {
        let Ok(key) = js_sys::Reflect::get(event.as_ref(), &wasm_bindgen::JsValue::from_str("key"))
        else {
            return;
        };
        let key_str = key.as_string();
        if !key.is_null() && key_str.as_deref() != Some(KEY) {
            return;
        }
        if key_str.as_deref() == Some(KEY) {
            let old =
                js_sys::Reflect::get(event.as_ref(), &wasm_bindgen::JsValue::from_str("oldValue"))
                    .ok()
                    .and_then(|v| v.as_string());
            let new =
                js_sys::Reflect::get(event.as_ref(), &wasm_bindgen::JsValue::from_str("newValue"))
                    .ok()
                    .and_then(|v| v.as_string());
            if same_login_refresh(old.as_deref(), new.as_deref()) {
                return;
            }
        }
        callback();
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

/// On the server nothing is renewed.
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub fn renew(_expires_at: i64) {}

/// On the server nothing is opened.
#[cfg(not(target_arch = "wasm32"))]
pub fn open(_address: &str) {}

/// On the server nothing is remembered.
#[cfg(not(target_arch = "wasm32"))]
pub fn forget() {}

/// On the server nothing is noted.
#[cfg(not(target_arch = "wasm32"))]
pub fn note_name(_name: Option<&str>, _tag: Option<u32>) {}

/// On the server nothing is noted.
#[cfg(not(target_arch = "wasm32"))]
pub fn note_rank(_rank: Option<&central_api::ranked::Rank>) {}

/// On the server nothing is remembered.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn remembered() -> Option<i64> {
    None
}

/// On the server there is no browser session to expire.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn access_expired() -> bool {
    false
}

/// The server retains no browser session marker.
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[must_use]
pub fn marker() -> Option<String> {
    None
}

/// The server retains no browser sign-in stamp.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn login_stamp() -> Option<String> {
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
    use super::{
        REFRESH_GRACE_SECS, extract_login_stamp, form_address, local, renewed_record,
        same_login_refresh, within_refresh_grace,
    };

    #[test]
    fn refresh_grace_covers_three_days_from_issue() {
        assert_eq!(REFRESH_GRACE_SECS, 258_300);
        let access_exp = 1_700_000_900;
        assert!(within_refresh_grace(access_exp, access_exp - 1));
        assert!(within_refresh_grace(access_exp, access_exp + 3600));
        assert!(within_refresh_grace(
            access_exp,
            access_exp + REFRESH_GRACE_SECS - 1
        ));
        assert!(!within_refresh_grace(
            access_exp,
            access_exp + REFRESH_GRACE_SECS
        ));
    }

    #[test]
    fn renew_preserves_change_and_keeps_exact_schema() {
        let old = r#"{"change":[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16],"expiresAt":1000}"#;
        let renewed = renewed_record(Some(old), 2000, || Some([9_u8; 16])).expect("renewed");
        let parsed: serde_json::Value = serde_json::from_str(&renewed).expect("json");
        let obj = parsed.as_object().expect("object");
        let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, ["change", "expiresAt"]);
        assert_eq!(extract_login_stamp(old), extract_login_stamp(&renewed));
        assert!(same_login_refresh(Some(old), Some(&renewed)));

        let other = r#"{"change":[9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9],"expiresAt":2000}"#;
        assert!(!same_login_refresh(Some(old), Some(other)));
        assert!(!same_login_refresh(Some(old), None));
        assert!(!same_login_refresh(None, Some(&renewed)));

        let legacy = r#"{"expiresAt":1000}"#;
        assert_eq!(extract_login_stamp(legacy), legacy);
        assert!(!same_login_refresh(Some(legacy), Some(legacy)));
        let upgraded = renewed_record(Some(legacy), 2000, || Some([7_u8; 16])).expect("upgraded");
        assert!(upgraded.contains("[7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7]"));
    }

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
