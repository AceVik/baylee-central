//! Requests from the browser to this origin: same-origin, without the
//! cookie, without a referrer, never following a redirect, never cached.

/// An answer: its status and its body as text.
pub type Answer = (u16, String);

/// The browser could not send the request or read its answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unsent;

/// Sends a request; `body`, when given, is JSON. `Err` when the browser
/// could not send it or read the answer (no network, a redirect).
#[cfg(target_arch = "wasm32")]
pub async fn request(
    method: &str,
    url: &str,
    body: Option<&str>,
    headers: &[(&str, &str)],
) -> Result<Answer, Unsent> {
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{
        ReferrerPolicy, RequestCache, RequestCredentials, RequestInit, RequestRedirect, Response,
    };

    let init = RequestInit::new();
    init.set_method(method);
    init.set_credentials(RequestCredentials::Omit);
    init.set_redirect(RequestRedirect::Error);
    init.set_referrer_policy(ReferrerPolicy::NoReferrer);
    init.set_cache(RequestCache::NoStore);
    if let Some(body) = body {
        init.set_body(&JsValue::from_str(body));
    }
    let request = web_sys::Request::new_with_str_and_init(url, &init).map_err(|_| Unsent)?;
    if body.is_some() {
        request
            .headers()
            .set("Content-Type", "application/json")
            .map_err(|_| Unsent)?;
    }
    for (name, value) in headers {
        request.headers().set(name, value).map_err(|_| Unsent)?;
    }
    let window = web_sys::window().ok_or(Unsent)?;
    let response: Response = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|_| Unsent)?
        .dyn_into()
        .map_err(|_| Unsent)?;
    let text = JsFuture::from(response.text().map_err(|_| Unsent)?)
        .await
        .map_err(|_| Unsent)?;
    Ok((response.status(), text.as_string().unwrap_or_default()))
}

/// On the server nothing is ever sent: islands only send from the browser.
#[cfg(not(target_arch = "wasm32"))]
pub fn request(
    _method: &str,
    _url: &str,
    _body: Option<&str>,
    _headers: &[(&str, &str)],
) -> std::future::Ready<Result<Answer, Unsent>> {
    std::future::ready(Err(Unsent))
}
