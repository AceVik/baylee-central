//! Times in the messenger, in the browser's own time zone: a message's
//! clock time, a day's heading and a list row's short time.

use crate::format;
use crate::words::{self, Lang};

/// A moment in the local calendar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Local {
    year: i64,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
}

impl Local {
    fn days(self) -> i64 {
        // days since 1970-01-01 of the calendar date (Howard Hinnant)
        let (year, month) = if self.month <= 2 {
            (self.year - 1, i64::from(self.month) + 9)
        } else {
            (self.year, i64::from(self.month) - 3)
        };
        let era = year.div_euclid(400);
        let yoe = year - era * 400;
        let doy = (153 * month + 2) / 5 + i64::from(self.day) - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    fn iso(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// Now, in Unix milliseconds.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn now() -> u64 {
    format!("{:.0}", js_sys::Date::now().trunc())
        .parse()
        .unwrap_or_default()
}

/// Now, in Unix milliseconds.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or_default()
}

/// `at` (Unix milliseconds) in the browser's time zone.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn local(at: u64) -> Local {
    let millis: f64 = at.to_string().parse().unwrap_or_default();
    let date = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(millis));
    Local {
        year: i64::from(date.get_full_year()),
        month: date.get_month() + 1,
        day: date.get_date(),
        hour: date.get_hours(),
        minute: date.get_minutes(),
    }
}

/// `at` (Unix milliseconds) in UTC: the server never draws a message.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn local(at: u64) -> Local {
    let seconds = i64::try_from(at / 1000).unwrap_or(i64::MAX);
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
    Local {
        year: yoe + era * 400 + i64::from(month <= 2),
        month: u32::try_from(month).unwrap_or(1),
        day: u32::try_from(day).unwrap_or(1),
        hour: u32::try_from(rest / 3600).unwrap_or(0),
        minute: u32::try_from(rest % 3600 / 60).unwrap_or(0),
    }
}

/// The clock time, `14:02`.
#[must_use]
pub fn clock(at: u64) -> String {
    let t = local(at);
    format!("{:02}:{:02}", t.hour, t.minute)
}

/// Whether two moments fall on the same local day.
#[must_use]
pub fn same_day(a: u64, b: u64) -> bool {
    local(a).days() == local(b).days()
}

/// A day's heading: today, yesterday, or the date.
#[must_use]
pub fn day_label(lang: Lang, at: u64, now: u64) -> String {
    let (then, today) = (local(at), local(now));
    match today.days() - then.days() {
        ..=0 => words::chat_today(lang).to_owned(),
        1 => words::chat_yesterday(lang).to_owned(),
        _ => format::date(lang, &then.iso()),
    }
}

/// A list row's time: the clock today, "yesterday", else a short date.
#[must_use]
pub fn short(lang: Lang, at: u64, now: u64) -> String {
    let (then, today) = (local(at), local(now));
    match today.days() - then.days() {
        ..=0 => clock(at),
        1 => words::chat_yesterday(lang).to_owned(),
        _ => match lang {
            Lang::En => {
                let month = format::date(lang, &then.iso());
                let name = month.split(' ').nth(1).unwrap_or_default();
                format!("{} {}", then.day, name.chars().take(3).collect::<String>())
            }
            Lang::De => format!("{}.{}.", then.day, then.month),
        },
    }
}

/// A machine-readable time for `<time datetime>`.
#[must_use]
pub fn machine(at: u64) -> String {
    let t = local(at);
    format!("{}T{:02}:{:02}", t.iso(), t.hour, t.minute)
}

#[cfg(test)]
mod tests {
    use super::*;

    // 2026-11-10 14:02 UTC
    const AT: u64 = 1_794_319_320_000;

    #[test]
    fn labels() {
        assert_eq!(clock(AT), "14:02");
        assert_eq!(machine(AT), "2026-11-10T14:02");
        assert_eq!(day_label(Lang::En, AT, AT + 3_600_000), "Today");
        assert_eq!(day_label(Lang::De, AT, AT + 86_400_000), "Gestern");
        assert_eq!(
            day_label(Lang::En, AT, AT + 3 * 86_400_000),
            "10 November 2026"
        );
        assert_eq!(short(Lang::En, AT, AT + 3 * 86_400_000), "10 Nov");
        assert_eq!(short(Lang::De, AT, AT + 3 * 86_400_000), "10.11.");
        assert!(same_day(AT, AT + 60_000));
        assert!(!same_day(AT, AT + 86_400_000));
        assert_eq!(local(AT).days(), 20_767);
    }
}
