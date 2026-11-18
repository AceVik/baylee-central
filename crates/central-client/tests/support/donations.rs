//! Ko-fi's webhook through the real account listener and PostgreSQL, and the
//! donation figures it feeds: a delivery taken once however often it comes, every
//! refusal one plain answer, the small body bound, a payment without a rate, a sale,
//! an address's budget, and the public figures read on both listeners. The token
//! and every delivery are synthetic (`docs/operations.md` §"Donations").
use super::{Http, browser::Browser};
use central_api::{
    Error,
    funding::{Funding, Monthly, Once},
    wire::{Request, Response},
};
use central_client::Transport;
use central_identity::{KOFI_LIMIT, KofiCounts};
use tokio::runtime::Runtime;

/// The fixture's synthetic verification token (`postgres.rs`, `serving`).
pub(crate) const TOKEN: &str = "synthetic-kofi-verification-token";

const FORM: (&str, &str) = ("Content-Type", "application/x-www-form-urlencoded");

/// A delivery's JSON as the descriptions of Ko-fi's format have it, with the
/// fields given replaced (`None` leaves a field out) and a giver who says things.
fn delivery(id: &str, changes: &[(&str, Option<&str>)]) -> String {
    let mut fields = vec![
        ("verification_token", format!("\"{TOKEN}\"")),
        (
            "message_id",
            "\"6a0d7a4e-7d0b-4a39-a2f4-6a1d8f2c0b11\"".to_owned(),
        ),
        ("timestamp", "\"2026-11-18T12:00:00Z\"".to_owned()),
        ("type", "\"Donation\"".to_owned()),
        ("is_public", "true".to_owned()),
        ("from_name", "\"Synthetic Giver\"".to_owned()),
        ("message", "\"Good luck!\"".to_owned()),
        ("amount", "\"3.00\"".to_owned()),
        (
            "url",
            "\"https://ko-fi.com/Home/CoffeeShop?txid=synthetic\"".to_owned(),
        ),
        ("email", "\"giver@example.org\"".to_owned()),
        ("currency", "\"EUR\"".to_owned()),
        ("is_subscription_payment", "false".to_owned()),
        ("is_first_subscription_payment", "false".to_owned()),
        ("kofi_transaction_id", format!("\"{id}\"")),
        ("shop_items", "null".to_owned()),
        ("tier_name", "null".to_owned()),
        ("shipping", "null".to_owned()),
    ];
    for (name, value) in changes {
        fields.retain(|(field, _)| field != name);
        if let Some(value) = value {
            fields.push((name, (*value).to_owned()));
        }
    }
    let inner: Vec<String> = fields
        .into_iter()
        .map(|(name, value)| format!("\"{name}\":{value}"))
        .collect();
    format!("{{{}}}", inner.join(","))
}

/// `data=` and the JSON, every byte but letters and digits percent-encoded.
fn form(data: &str) -> Vec<u8> {
    let mut body = b"data=".to_vec();
    for byte in data.bytes() {
        if byte.is_ascii_alphanumeric() {
            body.push(byte);
        } else {
            body.extend_from_slice(format!("%{byte:02X}").as_bytes());
        }
    }
    body
}

/// One delivery from the proxy's `hop`: its status and body.
fn deliver(http: &Http, hop: &str, headers: &[(&str, &str)], body: &[u8]) -> (u16, String) {
    let mut all = vec![("X-Forwarded-For", hop)];
    all.extend_from_slice(headers);
    let (status, _, body) = http.raw("POST /v1/kofi", &all, body);
    (status, String::from_utf8(body).unwrap())
}

fn figures(http: &Http) -> Funding {
    let Ok(Response::PublicFunding { funding }) = http.exchange(&Request::PublicFunding {}, None)
    else {
        panic!("the donation figures");
    };
    funding
}

/// What a request is answered when it is taken, and when it is refused.
fn taken() -> (u16, String) {
    (200, "ok\n".to_owned())
}

fn refused() -> (u16, String) {
    (400, "account request rejected\n".to_owned())
}

/// A delivery's headers and body.
type Sent = (Vec<(&'static str, &'static str)>, Vec<u8>);

pub(crate) fn run(http: &Http, browser: &Browser, runtime: &Runtime) {
    let refresh = || runtime.block_on(http.identity.refresh_donations()).unwrap();
    // Before the first read of the store: Ko-fi's figure and the sum are absent,
    // never zero, and the platforms not configured here are absent too.
    assert_eq!(figures(http), Funding::default());
    refresh();
    let empty = figures(http);
    assert_eq!(
        empty.kofi,
        Some(Monthly {
            month: 0,
            supporters: 0
        })
    );
    let since = empty.once.as_ref().unwrap().since.clone();
    assert_eq!((empty.github, empty.liberapay), (None, None));
    let first = form(&delivery("synthetic-kofi-0001", &[]));
    taking(http, &first);
    refresh();
    assert_eq!(
        figures(http),
        Funding {
            github: None,
            liberapay: None,
            kofi: Some(Monthly {
                month: 500,
                supporters: 1
            }),
            once: Some(Once {
                total: 300,
                count: 1,
                since,
            }),
        }
    );
    let counts = refusing(http);
    refresh();
    assert_eq!(figures(http).once.unwrap().total, 300);
    bounding(http, &first, counts);
    flooding(http, &first, counts);
    reading(http, browser);
}

/// A donation, the same delivery again and again, a regular payment, a sale and a
/// payment in a currency without a rate.
fn taking(http: &Http, first: &[u8]) {
    for _ in 0..3 {
        assert_eq!(deliver(http, "198.51.100.80", &[FORM], first), taken());
    }
    // again with another amount and kind: the first stands
    let changed = form(&delivery(
        "synthetic-kofi-0001",
        &[
            ("amount", Some("\"999.00\"")),
            ("type", Some("\"Subscription\"")),
            ("is_subscription_payment", Some("true")),
        ],
    ));
    assert_eq!(deliver(http, "198.51.100.80", &[FORM], &changed), taken());
    let monthly = form(&delivery(
        "synthetic-kofi-0002",
        &[
            ("type", Some("\"Subscription\"")),
            ("is_subscription_payment", Some("true")),
            ("is_first_subscription_payment", Some("true")),
            ("amount", Some("\"5.00\"")),
        ],
    ));
    assert_eq!(deliver(http, "198.51.100.80", &[FORM], &monthly), taken());
    // a sale buys something: taken, not counted
    let sale = form(&delivery(
        "synthetic-kofi-0003",
        &[
            ("type", Some("\"Shop Order\"")),
            ("amount", Some("\"40.00\"")),
        ],
    ));
    assert_eq!(deliver(http, "198.51.100.80", &[FORM], &sale), taken());
    // a currency without a rate (none is held here): kept, not counted, not guessed
    let dollars = form(&delivery(
        "synthetic-kofi-0004",
        &[("currency", Some("\"USD\"")), ("amount", Some("\"7.00\""))],
    ));
    assert_eq!(deliver(http, "198.51.100.80", &[FORM], &dollars), taken());
}

/// One delivery of `id` with `changes`, as a form.
fn changed(id: &str, changes: &[(&str, Option<&str>)]) -> Sent {
    (vec![FORM], form(&delivery(id, changes)))
}

/// Every refusal is one plain answer, whatever its reason, and changes nothing.
fn refusing(http: &Http) -> KofiCounts {
    let token = "verification_token";
    let refusals: Vec<Sent> = vec![
        // the token wrong, missing, empty, not text
        changed(
            "synthetic-kofi-0010",
            &[(token, Some("\"synthetic-kofi-verification-tokeN\""))],
        ),
        changed("synthetic-kofi-0011", &[(token, None)]),
        changed("synthetic-kofi-0012", &[(token, Some("\"\""))]),
        changed("synthetic-kofi-0013", &[(token, Some("1"))]),
        // not a form, no data, other fields, not JSON
        (
            vec![("Content-Type", "application/json")],
            delivery("synthetic-kofi-0014", &[]).into_bytes(),
        ),
        (vec![], form(&delivery("synthetic-kofi-0015", &[]))),
        (vec![FORM], b"other=1".to_vec()),
        (
            vec![FORM],
            [
                form(&delivery("synthetic-kofi-0016", &[])),
                b"&data=x".to_vec(),
            ]
            .concat(),
        ),
        (vec![FORM], form("{\"verification_token\":")),
        (vec![FORM], form("[]")),
        // hostile amounts and currencies, with the right token
        changed("synthetic-kofi-0020", &[("amount", Some("\"-3.00\""))]),
        changed("synthetic-kofi-0021", &[("amount", Some("\"1e9\""))]),
        changed("synthetic-kofi-0022", &[("amount", Some("\"0.00\""))]),
        changed("synthetic-kofi-0023", &[("amount", Some("\"3,00\""))]),
        changed("synthetic-kofi-0024", &[("amount", Some("3"))]),
        changed(
            "synthetic-kofi-0025",
            &[("amount", Some("\"9999999999.00\""))],
        ),
        // beyond belief in euros: a hundred thousand and a cent
        changed("synthetic-kofi-0026", &[("amount", Some("\"100000.01\""))]),
        changed("synthetic-kofi-0027", &[("currency", Some("\"eur\""))]),
        changed("synthetic-kofi-0028", &[("currency", Some("\"<b>\""))]),
        // a shape not read here
        changed("synthetic-kofi-0030", &[("type", Some("\"Tip\""))]),
        changed(
            "synthetic-kofi-0031",
            &[("kofi_transaction_id", Some("\"../../x\""))],
        ),
        changed("synthetic-kofi-0032", &[("is_subscription_payment", None)]),
    ];
    for (n, (headers, body)) in refusals.iter().enumerate() {
        assert_eq!(
            deliver(http, &format!("198.51.100.{}", 100 + n), headers, body),
            refused(),
            "refusal {n}"
        );
    }
    let counts = http.identity.kofi_counts();
    assert_eq!(
        counts,
        KofiCounts {
            // content type, form, json, token, shape, amount, currency
            refused: [2, 2, 3, 3, 4, 6, 2],
            recorded: 3,
            unconverted: 1,
            again: 3,
            unrecorded: 0,
            sales: 1,
        }
    );
    counts
}

/// Larger than the route's bound: refused before it is read. A browser's request is
/// no delivery, and another method is not the route's. None of them is counted.
fn bounding(http: &Http, first: &[u8], counts: KofiCounts) {
    let long = form(&delivery(
        "synthetic-kofi-0040",
        &[("message", Some(&format!("\"{}\"", "x".repeat(KOFI_LIMIT))))],
    ));
    assert!(long.len() > KOFI_LIMIT);
    assert_eq!(deliver(http, "198.51.100.140", &[FORM], &long).0, 413);
    assert_eq!(
        deliver(
            http,
            "198.51.100.141",
            &[FORM, ("Origin", "https://ko-fi.com")],
            first
        )
        .0,
        403
    );
    let (status, _, _) = http.raw("GET /v1/kofi", &[], b"");
    assert_ne!(status, 200);
    assert_eq!(http.identity.kofi_counts().refused, counts.refused);
}

/// A flood from one address: thirty a minute, then the budget's answer before the
/// token is read; another address is served meanwhile.
fn flooding(http: &Http, first: &[u8], counts: KofiCounts) {
    let wrong = form(&delivery(
        "synthetic-kofi-0050",
        &[("verification_token", Some("\"guess-guess-guess-guess\""))],
    ));
    let mut answers = Vec::new();
    for _ in 0..31 {
        answers.push(deliver(http, "198.51.100.150", &[FORM], &wrong).0);
    }
    assert_eq!(answers.iter().filter(|status| **status == 400).count(), 30);
    assert_eq!(answers.last(), Some(&429));
    assert_eq!(
        deliver(http, "198.51.100.150", &[FORM], first).0,
        429,
        "the right token buys no room either"
    );
    assert_eq!(deliver(http, "198.51.100.151", &[FORM], first), taken());
    assert_eq!(
        http.identity.kofi_counts().refused[3],
        counts.refused[3] + 30,
        "only the thirty that were read count as wrong tokens"
    );
}

/// The figures are the same on the browser listener, without a session, and name
/// no single donation; the operation takes nothing.
fn reading(http: &Http, browser: &Browser) {
    let Ok(Response::PublicFunding { funding }) = browser.answer(&Request::PublicFunding {}, None)
    else {
        panic!("the browser's donation figures");
    };
    assert_eq!(funding, figures(http));
    let wire =
        String::from_utf8(central_api::encode(&Response::PublicFunding { funding }).unwrap())
            .unwrap();
    for absent in [
        "synthetic",
        "Giver",
        "giver@",
        "transaction",
        "message",
        "2026-11-18T",
    ] {
        assert!(!wire.contains(absent), "{absent}");
    }
    assert!(matches!(
        http.written(r#"{"operation":"public_funding","after":null}"#, None),
        Err(Error::Rejected)
    ));
}
