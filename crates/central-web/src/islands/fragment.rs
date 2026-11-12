//! A code handed to a page in its address's fragment, as the mails'
//! buttons link (`docs/email.md`, "The code in the button's link"):
//! `/{lang}/reset#code=RESET-…` and `/{lang}/account/security#code=XXXX-XXXX`.
//! A browser sends no fragment to the site and puts none into a `Referer`.
//!
//! The island that takes one reads the fragment once, when it starts, keeps
//! it only if it is a code of its own kind, and removes it from the address
//! bar at once (`history.replaceState`, the rest of the address kept),
//! whatever followed `#code=`. A code that arrives while the page is open
//! (`watch`) is removed the same way and fills nothing. Nothing here sends,
//! logs or stores it, and no island submits a form by itself: the person
//! still presses the button, so a link scanner that opens the link spends
//! nothing. Read through `js_sys` alone, as `early` reads the fields, so no
//! further `web-sys` feature is needed.

/// What a fragment that hands a code begins with.
const PREFIX: &str = "#code=";
/// The most characters read after it: more than any code is written with
/// (a reset code has 45).
const LONGEST: usize = 64;

/// The code that `parse` reads in `hash` (as `location.hash` gives it,
/// with its `#`), if `hash` hands one: `#code=` and then at most
/// [`LONGEST`] letters, digits and hyphens that `parse` takes as a code of
/// its kind. Anything else hands no code.
pub fn code<T, E>(hash: &str, parse: impl FnOnce(&str) -> Result<T, E>) -> Option<T> {
    let text = hash.strip_prefix(PREFIX)?;
    let plain = !text.is_empty()
        && text.len() <= LONGEST
        && text.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-');
    plain.then(|| parse(text).ok()).flatten()
}

/// Whether the page removes the fragment `hash` from its address: one that
/// hands a code, or something in its place. Any other fragment (a place on
/// the page) stays.
#[cfg(any(target_arch = "wasm32", test))]
fn hands(hash: &str) -> bool {
    hash.starts_with(PREFIX)
}

/// The address `href` without its fragment.
#[cfg(any(target_arch = "wasm32", test))]
fn without_fragment(href: &str) -> &str {
    href.split_once('#').map_or(href, |(before, _)| before)
}

/// The property `name` of `target`, if it has one.
#[cfg(target_arch = "wasm32")]
fn get(target: &wasm_bindgen::JsValue, name: &str) -> Option<wasm_bindgen::JsValue> {
    js_sys::Reflect::get(target, &wasm_bindgen::JsValue::from_str(name))
        .ok()
        .filter(|value| !value.is_undefined() && !value.is_null())
}

/// The method `name` of `target`, if it has one.
#[cfg(target_arch = "wasm32")]
fn method(target: &wasm_bindgen::JsValue, name: &str) -> Option<js_sys::Function> {
    use wasm_bindgen::JsCast;
    get(target, name)?.dyn_into().ok()
}

/// The page's `location`, and its property `name` as text.
#[cfg(target_arch = "wasm32")]
fn location(name: &str) -> Option<(wasm_bindgen::JsValue, String)> {
    let location = get(&js_sys::global().into(), "location")?;
    let text = get(&location, name)
        .and_then(|value| value.as_string())
        .unwrap_or_default();
    Some((location, text))
}

/// The page's fragment, read once. One that hands a code is removed from
/// the address bar before this returns, the current entry of the history
/// replaced by the same address without it: going back or forward never
/// finds it again. On the server there is none.
#[cfg(target_arch = "wasm32")]
pub fn take() -> String {
    use wasm_bindgen::JsValue;
    let Some((location, hash)) = location("hash") else {
        return String::new();
    };
    if hands(&hash) {
        let href = get(&location, "href")
            .and_then(|value| value.as_string())
            .unwrap_or_default();
        let global: JsValue = js_sys::global().into();
        if let Some(history) = get(&global, "history")
            && let Some(replace) = method(&history, "replaceState")
        {
            // the same entry with its state, the same address but the code
            let state = get(&history, "state").unwrap_or(JsValue::NULL);
            let address = JsValue::from_str(without_fragment(&href));
            let _ = replace.call3(&history, &state, &JsValue::from_str(""), &address);
        }
    }
    hash
}

/// On the server there is no address to read.
#[cfg(not(target_arch = "wasm32"))]
pub fn take() -> String {
    String::new()
}

/// What the account page's link to the reset page puts in its fragment:
/// `/{lang}/account/admin#account=<the account's id in hex>`. An id is no
/// secret; in the fragment it stays out of the site's logs all the same.
#[cfg(any(target_arch = "wasm32", test))]
const ACCOUNT: &str = "#account=";

/// The account that the fragment `hash` names: `#account=` and the id as
/// 32 lowercase hex digits, and nothing else.
#[cfg(any(target_arch = "wasm32", test))]
fn account_in(hash: &str) -> Option<[u8; 16]> {
    crate::data::id_from_hex(hash.strip_prefix(ACCOUNT)?)
}

/// The account the page's fragment names, read once; a fragment that
/// begins `#account=` is removed from the address as a code's is, so the
/// page loaded again starts at its list.
#[cfg(target_arch = "wasm32")]
pub fn take_account() -> Option<[u8; 16]> {
    let (_, hash) = location("hash")?;
    if !hash.starts_with(ACCOUNT) {
        return None;
    }
    replace("");
    account_in(&hash)
}

/// On the server there is no address to read.
#[cfg(not(target_arch = "wasm32"))]
pub fn take_account() -> Option<[u8; 16]> {
    None
}

/// Puts `fragment` (`#state=approved`, or nothing) in place of the page's
/// own, in the same entry of the history: a filter the page shows stays
/// when it is loaded again, and never reaches the site.
#[cfg(target_arch = "wasm32")]
pub fn replace(fragment: &str) {
    use wasm_bindgen::JsValue;
    let Some((location, _)) = location("hash") else {
        return;
    };
    let href = get(&location, "href")
        .and_then(|value| value.as_string())
        .unwrap_or_default();
    let global: JsValue = js_sys::global().into();
    if let Some(history) = get(&global, "history")
        && let Some(replace) = method(&history, "replaceState")
    {
        let state = get(&history, "state").unwrap_or(JsValue::NULL);
        let address = format!("{}{fragment}", without_fragment(&href));
        let _ = replace.call3(
            &history,
            &state,
            &JsValue::from_str(""),
            &JsValue::from_str(&address),
        );
    }
}

/// On the server there is no address.
#[cfg(not(target_arch = "wasm32"))]
pub fn replace(_fragment: &str) {}

/// Watches for a fragment that hands a code after the page started: the
/// link opened in a tab that shows this page already, which a browser
/// takes as a move within the page and runs no island for. The fragment
/// is removed as at the start (`take`), and the code fills nothing: the
/// page is left as it is, a step it shows once (a new vault's recovery
/// code) included, and the person types the code from the mail. Another
/// fragment is left alone.
#[cfg(target_arch = "wasm32")]
pub fn watch() {
    use wasm_bindgen::{JsValue, closure::Closure};
    let changed = Closure::<dyn Fn()>::new(|| {
        let _ = take();
    });
    let global: JsValue = js_sys::global().into();
    if let Some(add) = method(&global, "addEventListener") {
        let _ = add.call2(&global, &JsValue::from_str("hashchange"), changed.as_ref());
    }
    // the page's whole life: the listener is never taken off
    changed.forget();
}

/// On the server nothing changes.
#[cfg(not(target_arch = "wasm32"))]
pub fn watch() {}

#[cfg(test)]
mod tests {
    use super::*;
    use central_api::email::MailCode;
    use central_api::reset_code::ResetCode;

    const RESET: &str = "RESET-0123-4567-89AB-CDEF-GHJK-MNPQ-RSTV-WXYZ";
    const MAILED: &str = "000G-40R4";

    fn reset(hash: &str) -> Option<String> {
        code(hash, ResetCode::parse).map(|code| code.display().to_string())
    }

    fn mailed(hash: &str) -> Option<String> {
        code(hash, MailCode::parse).map(|code| code.display().to_string())
    }

    /// Each page takes its own kind of code, in the form it is printed in
    /// and in the forms a person types (the parsers' own rules), and
    /// fills it in as it is printed.
    #[test]
    fn a_page_takes_a_code_of_its_own_kind() {
        assert_eq!(reset(&format!("#code={RESET}")).as_deref(), Some(RESET));
        assert_eq!(
            reset("#code=reset0123456789abcdefghjkmnpqrstvwxyz").as_deref(),
            Some(RESET)
        );
        assert_eq!(mailed(&format!("#code={MAILED}")).as_deref(), Some(MAILED));
        assert_eq!(mailed("#code=000g40r4").as_deref(), Some(MAILED));
        // the other page's code is none
        assert_eq!(reset(&format!("#code={MAILED}")), None);
        assert_eq!(mailed(&format!("#code={RESET}")), None);
    }

    /// Only `#code=` and a code's characters: no other name, nothing
    /// encoded, no second field, nothing longer than a code.
    #[test]
    fn anything_else_hands_no_code() {
        let long = format!("#code={RESET}-{}", "0".repeat(20));
        for hash in [
            "",
            "#",
            "#code=",
            "#code",
            "code=000G-40R4",
            "#Code=000G-40R4",
            "#CODE=000G-40R4",
            "# code=000G-40R4",
            "#x&code=000G-40R4",
            "#code=000G-40R4&code=000G-40R4",
            "#code=000G%2D40R4",
            "#code=000G 40R4",
            "#code=000G-40R4 ",
            "#code=000G-40R4/",
            "#code=000G-40R4?",
            "#code=000G-40RU",
            "#code=\u{ff10}00G-40R4",
            long.as_str(),
        ] {
            assert_eq!(mailed(hash), None, "{hash}");
            assert_eq!(reset(hash), None, "{hash}");
        }
        assert!(long.len() > PREFIX.len() + LONGEST);
    }

    /// The reset page opens the account its fragment names by its id in
    /// lowercase hex, and nothing else; the account is never a code.
    #[test]
    fn the_reset_page_opens_the_account_its_fragment_names() {
        let hex = "019a2b3c4d5e708090a0b0c0d0e0f0ff";
        let id = crate::data::id_from_hex(hex).expect("an id");
        assert_eq!(account_in(&format!("#account={hex}")), Some(id));
        for hash in [
            String::new(),
            "#account=".to_owned(),
            format!("#account={}", hex.to_uppercase()),
            format!("#account={hex}&x"),
            format!("#account= {hex}"),
            format!("#code={hex}"),
            format!("account={hex}"),
        ] {
            assert_eq!(account_in(&hash), None, "{hash}");
        }
        assert!(!hands(&format!("#account={hex}")));
    }

    /// A fragment that hands a code goes from the address bar, a good one
    /// or not; a place on the page stays.
    #[test]
    fn a_handed_code_is_removed_and_a_place_on_the_page_stays() {
        for hash in [
            "#code=000G-40R4",
            "#code=",
            "#code=garbage",
            "#code=%3Cb%3E",
        ] {
            assert!(hands(hash), "{hash}");
        }
        for hash in [
            "",
            "#",
            "#main",
            "#reset-mail-title",
            "#Code=000G-40R4",
            "#xcode=",
        ] {
            assert!(!hands(hash), "{hash}");
        }
        assert_eq!(
            without_fragment("https://baylee.app/en/reset?x=1#code=000G-40R4"),
            "https://baylee.app/en/reset?x=1"
        );
        assert_eq!(
            without_fragment("http://127.0.0.1:28792/de/account/security#code=a#b"),
            "http://127.0.0.1:28792/de/account/security"
        );
        assert_eq!(
            without_fragment("https://baylee.app/en/reset"),
            "https://baylee.app/en/reset"
        );
        // the server has no address to take one from
        assert_eq!(take(), "");
    }
}
