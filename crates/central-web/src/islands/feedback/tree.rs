//! A JSON value as a tree (the old administration's `JsonTree.tsx`):
//! objects and arrays fold on the browser's own `<details>`, so the
//! keyboard and a screen reader know them, and their children are drawn
//! only once unfolded, a page of them at a time, so a large client part
//! costs nothing until it is read. Long strings show their start until
//! asked; base64 is counted, not shown. Every key and value is text:
//! nothing here is markup.

use std::sync::Arc;

use leptos::prelude::*;
use serde_json::Value;

use crate::words::{self, Lang};

/// Strings longer than this show their start and a button for the rest.
const LONG_STRING: usize = 240;
/// Children drawn at once; a button draws the next as many.
const CHILDREN: usize = 200;
/// Levels drawn at most: deeper ones say how many they hold.
const DEPTH: usize = 32;
/// Keys whose value is not worth reading as text.
const OPAQUE: [&str; 2] = ["png_base64", "gzip_base64"];

fn number(lang: Lang, n: usize) -> String {
    words::number(lang, u64::try_from(n).unwrap_or(u64::MAX))
}

fn long_string(lang: Lang, value: String) -> AnyView {
    let length = value.chars().count();
    if length <= LONG_STRING {
        return view! { <span class="fb-j-string">{format!("“{value}”")}</span> }.into_any();
    }
    let all = RwSignal::new(false);
    let start: String = value.chars().take(LONG_STRING).collect();
    view! {
        <span class="fb-j-string">{move || if all.get() { format!("“{value}”") } else { format!("“{start}…”") }}</span>
        {move || (!all.get()).then(|| view! {
            " "
            <button type="button" class="text-link" on:click=move |_| all.set(true)>
                {words::fb_json_all(lang, &number(lang, length))}
            </button>
        })}
    }
    .into_any()
}

fn leaf(lang: Lang, name: Option<&str>, value: &Value) -> AnyView {
    match value {
        Value::String(s) if name.is_some_and(|n| OPAQUE.contains(&n)) => {
            view! { <span class="meta">{words::fb_json_base64(lang, &number(lang, s.chars().count()))}</span> }
                .into_any()
        }
        Value::String(s) => long_string(lang, s.clone()),
        Value::Number(n) => view! { <span class="fb-j-value">{n.to_string()}</span> }.into_any(),
        Value::Bool(b) => view! { <span class="fb-j-value">{b.to_string()}</span> }.into_any(),
        Value::Null => view! { <span class="fb-j-value">{"null"}</span> }.into_any(),
        _ => ().into_any(),
    }
}

/// The children of an object or an array, a page at a time.
fn children(lang: Lang, entries: Arc<Vec<(String, Value)>>, depth: usize) -> impl IntoView {
    let shown = RwSignal::new(CHILDREN.min(entries.len()));
    let list = entries.clone();
    let items = move || {
        list.iter()
            .take(shown.get())
            .map(|(key, value)| node(lang, Some(key), value, depth + 1, false))
            .collect_view()
    };
    let more = move || {
        let left = entries.len().saturating_sub(shown.get());
        (left > 0).then(|| {
            let next = left.min(CHILDREN);
            view! {
                <li>
                    <button type="button" class="text-link" on:click=move |_| shown.update(|n| *n += CHILDREN)>
                        {words::fb_json_more(lang, &number(lang, next), &number(lang, left))}
                    </button>
                </li>
            }
        })
    };
    view! { <ul class="fb-j-children">{items}{more}</ul> }
}

fn node(lang: Lang, name: Option<&str>, value: &Value, depth: usize, open: bool) -> AnyView {
    let label = name.map(|n| view! { <span class="fb-j-key">{format!("{n}: ")}</span> });
    let entries: Option<Vec<(String, Value)>> = match value {
        Value::Array(items) => Some(
            items
                .iter()
                .enumerate()
                .map(|(i, v)| (i.to_string(), v.clone()))
                .collect(),
        ),
        Value::Object(map) => Some(map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
        _ => None,
    };
    let Some(entries) = entries else {
        return view! { <li>{label}{leaf(lang, name, value)}</li> }.into_any();
    };
    let (a, b) = if value.is_array() {
        ("[", "]")
    } else {
        ("{", "}")
    };
    let count = if value.is_array() {
        words::fb_json_items(lang, &number(lang, entries.len()))
    } else {
        words::fb_json_keys(lang, &number(lang, entries.len()))
    };
    if entries.is_empty() || depth >= DEPTH {
        let inside = if entries.is_empty() {
            String::new()
        } else {
            count
        };
        return view! { <li>{label}<span class="meta">{format!("{a} {inside} {b}")}</span></li> }
            .into_any();
    }
    let unfolded = RwSignal::new(open);
    let entries = Arc::new(entries);
    view! {
        <li>
            <details open=open on:toggle=move |ev| {
                let now = event_target::<leptos::web_sys::HtmlDetailsElement>(&ev).open();
                unfolded.set(now);
            }>
                <summary>{label}<span class="meta">{format!("{a} {count} {b}")}</span></summary>
                {move || unfolded.get().then(|| children(lang, entries.clone(), depth))}
            </details>
        </li>
    }
    .into_any()
}

/// `value` as a tree, its top level unfolded when `open`.
pub fn tree(lang: Lang, value: &Value, open: bool) -> impl IntoView + use<> {
    view! { <ul class="fb-json" translate="no">{node(lang, None, value, 0, open)}</ul> }
}
