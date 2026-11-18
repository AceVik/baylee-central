//! The euro's reference rates, from the European Central Bank's daily feed: how many
//! units of each currency one euro was worth on the bank's last working day. Central
//! converts a donation in another currency with them and never guesses one it lacks.
//!
//! The feed is a small XML document (about 1.5 KiB, 30 currencies). It is read with
//! `roxmltree`, which is safe Rust, with document type declarations refused (so no
//! entity is ever expanded) and a bound on its nodes, from a body the caller has
//! already bounded ([`LIMIT`]). The ECB's statistical data service also answers in
//! JSON, but on 18.11.2026 a query for every currency took over a minute to start
//! answering when its cache was cold, while this feed answered in a tenth of a second.

use crate::Currency;
use std::collections::BTreeMap;

/// The ECB's euro reference rates of the last working day.
pub const ADDRESS: &str = "https://www.ecb.europa.eu/stats/eurofxref/eurofxref-daily.xml";

/// The most of an answer that is read: forty times the real one.
pub const LIMIT: u64 = 64 * 1024;

/// The most nodes the document may have; the real one has about 100.
const NODES: u32 = 1024;

/// The most currencies one feed may list; the real one lists 30.
const CURRENCIES: usize = 64;

/// The feed's own namespace, which every `Cube` element is in.
const VOCABULARY: &str = "http://www.ecb.int/vocabulary/2002-08-01/eurofxref";

/// The most one rate may be, in millionths: ten million units of a currency to the
/// euro, far beyond any the feed lists (the largest, the rupiah, is about 20 000).
const MOST_RATE: u64 = 10_000_000 * MILLION;

const MILLION: u64 = 1_000_000;

/// The rates of one day, each in millionths of a unit of its currency per euro.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rates {
    day: String,
    per_euro: BTreeMap<Currency, u64>,
}

/// A rate's decimal text (`1.0968`, `20020.71`, `137`) in millionths: one to eight
/// digits, then at most one point with one to six digits, and more than zero.
fn millionths(text: &str) -> Option<u64> {
    let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
    let digits = |part: &str| part.bytes().all(|b| b.is_ascii_digit());
    if whole.is_empty() || whole.len() > 8 || !digits(whole) || fraction.len() > 6 {
        return None;
    }
    if text.contains('.') && (fraction.is_empty() || !digits(fraction)) {
        return None;
    }
    let whole: u64 = whole.parse().ok()?;
    let mut part: u64 = if fraction.is_empty() {
        0
    } else {
        fraction.parse().ok()?
    };
    for _ in fraction.len()..6 {
        part *= 10;
    }
    let rate = whole.checked_mul(MILLION)?.checked_add(part)?;
    (rate > 0 && rate <= MOST_RATE).then_some(rate)
}

/// `hundredths` of a currency at `rate` millionths to the euro, in euro cents, rounded
/// half up. `None` for a rate of zero; the caller bounds the result.
#[must_use]
pub fn euro_cents(hundredths: u64, rate: u64) -> Option<u64> {
    if rate == 0 {
        return None;
    }
    let cents =
        (u128::from(hundredths) * u128::from(MILLION) + u128::from(rate / 2)) / u128::from(rate);
    u64::try_from(cents).ok()
}

impl Rates {
    /// The ECB's daily feed as the rates of its day. `None` for anything else: not
    /// XML, a document type, more nodes than a feed has, no day or more than one, a
    /// day of another shape, a currency or rate of another shape, a currency twice,
    /// the euro itself, or no rate at all.
    #[must_use]
    pub fn read_ecb(text: &str) -> Option<Self> {
        if text.len() as u64 > LIMIT {
            return None;
        }
        let options = roxmltree::ParsingOptions {
            allow_dtd: false,
            nodes_limit: NODES,
            // no external entity is ever fetched
            entity_resolver: None,
        };
        let document = roxmltree::Document::parse_with_options(text, options).ok()?;
        let cube = |node: &roxmltree::Node<'_, '_>| {
            node.is_element()
                && node.tag_name().name() == "Cube"
                && node.tag_name().namespace() == Some(VOCABULARY)
        };
        let mut days = document
            .descendants()
            .filter(|node| cube(node) && node.attribute("time").is_some());
        let day = days.next()?;
        if days.next().is_some() {
            return None;
        }
        let date = day.attribute("time")?;
        if !central_api::funding::is_day(date) {
            return None;
        }
        let mut per_euro = BTreeMap::new();
        for node in day.children().filter(roxmltree::Node::is_element) {
            if !cube(&node) || per_euro.len() == CURRENCIES {
                return None;
            }
            let currency = Currency::parse(node.attribute("currency")?)?;
            let rate = millionths(node.attribute("rate")?)?;
            if currency == Currency::EUR || per_euro.insert(currency, rate).is_some() {
                return None;
            }
        }
        (!per_euro.is_empty()).then(|| Self {
            day: date.to_owned(),
            per_euro,
        })
    }

    /// The rates of `day`, for a test or a store that held them.
    #[must_use]
    pub fn of(day: &str, rates: &[(Currency, u64)]) -> Self {
        Self {
            day: day.to_owned(),
            per_euro: rates.iter().copied().collect(),
        }
    }

    /// The day the rates are of, `YYYY-MM-DD`.
    #[must_use]
    pub fn day(&self) -> &str {
        &self.day
    }

    /// The rate of `currency` in millionths to the euro; `None` when the feed has
    /// none. The euro's own is one.
    #[must_use]
    pub fn rate(&self, currency: Currency) -> Option<u64> {
        if currency == Currency::EUR {
            return Some(MILLION);
        }
        self.per_euro.get(&currency).copied()
    }

    /// Every currency and its rate.
    pub fn each(&self) -> impl Iterator<Item = (Currency, u64)> + '_ {
        self.per_euro
            .iter()
            .map(|(currency, rate)| (*currency, *rate))
    }

    /// `hundredths` of `currency` in euro cents at these rates; `None` without a rate.
    #[must_use]
    pub fn euro_cents(&self, hundredths: u64, currency: Currency) -> Option<u64> {
        euro_cents(hundredths, self.rate(currency)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shape of the feed of 18.11.2026, shortened.
    const FEED: &str = r"<?xml version='1.0' encoding='UTF-8'?>
<gesmes:Envelope xmlns:gesmes='http://www.gesmes.org/xml/2002-08-01' xmlns='http://www.ecb.int/vocabulary/2002-08-01/eurofxref'>
	<gesmes:subject>Reference rates</gesmes:subject>
	<gesmes:Sender>
		<gesmes:name>European Central Bank</gesmes:name>
	</gesmes:Sender>
	<Cube>
		<Cube time='2026-11-18'>
			<Cube currency='USD' rate='1.0968'/>
			<Cube currency='JPY' rate='174.85'/>
			<Cube currency='GBP' rate='0.83290'/>
			<Cube currency='ISK' rate='137.00'/>
			<Cube currency='IDR' rate='20020.71'/>
		</Cube>
	</Cube>
</gesmes:Envelope>";

    fn usd() -> Currency {
        Currency::USD
    }

    #[test]
    fn the_feed_gives_each_rate_of_its_day() {
        let rates = Rates::read_ecb(FEED).expect("rates");
        assert_eq!(rates.day(), "2026-11-18");
        assert_eq!(rates.rate(usd()), Some(1_096_800));
        assert_eq!(rates.rate(Currency::parse("GBP").unwrap()), Some(832_900));
        assert_eq!(
            rates.rate(Currency::parse("IDR").unwrap()),
            Some(20_020_710_000)
        );
        assert_eq!(rates.rate(Currency::EUR), Some(1_000_000));
        assert_eq!(rates.rate(Currency::parse("CHF").unwrap()), None);
        assert_eq!(rates.each().count(), 5);
    }

    #[test]
    fn amounts_are_converted_by_integers_and_rounded_half_up() {
        let rates = Rates::read_ecb(FEED).unwrap();
        // five dollars at 1.0968 are 4.5587 euros
        assert_eq!(rates.euro_cents(500, usd()), Some(456));
        // a thousand yen at 174.85 are 5.7192 euros
        assert_eq!(
            rates.euro_cents(100_000, Currency::parse("JPY").unwrap()),
            Some(572)
        );
        assert_eq!(rates.euro_cents(1_234, Currency::EUR), Some(1_234));
        assert_eq!(rates.euro_cents(500, Currency::parse("CHF").unwrap()), None);
        // half a cent goes up, less than half goes down
        assert_eq!(euro_cents(1, 2_000_000), Some(1));
        assert_eq!(euro_cents(1, 2_000_001), Some(0));
        assert_eq!(euro_cents(0, 1_096_800), Some(0));
        assert_eq!(euro_cents(500, 0), None);
        // the largest amount at the smallest rate stays in range
        assert_eq!(euro_cents(99_999_999_999, 1), Some(99_999_999_999_000_000));
        assert_eq!(euro_cents(u64::MAX, 1), None);
    }

    #[test]
    fn a_rate_is_decimal_text_and_more_than_nothing() {
        for (text, rate) in [
            ("1.0968", Some(1_096_800)),
            ("137", Some(137_000_000)),
            ("0.000001", Some(1)),
            ("10000000", Some(MOST_RATE)),
            ("10000000.000001", None),
            ("0", None),
            ("0.000000", None),
            ("0.0000001", None),
            ("", None),
            ("1.", None),
            (".5", None),
            ("-1.0", None),
            ("1e3", None),
            ("1,0968", None),
            (" 1.0968", None),
            ("123456789", None),
            ("NaN", None),
        ] {
            assert_eq!(millionths(text), rate, "{text:?}");
        }
    }

    #[test]
    fn a_feed_of_another_shape_gives_no_rates() {
        let swapped = |from: &str, to: &str| FEED.replace(from, to);
        for text in [
            String::new(),
            "{}".to_owned(),
            "<Cube/>".to_owned(),
            // no day, a day of another shape, two days
            swapped("time='2026-11-18'", ""),
            swapped("2026-11-18", "18.11.2026"),
            swapped(
                "<Cube currency='USD' rate='1.0968'/>",
                "<Cube time='2026-11-17'><Cube currency='USD' rate='1.0'/></Cube>",
            ),
            // a currency or a rate of another shape, twice, the euro itself
            swapped("currency='USD'", "currency='usd'"),
            swapped("currency='USD'", "currency='US Dollar'"),
            swapped("rate='1.0968'", "rate='-1.0968'"),
            swapped("rate='1.0968'", "rate='1,0968'"),
            swapped("rate='1.0968'", "rate='0'"),
            swapped("currency='JPY'", "currency='USD'"),
            swapped("currency='JPY'", "currency='EUR'"),
            swapped(" rate='174.85'", ""),
            // an element that is not a rate among the rates
            swapped(
                "<Cube currency='USD' rate='1.0968'/>",
                "<Cube currency='USD' rate='1.0968'/><Other/>",
            ),
            // a Cube outside the feed's namespace
            swapped(
                "xmlns='http://www.ecb.int/vocabulary/2002-08-01/eurofxref'",
                "xmlns='http://example.org/'",
            ),
            // not XML
            swapped("</gesmes:Envelope>", ""),
        ] {
            assert_eq!(Rates::read_ecb(&text), None, "{text}");
        }
        // a day without rates is none either
        let empty = r"<e xmlns='http://www.ecb.int/vocabulary/2002-08-01/eurofxref'><Cube><Cube time='2026-11-18'></Cube></Cube></e>";
        assert_eq!(Rates::read_ecb(empty), None);
    }

    #[test]
    fn a_document_type_and_a_flood_of_nodes_are_refused() {
        // an entity declared to grow (the "billion laughs") is never expanded
        let laughs = format!(
            "<!DOCTYPE e [<!ENTITY a 'aaaaaaaaaa'><!ENTITY b '&a;&a;&a;&a;&a;&a;&a;&a;'>]>{}",
            FEED.replace("Reference rates", "&b;")
                .trim_start_matches("<?xml version='1.0' encoding='UTF-8'?>\n")
        );
        assert_eq!(Rates::read_ecb(&laughs), None);
        // more nodes than any feed has
        let many = FEED.replace(
            "<gesmes:subject>",
            &format!("{}<gesmes:subject>", "<x/>".repeat(2_000)),
        );
        assert!(many.len() < usize::try_from(LIMIT).unwrap());
        assert_eq!(Rates::read_ecb(&many), None);
        // more currencies than any feed lists, each a new one
        let letters = |n: usize| {
            let b = |i: usize| char::from(b'A' + u8::try_from(i % 26).unwrap());
            format!("{}{}{}", b(n / 676), b(n / 26), b(n))
        };
        let listed = (0..65)
            .map(|n| format!("<Cube currency='{}' rate='1.5'/>", letters(n + 30)))
            .collect::<Vec<_>>()
            .concat();
        let crowded = FEED.replace("<Cube currency='USD' rate='1.0968'/>", &listed);
        assert_eq!(Rates::read_ecb(&crowded), None);
        // and a document longer than the bound is not even parsed
        let long = format!("{FEED}{}", " ".repeat(usize::try_from(LIMIT).unwrap()));
        assert_eq!(Rates::read_ecb(&long), None);
    }
}
