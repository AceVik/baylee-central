//! Locale-aware date/time formatting and calendar arithmetic for the
//! `DatePicker` and Security sessions table.

use crate::words::Lang;

/// Parse an ISO `YYYY-MM-DD` date string into `(year, month 1..=12, day 1..=31)`.
#[must_use]
pub fn parse_iso_date(text: &str) -> Option<(i32, u8, u8)> {
    let trimmed = text.trim();
    if trimmed.len() != 10 {
        return None;
    }
    let bytes = trimmed.as_bytes();
    if bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    let year: i32 = trimmed[0..4].parse().ok()?;
    let month: u8 = trimmed[5..7].parse().ok()?;
    let day: u8 = trimmed[8..10].parse().ok()?;
    if !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month) {
        return None;
    }
    Some((year, month, day))
}

/// Format `(year, month, day)` as canonical `YYYY-MM-DD`.
#[must_use]
pub fn format_iso_date(year: i32, month: u8, day: u8) -> String {
    format!("{year:04}-{month:02}-{day:02}")
}

/// Whether `year` is a Gregorian leap year.
#[must_use]
pub const fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Number of days in Gregorian `(year, month 1..=12)`.
#[must_use]
pub const fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

/// Day of week (`0 = Monday .. 6 = Sunday`) for `(year, month 1..=12, day 1..=31)`.
#[must_use]
pub fn weekday_monday0(year: i32, month: u8, day: u8) -> u8 {
    // Sakamoto's algorithm yields 0 = Sunday .. 6 = Saturday.
    let offsets = [0_i32, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = year - i32::from(month < 3);
    let m = usize::from(month.saturating_sub(1).min(11));
    let w = (y + y / 4 - y / 100 + y / 400 + offsets[m] + i32::from(day)).rem_euclid(7);
    let sun0 = u8::try_from(w).unwrap_or(0);
    (sun0 + 6) % 7
}

/// Format an ISO date string (`YYYY-MM-DD`) in `lang`.
#[must_use]
#[cfg_attr(not(any(test, feature = "admin")), allow(dead_code))]
pub fn format_date_locale(lang: Lang, iso: &str) -> String {
    #[cfg(target_arch = "wasm32")]
    if let Some((y, m, d)) = parse_iso_date(iso)
        && let Some(formatted) = intl_format_ymd(lang, y, m, d)
    {
        return formatted;
    }
    crate::format::date(lang, iso)
}

/// Format a Unix timestamp in seconds using the browser's `Intl.DateTimeFormat`
/// in the selected `lang` and visitor's local timezone.
#[must_use]
pub fn format_datetime_local(lang: Lang, unix_secs: i64) -> String {
    #[cfg(target_arch = "wasm32")]
    if let Some(formatted) = intl_format_unix(lang, unix_secs, true) {
        return formatted;
    }
    let (y, m, d) = central_api::profile::utc_date_from_unix_secs(unix_secs);
    let iso = format_iso_date(y, m, d);
    crate::format::date(lang, &iso)
}

/// Format a Unix timestamp in seconds as a localized date only (`dateStyle: "long"`).
#[must_use]
pub fn format_unix_date_local(lang: Lang, unix_secs: i64) -> String {
    #[cfg(target_arch = "wasm32")]
    if let Some(formatted) = intl_format_unix(lang, unix_secs, false) {
        return formatted;
    }
    let (y, m, d) = central_api::profile::utc_date_from_unix_secs(unix_secs);
    let iso = format_iso_date(y, m, d);
    crate::format::date(lang, &iso)
}

/// Format an ISO/RFC3339 date-time or `YYYY-MM-DD` string in `lang` using the
/// visitor's local timezone (when a time component is present).
#[must_use]
#[cfg_attr(not(any(test, feature = "admin")), allow(dead_code))]
pub fn format_iso_datetime_local(lang: Lang, raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if parse_iso_date(trimmed).is_some() {
        return format_date_locale(lang, trimmed);
    }
    #[cfg(target_arch = "wasm32")]
    if let Some(formatted) = intl_format_iso_datetime(lang, trimmed) {
        return formatted;
    }
    if let Some(date_part) = trimmed.get(..10)
        && parse_iso_date(date_part).is_some()
    {
        let date = crate::format::date(lang, date_part);
        let rest = trimmed[10..].trim_start_matches(['T', ' ']);
        if rest.len() >= 5
            && rest.as_bytes()[2] == b':'
            && rest[..2].bytes().all(|b| b.is_ascii_digit())
            && rest[3..5].bytes().all(|b| b.is_ascii_digit())
        {
            return format!("{date}, {}", &rest[..5]);
        }
        return date;
    }
    trimmed.to_owned()
}

/// Seven short weekday names starting with Monday, localized via `Intl.DateTimeFormat`.
#[must_use]
pub fn weekday_shorts(lang: Lang) -> [String; 7] {
    #[cfg(target_arch = "wasm32")]
    if let Some(days) = intl_weekdays(lang) {
        return days;
    }
    match lang {
        Lang::De => ["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"].map(str::to_owned),
        _ => ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"].map(str::to_owned),
    }
}

#[cfg(target_arch = "wasm32")]
fn intl_format_ms(lang: Lang, unix_ms: f64, with_time: bool) -> Option<String> {
    use wasm_bindgen::{JsCast, JsValue};

    let intl = js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("Intl")).ok()?;
    let dtf: js_sys::Function = js_sys::Reflect::get(&intl, &JsValue::from_str("DateTimeFormat"))
        .ok()?
        .dyn_into()
        .ok()?;
    let opts = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &opts,
        &JsValue::from_str("dateStyle"),
        &JsValue::from_str("medium"),
    );
    if with_time {
        let _ = js_sys::Reflect::set(
            &opts,
            &JsValue::from_str("timeStyle"),
            &JsValue::from_str("short"),
        );
    }
    let formatter = js_sys::Reflect::construct(
        &dtf,
        &js_sys::Array::of2(&JsValue::from_str(lang.tag()), &opts),
    )
    .ok()?;
    let format_fn: js_sys::Function =
        js_sys::Reflect::get(&formatter, &JsValue::from_str("format"))
            .ok()?
            .dyn_into()
            .ok()?;
    let date = js_sys::Date::new(&JsValue::from_f64(unix_ms));
    format_fn.call1(&formatter, &date).ok()?.as_string()
}

#[cfg(target_arch = "wasm32")]
fn intl_format_unix(lang: Lang, unix_secs: i64, with_time: bool) -> Option<String> {
    // IEEE-754 `f64` represents exact integer Unix timestamps far beyond modern epochs.
    #[allow(clippy::cast_precision_loss)]
    let unix_ms = (unix_secs as f64) * 1000.0;
    intl_format_ms(lang, unix_ms, with_time)
}

#[cfg(target_arch = "wasm32")]
#[cfg_attr(not(any(test, feature = "admin")), allow(dead_code))]
fn intl_format_iso_datetime(lang: Lang, raw: &str) -> Option<String> {
    let ms = js_sys::Date::parse(raw);
    if ms.is_finite() {
        intl_format_ms(lang, ms, true)
    } else {
        None
    }
}

#[cfg(target_arch = "wasm32")]
#[cfg_attr(not(any(test, feature = "admin")), allow(dead_code))]
fn intl_format_ymd(lang: Lang, year: i32, month: u8, day: u8) -> Option<String> {
    use wasm_bindgen::{JsCast, JsValue};

    let intl = js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("Intl")).ok()?;
    let dtf: js_sys::Function = js_sys::Reflect::get(&intl, &JsValue::from_str("DateTimeFormat"))
        .ok()?
        .dyn_into()
        .ok()?;
    let opts = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &opts,
        &JsValue::from_str("year"),
        &JsValue::from_str("numeric"),
    );
    let _ = js_sys::Reflect::set(
        &opts,
        &JsValue::from_str("month"),
        &JsValue::from_str("long"),
    );
    let _ = js_sys::Reflect::set(
        &opts,
        &JsValue::from_str("day"),
        &JsValue::from_str("numeric"),
    );
    let _ = js_sys::Reflect::set(
        &opts,
        &JsValue::from_str("timeZone"),
        &JsValue::from_str("UTC"),
    );
    let formatter = js_sys::Reflect::construct(
        &dtf,
        &js_sys::Array::of2(&JsValue::from_str(lang.tag()), &opts),
    )
    .ok()?;
    let format_fn: js_sys::Function =
        js_sys::Reflect::get(&formatter, &JsValue::from_str("format"))
            .ok()?
            .dyn_into()
            .ok()?;
    let ms = utc_ms(year, month, day);
    let date = js_sys::Date::new(&JsValue::from_f64(ms));
    format_fn.call1(&formatter, &date).ok()?.as_string()
}

#[cfg(target_arch = "wasm32")]
fn utc_ms(year: i32, month: u8, day: u8) -> f64 {
    js_sys::Date::utc(f64::from(year), f64::from(month.saturating_sub(1)))
        + f64::from(day.saturating_sub(1)) * 86_400_000.0
}

#[cfg(target_arch = "wasm32")]
fn intl_weekdays(lang: Lang) -> Option<[String; 7]> {
    use wasm_bindgen::{JsCast, JsValue};

    let intl = js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("Intl")).ok()?;
    let dtf: js_sys::Function = js_sys::Reflect::get(&intl, &JsValue::from_str("DateTimeFormat"))
        .ok()?
        .dyn_into()
        .ok()?;
    let opts = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &opts,
        &JsValue::from_str("weekday"),
        &JsValue::from_str("short"),
    );
    let _ = js_sys::Reflect::set(
        &opts,
        &JsValue::from_str("timeZone"),
        &JsValue::from_str("UTC"),
    );
    let formatter = js_sys::Reflect::construct(
        &dtf,
        &js_sys::Array::of2(&JsValue::from_str(lang.tag()), &opts),
    )
    .ok()?;
    let format_fn: js_sys::Function =
        js_sys::Reflect::get(&formatter, &JsValue::from_str("format"))
            .ok()?
            .dyn_into()
            .ok()?;
    // 2024-01-01 is a Monday.
    let mut out = Vec::with_capacity(7);
    for day in 1_u8..=7 {
        let ms = utc_ms(2024, 1, day);
        let date = js_sys::Date::new(&JsValue::from_f64(ms));
        out.push(format_fn.call1(&formatter, &date).ok()?.as_string()?);
    }
    out.try_into().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dates_and_computes_weekdays() {
        assert_eq!(parse_iso_date("2000-02-29"), Some((2000, 2, 29)));
        assert_eq!(parse_iso_date("2001-02-29"), None);
        assert_eq!(parse_iso_date("not-a-date"), None);
        // 2024-01-01 was a Monday -> 0
        assert_eq!(weekday_monday0(2024, 1, 1), 0);
        // 2026-11-23 is a Monday -> 0
        assert_eq!(weekday_monday0(2026, 11, 23), 0);
    }

    #[test]
    fn formats_iso_datetime_fallbacks() {
        assert_eq!(
            format_iso_datetime_local(Lang::En, "2026-11-24"),
            crate::format::date(Lang::En, "2026-11-24")
        );
        assert_eq!(
            format_iso_datetime_local(Lang::De, "2026-11-24T14:35:00Z"),
            format!("{}, 14:35", crate::format::date(Lang::De, "2026-11-24"))
        );
        assert_eq!(format_iso_datetime_local(Lang::En, "  "), "");
    }
}
