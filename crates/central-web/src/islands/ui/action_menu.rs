//! Reusable `⋮` overflow action menu (`ui/action_menu.rs`) for row actions.
//!
//! Dismisses on outside pointer press or `Escape` (returning focus to the
//! trigger button) and attaches an icon + accessible label to every menu item.

use std::sync::atomic::{AtomicU32, Ordering};

use leptos::html;
use leptos::prelude::*;

use super::super::chat::glyph::{ELLIPSIS_VERTICAL, Shape, glyph};

static NEXT_MENU_ID: AtomicU32 = AtomicU32::new(1);

fn target_inside_wrap(
    ev: &leptos::ev::PointerEvent,
    wrap: &leptos::web_sys::HtmlDivElement,
) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        node_inside_wrap(ev.target(), wrap)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (ev, wrap);
        false
    }
}

#[cfg(target_arch = "wasm32")]
fn node_inside_wrap(
    target: Option<leptos::web_sys::EventTarget>,
    wrap: &leptos::web_sys::HtmlDivElement,
) -> bool {
    use js_sys::{Function, Reflect};
    use wasm_bindgen::{JsCast, JsValue};
    let Some(target) = target else {
        return false;
    };
    let el: &leptos::web_sys::Element = wrap.as_ref();
    let wrap_val: &JsValue = el.as_ref();
    let target_val: JsValue = target.into();
    Reflect::get(wrap_val, &JsValue::from_str("contains"))
        .ok()
        .and_then(|f| f.dyn_into::<Function>().ok())
        .and_then(|f| f.call1(wrap_val, &target_val).ok())
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

#[cfg(target_arch = "wasm32")]
fn focus_edge_item(item_refs: &[NodeRef<html::Button>], last: bool) {
    use wasm_bindgen::{JsCast, closure::Closure};
    let target = if last {
        item_refs.last().copied()
    } else {
        item_refs.first().copied()
    };
    let Some(target) = target else {
        return;
    };
    let cb = Closure::once_into_js(move || {
        if let Some(btn) = target.get_untracked() {
            let _ = btn.focus();
        }
    });
    if let Some(win) = leptos::web_sys::window() {
        let _ = win.request_animation_frame(cb.unchecked_ref());
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn focus_edge_item(_item_refs: &[NodeRef<html::Button>], _last: bool) {}

#[cfg(target_arch = "wasm32")]
fn step_menu_focus(item_refs: &[NodeRef<html::Button>], key: &str) -> bool {
    use wasm_bindgen::JsCast;
    if item_refs.is_empty() {
        return false;
    }
    let active = leptos::web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.active_element())
        .and_then(|el| el.dyn_into::<leptos::web_sys::Node>().ok());
    let current = active
        .as_ref()
        .and_then(|cur| {
            item_refs.iter().position(|r| {
                r.get_untracked().is_some_and(|btn| {
                    let node: &leptos::web_sys::Node = btn.as_ref();
                    node.is_same_node(Some(cur))
                })
            })
        })
        .unwrap_or(0);
    let len = item_refs.len();
    let next = match key {
        "ArrowDown" => (current + 1) % len,
        "ArrowUp" => (current + len - 1) % len,
        "Home" => 0,
        "End" => len - 1,
        _ => return false,
    };
    if let Some(btn) = item_refs[next].get_untracked() {
        let _ = btn.focus();
    }
    true
}

#[cfg(not(target_arch = "wasm32"))]
fn step_menu_focus(_item_refs: &[NodeRef<html::Button>], _key: &str) -> bool {
    false
}

/// One entry inside an [`action_menu`].
#[derive(Clone)]
pub struct ActionItem {
    /// Human-readable item label.
    pub label: String,
    /// Lucide icon shapes rendered beside the label.
    pub icon: &'static [Shape],
    /// Whether this action is destructive (styled in warning/danger ink).
    pub danger: bool,
    /// Handler invoked when the user activates the item.
    pub on_select: Callback<()>,
}

#[cfg(target_arch = "wasm32")]
fn related_inside_wrap(
    ev: &leptos::ev::FocusEvent,
    wrap: &leptos::web_sys::HtmlDivElement,
) -> bool {
    node_inside_wrap(ev.related_target(), wrap)
}

#[cfg(not(target_arch = "wasm32"))]
fn related_inside_wrap(
    ev: &leptos::ev::FocusEvent,
    wrap: &leptos::web_sys::HtmlDivElement,
) -> bool {
    let _ = (ev, wrap);
    false
}

fn attach_dismiss_listeners(
    open: RwSignal<bool>,
    wrap_ref: NodeRef<html::Div>,
    trigger_ref: NodeRef<html::Button>,
) {
    let outside_handle = window_event_listener(leptos::ev::pointerdown, move |ev| {
        if !open.get_untracked() {
            return;
        }
        let Some(wrap) = wrap_ref.get_untracked() else {
            return;
        };
        if !target_inside_wrap(&ev, &wrap) {
            open.set(false);
        }
    });
    let esc_handle = window_event_listener(leptos::ev::keydown, move |ev| {
        if ev.key() == "Escape" && open.get_untracked() {
            ev.stop_propagation();
            open.set(false);
            if let Some(btn) = trigger_ref.get_untracked() {
                let _ = btn.focus();
            }
        }
    });
    on_cleanup(move || {
        outside_handle.remove();
        esc_handle.remove();
    });
}

fn build_menu_rows(
    open: RwSignal<bool>,
    items: Vec<ActionItem>,
    item_refs: &[NodeRef<html::Button>],
) -> impl IntoView {
    items
        .into_iter()
        .zip(item_refs.iter().copied())
        .map(|(item, btn_ref)| {
            let cls = if item.danger {
                "action-menu-item danger"
            } else {
                "action-menu-item"
            };
            let on_select = item.on_select;
            view! {
                <button
                    type="button"
                    node_ref=btn_ref
                    class=cls
                    role="menuitem"
                    on:click=move |ev| {
                        ev.stop_propagation();
                        open.set(false);
                        on_select.run(());
                    }
                >
                    {glyph(item.icon)}
                    <span>{item.label}</span>
                </button>
            }
        })
        .collect_view()
}

/// Renders a compact `⋮` overflow menu trigger and floating action list.
pub fn action_menu(trigger_label: String, items: Vec<ActionItem>) -> AnyView {
    let id = NEXT_MENU_ID.fetch_add(1, Ordering::Relaxed);
    let menu_id = format!("action-menu-{id}");
    let open = RwSignal::new(false);
    let wrap_ref = NodeRef::<html::Div>::new();
    let trigger_ref = NodeRef::<html::Button>::new();
    let item_refs: Vec<NodeRef<html::Button>> = items
        .iter()
        .map(|_| NodeRef::<html::Button>::new())
        .collect();
    let item_refs_click = item_refs.clone();
    let item_refs_trigger_key = item_refs.clone();
    let item_refs_menu_key = item_refs.clone();

    attach_dismiss_listeners(open, wrap_ref, trigger_ref);
    let rows = build_menu_rows(open, items, &item_refs);

    view! {
        <div
            class="action-menu"
            node_ref=wrap_ref
            data-open=move || open.get().then_some("true")
            on:focusout=move |ev| {
                if !open.get_untracked() {
                    return;
                }
                let Some(wrap) = wrap_ref.get_untracked() else {
                    return;
                };
                if !related_inside_wrap(&ev, &wrap) {
                    open.set(false);
                }
            }
        >
            <button
                type="button"
                node_ref=trigger_ref
                class="action-menu-trigger"
                aria-haspopup="menu"
                aria-expanded=move || if open.get() { "true" } else { "false" }
                aria-controls=menu_id.clone()
                aria-label=trigger_label.clone()
                title=trigger_label
                on:click=move |ev| {
                    ev.stop_propagation();
                    let next = !open.get_untracked();
                    open.set(next);
                    if next {
                        focus_edge_item(&item_refs_click, false);
                    }
                }
                on:keydown=move |ev| {
                    let key = ev.key();
                    if matches!(key.as_str(), "ArrowDown" | "ArrowUp") {
                        ev.prevent_default();
                        ev.stop_propagation();
                        open.set(true);
                        focus_edge_item(&item_refs_trigger_key, key == "ArrowUp");
                    }
                }
            >
                {glyph(ELLIPSIS_VERTICAL)}
            </button>
            <div
                class="action-menu-popover"
                id=menu_id
                role="menu"
                hidden=move || !open.get()
                on:keydown=move |ev| {
                    let key = ev.key();
                    if key == "Tab" {
                        open.set(false);
                        return;
                    }
                    if step_menu_focus(&item_refs_menu_key, &key) {
                        ev.prevent_default();
                        ev.stop_propagation();
                    }
                }
            >
                {rows}
            </div>
        </div>
    }
    .into_any()
}
