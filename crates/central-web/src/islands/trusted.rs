//! Script addresses under Trusted Types. The site's policy
//! (`require-trusted-types-for 'script'; trusted-types baylee`) lets a
//! Worker start only from a script address this policy vouched for, and the
//! policy vouches only for the site's own hashed scripts (`/assets/….js`).

use std::cell::RefCell;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};

thread_local! {
    /// The policy, made once: Trusted Types refuses a second of one name.
    static POLICY: RefCell<Option<JsValue>> = const { RefCell::new(None) };
}

/// A hashed script of this site: `/assets/<name>.js`, nothing else.
fn ours(url: &str) -> bool {
    url.strip_prefix("/assets/")
        .and_then(|rest| rest.strip_suffix(".js"))
        .is_some_and(|name| {
            !name.is_empty()
                && name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
        })
}

fn policy(window: &web_sys::Window) -> Option<JsValue> {
    if let Some(policy) = POLICY.with(|slot| slot.borrow().clone()) {
        return Some(policy);
    }
    let factory = js_sys::Reflect::get(window, &JsValue::from_str("trustedTypes")).ok()?;
    if factory.is_undefined() || factory.is_null() {
        return None;
    }
    let vouch = Closure::<dyn Fn(String) -> Result<JsValue, JsValue>>::new(|url: String| {
        if ours(&url) {
            Ok(JsValue::from_str(&url))
        } else {
            Err(JsValue::from_str("not a script of this site"))
        }
    });
    let rules = js_sys::Object::new();
    js_sys::Reflect::set(
        &rules,
        &JsValue::from_str("createScriptURL"),
        vouch.as_ref(),
    )
    .ok()?;
    // The policy lives as long as the page; so does its function.
    vouch.forget();
    let create: js_sys::Function =
        js_sys::Reflect::get(&factory, &JsValue::from_str("createPolicy"))
            .ok()?
            .dyn_into()
            .ok()?;
    let policy = create
        .call2(&factory, &JsValue::from_str("baylee"), &rules)
        .ok()?;
    POLICY.with(|slot| *slot.borrow_mut() = Some(policy.clone()));
    Some(policy)
}

/// A module Worker running the site's script at `url`, vouched for by the
/// policy where the browser has Trusted Types.
pub fn module_worker(url: &str) -> Option<web_sys::Worker> {
    if !ours(url) {
        return None;
    }
    let window = web_sys::window()?;
    let address = match policy(&window) {
        Some(policy) => {
            let create: js_sys::Function =
                js_sys::Reflect::get(&policy, &JsValue::from_str("createScriptURL"))
                    .ok()?
                    .dyn_into()
                    .ok()?;
            create.call1(&policy, &JsValue::from_str(url)).ok()?
        }
        None => JsValue::from_str(url),
    };
    let options = js_sys::Object::new();
    js_sys::Reflect::set(
        &options,
        &JsValue::from_str("type"),
        &JsValue::from_str("module"),
    )
    .ok()?;
    let constructor: js_sys::Function = js_sys::Reflect::get(&window, &JsValue::from_str("Worker"))
        .ok()?
        .dyn_into()
        .ok()?;
    js_sys::Reflect::construct(&constructor, &js_sys::Array::of2(&address, &options))
        .ok()?
        .dyn_into()
        .ok()
}
