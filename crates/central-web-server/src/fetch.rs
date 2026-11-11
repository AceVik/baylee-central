//! The public data the pages show, read by this server and never by the
//! visitor's browser (no third party learns who reads the site): Baylee's
//! newest release, its card status and the CI state from GitHub, every ten
//! minutes; Central's gateway directory and its public figures (the number
//! of accounts, and what each gateway says is happening on it), every
//! minute. What does not answer keeps its last good value, else the
//! deployment's fallback copy; the figures are never kept beyond the minute
//! they were read in, so a number nobody reports any more goes.
//!
//! Only two fixed hosts are asked over HTTPS (`api.github.com` and
//! `raw.githubusercontent.com`, Baylee's repository only), with no proxy,
//! no redirect, bounded answers and a ten-second deadline; Central over
//! plain HTTP on loopback, as the site's own Caddy would.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use central_api::gateway::{GatewayListing, validate_origin};
use central_api::wire::{Request, Response};
use central_web::data::{
    Activity, CardStats, CardStatus, Gateway, Release, ReleaseFile, Repo, classify,
};
use central_web::site::Snapshot;
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::config::Config;

/// Baylee's repository on GitHub's API.
const API: &str = "https://api.github.com/repos/AceVik/baylee";
/// Baylee's files at a tag.
const RAW: &str = "https://raw.githubusercontent.com/AceVik/baylee";
/// Where a release's files must be downloaded from.
const DOWNLOADS: &str = "https://github.com/AceVik/baylee/releases/download/";
/// The largest answer read from GitHub.
const LIMIT: u64 = 4 * 1024 * 1024;

/// The data, shared by every request: a request takes the `Arc` and lets
/// the lock go; a change installs a new `Arc` ([`install`]), which also
/// tells the rendered pages they are stale (`routes::Pages`).
pub type Shared = Arc<RwLock<Arc<Snapshot>>>;

/// Whether two snapshots hold the same data. Every field is named, so a
/// new one cannot be forgotten here.
fn same(one: &Snapshot, other: &Snapshot) -> bool {
    let Snapshot {
        release,
        cards,
        gateways,
        repo,
        accounts,
        views,
    } = one;
    *release == other.release
        && *cards == other.cards
        && *gateways == other.gateways
        && *repo == other.repo
        && *accounts == other.accounts
        && *views == other.views
}

/// Change the data in `shared` with `change`: a new snapshot when that
/// changes anything, else the one there stays (and so do its pages).
pub fn install(shared: &Shared, change: impl FnOnce(&mut Snapshot)) {
    let Ok(mut current) = shared.write() else {
        return;
    };
    let mut next = Snapshot::clone(&current);
    change(&mut next);
    if !same(&current, &next) {
        *current = Arc::new(next);
    }
}

fn agent(https: bool) -> ureq::Agent {
    let config = ureq::Agent::config_builder()
        .https_only(https)
        .proxy(None)
        .max_redirects(0)
        .timeout_global(Some(Duration::from_secs(10)))
        .timeout_connect(Some(Duration::from_secs(3)))
        .max_response_header_size(16 * 1024)
        .http_status_as_error(false)
        .user_agent("baylee.app")
        .build();
    ureq::Agent::new_with_config(config)
}

fn get<T: DeserializeOwned>(agent: &ureq::Agent, url: &str) -> Option<T> {
    let mut response = agent
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
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
    serde_json::from_str(&text).ok()
}

#[derive(Deserialize)]
struct GhAsset {
    name: String,
    size: u64,
    browser_download_url: String,
}

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    html_url: String,
    published_at: Option<String>,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<GhAsset>,
}

fn plain_tag(tag: &str) -> bool {
    (1..=64).contains(&tag.len())
        && tag
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'+' | b'_'))
}

/// The newest release that is not a draft and offers at least one client
/// file, in GitHub's order (newest first), its files checked to come from
/// Baylee's own releases.
fn pick(releases: Vec<GhRelease>) -> Option<Release> {
    releases.into_iter().find_map(|release| {
        if release.draft
            || !plain_tag(&release.tag_name)
            || !release
                .html_url
                .starts_with("https://github.com/AceVik/baylee/")
        {
            return None;
        }
        let on_github = |asset: &GhAsset| asset.browser_download_url.starts_with(DOWNLOADS);
        let beside = |name: &str| {
            release
                .assets
                .iter()
                .find(|asset| asset.name == name && on_github(asset))
                .map(|asset| asset.browser_download_url.clone())
        };
        let files: Vec<ReleaseFile> = release
            .assets
            .iter()
            .filter(|asset| on_github(asset))
            .filter_map(|asset| {
                let (platform, kind) = classify(&asset.name)?;
                Some(ReleaseFile {
                    name: asset.name.clone(),
                    platform,
                    kind,
                    size: asset.size,
                    url: asset.browser_download_url.clone(),
                    checksum: beside(&format!("{}.sha256", asset.name)),
                    signature: beside(&format!("{}.sig", asset.name)),
                })
            })
            .collect();
        if files.is_empty() {
            return None;
        }
        let date = release
            .published_at
            .as_deref()
            .and_then(|d| d.get(..10))
            .unwrap_or("")
            .to_owned();
        Some(Release {
            tag: release.tag_name,
            page: release.html_url,
            date,
            prerelease: release.prerelease,
            files,
        })
    })
}

#[derive(Deserialize)]
struct Totals {
    #[serde(default)]
    implemented: u64,
    #[serde(default)]
    generated: u64,
    #[serde(default)]
    partial: u64,
}

#[derive(Deserialize)]
struct SetsFile {
    corpus: u64,
    generated_at: String,
    totals: Totals,
}

fn cards(agent: &ureq::Agent, tag: &str) -> Option<CardStatus> {
    let sets: SetsFile = get(agent, &format!("{RAW}/{tag}/web/site/data/sets.json"))?;
    Some(CardStatus {
        stats: CardStats {
            implemented: sets.totals.implemented,
            generated: sets.totals.generated,
            partial: sets.totals.partial,
            total: sets.corpus,
        },
        tag: tag.to_owned(),
        date: sets.generated_at.get(..10).unwrap_or("").to_owned(),
    })
}

#[derive(Deserialize)]
struct Runs {
    workflow_runs: Vec<Run>,
}

#[derive(Deserialize)]
struct Run {
    conclusion: Option<String>,
}

#[derive(Deserialize)]
struct Commit {
    sha: String,
    commit: CommitInner,
}

#[derive(Deserialize)]
struct CommitInner {
    committer: Committer,
    #[serde(default)]
    message: String,
}

#[derive(Deserialize)]
struct Repository {
    stargazers_count: u64,
    open_issues_count: u64,
}

/// A commit's subject: its first line, at most 120 characters, no control
/// characters (it is shown as text, never as markup).
fn subject(message: &str) -> Option<String> {
    let line: String = message
        .lines()
        .next()?
        .chars()
        .filter(|c| !c.is_control())
        .take(120)
        .collect();
    let line = line.trim().to_owned();
    (!line.is_empty()).then_some(line)
}

#[derive(Deserialize)]
struct Committer {
    date: String,
}

fn repo(agent: &ureq::Agent) -> Repo {
    let ci = get::<Runs>(
        agent,
        &format!(
            "{API}/actions/workflows/ci.yml/runs?branch=main&event=push&status=completed&per_page=1"
        ),
    )
    .and_then(|runs| runs.workflow_runs.into_iter().next())
    .and_then(|run| run.conclusion)
    .map(|conclusion| conclusion == "success");
    let newest = get::<Vec<Commit>>(agent, &format!("{API}/commits?sha=main&per_page=1"))
        .and_then(|commits| commits.into_iter().next())
        .filter(|commit| {
            commit.sha.len() >= 7 && commit.sha.bytes().all(|b| b.is_ascii_hexdigit())
        });
    let subject = newest
        .as_ref()
        .and_then(|commit| subject(&commit.commit.message));
    let commit = newest.map(|commit| {
        (
            commit.sha[..7].to_owned(),
            commit
                .commit
                .committer
                .date
                .get(..10)
                .unwrap_or("")
                .to_owned(),
        )
    });
    let repository = get::<Repository>(agent, API);
    Repo {
        ci,
        commit,
        subject,
        stars: repository.as_ref().map(|r| r.stargazers_count),
        issues: repository.as_ref().map(|r| r.open_issues_count),
    }
}

fn valid_id(id: [u8; 16]) -> bool {
    id[6] >> 4 == 7 && id[8] >> 6 == 2
}

/// Where Central is asked, and as whom: the site's own Caddy.
struct Central<'a> {
    agent: &'a ureq::Agent,
    address: std::net::SocketAddr,
    host: &'a str,
    origin: &'a str,
}

impl Central<'_> {
    /// One public request; `None` when Central does not answer `200` with
    /// a response.
    fn ask(&self, request: &Request) -> Option<Response> {
        let body = serde_json::to_string(request).ok()?;
        let mut response = self
            .agent
            .post(&format!("http://{}/api/v1", self.address))
            .header("Host", self.host)
            .header("Origin", self.origin)
            .header("Content-Type", "application/json")
            .header("X-Baylee-Central", "1")
            .header("X-Forwarded-For", "127.0.0.1")
            .send(body.as_bytes())
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
        serde_json::from_str(&text).ok()
    }
}

/// The most pages read of the directory or of the figures: 2000 gateways.
const PAGES: usize = 20;

/// Central's whole directory, checked as `central-client` checks it;
/// `None` when Central does not answer or answers wrong.
fn listings(central: &Central<'_>) -> Option<Vec<GatewayListing>> {
    let development = central.origin.starts_with("http://");
    let mut after: Option<[u8; 16]> = None;
    let mut all: Vec<GatewayListing> = Vec::new();
    for _ in 0..PAGES {
        let Response::GatewayDirectory { gateways } =
            central.ask(&Request::GatewayDirectory { after })?
        else {
            return None;
        };
        if gateways.len() > 100 {
            return None;
        }
        for gateway in &gateways {
            if !valid_id(gateway.id) || after.is_some_and(|id| id >= gateway.id) {
                return None;
            }
            validate_origin(&gateway.origin, development).ok()?;
            gateway.metadata.validate(&gateway.origin).ok()?;
            after = Some(gateway.id);
        }
        let full = gateways.len() == 100;
        all.extend(gateways);
        if !full {
            break;
        }
    }
    Some(all)
}

/// Central's public figures, checked like the directory: the number of
/// accounts and what each online gateway reports, by its id. `None` when
/// Central does not answer, answers wrong or is a Central from before the
/// figures; the directory is shown without them then.
fn figures(central: &Central<'_>) -> Option<(u64, BTreeMap<[u8; 16], Activity>)> {
    let mut after: Option<[u8; 16]> = None;
    let mut counted = None;
    let mut all = BTreeMap::new();
    for _ in 0..PAGES {
        let Response::PublicFigures { accounts, gateways } =
            central.ask(&Request::PublicFigures { after })?
        else {
            return None;
        };
        if gateways.len() > 100 {
            return None;
        }
        counted.get_or_insert(accounts);
        let full = gateways.len() == 100;
        for gateway in gateways {
            if !valid_id(gateway.id) || after.is_some_and(|id| id >= gateway.id) {
                return None;
            }
            gateway.activity.validate().ok()?;
            after = Some(gateway.id);
            all.insert(
                gateway.id,
                Activity {
                    players: gateway.activity.players,
                    playing: gateway.activity.playing,
                    tables: gateway.activity.tables,
                    games: gateway.activity.games,
                },
            );
        }
        if !full {
            break;
        }
    }
    Some((counted?, all))
}

/// The directory as the pages show it, official first, each online gateway
/// with what it reports.
fn shown(all: Vec<GatewayListing>, reported: &BTreeMap<[u8; 16], Activity>) -> Vec<Gateway> {
    let mut out: Vec<Gateway> = all
        .into_iter()
        .map(|gateway| Gateway {
            official: central_api::is_official_origin(&gateway.origin),
            // only what an online gateway said counts as now
            activity: reported
                .get(&gateway.id)
                .copied()
                .filter(|_| gateway.online),
            origin: gateway.origin,
            name: gateway.metadata.name,
            version: gateway.metadata.version,
            website: gateway.metadata.website,
            online: gateway.online,
        })
        .collect();
    out.sort_by_key(|gateway| !gateway.official);
    out
}

/// Central's directory with its figures: the gateways, and the number of
/// accounts when Central says it. `None` when the directory does not come.
fn directory(
    agent: &ureq::Agent,
    central: std::net::SocketAddr,
    host: &str,
    origin: &str,
) -> Option<(Vec<Gateway>, Option<u64>)> {
    let central = Central {
        agent,
        address: central,
        host,
        origin,
    };
    let all = listings(&central)?;
    let (accounts, reported) = match figures(&central) {
        Some((accounts, reported)) => (Some(accounts), reported),
        None => (None, BTreeMap::new()),
    };
    Some((shown(all, &reported), accounts))
}

fn read<T: DeserializeOwned>(dir: &Path, name: &str) -> Option<T> {
    let text = std::fs::read_to_string(dir.join(name)).ok()?;
    serde_json::from_str(&text).ok()
}

/// The deployment's fallback copy (and the tests' data).
#[must_use]
pub fn fallback(dir: Option<&Path>) -> Snapshot {
    let Some(dir) = dir else {
        return Snapshot::default();
    };
    Snapshot {
        release: read(dir, "release.json"),
        cards: read(dir, "cards.json"),
        gateways: read(dir, "gateways.json"),
        repo: read(dir, "repo.json").unwrap_or_default(),
        accounts: read(dir, "accounts.json"),
        views: read(dir, "views.json"),
    }
}

/// Takes away what was true for a minute only: the number of accounts and
/// every gateway's own counts.
fn forget_figures(snapshot: &mut Snapshot) {
    snapshot.accounts = None;
    for gateway in snapshot.gateways.iter_mut().flatten() {
        gateway.activity = None;
    }
}

/// Keeps `shared` fresh until the process ends: GitHub every ten minutes,
/// Central's directory and figures every minute.
pub fn spawn(config: &Config, shared: &Shared) {
    if config.github {
        let shared = Arc::clone(shared);
        tokio::spawn(async move {
            let mut every = tokio::time::interval(Duration::from_secs(600));
            loop {
                every.tick().await;
                let found = tokio::task::spawn_blocking(|| {
                    let agent = agent(true);
                    let releases: Option<Vec<GhRelease>> =
                        get(&agent, &format!("{API}/releases?per_page=10"));
                    let release = releases.and_then(pick);
                    let cards = release
                        .as_ref()
                        .and_then(|release| cards(&agent, &release.tag));
                    (release, cards, repo(&agent))
                })
                .await;
                if let Ok((release, cards, repo)) = found {
                    install(&shared, |snapshot| {
                        if release.is_some() {
                            snapshot.release = release;
                        }
                        if cards.is_some() {
                            snapshot.cards = cards;
                        }
                        if repo.ci.is_some() || repo.commit.is_some() || repo.stars.is_some() {
                            snapshot.repo = repo;
                        }
                    });
                }
            }
        });
    }
    if let Some(central) = config.central {
        let (host, origin) = config.central_site();
        let shared = Arc::clone(shared);
        tokio::spawn(async move {
            let mut every = tokio::time::interval(Duration::from_secs(60));
            loop {
                every.tick().await;
                let (host, origin) = (host.clone(), origin.clone());
                let found = tokio::task::spawn_blocking(move || {
                    directory(&agent(false), central, &host, &origin)
                })
                .await;
                install(&shared, |snapshot| match found {
                    Ok(Some((gateways, accounts))) => {
                        snapshot.gateways = Some(gateways);
                        snapshot.accounts = accounts;
                    }
                    // The directory keeps its last good value; "right now"
                    // does not: without Central nobody says it any more.
                    _ => forget_figures(snapshot),
                });
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(name: &str) -> GhAsset {
        GhAsset {
            name: name.into(),
            size: 1_000_000,
            browser_download_url: format!("{DOWNLOADS}v1/{name}"),
        }
    }

    #[test]
    fn a_commit_subject_is_one_short_line_of_text() {
        assert_eq!(
            subject("Card rules: Equip\n\nThe body.").as_deref(),
            Some("Card rules: Equip")
        );
        assert_eq!(subject("a\u{7}b\tc").as_deref(), Some("abc"));
        assert_eq!(
            subject(&"x".repeat(500)).map(|s| s.chars().count()),
            Some(120)
        );
        assert_eq!(subject("   \n body"), None);
        assert_eq!(subject(""), None);
    }

    #[test]
    fn picks_the_newest_release_with_files_from_github() {
        let draft = GhRelease {
            tag_name: "v2".into(),
            html_url: "https://github.com/AceVik/baylee/releases/tag/v2".into(),
            published_at: None,
            draft: true,
            prerelease: false,
            assets: vec![asset("Baylee-2-x86_64.AppImage")],
        };
        let foreign = GhRelease {
            tag_name: "v1.5".into(),
            html_url: "https://github.com/AceVik/baylee/releases/tag/v1.5".into(),
            published_at: None,
            draft: false,
            prerelease: false,
            assets: vec![GhAsset {
                browser_download_url: "https://evil.example/x".into(),
                ..asset("Baylee-1.5-x86_64.AppImage")
            }],
        };
        let good = GhRelease {
            tag_name: "v1".into(),
            html_url: "https://github.com/AceVik/baylee/releases/tag/v1".into(),
            published_at: Some("2026-11-01T10:00:00Z".into()),
            draft: false,
            prerelease: true,
            assets: vec![
                asset("Baylee-1-x86_64.AppImage"),
                asset("Baylee-1-x86_64.AppImage.sha256"),
                asset("notes.txt"),
            ],
        };
        let release = pick(vec![draft, foreign, good]).expect("a release");
        assert_eq!(release.tag, "v1");
        assert_eq!(release.date, "2026-11-01");
        assert_eq!(release.files.len(), 1);
        assert!(
            release.files[0]
                .checksum
                .as_deref()
                .is_some_and(|url| url.ends_with(".sha256"))
        );
        assert!(release.files[0].signature.is_none());
    }

    #[test]
    fn a_change_installs_a_new_snapshot_and_no_change_keeps_the_old() {
        let shared: Shared = Arc::new(RwLock::new(Arc::new(fallback(None))));
        let first = Arc::clone(&shared.read().unwrap());
        install(&shared, |snapshot| snapshot.gateways = None);
        assert!(
            Arc::ptr_eq(&first, &shared.read().unwrap()),
            "nothing changed"
        );
        install(&shared, |snapshot| snapshot.gateways = Some(Vec::new()));
        let second = Arc::clone(&shared.read().unwrap());
        assert!(!Arc::ptr_eq(&first, &second), "a new snapshot");
        assert_eq!(first.gateways, None, "the old one is left as it was");
        assert_eq!(second.gateways, Some(Vec::new()));
    }

    fn listing(id: u8, origin: &str, online: bool) -> GatewayListing {
        let mut bytes = [id; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        GatewayListing {
            id: bytes,
            origin: origin.into(),
            metadata: central_api::gateway::GatewayMetadata {
                name: format!("Synthetic {id}"),
                version: "1.0.0".into(),
                website: format!("{origin}/"),
            },
            online,
        }
    }

    #[test]
    fn only_an_online_gateway_s_own_report_is_shown_and_it_goes_with_central() {
        let counts = |players| Activity {
            players,
            playing: players - 3,
            tables: 2,
            games: 1,
        };
        let all = vec![
            listing(1, "https://one.example.org", true),
            listing(2, "https://two.example.org", false),
            listing(3, "https://gateway.baylee.app", true),
            listing(4, "https://four.example.org", true),
        ];
        let reported: BTreeMap<[u8; 16], Activity> = all
            .iter()
            .filter(|gateway| gateway.origin != "https://four.example.org")
            .zip([7, 8, 9])
            .map(|(gateway, players)| (gateway.id, counts(players)))
            .collect();
        let shown = shown(all, &reported);
        assert!(shown[0].official, "the official gateway first");
        let of = |origin: &str| {
            shown
                .iter()
                .find(|gateway| gateway.origin == origin)
                .expect("listed")
                .activity
        };
        assert_eq!(of("https://one.example.org"), Some(counts(7)));
        assert_eq!(of("https://gateway.baylee.app"), Some(counts(9)));
        // offline: what it said before is not "now"; and one that says nothing
        assert_eq!(of("https://two.example.org"), None);
        assert_eq!(of("https://four.example.org"), None);
        assert_eq!(
            central_web::data::activity_of(&shown),
            Some((16, 10, 4, 2)),
            "the strip adds up what was reported"
        );

        let mut snapshot = Snapshot {
            gateways: Some(shown),
            accounts: Some(12),
            ..Snapshot::default()
        };
        forget_figures(&mut snapshot);
        assert_eq!(snapshot.accounts, None);
        let gateways = snapshot.gateways.expect("the directory stays");
        assert_eq!(gateways.len(), 4);
        assert!(gateways.iter().all(|gateway| gateway.activity.is_none()));
        assert_eq!(central_web::data::activity_of(&gateways), None);
    }

    #[test]
    fn tags_are_plain() {
        assert!(plain_tag("v0.1.1-beta.9b"));
        assert!(!plain_tag("v1/../../x"));
        assert!(!plain_tag(""));
    }
}
