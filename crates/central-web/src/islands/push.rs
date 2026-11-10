//! Web Push on this device (`docs/notifications.md`), unchanged from the
//! previous site: the categories are the account's (Central), the opt-in is
//! this browser's. Enabling asks the browser's permission inside the click,
//! drops any earlier subscription (another account's on a shared device),
//! subscribes with Central's public key, and tells the service worker the
//! random installation nonce it must see in every push (`PUSH_STATE`, its
//! `IndexedDB` "baylee-push"); the nonce is also kept in localStorage
//! `baylee-push-installation-v1`. Disabling undoes it here first, so it
//! holds without a network.
//!
//! The browser's push interfaces are reached through `js_sys::Reflect`, not
//! through more `web-sys` features.

use std::fmt::Write as _;

use central_api::notifications::{Category, NotificationRequest, NotificationState, Subscription};
use central_api::wire::{Request, Response};

use super::central::{Failure, call};
use super::session;
use crate::words::Lang;

/// localStorage: this installation's nonce while it is enabled.
const INSTALLATION: &str = "baylee-push-installation-v1";

/// The categories a person sees: the two administrators' only for them.
#[must_use]
pub fn categories(admin: bool) -> Vec<Category> {
    let mut all = vec![Category::Chat, Category::Contacts, Category::Updates];
    if admin {
        all.extend([Category::Feedback, Category::Gateways]);
    }
    all
}

/// The bit of a category in Central's mask.
#[must_use]
pub fn bit(category: Category) -> u8 {
    u8::try_from(category.bit()).unwrap_or(0)
}

fn nonce_ok(nonce: &str) -> bool {
    nonce.len() == 32
        && nonce
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// The installation nonce, when this browser is enabled.
#[must_use]
pub fn installation() -> Option<String> {
    session::read(INSTALLATION).filter(|n| nonce_ok(n))
}

fn remember(nonce: Option<&str>) {
    match nonce {
        Some(nonce) => session::write(INSTALLATION, nonce),
        None => session::remove(INSTALLATION),
    }
}

/// Central's answer, checked: a mask of known bits and a VAPID key of the
/// right shape.
fn valid(state: &NotificationState) -> bool {
    state.enabled <= 31
        && state.public_key.as_deref().is_none_or(|key| {
            key.len() == 87
                && key.starts_with('B')
                && key
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        })
}

async fn central(request: NotificationRequest) -> Result<NotificationState, Failure> {
    match call(&Request::Notifications { request }).await? {
        Response::Notifications { state } if valid(&state) => Ok(state),
        _ => Err(Failure::Malformed),
    }
}

/// What this device and Central say now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Now {
    /// Central's mask of enabled categories.
    pub enabled: u8,
    /// Central's public key; none when push is not configured there.
    pub public_key: Option<String>,
    /// This browser is enabled: permission, subscription and nonce agree.
    pub active: bool,
}

/// Reads the state, and brings the service worker in line with it.
///
/// # Errors
/// Central's refusal.
pub async fn load() -> Result<Now, Failure> {
    let endpoint = if browser::supported() {
        browser::endpoint().await
    } else {
        None
    };
    let state = central(NotificationRequest::Read { endpoint }).await?;
    let active = browser::supported()
        && browser::permission() == "granted"
        && state.subscribed
        && installation().is_some();
    if !active && installation().is_some() {
        remember(None);
    }
    if browser::supported() {
        let nonce = installation();
        let _ = browser::tell_worker(
            nonce.as_deref().filter(|_| active),
            if active { state.enabled } else { 0 },
        )
        .await;
    }
    Ok(Now {
        enabled: state.enabled,
        public_key: state.public_key,
        active,
    })
}

/// Switches one category for the account; the device follows at once.
///
/// # Errors
/// Central's refusal.
pub async fn set(now: &Now, category: Category, enabled: bool) -> Result<u8, Failure> {
    if !enabled && now.active {
        // opting out holds here even when Central cannot be reached
        let _ = browser::tell_worker(installation().as_deref(), now.enabled & !bit(category)).await;
    }
    let state = central(NotificationRequest::Set { category, enabled }).await?;
    if now.active {
        let _ = browser::tell_worker(installation().as_deref(), state.enabled).await;
    }
    Ok(state.enabled)
}

/// Why enabling did not work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The person or the browser said no.
    Denied,
    /// Anything else.
    Failed,
}

/// Enables this device, after `asked` (the permission prompt, started
/// inside the click) resolved.
///
/// # Errors
/// Why not.
pub async fn enable(asked: browser::Asked, lang: Lang, public_key: &str) -> Result<u8, Refusal> {
    if browser::answer(asked).await != "granted" {
        return Err(Refusal::Denied);
    }
    // a fresh subscription: a previous account's queued push never lands here
    if let Some(old) = browser::endpoint().await {
        let _ = central(NotificationRequest::Unsubscribe { endpoint: old }).await;
        browser::unsubscribe().await;
    }
    let (endpoint, p256dh, auth) = browser::subscribe(public_key)
        .await
        .ok_or(Refusal::Failed)?;
    let Some(bytes) = browser::random() else {
        browser::unsubscribe().await;
        return Err(Refusal::Failed);
    };
    let nonce = bytes.iter().fold(String::new(), |mut hex, b| {
        let _ = write!(hex, "{b:02x}");
        hex
    });
    let subscription = Subscription {
        endpoint,
        p256dh,
        auth,
        lang: lang.code().to_owned(),
        nonce: nonce.clone(),
    };
    if let Ok(state) = central(NotificationRequest::Subscribe { subscription }).await {
        remember(Some(&nonce));
        let _ = browser::tell_worker(Some(&nonce), state.enabled).await;
        Ok(state.enabled)
    } else {
        browser::unsubscribe().await;
        Err(Refusal::Failed)
    }
}

/// Disables this device: here first, then at Central and the browser.
/// Also before a sign-in, a registration and a sign-out, as before.
///
/// # Errors
/// When not every step was confirmed (this browser is off regardless).
pub async fn disable() -> Result<(), Failure> {
    remember(None);
    if !browser::supported() {
        return Ok(());
    }
    let told = browser::tell_worker(None, 0).await;
    let mut confirmed = told;
    if let Some(endpoint) = browser::endpoint().await {
        confirmed &= central(NotificationRequest::Unsubscribe { endpoint })
            .await
            .is_ok();
        browser::unsubscribe().await;
    }
    if confirmed {
        Ok(())
    } else {
        Err(Failure::Unavailable)
    }
}

/// The browser's side, through `Reflect`.
#[cfg(target_arch = "wasm32")]
pub mod browser {
    use js_sys::{Array, Function, Object, Promise, Reflect, Uint8Array};
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::JsFuture;

    use super::super::b64;

    fn get(target: &JsValue, key: &str) -> JsValue {
        Reflect::get(target, &JsValue::from_str(key)).unwrap_or(JsValue::UNDEFINED)
    }

    fn call(target: &JsValue, method: &str, args: &[JsValue]) -> Option<JsValue> {
        let function = get(target, method).dyn_into::<Function>().ok()?;
        function.apply(target, &args.iter().collect::<Array>()).ok()
    }

    async fn wait(value: Option<JsValue>) -> Option<JsValue> {
        let promise = value?.dyn_into::<Promise>().ok()?;
        JsFuture::from(promise).await.ok()
    }

    fn global() -> JsValue {
        js_sys::global().into()
    }

    /// Sixteen random bytes from the browser's generator.
    pub fn random() -> Option<[u8; 16]> {
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).ok().map(|()| bytes)
    }

    /// Web Push exists here: a service worker, the Push API, notifications
    /// and a secure context.
    pub fn supported() -> bool {
        let g = global();
        let navigator = get(&g, "navigator");
        !get(&navigator, "serviceWorker").is_undefined()
            && !get(&g, "PushManager").is_undefined()
            && !get(&g, "Notification").is_undefined()
            && get(&g, "isSecureContext").as_bool() == Some(true)
    }

    /// `Notification.permission`.
    pub fn permission() -> String {
        get(&get(&global(), "Notification"), "permission")
            .as_string()
            .unwrap_or_default()
    }

    /// The permission prompt, started inside the click.
    pub struct Asked(Option<JsValue>);

    /// Starts the permission prompt; call it in the click handler.
    pub fn ask() -> Asked {
        Asked(call(
            &get(&global(), "Notification"),
            "requestPermission",
            &[],
        ))
    }

    /// What the prompt answered.
    pub async fn answer(asked: Asked) -> String {
        wait(asked.0)
            .await
            .and_then(|a| a.as_string())
            .unwrap_or_default()
    }

    /// The service worker's registration, if there is one: asked without
    /// waiting, so a browser that refused to register it (a policy, an
    /// untrusted certificate) never holds up a sign-in.
    async fn registration() -> Option<JsValue> {
        let container = get(&get(&global(), "navigator"), "serviceWorker");
        wait(call(
            &container,
            "getRegistration",
            &[JsValue::from_str("/")],
        ))
        .await
        .filter(|r| !r.is_undefined() && !r.is_null())
    }

    async fn subscription() -> Option<JsValue> {
        let manager = get(&registration().await?, "pushManager");
        wait(call(&manager, "getSubscription", &[]))
            .await
            .filter(|s| !s.is_null() && !s.is_undefined())
    }

    /// This browser's subscription endpoint.
    pub async fn endpoint() -> Option<String> {
        get(&subscription().await?, "endpoint").as_string()
    }

    /// Ends this browser's subscription.
    pub async fn unsubscribe() {
        if let Some(sub) = subscription().await {
            let _ = wait(call(&sub, "unsubscribe", &[])).await;
        }
    }

    /// Subscribes with Central's public key: endpoint, p256dh, auth.
    pub async fn subscribe(public_key: &str) -> Option<(String, String, String)> {
        let key = b64::decode_url(public_key)?;
        let manager = get(&registration().await?, "pushManager");
        let options = Object::new();
        Reflect::set(&options, &"userVisibleOnly".into(), &JsValue::TRUE).ok()?;
        let key = Uint8Array::from(key.as_slice());
        Reflect::set(&options, &"applicationServerKey".into(), &key).ok()?;
        let sub = wait(call(&manager, "subscribe", &[options.into()])).await?;
        let json = call(&sub, "toJSON", &[])?;
        let keys = get(&json, "keys");
        let found = (
            get(&sub, "endpoint").as_string(),
            get(&keys, "p256dh").as_string(),
            get(&keys, "auth").as_string(),
        );
        if let (Some(endpoint), Some(p256dh), Some(auth)) = found {
            Some((endpoint, p256dh, auth))
        } else {
            let _ = wait(call(&sub, "unsubscribe", &[])).await;
            None
        }
    }

    /// Tells the service worker this installation's nonce and mask
    /// (`PUSH_STATE`); whether it confirmed within five seconds.
    pub async fn tell_worker(nonce: Option<&str>, enabled: u8) -> bool {
        let Some(registration) = registration().await else {
            return false;
        };
        let worker = get(&registration, "active");
        if worker.is_null() || worker.is_undefined() {
            return false;
        }
        let Ok(channel) = Reflect::construct(
            &get(&global(), "MessageChannel").unchecked_into::<Function>(),
            &Array::new(),
        ) else {
            return false;
        };
        let (mine, theirs) = (get(&channel, "port1"), get(&channel, "port2"));
        let message = Object::new();
        let _ = Reflect::set(&message, &"type".into(), &"PUSH_STATE".into());
        let _ = Reflect::set(
            &message,
            &"nonce".into(),
            &nonce.map_or(JsValue::NULL, JsValue::from_str),
        );
        let _ = Reflect::set(&message, &"enabled".into(), &JsValue::from(enabled));
        let reply = Promise::new(&mut |resolve, _reject| {
            let late = resolve.clone();
            let on = Closure::once_into_js(move |event: JsValue| {
                let ok = get(&get(&event, "data"), "ok").as_bool() == Some(true);
                let _ = resolve.call1(&JsValue::NULL, &JsValue::from_bool(ok));
            });
            let _ = Reflect::set(&mine, &"onmessage".into(), &on);
            let timer = Closure::once_into_js(move || {
                let _ = late.call1(&JsValue::NULL, &JsValue::FALSE);
            });
            let _ = call(&global(), "setTimeout", &[timer, JsValue::from(5000)]);
        });
        let transfer = Array::of1(&theirs);
        if call(&worker, "postMessage", &[message.into(), transfer.into()]).is_none() {
            let _ = call(&mine, "close", &[]);
            return false;
        }
        let ok = JsFuture::from(reply).await.ok().and_then(|v| v.as_bool()) == Some(true);
        let _ = call(&mine, "close", &[]);
        ok
    }
}

/// On the server there is no browser.
#[cfg(not(target_arch = "wasm32"))]
pub mod browser {
    /// Never on the server.
    pub fn random() -> Option<[u8; 16]> {
        None
    }

    /// Never on the server.
    pub fn supported() -> bool {
        false
    }

    /// Never on the server.
    pub fn permission() -> String {
        String::new()
    }

    /// Never on the server.
    #[derive(Clone, Copy)]
    pub struct Asked;

    /// Never on the server.
    pub fn ask() -> Asked {
        Asked
    }

    /// Never on the server.
    pub fn answer(asked: Asked) -> std::future::Ready<String> {
        let Asked = asked;
        std::future::ready(String::new())
    }

    /// Never on the server.
    pub fn endpoint() -> std::future::Ready<Option<String>> {
        std::future::ready(None)
    }

    /// Never on the server.
    pub fn unsubscribe() -> std::future::Ready<()> {
        std::future::ready(())
    }

    /// Never on the server.
    pub fn subscribe(_public_key: &str) -> std::future::Ready<Option<(String, String, String)>> {
        std::future::ready(None)
    }

    /// Never on the server.
    pub fn tell_worker(_nonce: Option<&str>, _enabled: u8) -> std::future::Ready<bool> {
        std::future::ready(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categories_and_bits() {
        assert_eq!(categories(false).len(), 3);
        assert_eq!(categories(true).len(), 5);
        assert_eq!(bit(Category::Chat), 1);
        assert_eq!(bit(Category::Gateways), 16);
        assert!(nonce_ok("0123456789abcdef0123456789abcdef"));
        assert!(!nonce_ok("0123456789ABCDEF0123456789abcdef"));
    }

    #[test]
    fn central_state_is_checked() {
        let key = format!("B{}", "a".repeat(86));
        let good = NotificationState {
            enabled: 7,
            public_key: Some(key),
            subscribed: false,
        };
        assert!(valid(&good));
        let bad = NotificationState {
            enabled: 32,
            public_key: None,
            subscribed: false,
        };
        assert!(!valid(&bad));
        let odd = NotificationState {
            enabled: 1,
            public_key: Some("A".repeat(87)),
            subscribed: false,
        };
        assert!(!valid(&odd));
    }
}
