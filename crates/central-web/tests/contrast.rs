//! WCAG 2.2 contrast of every colour pair the pages draw, in both schemes,
//! measured from `style/tokens.css` itself: text 4.5:1, the focus ring and
//! borders that carry meaning 3:1.
#![forbid(unsafe_code)]

use std::collections::HashMap;

const TOKENS: &str = include_str!("../style/tokens.css");

/// The `--name: #rrggbb;` declarations of the block that starts at `head`.
fn block(head: &str) -> HashMap<String, [f64; 3]> {
    let start = TOKENS
        .find(head)
        .unwrap_or_else(|| panic!("{head} in tokens.css"))
        + head.len();
    let body = &TOKENS[start..start + TOKENS[start..].find('}').expect("a closed block")];
    let mut out = HashMap::new();
    for line in body.lines() {
        let Some((name, value)) = line.trim().split_once(':') else {
            continue;
        };
        let value = value.trim().trim_end_matches(';');
        let Some(hex) = value.strip_prefix('#').filter(|h| h.len() == 6) else {
            continue;
        };
        let channel =
            |at: usize| f64::from(u8::from_str_radix(&hex[at..at + 2], 16).expect("hex")) / 255.0;
        out.insert(name.trim().to_owned(), [channel(0), channel(2), channel(4)]);
    }
    out
}

fn luminance([r, g, b]: [f64; 3]) -> f64 {
    let linear = |c: f64| {
        if c <= 0.039_28 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
}

fn ratio(a: [f64; 3], b: [f64; 3]) -> f64 {
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// Text on grounds: 4.5:1.
const TEXT: [(&str, &str); 39] = [
    ("--ink", "--ground"),
    ("--ink", "--ground-alt"),
    ("--ink", "--surface"),
    ("--ink", "--surface-raised"),
    ("--ink", "--bubble-own"),
    ("--ink", "--warn-tint"),
    ("--ink", "--ok-tint"),
    ("--ink", "--bad-tint"),
    ("--ink-soft", "--ground"),
    ("--ink-soft", "--ground-alt"),
    ("--ink-soft", "--surface"),
    ("--ink-soft", "--surface-raised"),
    ("--ink-faint", "--ground"),
    ("--ink-faint", "--ground-alt"),
    ("--ink-faint", "--surface"),
    ("--title", "--ground"),
    ("--title", "--ground-alt"),
    ("--title", "--surface"),
    ("--accent", "--ground"),
    ("--accent", "--ground-alt"),
    ("--accent", "--surface"),
    ("--accent", "--surface-raised"),
    ("--on-action", "--action"),
    ("--on-action", "--action-hover"),
    ("--ok", "--surface"),
    ("--warn", "--ground"),
    ("--warn", "--surface"),
    ("--bad", "--ground"),
    ("--bad", "--surface"),
    // the messenger: badges, times in one's own bubbles, a changed key's
    // mark, and initials on the six avatar tints
    ("--surface", "--accent"),
    ("--ink-faint", "--bubble-own"),
    ("--bad", "--bad-tint"),
    ("--on-avatar", "--avatar-0"),
    ("--on-avatar", "--avatar-1"),
    ("--on-avatar", "--avatar-2"),
    ("--on-avatar", "--avatar-3"),
    ("--on-avatar", "--avatar-4"),
    ("--on-avatar", "--avatar-5"),
    ("--ink-faint", "--ground-alt"),
];

/// Non-text that carries meaning (the focus ring, field borders, the gold
/// button's edge): 3:1.
const GRAPHICS: [(&str, &str); 7] = [
    ("--focus", "--ground"),
    ("--focus", "--surface"),
    ("--focus", "--surface-raised"),
    ("--edge", "--ground"),
    ("--edge", "--surface"),
    ("--action-edge", "--ground"),
    ("--accent", "--surface-raised"),
];

fn check(scheme: &str, colours: &HashMap<String, [f64; 3]>) -> Vec<String> {
    let mut failures = Vec::new();
    for (pairs, least) in [(&TEXT[..], 4.5), (&GRAPHICS[..], 3.0)] {
        for (front, back) in pairs {
            let (Some(f), Some(b)) = (colours.get(*front), colours.get(*back)) else {
                failures.push(format!("{scheme}: {front} or {back} is missing"));
                continue;
            };
            let got = ratio(*f, *b);
            if got < least {
                failures.push(format!(
                    "{scheme}: {front} on {back} is {got:.2}:1, below {least}:1"
                ));
            }
        }
    }
    failures
}

#[test]
fn both_schemes_meet_aa() {
    let light = block(":root {");
    let dark = block(":root[data-theme=\"dark\"] {");
    let mut failures = check("light", &light);
    failures.extend(check("dark", &dark));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn the_dark_scheme_is_the_same_for_the_device_and_the_choice() {
    let chosen = block(":root[data-theme=\"dark\"] {");
    let device = block(":root:not([data-theme=\"light\"]) {");
    assert_eq!(chosen, device, "the two dark blocks must agree");
}
