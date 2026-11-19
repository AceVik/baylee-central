//! Ko-fi's webhook: a delivery to Central for each payment, which is the only way
//! Ko-fi tells anybody of one. Its shape is taken from descriptions of the format and
//! is **not confirmed** by Ko-fi itself: the first test delivery from Ko-fi's settings
//! decides (`docs/operations.md` §"Donations"). Whatever is not of the shape read here
//! is refused cleanly, and counted by its reason.
//!
//! A delivery is a form with one field, `data`, holding JSON. It is read in two steps:
//! first the verification token alone, compared in constant time with the one Central
//! holds (as a SHA-256 digest); only then the fields a sum needs. Neither step names
//! the giver's name, address, message, shipping or link, so none of them is parsed into
//! anything, kept or logged; nothing here keeps the body either.

use crate::{Currency, amount::hundredths};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

/// The most of a delivery's body that is taken: a payment with a long message fits
/// several times over. A longer body is refused as [`Refusal::Size`]; the listener
/// reads up to [`READ`] so that such a refusal is counted, and refuses a body longer
/// than that before reading it.
pub const LIMIT: usize = 16 * 1024;

/// The most of a delivery's body the listener reads at all: four times [`LIMIT`], the
/// light bound of the account listener's boundary.
pub const READ: usize = 4 * LIMIT;

/// The most one payment may be, in hundredths of its currency: the amount rule's own
/// bound. A sum in euros is bounded again when it is converted.
pub const MOST: u64 = 99_999_999_999;

/// Why a delivery was refused. Each is counted on its own, so the operator can tell
/// a format Ko-fi changed from a token that is wrong.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The body is not a form (`application/x-www-form-urlencoded`).
    ContentType,
    /// The form has no single `data` field, or other fields beside it.
    Form,
    /// `data` is not JSON, or its token is not text.
    Json,
    /// The token is missing or not the one Central holds.
    Token,
    /// A field a sum needs is missing, of another type or of another shape.
    Shape,
    /// The amount is not an amount, is nothing, or is beyond belief.
    Amount,
    /// The currency is not a currency's code.
    Currency,
    /// The body is longer than [`LIMIT`].
    Size,
}

impl Refusal {
    /// Every reason, in the order the operator's line names them.
    pub const ALL: [Self; 8] = [
        Self::ContentType,
        Self::Form,
        Self::Json,
        Self::Token,
        Self::Shape,
        Self::Amount,
        Self::Currency,
        Self::Size,
    ];

    /// The reason's name in the operator's line.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ContentType => "content type",
            Self::Form => "form",
            Self::Json => "json",
            Self::Token => "token",
            Self::Shape => "shape",
            Self::Amount => "amount",
            Self::Currency => "currency",
            Self::Size => "size",
        }
    }
}

/// Whether a payment is given once or is one of a membership's regular payments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A donation given once.
    Once,
    /// A regular payment.
    Recurring,
}

/// What Central keeps of a payment: no word about who made it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Payment {
    /// Given once, or regularly.
    pub kind: Kind,
    /// The amount, in hundredths of its currency.
    pub amount: u64,
    /// Its currency.
    pub currency: Currency,
    /// How Central remembers Ko-fi's transaction ([`crate::reference`]).
    pub reference: [u8; 32],
}

/// A delivery with the right token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Delivery {
    /// A donation, once or regular.
    Payment(Payment),
    /// A sale (a commission, a shop order): something was bought, so it is no
    /// donation and is not counted.
    Sale,
}

/// The digest Central holds of the verification token: the token itself is never kept
/// past the start.
#[must_use]
pub fn digest(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

/// Whether a token from the configuration has the shape of one: 8 to 128 visible ASCII
/// characters. Ko-fi's are UUIDs; the bound only keeps a mistake out.
#[must_use]
pub fn plain_token(token: &str) -> bool {
    (8..=128).contains(&token.len()) && token.bytes().all(|b| b.is_ascii_graphic())
}

/// Step one: the token alone. Every other field is passed over without being read.
#[derive(Deserialize)]
struct Verification {
    #[serde(default)]
    verification_token: Option<String>,
}

/// Step two: the fields a sum needs, and no other.
#[derive(Deserialize)]
struct Fields {
    #[serde(rename = "type")]
    kind: String,
    amount: String,
    currency: String,
    is_subscription_payment: bool,
    is_first_subscription_payment: bool,
    kofi_transaction_id: String,
}

/// Whether the header names a form, with or without parameters.
fn form(content_type: Option<&[u8]>) -> bool {
    let Some(value) = content_type.and_then(|value| std::str::from_utf8(value).ok()) else {
        return false;
    };
    let essence = value.split(';').next().unwrap_or("").trim();
    essence.eq_ignore_ascii_case("application/x-www-form-urlencoded")
}

/// The form's one `data` field.
fn data(body: &[u8]) -> Result<String, Refusal> {
    let mut found = None;
    for (key, value) in form_urlencoded::parse(body) {
        if key != "data" || found.is_some() {
            return Err(Refusal::Form);
        }
        found = Some(value.into_owned());
    }
    found.ok_or(Refusal::Form)
}

/// Reads one delivery of `body` with the header `content_type`, for the token whose
/// digest is `expected`. The token is checked before any other field is looked at, and
/// a missing token is refused as a wrong one.
///
/// # Errors
/// The [`Refusal`] that stopped it.
pub fn read(
    content_type: Option<&[u8]>,
    body: &[u8],
    expected: &[u8; 32],
) -> Result<Delivery, Refusal> {
    if !form(content_type) {
        return Err(Refusal::ContentType);
    }
    if body.len() > LIMIT {
        return Err(Refusal::Size);
    }
    let data = data(body)?;
    // An object, and not the array serde would also read a struct from.
    if !data.trim_start().starts_with('{') {
        return Err(Refusal::Json);
    }
    let verification: Verification = serde_json::from_str(&data).map_err(|_| Refusal::Json)?;
    let presented = digest(&verification.verification_token.ok_or(Refusal::Token)?);
    if !bool::from(presented.ct_eq(expected)) {
        return Err(Refusal::Token);
    }
    let fields: Fields = serde_json::from_str(&data).map_err(|_| Refusal::Shape)?;
    if !crate::plain_id(&fields.kofi_transaction_id, 64)
        || (fields.is_first_subscription_payment && !fields.is_subscription_payment)
    {
        return Err(Refusal::Shape);
    }
    let kind = match (fields.kind.as_str(), fields.is_subscription_payment) {
        ("Donation", false) => Kind::Once,
        ("Donation" | "Subscription", true) => Kind::Recurring,
        ("Commission" | "Shop Order", _) => return Ok(Delivery::Sale),
        _ => return Err(Refusal::Shape),
    };
    let amount = hundredths(&fields.amount)
        .filter(|amount| (1..=MOST).contains(amount))
        .ok_or(Refusal::Amount)?;
    let currency = Currency::parse(&fields.currency).ok_or(Refusal::Currency)?;
    Ok(Delivery::Payment(Payment {
        kind,
        amount,
        currency,
        reference: crate::reference("kofi", &fields.kofi_transaction_id),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKEN: &str = "synthetic-verification-token-0001";
    const FORM: &[u8] = b"application/x-www-form-urlencoded";

    /// A delivery as the descriptions have it, with made-up values and everything a
    /// giver may say in it.
    fn json(overrides: &[(&str, &str)]) -> String {
        let mut fields: Vec<(String, String)> = [
            ("verification_token", format!("\"{TOKEN}\"")),
            (
                "message_id",
                "\"3a1fac0c-f960-4506-a60e-824979a74e74\"".into(),
            ),
            ("timestamp", "\"2026-11-18T12:00:00Z\"".into()),
            ("type", "\"Donation\"".into()),
            ("is_public", "true".into()),
            ("from_name", "\"Some Giver\"".into()),
            ("message", "\"Good luck!\"".into()),
            ("amount", "\"3.00\"".into()),
            ("url", "\"https://ko-fi.com/Home/CoffeeShop?txid=x\"".into()),
            ("email", "\"giver@example.org\"".into()),
            ("currency", "\"EUR\"".into()),
            ("is_subscription_payment", "false".into()),
            ("is_first_subscription_payment", "false".into()),
            (
                "kofi_transaction_id",
                "\"00000000-1111-2222-3333-444444444444\"".into(),
            ),
            ("shop_items", "null".into()),
            ("tier_name", "null".into()),
            ("shipping", "null".into()),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect();
        for (key, value) in overrides {
            if let Some(field) = fields.iter_mut().find(|(name, _)| name == key) {
                field.1 = (*value).to_owned();
            } else {
                fields.push(((*key).to_owned(), (*value).to_owned()));
            }
        }
        let inner: Vec<String> = fields
            .into_iter()
            .filter(|(_, value)| value != "<absent>")
            .map(|(key, value)| format!("\"{key}\":{value}"))
            .collect();
        format!("{{{}}}", inner.join(","))
    }

    fn body(data: &str) -> Vec<u8> {
        form_urlencoded::Serializer::new(String::new())
            .append_pair("data", data)
            .finish()
            .into_bytes()
    }

    fn delivered(data: &str) -> Result<Delivery, Refusal> {
        read(Some(FORM), &body(data), &digest(TOKEN))
    }

    #[test]
    fn a_donation_keeps_its_kind_amount_currency_and_reference() {
        assert_eq!(
            delivered(&json(&[])),
            Ok(Delivery::Payment(Payment {
                kind: Kind::Once,
                amount: 300,
                currency: Currency::EUR,
                reference: crate::reference("kofi", "00000000-1111-2222-3333-444444444444"),
            }))
        );
        let monthly = json(&[
            ("type", "\"Subscription\""),
            ("is_subscription_payment", "true"),
            ("is_first_subscription_payment", "true"),
            ("amount", "\"5\""),
            ("currency", "\"USD\""),
        ]);
        let Ok(Delivery::Payment(payment)) = delivered(&monthly) else {
            panic!("a payment");
        };
        assert_eq!(
            (payment.kind, payment.amount, payment.currency),
            (Kind::Recurring, 500, Currency::USD)
        );
        // a header with parameters, in another case, is still a form
        assert!(
            read(
                Some(b"Application/X-WWW-Form-Urlencoded; charset=utf-8"),
                &body(&json(&[])),
                &digest(TOKEN)
            )
            .is_ok()
        );
    }

    #[test]
    fn a_sale_is_verified_and_not_counted() {
        for kind in ["\"Commission\"", "\"Shop Order\""] {
            assert_eq!(delivered(&json(&[("type", kind)])), Ok(Delivery::Sale));
        }
    }

    #[test]
    fn a_wrong_or_missing_token_is_refused_before_anything_else_is_read() {
        for token in [
            "\"synthetic-verification-token-0002\"",
            "\"\"",
            "\"synthetic-verification-token-000\"",
            "\"SYNTHETIC-VERIFICATION-TOKEN-0001\"",
            "null",
            "<absent>",
        ] {
            assert_eq!(
                delivered(&json(&[("verification_token", token)])),
                Err(Refusal::Token),
                "{token}"
            );
        }
        // with a wrong token, a broken field is never looked at
        assert_eq!(
            delivered(&json(&[
                ("verification_token", "\"wrong-token-of-length\""),
                ("amount", "\"lots\""),
                ("type", "1"),
            ])),
            Err(Refusal::Token)
        );
        // a token that is no text is no JSON of this shape
        assert_eq!(
            delivered(&json(&[("verification_token", "7")])),
            Err(Refusal::Json)
        );
    }

    #[test]
    fn a_body_that_is_no_delivery_is_refused_by_its_reason() {
        let good = body(&json(&[]));
        let expected = digest(TOKEN);
        assert_eq!(read(None, &good, &expected), Err(Refusal::ContentType));
        for header in [
            &b"application/json"[..],
            b"multipart/form-data",
            b"text/plain",
            b"\xff",
        ] {
            assert_eq!(
                read(Some(header), &good, &expected),
                Err(Refusal::ContentType)
            );
        }
        for form_body in [
            &b""[..],
            b"data",
            b"other=1",
            b"data=1&data=2",
            b"data=%7B%7D&extra=1",
        ] {
            let refused = read(Some(FORM), form_body, &expected);
            assert!(
                matches!(refused, Err(Refusal::Form | Refusal::Json | Refusal::Token)),
                "{form_body:?}: {refused:?}"
            );
        }
        assert_eq!(read(Some(FORM), b"other=1", &expected), Err(Refusal::Form));
        assert_eq!(
            read(Some(FORM), b"data=1&data=2", &expected),
            Err(Refusal::Form)
        );
        assert_eq!(read(Some(FORM), b"", &expected), Err(Refusal::Form));
        assert_eq!(read(Some(FORM), b"data=", &expected), Err(Refusal::Json));
        assert_eq!(
            read(Some(FORM), &body("{not json"), &expected),
            Err(Refusal::Json)
        );
        assert_eq!(read(Some(FORM), &body("[]"), &expected), Err(Refusal::Json));
        // longer than the bound, however well formed
        let long = json(&[("message", &format!("\"{}\"", "x".repeat(LIMIT)))]);
        assert_eq!(
            read(Some(FORM), &body(&long), &expected),
            Err(Refusal::Size)
        );
        // the listener reads four times as much, so that such a body is counted
        assert!(body(&long).len() < READ);
    }

    #[test]
    fn a_field_of_another_shape_is_refused_after_the_token() {
        for (field, value, reason) in [
            ("type", "\"Tip\"", Refusal::Shape),
            ("type", "1", Refusal::Shape),
            ("type", "<absent>", Refusal::Shape),
            ("type", "\"Subscription\"", Refusal::Shape),
            ("is_first_subscription_payment", "true", Refusal::Shape),
            ("is_subscription_payment", "\"false\"", Refusal::Shape),
            ("kofi_transaction_id", "\"\"", Refusal::Shape),
            ("kofi_transaction_id", "\"a b\"", Refusal::Shape),
            ("kofi_transaction_id", "\"../x\"", Refusal::Shape),
            ("kofi_transaction_id", "17", Refusal::Shape),
            ("amount", "3", Refusal::Shape),
            ("amount", "\"0.00\"", Refusal::Amount),
            ("amount", "\"-3.00\"", Refusal::Amount),
            ("amount", "\"3,00\"", Refusal::Amount),
            ("amount", "\"1e9\"", Refusal::Amount),
            ("amount", "\"3.001\"", Refusal::Amount),
            ("amount", "\"1000000000.00\"", Refusal::Amount),
            ("amount", "\"NaN\"", Refusal::Amount),
            ("amount", "\" 3.00\"", Refusal::Amount),
            ("currency", "\"eur\"", Refusal::Currency),
            ("currency", "\"€\"", Refusal::Currency),
            ("currency", "\"EURO\"", Refusal::Currency),
            ("currency", "\"\"", Refusal::Currency),
        ] {
            assert_eq!(
                delivered(&json(&[(field, value)])),
                Err(reason),
                "{field} {value}"
            );
        }
        // the largest amount the rule takes is taken, in any three-letter currency
        let Ok(Delivery::Payment(payment)) = delivered(&json(&[
            ("amount", "\"999999999.99\""),
            ("currency", "\"XYZ\""),
        ])) else {
            panic!("a payment");
        };
        assert_eq!(payment.amount, MOST);
    }

    #[test]
    fn what_a_giver_says_is_never_read() {
        // whatever the giver's fields hold, even of a type no field here has, the
        // payment is the same
        let plain = delivered(&json(&[]));
        let odd = json(&[
            ("from_name", "{\"nested\":[1,2,{\"deep\":true}]}"),
            ("message", "12345"),
            ("email", "null"),
            ("shipping", "{\"full_name\":\"x\",\"street_address\":\"y\"}"),
            ("shop_items", "[{\"direct_link_code\":\"z\"}]"),
        ]);
        assert_eq!(delivered(&odd), plain);
    }

    #[test]
    fn a_configured_token_has_a_token_s_shape() {
        assert!(plain_token(TOKEN));
        assert!(plain_token("3a1fac0c-f960-4506-a60e-824979a74e74"));
        for token in [
            "",
            "short",
            "with space inside",
            "tab\there!",
            &"x".repeat(129),
            "töken-öf-length",
        ] {
            assert!(!plain_token(token), "{token:?}");
        }
    }
}
