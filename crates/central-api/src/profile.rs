//! Display names and tags: what other players see of an account, drawn as
//! the Baylee client draws them (`Alice#0007`; `docs/web-accounts.md`
//! §"Display name and tag"). The name is not unique; the tag is, and it never
//! changes once an account holds it, because chat on a gateway is keyed by it.
use crate::Error;
use std::ops::RangeInclusive;

/// The lowest tag a registered account draws. Accounts moved from the
/// official gateway keep that gateway's tags, which stay below it (its
/// highest was 13 on 23.10.2026, and it makes no local accounts once bound
/// to Central). Migration 000011 draws from the same floor.
pub const REGISTERED_TAG_FLOOR: u32 = 0x1000;

/// The highest tag: a gateway keeps tags in a signed 32-bit column.
pub const MAX_TAG: u32 = 0x7fff_ffff;

/// Names a person may not choose, compared without case, `_` and `-`
/// ([`reserved`]). A name carried from the official gateway is kept as it is.
pub const RESERVED_NAMES: [&str; 8] = [
    "admin",
    "administrator",
    "baylee",
    "central",
    "moderator",
    "official",
    "support",
    "system",
];

/// The gateway's rule exactly (Baylee `crates/baylee-gateway/src/auth.rs`
/// `valid_display_name`): 3 to 16 ASCII letters and digits, with `_` and `-`
/// only between them; case is kept. Migration 000011's CHECK holds the same
/// rule, and both repositories test the same table of cases.
#[must_use]
pub fn valid_display_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    let plain = |b: Option<&u8>| b.is_some_and(u8::is_ascii_alphanumeric);
    (3..=16).contains(&bytes.len())
        && plain(bytes.first())
        && plain(bytes.last())
        && bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || *b == b'_' || *b == b'-')
}

/// Whether `name` is one of [`RESERVED_NAMES`], read without case, `_` and `-`.
#[must_use]
pub fn reserved(name: &str) -> bool {
    let plain: String = name
        .chars()
        .filter(|c| *c != '_' && *c != '-')
        .map(|c| c.to_ascii_lowercase())
        .collect();
    RESERVED_NAMES.contains(&plain.as_str())
}

/// A name a person may choose: [`valid_display_name`] and not [`reserved`].
#[must_use]
pub fn choosable(name: &str) -> bool {
    valid_display_name(name) && !reserved(name)
}

/// A tag is positive and fits a gateway's column.
#[must_use]
pub const fn valid_tag(tag: u32) -> bool {
    tag >= 1 && tag <= MAX_TAG
}

/// A tag as a player reads it: lowercase hex, at least four digits, as
/// Baylee's `handle.rs` draws it (`7` is `0007`, `0x10000` is `10000`).
#[must_use]
pub fn tag_text(tag: u32) -> String {
    format!("{tag:04x}")
}

/// The whole handle, `Alice#0007`.
#[must_use]
pub fn handle(name: &str, tag: u32) -> String {
    format!("{name}#{}", tag_text(tag))
}

/// The range a registered account's tag is drawn from first, and each one
/// after it: `[f, 16f - 1]` from `floor`, sixteen times wider each time, the
/// last ending at [`MAX_TAG`]. From [`REGISTERED_TAG_FLOOR`] these are the
/// tags of four hex digits, then of five, and so on. Empty for a floor of 0
/// or above [`MAX_TAG`].
#[must_use]
pub fn tag_ranges(floor: u32) -> Vec<RangeInclusive<u32>> {
    let mut ranges = Vec::new();
    let mut low = u64::from(floor);
    while low >= 1 && low <= u64::from(MAX_TAG) {
        let high = (low * 16 - 1).min(u64::from(MAX_TAG));
        // Both ends are at most MAX_TAG here.
        ranges.push(u32::try_from(low).unwrap_or(MAX_TAG)..=u32::try_from(high).unwrap_or(MAX_TAG));
        low = high + 1;
    }
    ranges
}

/// A tag drawn uniformly from `range` with operating-system randomness.
/// Tags are public, so this hides only how many accounts exist.
///
/// # Errors
/// `Rejected` for an empty range; `Unavailable` when randomness is.
pub fn draw_tag(range: &RangeInclusive<u32>) -> Result<u32, Error> {
    let (low, high) = (*range.start(), *range.end());
    if low > high {
        return Err(Error::Rejected);
    }
    let span = u64::from(high - low) + 1;
    // Rejection sampling: no value of the range is likelier than another.
    let limit = (1u64 << 32) - (1u64 << 32) % span;
    loop {
        let mut bytes = [0; 4];
        getrandom::fill(&mut bytes).map_err(|_| Error::Unavailable)?;
        let value = u64::from(u32::from_le_bytes(bytes));
        if value < limit {
            // `value % span` is below `span`, so the sum is at most `high`.
            return u32::try_from(u64::from(low) + value % span).map_err(|_| Error::Rejected);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The gateway's table of cases (Baylee `auth.rs`, `display_name_validation`),
    /// which Baylee's tests hold against this function too.
    const NAMES: [(&str, bool); 13] = [
        ("Alice", true),
        ("player_one-99", true),
        ("ab", false),
        ("has space", false),
        ("emoji🎉", false),
        ("sixteen_chars_yz", true),
        ("seventeen_chars_x", false),
        ("alice@example.com", false),
        ("Alice#af03", false),
        ("_Alice", false),
        ("Alice-", false),
        ("abc", true),
        ("a_-b", true),
    ];

    #[test]
    fn a_display_name_keeps_the_gateways_rule() {
        for (name, valid) in NAMES {
            assert_eq!(valid_display_name(name), valid, "{name}");
        }
        // Case is kept, and every ASCII letter and digit counts.
        assert!(valid_display_name("ZZ9"));
        assert!(valid_display_name("0-9"));
        // Non-ASCII letters are no letters here, even at sixteen bytes or fewer.
        assert!(!valid_display_name("Zoë"));
        assert!(!valid_display_name("Ǆab"));
        assert!(!valid_display_name(""));
        assert!(!valid_display_name("-"));
        assert!(!valid_display_name("abc\n"));
    }

    #[test]
    fn reserved_names_are_read_without_case_underscores_and_hyphens() {
        for name in [
            "admin",
            "Admin",
            "ADMINISTRATOR",
            "Bay-lee",
            "b_a_y_l_e_e",
            "Central",
            "moder-ator",
            "Official",
            "SUPPORT",
            "sys_tem",
        ] {
            assert!(reserved(name), "{name}");
            assert!(!choosable(name), "{name}");
        }
        for name in ["admins", "Admin1", "Baylee2", "centrally", "Alice"] {
            assert!(!reserved(name), "{name}");
            assert!(choosable(name), "{name}");
        }
        assert!(
            !choosable("_admin"),
            "a malformed name is not choosable either"
        );
    }

    #[test]
    fn a_tag_reads_as_the_client_draws_it() {
        assert_eq!(tag_text(7), "0007");
        assert_eq!(tag_text(0x3fa2), "3fa2");
        assert_eq!(tag_text(0xffff), "ffff");
        assert_eq!(tag_text(0x10000), "10000");
        assert_eq!(tag_text(MAX_TAG), "7fffffff");
        assert_eq!(handle("Alice", 7), "Alice#0007");
        assert!(!valid_tag(0));
        assert!(valid_tag(1));
        assert!(valid_tag(MAX_TAG));
        assert!(!valid_tag(MAX_TAG + 1));
    }

    #[test]
    fn registered_tags_widen_by_a_hex_digit_at_a_time() {
        let ranges = tag_ranges(REGISTERED_TAG_FLOOR);
        assert_eq!(ranges[0], 0x1000..=0xffff);
        assert_eq!(ranges[1], 0x1_0000..=0xf_ffff);
        assert_eq!(ranges.last(), Some(&(0x1000_0000..=MAX_TAG)));
        assert_eq!(ranges.len(), 5);
        for pair in ranges.windows(2) {
            assert_eq!(*pair[0].end() + 1, *pair[1].start());
        }
        assert_eq!(tag_ranges(1)[..2], [1..=15, 16..=255]);
        assert_eq!(tag_ranges(0x2000)[0], 0x2000..=0x1_ffff);
        assert_eq!(tag_ranges(0).len(), 0);
        assert_eq!(tag_ranges(MAX_TAG + 1).len(), 0);
        let last = tag_ranges(MAX_TAG);
        assert_eq!((last.len(), last.first()), (1, Some(&(MAX_TAG..=MAX_TAG))));
    }

    #[test]
    fn a_draw_stays_in_its_range() {
        for range in [
            1..=1,
            1..=2,
            0x1000..=0xffff,
            MAX_TAG - 3..=MAX_TAG,
            0..=u32::MAX,
        ] {
            for _ in 0..64 {
                assert!(range.contains(&draw_tag(&range).unwrap()), "{range:?}");
            }
        }
        let mut seen = [false; 4];
        for _ in 0..256 {
            seen[usize::try_from(draw_tag(&(10..=13)).unwrap() - 10).unwrap()] = true;
        }
        assert!(
            seen.iter().all(|s| *s),
            "every tag of a small range is drawn"
        );
        assert_eq!(draw_tag(&RangeInclusive::new(5, 4)), Err(Error::Rejected));
    }
}
