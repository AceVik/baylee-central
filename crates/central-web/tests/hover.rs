//! The owner's rule (11.11.2026): animations are welcome, but hovering never
//! changes an element's size or place as a whole. A slide inside it (a
//! pseudo-element or a child moving) is fine. So no rule whose subject is
//! the hovered element itself may set a transform, a size, a margin, a
//! padding, an offset or a font size.
#![forbid(unsafe_code)]

use std::fs;
use std::path::Path;

/// What moves or resizes a box.
const FORBIDDEN: [&str; 19] = [
    "transform",
    "translate",
    "scale",
    "rotate",
    "width",
    "height",
    "min-width",
    "min-height",
    "inline-size",
    "block-size",
    "margin",
    "padding",
    "inset",
    "top",
    "left",
    "right",
    "bottom",
    "font-size",
    "position",
];

/// Every `selector { declarations }` of `css`, nested blocks (`@media`,
/// `@supports`) opened, `@keyframes` skipped, comments removed.
fn rules(css: &str) -> Vec<(String, String)> {
    let mut text = String::new();
    let mut rest = css;
    while let Some(at) = rest.find("/*") {
        text.push_str(&rest[..at]);
        rest = rest[at..]
            .find("*/")
            .map_or("", |end| &rest[at + end + 2..]);
    }
    text.push_str(rest);
    let mut out = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut start = 0;
    for (at, c) in text.char_indices() {
        match c {
            '{' => {
                stack.push(text[start..at].trim().to_owned());
                start = at + 1;
            }
            '}' => {
                let body = text[start..at].trim();
                let selector = stack.pop().unwrap_or_default();
                let inside_keyframes = stack.iter().any(|s| s.starts_with("@keyframes"));
                if !selector.starts_with('@') && !inside_keyframes && !body.is_empty() {
                    out.push((selector, body.to_owned()));
                }
                start = at + 1;
            }
            ';' if stack.is_empty() => start = at + 1,
            _ => {}
        }
    }
    out
}

/// Whether the hovered element itself is the subject of `selector`: `:hover`
/// in its last compound, with no pseudo-element after it.
fn hovers_itself(selector: &str) -> bool {
    selector.split(',').any(|one| {
        let one = one.trim();
        let last = one
            .rsplit(|c: char| c.is_whitespace() || c == '>' || c == '+' || c == '~')
            .find(|part| !part.is_empty())
            .unwrap_or("");
        last.contains(":hover") && !last[last.find(":hover").unwrap_or(0)..].contains("::")
    })
}

fn offences(css: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (selector, body) in rules(css) {
        if !hovers_itself(&selector) {
            continue;
        }
        for declaration in body.split(';') {
            let Some((property, _)) = declaration.split_once(':') else {
                continue;
            };
            let property = property.trim();
            if FORBIDDEN.contains(&property)
                || property.starts_with("margin-")
                || property.starts_with("padding-")
                || property.starts_with("inset-")
            {
                found.push(format!("{selector} {{ {} }}", declaration.trim()));
            }
        }
    }
    found
}

#[test]
fn hovering_never_moves_or_resizes() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("style");
    let mut found = Vec::new();
    for entry in fs::read_dir(&dir).expect("the stylesheets") {
        let path = entry.expect("an entry").path();
        if path.extension().is_some_and(|e| e == "css") {
            let css = fs::read_to_string(&path).expect("a stylesheet");
            found.extend(
                offences(&css)
                    .into_iter()
                    .map(|o| format!("{}: {o}", path.display())),
            );
        }
    }
    assert!(
        found.is_empty(),
        "hover rules that move or resize: {found:#?}"
    );
}

#[test]
fn the_check_finds_what_it_must() {
    assert_eq!(
        offences(".a:hover { transform: translateY(-2px); }").len(),
        1
    );
    assert_eq!(
        offences(".a:hover { padding-inline: 1em; color: red; }").len(),
        1
    );
    assert_eq!(
        offences("@media (min-width: 1px) { .b .a:hover { margin: 0; } }").len(),
        1
    );
    // inside the element, or not on hover: allowed
    assert_eq!(
        offences(".a:hover::after { transform: translateX(100%); }"),
        Vec::<String>::new()
    );
    assert_eq!(
        offences(".a:hover .b { transform: rotate(1deg); }"),
        Vec::<String>::new()
    );
    assert_eq!(
        offences(".a:hover { color: red; background: blue; }"),
        Vec::<String>::new()
    );
    assert_eq!(
        offences("@keyframes k { from { transform: none; } }"),
        Vec::<String>::new()
    );
    assert_eq!(
        offences(".a:active { transform: none; }"),
        Vec::<String>::new()
    );
}
