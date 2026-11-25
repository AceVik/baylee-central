//! Live `/api/v1/ws` WebSocket push channel and peer availability state.
//!
//! Replaces periodic inbox/contacts/groups polling loops: initial snapshots
//! are fetched once on startup, after which Central pushes `ready`, `presence`,
//! `chat`, `social` (`thread`, `friend_request`, `friend_accept`,
//! `contact_changed`), `table`, and `revoked` events over the same-origin
//! session socket (`__Host-central` cookie).

use std::collections::BTreeMap;

use leptos::prelude::*;

use super::Ctl;
use crate::words::{self, Lang};

/// Live availability of a peer reported over `/api/v1/ws`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PeerState {
    /// Not currently connected to Central.
    #[default]
    Offline,
    /// Active browser/game session.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    Online,
    /// Tab hidden or idle.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    Away,
    /// Actively in a match.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    Playing,
    /// Seated at a table.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    AtTable,
}

impl PeerState {
    /// Machine-readable attribute for `.presence-dot[data-state]`.
    #[must_use]
    pub const fn attr(self) -> &'static str {
        match self {
            Self::Offline => "offline",
            Self::Online => "online",
            Self::Away => "away",
            Self::Playing => "playing",
            Self::AtTable => "table",
        }
    }

    /// Localized human-readable label for screen readers and status rows.
    #[must_use]
    pub fn label(self, lang: Lang) -> &'static str {
        match self {
            Self::Offline => words::chat_presence_offline(lang),
            Self::Online => words::chat_presence_online(lang),
            Self::Away => words::chat_presence_away(lang),
            Self::Playing => words::chat_presence_playing(lang),
            Self::AtTable => words::chat_presence_table(lang),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct PresenceState {
    own: u32,
    me: PeerState,
    peers: BTreeMap<u32, PeerState>,
}

/// Reactive map of live peer availability shared via Leptos context.
#[derive(Clone, Copy)]
pub struct Presence {
    inner: RwSignal<PresenceState>,
}

impl Presence {
    /// Provides an empty [`Presence`] store in the current reactive scope.
    pub fn provide() -> Self {
        let store = Self {
            inner: RwSignal::new(PresenceState::default()),
        };
        provide_context(store);
        store
    }

    /// Sets the signed-in account's own tag for matching `me` presence.
    pub fn set_own(self, own: u32) {
        self.inner.update(|s| s.own = own);
    }

    /// Reads the live presence state for `tag` (`Offline` if absent).
    #[must_use]
    pub fn get(self, tag: u32) -> PeerState {
        self.inner.with(|s| {
            if s.own != 0 && tag == s.own {
                s.me
            } else {
                s.peers.get(&tag).copied().unwrap_or(PeerState::Offline)
            }
        })
    }

    #[cfg(target_arch = "wasm32")]
    fn set_offline(self) {
        self.inner.update(|s| {
            s.me = PeerState::Offline;
            if !s.peers.is_empty() {
                s.peers.clear();
            }
        });
    }

    #[cfg(target_arch = "wasm32")]
    fn apply_presence(self, online: &[String], states: &str, me: Option<&str>) {
        use super::model::tag_of;

        let mut next = BTreeMap::new();
        let mut chars = states.chars();
        for hex in online {
            let ch = chars.next().unwrap_or('o');
            if let Some(tag) = tag_of(hex) {
                next.insert(tag, parse_state_char(ch));
            }
        }
        let next_me = match me {
            Some("afk") => PeerState::Away,
            Some("offline") => PeerState::Offline,
            _ => PeerState::Online,
        };
        self.inner.update(|s| {
            if s.peers != next {
                s.peers = next;
            }
            if s.me != next_me {
                s.me = next_me;
            }
        });
    }
}

/// Reads the live presence for `tag` from context, returning `None` when
/// offline or outside the messenger context.
#[must_use]
pub fn presence_of(tag: u32) -> Option<PeerState> {
    use_context::<Presence>()
        .map(|p| p.get(tag))
        .filter(|&state| state != PeerState::Offline)
}

#[cfg(target_arch = "wasm32")]
const fn parse_state_char(ch: char) -> PeerState {
    match ch {
        'a' => PeerState::Away,
        'g' | 'G' => PeerState::Playing,
        't' | 'T' => PeerState::AtTable,
        _ => PeerState::Online,
    }
}

/// Starts the persistent `/api/v1/ws` client in the browser and ties its
/// lifetime to the current Leptos reactive owner.
#[cfg(target_arch = "wasm32")]
pub fn start(ctl: Ctl, presence: Presence) {
    browser::start(ctl, presence);
}

/// Server stub for WebSocket startup.
#[cfg(not(target_arch = "wasm32"))]
pub fn start(ctl: Ctl, presence: Presence) {
    let _ = (ctl, presence);
}

#[cfg(target_arch = "wasm32")]
mod browser {
    use std::cell::RefCell;
    use std::rc::{Rc, Weak};
    use std::time::Duration;

    use js_sys::{Array, Function, Reflect};
    use leptos::prelude::*;
    use leptos::web_sys;
    use serde_json::Value;
    use wasm_bindgen::{JsCast, JsValue, closure::Closure};

    use super::{Ctl, Presence};

    const INITIAL_BACKOFF_MS: u32 = 1_000;
    const MAX_BACKOFF_MS: u32 = 30_000;
    const IDLE_AWAY_MS: u64 = 300_000;

    type AnyClosure = Closure<dyn FnMut(JsValue)>;

    struct SocketHandles {
        ws: JsValue,
        _onopen: AnyClosure,
        _onmessage: AnyClosure,
        _onerror: AnyClosure,
        _onclose: AnyClosure,
    }

    struct ClientState {
        ctl: Ctl,
        presence: Presence,
        owner: Owner,
        closed: bool,
        had_disconnected: bool,
        attempt: u32,
        generation: u64,
        sent_away: Option<bool>,
        idle_away: bool,
        socket: Option<SocketHandles>,
        reconnect_timer: Option<TimeoutHandle>,
        idle_timer: Option<TimeoutHandle>,
        listeners: Vec<WindowListenerHandle>,
    }

    pub(super) fn start(ctl: Ctl, presence: Presence) {
        let state = Rc::new(RefCell::new(ClientState {
            ctl,
            presence,
            owner: Owner::new_root(None),
            closed: false,
            had_disconnected: false,
            attempt: 0,
            generation: 0,
            sent_away: None,
            idle_away: false,
            socket: None,
            reconnect_timer: None,
            idle_timer: None,
            listeners: Vec::new(),
        }));

        bind_listeners(&state);
        reset_idle_timer(&state);
        connect(&state);

        let teardown = StoredValue::new_local(state);
        on_cleanup(move || {
            let _ = teardown.try_with_value(shutdown);
        });
    }

    fn ws_url() -> Option<String> {
        let loc = web_sys::window()?.location();
        let proto = loc.protocol().ok()?;
        let host = loc.host().ok()?;
        let ws_proto = if proto == "http:" { "ws:" } else { "wss:" };
        Some(format!("{ws_proto}//{host}/api/v1/ws"))
    }

    fn backoff_ms(attempt: u32) -> u64 {
        let exp = attempt.min(5);
        let base = INITIAL_BACKOFF_MS
            .saturating_mul(1_u32 << exp)
            .min(MAX_BACKOFF_MS);
        let jitter = (attempt.saturating_mul(313)) % 500;
        u64::from(base.saturating_add(jitter).min(MAX_BACKOFF_MS))
    }

    fn clear_reconnect(st: &mut ClientState) {
        if let Some(h) = st.reconnect_timer.take() {
            h.clear();
        }
    }

    fn clear_idle(st: &mut ClientState) {
        if let Some(h) = st.idle_timer.take() {
            h.clear();
        }
    }

    fn close_raw_socket(handles: Option<SocketHandles>) {
        let Some(h) = handles else {
            return;
        };
        for prop in ["onopen", "onmessage", "onerror", "onclose"] {
            let _ = Reflect::set(&h.ws, &JsValue::from_str(prop), &JsValue::NULL);
        }
        if let Some(close_fn) = Reflect::get(&h.ws, &JsValue::from_str("close"))
            .ok()
            .and_then(|v| v.dyn_into::<Function>().ok())
        {
            let _ = close_fn.call0(&h.ws);
        }
    }

    fn shutdown(state: &Rc<RefCell<ClientState>>) {
        let (handles, presence) = {
            let mut st = state.borrow_mut();
            st.closed = true;
            clear_reconnect(&mut st);
            clear_idle(&mut st);
            for listener in st.listeners.drain(..) {
                listener.remove();
            }
            (st.socket.take(), st.presence)
        };
        presence.set_offline();
        close_raw_socket(handles);
    }

    fn effective_away(st: &ClientState) -> bool {
        document().hidden() || st.idle_away
    }

    fn push_away(state: &Rc<RefCell<ClientState>>, force: bool) {
        let payload = {
            let mut st = state.borrow_mut();
            if st.closed {
                return;
            }
            let ws = {
                let Some(handles) = &st.socket else {
                    return;
                };
                handles.ws.clone()
            };
            let ready = Reflect::get(&ws, &JsValue::from_str("readyState"))
                .ok()
                .and_then(|v| v.as_f64())
                .is_some_and(|n| (n - 1.0).abs() < 0.1);
            if !ready {
                return;
            }
            let away = effective_away(&st);
            if !force && st.sent_away == Some(away) {
                return;
            }
            st.sent_away = Some(away);
            let text = serde_json::json!({ "event": "presence", "away": away }).to_string();
            Some((ws, text))
        };
        if let Some((ws, text)) = payload
            && let Some(send_fn) = Reflect::get(&ws, &JsValue::from_str("send"))
                .ok()
                .and_then(|v| v.dyn_into::<Function>().ok())
        {
            let _ = send_fn.call1(&ws, &JsValue::from_str(&text));
        }
    }

    fn reset_idle_timer(state: &Rc<RefCell<ClientState>>) {
        let (owner, became_active) = {
            let mut st = state.borrow_mut();
            if st.closed {
                return;
            }
            clear_idle(&mut st);
            let active = st.idle_away;
            st.idle_away = false;
            (st.owner.clone(), active)
        };
        if became_active {
            push_away(state, false);
        }
        let weak: Weak<RefCell<ClientState>> = Rc::downgrade(state);
        let timer = owner.with(|| {
            set_timeout(
                move || {
                    if let Some(st_rc) = weak.upgrade() {
                        {
                            let mut st = st_rc.borrow_mut();
                            if st.closed {
                                return;
                            }
                            st.idle_away = true;
                        }
                        push_away(&st_rc, false);
                    }
                },
                Duration::from_millis(IDLE_AWAY_MS),
            )
        });
        if let Ok(handle) = timer {
            state.borrow_mut().idle_timer = Some(handle);
        }
    }

    fn bind_listeners(state: &Rc<RefCell<ClientState>>) {
        let owner = state.borrow().owner.clone();
        let weak_vis = Rc::downgrade(state);
        let vis = owner.with(|| {
            window_event_listener_untyped("visibilitychange", move |_| {
                if let Some(st_rc) = weak_vis.upgrade() {
                    let hidden = document().hidden();
                    let need_reconnect = {
                        let mut st = st_rc.borrow_mut();
                        if !hidden {
                            st.idle_away = false;
                        }
                        !hidden && !st.closed && st.socket.is_none()
                    };
                    if need_reconnect {
                        state_reconnect_now(&st_rc);
                    } else {
                        push_away(&st_rc, false);
                    }
                    if !hidden {
                        reset_idle_timer(&st_rc);
                    }
                }
            })
        });

        let weak_online = Rc::downgrade(state);
        let online = owner.with(|| {
            window_event_listener_untyped("online", move |_| {
                if let Some(st_rc) = weak_online.upgrade() {
                    let need_reconnect = {
                        let st = st_rc.borrow();
                        !st.closed && st.socket.is_none()
                    };
                    if need_reconnect {
                        state_reconnect_now(&st_rc);
                    }
                }
            })
        });

        let weak_ptr = Rc::downgrade(state);
        let ptr = owner.with(|| {
            window_event_listener_untyped("pointerdown", move |_| {
                if let Some(st_rc) = weak_ptr.upgrade() {
                    reset_idle_timer(&st_rc);
                }
            })
        });

        let weak_key = Rc::downgrade(state);
        let key = owner.with(|| {
            window_event_listener_untyped("keydown", move |_| {
                if let Some(st_rc) = weak_key.upgrade() {
                    reset_idle_timer(&st_rc);
                }
            })
        });

        state.borrow_mut().listeners = vec![vis, online, ptr, key];
    }

    fn state_reconnect_now(state: &Rc<RefCell<ClientState>>) {
        {
            let mut st = state.borrow_mut();
            if st.closed {
                return;
            }
            st.attempt = 0;
            clear_reconnect(&mut st);
        }
        connect(state);
    }

    fn schedule_reconnect(state: &Rc<RefCell<ClientState>>, gen_id: u64) {
        let (owner, delay_ms, presence) = {
            let mut st = state.borrow_mut();
            if st.closed || st.generation != gen_id {
                return;
            }
            let prev = st.socket.take();
            close_raw_socket(prev);
            st.had_disconnected = true;
            st.sent_away = None;
            clear_reconnect(&mut st);
            let wait = backoff_ms(st.attempt);
            st.attempt = st.attempt.saturating_add(1);
            (st.owner.clone(), wait, st.presence)
        };
        presence.set_offline();

        let weak: Weak<RefCell<ClientState>> = Rc::downgrade(state);
        let timer = owner.with(|| {
            set_timeout(
                move || {
                    if let Some(st_rc) = weak.upgrade() {
                        connect(&st_rc);
                    }
                },
                Duration::from_millis(delay_ms),
            )
        });
        if let Ok(handle) = timer {
            state.borrow_mut().reconnect_timer = Some(handle);
        }
    }

    fn connect(state: &Rc<RefCell<ClientState>>) {
        let Some(url) = ws_url() else {
            return;
        };
        let Some(win) = web_sys::window() else {
            return;
        };
        let win_val: JsValue = win.into();
        let Ok(ctor) = Reflect::get(&win_val, &JsValue::from_str("WebSocket"))
            .and_then(wasm_bindgen::JsCast::dyn_into::<Function>)
        else {
            return;
        };
        let Ok(ws) = Reflect::construct(&ctor, &Array::of1(&JsValue::from_str(&url))) else {
            let gen_id = state.borrow().generation;
            schedule_reconnect(state, gen_id);
            return;
        };

        let gen_id = {
            let mut st = state.borrow_mut();
            if st.closed {
                drop(st);
                let _ = close_raw_ws_only(&ws);
                return;
            }
            clear_reconnect(&mut st);
            let prev = st.socket.take();
            close_raw_socket(prev);
            st.generation = st.generation.wrapping_add(1);
            st.sent_away = None;
            st.generation
        };

        let weak_open = Rc::downgrade(state);
        let onopen = Closure::wrap(Box::new(move |_ev: JsValue| {
            let Some(st_rc) = weak_open.upgrade() else {
                return;
            };
            {
                let mut st = st_rc.borrow_mut();
                if st.closed || st.generation != gen_id {
                    return;
                }
                st.attempt = 0;
            }
            push_away(&st_rc, true);
        }) as Box<dyn FnMut(JsValue)>);

        let weak_msg = Rc::downgrade(state);
        let onmessage = Closure::wrap(Box::new(move |ev: JsValue| {
            let Some(st_rc) = weak_msg.upgrade() else {
                return;
            };
            if st_rc.borrow().generation != gen_id || st_rc.borrow().closed {
                return;
            }
            if let Ok(data) = Reflect::get(&ev, &JsValue::from_str("data"))
                && let Some(text) = data.as_string()
            {
                handle_frame(&st_rc, &text);
            }
        }) as Box<dyn FnMut(JsValue)>);

        let weak_err = Rc::downgrade(state);
        let onerror = Closure::wrap(Box::new(move |_ev: JsValue| {
            if let Some(st_rc) = weak_err.upgrade() {
                schedule_reconnect(&st_rc, gen_id);
            }
        }) as Box<dyn FnMut(JsValue)>);

        let weak_close = Rc::downgrade(state);
        let onclose = Closure::wrap(Box::new(move |_ev: JsValue| {
            if let Some(st_rc) = weak_close.upgrade() {
                schedule_reconnect(&st_rc, gen_id);
            }
        }) as Box<dyn FnMut(JsValue)>);

        let _ = Reflect::set(&ws, &JsValue::from_str("onopen"), onopen.as_ref());
        let _ = Reflect::set(&ws, &JsValue::from_str("onmessage"), onmessage.as_ref());
        let _ = Reflect::set(&ws, &JsValue::from_str("onerror"), onerror.as_ref());
        let _ = Reflect::set(&ws, &JsValue::from_str("onclose"), onclose.as_ref());

        state.borrow_mut().socket = Some(SocketHandles {
            ws,
            _onopen: onopen,
            _onmessage: onmessage,
            _onerror: onerror,
            _onclose: onclose,
        });
    }

    fn close_raw_ws_only(ws: &JsValue) -> Result<JsValue, JsValue> {
        if let Some(close_fn) = Reflect::get(ws, &JsValue::from_str("close"))
            .ok()
            .and_then(|v| v.dyn_into::<Function>().ok())
        {
            close_fn.call0(ws)
        } else {
            Ok(JsValue::UNDEFINED)
        }
    }

    fn handle_frame(state: &Rc<RefCell<ClientState>>, raw: &str) {
        let Ok(val) = serde_json::from_str::<Value>(raw) else {
            return;
        };
        let Some(event) = val.get("event").and_then(Value::as_str) else {
            return;
        };
        match event {
            "ready" => {
                let (ctl, resync) = {
                    let mut st = state.borrow_mut();
                    st.attempt = 0;
                    let resync = st.had_disconnected;
                    st.had_disconnected = false;
                    (st.ctl, resync)
                };
                if resync {
                    ctl.poll();
                    ctl.reload_contacts();
                    ctl.reload_groups();
                    ctl.bump_invites();
                }
            }
            "presence" => {
                let online: Vec<String> = val
                    .get("online")
                    .and_then(Value::as_array)
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(str::to_owned))
                            .collect()
                    })
                    .unwrap_or_default();
                let states = val.get("states").and_then(Value::as_str).unwrap_or("");
                let me = val.get("me").and_then(Value::as_str);
                let presence = state.borrow().presence;
                presence.apply_presence(&online, states, me);
            }
            "chat" => {
                let ctl = state.borrow().ctl;
                ctl.poll();
            }
            "social" => {
                let ctl = state.borrow().ctl;
                let kind = val.get("kind").and_then(Value::as_str);
                match kind {
                    Some("thread") => ctl.reload_groups(),
                    Some("friend_request") => ctl.reload_contacts(),
                    Some("friend_accept") => {
                        ctl.reload_contacts();
                        ctl.bump_invites();
                    }
                    Some("contact_changed") => {
                        ctl.reload_contacts();
                        ctl.reload_groups();
                    }
                    _ => {
                        ctl.reload_contacts();
                        ctl.reload_groups();
                        ctl.bump_invites();
                    }
                }
            }
            "revoked" => {
                let ctl = state.borrow().ctl;
                shutdown(state);
                ctl.session_revoked();
            }
            // Forward-compatible handling for `table` and future push events.
            _ => {}
        }
    }
}
