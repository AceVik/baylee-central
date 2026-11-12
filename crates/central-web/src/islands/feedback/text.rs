//! The small pure pieces the moderation pages share: ids as hex, a
//! report's first line, a reporter's alias, sizes, times as ISO, a report's
//! id as pasted, the chart's days and a history line's detail. Nothing a
//! report says is ever HTML here: these return text for the views to draw
//! as text.

use central_api::reports::{DayCount, Priority, ReportKind, ReportStatus, STATS_DAYS};

use crate::words::{self, Lang};

/// Lowercase hex of `bytes`.
#[must_use]
pub fn hex(bytes: &[u8]) -> String {
    central_api::reports::hex(bytes)
}

/// The id `text` names: 32 lowercase hex digits of a v7 id, as Central
/// makes every one; anything else is none.
#[must_use]
pub fn id_of(text: &str) -> Option<[u8; 16]> {
    if text.len() != 32 || !text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
        return None;
    }
    let mut id = [0_u8; 16];
    for (at, byte) in id.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[at * 2..at * 2 + 2], 16).ok()?;
    }
    super::super::central::uuid_v7(&id).then_some(id)
}

/// A report's id as the page shows it short: its last eight hex digits,
/// the random part of a v7 id.
#[must_use]
pub fn short_id(id: &[u8; 16]) -> String {
    hex(&id[12..])
}

/// A report's id as an administrator pastes it: 32 hex digits, a UUID, or
/// its page's address.
#[must_use]
pub fn read_report_id(text: &str) -> Option<[u8; 16]> {
    let plain = text.trim().to_ascii_lowercase().replace('-', "");
    let bytes = plain.as_bytes();
    // a run of exactly 32 hex digits, alone or as a path's segment
    let mut at = 0;
    while at + 32 <= bytes.len() {
        let before = at == 0 || bytes[at - 1] == b'/';
        let run = bytes[at..at + 32]
            .iter()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
        let after = at + 32 == bytes.len() || matches!(bytes[at + 32], b'/' | b'?' | b'#');
        if before && run && after {
            return id_of(&plain[at..at + 32]);
        }
        at += 1;
    }
    None
}

/// The first line of `text` that is not blank, cut to `max` characters
/// with an ellipsis.
#[must_use]
pub fn first_line(text: &str, max: usize) -> String {
    let line = text
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
        .trim();
    if line.chars().count() > max {
        let cut: String = line.chars().take(max.saturating_sub(1)).collect();
        format!("{}…", cut.trim_end())
    } else {
        line.to_owned()
    }
}

const ADJECTIVES: [&str; 64] = [
    "Amber", "Bold", "Brisk", "Calm", "Clever", "Cool", "Crisp", "Dapper", "Deep", "Eager", "Fair",
    "Fleet", "Fond", "Frank", "Gentle", "Glad", "Grand", "Hardy", "Hollow", "Humble", "Jolly",
    "Keen", "Kind", "Late", "Light", "Lively", "Loyal", "Lucky", "Merry", "Mild", "Misty", "Neat",
    "Noble", "Odd", "Pale", "Plain", "Proud", "Quick", "Quiet", "Rapid", "Rare", "Ready", "Rosy",
    "Royal", "Rusty", "Sharp", "Shy", "Silent", "Sly", "Soft", "Solid", "Spry", "Steady", "Still",
    "Stout", "Swift", "Tidy", "True", "Vivid", "Warm", "Wild", "Wise", "Witty", "Young",
];

const NOUNS: [&str; 64] = [
    "Badger", "Bear", "Beetle", "Bison", "Crane", "Crow", "Deer", "Dove", "Eagle", "Eel", "Elk",
    "Falcon", "Ferret", "Finch", "Fox", "Frog", "Gull", "Hare", "Hawk", "Heron", "Hornet", "Ibis",
    "Jay", "Koi", "Lark", "Lemur", "Lion", "Llama", "Lynx", "Mole", "Moose", "Moth", "Newt",
    "Orca", "Otter", "Owl", "Panda", "Pike", "Puma", "Quail", "Raven", "Robin", "Seal", "Shrew",
    "Snail", "Sparrow", "Stag", "Stork", "Swan", "Swift", "Tapir", "Tern", "Toad", "Trout",
    "Viper", "Vole", "Wasp", "Whale", "Wolf", "Wren", "Yak", "Zebra", "Carp", "Crab",
];

/// A reporter's pseudonym as an administrator tells it apart at a glance:
/// two words from its own first hex digits (`Quiet Heron`) and its first
/// four digits. The pseudonym is an HMAC only the gateway or the device
/// can make (`docs/reports.md` §"Privacy invariants"); nothing more can be
/// read from the alias than from the hex.
#[must_use]
pub fn alias(pseudonym: &str) -> (String, String) {
    let head: String = pseudonym.chars().take(8).collect();
    let padded = format!("{head:0<8}");
    let n = u32::from_str_radix(&padded, 16).unwrap_or(0);
    let at = |i: u32| usize::try_from(i % 64).unwrap_or(0);
    let adjective = ADJECTIVES[at(n)];
    let noun = NOUNS[at(n / 64)];
    let short: String = pseudonym.chars().take(4).collect();
    (format!("{adjective} {noun}"), short.to_ascii_lowercase())
}

/// `bytes` as a person reads it: `912 B`, `21.1 KiB`, `3.4 MiB`.
#[must_use]
pub fn size(lang: Lang, bytes: u64) -> String {
    if bytes < 1024 {
        return format!("{} B", words::number(lang, bytes));
    }
    let units = ["KiB", "MiB", "GiB"];
    let mut divisor: u64 = 1024;
    let mut unit = 0;
    while bytes / divisor >= 1024 && unit < units.len() - 1 {
        divisor *= 1024;
        unit += 1;
    }
    let decimal = match lang {
        Lang::En | Lang::Ja | Lang::Ko | Lang::ZhHans | Lang::ZhHant => '.',
        _ => ',',
    };
    // under ten one decimal, rounded; a whole number above
    if bytes / divisor < 10 {
        let tenths = (bytes.saturating_mul(10) + divisor / 2) / divisor;
        if tenths.is_multiple_of(10) {
            format!("{} {}", tenths / 10, units[unit])
        } else {
            format!("{}{decimal}{} {}", tenths / 10, tenths % 10, units[unit])
        }
    } else {
        let whole = (bytes + divisor / 2) / divisor;
        format!("{} {}", words::number(lang, whole), units[unit])
    }
}

/// The civil date and time of Unix `seconds`, UTC.
fn civil(seconds: i64) -> (i64, i64, i64, i64, i64, i64) {
    let days = seconds.div_euclid(86_400);
    let rest = seconds.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day, rest / 3600, rest % 3600 / 60, rest % 60)
}

/// Unix `seconds` as `toISOString` writes them: `2026-11-12T09:30:00.000Z`.
#[must_use]
pub fn iso(seconds: i64) -> String {
    let (year, month, day, hour, minute, second) = civil(seconds);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.000Z")
}

/// The UTC day of Unix `seconds`, `YYYY-MM-DD`.
#[must_use]
pub fn day_of(seconds: i64) -> String {
    let (year, month, day, ..) = civil(seconds);
    format!("{year:04}-{month:02}-{day:02}")
}

/// One day of the chart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Day {
    /// `YYYY-MM-DD`, UTC.
    pub date: String,
    /// Crashes that day.
    pub crashes: u64,
    /// The other reports that day.
    pub others: u64,
}

/// The last 30 UTC days up to `now`'s, each with its counts (none for a
/// day Central did not name).
#[must_use]
pub fn last_days(counts: &[DayCount], now: i64) -> Vec<Day> {
    let today = now.div_euclid(86_400);
    let n = i64::from(STATS_DAYS);
    (0..n)
        .map(|i| {
            let date = day_of((today - (n - 1 - i)) * 86_400);
            let of = counts.iter().filter(|c| c.day == date);
            let (mut crashes, mut others) = (0_u64, 0_u64);
            for count in of {
                if count.kind == ReportKind::Crash {
                    crashes = crashes.saturating_add(count.count);
                } else {
                    others = others.saturating_add(count.count);
                }
            }
            Day {
                date,
                crashes,
                others,
            }
        })
        .collect()
}

/// A history line's detail, as Central wrote it, in words: a status or a
/// priority in the page's language, an administrator's id named by
/// `name`, a report's id short. `None` for a detail that is empty.
#[must_use]
pub fn detail_text(
    lang: Lang,
    action: &str,
    detail: &str,
    name: &dyn Fn(Option<[u8; 16]>) -> String,
) -> Option<String> {
    if detail.is_empty() {
        return None;
    }
    if detail == "none" {
        return Some(words::fb_h_none(lang).to_owned());
    }
    let word = |w: &str| {
        if let Ok(status) = ReportStatus::from_stored(w) {
            status_word(lang, status).to_owned()
        } else if let Ok(priority) = Priority::from_stored(w) {
            priority_text(lang, Some(priority)).to_owned()
        } else {
            w.to_owned()
        }
    };
    let uuid = |text: &str| {
        let plain = text.replace('-', "");
        (text.len() == 36).then(|| id_of(&plain)).flatten()
    };
    Some(match action {
        "report.status" => detail
            .split(" → ")
            .map(word)
            .collect::<Vec<_>>()
            .join(" → "),
        "report.priority" => word(detail),
        "report.assignee" => uuid(detail).map_or_else(|| detail.to_owned(), |id| name(Some(id))),
        "report.duplicate" => match uuid(detail) {
            Some(id) => format!("…{}", short_id(&id)),
            None if detail.starts_with("none:") => words::fb_h_original_gone(lang).to_owned(),
            None => detail.to_owned(),
        },
        "report.note" => match detail {
            "added" => words::fb_h_note_added(lang).to_owned(),
            "removed" => words::fb_h_note_removed(lang).to_owned(),
            other => other.to_owned(),
        },
        "report.filed" => detail.strip_prefix("by ").unwrap_or(detail).to_owned(),
        _ => detail.to_owned(),
    })
}

/// What a history line's action did, in words; `None` for an action this
/// page does not know (shown as Central wrote it).
#[must_use]
pub fn action_text(lang: Lang, action: &str) -> Option<&'static str> {
    Some(match action {
        "report.filed" => words::fb_h_filed(lang),
        "report.status" => words::fb_h_status(lang),
        "report.issue" => words::fb_h_issue(lang),
        "report.assignee" => words::fb_h_assignee(lang),
        "report.priority" => words::fb_h_priority(lang),
        "report.labels" => words::fb_h_labels(lang),
        "report.duplicate" => words::fb_h_duplicate(lang),
        "report.trashed" => words::fb_h_trashed(lang),
        "report.restored" => words::fb_h_restored(lang),
        "report.purged" => words::fb_h_purged(lang),
        "report.note" => words::fb_h_note(lang),
        "report.swept" => words::fb_h_swept(lang),
        _ => return None,
    })
}

/// A status in words.
#[must_use]
pub fn status_word(lang: Lang, status: ReportStatus) -> &'static str {
    match status {
        ReportStatus::New => words::fb_status_new(lang),
        ReportStatus::Triaged => words::fb_status_triaged(lang),
        ReportStatus::InProgress => words::fb_status_in_progress(lang),
        ReportStatus::Resolved => words::fb_status_resolved(lang),
        ReportStatus::WontFix => words::fb_status_wont_fix(lang),
        ReportStatus::Duplicate => words::fb_status_duplicate(lang),
    }
}

/// A kind in words.
#[must_use]
pub fn kind_word(lang: Lang, kind: ReportKind) -> &'static str {
    match kind {
        ReportKind::Bug => words::fb_kind_bug(lang),
        ReportKind::Improvement => words::fb_kind_improvement(lang),
        ReportKind::Feedback => words::fb_kind_feedback(lang),
        ReportKind::Crash => words::fb_kind_crash(lang),
        ReportKind::Other => words::fb_kind_other(lang),
    }
}

/// A priority, or none, in words.
#[must_use]
pub fn priority_text(lang: Lang, priority: Option<Priority>) -> &'static str {
    match priority {
        Some(Priority::Critical) => words::fb_prio_critical(lang),
        Some(Priority::High) => words::fb_prio_high(lang),
        Some(Priority::Normal) => words::fb_prio_normal(lang),
        Some(Priority::Low) => words::fb_prio_low(lang),
        None => words::fb_prio_none(lang),
    }
}

/// The pill's tone of a status.
#[must_use]
pub const fn status_tone(status: ReportStatus) -> &'static str {
    match status {
        ReportStatus::New => "pill info",
        ReportStatus::Triaged => "pill",
        ReportStatus::InProgress => "pill warn",
        ReportStatus::Resolved => "pill ok",
        ReportStatus::WontFix | ReportStatus::Duplicate => "pill off",
    }
}

/// The pill's tone of a priority.
#[must_use]
pub const fn priority_tone(priority: Priority) -> &'static str {
    match priority {
        Priority::Critical => "pill bad",
        Priority::High => "pill warn",
        Priority::Normal => "pill",
        Priority::Low => "pill off",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u8) -> [u8; 16] {
        let mut id = [0x10; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        id[15] = n;
        id
    }

    #[test]
    fn ids_are_v7_hex_and_nothing_else() {
        let text = hex(&id(7));
        assert_eq!(id_of(&text), Some(id(7)));
        let mut upper = id(7);
        upper[15] = 0xab;
        assert_eq!(id_of(&hex(&upper).to_uppercase()), None);
        assert_eq!(id_of(&"00".repeat(16)), None, "not a v7 id");
        assert_eq!(id_of("1234"), None);
        assert_eq!(short_id(&id(7)), "10101007");
        let uuid = format!(
            "{}-{}-{}-{}-{}",
            &text[..8],
            &text[8..12],
            &text[12..16],
            &text[16..20],
            &text[20..]
        );
        for pasted in [
            text.clone(),
            uuid.to_uppercase(),
            format!("  https://baylee.app/en/account/admin/feedback/{text}?status=new "),
            format!("/admin/feedback/{text}/"),
        ] {
            assert_eq!(read_report_id(&pasted), Some(id(7)), "{pasted}");
        }
        assert_eq!(read_report_id(&format!("x{text}")), None);
        assert_eq!(read_report_id("#311"), None);
    }

    #[test]
    fn the_first_line_is_cut_with_an_ellipsis() {
        assert_eq!(first_line("\n  \n  It crashed \nagain", 120), "It crashed");
        assert_eq!(first_line("abcdef", 4), "abc…");
        assert_eq!(first_line("", 4), "");
    }

    /// The alias is the old administration's, so two pages show one
    /// reporter alike.
    #[test]
    fn a_reporter_reads_the_same_everywhere() {
        // 0x5bdcc146 = 1541194054: % 64 = 6 (Crisp), / 64 % 64 = 5 (Crow)
        assert_eq!(alias("5bdcc146bf60"), ("Crisp Crow".into(), "5bdc".into()));
        assert_eq!(alias("AB").0, alias("ab000000").0);
        assert_eq!(alias("ab").1, "ab");
    }

    #[test]
    fn sizes_and_times() {
        assert_eq!(size(Lang::En, 912), "912 B");
        assert_eq!(size(Lang::En, 21_606), "21 KiB");
        assert_eq!(size(Lang::En, 3_565_158), "3.4 MiB");
        assert_eq!(size(Lang::De, 3_565_158), "3,4 MiB");
        assert_eq!(size(Lang::En, 64 * 1024 * 1024), "64 MiB");
        assert_eq!(iso(1_794_000_000), "2026-11-06T21:20:00.000Z");
        assert_eq!(day_of(1_794_000_000), "2026-11-06");
    }

    #[test]
    fn the_chart_counts_thirty_days_up_to_today() {
        let now = 1_794_000_000;
        let counts = [
            DayCount {
                day: "2026-11-06".into(),
                kind: ReportKind::Crash,
                count: 2,
            },
            DayCount {
                day: "2026-11-06".into(),
                kind: ReportKind::Bug,
                count: 3,
            },
            DayCount {
                day: "2026-10-08".into(),
                kind: ReportKind::Other,
                count: 1,
            },
            // outside the window: not drawn
            DayCount {
                day: "2026-10-01".into(),
                kind: ReportKind::Bug,
                count: 9,
            },
        ];
        let days = last_days(&counts, now);
        assert_eq!(days.len(), 30);
        assert_eq!(days[0].date, "2026-10-08");
        assert_eq!((days[0].crashes, days[0].others), (0, 1));
        let today = days.last().expect("today");
        assert_eq!(
            (today.date.as_str(), today.crashes, today.others),
            ("2026-11-06", 2, 3)
        );
        assert_eq!(days.iter().map(|d| d.others + d.crashes).sum::<u64>(), 6);
    }

    #[test]
    fn a_history_line_in_words() {
        let name = |id: Option<[u8; 16]>| id.map_or("—".into(), |id| format!("admin {}", id[15]));
        let lang = Lang::En;
        assert_eq!(
            detail_text(lang, "report.status", "new → in_progress", &name).as_deref(),
            Some("New → In progress")
        );
        assert_eq!(
            detail_text(lang, "report.priority", "critical", &name).as_deref(),
            Some("Critical")
        );
        let text = hex(&id(9));
        let uuid = format!(
            "{}-{}-{}-{}-{}",
            &text[..8],
            &text[8..12],
            &text[12..16],
            &text[16..20],
            &text[20..]
        );
        assert_eq!(
            detail_text(lang, "report.assignee", &uuid, &name).as_deref(),
            Some("admin 9")
        );
        assert_eq!(
            detail_text(lang, "report.duplicate", &uuid, &name).as_deref(),
            Some("…10101009")
        );
        assert_eq!(
            detail_text(lang, "report.filed", "by gateway", &name).as_deref(),
            Some("gateway")
        );
        assert_eq!(detail_text(lang, "report.trashed", "", &name), None);
        // a word Central wrote that this page does not know stays as it is: text
        assert_eq!(
            detail_text(lang, "report.labels", "<b>x</b>", &name).as_deref(),
            Some("<b>x</b>")
        );
        assert_eq!(action_text(lang, "report.unknown"), None);
        assert!(action_text(lang, "report.purged").is_some());
    }
}
