//! The Markdown subset of the legal texts (`legal/README.md`), read as the
//! Baylee client reads it, so a text looks alike on the site and in the
//! game: `#` and `##`…`######` headings, paragraphs (lines joined until a
//! blank line), `-`/`*`/`+` and `1.`/`1)` list items (an indented line
//! continues one), `---`/`***`/`___` rules, `**bold**`, `*italic*` and
//! `_italic_` (an `_` inside a word is a letter), backslash escapes,
//! `[label](url)` and `<https://…>`/`<mailto:…>`. Comment lines are dropped.
//! No HTML is ever interpreted: everything else is text. Only `https:` and
//! `mailto:` addresses become links; any other stays text beside its label.

use std::fmt::Write as _;

use leptos::prelude::*;

/// A run of text with one style.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    /// The text.
    pub text: String,
    /// Bold.
    pub bold: bool,
    /// Italic.
    pub italic: bool,
    /// A link's address, `https:` or `mailto:` only.
    pub href: Option<String>,
}

/// A block of a legal text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    /// A heading: level 1 for `#`, 2 for the rest.
    Heading(u8, Vec<Span>),
    /// A paragraph.
    Paragraph(Vec<Span>),
    /// A list item, numbered or not.
    Item(Option<u32>, Vec<Span>),
    /// A horizontal rule.
    Rule,
}

fn is_rule(line: &str) -> bool {
    let compact: String = line.chars().filter(|c| !c.is_whitespace()).collect();
    compact.len() >= 3
        && ['-', '*', '_']
            .iter()
            .any(|&c| compact.chars().all(|x| x == c))
}

fn heading(line: &str) -> Option<(u8, &str)> {
    let hashes = line.bytes().take_while(|&b| b == b'#').count();
    if hashes == 0 || hashes > 6 || line.as_bytes().get(hashes) != Some(&b' ') {
        return None;
    }
    let rest = line[hashes + 1..].trim().trim_end_matches('#').trim();
    Some((if hashes == 1 { 1 } else { 2 }, rest))
}

fn ordered(line: &str) -> Option<(u32, &str)> {
    let digits = line.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 || digits > 9 {
        return None;
    }
    let rest = &line[digits..];
    let rest = rest
        .strip_prefix(". ")
        .or_else(|| rest.strip_prefix(") "))?;
    Some((line[..digits].parse().ok()?, rest.trim()))
}

fn bullet(line: &str) -> Option<&str> {
    ["- ", "* ", "+ "]
        .iter()
        .find_map(|p| line.strip_prefix(p))
        .map(str::trim)
}

fn safe_href(url: &str) -> Option<String> {
    (url.starts_with("https://") || url.starts_with("mailto:")).then(|| url.to_owned())
}

/// Spans being collected: the current styles and the text since the last
/// change of style.
#[derive(Default)]
struct Spans {
    done: Vec<Span>,
    word: String,
    bold: bool,
    italic: bool,
}

impl Spans {
    /// Ends the current word, as a link when `href` is given.
    fn push(&mut self, href: Option<String>) {
        if self.word.is_empty() {
            return;
        }
        if let Some(last) = self.done.last_mut()
            && last.bold == self.bold
            && last.italic == self.italic
            && last.href.is_none()
            && href.is_none()
        {
            last.text.push_str(&self.word);
            self.word.clear();
            return;
        }
        let text = std::mem::take(&mut self.word);
        self.done.push(Span {
            text,
            bold: self.bold,
            italic: self.italic,
            href,
        });
    }
}

/// `[label](url)` starting at `at`: the label, the address and the index
/// after the closing parenthesis.
fn link_at(chars: &[char], at: usize) -> Option<(String, String, usize)> {
    let close = chars[at + 1..].iter().position(|&x| x == ']')? + at + 1;
    if chars.get(close + 1) != Some(&'(') {
        return None;
    }
    let end = chars[close + 2..].iter().position(|&x| x == ')')? + close + 2;
    let label: String = chars[at + 1..close].iter().collect();
    let url = chars[close + 2..end]
        .iter()
        .collect::<String>()
        .trim()
        .to_owned();
    Some((label, url, end + 1))
}

/// `<https://…>` or `<mailto:…>` starting at `at`: the address and the
/// index after the `>`.
fn autolink_at(chars: &[char], at: usize) -> Option<(String, usize)> {
    let end = chars[at + 1..].iter().position(|&x| x == '>')? + at + 1;
    let inner: String = chars[at + 1..end].iter().collect();
    ["http://", "https://", "mailto:"]
        .iter()
        .any(|scheme| inner.starts_with(scheme))
        .then_some((inner, end + 1))
}

/// The inline styles of one line of text.
#[must_use]
pub fn inline(text: &str) -> Vec<Span> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Spans::default();
    let alnum = |c: Option<&char>| c.is_some_and(|c| c.is_alphanumeric());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1);
        if c == '\\' && next.is_some_and(char::is_ascii_punctuation) {
            out.word.push(chars[i + 1]);
            i += 2;
        } else if c == '*' && next == Some(&'*') {
            out.push(None);
            out.bold = !out.bold;
            i += 2;
        } else if (c == '*' || c == '_')
            && (c == '*' || !(i > 0 && alnum(chars.get(i - 1)) && alnum(next)))
        {
            out.push(None);
            out.italic = !out.italic;
            i += 1;
        } else if let Some((label, url, after)) = (c == '[').then(|| link_at(&chars, i)).flatten() {
            out.push(None);
            out.word.push_str(&label);
            if let Some(href) = safe_href(&url) {
                out.push(Some(href));
            } else if !url.is_empty() && url != label {
                let _ = write!(out.word, " ({url})");
            }
            i = after;
        } else if let Some((inner, after)) = (c == '<').then(|| autolink_at(&chars, i)).flatten() {
            out.push(None);
            out.word.push_str(&inner);
            if let Some(href) = safe_href(&inner) {
                out.push(Some(href));
            }
            i = after;
        } else {
            out.word.push(c);
            i += 1;
        }
    }
    out.push(None);
    out.done
}

/// The text of `markdown` as blocks.
#[must_use]
pub fn parse(markdown: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut paragraph: Vec<&str> = Vec::new();
    let mut item: Option<(Option<u32>, Vec<&str>)> = None;
    let flush = |blocks: &mut Vec<Block>,
                 paragraph: &mut Vec<&str>,
                 item: &mut Option<(Option<u32>, Vec<&str>)>| {
        if !paragraph.is_empty() {
            blocks.push(Block::Paragraph(inline(&paragraph.join(" "))));
            paragraph.clear();
        }
        if let Some((number, lines)) = item.take() {
            blocks.push(Block::Item(number, inline(&lines.join(" "))));
        }
    };
    for raw in markdown.lines() {
        let line = raw.trim();
        if line.is_empty() || (line.starts_with("<!--") && line.ends_with("-->")) {
            flush(&mut blocks, &mut paragraph, &mut item);
            continue;
        }
        if item.is_some()
            && raw.starts_with([' ', '\t'])
            && bullet(line).is_none()
            && ordered(line).is_none()
        {
            if let Some((_, lines)) = item.as_mut() {
                lines.push(line);
            }
            continue;
        }
        if let Some((level, text)) = heading(line) {
            flush(&mut blocks, &mut paragraph, &mut item);
            blocks.push(Block::Heading(level, inline(text)));
        } else if is_rule(line) {
            flush(&mut blocks, &mut paragraph, &mut item);
            blocks.push(Block::Rule);
        } else if let Some(text) = bullet(line) {
            flush(&mut blocks, &mut paragraph, &mut item);
            item = Some((None, vec![text]));
        } else if let Some((number, text)) = ordered(line) {
            flush(&mut blocks, &mut paragraph, &mut item);
            item = Some((Some(number), vec![text]));
        } else if let Some((_, lines)) = item.as_mut() {
            lines.push(line);
        } else {
            paragraph.push(line);
        }
    }
    flush(&mut blocks, &mut paragraph, &mut item);
    blocks
}

fn spans_view(spans: Vec<Span>) -> AnyView {
    spans
        .into_iter()
        .map(|span| {
            let text = match (span.bold, span.italic) {
                (true, true) => view! { <strong><em>{span.text}</em></strong> }.into_any(),
                (true, false) => view! { <strong>{span.text}</strong> }.into_any(),
                (false, true) => view! { <em>{span.text}</em> }.into_any(),
                (false, false) => span.text.into_any(),
            };
            match span.href {
                Some(href) if href.starts_with("https://baylee.app/") => {
                    view! { <a href=href>{text}</a> }.into_any()
                }
                Some(href) => {
                    view! { <a href=href target="_blank" rel="noopener noreferrer">{text}</a> }
                        .into_any()
                }
                None => text,
            }
        })
        .collect_view()
        .into_any()
}

/// A legal text drawn as HTML: its `#` heading is left out (the page has
/// its own `h1`), `##` become `h2`, list items are grouped into lists.
#[must_use]
pub fn render(markdown: &str) -> AnyView {
    let mut out: Vec<AnyView> = Vec::new();
    let mut list: Vec<AnyView> = Vec::new();
    let mut numbered: Option<u32> = None;
    let close = |out: &mut Vec<AnyView>, list: &mut Vec<AnyView>, numbered: &mut Option<u32>| {
        if list.is_empty() {
            return;
        }
        let items = std::mem::take(list);
        out.push(match numbered.take() {
            Some(start) => view! { <ol start=start.to_string()>{items}</ol> }.into_any(),
            None => view! { <ul>{items}</ul> }.into_any(),
        });
    };
    for block in parse(markdown) {
        match block {
            Block::Item(number, spans) => {
                if list.is_empty() {
                    numbered = number;
                }
                list.push(view! { <li>{spans_view(spans)}</li> }.into_any());
            }
            other => {
                close(&mut out, &mut list, &mut numbered);
                match other {
                    Block::Heading(1, _) | Block::Item(..) => {}
                    Block::Heading(_, spans) => {
                        out.push(view! { <h2>{spans_view(spans)}</h2> }.into_any());
                    }
                    Block::Paragraph(spans) => {
                        out.push(view! { <p>{spans_view(spans)}</p> }.into_any());
                    }
                    Block::Rule => out.push(view! { <hr /> }.into_any()),
                }
            }
        }
    }
    close(&mut out, &mut list, &mut numbered);
    out.into_iter().collect_view().into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_styles_and_links() {
        let spans =
            inline("A **bold** _it_ snake_case [x](https://a.b) [y](javascript:alert(1)) \\*no\\*");
        let texts: Vec<(&str, bool, bool, Option<&str>)> = spans
            .iter()
            .map(|s| (s.text.as_str(), s.bold, s.italic, s.href.as_deref()))
            .collect();
        assert_eq!(
            texts,
            [
                ("A ", false, false, None),
                ("bold", true, false, None),
                (" ", false, false, None),
                ("it", false, true, None),
                (" snake_case ", false, false, None),
                ("x", false, false, Some("https://a.b")),
                (" y (javascript:alert(1)) *no*", false, false, None),
            ]
        );
    }

    #[test]
    fn blocks() {
        let text = "<!-- version: 1 -->\n# Title\n\nPara\nline\n\n- one\n  more\n- two\n\n1. first\n---\n## Sub";
        assert_eq!(
            parse(text),
            [
                Block::Heading(1, inline("Title")),
                Block::Paragraph(inline("Para line")),
                Block::Item(None, inline("one more")),
                Block::Item(None, inline("two")),
                Block::Item(Some(1), inline("first")),
                Block::Rule,
                Block::Heading(2, inline("Sub")),
            ]
        );
    }
}
