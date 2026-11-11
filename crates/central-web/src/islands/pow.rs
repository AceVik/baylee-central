//! The proof of work the Impressum and registration ask for: for each part,
//! a counter whose SHA-256 over `nonce ":" part ":" counter` starts with
//! `difficulty` zero bits, found by `pow-worker.js` in a Worker so the page
//! stays responsive (`central_api::register` checks it the same way).

/// Solves the puzzle in the Worker at `worker_url` (the site's hashed
/// `pow-worker.js`), telling `progress` the share done, 0 to 100. Each
/// caller bounds the difficulty it accepts; the Worker takes 1 to 32.
#[cfg(target_arch = "wasm32")]
pub async fn solve(
    worker_url: &str,
    nonce: &str,
    parts: u32,
    difficulty: u32,
    progress: impl Fn(u32) + 'static,
) -> Option<Vec<u64>> {
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen::JsCast;
    use wasm_bindgen::JsValue;
    use wasm_bindgen::closure::Closure;

    #[derive(serde::Deserialize)]
    struct Message {
        progress: Option<u32>,
        counters: Option<Vec<u64>>,
    }

    let worker = super::trusted::module_worker(worker_url)?;
    let slot: Rc<RefCell<Option<(js_sys::Function, js_sys::Function)>>> = Rc::default();
    let promise = js_sys::Promise::new(&mut |resolve, reject| {
        *slot.borrow_mut() = Some((resolve, reject));
    });
    let settle = {
        let slot = Rc::clone(&slot);
        move |ok: bool, value: &JsValue| {
            if let Some((resolve, reject)) = slot.borrow_mut().take() {
                let _ = if ok {
                    resolve.call1(&JsValue::NULL, value)
                } else {
                    reject.call0(&JsValue::NULL)
                };
            }
        }
    };
    let on_message = {
        let settle = settle.clone();
        Closure::<dyn FnMut(web_sys::MessageEvent)>::new(move |event: web_sys::MessageEvent| {
            let text = event.data().as_string().unwrap_or_default();
            match serde_json::from_str::<Message>(&text) {
                Ok(Message {
                    counters: Some(_), ..
                }) => settle(true, &JsValue::from_str(&text)),
                Ok(Message {
                    progress: Some(percent),
                    ..
                }) => progress(percent.min(100)),
                _ => settle(false, &JsValue::NULL),
            }
        })
    };
    let on_error = Closure::<dyn FnMut(web_sys::Event)>::new(move |_event: web_sys::Event| {
        settle(false, &JsValue::NULL);
    });
    worker.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    worker.set_onerror(Some(on_error.as_ref().unchecked_ref()));
    let request = serde_json::json!({"nonce": nonce, "parts": parts, "difficulty": difficulty});
    let posted = worker.post_message(&JsValue::from_str(&request.to_string()));
    let outcome = if posted.is_ok() {
        wasm_bindgen_futures::JsFuture::from(promise).await.ok()
    } else {
        None
    };
    worker.terminate();
    drop((on_message, on_error));
    let text = outcome?.as_string()?;
    let counters = serde_json::from_str::<Message>(&text).ok()?.counters?;
    (counters.len() == usize::try_from(parts).ok()?).then_some(counters)
}

/// On the server nothing is solved.
#[cfg(not(target_arch = "wasm32"))]
pub fn solve(
    _worker_url: &str,
    _nonce: &str,
    _parts: u32,
    _difficulty: u32,
    _progress: impl Fn(u32) + 'static,
) -> std::future::Ready<Option<Vec<u64>>> {
    std::future::ready(None)
}

/// Waits `millis` milliseconds (the browser's timer).
#[cfg(target_arch = "wasm32")]
pub async fn wait(millis: u32) {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        let _ = web_sys::window().map(|window| {
            window.set_timeout_with_callback_and_timeout_and_arguments_0(
                &resolve,
                i32::try_from(millis).unwrap_or(i32::MAX),
            )
        });
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

/// On the server nothing waits.
#[cfg(not(target_arch = "wasm32"))]
pub fn wait(_millis: u32) -> std::future::Ready<()> {
    std::future::ready(())
}

/// Now, in milliseconds of the browser's clock.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn now_ms() -> u64 {
    format!("{:.0}", js_sys::Date::now().trunc())
        .parse()
        .unwrap_or_default()
}

/// On the server, no clock is read.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn now_ms() -> u64 {
    0
}
