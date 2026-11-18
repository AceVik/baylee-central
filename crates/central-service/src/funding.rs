//! Donations (`docs/operations.md` §"Donations"): the optional `funding` block
//! of a service's configuration, and the one task that asks the platforms. Live
//! only: a development service collects nothing and asks nobody.
//!
//! Without the block Central reads Liberapay's public figures and nothing else;
//! GitHub Sponsors needs the sponsored account's token, Ko-fi's webhook its
//! verification token. Both arrive with the other secrets over standard input,
//! are checked for their shape, and are never logged or echoed: GitHub's lives
//! in this task to be sent to GitHub alone, Ko-fi's is kept as a digest only.
//!
//! Every request goes through `central-outbound`: HTTPS to public addresses
//! only, no proxy, no redirect, short time limits, and a bounded body. A poll
//! that fails leaves the last figure standing; a source that never answered
//! stays absent.
use central_api::funding::Monthly;
use central_funding::{Rates, github, liberapay, rates};
use central_identity::{FundingSources, IdentityService, KofiCounts, KofiIntake, kofi_digest};
use central_outbound::{Agent, Limits, agent};
use serde::Deserialize;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::watch;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

/// How often Liberapay and GitHub are asked: their figures change with a
/// payday or when somebody starts or stops giving.
const PLATFORMS: Duration = Duration::from_mins(30);
/// How often the reference rates are fetched: the ECB publishes them once a
/// working day, around 16:00 in Frankfurt.
const RATES: Duration = Duration::from_hours(6);
/// How often the stored sums are read again for the public figures, and the
/// log's counts looked at.
const SUMS: Duration = Duration::from_mins(1);
/// The least time between two lines of the same kind in the log.
const QUIET: Duration = Duration::from_hours(1);

fn yes() -> bool {
    true
}

/// The `funding` block. Absent, Liberapay is read and nothing else.
#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
pub(super) struct Funding {
    /// Read Liberapay's public figures (no key); `false` switches it off.
    #[serde(default = "yes")]
    liberapay: bool,
    /// GitHub Sponsors, asked with the sponsored account's token.
    #[serde(default)]
    github: Option<GitHub>,
    /// Ko-fi's webhook.
    #[serde(default)]
    kofi: Option<Kofi>,
}

/// GitHub Sponsors' block.
#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
struct GitHub {
    /// A token of the sponsored account that may read its sponsorships.
    token: String,
    /// `false` keeps the token in the file and asks GitHub nothing.
    #[serde(default = "yes")]
    on: bool,
}

/// Ko-fi's block.
#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
struct Kofi {
    /// The verification token Ko-fi shows under its webhook's advanced settings.
    token: String,
    /// Store and count verified payments. Absent or `false`, each delivery is
    /// verified and read and nothing is kept: the state for Ko-fi's test.
    #[serde(default)]
    record: bool,
    /// `false` keeps the token in the file and answers the webhook "not found".
    #[serde(default = "yes")]
    on: bool,
}

/// What the task asks, and the one secret it sends.
pub(super) struct Plan {
    liberapay: bool,
    github: Option<Zeroizing<String>>,
    /// Rates are needed when anything may come in another currency.
    rates: bool,
    /// The store's sums are read when anything is stored.
    sums: bool,
}

/// A GitHub token's shape: 20 to 255 visible ASCII characters (a classic token
/// has 40, a fine-grained one about 93).
fn plain_github_token(token: &str) -> bool {
    (20..=255).contains(&token.len()) && token.bytes().all(|b| b.is_ascii_graphic())
}

impl Funding {
    /// What the configuration turns on, its tokens moved out, not copied. A token
    /// of another shape refuses the whole configuration, so that a service
    /// configured wrongly does not start; the refusal names no value.
    pub(super) fn plan(&mut self) -> Result<(FundingSources, Plan), ()> {
        let github = match self.github.as_mut() {
            Some(block) if !plain_github_token(&block.token) => return Err(()),
            Some(block) if block.on => Some(Zeroizing::new(std::mem::take(&mut block.token))),
            _ => None,
        };
        let kofi = match self.kofi.as_ref() {
            Some(block) if !central_funding::kofi::plain_token(&block.token) => return Err(()),
            Some(block) if block.on => Some(KofiIntake {
                token: kofi_digest(&block.token),
                record: block.record,
            }),
            _ => None,
        };
        let sources = FundingSources {
            liberapay: self.liberapay,
            github: github.is_some(),
            kofi,
        };
        let stored = github.is_some() || kofi.is_some_and(|kofi| kofi.record);
        Ok((
            sources,
            Plan {
                liberapay: self.liberapay,
                rates: github.is_some() || kofi.is_some(),
                sums: stored,
                github,
            },
        ))
    }

    /// The plan without a block: Liberapay alone.
    pub(super) fn absent() -> (FundingSources, Plan) {
        (
            FundingSources {
                liberapay: true,
                github: false,
                kofi: None,
            },
            Plan {
                liberapay: true,
                github: None,
                rates: false,
                sums: false,
            },
        )
    }
}

/// What the start's log line says is collected: the sources, never a value.
pub(super) fn announced(sources: &FundingSources) -> String {
    let mut named = Vec::new();
    if sources.liberapay {
        named.push("Liberapay");
    }
    if sources.github {
        named.push("GitHub Sponsors");
    }
    match sources.kofi {
        Some(kofi) if kofi.record => named.push("Ko-fi"),
        Some(_) => named.push("Ko-fi (verifying only, kofi.record is false)"),
        None => {}
    }
    if named.is_empty() {
        "central: donations: nothing is collected".to_owned()
    } else {
        format!("central: donations: {}", named.join(", "))
    }
}

/// The one way the task reads a platform: a GET of a public document, or
/// GitHub's GraphQL with the token. The answer's body as text, at most `limit`
/// bytes, and only for `200`. A test puts its own in its place.
pub(super) trait Fetch: Send + Sync + 'static {
    /// `GET url`.
    fn get(&self, url: &str, accept: &str, limit: u64) -> Option<String>;
    /// `POST url` with a JSON body and the token as a bearer.
    fn graphql(&self, url: &str, token: &str, body: &str, limit: u64) -> Option<String>;
}

/// The deployed fetch: `central-outbound`'s agent, port 443 only.
struct Outbound(Agent);

impl Outbound {
    fn new() -> Self {
        Self(agent(Limits {
            port: Some(443),
            total: Duration::from_secs(15),
            connect: Duration::from_secs(5),
            pin: None,
        }))
    }
}

impl Fetch for Outbound {
    fn get(&self, url: &str, accept: &str, limit: u64) -> Option<String> {
        let mut response = self
            .0
            .get(url)
            .header("Accept", accept)
            .header("User-Agent", "baylee-central")
            .call()
            .ok()?;
        if response.status() != 200 {
            return None;
        }
        response
            .body_mut()
            .with_config()
            .limit(limit)
            .read_to_string()
            .ok()
    }

    fn graphql(&self, url: &str, token: &str, body: &str, limit: u64) -> Option<String> {
        let bearer = Zeroizing::new(format!("bearer {token}"));
        let mut response = self
            .0
            .post(url)
            .header("Authorization", bearer.as_str())
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("User-Agent", "baylee-central")
            .send(body)
            .ok()?;
        if response.status() != 200 {
            return None;
        }
        response
            .body_mut()
            .with_config()
            .limit(limit)
            .read_to_string()
            .ok()
    }
}

/// Liberapay's figure, or nothing.
fn liberapay_figure(fetch: &dyn Fetch) -> Option<Monthly> {
    liberapay::read(&fetch.get(liberapay::ADDRESS, "application/json", liberapay::LIMIT)?)
}

/// The ECB's rates, or nothing.
fn reference_rates(fetch: &dyn Fetch) -> Option<Rates> {
    Rates::read_ecb(&fetch.get(rates::ADDRESS, "application/xml, text/xml", rates::LIMIT)?)
}

/// GitHub's estimate and count, and every one-time sponsorship and refund since
/// `since`, read page by page up to [`github::PAGES`]; nothing when any page fails
/// or there would be more.
fn github_figures(fetch: &dyn Fetch, token: &str, since: &str) -> Option<(github::Answer, usize)> {
    let mut after: Option<String> = None;
    let mut whole: Option<github::Answer> = None;
    for pages in 1..=github::PAGES {
        let body = github::request(since, after.as_deref());
        let answer = github::read(&fetch.graphql(github::ADDRESS, token, &body, github::LIMIT)?)?;
        let next = answer.next.clone();
        match whole.as_mut() {
            None => whole = Some(answer),
            Some(whole) => whole.activities.extend(answer.activities),
        }
        match next {
            None => return whole.map(|whole| (whole, pages)),
            Some(cursor) => after = Some(cursor),
        }
    }
    None
}

/// Says one kind of line at most once in [`QUIET`].
#[derive(Default)]
struct Quiet(Option<Instant>);

impl Quiet {
    /// Writes the line unless one of its kind was written in the last [`QUIET`].
    fn say(&mut self, line: impl FnOnce() -> String) {
        self.said(line);
    }

    /// [`Self::say`], and whether the line was written.
    fn said(&mut self, line: impl FnOnce() -> String) -> bool {
        if self.0.is_some_and(|at| at.elapsed() < QUIET) {
            return false;
        }
        self.0 = Some(Instant::now());
        eprintln!("{}", line());
        true
    }
}

/// The lines of the log the task writes, each kept quiet on its own.
#[derive(Default)]
struct Log {
    liberapay: Quiet,
    github: Quiet,
    rates: Quiet,
    store: Quiet,
    unconverted: Quiet,
    refused: Quiet,
    said: KofiCounts,
}

/// Ko-fi's counts, as lines for what changed since `said`. A refusal's line
/// waits for its quiet hour; the others are rare and said at once.
fn counted(now: &KofiCounts, said: &KofiCounts) -> Vec<String> {
    let mut lines = Vec::new();
    if now.recorded > said.recorded {
        lines.push(format!(
            "central: {} Ko-fi payment(s) recorded since start",
            now.recorded
        ));
    }
    if now.unconverted > said.unconverted {
        lines.push(format!(
            "central: {} Ko-fi payment(s) came in a currency without a reference rate and wait for one",
            now.unconverted
        ));
    }
    if now.unrecorded > said.unrecorded {
        lines.push(format!(
            "central: {} Ko-fi delivery(ies) verified and read, and not recorded (kofi.record is false)",
            now.unrecorded
        ));
    }
    if now.sales > said.sales {
        lines.push(format!(
            "central: {} Ko-fi sale(s) verified and not counted: a sale is no donation",
            now.sales
        ));
    }
    lines
}

/// The refusals by reason, for the log: only those that happened.
fn refusals(now: &KofiCounts) -> Option<String> {
    let named: Vec<String> = central_funding::kofi::Refusal::ALL
        .iter()
        .zip(now.refused)
        .filter(|(_, count)| *count > 0)
        .map(|(reason, count)| format!("{} {count}", reason.as_str()))
        .collect();
    (!named.is_empty()).then(|| {
        format!(
            "central: Ko-fi deliveries refused since start, by reason: {} (docs/operations.md, \"Donations\")",
            named.join(", ")
        )
    })
}

async fn rest(duration: Duration, stop: &mut watch::Receiver<bool>) {
    tokio::select! {
        _ = stop.wait_for(|value| *value) => {}
        () = tokio::time::sleep(duration) => {}
    }
}

/// Runs `job` on a blocking thread with the shared fetch.
async fn blocking<T: Send + 'static>(
    fetch: &Arc<dyn Fetch>,
    job: impl FnOnce(&dyn Fetch) -> T + Send + 'static,
) -> Option<T> {
    let fetch = Arc::clone(fetch);
    tokio::task::spawn_blocking(move || job(fetch.as_ref()))
        .await
        .ok()
}

/// Asks what `plan` names until `stop` says so.
pub(super) async fn run(identity: Arc<IdentityService>, plan: Plan, stop: watch::Receiver<bool>) {
    let fetch: Arc<dyn Fetch> = Arc::new(Outbound::new());
    work(identity, plan, fetch, stop).await;
}

async fn work(
    identity: Arc<IdentityService>,
    plan: Plan,
    fetch: Arc<dyn Fetch>,
    mut stop: watch::Receiver<bool>,
) {
    if !plan.liberapay && plan.github.is_none() && !plan.rates && !plan.sums {
        return;
    }
    let mut log = Log::default();
    let mut platforms_due = Instant::now();
    let mut rates_due = Instant::now();
    let github = plan.github.map(Arc::new);
    while !*stop.borrow() {
        let now = Instant::now();
        if plan.rates && now >= rates_due {
            let fresh = blocking(&fetch, reference_rates).await.flatten();
            rates_due = now
                + if fresh.is_some() {
                    RATES
                } else {
                    // until a first rate is held, sooner
                    PLATFORMS
                };
            match fresh {
                Some(rates) => identity.funding_rates(rates),
                None => log.rates.say(|| {
                    "central: the ECB's reference rates did not come; the last ones stand"
                        .to_owned()
                }),
            }
        }
        if now >= platforms_due {
            platforms_due = now + PLATFORMS;
            if plan.liberapay {
                match blocking(&fetch, liberapay_figure).await.flatten() {
                    Some(monthly) => identity.funding_liberapay(monthly),
                    None => log.liberapay.say(|| {
                        "central: Liberapay's figures did not come; the last ones stand".to_owned()
                    }),
                }
            }
            if let Some(token) = &github {
                poll_github(&identity, &fetch, token, &mut log).await;
            }
        }
        if plan.sums {
            sums(&identity, &mut log).await;
        }
        rest(SUMS, &mut stop).await;
    }
}

async fn poll_github(
    identity: &IdentityService,
    fetch: &Arc<dyn Fetch>,
    token: &Arc<Zeroizing<String>>,
    log: &mut Log,
) {
    let Ok(since) = identity.github_window().await else {
        log.store
            .say(|| "central: the donations' store did not answer".to_owned());
        return;
    };
    let token = Arc::clone(token);
    let read = blocking(fetch, move |fetch| {
        github_figures(fetch, token.as_str(), &since)
    })
    .await
    .flatten();
    let Some((answer, _)) = read else {
        log.github.say(|| {
            "central: GitHub Sponsors' figures did not come (a token without the scope they need answers so too); the last ones stand".to_owned()
        });
        return;
    };
    identity.funding_github(answer.month, answer.sponsors);
    if identity.record_github(&answer.activities).await.is_err() {
        log.store
            .say(|| "central: the donations' store did not answer".to_owned());
    }
}

async fn sums(identity: &IdentityService, log: &mut Log) {
    match identity.convert_donations().await {
        Ok(waiting) if !waiting.is_empty() => log.unconverted.say(|| {
            let named: Vec<String> = waiting
                .iter()
                .map(|(currency, rows)| format!("{currency} {rows}"))
                .collect();
            format!(
                "central: donations not counted, for want of a reference rate: {}",
                named.join(", ")
            )
        }),
        Ok(_) => {}
        Err(_) => log
            .store
            .say(|| "central: the donations' store did not answer".to_owned()),
    }
    if identity.refresh_donations().await.is_err() {
        log.store
            .say(|| "central: the donations' store did not answer".to_owned());
    }
    let now = identity.kofi_counts();
    for line in counted(&now, &log.said) {
        eprintln!("{line}");
    }
    // The refusals wait for their quiet hour: a flood writes one line an hour.
    let mut refused = log.said.refused;
    if now.refused != refused
        && let Some(line) = refusals(&now)
        && log.refused.said(|| line)
    {
        refused = now.refused;
    }
    log.said = KofiCounts { refused, ..now };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// A token as the owner would paste one, made up.
    const GITHUB: &str = "synthetic-github-token-0123456789";
    const KOFI: &str = "synthetic-kofi-verification-token";

    fn parsed(text: &str) -> Result<Funding, serde_json::Error> {
        serde_json::from_str(text)
    }

    #[test]
    fn without_a_block_liberapay_alone_is_read() {
        let (sources, plan) = Funding::absent();
        assert!(sources.liberapay && !sources.github && sources.kofi.is_none());
        assert!(plan.liberapay && plan.github.is_none() && !plan.rates && !plan.sums);
        let (sources, plan) = parsed("{}").unwrap().plan().unwrap();
        assert!(sources.liberapay && !sources.github && sources.kofi.is_none());
        assert!(!plan.rates);
        assert_eq!(announced(&sources), "central: donations: Liberapay");
    }

    #[test]
    fn each_source_has_its_switch_and_its_token_is_moved_out() {
        let mut all = parsed(&format!(
            r#"{{"liberapay":false,"github":{{"token":"{GITHUB}"}},"kofi":{{"token":"{KOFI}","record":true}}}}"#
        ))
        .unwrap();
        let (sources, plan) = all.plan().unwrap();
        assert!(!sources.liberapay && sources.github);
        let kofi = sources.kofi.unwrap();
        assert!(kofi.record);
        assert_eq!(kofi.token, kofi_digest(KOFI));
        assert_eq!(plan.github.as_deref().map(String::as_str), Some(GITHUB));
        assert!(plan.rates && plan.sums && !plan.liberapay);
        assert_eq!(
            all.github.as_ref().unwrap().token,
            "",
            "moved out, not copied"
        );
        assert_eq!(
            announced(&sources),
            "central: donations: GitHub Sponsors, Ko-fi"
        );
        // switched off, with the tokens left in the file
        let (sources, plan) = parsed(&format!(
            r#"{{"github":{{"token":"{GITHUB}","on":false}},"kofi":{{"token":"{KOFI}","on":false}}}}"#
        ))
        .unwrap()
        .plan()
        .unwrap();
        assert!(!sources.github && sources.kofi.is_none() && plan.github.is_none());
        assert!(!plan.rates && !plan.sums);
        // Ko-fi without recording: verified only, nothing stored, rates still kept
        let (sources, plan) = parsed(&format!(r#"{{"kofi":{{"token":"{KOFI}"}}}}"#))
            .unwrap()
            .plan()
            .unwrap();
        assert!(!sources.kofi.unwrap().record);
        assert!(plan.rates && !plan.sums);
        assert_eq!(
            announced(&sources),
            "central: donations: Liberapay, Ko-fi (verifying only, kofi.record is false)"
        );
    }

    #[test]
    fn a_block_of_another_shape_or_a_token_that_is_none_is_refused() {
        for text in [
            r#"{"github":{}}"#,
            r#"{"github":"token"}"#,
            r#"{"github":{"token":"x","scope":"read:user"}}"#,
            r#"{"kofi":{"token":"synthetic-kofi-verification-token","record":"yes"}}"#,
            r#"{"kofi":{"token":"synthetic-kofi-verification-token","path":"/hook"}}"#,
            r#"{"liberapay":"true"}"#,
            r#"{"patreon":{"token":"x"}}"#,
        ] {
            assert!(parsed(text).is_err(), "{text}");
        }
        for text in [
            r#"{"github":{"token":"short"}}"#,
            r#"{"github":{"token":"with a space in the middle of it"}}"#,
            r#"{"kofi":{"token":"short"}}"#,
            r#"{"kofi":{"token":"synthetic kofi token"}}"#,
            // a token switched off is still checked: the file must be right
            r#"{"github":{"token":"short","on":false}}"#,
        ] {
            assert!(parsed(text).unwrap().plan().is_err(), "{text}");
        }
    }

    /// A platform as a test answers for it: fixed bodies by address, and what was
    /// asked.
    #[derive(Default)]
    struct Answers {
        get: Vec<(&'static str, String)>,
        pages: Mutex<Vec<String>>,
        asked: Mutex<Vec<String>>,
    }

    impl Fetch for Answers {
        fn get(&self, url: &str, _: &str, limit: u64) -> Option<String> {
            self.asked.lock().unwrap().push(url.to_owned());
            self.get
                .iter()
                .find(|(address, _)| *address == url)
                .map(|(_, body)| body.clone())
                .filter(|body| body.len() as u64 <= limit)
        }

        fn graphql(&self, url: &str, token: &str, body: &str, _: u64) -> Option<String> {
            assert_eq!(url, github::ADDRESS);
            assert_eq!(token, GITHUB);
            assert!(!body.contains(GITHUB), "the token goes in the header alone");
            self.asked.lock().unwrap().push(body.to_owned());
            let mut pages = self.pages.lock().unwrap();
            (!pages.is_empty()).then(|| pages.remove(0))
        }
    }

    fn page(month: u64, nodes: &[(&str, u64)], next: Option<&str>) -> String {
        let nodes: Vec<String> = nodes
            .iter()
            .map(|(id, cents)| {
                format!(
                    r#"{{"id":"{id}","action":"NEW_SPONSORSHIP","sponsorsTier":{{"isOneTime":true,"monthlyPriceInCents":{cents}}}}}"#
                )
            })
            .collect();
        format!(
            r#"{{"data":{{"viewer":{{"monthlyEstimatedSponsorsIncomeInCents":{month},"sponsorshipsAsMaintainer":{{"totalCount":2}},"sponsorsActivities":{{"pageInfo":{{"hasNextPage":{},"endCursor":{}}},"nodes":[{}]}}}}}}}}"#,
            next.is_some(),
            next.map_or("null".to_owned(), |cursor| format!("\"{cursor}\"")),
            nodes.join(",")
        )
    }

    #[test]
    fn github_s_pages_are_read_to_the_end_or_not_at_all() {
        let answers = Answers {
            pages: Mutex::new(vec![
                page(4_600, &[("SA_one", 500)], Some("Y3Vyc29yOjE=")),
                page(4_600, &[("SA_two", 2_500)], None),
            ]),
            ..Answers::default()
        };
        let (answer, pages) =
            github_figures(&answers, GITHUB, "2026-11-18T00:00:00Z").expect("two pages");
        assert_eq!((answer.month, answer.sponsors, pages), (4_600, 2, 2));
        assert_eq!(
            answer
                .activities
                .iter()
                .map(|a| a.cents)
                .collect::<Vec<_>>(),
            vec![500, 2_500]
        );
        let asked = answers.asked.lock().unwrap();
        assert!(
            asked[1].contains("Y3Vyc29yOjE="),
            "the second page by its cursor"
        );
        // a page that fails fails the poll
        let broken = Answers {
            pages: Mutex::new(vec![page(4_600, &[], Some("Y3Vyc29yOjE="))]),
            ..Answers::default()
        };
        assert!(github_figures(&broken, GITHUB, "2026-11-18T00:00:00Z").is_none());
        // more pages than are read: none of it is taken
        let endless = Answers {
            pages: Mutex::new(
                (0..=github::PAGES)
                    .map(|_| page(1, &[], Some("bW9yZQ==")))
                    .collect(),
            ),
            ..Answers::default()
        };
        assert!(github_figures(&endless, GITHUB, "2026-11-18T00:00:00Z").is_none());
    }

    #[test]
    fn liberapay_and_the_rates_are_read_from_their_fixed_addresses() {
        let answers = Answers {
            get: vec![
                (
                    liberapay::ADDRESS,
                    r#"{"receiving":{"amount":"23.10","currency":"EUR"},"npatrons":3}"#.to_owned(),
                ),
                (
                    rates::ADDRESS,
                    "<e xmlns='http://www.ecb.int/vocabulary/2002-08-01/eurofxref'><Cube><Cube time='2026-11-18'><Cube currency='USD' rate='1.0968'/></Cube></Cube></e>".to_owned(),
                ),
            ],
            ..Answers::default()
        };
        assert_eq!(
            liberapay_figure(&answers),
            Some(Monthly {
                month: 10_010,
                supporters: 3
            })
        );
        let rates = reference_rates(&answers).unwrap();
        assert_eq!(rates.rate(central_funding::Currency::USD), Some(1_096_800));
        assert_eq!(
            *answers.asked.lock().unwrap(),
            vec![liberapay::ADDRESS.to_owned(), rates::ADDRESS.to_owned()]
        );
        // a platform that does not answer gives nothing, never a zero
        let silent = Answers::default();
        assert_eq!(liberapay_figure(&silent), None);
        assert!(reference_rates(&silent).is_none());
    }

    #[test]
    fn the_log_names_counts_and_reasons_never_a_value() {
        let mut now = KofiCounts::default();
        assert_eq!(counted(&now, &KofiCounts::default()), Vec::<String>::new());
        assert_eq!(refusals(&now), None);
        now.refused[3] = 3;
        now.refused[1] = 1;
        now.recorded = 2;
        now.unrecorded = 1;
        assert_eq!(
            refusals(&now).unwrap(),
            "central: Ko-fi deliveries refused since start, by reason: form 1, token 3 (docs/operations.md, \"Donations\")"
        );
        let lines = counted(&now, &KofiCounts::default());
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("2 Ko-fi payment(s) recorded"));
        assert!(lines[1].contains("kofi.record is false"));
        assert_eq!(
            counted(&now, &now),
            Vec::<String>::new(),
            "nothing new, nothing said"
        );
    }

    #[test]
    fn a_github_token_has_a_token_s_shape() {
        assert!(plain_github_token(GITHUB));
        assert!(plain_github_token(&"a".repeat(93)));
        for token in [
            "",
            "short",
            &"a".repeat(256),
            "token with spaces in it",
            "tökentökentökentöken",
        ] {
            assert!(!plain_github_token(token), "{token:?}");
        }
    }
}
