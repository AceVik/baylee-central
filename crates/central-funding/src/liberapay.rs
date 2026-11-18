//! Liberapay's public figures of the recipient, which it gives anybody without a key
//! (moved here from `central-web-server`, 18.11.2026, with its bounds). Of the answer
//! two numbers are kept: what is received a week, as a month's amount, and by how
//! many. The answer also names the recipient and carries his own texts; none of that is
//! deserialised, kept or passed on, and nothing in it names a donor.

use central_api::funding::Monthly;
use serde::Deserialize;

/// Liberapay's public page of the recipient, as JSON.
pub const ADDRESS: &str = "https://liberapay.com/AceVik/public.json";

/// The most of an answer that is read. The real one is about 4 KiB.
pub const LIMIT: u64 = 64 * 1024;

/// The most a week that is believed, in cents: beyond it the answer is taken for
/// broken and dropped (a hundred thousand euros).
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

/// Liberapay's answer as the two figures kept of it: what is received, as a month's
/// amount in euro cents (a week's counts as 52/12 of a month's), and from how many.
/// `None` for an answer that is not JSON of this shape, hides the amount, is in another
/// currency or is beyond belief.
#[must_use]
pub fn read(text: &str) -> Option<Monthly> {
    let public: Public = serde_json::from_str(text).ok()?;
    let receiving = public.receiving?;
    if receiving.currency != "EUR" {
        return None;
    }
    let week = crate::amount::hundredths(&receiving.amount).filter(|week| *week <= MOST)?;
    Some(Monthly {
        month: week * 52 / 12,
        supporters: public.npatrons.unwrap_or(0).min(1_000_000),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
            r#"{"receiving":{"amount":"-5.00","currency":"EUR"}}"#,
            r#"{"receiving":{"amount":"5.00","currency":"EUR"},"npatrons":-1}"#,
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
        // the bounds hold: the most a week, as a month, with the count capped
        assert_eq!(
            read(r#"{"receiving":{"amount":"100000.00","currency":"EUR"},"npatrons":99999999}"#),
            Some(Monthly {
                month: 43_333_333,
                supporters: 1_000_000
            })
        );
    }
}
