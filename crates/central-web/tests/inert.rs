//! No inert element in an island. Leptos builds an element whose attributes
//! and children are all literal ("inert") in the browser through `innerHTML`
//! on a `<template>`, which the site's Trusted Types forbid: such an element
//! works when the server rendered it and throws when the browser renders it
//! later (a form shown again, a list filled in). Literal text goes in braces,
//! `{"Website"}`, which Leptos renders with plain DOM calls.
//!
//! The rule mirrors `leptos_macro`'s `is_inert_element` (0.8): an element,
//! not a component, with attributes or children, where every attribute is
//! bare or a string literal under a plain name, and every child is text or
//! such an element.
#![forbid(unsafe_code)]

use std::fs;
use std::path::Path;

fn sources(dir: &Path, out: &mut Vec<(String, String)>) {
    for entry in fs::read_dir(dir).expect("the islands' folder") {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let text = fs::read_to_string(&path).expect("a source");
            out.push((path.display().to_string(), text));
        }
    }
}

fn spaces(out: &mut [u8]) {
    for byte in out.iter_mut().filter(|b| **b != b'\n') {
        *byte = b' ';
    }
}

/// `text` with comments and the insides of string literals turned to spaces,
/// byte for byte, so that what a literal or a comment says can pass neither
/// for markup nor for code.
fn blank(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = bytes.to_vec();
    let ident =
        |at: usize| at > 0 && (bytes[at - 1].is_ascii_alphanumeric() || bytes[at - 1] == b'_');
    let mut i = 0;
    while i < bytes.len() {
        let next = |n: usize| bytes.get(i + n).copied();
        match bytes[i] {
            b'/' if next(1) == Some(b'/') => {
                let end = text[i..].find('\n').map_or(bytes.len(), |n| i + n);
                spaces(&mut out[i..end]);
                i = end;
            }
            b'/' if next(1) == Some(b'*') => {
                let end = text[i + 2..].find("*/").map_or(bytes.len(), |n| i + n + 4);
                spaces(&mut out[i..end]);
                i = end;
            }
            b'\'' if next(1) == Some(b'"') && next(2) == Some(b'\'') => i += 3,
            b'\'' if next(1) == Some(b'\\') && next(3) == Some(b'\'') => i += 4,
            b'r' if !ident(i) && matches!(next(1), Some(b'#' | b'"')) => {
                let hashes = bytes[i + 1..].iter().take_while(|b| **b == b'#').count();
                if next(1 + hashes) != Some(b'"') {
                    i += 1;
                    continue;
                }
                let body = i + hashes + 2;
                let closing = format!("\"{}", "#".repeat(hashes));
                let end = text[body..]
                    .find(&closing)
                    .map_or(bytes.len(), |n| body + n);
                spaces(&mut out[body..end]);
                i = end + closing.len();
            }
            b'"' => {
                let mut end = i + 1;
                while end < bytes.len() && bytes[end] != b'"' {
                    end += if bytes[end] == b'\\' { 2 } else { 1 };
                }
                let end = end.min(bytes.len());
                spaces(&mut out[i + 1..end]);
                i = end + 1;
            }
            _ => i += 1,
        }
    }
    String::from_utf8(out).expect("whole characters were blanked")
}

/// The tag opening at `start` in blanked text: its name and where its `>` is.
fn tag(text: &str, start: usize) -> Option<(&str, usize)> {
    let rest = &text[start + 1..];
    let len = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
        .unwrap_or(rest.len());
    let name = &rest[..len];
    name.starts_with(|c: char| c.is_ascii_alphabetic())
        .then(|| rest.find('>').map(|end| (name, start + 1 + end)))
        .flatten()
}

/// Just past the closing tag of the `name` element whose opening tag ends
/// at `from`, counting nested elements of the same name.
fn close(text: &str, name: &str, from: usize) -> Option<usize> {
    let open = format!("<{name}");
    let shut = format!("</{name}>");
    let mut depth = 1_usize;
    let mut at = from;
    while depth > 0 {
        let next = at + text[at..].find(&shut)?;
        depth += text[at..next]
            .match_indices(&open)
            .filter(|(i, _)| {
                let p = at + i;
                let after = text[p + open.len()..].chars().next();
                after.is_some_and(|c| c.is_whitespace() || c == '>' || c == '/')
                    && tag(text, p).is_some_and(|(_, end)| !text[..end].ends_with('/'))
            })
            .count();
        depth -= 1;
        at = next + shut.len();
    }
    Some(at)
}

/// Whether every attribute in a tag's attribute text is bare or a string
/// literal under a name Leptos treats as plain.
fn literal(attrs: &str) -> bool {
    let plain = attrs.split_whitespace().all(|token| {
        !["on:", "prop:", "style:", "use:", "bind"]
            .iter()
            .any(|p| token.starts_with(p))
    });
    let quoted = attrs
        .match_indices('=')
        .all(|(i, _)| attrs[i + 1..].trim_start().starts_with('"'));
    plain && quoted
}

/// The inert elements in `source`.
fn inert(source: &str) -> Vec<String> {
    let text = blank(source);
    let mut found = Vec::new();
    for (start, _) in text.match_indices('<') {
        let Some((name, open_end)) = tag(&text, start) else {
            continue;
        };
        if !name.starts_with(|c: char| c.is_ascii_lowercase()) {
            continue;
        }
        let attrs = &text[start + 1 + name.len()..open_end];
        let (end, empty) = if let Some(attrs) = attrs.strip_suffix('/') {
            (open_end + 1, attrs.trim().is_empty())
        } else {
            let Some(end) = close(&text, name, open_end + 1) else {
                continue;
            };
            let inner = &text[open_end + 1..end - name.len() - 3];
            (end, attrs.trim().is_empty() && inner.trim().is_empty())
        };
        let whole = &text[start..end];
        let all_literal = !empty
            && !whole.contains('{')
            && whole.match_indices('<').all(|(i, _)| match tag(whole, i) {
                Some((inner, tag_end)) => {
                    inner.starts_with(|c: char| c.is_ascii_lowercase())
                        && literal(&whole[i + 1 + inner.len()..tag_end])
                }
                None => true,
            });
        if all_literal {
            found.push(source[start..end].chars().take(80).collect());
        }
    }
    found
}

#[test]
fn islands_and_their_views_hold_no_inert_element() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    sources(&root.join("islands"), &mut files);
    // Views the islands render from outside their folder.
    for shared in ["rich.rs", "markdown.rs"] {
        let path = root.join(shared);
        let text = fs::read_to_string(&path).expect("a shared view");
        files.push((path.display().to_string(), text));
    }
    let found: Vec<String> = files
        .iter()
        .flat_map(|(file, text)| inert(text).into_iter().map(move |e| format!("{file}: {e}")))
        .collect();
    assert!(found.is_empty(), "inert elements in islands: {found:#?}");
}

#[test]
fn the_scan_finds_what_it_must() {
    let none = Vec::<String>::new();
    assert_eq!(inert(r#"<label for="x">Website</label>"#).len(), 1);
    assert_eq!(inert(r#"<option value="30">"30"</option>"#).len(), 1);
    assert_eq!(inert(r#"<input type="text" required />"#).len(), 1);
    assert_eq!(inert(r#"<p>"plain"</p>"#).len(), 1);
    // What a literal says is not code: neither a binding nor a brace.
    assert_eq!(inert(r#"<p class="x">"Registration: done"</p>"#).len(), 1);
    assert_eq!(inert(r#"<p class="x">"a { b"</p>"#).len(), 1);
    // Nested: the inner one, not the outer one that holds code.
    assert_eq!(
        inert(r#"<div class="a"><span>"x"</span><b>{y}</b></div>"#),
        vec![r#"<span>"x"</span>"#.to_owned()]
    );
    assert_eq!(
        inert(r#"<div class="a"><div class="b">"x"</div>{y}</div>"#),
        vec![r#"<div class="b">"x"</div>"#.to_owned()]
    );
    // A stray quote in a comment does not swallow the file.
    assert_eq!(inert("// it's \"odd\n<p class=\"x\">{y}</p>"), none);
    assert_eq!(inert(r#"<label for="x">{"Website"}</label>"#), none);
    assert_eq!(inert(r#"<p class="meta">{words::x(lang)}</p>"#), none);
    assert_eq!(
        inert(r#"<input id="x" on:input=move |ev| set(ev) />"#),
        none
    );
    assert_eq!(inert(r#"<input type="checkbox" prop:checked=on />"#), none);
    assert_eq!(inert(r#"<p style:color="red">"x"</p>"#), none);
    assert_eq!(
        inert(r#"<option value="90" selected=true>"90"</option>"#),
        none
    );
    assert_eq!(inert(r#"<A href="/x">"x"</A>"#), none);
    assert_eq!(inert(r#"<p><A href="/x">"x"</A></p>"#), none);
    assert_eq!(inert("<br />"), none);
    assert_eq!(inert("<p></p>"), none);
    assert_eq!(inert("let v: Vec<u8> = Vec::new(); if a <b {}"), none);
}
