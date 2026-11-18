//! GitHub Sponsors, asked with the sponsored account's own token through GitHub's
//! GraphQL API: the platform's estimate of the monthly income (US cents), how many
//! sponsorships are active, and the one-time sponsorships and refunds of the last days.
//!
//! The query names no sponsor: not `sponsor`, `sponsorEntity`, a login or a name. Of an
//! activity it reads the activity's own id (to count it once), what happened and its
//! tier's price and whether that tier is one-time. The field names were read from
//! GitHub's published schema (`schema.docs.graphql`, 18.11.2026); no answer with real
//! sponsors has been read yet, so an answer of another shape counts as no answer.

use serde::Deserialize;

/// GitHub's GraphQL endpoint.
pub const ADDRESS: &str = "https://api.github.com/graphql";

/// The most of an answer that is read: a full page of activities is about 15 KiB.
pub const LIMIT: u64 = 256 * 1024;

/// Activities asked for in one page, GitHub's most.
pub const PAGE: u8 = 100;

/// The most pages read in one poll: a thousand one-time sponsorships and refunds in
/// the window (`docs/operations.md` §"Donations").
pub const PAGES: usize = 10;

/// The most a monthly estimate is believed, in US cents: ten million dollars.
const MOST_MONTH: u64 = 1_000_000_000;

/// The most one sponsorship is believed, in US cents: a hundred thousand dollars.
const MOST_ONE: u64 = 10_000_000;

/// The query. `$since` bounds the activities to the window Central has not swept yet;
/// `$after` is the page cursor. Private sponsorships are counted too (both connections
/// say so explicitly), and nothing about who they are from is asked.
pub const QUERY: &str = "query($since: DateTime!, $after: String) { viewer { \
monthlyEstimatedSponsorsIncomeInCents \
sponsorshipsAsMaintainer(activeOnly: true, includePrivate: true) { totalCount } \
sponsorsActivities(first: 100, after: $after, since: $since, actions: [NEW_SPONSORSHIP, REFUND], includePrivate: true) { \
pageInfo { hasNextPage endCursor } \
nodes { id action sponsorsTier { isOneTime monthlyPriceInCents } } } } }";

/// The request's body for the window from `since` (an RFC 3339 time) and the page
/// after `after`.
#[must_use]
pub fn request(since: &str, after: Option<&str>) -> String {
    serde_json::json!({
        "query": QUERY,
        "variables": { "since": since, "after": after },
    })
    .to_string()
}

/// What happened to one sponsorship, as far as the sum of what was given once cares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A one-time sponsorship was made.
    Once,
    /// A one-time sponsorship's money went back.
    Refund,
}

/// One one-time sponsorship or its refund.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Activity {
    /// How Central remembers it ([`crate::reference`]).
    pub reference: [u8; 32],
    /// What happened.
    pub kind: Kind,
    /// The tier's price, in US cents.
    pub cents: u64,
}

/// What one answer says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Answer {
    /// GitHub's estimate of the monthly income, in US cents.
    pub month: u64,
    /// How many sponsorships are active.
    pub sponsors: u64,
    /// The one-time sponsorships and refunds on this page.
    pub activities: Vec<Activity>,
    /// The cursor of the next page, if there is one.
    pub next: Option<String>,
}

#[derive(Deserialize)]
struct Envelope {
    data: Option<Data>,
    #[serde(default)]
    errors: Option<serde::de::IgnoredAny>,
}

#[derive(Deserialize)]
struct Data {
    viewer: Viewer,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Viewer {
    monthly_estimated_sponsors_income_in_cents: i64,
    sponsorships_as_maintainer: Total,
    sponsors_activities: Activities,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Total {
    total_count: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Activities {
    page_info: PageInfo,
    nodes: Vec<Option<Node>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageInfo {
    has_next_page: bool,
    end_cursor: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Node {
    id: String,
    action: String,
    sponsors_tier: Option<Tier>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Tier {
    is_one_time: bool,
    monthly_price_in_cents: i64,
}

/// A page cursor as GitHub writes one: 1 to 200 characters of base64.
fn plain_cursor(cursor: &str) -> bool {
    (1..=200).contains(&cursor.len())
        && cursor
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'=' | b'-' | b'_'))
}

/// GitHub's answer as the figures kept of it. `None` for an answer with errors (a
/// token without the scope these fields need answers so, with status 200), without
/// data, of another shape, with a figure beyond belief, more nodes than a page has or
/// a node whose id or cursor is of another shape. An activity that is not about a
/// one-time tier is passed over: what is given monthly is in the estimate.
#[must_use]
pub fn read(text: &str) -> Option<Answer> {
    let envelope: Envelope = serde_json::from_str(text).ok()?;
    if envelope.errors.is_some() {
        return None;
    }
    let viewer = envelope.data?.viewer;
    let month = u64::try_from(viewer.monthly_estimated_sponsors_income_in_cents)
        .ok()
        .filter(|month| *month <= MOST_MONTH)?;
    let sponsors = u64::try_from(viewer.sponsorships_as_maintainer.total_count)
        .ok()
        .filter(|count| *count <= central_api::funding::MOST_COUNT)?;
    let page = viewer.sponsors_activities;
    if page.nodes.len() > usize::from(PAGE) {
        return None;
    }
    let mut activities = Vec::new();
    for node in page.nodes.into_iter().flatten() {
        if !crate::plain_id(&node.id, 100) {
            return None;
        }
        let kind = match node.action.as_str() {
            "NEW_SPONSORSHIP" => Kind::Once,
            "REFUND" => Kind::Refund,
            _ => continue,
        };
        let Some(tier) = node.sponsors_tier.filter(|tier| tier.is_one_time) else {
            continue;
        };
        let cents = u64::try_from(tier.monthly_price_in_cents)
            .ok()
            .filter(|cents| (1..=MOST_ONE).contains(cents))?;
        activities.push(Activity {
            reference: crate::reference("github", &node.id),
            kind,
            cents,
        });
    }
    let next = if page.page_info.has_next_page {
        Some(
            page.page_info
                .end_cursor
                .filter(|cursor| plain_cursor(cursor))?,
        )
    } else {
        None
    };
    Some(Answer {
        month,
        sponsors,
        activities,
        next,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An answer of the schema's shape, with made-up values.
    const ANSWER: &str = r#"{"data":{"viewer":{
        "monthlyEstimatedSponsorsIncomeInCents":4600,
        "sponsorshipsAsMaintainer":{"totalCount":2},
        "sponsorsActivities":{
            "pageInfo":{"hasNextPage":false,"endCursor":"Y3Vyc29yOjI="},
            "nodes":[
                {"id":"SA_synthetic1","action":"NEW_SPONSORSHIP","sponsorsTier":{"isOneTime":true,"monthlyPriceInCents":2500}},
                {"id":"SA_synthetic2","action":"NEW_SPONSORSHIP","sponsorsTier":{"isOneTime":false,"monthlyPriceInCents":500}},
                {"id":"SA_synthetic3","action":"REFUND","sponsorsTier":{"isOneTime":true,"monthlyPriceInCents":1000}},
                {"id":"SA_synthetic4","action":"CANCELLED_SPONSORSHIP","sponsorsTier":{"isOneTime":true,"monthlyPriceInCents":1000}},
                {"id":"SA_synthetic5","action":"NEW_SPONSORSHIP","sponsorsTier":null},
                null
            ]}}}}"#;

    #[test]
    fn the_query_names_no_sponsor() {
        for field in [
            "sponsor ",
            "sponsor{",
            "sponsorEntity",
            "login",
            "name",
            "email",
        ] {
            assert!(!QUERY.contains(field), "{field}");
        }
        let body: serde_json::Value =
            serde_json::from_str(&request("2026-11-18T00:00:00Z", None)).unwrap();
        assert_eq!(body["variables"]["since"], "2026-11-18T00:00:00Z");
        assert!(body["variables"]["after"].is_null());
        let next: serde_json::Value =
            serde_json::from_str(&request("2026-11-18T00:00:00Z", Some("Y3Vyc29yOjI="))).unwrap();
        assert_eq!(next["variables"]["after"], "Y3Vyc29yOjI=");
    }

    #[test]
    fn of_an_answer_the_estimate_the_count_and_one_time_tiers_are_kept() {
        let answer = read(ANSWER).expect("an answer");
        assert_eq!((answer.month, answer.sponsors), (4600, 2));
        assert_eq!(
            answer.activities,
            vec![
                Activity {
                    reference: crate::reference("github", "SA_synthetic1"),
                    kind: Kind::Once,
                    cents: 2500
                },
                Activity {
                    reference: crate::reference("github", "SA_synthetic3"),
                    kind: Kind::Refund,
                    cents: 1000
                },
            ]
        );
        assert_eq!(answer.next, None);
        let more = ANSWER.replace(r#""hasNextPage":false"#, r#""hasNextPage":true"#);
        assert_eq!(read(&more).unwrap().next.as_deref(), Some("Y3Vyc29yOjI="));
    }

    #[test]
    fn an_answer_of_another_shape_or_beyond_belief_is_none() {
        let swapped = |from: &str, to: &str| ANSWER.replace(from, to);
        for text in [
            String::new(),
            "{}".to_owned(),
            r#"{"data":null}"#.to_owned(),
            // a missing scope answers 200 with errors and no data
            r#"{"data":null,"errors":[{"type":"INSUFFICIENT_SCOPES","message":"x"}]}"#.to_owned(),
            // errors beside data are errors too
            ANSWER.replacen("{\"data\"", "{\"errors\":[{\"message\":\"x\"}],\"data\"", 1),
            swapped("4600", "-1"),
            swapped("4600", "1000000001"),
            swapped("4600", "\"4600\""),
            swapped(r#""totalCount":2"#, r#""totalCount":-2"#),
            swapped(r#""totalCount":2"#, r#""totalCount":1000001"#),
            swapped("2500", "0"),
            swapped("2500", "-2500"),
            swapped("2500", "10000001"),
            swapped("SA_synthetic1", "SA synthetic"),
            swapped("SA_synthetic1", "<b>"),
            swapped(
                r#""hasNextPage":false,"endCursor":"Y3Vyc29yOjI=""#,
                r#""hasNextPage":true,"endCursor":null"#,
            ),
            swapped(
                r#""hasNextPage":false,"endCursor":"Y3Vyc29yOjI=""#,
                r#""hasNextPage":true,"endCursor":"a b""#,
            ),
            swapped("\"sponsorshipsAsMaintainer\":{\"totalCount\":2},", ""),
        ] {
            assert_eq!(read(&text), None, "{text}");
        }
        // more nodes than a page has
        let node = r#"{"id":"SA_x","action":"NEW_SPONSORSHIP","sponsorsTier":{"isOneTime":true,"monthlyPriceInCents":100}}"#;
        let crowded = format!(
            r#"{{"data":{{"viewer":{{"monthlyEstimatedSponsorsIncomeInCents":0,"sponsorshipsAsMaintainer":{{"totalCount":0}},"sponsorsActivities":{{"pageInfo":{{"hasNextPage":false,"endCursor":null}},"nodes":[{}]}}}}}}}}"#,
            vec![node; 101].join(",")
        );
        assert_eq!(read(&crowded), None);
        let full = crowded.replacen(&format!("{node},"), "", 1);
        assert_eq!(read(&full).map(|answer| answer.activities.len()), Some(100));
    }

    #[test]
    fn a_name_in_an_answer_is_never_read() {
        // were GitHub to send a sponsor's name with the activity, it would be passed over
        let named = ANSWER.replace(
            r#""action":"NEW_SPONSORSHIP","sponsorsTier":{"isOneTime":true,"monthlyPriceInCents":2500}"#,
            r#""action":"NEW_SPONSORSHIP","sponsor":{"login":"somebody","name":"Some Body"},"sponsorsTier":{"isOneTime":true,"monthlyPriceInCents":2500}"#,
        );
        assert_eq!(read(&named), read(ANSWER));
    }
}
