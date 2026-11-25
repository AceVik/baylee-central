//! `#CardName` (`#Card#set#nr#foil`) and `@UserHandle` mention detection for the
//! chat composer plus safe, Trusted-Types-clean Leptos chip rendering for
//! message text (zero `innerHTML`, zero inline `style`).

use central_api::social::{CardPrinting, FINISH_ETCHED, FINISH_FOIL, FINISH_NONFOIL, Person};
use leptos::prelude::*;

use super::super::ui::copy_handle::player_path;
use super::glyph::{self, glyph};
use super::model::{self, tag_of, tag_text};
use super::preview::{PreviewCtl, PreviewTarget};
use super::wire::{encode_query_component, uuid_any};
use crate::words::{self, Lang};

/// Which autocomplete kind is currently active at the composer cursor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriggerKind {
    /// `#CardName` (or `#CardName#set#nr#finish`) lookup against Central's card catalog.
    Card,
    /// `@User` lookup against conversation members and contacts.
    User,
}

/// Visual finish of a mentioned card (`Normal`, `Foil`, `Etched`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CardFinish {
    /// Standard non-foil printing.
    #[default]
    Normal,
    /// Traditional foil treatment (`print_finish == 1`).
    Foil,
    /// Foil-etched metallic treatment (`print_finish == 2`).
    Etched,
}

impl CardFinish {
    /// `data-finish` HTML attribute value when non-normal (`"foil"` / `"etched"`).
    #[must_use]
    pub const fn attr(self) -> Option<&'static str> {
        match self {
            Self::Normal => None,
            Self::Foil => Some("foil"),
            Self::Etched => Some("etched"),
        }
    }

    /// Canonical token suffix inserted in raw chat syntax (`""`, `"#foil"`, `"#etched"`).
    #[must_use]
    pub const fn suffix(self) -> &'static str {
        match self {
            Self::Normal => "",
            Self::Foil => "#foil",
            Self::Etched => "#etched",
        }
    }

    /// Corresponding `CardPrinting::finishes` bitmask constant.
    #[must_use]
    pub const fn bit(self) -> u8 {
        match self {
            Self::Normal => FINISH_NONFOIL,
            Self::Foil => FINISH_FOIL,
            Self::Etched => FINISH_ETCHED,
        }
    }

    /// Parses an exact finish keyword (`foil`, `etched`, `normal`, `nonfoil`).
    #[must_use]
    pub fn from_keyword(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "foil" => Some(Self::Foil),
            "etched" => Some(Self::Etched),
            "normal" | "nonfoil" | "plain" => Some(Self::Normal),
            _ => None,
        }
    }

    /// Parses a prefix while typing `#foil`, `#etched`, or `#normal` in `#Card#...`.
    #[must_use]
    pub fn from_prefix(raw: &str) -> Option<Self> {
        let lower = raw.trim().to_ascii_lowercase();
        if lower.len() >= 2 && "foil".starts_with(&lower) {
            Some(Self::Foil)
        } else if lower.len() >= 2 && "etched".starts_with(&lower) {
            Some(Self::Etched)
        } else if lower.len() >= 3
            && ("normal".starts_with(&lower) || "nonfoil".starts_with(&lower))
        {
            Some(Self::Normal)
        } else {
            None
        }
    }
}

/// Structured query parsed out of `#Name#set#collector#finish` while typing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardTriggerQuery {
    /// Card name search prefix/substring.
    pub name_query: String,
    /// Optional set code or set name filter.
    pub set_filter: Option<String>,
    /// Optional collector number filter.
    pub collector_filter: Option<String>,
    /// Optional protocol finish filter (`foil`, `etched`, `nonfoil`).
    pub finish_filter: Option<String>,
    /// Preferred visual finish inferred from typed modifiers.
    pub preferred_finish: Option<CardFinish>,
}

fn classify_card_modifier(
    token: &str,
    set_filter: &mut Option<String>,
    collector_filter: &mut Option<String>,
    preferred_finish: &mut Option<CardFinish>,
) {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return;
    }
    if let Some(finish) = CardFinish::from_prefix(trimmed) {
        *preferred_finish = Some(finish);
        return;
    }
    let only_digits = trimmed.bytes().all(|b| b.is_ascii_digit());
    let looks_collector = set_filter.is_some()
        && trimmed.bytes().next().is_some_and(|b| b.is_ascii_digit())
        && trimmed.len() <= 10;
    if only_digits || looks_collector {
        *collector_filter = Some(trimmed.to_ascii_lowercase());
    } else if set_filter.is_none() {
        *set_filter = Some(trimmed.to_ascii_lowercase());
    }
}

/// Parses `#CardName#Edition#Collector#Foil` style composer trigger queries.
#[must_use]
pub fn parse_card_trigger(raw: &str) -> CardTriggerQuery {
    let mut parts = raw.split('#');
    let name_query = parts.next().unwrap_or_default().trim().to_owned();
    let mut set_filter = None;
    let mut collector_filter = None;
    let mut preferred_finish = None;

    for raw_mod in parts {
        let m = raw_mod.trim();
        if m.is_empty() {
            continue;
        }
        // Also support `#2x2 foil` or `#lea etched` typed with a space inside one `#` segment.
        if let Some((head, tail)) = m.rsplit_once(' ')
            && let Some(finish) = CardFinish::from_keyword(tail)
        {
            preferred_finish = Some(finish);
            classify_card_modifier(
                head,
                &mut set_filter,
                &mut collector_filter,
                &mut preferred_finish,
            );
            continue;
        }
        classify_card_modifier(
            m,
            &mut set_filter,
            &mut collector_filter,
            &mut preferred_finish,
        );
    }

    let finish_filter = preferred_finish.map(|f| {
        match f {
            CardFinish::Normal => "nonfoil",
            CardFinish::Foil => "foil",
            CardFinish::Etched => "etched",
        }
        .to_owned()
    });

    CardTriggerQuery {
        name_query,
        set_filter,
        collector_filter,
        finish_filter,
        preferred_finish,
    }
}

/// An active `#` or `@` mention trigger around the composer caret.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trigger {
    /// Card or user trigger.
    pub kind: TriggerKind,
    /// Byte index of the `#` or `@` sigil in `text`.
    pub start: usize,
    /// Byte index of the caret (`end > start`).
    pub end: usize,
    /// The raw query typed after `#` or `@`.
    pub query: String,
}

fn is_boundary_before(ch: char) -> bool {
    ch.is_whitespace() || matches!(ch, '(' | '[' | '{' | '"' | '\'' | ',' | ':' | ';')
}

fn clamp_char_boundary(text: &str, mut index: usize) -> usize {
    index = index.min(text.len());
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

/// Finds an active `#query` (including `#Name#set#nr#foil`) or `@query`
/// trigger immediately ending at `cursor_bytes`.
#[must_use]
pub fn active_trigger(text: &str, cursor_bytes: usize) -> Option<Trigger> {
    let end = clamp_char_boundary(text, cursor_bytes);
    let prefix = &text[..end];
    let mut spaces = 0_usize;
    let mut seen_chars = 0_usize;

    for (idx, ch) in prefix.char_indices().rev() {
        if matches!(ch, '\n' | '\r' | '[' | ']') {
            return None;
        }
        if ch == '@' {
            let boundary = prefix[..idx]
                .chars()
                .next_back()
                .is_none_or(is_boundary_before);
            if !boundary {
                return None;
            }
            let query = &prefix[idx + 1..];
            if query.contains('#') || spaces > 1 {
                return None;
            }
            return Some(Trigger {
                kind: TriggerKind::User,
                start: idx,
                end,
                query: query.to_owned(),
            });
        }
        if ch == '#' {
            let boundary = prefix[..idx]
                .chars()
                .next_back()
                .is_none_or(is_boundary_before);
            if boundary {
                let query = &prefix[idx + 1..];
                if query.is_empty() || query.starts_with(char::is_whitespace) || spaces > 5 {
                    return None;
                }
                return Some(Trigger {
                    kind: TriggerKind::Card,
                    start: idx,
                    end,
                    query: query.to_owned(),
                });
            }
            // Internal `#` separator inside `#CardName#set#nr#foil` or `@Name#tag`:
            // keep scanning left to find the boundary sigil.
        }
        if ch.is_whitespace() {
            spaces += 1;
            if spaces > 5 {
                return None;
            }
        }
        seen_chars += 1;
        if seen_chars > 64 {
            return None;
        }
    }
    None
}

/// Replaces `trigger` inside `text` with `replacement` (appending a trailing
/// space when the following character is not already whitespace) and returns
/// `(new_text, new_caret_byte_index)`.
#[must_use]
pub fn replace_trigger(text: &str, trigger: &Trigger, replacement: &str) -> (String, usize) {
    let before = &text[..trigger.start];
    let after = &text[trigger.end..];
    let needs_space = after.chars().next().is_none_or(|c| !c.is_whitespace());
    let mut out = String::with_capacity(
        before.len() + replacement.len() + usize::from(needs_space) + after.len(),
    );
    out.push_str(before);
    out.push_str(replacement);
    if needs_space {
        out.push(' ');
    }
    let caret = out.len();
    out.push_str(after);
    (out, caret)
}

/// Formats a clean card mention token inserted into the composer:
/// `[Name]`, `[Name (SET #NR)]`, `[Name (SET #NR lang)]`, `[Name#foil]`,
/// or `[Name (SET #NR)#foil]`.
#[must_use]
pub fn card_replacement(
    card: &str,
    en_name: &str,
    printing: Option<&CardPrinting>,
    finish: CardFinish,
) -> String {
    let suffix = finish.suffix();
    if let Some(p) = printing {
        let set_upper = p.set_code.to_ascii_uppercase();
        let explicit_lang =
            if !p.lang.eq_ignore_ascii_case("en") && card.eq_ignore_ascii_case(en_name) {
                format!(" {}", p.lang.to_ascii_lowercase())
            } else {
                String::new()
            };
        format!(
            "[{card} ({set_upper} #{}{explicit_lang}){suffix}]",
            p.collector_number
        )
    } else {
        format!("[{card}{suffix}]")
    }
}

/// Formats a user mention token inserted into the composer (`[@Name#tag]`).
#[must_use]
pub fn user_replacement(person: &Person) -> String {
    format!("[@{}#{}]", person.display_name, tag_text(person.tag))
}

/// Parsed metadata for one card mention token in a message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardMention {
    /// Rendered chip text (never contains `"Foil"` or `"Etched"`).
    pub label: String,
    /// Card name (localized or English) for catalog lookup and Scryfall URLs.
    pub name: String,
    /// Optional lowercase Scryfall set code (e.g. `2x2`, `lea`).
    pub set: Option<String>,
    /// Optional collector number (e.g. `117`, `162a`).
    pub collector: Option<String>,
    /// Optional explicit Scryfall language tag encoded in `(SET #NR lang)`.
    pub lang: Option<String>,
    /// Selected visual finish (`Normal`, `Foil`, `Etched`).
    pub finish: CardFinish,
}

/// A parsed segment of a chat message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Segment {
    /// Ordinary plain text.
    Text(String),
    /// A clickable `@Name#tag` player mention.
    User {
        /// Display name.
        name: String,
        /// Lowercase canonical hex tag without `#`.
        tag: String,
    },
    /// A clickable MTG card mention (optionally edition- & finish-tagged).
    Card(CardMention),
}

fn push_text(out: &mut Vec<Segment>, chunk: &str) {
    if chunk.is_empty() {
        return;
    }
    if let Some(Segment::Text(prev)) = out.last_mut() {
        prev.push_str(chunk);
    } else {
        out.push(Segment::Text(chunk.to_owned()));
    }
}

fn valid_card_name(name: &str) -> bool {
    let count = name.chars().count();
    (2..=80).contains(&count)
        && name.chars().next().is_some_and(char::is_alphanumeric)
        && name.chars().any(char::is_alphabetic)
}

type PrintSuffix = (String, String, Option<String>);

fn valid_lang_tag(raw: &str) -> bool {
    (2..=6).contains(&raw.len())
        && raw
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_uppercase() || b == b'-')
}

fn known_scryfall_lang(raw: &str) -> bool {
    matches!(
        raw.to_ascii_lowercase().as_str(),
        "de" | "en" | "fr" | "it" | "es" | "pt" | "ru" | "ja" | "ko" | "zhs" | "zht"
    )
}

fn parse_print_suffix(body: &str) -> (&str, Option<PrintSuffix>) {
    let trimmed = body.trim_end();
    if let Some(no_paren) = trimmed.strip_suffix(')')
        && let Some((head, inside)) = no_paren.rsplit_once(" (")
        && let Some((set_raw, col_and_lang)) = inside.split_once('#')
    {
        let set_s = set_raw.trim();
        let rest = col_and_lang.trim();
        let (col_s, lang_opt) = match rest.split_once(char::is_whitespace) {
            Some((c, l)) => {
                let l_trim = l.trim();
                let parsed_lang = valid_lang_tag(l_trim).then(|| l_trim.to_ascii_lowercase());
                (c.trim(), parsed_lang)
            }
            None => (rest, None),
        };
        if (1..=12).contains(&set_s.len())
            && set_s.bytes().all(|b| b.is_ascii_alphanumeric())
            && (1..=16).contains(&col_s.len())
            && !col_s.chars().any(char::is_whitespace)
        {
            return (
                head.trim(),
                Some((set_s.to_ascii_lowercase(), col_s.to_owned(), lang_opt)),
            );
        }
    }
    (trimmed, None)
}

/// Parses the interior of `[Card Name]`, `[Card (SET #NR)#foil]`,
/// `[Card (SET #NR de)#foil]`, or `[Card#etched]`.
#[must_use]
pub fn parse_card_spec(raw: &str) -> Option<CardMention> {
    let mut body = raw.trim();
    let mut finish = CardFinish::Normal;
    if let Some((head, tag)) = body.rsplit_once('#')
        && let Some(parsed) = CardFinish::from_keyword(tag)
        && !head.ends_with(' ')
        && !head.trim_end().ends_with(" (")
    {
        finish = parsed;
        body = head.trim();
    }
    let (name, print) = parse_print_suffix(body);
    if !valid_card_name(name) {
        return None;
    }
    let (set, collector, lang) = match print {
        Some((s, c, l)) => (Some(s), Some(c), l),
        None => (None, None, None),
    };
    Some(CardMention {
        label: name.to_owned(),
        name: name.to_owned(),
        set,
        collector,
        lang,
        finish,
    })
}

fn parse_bracket(rest: &str) -> Option<(Segment, usize)> {
    // Support both `[Card Name]` and `[[Card Name]]` conventions as well as `[@Name#tag]`.
    let double = rest.starts_with("[[");
    let open_len = if double { 2 } else { 1 };
    let close_pat = if double { "]]" } else { "]" };
    let inner_rest = &rest[open_len..];
    let end_rel = inner_rest.find(close_pat)?;
    let inner = &inner_rest[..end_rel];
    let consumed = open_len + end_rel + close_pat.len();
    if inner.is_empty() || inner.contains(['[', ']', '\n', '\r']) {
        return None;
    }
    if let Some(user_body) = inner.strip_prefix('@') {
        let (name, raw_tag) = user_body.rsplit_once('#')?;
        let tag = tag_of(raw_tag)?;
        let hex = tag_text(tag);
        if raw_tag == hex && central_api::profile::valid_display_name(name) {
            return Some((
                Segment::User {
                    name: name.to_owned(),
                    tag: hex,
                },
                consumed,
            ));
        }
        return None;
    }
    parse_card_spec(inner).map(|card| (Segment::Card(card), consumed))
}

fn parse_bare_user(rest: &str) -> Option<(Segment, usize)> {
    let body = rest.strip_prefix('@')?;
    let mut hash_at = None;
    for (i, ch) in body.char_indices() {
        if ch == '#' {
            hash_at = Some(i);
            break;
        }
        if ch.is_whitespace() || matches!(ch, '[' | ']' | '@' | '\n' | '\r') || i >= 48 {
            return None;
        }
    }
    let hash_idx = hash_at?;
    let name = &body[..hash_idx];
    if name.is_empty() || !central_api::profile::valid_display_name(name) {
        return None;
    }
    let after_hash = &body[hash_idx + 1..];
    let hex_len = after_hash
        .bytes()
        .take_while(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b))
        .count();
    let raw_tag = &after_hash[..hex_len];
    let tag = tag_of(raw_tag)?;
    let hex = tag_text(tag);
    if raw_tag != hex {
        return None;
    }
    if after_hash[hex_len..]
        .chars()
        .next()
        .is_some_and(char::is_alphanumeric)
    {
        return None;
    }
    Some((
        Segment::User {
            name: name.to_owned(),
            tag: hex,
        },
        1 + hash_idx + 1 + hex_len,
    ))
}

fn parse_bare_card(rest: &str) -> Option<(Segment, usize)> {
    let body = rest.strip_prefix('#')?;
    let mut total_len = 0_usize;
    for (i, ch) in body.char_indices() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '\'' | '_' | '#') {
            total_len = i + ch.len_utf8();
            if total_len > 64 {
                return None;
            }
        } else {
            break;
        }
    }
    let full = body[..total_len].trim_end_matches('#');
    if full.is_empty() {
        return None;
    }
    let mut parts = full.split('#');
    let word = parts.next()?;
    let letters = word.bytes().filter(u8::is_ascii_alphabetic).count();
    if !(2..=36).contains(&word.len()) || letters < 2 {
        return None;
    }
    // Never mistake a bare `#1a2b` player tag for a card mention.
    if tag_of(word).is_some() {
        return None;
    }
    let display = word.replace('_', " ");
    let mut set = None;
    let mut collector = None;
    let mut lang = None;
    let mut finish = CardFinish::Normal;
    for mod_part in parts {
        if mod_part.is_empty() {
            continue;
        }
        if let Some(f) = CardFinish::from_keyword(mod_part) {
            finish = f;
        } else if collector.is_some() && lang.is_none() && known_scryfall_lang(mod_part) {
            lang = Some(mod_part.to_ascii_lowercase());
        } else if mod_part.bytes().all(|b| b.is_ascii_digit())
            || (set.is_some() && mod_part.bytes().next().is_some_and(|b| b.is_ascii_digit()))
        {
            collector = Some(mod_part.to_ascii_lowercase());
        } else if set.is_none() {
            set = Some(mod_part.to_ascii_lowercase());
        }
    }
    Some((
        Segment::Card(CardMention {
            label: display.clone(),
            name: display,
            set,
            collector,
            lang,
            finish,
        }),
        1 + full.len(),
    ))
}

/// Parses plain message text into plain-text, `@User`, and `#Card`/`[Card]`
/// segments without producing any HTML strings.
#[must_use]
pub fn parse_segments(text: &str) -> Vec<Segment> {
    let mut out = Vec::new();
    let mut i = 0_usize;
    let mut plain_start = 0_usize;

    while i < text.len() {
        let rest = &text[i..];
        let prev = text[..i].chars().next_back();
        let boundary = prev.is_none_or(is_boundary_before);
        let parsed = if rest.starts_with('[') {
            parse_bracket(rest)
        } else if rest.starts_with('@') && boundary {
            parse_bare_user(rest)
        } else if rest.starts_with('#') && boundary {
            parse_bare_card(rest)
        } else {
            None
        };

        if let Some((seg, consumed)) = parsed {
            push_text(&mut out, &text[plain_start..i]);
            out.push(seg);
            i += consumed;
            plain_start = i;
        } else if let Some(ch) = rest.chars().next() {
            i += ch.len_utf8();
        } else {
            break;
        }
    }
    push_text(&mut out, &text[plain_start..]);
    out
}

/// Flattens a chat message into clean preview text (hiding `[...]` brackets,
/// `#foil`/`#etched` suffixes, and set codes) for conversation list snippets.
#[must_use]
pub fn plain_preview(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for seg in parse_segments(text) {
        match seg {
            Segment::Text(s) => out.push_str(&s),
            Segment::User { name, tag } => {
                out.push('@');
                out.push_str(&name);
                out.push('#');
                out.push_str(&tag);
            }
            Segment::Card(card) => out.push_str(&card.label),
        }
    }
    out
}

/// Builds a safe exact-match Scryfall search URL for `card`.
#[must_use]
pub fn scryfall_url(card: &str) -> String {
    let exact = format!("!\"{card}\"");
    format!(
        "https://scryfall.com/search?q={}",
        encode_query_component(&exact)
    )
}

/// Builds a safe Scryfall page URL for a [`CardMention`], linking straight to
/// `/card/{set}/{collector}` when an exact print was chosen.
#[must_use]
pub fn scryfall_page_url(card: &CardMention) -> String {
    if let (Some(set), Some(col)) = (card.set.as_deref(), card.collector.as_deref()) {
        return format!(
            "https://scryfall.com/card/{}/{}",
            encode_query_component(set),
            encode_query_component(col)
        );
    }
    if let Some(set) = card.set.as_deref() {
        let exact = format!("!\"{}\" set:{set}", card.name);
        return format!(
            "https://scryfall.com/search?q={}",
            encode_query_component(&exact)
        );
    }
    scryfall_url(&card.name)
}

/// Validates a Scryfall printing UUID and returns its unaltered front-face
/// `cards.scryfall.io` image URL (`488×680` JPG).
#[must_use]
pub fn scryfall_image_url(uuid: &str) -> Option<String> {
    if !uuid_any(uuid) {
        return None;
    }
    let a = uuid.get(0..1)?;
    let b = uuid.get(1..2)?;
    Some(format!(
        "https://cards.scryfall.io/normal/front/{a}/{b}/{uuid}.jpg"
    ))
}

fn render_user_chip(
    lang: Lang,
    name: String,
    tag: String,
    own_tag: u32,
    preview: Option<PreviewCtl>,
) -> AnyView {
    let href = player_path(lang, &name, &tag);
    let is_self = model::tag_of(&tag) == Some(own_tag);
    let target = PreviewTarget::User {
        name: name.clone(),
        tag_hex: tag.clone(),
    };
    let t_enter = target.clone();
    let t_focus = target.clone();
    let t_click = target;
    let prefix = "@";
    let tag_sigil = "#";
    view! {
        <a
            class="chat-mention chat-mention-user"
            href=href
            target="_blank"
            rel="noopener noreferrer"
            data-self=is_self.then_some("true")
            aria-label=words::player_open_profile(lang)
            translate="no"
            on:pointerdown=move |ev| {
                if let Some(p) = preview {
                    p.on_pointer_down(&ev);
                }
            }
            on:pointerenter=move |ev| {
                if let Some(p) = preview {
                    p.on_pointer_enter(&ev, t_enter.clone());
                }
            }
            on:pointerleave=move |ev| {
                if let Some(p) = preview {
                    p.on_chip_leave(&ev);
                }
            }
            on:focus=move |ev| {
                if let Some(p) = preview {
                    p.on_focus(&ev, t_focus.clone());
                }
            }
            on:blur=move |_| {
                if let Some(p) = preview {
                    p.on_blur();
                }
            }
            on:click=move |ev| {
                if let Some(p) = preview {
                    p.on_chip_click(&ev, t_click.clone());
                }
            }
        >
            <span class="chat-mention-prefix">{prefix}</span>
            <bdi>{name}</bdi>
            <span class="chat-mention-tag">{tag_sigil}{tag}</span>
        </a>
    }
    .into_any()
}

fn render_card_chip(lang: Lang, card: CardMention, preview: Option<PreviewCtl>) -> AnyView {
    let href = scryfall_page_url(&card);
    let finish_attr = card.finish.attr();
    let set_badge = card.set.as_ref().map(|s| s.to_ascii_uppercase());
    let label = card.label.clone();
    let target = PreviewTarget::Card(card);
    let t_enter = target.clone();
    let t_focus = target.clone();
    let t_click = target;
    view! {
        <a
            class="chat-mention chat-mention-card"
            href=href
            target="_blank"
            rel="noopener noreferrer"
            data-finish=finish_attr
            aria-label=words::chat_card_open_scryfall(lang)
            translate="no"
            on:pointerdown=move |ev| {
                if let Some(p) = preview {
                    p.on_pointer_down(&ev);
                }
            }
            on:pointerenter=move |ev| {
                if let Some(p) = preview {
                    p.on_pointer_enter(&ev, t_enter.clone());
                }
            }
            on:pointerleave=move |ev| {
                if let Some(p) = preview {
                    p.on_chip_leave(&ev);
                }
            }
            on:focus=move |ev| {
                if let Some(p) = preview {
                    p.on_focus(&ev, t_focus.clone());
                }
            }
            on:blur=move |_| {
                if let Some(p) = preview {
                    p.on_blur();
                }
            }
            on:click=move |ev| {
                if let Some(p) = preview {
                    p.on_chip_click(&ev, t_click.clone());
                }
            }
        >
            {glyph(glyph::LAYERS)}
            <span class="chat-mention-name">{label}</span>
            {set_badge.map(|s| view! { <span class="chat-mention-set">{s}</span> })}
        </a>
    }
    .into_any()
}

/// Renders a chat message with interactive hover/focus/fingerpress preview overlays.
pub fn render_message_with(
    lang: Lang,
    text: &str,
    own_tag: u32,
    preview: Option<PreviewCtl>,
) -> AnyView {
    let parts = parse_segments(text)
        .into_iter()
        .map(|seg| match seg {
            Segment::Text(plain) => view! { {plain} }.into_any(),
            Segment::User { name, tag } => render_user_chip(lang, name, tag, own_tag, preview),
            Segment::Card(card) => render_card_chip(lang, card, preview),
        })
        .collect_view();
    view! { {parts} }.into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_detects_cards_modifiers_and_users() {
        let card = active_trigger("cast #Lightning Bo", 18).expect("card");
        assert_eq!(card.kind, TriggerKind::Card);
        assert_eq!(card.start, 5);
        assert_eq!(card.query, "Lightning Bo");

        let modded = active_trigger("cast #Blitzschlag#2x2#117#foil", 30).expect("modded");
        assert_eq!(modded.kind, TriggerKind::Card);
        assert_eq!(modded.start, 5);
        assert_eq!(modded.query, "Blitzschlag#2x2#117#foil");
        let parsed = parse_card_trigger(&modded.query);
        assert_eq!(parsed.name_query, "Blitzschlag");
        assert_eq!(parsed.set_filter.as_deref(), Some("2x2"));
        assert_eq!(parsed.collector_filter.as_deref(), Some("117"));
        assert_eq!(parsed.finish_filter.as_deref(), Some("foil"));
        assert_eq!(parsed.preferred_finish, Some(CardFinish::Foil));

        let etched = parse_card_trigger("Sol Ring#cmd#etched");
        assert_eq!(etched.name_query, "Sol Ring");
        assert_eq!(etched.set_filter.as_deref(), Some("cmd"));
        assert_eq!(etched.preferred_finish, Some(CardFinish::Etched));

        let user = active_trigger("hey @eri", 8).expect("user");
        assert_eq!(user.kind, TriggerKind::User);
        assert_eq!(user.start, 4);
        assert_eq!(user.query, "eri");

        assert!(active_trigger("email@example.com", 17).is_none());
        assert!(active_trigger("plain #", 7).is_none());
        assert!(active_trigger("[Lightning Bolt] ", 17).is_none());

        let (replaced, caret) = replace_trigger(
            "cast #Bolt now",
            &Trigger {
                kind: TriggerKind::Card,
                start: 5,
                end: 10,
                query: "Bolt".into(),
            },
            "[Lightning Bolt]",
        );
        assert_eq!(replaced, "cast [Lightning Bolt] now");
        assert_eq!(caret, 21);
    }

    #[test]
    fn parses_bracketed_bare_and_printed_mentions_without_showing_finish_text() {
        let segs = parse_segments(
            "Hi [@Erika#1a2b] and @Ada#00ff, play [Lightning Bolt (2X2 #117)#foil] or [Blitzschlag#etched] or #Counterspell#lea#foil not #1a2b!",
        );
        assert_eq!(
            segs,
            vec![
                Segment::Text("Hi ".into()),
                Segment::User {
                    name: "Erika".into(),
                    tag: "1a2b".into(),
                },
                Segment::Text(" and ".into()),
                Segment::User {
                    name: "Ada".into(),
                    tag: "00ff".into(),
                },
                Segment::Text(", play ".into()),
                Segment::Card(CardMention {
                    label: "Lightning Bolt".into(),
                    name: "Lightning Bolt".into(),
                    set: Some("2x2".into()),
                    collector: Some("117".into()),
                    lang: None,
                    finish: CardFinish::Foil,
                }),
                Segment::Text(" or ".into()),
                Segment::Card(CardMention {
                    label: "Blitzschlag".into(),
                    name: "Blitzschlag".into(),
                    set: None,
                    collector: None,
                    lang: None,
                    finish: CardFinish::Etched,
                }),
                Segment::Text(" or ".into()),
                Segment::Card(CardMention {
                    label: "Counterspell".into(),
                    name: "Counterspell".into(),
                    set: Some("lea".into()),
                    collector: None,
                    lang: None,
                    finish: CardFinish::Foil,
                }),
                Segment::Text(" not #1a2b!".into()),
            ]
        );
    }

    #[test]
    fn formats_card_specs_replacements_and_scryfall_urls() {
        let with_lang = parse_card_spec("Armageddon (4ED #3 de)#foil").expect("with lang");
        assert_eq!(with_lang.name, "Armageddon");
        assert_eq!(with_lang.set.as_deref(), Some("4ed"));
        assert_eq!(with_lang.collector.as_deref(), Some("3"));
        assert_eq!(with_lang.lang.as_deref(), Some("de"));
        assert_eq!(with_lang.finish, CardFinish::Foil);
        let de_printing = CardPrinting {
            scryfall_id: None,
            set_code: "4ed".into(),
            set_name: "Fourth Edition".into(),
            collector_number: "3".into(),
            lang: "de".into(),
            finishes: FINISH_FOIL,
            ub: false,
        };
        assert_eq!(
            card_replacement(
                "Armageddon",
                "Armageddon",
                Some(&de_printing),
                CardFinish::Foil
            ),
            "[Armageddon (4ED #3 de)#foil]"
        );
        assert_eq!(
            card_replacement(
                "Blitzschlag",
                "Lightning Bolt",
                Some(&de_printing),
                CardFinish::Normal
            ),
            "[Blitzschlag (4ED #3)]"
        );
        assert_eq!(
            plain_preview("Play [Blitzschlag (2X2 #117)#foil] now"),
            "Play Blitzschlag now"
        );
        assert_eq!(
            scryfall_page_url(&CardMention {
                label: "Lightning Bolt".into(),
                name: "Lightning Bolt".into(),
                set: Some("2x2".into()),
                collector: Some("117".into()),
                lang: None,
                finish: CardFinish::Foil,
            }),
            "https://scryfall.com/card/2x2/117"
        );
        assert_eq!(
            scryfall_url("Sol Ring"),
            "https://scryfall.com/search?q=%21%22Sol%20Ring%22"
        );
        assert_eq!(
            scryfall_image_url("e3285e6b-3e79-4d7c-bf96-d920f973b122").as_deref(),
            Some(
                "https://cards.scryfall.io/normal/front/e/3/e3285e6b-3e79-4d7c-bf96-d920f973b122.jpg"
            )
        );
        assert!(scryfall_image_url("not-a-uuid").is_none());
    }
}
