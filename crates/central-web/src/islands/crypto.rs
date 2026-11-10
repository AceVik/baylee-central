//! The page's side of the browser crypto: a module Worker running
//! `crypto-worker.js` with `central-web-crypto`'s committed module, one per
//! flow, terminated when the flow drops it. The page sends passwords and
//! bytes and receives bytes and words; the master key and every OPAQUE state
//! stay in the Worker (`docs/web-accounts.md` §"Browser crypto").

use serde_json::Value;

use super::central::Failure;

/// The Worker of one flow.
#[cfg(target_arch = "wasm32")]
pub struct Crypto {
    worker: web_sys::Worker,
    next: std::cell::Cell<u32>,
    waiting: std::rc::Rc<std::cell::RefCell<std::collections::HashMap<u32, js_sys::Function>>>,
    _listen: wasm_bindgen::closure::Closure<dyn FnMut(web_sys::MessageEvent)>,
    _fail: wasm_bindgen::closure::Closure<dyn FnMut(web_sys::Event)>,
}

#[cfg(target_arch = "wasm32")]
impl Crypto {
    /// Starts the Worker at `url` (the site's hashed `crypto-worker.js`).
    pub fn start(url: &str) -> Option<Crypto> {
        use std::cell::RefCell;
        use std::collections::HashMap;
        use std::rc::Rc;

        use wasm_bindgen::closure::Closure;
        use wasm_bindgen::{JsCast, JsValue};

        let worker = super::trusted::module_worker(url)?;
        let waiting: Rc<RefCell<HashMap<u32, js_sys::Function>>> = Rc::default();
        let listen = {
            let waiting = Rc::clone(&waiting);
            Closure::<dyn FnMut(web_sys::MessageEvent)>::new(move |event: web_sys::MessageEvent| {
                let Some(text) = event.data().as_string() else {
                    return;
                };
                let Some(id) = serde_json::from_str::<Value>(&text)
                    .ok()
                    .and_then(|reply| reply.get("id").and_then(Value::as_u64))
                    .and_then(|id| u32::try_from(id).ok())
                else {
                    return;
                };
                if let Some(resolve) = waiting.borrow_mut().remove(&id) {
                    let _ = resolve.call1(&JsValue::NULL, &JsValue::from_str(&text));
                }
            })
        };
        let fail = {
            let waiting = Rc::clone(&waiting);
            Closure::<dyn FnMut(web_sys::Event)>::new(move |_event: web_sys::Event| {
                for (id, resolve) in waiting.borrow_mut().drain() {
                    let reply = serde_json::json!({"id": id, "error": "failed"}).to_string();
                    let _ = resolve.call1(&JsValue::NULL, &JsValue::from_str(&reply));
                }
            })
        };
        worker.set_onmessage(Some(listen.as_ref().unchecked_ref()));
        worker.set_onerror(Some(fail.as_ref().unchecked_ref()));
        Some(Crypto {
            worker,
            next: std::cell::Cell::new(1),
            waiting,
            _listen: listen,
            _fail: fail,
        })
    }

    /// Asks the Worker for the step `op` with `fields`; its answer.
    ///
    /// # Errors
    /// `Rejected` (a wrong password, an answer that does not verify),
    /// `Unavailable`, or `Malformed` for anything else.
    pub async fn ask(&self, op: &str, mut fields: Value) -> Result<Value, Failure> {
        use wasm_bindgen::JsValue;

        let id = self.next.get();
        self.next.set(id.wrapping_add(1));
        let object = fields.as_object_mut().ok_or(Failure::Malformed)?;
        object.insert("id".into(), Value::from(id));
        object.insert("op".into(), Value::from(op));
        let waiting = std::rc::Rc::clone(&self.waiting);
        let promise = js_sys::Promise::new(&mut |resolve, _reject| {
            waiting.borrow_mut().insert(id, resolve);
        });
        self.worker
            .post_message(&JsValue::from_str(&fields.to_string()))
            .map_err(|_| Failure::Unavailable)?;
        let reply = wasm_bindgen_futures::JsFuture::from(promise)
            .await
            .map_err(|_| Failure::Unavailable)?;
        let reply: Value = serde_json::from_str(&reply.as_string().unwrap_or_default())
            .map_err(|_| Failure::Malformed)?;
        match (reply.get("ok"), reply.get("error").and_then(Value::as_str)) {
            (Some(ok), None) => Ok(ok.clone()),
            (_, Some("rejected")) => Err(Failure::Rejected),
            (_, Some("unavailable")) => Err(Failure::Unavailable),
            _ => Err(Failure::Malformed),
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl Crypto {
    /// Ends the Worker now, discarding its memory.
    pub fn close(self) {
        drop(self);
    }

    /// Ends the Worker now for a holder that shares it; every step still
    /// waiting fails.
    pub fn end(&self) {
        use wasm_bindgen::JsValue;

        self.worker.terminate();
        for (id, resolve) in self.waiting.borrow_mut().drain() {
            let reply = serde_json::json!({"id": id, "error": "failed"}).to_string();
            let _ = resolve.call1(&JsValue::NULL, &JsValue::from_str(&reply));
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl Drop for Crypto {
    fn drop(&mut self) {
        self.worker.terminate();
    }
}

/// On the server there is no Worker: islands only run their flows in the browser.
#[cfg(not(target_arch = "wasm32"))]
pub struct Crypto;

#[cfg(not(target_arch = "wasm32"))]
impl Crypto {
    /// Never on the server.
    pub fn start(_url: &str) -> Option<Crypto> {
        None
    }

    /// Never on the server.
    pub fn ask(&self, _op: &str, _fields: Value) -> std::future::Ready<Result<Value, Failure>> {
        let &Crypto = self;
        std::future::ready(Err(Failure::Unavailable))
    }

    /// Never on the server.
    pub fn close(self) {
        let Crypto = self;
    }

    /// Never on the server.
    pub fn end(&self) {
        let &Crypto = self;
    }
}

/// The bytes of a field of a Worker's answer, exactly `len` long when given.
pub fn field_bytes(value: &Value, name: &str, len: Option<usize>) -> Result<Vec<u8>, Failure> {
    let bytes: Vec<u8> = value
        .get(name)
        .and_then(Value::as_array)
        .ok_or(Failure::Malformed)?
        .iter()
        .map(|b| b.as_u64().and_then(|b| u8::try_from(b).ok()))
        .collect::<Option<_>>()
        .ok_or(Failure::Malformed)?;
    match len {
        Some(len) if bytes.len() != len => Err(Failure::Malformed),
        _ => Ok(bytes),
    }
}
