//! First-party index of public Magic card names for chat `#Card` autocomplete.
//!
//! Only card names live here (`data/card-names.txt`); no rules text, artwork,
//! or third-party scripts are included. The list is parsed lazily once into
//! memory and searched case-insensitively with deterministic ranking.

use std::sync::OnceLock;

/// Default maximum number of suggestions returned per query.
pub const DEFAULT_LIMIT: usize = 8;

/// Maximum accepted query length in Unicode scalar values.
pub const MAX_QUERY_LEN: usize = 64;

/// Sorted, unique public card names, one per line.
const RAW_NAMES: &str = include_str!("../data/card-names.txt");

struct Entry {
    name: &'static str,
    lower: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Rank {
    Exact,
    Prefix,
    WordStart,
    Substring,
}

static INDEX: OnceLock<Vec<Entry>> = OnceLock::new();

fn build_index() -> Vec<Entry> {
    RAW_NAMES
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|name| Entry {
            name,
            lower: name.to_lowercase(),
        })
        .collect()
}

fn index() -> &'static [Entry] {
    INDEX.get_or_init(build_index)
}

/// Whether `haystack` matches `needle` starting at `first` or later immediately
/// after a non-alphanumeric word boundary.
fn has_word_start_match(haystack: &str, first: usize, needle: &str) -> bool {
    let mut offset = first;
    while let Some(rel) = haystack[offset..].find(needle) {
        let start = offset + rel;
        if start > 0
            && haystack[..start]
                .chars()
                .next_back()
                .is_some_and(|ch| !ch.is_alphanumeric())
        {
            return true;
        }
        let step = haystack[start..].chars().next().map_or(1, char::len_utf8);
        offset = start + step;
    }
    false
}

fn classify(lower: &str, needle: &str) -> Option<Rank> {
    let first = lower.find(needle)?;
    if first == 0 {
        return Some(if lower.len() == needle.len() {
            Rank::Exact
        } else {
            Rank::Prefix
        });
    }
    if has_word_start_match(lower, first, needle) {
        Some(Rank::WordStart)
    } else {
        Some(Rank::Substring)
    }
}

/// Total number of indexed card names.
#[must_use]
pub fn count() -> usize {
    index().len()
}

/// Returns up to `limit` matching card names for `query`, ranked by match
/// quality: exact match, prefix, word-start, then substring (breaking ties
/// by shorter name, then lexicographically).
///
/// Empty queries, queries containing control characters, or queries longer
/// than [`MAX_QUERY_LEN`] Unicode scalar values return an empty list.
#[must_use]
pub fn suggest(query: &str, limit: usize) -> Vec<&'static str> {
    let trimmed = query.trim();
    if limit == 0
        || trimmed.is_empty()
        || trimmed.chars().count() > MAX_QUERY_LEN
        || trimmed.chars().any(char::is_control)
    {
        return Vec::new();
    }
    let needle = trimmed.to_lowercase();
    let mut matches: Vec<(Rank, usize, &str, &'static str)> = index()
        .iter()
        .filter_map(|entry| {
            let rank = classify(&entry.lower, &needle)?;
            Some((rank, entry.name.len(), entry.lower.as_str(), entry.name))
        })
        .collect();
    matches.sort_unstable_by(|a, b| (a.0, a.1, a.2).cmp(&(b.0, b.1, b.2)));
    matches.truncate(limit);
    matches.into_iter().map(|(_, _, _, name)| name).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_is_populated() {
        assert!(count() > 20_000, "expected full card name pool");
    }

    #[test]
    fn suggests_lightning_bolt_and_sol_ring() {
        let sol = suggest("sol r", DEFAULT_LIMIT);
        assert_eq!(sol.first().copied(), Some("Sol Ring"));
        assert_eq!(suggest("SOL R", DEFAULT_LIMIT), sol);

        let bolt_exact = suggest("lightning bolt", DEFAULT_LIMIT);
        assert_eq!(bolt_exact.first().copied(), Some("Lightning Bolt"));

        let bolt_prefix = suggest("lightning b", DEFAULT_LIMIT);
        assert!(
            bolt_prefix.contains(&"Lightning Bolt"),
            "expected Lightning Bolt in {bolt_prefix:?}"
        );

        let light = suggest("light", DEFAULT_LIMIT);
        assert_eq!(light.len(), DEFAULT_LIMIT);
        assert!(
            light
                .iter()
                .all(|name| name.to_lowercase().starts_with("light")),
            "expected prefix matches for `light`: {light:?}"
        );

        let light_wide = suggest("light", 25);
        assert!(
            light_wide.contains(&"Lightning Bolt"),
            "expected Lightning Bolt in wider `light` prefix matches: {light_wide:?}"
        );
    }

    #[test]
    fn ranks_exact_prefix_word_start_then_substring() {
        assert_eq!(classify("bolt", "bolt"), Some(Rank::Exact));
        assert_eq!(classify("bolt bend", "bolt"), Some(Rank::Prefix));
        assert_eq!(classify("lightning bolt", "bolt"), Some(Rank::WordStart));
        assert_eq!(classify("light-paws", "paws"), Some(Rank::WordStart));
        assert_eq!(classify("firebolt", "bolt"), Some(Rank::Substring));
        assert_eq!(classify("sol ring", "bolt"), None);
    }

    #[test]
    fn rejects_empty_control_and_overlong_queries() {
        assert_eq!(suggest("", DEFAULT_LIMIT), [] as [&str; 0]);
        assert_eq!(suggest("   \t ", DEFAULT_LIMIT), [] as [&str; 0]);
        assert_eq!(suggest("sol\0ring", DEFAULT_LIMIT), [] as [&str; 0]);
        assert_eq!(suggest("sol\nring", DEFAULT_LIMIT), [] as [&str; 0]);
        assert_eq!(
            suggest(&"a".repeat(MAX_QUERY_LEN + 1), DEFAULT_LIMIT),
            [] as [&str; 0]
        );
        assert_eq!(suggest("sol r", 0), [] as [&str; 0]);
    }
}
