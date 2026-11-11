//! Turns the site's words (`locales/<lang>/site.ftl`, Project Fluent) into
//! typed Rust: one function per message, taking the language and the
//! message's arguments. A message missing in a language, an argument a
//! language uses that English does not, or a misspelt id at a call site does
//! not compile.
//!
//! The subset of Fluent the site uses, and nothing more: messages (no terms,
//! no attributes), text spanning indented lines (a blank line inside keeps a
//! paragraph break, any other line break is a space), `{ $name }`,
//! `{ "literal" }` and `{ $n -> [one] … [0] … *[other] … }` selecting on a
//! number by CLDR's plural categories or an exact value. A variable that a
//! selector reads is a `u64` (written as the language writes numbers); every
//! other variable is text. A message whose text holds a pair of tags
//! (`<b>…</b>`, `<link>…</link>`) returns pieces for the view to draw; an
//! argument can never open a tag.
#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;
use std::{env, fs};

/// The languages: their file folder (`Lang::code`), their `Lang` variant and
/// the plural categories a selector on a count must name in them (CLDR's
/// for whole numbers, without the millions' `many` of the Romance
/// languages and without `other`, which the default covers). English
/// first: it decides which messages exist and which arguments each takes.
const LANGS: [(&str, &str, &[&str]); 11] = [
    ("en", "En", &["one"]),
    ("de", "De", &["one"]),
    ("fr", "Fr", &["one"]),
    ("it", "It", &["one"]),
    ("es", "Es", &["one"]),
    ("pt-br", "PtBr", &["one"]),
    ("ru", "Ru", &["one", "few", "many"]),
    ("ja", "Ja", &[]),
    ("ko", "Ko", &[]),
    ("zh-hans", "ZhHans", &[]),
    ("zh-hant", "ZhHant", &[]),
];

#[derive(Clone, Debug, PartialEq)]
enum Node {
    Text(String),
    Var(String),
    Select {
        var: String,
        variants: Vec<(Key, Vec<Node>)>,
        default: usize,
    },
}

#[derive(Clone, Debug, PartialEq)]
enum Key {
    Category(String),
    Exact(u64),
}

type Messages = BTreeMap<String, Vec<Node>>;

fn main() {
    let root = Path::new(&env::var("CARGO_MANIFEST_DIR").expect("cargo sets the manifest dir"))
        .join("locales");
    println!("cargo::rerun-if-changed=locales");
    // Every folder of words is a language here: the site's build reads the
    // folders (xtask's site.rs), so the two lists cannot drift apart.
    for entry in fs::read_dir(&root).unwrap_or_else(|error| panic!("locales: {error}")) {
        let name = entry
            .unwrap_or_else(|error| panic!("locales: {error}"))
            .file_name();
        let name = name.to_string_lossy();
        assert!(
            LANGS.iter().any(|(code, _, _)| *code == name) || name.starts_with('.'),
            "locales/{name} is no language of build.rs"
        );
    }
    let mut languages = Vec::new();
    for (code, _, _) in LANGS {
        let path = root.join(code).join("site.ftl");
        println!("cargo::rerun-if-changed={}", path.display());
        let text =
            fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let messages = parse(&text).unwrap_or_else(|error| panic!("{code}/site.ftl: {error}"));
        languages.push(messages);
    }
    check(&languages).unwrap_or_else(|error| panic!("{error}"));
    let code = generate(&languages);
    let out = Path::new(&env::var("OUT_DIR").expect("cargo sets the output dir")).join("words.rs");
    fs::write(&out, code).unwrap_or_else(|error| panic!("{}: {error}", out.display()));
}

// ---------------------------------------------------------------- parsing

fn parse(text: &str) -> Result<Messages, String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut messages = Messages::new();
    let mut at = 0;
    while at < lines.len() {
        let line = lines[at];
        if line.trim().is_empty() || line.starts_with('#') {
            at += 1;
            continue;
        }
        if line.starts_with(char::is_whitespace) {
            return Err(format!(
                "line {}: an indented line outside a message",
                at + 1
            ));
        }
        let (id, first) = line
            .split_once('=')
            .ok_or_else(|| format!("line {}: no `=`", at + 1))?;
        let id = id.trim();
        if !valid_id(id) {
            return Err(format!("line {}: `{id}` is not a message id", at + 1));
        }
        at += 1;
        let mut block: Vec<&str> = Vec::new();
        while at < lines.len() {
            let next = lines[at];
            // A selector's closing brace may stand at the start of a line.
            if next.starts_with(' ') || next.starts_with('}') {
                block.push(next);
                at += 1;
            } else if next.trim().is_empty() {
                let mut end = at;
                while end < lines.len() && lines[end].trim().is_empty() {
                    end += 1;
                }
                if end < lines.len() && lines[end].starts_with(' ') {
                    block.extend(std::iter::repeat_n("", end - at));
                    at = end;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        let indent = block
            .iter()
            .filter(|line| !line.trim().is_empty() && !line.starts_with('}'))
            .map(|line| line.len() - line.trim_start().len())
            .min()
            .unwrap_or(0);
        let mut value = first.trim_start().to_owned();
        for line in block {
            value.push('\n');
            value.push_str(if line.starts_with('}') {
                line
            } else {
                line.get(indent..).unwrap_or("")
            });
        }
        let chars: Vec<char> = value.trim().chars().collect();
        let mut parser = Parser { chars, at: 0 };
        let nodes = parser.pattern(false)?;
        if parser.at != parser.chars.len() {
            return Err(format!("`{id}`: text after the end"));
        }
        if nodes.is_empty() {
            return Err(format!("`{id}`: empty message"));
        }
        if messages.insert(id.to_owned(), normalise(nodes)).is_some() {
            return Err(format!("`{id}` appears twice"));
        }
    }
    Ok(messages)
}

fn valid_id(id: &str) -> bool {
    let mut chars = id.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '-')
}

struct Parser {
    chars: Vec<char>,
    at: usize,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }

    fn skip_space(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.at += 1;
        }
    }

    fn expect(&mut self, wanted: char) -> Result<(), String> {
        if self.peek() == Some(wanted) {
            self.at += 1;
            Ok(())
        } else {
            Err(format!("expected `{wanted}` at {}", self.at))
        }
    }

    /// A pattern up to the end of the value, or (in a variant) up to the
    /// line that starts the next variant or closes the selector.
    fn pattern(&mut self, variant: bool) -> Result<Vec<Node>, String> {
        let mut nodes = Vec::new();
        let mut text = String::new();
        while let Some(c) = self.peek() {
            if variant && c == '\n' {
                let mut look = self.at;
                while self.chars.get(look).is_some_and(|c| c.is_whitespace()) {
                    look += 1;
                }
                if matches!(self.chars.get(look), Some('[' | '*' | '}')) {
                    break;
                }
            }
            match c {
                '{' => {
                    self.at += 1;
                    if !text.is_empty() {
                        nodes.push(Node::Text(std::mem::take(&mut text)));
                    }
                    nodes.push(self.placeable()?);
                }
                '}' => return Err(format!("an unopened `}}` at {}", self.at)),
                _ => {
                    text.push(c);
                    self.at += 1;
                }
            }
        }
        if !text.is_empty() {
            nodes.push(Node::Text(text));
        }
        Ok(nodes)
    }

    fn placeable(&mut self) -> Result<Node, String> {
        self.skip_space();
        match self.peek() {
            Some('"') => {
                self.at += 1;
                let mut literal = String::new();
                loop {
                    match self.peek() {
                        Some('"') => break,
                        Some('\\') => {
                            self.at += 1;
                            literal.push(self.peek().ok_or("an unfinished escape")?);
                        }
                        Some(c) => literal.push(c),
                        None => return Err("an unfinished string".into()),
                    }
                    self.at += 1;
                }
                self.at += 1;
                self.skip_space();
                self.expect('}')?;
                Ok(Node::Text(literal))
            }
            Some('$') => {
                self.at += 1;
                let mut name = String::new();
                while let Some(c) = self
                    .peek()
                    .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
                {
                    name.push(c);
                    self.at += 1;
                }
                if !name.starts_with(|c: char| c.is_ascii_lowercase())
                    || name.chars().any(|c| c.is_ascii_uppercase())
                {
                    return Err(format!("`${name}`: variables are snake_case"));
                }
                self.skip_space();
                if self.peek() == Some('-') && self.chars.get(self.at + 1) == Some(&'>') {
                    self.at += 2;
                    self.select(name)
                } else {
                    self.expect('}')?;
                    Ok(Node::Var(name))
                }
            }
            _ => Err(format!("an unknown placeable at {}", self.at)),
        }
    }

    fn select(&mut self, var: String) -> Result<Node, String> {
        let mut variants = Vec::new();
        let mut default = None;
        loop {
            self.skip_space();
            match self.peek() {
                Some('}') => {
                    self.at += 1;
                    break;
                }
                Some('*') => {
                    self.at += 1;
                    if default.replace(variants.len()).is_some() {
                        return Err(format!("`${var}`: two defaults"));
                    }
                }
                _ => {}
            }
            self.expect('[')?;
            let mut key = String::new();
            while let Some(c) = self.peek().filter(|c| *c != ']') {
                key.push(c);
                self.at += 1;
            }
            self.expect(']')?;
            let key = key.trim();
            let key = match key.parse::<u64>() {
                Ok(n) => Key::Exact(n),
                Err(_) if matches!(key, "zero" | "one" | "two" | "few" | "many" | "other") => {
                    Key::Category(key.to_owned())
                }
                Err(_) => return Err(format!("`${var}`: `[{key}]` is no plural category")),
            };
            while self.peek() == Some(' ') {
                self.at += 1;
            }
            let pattern = self.pattern(true)?;
            variants.push((key, normalise(pattern)));
        }
        let default = default.ok_or_else(|| format!("`${var}`: no default variant"))?;
        Ok(Node::Select {
            var,
            variants,
            default,
        })
    }
}

/// Line breaks inside text become spaces, a blank line stays a paragraph
/// break (`\n\n`), and the ends are trimmed.
fn normalise(nodes: Vec<Node>) -> Vec<Node> {
    let last = nodes.len().saturating_sub(1);
    nodes
        .into_iter()
        .enumerate()
        .filter_map(|(at, node)| match node {
            Node::Text(text) => {
                let mut out = String::new();
                let mut chars = text.chars().peekable();
                while let Some(c) = chars.next() {
                    if c == '\n' || (c == ' ' && chars.peek() == Some(&'\n')) {
                        let mut breaks = usize::from(c == '\n');
                        while let Some(&next) = chars.peek().filter(|c| c.is_whitespace()) {
                            breaks += usize::from(next == '\n');
                            chars.next();
                        }
                        out.push_str(if breaks >= 2 { "\n\n" } else { " " });
                    } else {
                        out.push(c);
                    }
                }
                let out = if at == 0 {
                    out.trim_start().to_owned()
                } else {
                    out
                };
                let out = if at == last {
                    out.trim_end().to_owned()
                } else {
                    out
                };
                (!out.is_empty()).then_some(Node::Text(out))
            }
            other => Some(other),
        })
        .collect()
}

// ---------------------------------------------------------------- checking

/// The variables of a pattern in order of first use, each with whether a
/// selector reads it.
fn variables(nodes: &[Node], out: &mut Vec<(String, bool)>) {
    fn note(out: &mut Vec<(String, bool)>, name: &str, number: bool) {
        if let Some(entry) = out.iter_mut().find(|(n, _)| n == name) {
            entry.1 |= number;
        } else {
            out.push((name.to_owned(), number));
        }
    }
    for node in nodes {
        match node {
            Node::Text(_) => {}
            Node::Var(name) => note(out, name, false),
            Node::Select { var, variants, .. } => {
                note(out, var, true);
                for (_, pattern) in variants {
                    variables(pattern, out);
                }
            }
        }
    }
}

fn texts(nodes: &[Node], out: &mut Vec<String>) {
    for node in nodes {
        match node {
            Node::Text(text) => out.push(text.clone()),
            Node::Var(_) => {}
            Node::Select { variants, .. } => {
                for (_, pattern) in variants {
                    texts(pattern, out);
                }
            }
        }
    }
}

/// The tags a message's text holds, in order (`<b>` → `b`).
fn tags(nodes: &[Node]) -> Vec<String> {
    let mut all = Vec::new();
    texts(nodes, &mut all);
    let mut found = Vec::new();
    for text in all {
        let mut rest = text.as_str();
        while let Some(open) = rest.find('<') {
            rest = &rest[open + 1..];
            if let Some(close) = rest.find('>') {
                let name = &rest[..close];
                if !name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase()) {
                    found.push(name.to_owned());
                }
            }
        }
    }
    found
}

/// The plural categories each selector of a pattern names, the default's
/// among them; a selector on exact values only names none.
fn categories(nodes: &[Node], out: &mut Vec<Vec<String>>) {
    for node in nodes {
        if let Node::Select { variants, .. } = node {
            let named: Vec<String> = variants
                .iter()
                .filter_map(|(key, _)| match key {
                    Key::Category(name) => Some(name.clone()),
                    Key::Exact(_) => None,
                })
                .collect();
            out.push(named);
            for (_, pattern) in variants {
                categories(pattern, out);
            }
        }
    }
}

fn check(languages: &[Messages]) -> Result<(), String> {
    let english = &languages[0];
    for (index, messages) in languages.iter().enumerate() {
        let (code, _, needed) = LANGS[index];
        for id in english.keys() {
            if !messages.contains_key(id) {
                return Err(format!("{code}: `{id}` is missing"));
            }
        }
        for (id, nodes) in messages {
            let Some(reference) = english.get(id) else {
                return Err(format!("{code}: `{id}` is not an English message"));
            };
            // The same arguments in every language: one a translation
            // drops would be a number or a name the reader never sees.
            let mut ours = Vec::new();
            variables(nodes, &mut ours);
            let mut theirs = Vec::new();
            variables(reference, &mut theirs);
            for (name, _) in &ours {
                if !theirs.iter().any(|(n, _)| n == name) {
                    return Err(format!(
                        "{code}: `{id}` uses `${name}`, which English does not"
                    ));
                }
            }
            for (name, _) in &theirs {
                if !ours.iter().any(|(n, _)| n == name) {
                    return Err(format!(
                        "{code}: `{id}` leaves out `${name}`, which English writes"
                    ));
                }
            }
            let (mut a, mut b) = (tags(nodes), tags(reference));
            a.sort();
            b.sort();
            if a != b {
                return Err(format!(
                    "{code}: `{id}` has other tags than English ({a:?}, {b:?})"
                ));
            }
            // A count is selected by every category the language tells
            // apart, or its nouns stand in the wrong form.
            let mut selectors = Vec::new();
            categories(nodes, &mut selectors);
            for named in selectors.iter().filter(|named| !named.is_empty()) {
                if let Some(missing) = needed.iter().find(|need| !named.iter().any(|n| n == *need))
                {
                    return Err(format!(
                        "{code}: a selector of `{id}` names no `[{missing}]`, which {code} needs"
                    ));
                }
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------- generating

fn snake(id: &str) -> String {
    let mut out = String::new();
    for c in id.chars() {
        if c == '-' {
            out.push('_');
        } else if c.is_ascii_uppercase() {
            out.push('_');
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// CLDR's plural categories, as Fluent names them and as `Plural` does.
const PLURALS: [(&str, &str); 6] = [
    ("zero", "Zero"),
    ("one", "One"),
    ("two", "Two"),
    ("few", "Few"),
    ("many", "Many"),
    ("other", "Other"),
];

#[derive(Clone, Copy, PartialEq)]
enum Shape {
    Fixed,
    Text,
    Rich,
}

fn generate(languages: &[Messages]) -> String {
    let mut out =
        String::from("// Generated by build.rs from locales/*/site.ftl. Do not edit.\n\n");
    for (id, english) in &languages[0] {
        let mut vars = Vec::new();
        for messages in languages {
            variables(&messages[id], &mut vars);
        }
        // English's order first, then any number flag a translation adds.
        let mut ordered = Vec::new();
        variables(english, &mut ordered);
        for (name, number) in &mut ordered {
            *number |= vars.iter().any(|(n, num)| n == name && *num);
        }
        let rich = languages
            .iter()
            .any(|messages| !tags(&messages[id]).is_empty());
        let shape = if rich {
            Shape::Rich
        } else if ordered.is_empty() {
            Shape::Fixed
        } else {
            Shape::Text
        };
        let arms: Vec<String> = LANGS
            .iter()
            .zip(languages)
            .map(|(_, messages)| body(&messages[id], shape))
            .collect();
        let same = arms.iter().all(|arm| arm == &arms[0]);
        // The language is read to pick an arm, to write a number or to
        // choose a plural; a message that does none of these ignores it.
        let reads = arms[0].contains("(lang, ");
        let lang = if same && !reads { "_lang" } else { "lang" };
        let mut params = format!("{lang}: Lang");
        for (name, number) in &ordered {
            let _ = write!(params, ", {name}: {}", if *number { "u64" } else { "&str" });
        }
        let returns = match shape {
            Shape::Fixed => "&'static str",
            Shape::Text => "String",
            Shape::Rich => "Vec<Piece>",
        };
        let _ = writeln!(out, "/// The message `{id}`.\n#[must_use]");
        let _ = writeln!(out, "pub fn {}({params}) -> {returns} {{", snake(id));
        if same {
            let _ = writeln!(out, "    {}", arms[0]);
        } else {
            // Languages that say the same share an arm (a name, a number).
            let mut grouped: Vec<(Vec<&str>, &String)> = Vec::new();
            for ((_, variant, _), arm) in LANGS.iter().zip(&arms) {
                match grouped.iter_mut().find(|(_, kept)| *kept == arm) {
                    Some((variants, _)) => variants.push(variant),
                    None => grouped.push((vec![variant], arm)),
                }
            }
            let _ = writeln!(out, "    match lang {{");
            for (variants, arm) in grouped {
                let pattern: Vec<String> = variants.iter().map(|v| format!("Lang::{v}")).collect();
                let _ = writeln!(out, "        {} => {arm},", pattern.join(" | "));
            }
            let _ = writeln!(out, "    }}");
        }
        let _ = writeln!(out, "}}\n");
    }
    out
}

/// One language's expression for a message (it may read `lang`).
fn body(nodes: &[Node], shape: Shape) -> String {
    if shape == Shape::Fixed {
        let mut text = String::new();
        for node in nodes {
            if let Node::Text(t) = node {
                text.push_str(t);
            }
        }
        return format!("{text:?}");
    }
    let mut code = String::from("{ let mut s = String::new(); ");
    push_nodes(nodes, shape == Shape::Rich, &mut code);
    code.push_str(if shape == Shape::Rich {
        "pieces(&s) }"
    } else {
        "s }"
    });
    code
}

fn push_nodes(nodes: &[Node], rich: bool, code: &mut String) {
    for node in nodes {
        match node {
            Node::Text(text) => {
                let mut chars = text.chars();
                match (chars.next(), chars.next()) {
                    (Some(c), None) => {
                        let _ = write!(code, "s.push({c:?}); ");
                    }
                    _ => {
                        let _ = write!(code, "s.push_str({text:?}); ");
                    }
                }
            }
            Node::Var(name) => {
                // A number written where the variable is also selected on.
                let _ = write!(code, "s.push_str(&arg(lang, &{name}, {rich})); ");
            }
            Node::Select {
                var,
                variants,
                default,
            } => {
                // Exact values first, as an if-chain; then the plural
                // categories as an exhaustive match without a wildcard.
                let mut exact = false;
                for (key, pattern) in variants {
                    if let Key::Exact(n) = key {
                        let _ = write!(code, "if {var} == {n} {{ ");
                        push_nodes(pattern, rich, code);
                        code.push_str("} else ");
                        exact = true;
                    }
                }
                if exact {
                    code.push_str("{ ");
                }
                let categories: Vec<(&str, &Vec<Node>)> = variants
                    .iter()
                    .enumerate()
                    .filter_map(|(index, (key, pattern))| match key {
                        Key::Category(name) if index != *default => Some((name.as_str(), pattern)),
                        _ => None,
                    })
                    .collect();
                // Each category's text, the default's where the selector
                // names none for it; categories that read the same share an
                // arm, and one arm for all of them is no match at all.
                let mut arms: Vec<(Vec<&str>, String)> = Vec::new();
                for (name, rust) in PLURALS {
                    let pattern = categories
                        .iter()
                        .find(|(n, _)| *n == name)
                        .map_or(&variants[*default].1, |(_, pattern)| *pattern);
                    let mut text = String::new();
                    push_nodes(pattern, rich, &mut text);
                    match arms.iter_mut().find(|(_, kept)| *kept == text) {
                        Some((names, _)) => names.push(rust),
                        None => arms.push((vec![rust], text)),
                    }
                }
                if let [(_, only)] = arms.as_slice() {
                    code.push_str(only);
                } else {
                    let _ = write!(code, "match plural(lang, {var}) {{ ");
                    for (names, text) in &arms {
                        let pattern: Vec<String> =
                            names.iter().map(|n| format!("Plural::{n}")).collect();
                        let _ = write!(code, "{} => {{ {text}}} ", pattern.join(" | "));
                    }
                    code.push_str("} ");
                }
                if exact {
                    code.push_str("} ");
                }
            }
        }
    }
}
