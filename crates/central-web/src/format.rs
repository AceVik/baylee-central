//! Dates and sizes as each language writes them (CLDR's long date, as
//! `Intl.DateTimeFormat` gives it; the tests hold the two together).

use crate::words::Lang;

// A month's name as it stands inside a date: Russian's in the genitive.
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
const MONTHS_FR: [&str; 12] = [
    "janvier",
    "février",
    "mars",
    "avril",
    "mai",
    "juin",
    "juillet",
    "août",
    "septembre",
    "octobre",
    "novembre",
    "décembre",
];
const MONTHS_IT: [&str; 12] = [
    "gennaio",
    "febbraio",
    "marzo",
    "aprile",
    "maggio",
    "giugno",
    "luglio",
    "agosto",
    "settembre",
    "ottobre",
    "novembre",
    "dicembre",
];
const MONTHS_ES: [&str; 12] = [
    "enero",
    "febrero",
    "marzo",
    "abril",
    "mayo",
    "junio",
    "julio",
    "agosto",
    "septiembre",
    "octubre",
    "noviembre",
    "diciembre",
];
const MONTHS_PT_BR: [&str; 12] = [
    "janeiro",
    "fevereiro",
    "março",
    "abril",
    "maio",
    "junho",
    "julho",
    "agosto",
    "setembro",
    "outubro",
    "novembro",
    "dezembro",
];
const MONTHS_RU: [&str; 12] = [
    "января",
    "февраля",
    "марта",
    "апреля",
    "мая",
    "июня",
    "июля",
    "августа",
    "сентября",
    "октября",
    "ноября",
    "декабря",
];

/// `YYYY-MM-DD` as the language writes a date (`10 November 2026`,
/// `10. November 2026`, `2026年11月10日`); anything else as it is.
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
        Lang::Fr => format!("{day} {} {year}", MONTHS_FR[index]),
        Lang::It => format!("{day} {} {year}", MONTHS_IT[index]),
        Lang::Es => format!("{day} de {} de {year}", MONTHS_ES[index]),
        Lang::PtBr => format!("{day} de {} de {year}", MONTHS_PT_BR[index]),
        Lang::Ru => format!("{day} {} {year} г.", MONTHS_RU[index]),
        Lang::Ja | Lang::ZhHans | Lang::ZhHant => format!("{year}年{month}月{day}日"),
        Lang::Ko => format!("{year}년 {month}월 {day}일"),
    }
}

/// A size in bytes as megabytes with one decimal: `12.3` or `12,3`.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn megabytes(lang: Lang, bytes: u64) -> String {
    let tenths = bytes.saturating_add(50_000) / 100_000;
    let separator = match lang {
        Lang::En | Lang::Ja | Lang::Ko | Lang::ZhHans | Lang::ZhHant => '.',
        Lang::De | Lang::Fr | Lang::It | Lang::Es | Lang::PtBr | Lang::Ru => ',',
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
        assert_eq!(megabytes(Lang::Fr, 12_345_678), "12,3");
        assert_eq!(megabytes(Lang::Ja, 12_345_678), "12.3");
    }

    #[test]
    fn dates_as_intl_writes_them() {
        // Intl.DateTimeFormat, day numeric, month long, year numeric
        const DAYS: [&str; 4] = ["2026-11-10", "2026-03-01", "2026-01-31", "2026-08-05"];
        // the four days of a language, one after the other
        let expected: [(Lang, &str); 11] = [
            (
                Lang::En,
                "10 November 2026|1 March 2026|31 January 2026|5 August 2026",
            ),
            (
                Lang::De,
                "10. November 2026|1. März 2026|31. Januar 2026|5. August 2026",
            ),
            (
                Lang::Fr,
                "10 novembre 2026|1 mars 2026|31 janvier 2026|5 août 2026",
            ),
            (
                Lang::It,
                "10 novembre 2026|1 marzo 2026|31 gennaio 2026|5 agosto 2026",
            ),
            (
                Lang::Es,
                "10 de noviembre de 2026|1 de marzo de 2026|31 de enero de 2026|5 de agosto de 2026",
            ),
            (
                Lang::PtBr,
                "10 de novembro de 2026|1 de março de 2026|31 de janeiro de 2026|5 de agosto de 2026",
            ),
            (
                Lang::Ru,
                "10 ноября 2026 г.|1 марта 2026 г.|31 января 2026 г.|5 августа 2026 г.",
            ),
            (
                Lang::Ja,
                "2026年11月10日|2026年3月1日|2026年1月31日|2026年8月5日",
            ),
            (
                Lang::Ko,
                "2026년 11월 10일|2026년 3월 1일|2026년 1월 31일|2026년 8월 5일",
            ),
            (
                Lang::ZhHans,
                "2026年11月10日|2026年3月1日|2026年1月31日|2026年8月5日",
            ),
            (
                Lang::ZhHant,
                "2026年11月10日|2026年3月1日|2026年1月31日|2026年8月5日",
            ),
        ];
        for (lang, written) in expected {
            for (day, want) in DAYS.into_iter().zip(written.split('|')) {
                assert_eq!(date(lang, day), want, "{lang:?} {day}");
            }
            assert_eq!(written.split('|').count(), DAYS.len());
        }
        assert_eq!(Lang::ALL.len(), expected.len());
    }
}
