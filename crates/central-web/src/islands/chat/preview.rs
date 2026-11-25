//! Shared hover, keyboard-focus, and mobile-fingerpress preview overlays
//! for `#Card` and `@User` mentions in the signed-in messenger, plus local
//! card-preview preference persistence (`baylee-chat-card-previews`).
//!
//! Card hover/tap previews are strictly read-only and faithfully render the
//! exact language, printing, and finish (Normal / Foil / Etched) defined by
//! the message author, falling back to Scryfall's default printing when
//! unspecified.

use std::time::Duration;

use central_api::profile::PublicProfile;
use central_api::ranked::{Rank, Track};
use central_api::social::{CardPrinting, CardSuggestion};
use leptos::html;
use leptos::prelude::*;
use leptos::web_sys;

use super::super::central;
use super::super::picture;
use super::super::ui::badges::{preview_badges_showcase, rank_title};
use super::super::ui::copy_handle::{handle_with_copy, player_path};
use super::super::ui::rank_avatar::{AvatarSize, rank_avatar, rank_of};
use super::cache;
use super::glyph::{self, glyph};
use super::mentions::{self, CardFinish, CardMention};
use super::model::hue;
use super::wire;
use crate::words::{self, Lang};

const PREVIEWS_STORAGE_KEY: &str = "baylee-chat-card-previews";
const HOVER_DWELL_FRESH_MS: u64 = 120;
const HOVER_DWELL_SWITCH_MS: u64 = 60;
const HOVER_LEAVE_GRACE_MS: u64 = 140;
const FOCUS_BLUR_GRACE_MS: u64 = 110;
const CLOSE_ANIM_MS: u64 = 115;

/// Reads whether direct Scryfall card image previews are enabled in chat
/// (default ON when signed in; stored locally under `baylee-chat-card-previews`).
#[must_use]
pub fn stored_card_previews_enabled() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|s| s.get_item(PREVIEWS_STORAGE_KEY).ok().flatten())
            .is_none_or(|v| v != "0")
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = PREVIEWS_STORAGE_KEY;
        true
    }
}

/// Persists the local card-preview preference in `localStorage`.
pub fn save_card_previews_enabled(enabled: bool) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
            let val = if enabled { "1" } else { "0" };
            let _ = storage.set_item(PREVIEWS_STORAGE_KEY, val);
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = enabled;
    }
}

/// Returns true on coarse/touch primary pointers where hover is replaced by fingerpress.
#[must_use]
pub fn is_coarse_pointer() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        use js_sys::{Function, Reflect};
        use wasm_bindgen::{JsCast, JsValue};
        let Some(w) = web_sys::window() else {
            return false;
        };
        let window: JsValue = w.into();
        Reflect::get(&window, &JsValue::from_str("matchMedia"))
            .ok()
            .and_then(|f| f.dyn_into::<Function>().ok())
            .and_then(|f| {
                f.call1(&window, &JsValue::from_str("(pointer: coarse)"))
                    .ok()
            })
            .and_then(|list| Reflect::get(&list, &JsValue::from_str("matches")).ok())
            .and_then(|m| m.as_bool())
            .unwrap_or(false)
    }
    #[cfg(not(target_arch = "wasm32"))]
    false
}

/// Which mention chip kind triggered the preview overlay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreviewTarget {
    /// `#Card` or `[Card]` mention.
    Card(CardMention),
    /// `@Name#tag` mention.
    User {
        /// Display name.
        name: String,
        /// Lowercase canonical hex tag without `#`.
        tag_hex: String,
    },
}

impl PreviewTarget {
    fn key(&self) -> String {
        match self {
            Self::Card(c) => format!(
                "c|{}|{}|{}|{}|{:?}",
                c.name.to_ascii_lowercase(),
                c.set.as_deref().unwrap_or(""),
                c.collector.as_deref().unwrap_or(""),
                c.lang.as_deref().unwrap_or(""),
                c.finish
            ),
            Self::User { name, tag_hex } => {
                format!("u|{}#{}", name.to_ascii_lowercase(), tag_hex)
            }
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
#[derive(Clone, Copy, Debug)]
struct AnchorRect {
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
}

#[derive(Clone, Debug)]
struct CardOverlay {
    mention: CardMention,
    loading: bool,
    card: Option<CardSuggestion>,
    printing: Option<CardPrinting>,
}

#[derive(Clone, Debug)]
struct UserOverlay {
    name: String,
    tag_hex: String,
    loading: bool,
    profile: Option<PublicProfile>,
}

#[derive(Clone, Debug)]
enum OverlayBody {
    Card(CardOverlay),
    User(UserOverlay),
}

#[derive(Clone, Debug)]
struct OverlayState {
    key: String,
    body: OverlayBody,
}

impl OverlayState {
    fn is_loading(&self) -> bool {
        match &self.body {
            OverlayBody::Card(c) => c.loading,
            OverlayBody::User(u) => u.loading,
        }
    }
}

fn pick_author_printing(
    printings: &[CardPrinting],
    mention: &CardMention,
    preferred_lang: &str,
) -> Option<CardPrinting> {
    let matches_lang = |p: &CardPrinting| p.lang.eq_ignore_ascii_case(preferred_lang);
    let matches_set = |p: &CardPrinting| {
        mention
            .set
            .as_deref()
            .is_none_or(|s| p.set_code.eq_ignore_ascii_case(s))
    };
    let matches_col = |p: &CardPrinting| {
        mention
            .collector
            .as_deref()
            .is_none_or(|c| p.collector_number.eq_ignore_ascii_case(c))
    };
    let finish_ok = |p: &CardPrinting| {
        mention.finish == CardFinish::Normal || (p.finishes & mention.finish.bit()) != 0
    };

    if mention.set.is_some() || mention.collector.is_some() {
        return printings
            .iter()
            .find(|p| matches_lang(p) && matches_set(p) && matches_col(p) && finish_ok(p))
            .or_else(|| {
                printings
                    .iter()
                    .find(|p| matches_set(p) && matches_col(p) && finish_ok(p))
            })
            .or_else(|| {
                printings
                    .iter()
                    .find(|p| matches_lang(p) && matches_set(p) && matches_col(p))
            })
            .or_else(|| printings.iter().find(|p| matches_set(p) && matches_col(p)))
            .or_else(|| printings.first())
            .cloned();
    }

    printings
        .iter()
        .find(|p| !p.ub && matches_lang(p) && finish_ok(p))
        .or_else(|| printings.iter().find(|p| !p.ub && matches_lang(p)))
        .or_else(|| printings.iter().find(|p| !p.ub && finish_ok(p)))
        .or_else(|| printings.iter().find(|p| !p.ub))
        .or_else(|| printings.first())
        .cloned()
}

fn finish_filter_str(finish: CardFinish) -> Option<&'static str> {
    match finish {
        CardFinish::Normal => None,
        CardFinish::Foil => Some("foil"),
        CardFinish::Etched => Some("etched"),
    }
}

fn measure_anchor(el: &web_sys::Element) -> AnchorRect {
    #[cfg(target_arch = "wasm32")]
    {
        use js_sys::{Function, Reflect};
        use wasm_bindgen::{JsCast, JsValue};
        let target: &JsValue = el.as_ref();
        let rect = Reflect::get(target, &JsValue::from_str("getBoundingClientRect"))
            .ok()
            .and_then(|f| f.dyn_into::<Function>().ok())
            .and_then(|f| f.call0(target).ok())
            .unwrap_or(JsValue::UNDEFINED);
        let num = |k: &str, fallback: f64| {
            Reflect::get(&rect, &JsValue::from_str(k))
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(fallback)
        };
        AnchorRect {
            left: num("left", 24.0),
            right: num("right", 140.0),
            top: num("top", 96.0),
            bottom: num("bottom", 120.0),
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = el;
        AnchorRect {
            left: 24.0,
            right: 140.0,
            top: 96.0,
            bottom: 120.0,
        }
    }
}

fn position_popover(node: &web_sys::Element, rect: AnchorRect) {
    #[cfg(target_arch = "wasm32")]
    {
        use js_sys::{Function, Reflect};
        use wasm_bindgen::{JsCast, JsValue};
        let Some(win) = web_sys::window() else {
            return;
        };
        let vw = win
            .inner_width()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(1024.0);
        let vh = win
            .inner_height()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(768.0);
        let target: &JsValue = node.as_ref();
        let dim = |k: &str, fallback: f64| {
            Reflect::get(target, &JsValue::from_str(k))
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(fallback)
        };
        let pop_w = dim("offsetWidth", 280.0).max(220.0);
        let pop_h = dim("offsetHeight", 380.0).max(160.0);
        let pad = 12.0;
        let gap = 10.0;

        let can_above = rect.top - gap - pop_h >= pad;
        let can_below = rect.bottom + gap + pop_h <= vh - pad;
        let can_right = rect.right + gap + pop_w <= vw - pad;
        let can_left = rect.left - gap - pop_w >= pad;
        let cx = ((rect.left + rect.right - pop_w) * 0.5).clamp(pad, (vw - pop_w - pad).max(pad));

        let (x, y) = if can_above {
            (cx, rect.top - pop_h - gap)
        } else if can_below {
            (cx, rect.bottom + gap)
        } else if can_right {
            let cy =
                ((rect.top + rect.bottom - pop_h) * 0.5).clamp(pad, (vh - pop_h - pad).max(pad));
            (rect.right + gap, cy)
        } else if can_left {
            let cy =
                ((rect.top + rect.bottom - pop_h) * 0.5).clamp(pad, (vh - pop_h - pad).max(pad));
            (rect.left - pop_w - gap, cy)
        } else {
            let space_above = rect.top - pad;
            let space_below = vh - pad - rect.bottom;
            let clamp_y = if space_above >= space_below {
                pad
            } else {
                (vh - pop_h - pad).max(pad)
            };
            (cx, clamp_y)
        };

        if let Ok(style) = Reflect::get(target, &JsValue::from_str("style"))
            && let Ok(set_prop) = Reflect::get(&style, &JsValue::from_str("setProperty"))
            && let Ok(func) = set_prop.dyn_into::<Function>()
        {
            let _ = func.call2(
                &style,
                &JsValue::from_str("--preview-x"),
                &JsValue::from_str(&format!("{x:.0}px")),
            );
            let _ = func.call2(
                &style,
                &JsValue::from_str("--preview-y"),
                &JsValue::from_str(&format!("{y:.0}px")),
            );
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (node, rect);
    }
}

fn target_from_anchor_event(el: Option<&web_sys::EventTarget>) -> Option<web_sys::Element> {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        el.and_then(|t| t.dyn_ref::<web_sys::Element>().cloned())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = el;
        None
    }
}

/// Reactive controller shared across an open conversation's message list for
/// `@User` and `#Card` previews + direct image opt-out toggle.
#[derive(Clone, Copy)]
pub struct PreviewCtl {
    lang: Lang,
    /// Whether Scryfall direct card image loading in signed-in chat is enabled.
    pub previews_enabled: RwSignal<bool>,
    overlay: RwSignal<Option<OverlayState>>,
    open: RwSignal<bool>,
    placed: RwSignal<bool>,
    anchor: RwSignal<Option<AnchorRect>>,
    pinned: RwSignal<bool>,
    dwell: StoredValue<Option<TimeoutHandle>>,
    grace: StoredValue<Option<TimeoutHandle>>,
    close_timer: StoredValue<Option<TimeoutHandle>>,
    request_gen: StoredValue<u64>,
    touch_down: StoredValue<bool>,
}

impl PreviewCtl {
    /// Creates a conversation-scoped preview controller initialized from local storage.
    pub fn new(lang: Lang) -> Self {
        let dwell = StoredValue::new(None::<TimeoutHandle>);
        let grace = StoredValue::new(None::<TimeoutHandle>);
        let close_timer = StoredValue::new(None::<TimeoutHandle>);
        on_cleanup(move || {
            if let Some(Some(h)) = dwell.try_get_value() {
                h.clear();
            }
            if let Some(Some(h)) = grace.try_get_value() {
                h.clear();
            }
            if let Some(Some(h)) = close_timer.try_get_value() {
                h.clear();
            }
        });
        Self {
            lang,
            previews_enabled: RwSignal::new(stored_card_previews_enabled()),
            overlay: RwSignal::new(None),
            open: RwSignal::new(false),
            placed: RwSignal::new(false),
            anchor: RwSignal::new(None),
            pinned: RwSignal::new(false),
            dwell,
            grace,
            close_timer,
            request_gen: StoredValue::new(0),
            touch_down: StoredValue::new(false),
        }
    }

    /// Toggles chat card preview image loading and persists the choice locally.
    pub fn toggle_previews(self) {
        self.previews_enabled.update(|on| {
            *on = !*on;
            save_card_previews_enabled(*on);
        });
    }

    fn clear_dwell(self) {
        if let Some(Some(h)) = self.dwell.try_get_value() {
            h.clear();
        }
        let _ = self.dwell.try_set_value(None);
    }

    fn clear_grace(self) {
        if let Some(Some(h)) = self.grace.try_get_value() {
            h.clear();
        }
        let _ = self.grace.try_set_value(None);
    }

    fn clear_close_timer(self) {
        if let Some(Some(h)) = self.close_timer.try_get_value() {
            h.clear();
        }
        let _ = self.close_timer.try_set_value(None);
    }

    fn close_gracefully(self) {
        self.clear_dwell();
        self.clear_grace();
        self.clear_close_timer();
        if self.overlay.with_untracked(Option::is_none) {
            self.pinned.set(false);
            self.open.set(false);
            self.placed.set(false);
            return;
        }
        self.request_gen.update_value(|g| *g = g.wrapping_add(1));
        self.open.set(false);
        let handle = set_timeout(
            move || {
                let _ = self.close_timer.try_set_value(None);
                if !self.open.get_untracked() {
                    self.overlay.set(None);
                    self.pinned.set(false);
                    self.placed.set(false);
                }
            },
            Duration::from_millis(CLOSE_ANIM_MS),
        )
        .ok();
        let _ = self.close_timer.try_set_value(handle);
    }

    /// Closes any active preview overlay smoothly and cancels pending timers.
    pub fn dismiss(self) {
        self.close_gracefully();
    }

    /// Dismisses unpinned desktop hover/focus overlays (for example on scroll).
    pub fn dismiss_hover(self) {
        if !self.pinned.get_untracked() {
            self.close_gracefully();
        }
    }

    /// Notes pointer modality before hover/click so touch presses never trigger hover dwell.
    pub fn on_pointer_down(self, ev: &leptos::ev::PointerEvent) {
        let touch = ev.pointer_type() == "touch";
        let _ = self.touch_down.try_set_value(touch);
    }

    /// Starts the desktop hover dwell timer (`120ms` fresh, `60ms` when switching chips).
    pub fn on_pointer_enter(self, ev: &leptos::ev::PointerEvent, target: PreviewTarget) {
        if ev.pointer_type() == "touch" || is_coarse_pointer() {
            return;
        }
        self.clear_grace();
        self.clear_close_timer();
        self.clear_dwell();
        let rect =
            target_from_anchor_event(ev.current_target().as_ref()).map(|el| measure_anchor(&el));
        let key = target.key();
        if self.open.get_untracked()
            && self
                .overlay
                .with_untracked(|o| o.as_ref().is_some_and(|s| s.key == key))
        {
            return;
        }
        if self.pinned.get_untracked() {
            return;
        }
        let delay = if self.open.get_untracked() {
            HOVER_DWELL_SWITCH_MS
        } else {
            HOVER_DWELL_FRESH_MS
        };
        let handle = set_timeout(
            move || {
                let _ = self.dwell.try_set_value(None);
                self.open(target, rect, false);
            },
            Duration::from_millis(delay),
        )
        .ok();
        let _ = self.dwell.try_set_value(handle);
    }

    /// Starts the desktop hover grace timer when leaving a chip.
    pub fn on_chip_leave(self, ev: &leptos::ev::PointerEvent) {
        if ev.pointer_type() == "touch" || is_coarse_pointer() {
            return;
        }
        self.clear_dwell();
        if self.pinned.get_untracked() {
            return;
        }
        self.schedule_grace(HOVER_LEAVE_GRACE_MS);
    }

    /// Opens the preview overlay immediately on keyboard focus unless already pinned elsewhere.
    pub fn on_focus(self, ev: &leptos::ev::FocusEvent, target: PreviewTarget) {
        if is_coarse_pointer() || self.touch_down.get_value() {
            return;
        }
        self.clear_grace();
        self.clear_close_timer();
        self.clear_dwell();
        if self.pinned.get_untracked() {
            return;
        }
        let rect =
            target_from_anchor_event(ev.current_target().as_ref()).map(|el| measure_anchor(&el));
        self.open(target, rect, false);
    }

    /// Schedules closing after keyboard blur (`110ms` grace so moving focus into the overlay works).
    pub fn on_blur(self) {
        if self.pinned.get_untracked() {
            return;
        }
        self.schedule_grace(FOCUS_BLUR_GRACE_MS);
    }

    /// Intercepts touch/fingerpress clicks to pin the preview overlay with explicit open actions.
    pub fn on_chip_click(self, ev: &leptos::ev::MouseEvent, target: PreviewTarget) {
        if is_coarse_pointer() || self.touch_down.get_value() {
            ev.prevent_default();
            let _ = self.touch_down.try_set_value(false);
            self.clear_dwell();
            self.clear_grace();
            self.clear_close_timer();
            let rect = target_from_anchor_event(ev.current_target().as_ref())
                .map(|el| measure_anchor(&el));
            let key = target.key();
            if self.pinned.get_untracked()
                && self.open.get_untracked()
                && self
                    .overlay
                    .with_untracked(|o| o.as_ref().is_some_and(|s| s.key == key))
            {
                self.dismiss();
            } else {
                self.open(target, rect, true);
            }
        }
    }

    fn schedule_grace(self, millis: u64) {
        self.clear_grace();
        let handle = set_timeout(
            move || {
                let _ = self.grace.try_set_value(None);
                if !self.pinned.get_untracked() {
                    self.close_gracefully();
                }
            },
            Duration::from_millis(millis),
        )
        .ok();
        let _ = self.grace.try_set_value(handle);
    }

    fn open(self, target: PreviewTarget, rect: Option<AnchorRect>, pin: bool) {
        let key = target.key();
        self.clear_close_timer();
        let was_open = self.overlay.with_untracked(Option::is_some) && self.open.get_untracked();
        if !was_open {
            self.placed.set(false);
        }
        if let Some(rect) = rect {
            self.anchor.set(Some(rect));
        }
        self.pinned.set(pin);
        self.open.set(true);
        let gen_id = self.request_gen.get_value().wrapping_add(1);
        self.request_gen.set_value(gen_id);

        match target {
            PreviewTarget::Card(mention) => self.open_card(key, mention, gen_id),
            PreviewTarget::User { name, tag_hex } => self.open_user(key, name, tag_hex, gen_id),
        }
    }

    fn maybe_warm_image(self, printing: Option<&CardPrinting>) {
        if !self.previews_enabled.get_untracked() {
            return;
        }
        if let Some(p) = printing
            && !p.ub
            && let Some(uuid) = p.scryfall_id.as_deref()
            && let Some(img_url) = mentions::scryfall_image_url(uuid)
        {
            cache::warm_card_image(&img_url);
        }
    }

    fn open_card(self, key: String, mention: CardMention, gen_id: u64) {
        let lookup_lang = mention.lang.as_deref().unwrap_or_else(|| self.lang.code());
        let query_finish = if mention.set.is_some() || mention.collector.is_some() {
            None
        } else {
            finish_filter_str(mention.finish)
        };
        let cache_key = cache::card_cache_key(
            &mention.name,
            lookup_lang,
            mention.set.as_deref(),
            mention.collector.as_deref(),
            query_finish,
        );
        if let Some((card, printings)) = cache::get_card_printings(&cache_key) {
            let pref_lang = mention
                .lang
                .as_deref()
                .or_else(|| card.as_ref().map(|c| c.matched_lang.as_str()))
                .unwrap_or(lookup_lang);
            let printing = pick_author_printing(&printings, &mention, pref_lang);
            self.maybe_warm_image(printing.as_ref());
            self.overlay.set(Some(OverlayState {
                key,
                body: OverlayBody::Card(CardOverlay {
                    mention,
                    loading: false,
                    card,
                    printing,
                }),
            }));
            return;
        }

        self.overlay.set(Some(OverlayState {
            key: key.clone(),
            body: OverlayBody::Card(CardOverlay {
                mention: mention.clone(),
                loading: true,
                card: None,
                printing: None,
            }),
        }));

        let lookup_lang_owned = lookup_lang.to_owned();
        leptos::task::spawn_local(async move {
            let mut res = wire::card_printings_lang(
                &mention.name,
                &lookup_lang_owned,
                mention.set.as_deref(),
                mention.collector.as_deref(),
                query_finish,
            )
            .await
            .unwrap_or_default();
            if res.1.is_empty() && mention.collector.is_some() && mention.set.is_some() {
                res = wire::card_printings_lang(
                    &mention.name,
                    &lookup_lang_owned,
                    mention.set.as_deref(),
                    None,
                    None,
                )
                .await
                .unwrap_or_default();
            }
            if res.1.is_empty()
                && (mention.set.is_some() || mention.collector.is_some() || query_finish.is_some())
            {
                res =
                    wire::card_printings_lang(&mention.name, &lookup_lang_owned, None, None, None)
                        .await
                        .unwrap_or_default();
            }
            cache::put_card_printings(cache_key, res.0.clone(), res.1.clone());

            let pref_lang = mention
                .lang
                .as_deref()
                .or_else(|| res.0.as_ref().map(|c| c.matched_lang.as_str()))
                .unwrap_or(&lookup_lang_owned);
            let printing = pick_author_printing(&res.1, &mention, pref_lang);
            self.maybe_warm_image(printing.as_ref());

            if self.request_gen.get_value() != gen_id {
                return;
            }
            self.overlay.set(Some(OverlayState {
                key,
                body: OverlayBody::Card(CardOverlay {
                    mention,
                    loading: false,
                    card: res.0,
                    printing,
                }),
            }));
        });
    }

    fn open_user(self, key: String, name: String, tag_hex: String, gen_id: u64) {
        let handle = format!("{name}#{tag_hex}");
        let cache_key = handle.to_ascii_lowercase();
        if let Some(hit) = cache::get_user_profile(&cache_key) {
            let profile = match hit {
                cache::CachedUserProfile::Found(profile) => Some(*profile),
                cache::CachedUserProfile::Missing => None,
            };
            self.overlay.set(Some(OverlayState {
                key,
                body: OverlayBody::User(UserOverlay {
                    name,
                    tag_hex,
                    loading: false,
                    profile,
                }),
            }));
            return;
        }

        self.overlay.set(Some(OverlayState {
            key: key.clone(),
            body: OverlayBody::User(UserOverlay {
                name: name.clone(),
                tag_hex: tag_hex.clone(),
                loading: true,
                profile: None,
            }),
        }));

        leptos::task::spawn_local(async move {
            let loaded = central::public_profile(&handle).await.ok();
            cache::put_user_profile(cache_key, loaded.clone());
            if self.request_gen.get_value() != gen_id {
                return;
            }
            self.overlay.set(Some(OverlayState {
                key,
                body: OverlayBody::User(UserOverlay {
                    name,
                    tag_hex,
                    loading: false,
                    profile: loaded,
                }),
            }));
        });
    }

    fn reposition_popover(self, pop_ref: NodeRef<html::Aside>) {
        let Some(rect) = self.anchor.get_untracked() else {
            return;
        };
        let _ = request_animation_frame(move || {
            if let Some(node) = pop_ref.get_untracked() {
                let el: &web_sys::Element = node.as_ref();
                position_popover(el, rect);
                self.placed.set(true);
            }
        });
    }

    /// Renders the shared persistent floating/bottom-sheet preview overlay when open.
    pub fn view(self) -> AnyView {
        let pop_ref: NodeRef<html::Aside> = NodeRef::new();

        Effect::new(move |_| {
            let snap = self
                .overlay
                .with(|o| o.as_ref().map(|s| (s.key.clone(), s.is_loading())));
            let rect = self.anchor.get();
            let _ = self.previews_enabled.get();
            let is_open = self.open.get();
            if is_open && snap.is_some() && rect.is_some() {
                self.reposition_popover(pop_ref);
            }
        });

        let has_overlay = Memo::new(move |_| self.overlay.with(Option::is_some));

        view! {
            <>
                {move || (has_overlay.get() && self.pinned.get()).then(|| {
                    let closing = !self.open.get();
                    view! {
                        <div
                            class="chat-preview-backdrop"
                            data-closing=closing.then_some("true")
                            aria-hidden="true"
                            on:click=move |_| self.dismiss()
                        ></div>
                    }
                })}
                {move || {
                    has_overlay.get().then(|| view! {
                        <aside
                            class="chat-preview-popover"
                            node_ref=pop_ref
                            role="dialog"
                            aria-modal=move || self.pinned.get().then_some("true")
                            tabindex="-1"
                            data-kind=move || self.overlay.with(|o| match o.as_ref().map(|s| &s.body) {
                                Some(OverlayBody::User(_)) => "user",
                                Some(OverlayBody::Card(_)) | None => "card",
                            })
                            data-pinned=move || self.pinned.get().then_some("true")
                            data-state=move || if self.open.get() { "open" } else { "closing" }
                            data-placed=move || self.placed.get().then_some("true")
                            on:pointerenter=move |_| {
                                self.clear_grace();
                                if self.open.get_untracked() {
                                    self.clear_close_timer();
                                }
                            }
                            on:pointerleave=move |ev: leptos::ev::PointerEvent| {
                                if ev.pointer_type() != "touch" && !is_coarse_pointer() && !self.pinned.get_untracked() {
                                    self.schedule_grace(HOVER_LEAVE_GRACE_MS);
                                }
                            }
                            on:focusin=move |_| self.clear_grace()
                            on:focusout=move |_| {
                                if !self.pinned.get_untracked() {
                                    self.schedule_grace(FOCUS_BLUR_GRACE_MS);
                                }
                            }
                            on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                                if ev.key() == "Escape" {
                                    ev.prevent_default();
                                    self.dismiss();
                                }
                            }
                        >
                            {move || {
                                let state = self.overlay.get()?;
                                let is_pinned = self.pinned.get();
                                Some(match state.body {
                                    OverlayBody::Card(card) => self.render_card(&card, is_pinned, pop_ref),
                                    OverlayBody::User(user) => self.render_user(user, is_pinned),
                                })
                            }}
                        </aside>
                    })
                }}
            </>
        }
        .into_any()
    }

    fn card_visual(
        self,
        state: &CardOverlay,
        printing: Option<&CardPrinting>,
        title: &str,
        pop_ref: NodeRef<html::Aside>,
    ) -> AnyView {
        let lang = self.lang;
        if !self.previews_enabled.get() {
            return view! {
                <div class="card-preview-notice">
                    {glyph(glyph::EYE_OFF)}
                    <p>{words::chat_card_previews_disabled(lang)}</p>
                    <button type="button" class="button small" on:click=move |_| self.toggle_previews()>
                        {words::chat_card_previews_off(lang)}
                    </button>
                </div>
            }
            .into_any();
        }
        if state.loading {
            return view! {
                <div class="preview-card-stage">
                    <div class="card-preview-skeleton" role="status">
                        <span>{words::account_loading(lang)}</span>
                    </div>
                </div>
            }
            .into_any();
        }
        if let Some(p) = printing
            && !p.ub
            && let Some(uuid) = p.scryfall_id.as_deref()
            && let Some(img_url) = mentions::scryfall_image_url(uuid)
        {
            let finish_attr = state.mention.finish.attr();
            let foil_w = "244";
            let foil_h = "340";
            return view! {
                <div class="preview-card-stage" data-finish=finish_attr>
                    <div class="preview-card-tilt">
                        <div class="card-preview-frame" data-finish=finish_attr>
                            <img
                                class="card-preview-img"
                                src=img_url
                                alt=title.to_owned()
                                referrerpolicy="no-referrer"
                                decoding="async"
                                on:load=move |_| self.reposition_popover(pop_ref)
                            />
                            {finish_attr.map(|f| view! {
                                <canvas
                                    class="card-foil-canvas"
                                    width=foil_w
                                    height=foil_h
                                    aria-hidden="true"
                                    data-card-finish=f
                                ></canvas>
                            })}
                        </div>
                    </div>
                </div>
            }
            .into_any();
        }
        let msg = if printing.is_some_and(|p| p.ub) {
            words::chat_card_ub_notice(lang)
        } else {
            words::chat_card_open_scryfall(lang)
        };
        view! {
            <div class="preview-card-stage">
                <div class="card-preview-notice">
                    {glyph(glyph::LAYERS)}
                    <p>{msg}</p>
                </div>
            </div>
        }
        .into_any()
    }

    fn render_card(
        self,
        state: &CardOverlay,
        is_pinned: bool,
        pop_ref: NodeRef<html::Aside>,
    ) -> AnyView {
        let lang = self.lang;
        let printing = state.printing.as_ref();
        let title = state
            .card
            .as_ref()
            .map_or_else(|| state.mention.name.clone(), |c| c.name.clone());
        let subtitle = state
            .card
            .as_ref()
            .filter(|c| !c.en_name.eq_ignore_ascii_case(&title))
            .map(|c| c.en_name.clone());
        let target_mention = CardMention {
            label: title.clone(),
            name: state.mention.name.clone(),
            set: printing
                .map(|p| p.set_code.clone())
                .or_else(|| state.mention.set.clone()),
            collector: printing
                .map(|p| p.collector_number.clone())
                .or_else(|| state.mention.collector.clone()),
            lang: printing
                .map(|p| p.lang.clone())
                .or_else(|| state.mention.lang.clone()),
            finish: state.mention.finish,
        };
        let scryfall_href = mentions::scryfall_page_url(&target_mention);
        let visual = self.card_visual(state, printing, &title, pop_ref);
        let finish_badge = match state.mention.finish {
            CardFinish::Normal => None,
            CardFinish::Foil => Some(("foil", words::chat_card_finish_foil(lang))),
            CardFinish::Etched => Some(("etched", words::chat_card_finish_etched(lang))),
        };
        let print_line = printing.map(|p| {
            format!(
                "{} #{} · {} ({})",
                p.set_code.to_ascii_uppercase(),
                p.collector_number,
                p.set_name,
                p.lang.to_ascii_uppercase()
            )
        });
        let close_btn = is_pinned.then(|| {
            view! {
                <button
                    type="button"
                    class="dialog-close preview-card-close"
                    aria-label=words::chat_close(lang)
                    title=words::chat_close(lang)
                    on:click=move |_| self.dismiss()
                >
                    {glyph(glyph::CLOSE)}
                </button>
            }
        });

        view! {
            <div class="preview-card-sheet">
                {close_btn}
                <div class="preview-card-visual">{visual}</div>
                <div class="preview-card-plinth">
                    <div class="preview-card-plinth-head">
                        <div class="preview-card-titles">
                            <h4 translate="no">{title}</h4>
                            {subtitle.map(|en| view! { <span class="preview-card-sub" translate="no">{en}</span> })}
                        </div>
                        {finish_badge.map(|(kind, label)| view! {
                            <span class="preview-card-finish" data-finish=kind>
                                {glyph(glyph::SPARKLES)}
                                <span>{label}</span>
                            </span>
                        })}
                    </div>
                    {print_line.map(|line| view! { <p class="preview-card-print-line" translate="no">{line}</p> })}
                    <footer class="preview-card-foot">
                        <span class="preview-card-credit">{words::chat_card_credit(lang)}</span>
                        <a
                            class="button small quiet"
                            href=scryfall_href
                            target="_blank"
                            rel="noopener noreferrer"
                        >
                            {glyph(glyph::EXTERNAL_LINK)}
                            <span>{words::chat_card_open_scryfall(lang)}</span>
                        </a>
                    </footer>
                </div>
            </div>
        }
        .into_any()
    }

    fn render_user(self, state: UserOverlay, is_pinned: bool) -> AnyView {
        let lang = self.lang;
        let tag_num = super::model::tag_of(&state.tag_hex);
        let seed = tag_num.map_or_else(|| state.tag_hex.clone(), |t| t.to_string());
        let ctx_pic = tag_num.and_then(picture::face_of);
        let ctx_rank: Option<Rank> = tag_num.and_then(rank_of);

        let (display_name, pic, best_rank) = match &state.profile {
            Some(p) => (
                p.display_name.clone(),
                p.picture.as_ref().map(|k| (k.key, k.moving)).or(ctx_pic),
                p.badges.best_rank.or(ctx_rank),
            ),
            None => (state.name.clone(), ctx_pic, ctx_rank),
        };
        let profile_href = player_path(lang, &display_name, &state.tag_hex);

        let body = user_overlay_body(lang, &state);

        let close_btn = is_pinned.then(|| {
            view! {
                <button
                    type="button"
                    class="dialog-close"
                    aria-label=words::chat_close(lang)
                    title=words::chat_close(lang)
                    on:click=move |_| self.dismiss()
                >
                    {glyph(glyph::CLOSE)}
                </button>
            }
        });

        let presence_badge = move || {
            tag_num.and_then(super::socket::presence_of).map(|p_state| {
                let attr = p_state.attr();
                let label = p_state.label(lang);
                view! {
                    <span class="preview-user-presence" data-state=attr>
                        <span class="presence-dot" data-state=attr aria-hidden="true">{""}</span>
                        <span>{label}</span>
                    </span>
                }
            })
        };

        view! {
            <div class="preview-user-sheet">
                <div class="preview-user-banner" data-hue=hue(&seed).to_string()>
                    {close_btn}
                </div>
                <div class="preview-user-head">
                    {rank_avatar(display_name.clone(), &seed, pic, best_rank, AvatarSize::Medium)}
                    <div class="preview-user-identity">
                        <div class="preview-user-title-row">
                            <h4><bdi translate="no">{display_name.clone()}</bdi></h4>
                            {presence_badge}
                        </div>
                        {handle_with_copy(lang, display_name, state.tag_hex, false)}
                    </div>
                </div>
                <div class="preview-user-body">{body}</div>
                {state.profile.is_some().then(|| view! {
                    <footer class="preview-user-foot">
                        <a
                            class="button small"
                            href=profile_href
                            target="_blank"
                            rel="noopener noreferrer"
                        >
                            {glyph(glyph::USER)}
                            <span>{words::chat_user_open_profile(lang)}</span>
                        </a>
                    </footer>
                })}
            </div>
        }
        .into_any()
    }
}

fn user_overlay_body(lang: Lang, state: &UserOverlay) -> AnyView {
    if state.loading {
        return view! { <p class="meta" role="status">{words::account_loading(lang)}</p> }
            .into_any();
    }
    let Some(p) = &state.profile else {
        return view! { <p class="meta">{words::chat_user_private(lang)}</p> }.into_any();
    };
    let games: u32 = p
        .badges
        .ranked_standings
        .iter()
        .filter(|s| s.track == Track::Player)
        .map(|s| s.career.games)
        .sum();
    let wins: u32 = p
        .badges
        .ranked_standings
        .iter()
        .filter(|s| s.track == Track::Player)
        .map(|s| s.career.wins)
        .sum();
    let best_label = p.badges.best_rank.map_or_else(
        || words::rank_league_unranked(lang).to_owned(),
        |r| rank_title(lang, r),
    );
    view! {
        <div class="preview-user-content">
            {preview_badges_showcase(lang, &p.badges)}
            <dl class="preview-user-stats">
                <div>
                    <dt>{words::player_best_rank(lang)}</dt>
                    <dd>{best_label}</dd>
                </div>
                <div>
                    <dt>{words::player_career_games(lang)}</dt>
                    <dd translate="no">{games.to_string()}</dd>
                </div>
                <div>
                    <dt>{words::player_career_wins(lang)}</dt>
                    <dd translate="no">{wins.to_string()}</dd>
                </div>
            </dl>
        </div>
    }
    .into_any()
}
