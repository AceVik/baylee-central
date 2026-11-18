//! What the donation platforms publish of the monthly goal
//! (`central_web::data::Funding`): today Liberapay's public figures, which
//! it gives anybody without a key. The server asks, never a reader's
//! browser, and keeps two numbers of the answer: what is given per week (as
//! a month's amount) and by how many. The answer also names the recipient and carries his own
//! texts; none of that is read, kept or passed on, and nothing in it names a
//! donor.
//!
//! GitHub Sponsors and Ko-fi publish no figure without a key or a webhook,
//! and this server holds no secret (`config.rs`), so it does not ask them:
//! their figures are Baylee Central's to collect.

use std::time::Duration;

use central_web::data::Monthly;
use serde::Deserialize;

/// Liberapay's public page of the recipient, as JSON.
const LIBERAPAY: &str = "https://liberapay.com/AceVik/public.json";

/// The most of an answer that is read. The real one is about 4 KiB.
const LIMIT: u64 = 64 * 1024;

/// How often the platforms are asked. Liberapay's figures change with its
/// weekly payday and when somebody starts or stops giving.
pub const EVERY: Duration = Duration::from_mins(30);

/// The most a week that is believed, in cents: beyond it the answer is
/// taken for broken and dropped (a hundred thousand euros).
const MOST: u64 = 10_000_000;

#[derive(Deserialize)]
struct Amount {
    amount: String,
    currency: String,
}

#[derive(Deserialize)]
struct Public {
    #[serde(default)]
    receiving: Option<Amount>,
    #[serde(default)]
    npatrons: Option<u64>,
}

/// A platform's decimal amount (`185.00`, `7`, `0.5`) in cents: digits,
/// then at most one point with one or two digits. Nothing else is an
/// amount: no sign, no exponent, no grouping, no blank.
fn cents(text: &str) -> Option<u64> {
    let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
    let digits = |part: &str| part.bytes().all(|b| b.is_ascii_digit());
    if whole.is_empty() || whole.len() > 9 || !digits(whole) || fraction.len() > 2 {
        return None;
    }
    if text.contains('.') && (fraction.is_empty() || !digits(fraction)) {
        return None;
    }
    let whole: u64 = whole.parse().ok()?;
    let part: u64 = match fraction.len() {
        0 => 0,
        1 => fraction.parse::<u64>().ok()? * 10,
        _ => fraction.parse().ok()?,
    };
    whole.checked_mul(100)?.checked_add(part)
}

/// Liberapay's answer as the two figures kept of it: what is received, as
/// a month's amount in euro cents (a week's counts as 52/12 of a month's),
/// and from how many. `None` for an answer that is
/// not JSON of this shape, hides the amount, is in another currency or is
/// beyond belief.
#[must_use]
pub fn read(text: &str) -> Option<Monthly> {
    let public: Public = serde_json::from_str(text).ok()?;
    let receiving = public.receiving?;
    if receiving.currency != "EUR" {
        return None;
    }
    let week = cents(&receiving.amount).filter(|week| *week <= MOST)?;
    Some(Monthly {
        month: week * 52 / 12,
        supporters: public.npatrons.unwrap_or(0).min(1_000_000),
    })
}

/// Asks Liberapay.
#[must_use]
pub fn liberapay(agent: &ureq::Agent) -> Option<Monthly> {
    let mut response = agent
        .get(LIBERAPAY)
        .header("Accept", "application/json")
        .call()
        .ok()?;
    if response.status() != 200 {
        return None;
    }
    let text = response
        .body_mut()
        .with_config()
        .limit(LIMIT)
        .read_to_string()
        .ok()?;
    read(&text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_amount_is_digits_with_at_most_two_after_the_point() {
        for (text, value) in [
            ("0", Some(0)),
            ("0.00", Some(0)),
            ("7", Some(700)),
            ("0.5", Some(50)),
            ("185.00", Some(18_500)),
            ("23.10", Some(2310)),
            ("999999999.99", Some(99_999_999_999)),
            ("", None),
            (".", None),
            ("1.", None),
            (".5", None),
            ("-1", None),
            ("+1", None),
            ("1e3", None),
            ("1,5", None),
            ("1.234", None),
            (" 1", None),
            ("1 ", None),
            ("1.2.3", None),
            ("1234567890", None),
            ("١٢", None),
        ] {
            assert_eq!(cents(text), value, "{text:?}");
        }
    }

    #[test]
    fn of_liberapay_s_answer_two_figures_are_kept() {
        // the shape of the real answer (18.11.2026), with a stand-in's values
        let text = r#"{"avatar":"https://example.org/a.png","display_name":"Somebody",
            "giving":{"amount":"0.00","currency":"EUR"},
            "goal":{"amount":"185.00","currency":"EUR"},
            "id":1,"kind":"individual","npatrons":3,
            "receiving":{"amount":"23.10","currency":"EUR"},
            "statements":[{"content":"words","lang":"en"}],"username":"Somebody"}"#;
        assert_eq!(
            read(text),
            Some(Monthly {
                month: 10_010,
                supporters: 3
            })
        );
    }

    #[test]
    fn an_answer_that_says_no_amount_in_euros_is_dropped() {
        for text in [
            "",
            "[]",
            "{}",
            r#"{"receiving":null,"npatrons":3}"#,
            r#"{"receiving":{"amount":"5.00","currency":"USD"},"npatrons":1}"#,
            r#"{"receiving":{"amount":"lots","currency":"EUR"}}"#,
            r#"{"receiving":{"amount":"100000.01","currency":"EUR"}}"#,
            r#"{"receiving":{"amount":5,"currency":"EUR"}}"#,
        ] {
            assert_eq!(read(text), None, "{text}");
        }
        // without a count the amount still stands
        assert_eq!(
            read(r#"{"receiving":{"amount":"5.00","currency":"EUR"}}"#),
            Some(Monthly {
                month: 2166,
                supporters: 0
            })
        );
    }
}
