//! Amounts and currencies as the platforms write them.

/// A platform's decimal amount (`185.00`, `7`, `0.5`) in hundredths of its currency:
/// one to nine digits, then at most one point with one or two digits. Nothing else is
/// an amount: no sign, no exponent, no grouping, no blank, no other script's digits.
/// Liberapay's figure and Ko-fi's payments are read by this one rule.
#[must_use]
pub fn hundredths(text: &str) -> Option<u64> {
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

/// A currency's code: three capital Latin letters (ISO 4217's shape). Whether Central
/// knows a rate for it is the reference rates' to say.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Currency([u8; 3]);

impl Currency {
    /// The euro, which needs no rate.
    pub const EUR: Self = Self(*b"EUR");
    /// The US dollar, GitHub Sponsors' currency.
    pub const USD: Self = Self(*b"USD");

    /// `text` as a currency's code, if it is three capital Latin letters.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let bytes: [u8; 3] = text.as_bytes().try_into().ok()?;
        bytes
            .iter()
            .all(u8::is_ascii_uppercase)
            .then_some(Self(bytes))
    }

    /// The code as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        // three ASCII capitals, checked when it was made
        std::str::from_utf8(&self.0).unwrap_or("XXX")
    }
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
            ("1E3", None),
            ("1,5", None),
            ("1.234", None),
            (" 1", None),
            ("1 ", None),
            ("1.2.3", None),
            ("1234567890", None),
            ("١٢", None),
            ("１２", None),
            ("0x10", None),
            ("NaN", None),
            ("inf", None),
            ("1_000", None),
            ("1\u{a0}000", None),
        ] {
            assert_eq!(hundredths(text), value, "{text:?}");
        }
    }

    #[test]
    fn a_currency_is_three_capital_letters() {
        for code in ["EUR", "USD", "JPY", "XXX"] {
            assert_eq!(
                Currency::parse(code).map(|c| c.as_str().to_owned()),
                Some(code.into())
            );
        }
        for text in [
            "", "eur", "Eur", "EU", "EURO", "E1R", "€", "ÄUR", "EU ", "<b>",
        ] {
            assert_eq!(Currency::parse(text), None, "{text:?}");
        }
        assert_eq!(Currency::parse("EUR"), Some(Currency::EUR));
    }
}
