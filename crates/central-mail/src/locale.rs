//! The words of the mails: eleven languages, each a file of `id = text`
//! lines included at compile time (`locales/<code>/mail.ftl`). The format
//! is a small part of Fluent's and nothing else of Fluent is read: one
//! message a line, `{ $name }` where a value goes, `#` before a comment.

/// A language Central writes mail in: the website's eleven.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Lang {
    /// English, and the language of every tag without words of its own.
    En,
    /// German.
    De,
    /// French.
    Fr,
    /// Italian.
    It,
    /// Spanish.
    Es,
    /// Brazilian Portuguese.
    PtBr,
    /// Russian.
    Ru,
    /// Japanese.
    Ja,
    /// Korean.
    Ko,
    /// Chinese, simplified.
    ZhHans,
    /// Chinese, traditional.
    ZhHant,
}

impl Lang {
    /// Every language, English first.
    pub const ALL: [Self; 11] = [
        Self::En,
        Self::De,
        Self::Fr,
        Self::It,
        Self::Es,
        Self::PtBr,
        Self::Ru,
        Self::Ja,
        Self::Ko,
        Self::ZhHans,
        Self::ZhHant,
    ];

    /// The language of a tag as a request names it (`de`, `pt-br`). A tag
    /// Central has no words for is English: the tag itself is never used
    /// again, in a text or in a link.
    #[must_use]
    pub fn from_tag(tag: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|lang| lang.code() == tag)
            .unwrap_or(Self::En)
    }

    /// The language's tag, which is also its part of the website's paths.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::De => "de",
            Self::Fr => "fr",
            Self::It => "it",
            Self::Es => "es",
            Self::PtBr => "pt-br",
            Self::Ru => "ru",
            Self::Ja => "ja",
            Self::Ko => "ko",
            Self::ZhHans => "zh-hans",
            Self::ZhHant => "zh-hant",
        }
    }

    /// The language's file of words.
    pub(crate) const fn source(self) -> &'static str {
        match self {
            Self::En => include_str!("../locales/en/mail.ftl"),
            Self::De => include_str!("../locales/de/mail.ftl"),
            Self::Fr => include_str!("../locales/fr/mail.ftl"),
            Self::It => include_str!("../locales/it/mail.ftl"),
            Self::Es => include_str!("../locales/es/mail.ftl"),
            Self::PtBr => include_str!("../locales/pt-br/mail.ftl"),
            Self::Ru => include_str!("../locales/ru/mail.ftl"),
            Self::Ja => include_str!("../locales/ja/mail.ftl"),
            Self::Ko => include_str!("../locales/ko/mail.ftl"),
            Self::ZhHans => include_str!("../locales/zh-hans/mail.ftl"),
            Self::ZhHant => include_str!("../locales/zh-hant/mail.ftl"),
        }
    }

    /// The text of the message `id` with `values` in its placeables, in
    /// English where this language lacks the message. The tests hold every
    /// language to every message, so the fallback is for a file edited
    /// wrongly later, and a mail then still says what it has to.
    pub(crate) fn text(self, id: &str, values: &[(&str, &str)]) -> String {
        let find = |source| {
            messages(source)
                .flatten()
                .find_map(|(name, text)| (name == id).then_some(text))
        };
        let text = find(self.source())
            .or_else(|| find(Self::En.source()))
            .unwrap_or_default();
        fill(text, values)
    }
}

/// A line of a words file that is neither blank, a comment nor a message.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Malformed;

/// The messages of `source` in their order, as `(id, text)`; a line that
/// is no message is reported in its place.
pub(crate) fn messages(source: &str) -> impl Iterator<Item = Result<(&str, &str), Malformed>> {
    source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(message)
}

/// One `id = text` line: an id of lower-case letters, digits and hyphens
/// that starts with a letter, and a text without control characters whose
/// placeables are whole.
fn message(line: &str) -> Result<(&str, &str), Malformed> {
    let (id, text) = line.split_once('=').ok_or(Malformed)?;
    let (id, text) = (id.trim_end(), text.trim_start());
    let named = id.bytes().next().is_some_and(|b| b.is_ascii_lowercase())
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
    if !named || text.is_empty() || text.chars().any(char::is_control) {
        return Err(Malformed);
    }
    for part in parts(text) {
        part?;
    }
    Ok((id, text))
}

/// A piece of a message's text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Part<'a> {
    /// Words, as they are written.
    Words(&'a str),
    /// The name of a value that goes here.
    Value(&'a str),
}

/// A text split at its placeables.
pub(crate) struct Parts<'a>(&'a str);

/// Split `text` at its `{ $name }` placeables. A brace that is no part of
/// one is reported, and the rest of the text is not read.
pub(crate) const fn parts(text: &str) -> Parts<'_> {
    Parts(text)
}

impl<'a> Iterator for Parts<'a> {
    type Item = Result<Part<'a>, Malformed>;

    fn next(&mut self) -> Option<Self::Item> {
        let text = self.0;
        if text.is_empty() {
            return None;
        }
        self.0 = "";
        let Some(inside) = text.strip_prefix('{') else {
            let end = text.find('{').unwrap_or(text.len());
            let (words, rest) = text.split_at(end);
            if words.contains('}') {
                return Some(Err(Malformed));
            }
            self.0 = rest;
            return Some(Ok(Part::Words(words)));
        };
        let Some((name, rest)) = inside.split_once('}') else {
            return Some(Err(Malformed));
        };
        let Some(name) = name.trim().strip_prefix('$') else {
            return Some(Err(Malformed));
        };
        if name.is_empty() || !name.bytes().all(|b| b.is_ascii_lowercase()) {
            return Some(Err(Malformed));
        }
        self.0 = rest;
        Some(Ok(Part::Value(name)))
    }
}

/// Write `text` with `values` in its placeables. A placeable without a
/// value writes nothing. The result is plain text: whoever puts it into
/// HTML escapes all of it, the values with the words.
fn fill(text: &str, values: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(text.len());
    for part in parts(text).flatten() {
        match part {
            Part::Words(words) => out.push_str(words),
            Part::Value(name) => {
                if let Some((_, value)) = values.iter().find(|(given, _)| *given == name) {
                    out.push_str(value);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// A language's messages: each id with the placeables of its text.
    fn table(lang: Lang) -> BTreeMap<&'static str, Vec<&'static str>> {
        let mut table = BTreeMap::new();
        for line in messages(lang.source()) {
            let (id, text) = line.unwrap_or_else(|_| panic!("{}: a malformed line", lang.code()));
            let mut values: Vec<&str> = parts(text)
                .filter_map(|part| match part {
                    Ok(Part::Value(name)) => Some(name),
                    _ => None,
                })
                .collect();
            values.sort_unstable();
            assert!(
                table.insert(id, values).is_none(),
                "{}: {id} twice",
                lang.code()
            );
        }
        table
    }

    #[test]
    fn every_language_has_every_message_with_the_same_placeables() {
        let english = table(Lang::En);
        assert!(english.len() >= 20);
        for lang in Lang::ALL {
            assert_eq!(table(lang), english, "{}", lang.code());
        }
        // The placeables are the ones the mails fill.
        let used: Vec<&str> = english.values().flatten().copied().collect();
        assert!(used.iter().all(|name| ["minutes", "name"].contains(name)));
    }

    #[test]
    fn every_language_has_words_of_its_own() {
        let words = |lang: Lang| -> Vec<&str> {
            messages(lang.source())
                .map(|line| line.unwrap().1)
                .collect()
        };
        let english = words(Lang::En);
        for lang in Lang::ALL {
            if lang == Lang::En {
                continue;
            }
            // only the brand's name is the same in every language
            let same = words(lang)
                .iter()
                .zip(&english)
                .filter(|(theirs, ours)| theirs == ours)
                .count();
            assert_eq!(same, 1, "{}", lang.code());
        }
    }

    #[test]
    fn a_tag_without_words_is_english() {
        for lang in Lang::ALL {
            assert_eq!(Lang::from_tag(lang.code()), lang);
            // the website's paths and the request's check take the same tags
            assert!(central_api::email::language(lang.code()).is_ok());
        }
        for tag in [
            "", "xx", "EN", "de-at", "pt", "zh", "en/../x", "de\r\n", "klingon",
        ] {
            assert_eq!(Lang::from_tag(tag), Lang::En, "{tag}");
        }
    }

    #[test]
    fn lines_are_messages_comments_or_refused() {
        let read = |source: &'static str| messages(source).collect::<Vec<_>>();
        assert_eq!(
            read("# a comment\n\n  a-1 = Words { $name } and {$minutes}.  \nb=x\n"),
            vec![
                Ok(("a-1", "Words { $name } and {$minutes}.")),
                Ok(("b", "x"))
            ]
        );
        for bad in [
            "no equals sign",
            "= no id",
            "Upper = x",
            "1st = x",
            "under_score = x",
            "empty =",
            "open = a { $name",
            "close = a } b",
            "plain = a { name } b",
            "nameless = a { $ } b",
            "upper = a { $Name } b",
            "nested = a { { $name } } b",
            "control = a \u{7} b",
        ] {
            assert_eq!(read(bad), vec![Err(Malformed)], "{bad}");
        }
    }

    #[test]
    fn values_go_where_they_are_named_and_only_there() {
        assert_eq!(
            fill(
                "For { $name }: {$minutes} minutes, { $name }.",
                &[("name", "player"), ("minutes", "15")]
            ),
            "For player: 15 minutes, player."
        );
        // a value is written as it is and never read as a placeable
        assert_eq!(
            fill("{ $name }", &[("name", "{ $minutes }")]),
            "{ $minutes }"
        );
        // a placeable nobody fills writes nothing
        assert_eq!(fill("a{ $other }b", &[("name", "x")]), "ab");
        // English words for a message that is in every file
        assert_eq!(Lang::De.text("brand", &[]), "Baylee");
        assert_eq!(Lang::En.text("no-such-message", &[]), "");
        assert_eq!(
            Lang::De.text("confirm-valid", &[("minutes", "15")]),
            "Der Code gilt 15 Minuten."
        );
    }
}
