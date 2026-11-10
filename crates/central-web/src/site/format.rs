//! Dates and sizes as each language writes them.

use crate::words::Lang;

const MONTHS_EN: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const MONTHS_DE: [&str; 12] = [
    "Januar",
    "Februar",
    "März",
    "April",
    "Mai",
    "Juni",
    "Juli",
    "August",
    "September",
    "Oktober",
    "November",
    "Dezember",
];

/// `YYYY-MM-DD` as the language writes a date (`10 November 2026`,
/// `10. November 2026`); anything else as it is.
#[must_use]
pub fn date(lang: Lang, iso: &str) -> String {
    let mut parts = iso.get(..10).unwrap_or(iso).split('-');
    let (Some(year), Some(month), Some(day)) = (parts.next(), parts.next(), parts.next()) else {
        return iso.to_owned();
    };
    let (Ok(month), Ok(day)) = (month.parse::<usize>(), day.parse::<u8>()) else {
        return iso.to_owned();
    };
    let Some(index) = month.checked_sub(1).filter(|m| *m < 12) else {
        return iso.to_owned();
    };
    match lang {
        Lang::En => format!("{day} {} {year}", MONTHS_EN[index]),
        Lang::De => format!("{day}. {} {year}", MONTHS_DE[index]),
    }
}

/// A size in bytes as megabytes with one decimal: `12.3` or `12,3`.
#[must_use]
pub fn megabytes(lang: Lang, bytes: u64) -> String {
    let tenths = bytes.saturating_add(50_000) / 100_000;
    let separator = match lang {
        Lang::En => '.',
        Lang::De => ',',
    };
    format!("{}{separator}{}", tenths / 10, tenths % 10)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_and_sizes() {
        assert_eq!(date(Lang::En, "2026-11-10"), "10 November 2026");
        assert_eq!(date(Lang::De, "2026-03-01T12:00:00Z"), "1. März 2026");
        assert_eq!(date(Lang::En, "soon"), "soon");
        assert_eq!(date(Lang::En, "2026-13-01"), "2026-13-01");
        assert_eq!(megabytes(Lang::En, 12_345_678), "12.3");
        assert_eq!(megabytes(Lang::De, 99_960_000), "100,0");
    }
}
