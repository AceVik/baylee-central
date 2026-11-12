//! Every page in every language, as a browser and a search engine get it:
//! status, headers, one `h1`, canonical address and languages, sharing
//! card, structured data that parses, no inline style or script, pictures
//! with their size, the Fan Content notice, and nothing from another origin;
//! and how a browser's cache revalidates them.
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use central_web::data::{Section, machine};
use central_web::site::{Assets, Context, Page, render};
use central_web::words::Lang;
use central_web_server::config::Config;
use central_web_server::fetch::{fallback, install};
use central_web_server::routes::router;
use tower::ServiceExt as _;

/// A throwaway site folder: an empty manifest and the root files.
fn site() -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "central-web-test-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("temporary folder");
    std::fs::write(dir.join("manifest.json"), "{\"files\":{}}").expect("manifest");
    for name in central_web_server::files::ROOT {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("root file's folder");
        }
        std::fs::write(path, b"x").expect("root file");
    }
    // the BIMI mark as the build copies it
    std::fs::write(dir.join("bimi/baylee.svg"), bimi()).expect("BIMI mark");
    dir
}

/// The BIMI mark in the site's sources.
fn bimi() -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../central-web/assets/bimi/baylee.svg"),
    )
    .expect("BIMI mark")
}

fn data() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data")
}

fn app(fallback: bool) -> axum::Router {
    let mut config = Config::development(site(), fallback.then(data));
    config.origin = "https://baylee.app".into();
    router(central_web_server::app(config).expect("app"))
}

async fn get(
    app: &axum::Router,
    path: &str,
    headers: &[(&str, &str)],
) -> (StatusCode, axum::http::HeaderMap, String) {
    let mut request = Request::builder().uri(path);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::empty()).expect("request"))
        .await
        .expect("response");
    let (parts, body) = response.into_parts();
    let bytes = to_bytes(body, 4 * 1024 * 1024).await.expect("body");
    (
        parts.status,
        parts.headers,
        String::from_utf8_lossy(&bytes).into_owned(),
    )
}

fn count(haystack: &str, needle: &str) -> usize {
    haystack.matches(needle).count()
}

/// Every `<tag …>` opening tag of a kind.
fn tags<'a>(html: &'a str, tag: &str) -> Vec<&'a str> {
    let open = format!("<{tag}");
    let mut out = Vec::new();
    let mut rest = html;
    while let Some(at) = rest.find(&open) {
        let after = &rest[at..];
        let end = after.find('>').unwrap_or(after.len());
        let candidate = &after[..end];
        if candidate[open.len()..].starts_with([' ', '>', '/']) || candidate.len() == open.len() {
            out.push(candidate);
        }
        rest = &after[end..];
    }
    out
}

/// The Fan Content notice in the page's language, and on a page a machine
/// translated, the note that says so.
fn check_notices(html: &str, lang: Lang, page: Page) {
    let code = lang.code();
    // the notice in the page's language, as the words have it: its
    // opening up to the policy's name
    let notice = central_web::words::fan_content(lang)
        .into_iter()
        .next()
        .map(|piece| piece.text)
        .unwrap_or_default();
    assert!(
        notice.contains("Baylee") && html.contains(notice.trim()),
        "{code} {page:?}: the Fan Content notice"
    );
    assert_eq!(
        html.contains("class=\"machine-note\""),
        lang.machine_translated(),
        "{code} {page:?}: the note on a machine's translation"
    );
}

fn check_page(html: &str, lang: Lang, page: Page) {
    let code = lang.code();
    assert!(
        html.starts_with("<!doctype html><html lang=\""),
        "{code} {page:?}: a doctype and html lang"
    );
    assert!(
        html.contains(&format!("<html lang=\"{}\"", lang.tag())),
        "{code} {page:?}: lang"
    );
    assert_eq!(count(html, "<h1"), 1, "{code} {page:?}: one h1");
    assert_eq!(count(html, "<title>"), 1, "{code} {page:?}: one title");
    assert!(
        html.contains("<meta name=\"description\" content=\""),
        "{code} {page:?}: description"
    );
    assert!(
        !html.contains(" style=\""),
        "{code} {page:?}: no inline style"
    );
    for script in tags(html, "script") {
        assert!(
            script.contains(" src=\"/assets/") || script.contains("type=\"application/ld+json\""),
            "{code} {page:?}: only external scripts and data blocks: {script}"
        );
    }
    for img in tags(html, "img") {
        assert!(img.contains(" alt=\""), "{code} {page:?}: alt: {img}");
        assert!(
            img.contains(" width=\"") && img.contains(" height=\""),
            "{code} {page:?}: size: {img}"
        );
    }
    for attribute in [" src=\"http", " href=\"http://", " srcset=\"http"] {
        assert!(
            !html.contains(attribute),
            "{code} {page:?}: nothing loaded from elsewhere ({attribute})"
        );
    }
    // a link that leaves the site opens a tab of its own and tells the other
    // site nothing about this one
    for anchor in tags(html, "a") {
        let away =
            anchor.contains(" href=\"https://") && !anchor.contains(" href=\"https://baylee.app/");
        if away {
            assert!(
                anchor.contains(" target=\"_blank\"") && anchor.contains("noopener noreferrer"),
                "{code} {page:?}: a link away opens its own tab: {anchor}"
            );
        } else {
            assert!(
                !anchor.contains(" target=\""),
                "{code} {page:?}: a link within the site stays in the tab: {anchor}"
            );
        }
    }
    let canonical = format!(
        "<link rel=\"canonical\" href=\"https://baylee.app{}\"",
        page.path(lang)
    );
    assert!(html.contains(&canonical), "{code} {page:?}: canonical");
    for other in Lang::ALL {
        let alternate = format!(
            "hreflang=\"{}\" href=\"https://baylee.app{}\"",
            other.tag(),
            page.path(other)
        );
        assert!(
            html.contains(&alternate),
            "{code} {page:?}: hreflang {}",
            other.tag()
        );
    }
    assert!(
        html.contains("hreflang=\"x-default\""),
        "{code} {page:?}: x-default"
    );
    assert!(
        html.contains("property=\"og:image\" content=\"https://baylee.app/"),
        "{code} {page:?}: og:image"
    );
    assert!(
        html.contains("company.wizards.com/"),
        "{code} {page:?}: the Fan Content Policy link"
    );
    check_notices(html, lang, page);
    for block in html.split("<script type=\"application/ld+json\">").skip(1) {
        let json = block.split("</script>").next().unwrap_or("");
        let value: serde_json::Value = serde_json::from_str(json).expect("structured data parses");
        assert_eq!(
            value["@context"], "https://schema.org",
            "{code} {page:?}: schema.org"
        );
    }
}

#[tokio::test]
async fn every_page_in_every_language() {
    let app = app(true);
    for lang in Lang::ALL {
        for page in Page::LISTED {
            let (status, headers, html) = get(&app, &page.path(lang), &[]).await;
            assert_eq!(status, StatusCode::OK, "{page:?} {}", lang.code());
            assert_eq!(headers["content-type"], "text/html; charset=utf-8");
            assert!(
                headers["content-security-policy"]
                    .to_str()
                    .unwrap_or("")
                    .contains("default-src 'none'")
            );
            check_page(&html, lang, page);
            let noindex = html.contains("<meta name=\"robots\" content=\"noindex");
            assert_eq!(noindex, !page.indexed(), "{page:?}: robots");
        }
    }
}

#[tokio::test]
async fn pages_hold_up_without_any_data() {
    let app = app(false);
    for page in Page::LISTED {
        let (status, _, html) = get(&app, &page.path(Lang::En), &[]).await;
        assert_eq!(status, StatusCode::OK, "{page:?}");
        check_page(&html, Lang::En, page);
    }
    let (_, _, html) = get(&app, "/en/download", &[]).await;
    assert!(
        html.contains("github.com/AceVik/baylee/releases"),
        "the releases page is offered"
    );
}

#[tokio::test]
async fn the_download_follows_the_machine() {
    let app = app(true);
    let windows = [(
        "user-agent",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/140.0",
    )];
    let (_, headers, html) = get(&app, "/en/download", &windows).await;
    assert!(html.contains("Download for Windows"), "the Windows button");
    assert!(
        html.contains("Baylee-Setup-0.2.0-beta.o1-x64.exe"),
        "the installer first"
    );
    assert!(headers.contains_key("accept-ch"));
    let arm = [
        (
            "user-agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/140.0",
        ),
        ("sec-ch-ua-platform", "\"Windows\""),
        ("sec-ch-ua-arch", "\"arm\""),
        ("sec-ch-ua-bitness", "\"64\""),
    ];
    let (_, _, html) = get(&app, "/en/download", &arm).await;
    assert!(html.contains("href=\"https://github.com/AceVik/baylee/releases/download/v0.2.0-beta.o1/Baylee-Setup-0.2.0-beta.o1-arm64.exe\""));
    let phone = [(
        "user-agent",
        "Mozilla/5.0 (Linux; Android 15; Pixel 7) Mobile",
    )];
    let (_, _, html) = get(&app, "/de/download", &phone).await;
    assert!(html.contains("Für Handys und Tablets gibt es noch keine Version."));
    assert!(
        html.contains("Archiv für ältere Prozessoren"),
        "the -v2 fallback is offered"
    );
}

#[tokio::test]
async fn operator_text_is_never_markup() {
    let app = app(true);
    let (_, _, html) = get(&app, "/en/gateways", &[]).await;
    assert!(
        html.contains("Kitchen &lt;Table&gt;"),
        "a gateway's name is text"
    );
    assert!(!html.contains("Kitchen <Table>"));
    // the server's order: the origin Central's API calls official first
    let first = html.find("Sanctuary").expect("first");
    let other = html.find("Kitchen").expect("other");
    assert!(first < other, "the server's order");
}

/// There are no official gateways (owner, 12.11.2026): no card, list,
/// chip or count says that one is, although Central's directory still
/// names an official origin.
#[tokio::test]
async fn no_gateway_is_marked_official() {
    let app = app(true);
    for path in ["/en/", "/en/gateways", "/de/", "/de/gateways"] {
        let (_, _, html) = get(&app, path, &[]).await;
        assert!(html.contains("Sanctuary"), "{path}: the gateway is listed");
        for never in [
            ">Official<",
            ">Offiziell<",
            "gw-official",
            "gw-is-official",
            "data-official",
            "data-show=\"official\"",
            "marked official",
            "als offiziell markierte",
        ] {
            assert!(!html.contains(never), "{path}: {never}");
        }
    }
}

/// An address, the request's headers, the status and where it sends to.
type Case = (
    &'static str,
    &'static [(&'static str, &'static str)],
    StatusCode,
    Option<&'static str>,
);

#[tokio::test]
async fn addresses_and_status_codes() {
    let app = app(true);
    let cases: [Case; 10] = [
        (
            "/",
            &[("accept-language", "de-DE,de;q=0.9,en;q=0.5")],
            StatusCode::FOUND,
            Some("/de/"),
        ),
        ("/", &[], StatusCode::FOUND, Some("/en/")),
        (
            "/download?lang=de",
            &[],
            StatusCode::FOUND,
            Some("/de/download"),
        ),
        (
            "/account/chat?with=3fa2&lang=de",
            &[],
            StatusCode::FOUND,
            Some("/de/account/chat?with=3fa2"),
        ),
        (
            "/register",
            &[("accept-language", "en")],
            StatusCode::FOUND,
            Some("/en/register"),
        ),
        (
            "/cards",
            &[("accept-language", "de")],
            StatusCode::FOUND,
            Some("/de/#cards"),
        ),
        (
            "/de/cards",
            &[],
            StatusCode::MOVED_PERMANENTLY,
            Some("/de/#cards"),
        ),
        (
            "/en/faq/",
            &[],
            StatusCode::MOVED_PERMANENTLY,
            Some("/en/faq"),
        ),
        ("/en/no-such-page", &[], StatusCode::NOT_FOUND, None),
        (
            "/wp-login.php",
            &[("accept-language", "de")],
            StatusCode::NOT_FOUND,
            None,
        ),
    ];
    for (path, headers, status, location) in cases {
        let (got, response_headers, _) = get(&app, path, headers).await;
        assert_eq!(got, status, "{path}");
        if let Some(location) = location {
            let to = response_headers["location"].to_str().unwrap_or("");
            assert!(to == location || to.ends_with(location), "{path} → {to}");
        }
    }
    let (_, headers, html) = get(&app, "/de/nope", &[]).await;
    assert!(html.contains("<html lang=\"de\""));
    assert_eq!(headers["x-robots-tag"], "noindex, noarchive, nosnippet");
    // The unprefixed card address is chosen by the visitor's language: no
    // cache may keep one visitor's answer for another, nor for good.
    let (status, headers, _) = get(&app, "/cards", &[("accept-language", "en")]).await;
    assert_eq!(status, StatusCode::FOUND);
    assert_eq!(headers["location"], "/en/#cards");
    assert_eq!(headers["vary"], "Accept-Language");
    assert_eq!(headers["cache-control"], "no-cache");
    // Under a language it is fixed, and stays permanent.
    let (status, headers, _) = get(&app, "/de/cards", &[("accept-language", "en")]).await;
    assert_eq!(status, StatusCode::MOVED_PERMANENTLY);
    assert!(!headers.contains_key("vary"));
    // An address under no language is not found in the language the
    // browser asks for, and the answer says that it depends on it; under
    // a language it depends on nothing.
    for (wanted, lang) in [("de", "de"), ("en", "en"), ("fr", "fr")] {
        let (status, headers, html) =
            get(&app, "/wp-login.php", &[("accept-language", wanted)]).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(html.contains(&format!("<html lang=\"{lang}\"")), "{wanted}");
        assert_eq!(headers["vary"], "Accept-Language");
        secured(&headers);
    }
    let (status, headers, _) = get(&app, "/en/no-such-page", &[("accept-language", "de")]).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(!headers.contains_key("vary"));
}

#[tokio::test]
async fn generated_documents() {
    let app = app(true);
    let (status, headers, sitemap) = get(&app, "/sitemap.xml", &[]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers["content-type"], "application/xml; charset=utf-8");
    assert!(
        sitemap.contains("<lastmod>2026-11-08</lastmod>"),
        "the release's date"
    );
    let (_, _, robots) = get(&app, "/robots.txt", &[]).await;
    assert!(robots.contains("Sitemap: https://baylee.app/sitemap.xml"));
    let (_, headers, manifest) = get(&app, "/de/manifest.webmanifest", &[]).await;
    assert_eq!(headers["content-type"], "application/manifest+json");
    let manifest: serde_json::Value = serde_json::from_str(&manifest).expect("json");
    assert_eq!(manifest["id"], "/");
    assert_eq!(manifest["start_url"], "/de/account");
    let (status, headers, _) = get(&app, "/sw.js", &[]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers["cache-control"], "no-cache");
}

#[tokio::test]
async fn only_get_and_head() {
    let app = app(false);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/en/")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    let response = app
        .oneshot(
            Request::builder()
                .method("HEAD")
                .uri("/en/")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn forms_and_the_account_area_are_islands_and_not_indexed() {
    let app = app(false);
    for path in [
        "/en/signin",
        "/de/register",
        "/en/reset",
        "/de/reset",
        "/en/account",
        "/de/account/profile",
        "/en/account/security",
        "/en/account/privacy",
        "/de/account/gateways",
        "/en/account/admin",
    ] {
        let (status, headers, html) = get(&app, path, &[]).await;
        assert_eq!(status, StatusCode::OK, "{path}");
        assert_eq!(count(&html, "<h1"), 1, "{path}: one h1");
        assert!(html.contains("<leptos-island"), "{path}: an island");
        // the administration's page loads its own module (below)
        let script = if path.contains("/account/admin") {
            "admin-islands.js"
        } else {
            "islands.js"
        };
        assert!(
            html.contains(&format!("<script type=\"module\" src=\"/assets/{script}\"")),
            "{path}: the islands' script"
        );
        assert!(
            html.contains("<meta name=\"robots\" content=\"noindex"),
            "{path}: not indexed"
        );
        assert_eq!(headers["x-robots-tag"], "noindex, noarchive, nosnippet");
        assert!(!html.contains(" style=\""), "{path}: no inline style");
        assert!(
            html.contains("/assets/crypto-worker.js"),
            "{path}: the crypto Worker's address"
        );
    }
}

/// The administration's islands are a module of their own
/// (`docs/proposals/frontend-rewrite.md` §27): every page under
/// `/{lang}/account/admin` (the feedback's among them) has the
/// administration's island and loads the administration's module and no
/// other; every other page with islands (the forms, the account area, the
/// messenger, a page that opens the sign-in over itself) loads the public
/// module, which has no administration's island to draw, and names neither.
#[tokio::test]
async fn only_the_administration_s_pages_load_its_module() {
    const ID: &str = "019a2b3c4d5e708090a0b0c0d0e0f0ff";
    const PUBLIC: &str = "<script type=\"module\" src=\"/assets/islands.js\"";
    const ADMIN: &str = "<script type=\"module\" src=\"/assets/admin-islands.js\"";
    let app = app(true);
    for lang in Lang::ALL {
        let code = lang.code();
        let mut admin: Vec<String> = Section::ROUTED
            .into_iter()
            .filter(|section| section.administration())
            .map(|section| Page::Account(section).path(lang))
            .collect();
        admin.push(format!("/{code}/account/admin/users/{ID}"));
        admin.push(format!("/{code}/account/admin/feedback/{ID}"));
        for path in &admin {
            let (status, _, html) = get(&app, path, &[]).await;
            assert_eq!(status, StatusCode::OK, "{path}");
            assert_eq!(
                count(&html, ADMIN),
                1,
                "{path}: the administration's module"
            );
            assert!(!html.contains(PUBLIC), "{path}: the public module");
            assert_eq!(count(&html, "<leptos-island"), 1, "{path}: one island");
            assert!(
                html.contains("data-component=\"Administration_"),
                "{path}: the administration's island"
            );
        }
        let public = [
            Page::Home.path(lang),
            Page::Contact.path(lang),
            Page::Download.path(lang),
            Page::SignIn.path(lang),
            Page::Register.path(lang),
            Page::Reset.path(lang),
        ]
        .into_iter()
        .chain(
            Section::ROUTED
                .into_iter()
                .filter(|section| !section.administration())
                .map(|section| Page::Account(section).path(lang)),
        );
        for path in public {
            let (status, _, html) = get(&app, &path, &[]).await;
            assert_eq!(status, StatusCode::OK, "{path}");
            assert_eq!(count(&html, PUBLIC), 1, "{path}: the public module");
            assert!(
                !html.contains("admin-islands"),
                "{path}: the administration's module"
            );
            assert!(
                !html.contains("data-component=\"Administration_"),
                "{path}: the administration's island"
            );
        }
    }
    // a page of the administration is one exactly when it is under its address
    for section in Section::ROUTED {
        assert_eq!(
            section.administration(),
            section.slug().starts_with("account/admin"),
            "{section:?}"
        );
    }
}

/// The administration's page that makes a reset code
/// (`docs/account-reset.md`): an address under a language, never Caddy's
/// `/admin…` of the old build; a page of the account area, not indexed and
/// in no sitemap. The server cannot know the role, so no page it renders
/// links to it: the navigation is the same for everyone, and only the
/// island leads an administrator there.
#[tokio::test]
async fn the_administration_s_page_is_linked_from_nowhere() {
    let app = app(true);
    for lang in Lang::ALL {
        let path = format!("/{}/account/admin", lang.code());
        let (status, headers, html) = get(&app, &path, &[]).await;
        assert_eq!(status, StatusCode::OK, "{lang:?}");
        assert_eq!(headers["x-robots-tag"], "noindex, noarchive, nosnippet");
        assert!(
            html.contains("<leptos-island"),
            "{lang:?}: the account island"
        );
        assert!(!html.contains(" style=\""), "{lang:?}: no inline style");
        // the island renders "loading" on the server: nothing of the tool
        assert!(!html.contains("id=\"admin-search\""), "{lang:?}");
    }
    let (_, _, html) = get(&app, "/en/account/admin", &[]).await;
    assert!(html.contains("<h1>Reset a login</h1>"));
    assert!(html.contains("Reset a login – Baylee"));
    // its address is the account area's: Caddy's `/admin*` is the old build's
    assert_eq!(
        Page::Account(Section::Admin).path(Lang::En),
        "/en/account/admin"
    );
    assert!(!Page::Account(Section::Admin).indexed());
    assert!(!Section::ALL.contains(&Section::Admin));
    for page in Section::ALL.map(Page::Account) {
        let (_, _, html) = get(&app, &page.path(Lang::De), &[]).await;
        assert!(!html.contains("/account/admin"), "{page:?}: a link to it");
    }
    let (_, _, sitemap) = get(&app, "/sitemap.xml", &[]).await;
    assert!(!sitemap.contains("/account"));
    // as every page of the area, the address without a language is moved
    let (status, headers, _) = get(&app, "/account/admin", &[("accept-language", "de")]).await;
    assert_eq!(status, StatusCode::FOUND);
    assert_eq!(headers["location"], "/de/account/admin");
}

/// The administration's other pages (the overview, the accounts, one
/// account, the gateways, the audit log, the legacy store): under the
/// account area in every language, islands, not indexed, in no sitemap,
/// and their own navigation is the island's, for an administrator: the
/// server renders no link to any of them. One account's page answers its
/// id as 32 lowercase hex digits only, and is never kept, so that requests
/// for ids cannot fill the table of rendered pages.
#[tokio::test]
async fn the_administration_s_other_pages_are_islands_linked_from_nowhere() {
    const ID: &str = "019a2b3c4d5e708090a0b0c0d0e0f0ff";
    let mut config = Config::development(site(), Some(data()));
    config.origin = "https://baylee.app".into();
    let state = central_web_server::app(config).expect("app");
    let app = router(Arc::clone(&state));
    let user = format!("users/{ID}");
    let titles = [
        ("overview", "Administration", "Administration"),
        ("users", "Accounts", "Accounts"),
        (user.as_str(), "An account", "Ein Account"),
        ("gateways", "Gateways", "Gateways"),
        ("audit", "Audit log", "Protokoll"),
        ("legacy", "Legacy accounts", "Alte Konten"),
    ];
    for (tail, english, german) in titles {
        for lang in Lang::ALL {
            let path = format!("/{}/account/admin/{tail}", lang.code());
            let (status, headers, html) = get(&app, &path, &[]).await;
            assert_eq!(status, StatusCode::OK, "{path}");
            assert_eq!(headers["x-robots-tag"], "noindex, noarchive, nosnippet");
            assert!(
                html.contains("<meta name=\"robots\" content=\"noindex"),
                "{path}"
            );
            assert_eq!(count(&html, "<h1"), 1, "{path}: one h1");
            assert!(
                html.contains("<leptos-island"),
                "{path}: the account island"
            );
            assert!(!html.contains(" style=\""), "{path}: no inline style");
            // the island renders "loading" on the server: no navigation of
            // the administration, no link to another of its pages (the
            // language menu names the page itself)
            assert!(!html.contains("admin-nav"), "{path}");
            for other in [
                "overview", "users", "gateways", "audit", "legacy", "feedback",
            ] {
                if other == tail {
                    continue;
                }
                let link = format!("href=\"/{}/account/admin/{other}\"", lang.code());
                assert!(!html.contains(&link), "{path}: {link}");
            }
            assert!(!html.contains(&format!("href=\"/{}/account/admin\"", lang.code())));
            // the language menu leads to the same page, the same account
            let menu = format!("href=\"/de/account/admin/{tail}\"");
            assert!(html.contains(&menu), "{path}: {menu}");
        }
        let (_, _, html) = get(&app, &format!("/en/account/admin/{tail}"), &[]).await;
        assert!(html.contains(&format!("<h1>{english}</h1>")), "{tail}");
        assert!(html.contains(&format!("{english} – Baylee")), "{tail}");
        let (_, _, html) = get(&app, &format!("/de/account/admin/{tail}"), &[]).await;
        assert!(html.contains(&format!("<h1>{german}</h1>")), "{tail}");
    }
    let id = central_web::data::id_from_hex(ID).expect("an id");
    let page = Page::Account(Section::AdminUser(id));
    assert_eq!(page.path(Lang::Fr), format!("/fr/account/admin/users/{ID}"));
    assert_eq!(
        Page::from_slug(&format!("account/admin/users/{ID}")),
        Some(page)
    );
    assert!(!page.indexed());
    assert!(!page.kept());
    for section in Section::ROUTED.into_iter().filter(|s| s.admin()) {
        assert!(!Page::Account(section).indexed(), "{section:?}");
        assert!(Page::Account(section).kept(), "{section:?}");
    }
    // one account's page has one address
    for bad in [
        ID.to_uppercase(),
        ID[..31].to_owned(),
        format!("{ID}0"),
        format!("{ID}/x"),
        format!("{}g", &ID[..31]),
        format!("{ID}x"),
    ] {
        let path = format!("/en/account/admin/users/{bad}");
        let (status, _, _) = get(&app, &path, &[]).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
    }
    let (status, headers, _) = get(&app, &format!("/en/account/admin/users/{ID}/"), &[]).await;
    assert_eq!(status, StatusCode::MOVED_PERMANENTLY);
    assert_eq!(headers["location"], format!("/en/account/admin/users/{ID}"));
    // rendered for each request, never kept: other pages are
    let kept = state.pages.kept();
    for n in 0..20_u8 {
        let path = format!("/en/account/admin/users/{ID:.30}{n:02x}");
        let (status, _, html) = get(&app, &path, &[]).await;
        assert_eq!(status, StatusCode::OK, "{path}");
        assert!(html.contains(&format!("href=\"/de/account/admin/users/{ID:.30}{n:02x}\"")));
    }
    assert_eq!(state.pages.kept(), kept);
    // no page of the area links to the administration's pages
    for page in Section::ALL.map(Page::Account) {
        let (_, _, html) = get(&app, &page.path(Lang::En), &[]).await;
        assert!(!html.contains("/account/admin"), "{page:?}: a link to it");
    }
    let (_, _, sitemap) = get(&app, "/sitemap.xml", &[]).await;
    assert!(!sitemap.contains("/admin"));
    let (status, headers, _) = get(
        &app,
        "/account/admin/gateways",
        &[("accept-language", "de")],
    )
    .await;
    assert_eq!(status, StatusCode::FOUND);
    assert_eq!(headers["location"], "/de/account/admin/gateways");
}
/// The feedback moderation: the list and one report's page, under the
/// account area as the reset code's page is, not indexed, in no sitemap
/// and linked from no page the server renders. Every report's address is
/// one page: the id is the island's to read, so the server neither keeps
/// nor varies a page by it, and anything but 32 lowercase hex digits is no
/// report's address.
#[tokio::test]
async fn the_feedback_moderation_is_two_pages_linked_from_nowhere() {
    let app = app(true);
    let id = "0192d0c57b3e7a1f8c2d3e4f5a6b7c8d";
    for lang in Lang::ALL {
        for path in [
            format!("/{}/account/admin/feedback", lang.code()),
            format!("/{}/account/admin/feedback/{id}", lang.code()),
        ] {
            let (status, headers, html) = get(&app, &path, &[]).await;
            assert_eq!(status, StatusCode::OK, "{path}");
            assert_eq!(headers["x-robots-tag"], "noindex, noarchive, nosnippet");
            assert_eq!(count(&html, "<h1"), 1, "{path}: one h1");
            assert!(
                html.contains("<leptos-island"),
                "{path}: the account island"
            );
            assert!(!html.contains(" style=\""), "{path}: no inline style");
            // the island renders "loading" on the server: nothing of the tool
            assert!(!html.contains("id=\"fb-search\""), "{path}");
            assert!(!html.contains(id), "{path}: the id is not in the page");
        }
    }
    let (_, _, list) = get(&app, "/en/account/admin/feedback", &[]).await;
    assert!(list.contains("<h1>Feedback and crash reports</h1>"));
    let (_, _, one) = get(&app, &format!("/de/account/admin/feedback/{id}"), &[]).await;
    assert!(one.contains("<h1>Bericht</h1>"));
    assert_eq!(
        Page::from_slug(&format!("account/admin/feedback/{id}")),
        Some(Page::Account(Section::Report))
    );
    assert_eq!(
        Page::Account(Section::Feedback).path(Lang::En),
        "/en/account/admin/feedback"
    );
    assert!(!Page::Account(Section::Feedback).indexed());
    assert!(!Page::Account(Section::Report).indexed());
    assert!(!Section::ALL.contains(&Section::Feedback));
    assert!(!Section::ALL.contains(&Section::Report));
    for bad in [
        format!("/en/account/admin/feedback/{}", id.to_uppercase()),
        format!("/en/account/admin/feedback/{}", &id[1..]),
        format!("/en/account/admin/feedback/{id}0"),
        format!("/en/account/admin/feedback/{id}/x"),
        "/en/account/admin/feedback/%3Cscript%3E".to_owned(),
    ] {
        let (status, _, _) = get(&app, &bad, &[]).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{bad}");
    }
    for page in Section::ALL.map(Page::Account) {
        let (_, _, html) = get(&app, &page.path(Lang::De), &[]).await;
        assert!(
            !html.contains("/account/admin/feedback"),
            "{page:?}: a link to it"
        );
    }
    let (_, _, sitemap) = get(&app, "/sitemap.xml", &[]).await;
    assert!(!sitemap.contains("feedback"));
    // as every page of the area, the address without a language is moved
    let (status, headers, _) = get(
        &app,
        "/account/admin/feedback?status=new",
        &[("accept-language", "de")],
    )
    .await;
    assert_eq!(status, StatusCode::FOUND);
    assert_eq!(headers["location"], "/de/account/admin/feedback?status=new");
}

/// The reset page (`docs/account-reset.md`): a form page as signing in is,
/// complete in the first response with every field labelled and the
/// warning before a new vault; reached by a plain link from the sign-in
/// form (on its page and in its dialog) and from the FAQ; in no sitemap.
/// Without script it says that it needs script, of the recovery code too.
#[tokio::test]
async fn the_reset_page_is_linked_from_signing_in_and_listed_nowhere() {
    let app = app(true);
    let (status, _, html) = get(&app, "/en/reset", &[]).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("Set a new password – Baylee"));
    for id in [
        "reset-mail-name",
        "reset-name",
        "reset-code",
        "reset-keep",
        "reset-new",
        "reset-recovery",
        "reset-confirm",
        "reset-password",
        "reset-repeat",
    ] {
        assert_eq!(count(&html, &format!(" id=\"{id}\"")), 1, "{id}");
        assert_eq!(count(&html, &format!(" for=\"{id}\"")), 1, "{id}: a label");
    }
    // the data is kept unless the person says otherwise, and the choice
    // that loses it is described by the warning
    let radios: Vec<&str> = tags(&html, "input")
        .into_iter()
        .filter(|tag| tag.contains("type=\"radio\""))
        .collect();
    assert_eq!(radios.len(), 2);
    assert!(radios[0].contains("id=\"reset-keep\"") && radios[0].contains(" checked"));
    assert!(
        radios[1].contains("id=\"reset-new\"")
            && radios[1].contains("aria-describedby=\"reset-warning\"")
            && !radios[1].contains(" checked")
    );
    assert_eq!(count(&html, " id=\"reset-warning\""), 1);
    assert!(html.contains("your encrypted data is lost for good"));
    // the block for someone without a code stands before the form that
    // redeems one, in the first response, and nothing typed has a name
    let block = html
        .find("aria-labelledby=\"reset-mail-title\"")
        .expect("the block");
    let steps = html.find("id=\"reset-steps\"").expect("the form");
    assert!(block < steps);
    assert_eq!(count(&html, " id=\"reset-mail-title\""), 1);
    assert!(html.contains("Send a code by e-mail"));
    let named: Vec<&str> = tags(&html[block..], "input")
        .into_iter()
        .filter(|tag| tag.contains(" name=") && !tag.contains("type=\"radio\""))
        .collect();
    assert_eq!(named, Vec::<&str>::new());
    let noscript = html.split("<noscript>").nth(1).expect("a noscript");
    assert!(noscript.starts_with("<p class=\"notice\">Setting a new password needs JavaScript"));
    // a form page of its own has no dialogs over it
    assert!(!html.contains("id=\"auth-signin\""));

    let (status, headers, _) = get(&app, "/reset", &[("accept-language", "de")]).await;
    assert_eq!(status, StatusCode::FOUND);
    assert_eq!(headers["location"], "/de/reset");

    // the sign-in form's link, on its page and in the dialog over a page
    let (_, _, signin) = get(&app, "/de/signin", &[]).await;
    assert_eq!(count(&signin, "href=\"/de/reset\""), 1);
    let (_, _, home) = get(&app, "/de/", &[]).await;
    assert_eq!(count(&home, "href=\"/de/reset\""), 1, "in the dialog");
    // the FAQ's answer, beside the dialog's
    let (_, _, faq) = get(&app, "/en/faq", &[]).await;
    assert_eq!(count(&faq, "href=\"/en/reset\""), 2);
    assert!(!faq.contains("cannot be redeemed"));

    let (_, _, sitemap) = get(&app, "/sitemap.xml", &[]).await;
    assert!(!sitemap.contains("/reset"));
    for lang in Lang::ALL {
        let (status, _, html) = get(&app, &format!("/{}/reset", lang.code()), &[]).await;
        assert_eq!(status, StatusCode::OK, "{lang:?}");
        assert_eq!(count(&html, " id=\"reset-warning\""), 1, "{lang:?}");
    }
}

/// The privacy notice tells of the optional e-mail address in a part of
/// its own, after the account's, in the contents too; the account's part
/// still says that no account needs one.
#[tokio::test]
async fn the_privacy_notice_has_a_part_for_the_e_mail_address() {
    let app = app(true);
    for lang in Lang::ALL {
        let (status, _, html) = get(&app, &format!("/{}/privacy", lang.code()), &[]).await;
        assert_eq!(status, StatusCode::OK, "{lang:?}");
        assert_eq!(count(&html, "<h2 id=\"email\">"), 1, "{lang:?}");
        assert_eq!(count(&html, "href=\"#email\""), 1, "{lang:?}");
        let at = |id: &str| html.find(&format!("<h2 id=\"{id}\">")).expect("a part");
        assert!(
            at("account") < at("email") && at("email") < at("cookie"),
            "{lang:?}"
        );
        // the processor by its names, in every language
        assert!(
            html.contains("mailbox.org") && html.contains("Heinlein Hosting GmbH"),
            "{lang:?}"
        );
    }
    let (_, _, english) = get(&app, "/en/privacy", &[]).await;
    assert!(english.contains("An e-mail address is optional; an account needs none."));
    assert!(english.contains("you need no e-mail address"));
    assert!(english.contains("As of 12 November 2026"));
    let (_, _, german) = get(&app, "/de/privacy", &[]).await;
    assert!(german.contains("Eine E-Mail-Adresse ist freiwillig; ein Account braucht keine."));
    assert!(german.contains("eine E-Mail-Adresse brauchst du nicht"));
}

/// Central's figures (the accounts, and what a gateway says is happening
/// on it): in the strip and on the gateway's card while a source reports
/// them, and nowhere when nobody does. A number nobody reported is never
/// drawn as a zero.
#[tokio::test]
async fn the_figures_are_shown_only_while_a_source_reports_them() {
    let reported = app(true);
    let (_, _, home) = get(&reported, "/de/", &[]).await;
    for number in [
        "1.234 Konten",
        "12 Spieler",
        "8 im Spiel",
        "3 offene Tische",
        "4 Partien",
    ] {
        assert_eq!(
            count(&home, number),
            2 - usize::from(number.contains("Konten")),
            "{number}"
        );
    }
    let (_, _, gateways) = get(&reported, "/en/gateways", &[]).await;
    // the online gateway that reports, not the offline one beside it
    assert_eq!(count(&gateways, "class=\"gw-now\""), 1);
    for number in ["12 players", "8 in a game", "3 open tables", "4 games"] {
        assert_eq!(count(&gateways, number), 1, "{number}");
    }

    let silent = app(false);
    for path in ["/de/", "/en/", "/en/gateways"] {
        let (_, _, html) = get(&silent, path, &[]).await;
        for never in [
            "gw-now",
            "Konten",
            " accounts<",
            "Spieler<",
            " players<",
            " im Spiel<",
            " in a game<",
            "0 Partien",
            "0 games",
        ] {
            assert!(!html.contains(never), "{path}: {never}");
        }
    }
}

/// The strip under the bar, for every set of silent sources: nothing of
/// GitHub's, the figures the most telling first, one cell for each figure a
/// source reports and none for one it does not, and the number of cells
/// written where the stylesheet reads it (`data-count`, by which it lays
/// them out in even lines).
#[tokio::test]
async fn the_strip_has_a_cell_for_each_reported_figure_and_nothing_of_github() {
    for silent in 0_u8..16 {
        let mut config = Config::development(site(), Some(data()));
        config.origin = "https://baylee.app".into();
        let state = central_web_server::app(config).expect("app");
        let app = router(Arc::clone(&state));
        let live = silent & 1 == 0;
        let accounts = silent & 2 == 0;
        let cards = silent & 4 == 0;
        let views = silent & 8 == 0;
        install(&state.data, |snapshot| {
            if !live {
                for gateway in snapshot.gateways.iter_mut().flatten() {
                    gateway.activity = None;
                }
            }
            if !accounts {
                snapshot.accounts = None;
            }
            if !cards {
                snapshot.cards = None;
            }
            if !views {
                snapshot.views = None;
            }
        });
        let (_, _, home) = get(&app, "/en/", &[]).await;
        let mark = home.find("class=\"strip\"").expect("the strip");
        let start = home[..mark].rfind("<section").expect("its section");
        let strip = &home[start..start + home[start..].find("</section>").expect("its end")];
        let cells = 1
            + 4 * usize::from(live)
            + usize::from(accounts)
            + usize::from(cards)
            + usize::from(views);
        assert_eq!(count(strip, "class=\"strip-cell\""), cells, "{silent:04b}");
        assert!(
            strip.contains(&format!("data-count=\"{cells}\"")),
            "{silent:04b}"
        );
        assert!(strip.contains("aria-label=\"Baylee in numbers\""));
        // the most telling first: what is played, the gateways, the
        // accounts, the cards, the views
        let order: Vec<usize> = [
            (live, "12 players"),
            (live, "4 games"),
            (true, "1 online"),
            (accounts, "1,234 accounts"),
            (cards, "8,348"),
            (views, "48,213 views"),
        ]
        .iter()
        .map(|(shown, words)| {
            assert_eq!(strip.contains(words), *shown, "{silent:04b}: {words}");
            strip.find(words).unwrap_or(0)
        })
        .filter(|at| *at > 0)
        .collect();
        assert!(order.is_sorted(), "{silent:04b}: {order:?}");
        for never in [
            "github.com",
            "<code",
            "v0.2.0",
            "passing",
            "Build",
            "Commit",
            "Stars",
            "GitHub",
        ] {
            assert!(!strip.contains(never), "{silent:04b}: {never}");
        }
    }
}

/// The count of page views: shown in the strip and told in the privacy
/// notice only where a deployment counts (here the fixture stands in for
/// one), and nowhere else.
#[tokio::test]
async fn page_views_are_shown_and_told_only_where_they_are_counted() {
    let counted = app(true);
    let (_, _, home) = get(&counted, "/de/", &[]).await;
    assert_eq!(count(&home, "48.213 Aufrufe"), 1);
    assert!(home.contains("Seitenaufrufe seit 1. November 2026"));
    let (_, _, english) = get(&counted, "/en/", &[]).await;
    assert_eq!(count(&english, "48,213 views"), 1);
    let (_, _, privacy) = get(&counted, "/de/privacy", &[]).await;
    assert_eq!(count(&privacy, "id=\"views\""), 1);
    assert_eq!(count(&privacy, "href=\"#views\""), 1);
    assert!(privacy.contains("keine IP-Adresse, kein Cookie, keine Kennung"));

    let silent = app(false);
    for path in ["/de/", "/en/", "/de/privacy", "/en/privacy"] {
        let (_, _, html) = get(&silent, path, &[]).await;
        for never in ["Aufruf", " views<", "id=\"views\"", "#views"] {
            assert!(!html.contains(never), "{path}: {never}");
        }
    }
}

/// Counting itself: a browser opening a page that exists adds one; a
/// script's fetch, a page fetched ahead, a missing page and a file do not.
/// The page shows the number as it stood when the data was last installed,
/// never per request: its bytes and its entity tag stay the same between
/// two visits.
#[tokio::test]
async fn a_visit_counts_once_and_never_changes_the_page_it_gets() {
    let folder = site();
    let file = folder.join("views.json");
    std::fs::write(&file, "{\"views\":41,\"since\":\"2026-11-01\"}").expect("the count");
    let mut config = Config::development(site(), None);
    config.origin = "https://baylee.app".into();
    config.views = Some(file.clone());
    let state = central_web_server::app(config).expect("app");
    let app = router(Arc::clone(&state));
    let views = state.views.clone().expect("counting");
    assert_eq!(views.now().count, 41);

    let visit = [
        ("sec-fetch-mode", "navigate"),
        ("sec-fetch-dest", "document"),
    ];
    let (status, first_headers, first) = get(&app, "/en/", &visit).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(views.now().count, 42);
    assert_eq!(count(&first, "41 views"), 1);
    // the second visit counts, and gets the very same page
    let (_, again_headers, again) = get(&app, "/en/", &visit).await;
    assert_eq!(views.now().count, 43);
    assert!(first == again, "a visit changed the page");
    assert_eq!(first_headers.get("etag"), again_headers.get("etag"));
    // a revalidation that finds the page unchanged is a visit too
    let tag = first_headers
        .get("etag")
        .and_then(|tag| tag.to_str().ok())
        .expect("an entity tag")
        .to_owned();
    let mut revisit = visit.to_vec();
    revisit.push(("if-none-match", &tag));
    let (status, _, _) = get(&app, "/en/", &revisit).await;
    assert_eq!(status, StatusCode::NOT_MODIFIED);
    assert_eq!(views.now().count, 44);

    // not a person opening a page
    get(&app, "/en/", &[]).await;
    get(
        &app,
        "/en/faq",
        &[("sec-fetch-mode", "cors"), ("sec-fetch-dest", "empty")],
    )
    .await;
    let mut ahead = visit.to_vec();
    ahead.push(("sec-purpose", "prefetch"));
    get(&app, "/en/faq", &ahead).await;
    let (status, _, _) = get(&app, "/en/nope", &visit).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    get(&app, "/robots.txt", &visit).await;
    get(&app, "/sitemap.xml", &visit).await;
    assert_eq!(views.now().count, 44);

    // installed, the pages show it; saved, the file holds it and no more
    install(&state.data, |snapshot| snapshot.views = Some(views.now()));
    let (_, _, later) = get(&app, "/en/", &[]).await;
    assert_eq!(count(&later, "44 views"), 1);
    views.save();
    assert_eq!(
        std::fs::read_to_string(&file).expect("the file").trim(),
        "{\"views\":44,\"since\":\"2026-11-01\"}"
    );
}

/// A views file that is not the count's stops the server at start.
#[test]
fn a_views_file_that_cannot_be_used_stops_the_start() {
    let folder = site();
    let file = folder.join("views.json");
    std::fs::write(&file, "not the count").expect("written");
    let mut config = Config::development(site(), None);
    config.views = Some(file);
    let Err(broken) = central_web_server::app(config) else {
        panic!("a malformed views file was accepted");
    };
    assert_eq!(broken.0, "the views file is malformed");
}

/// The Impressum as the server writes it holds none of the operator's
/// details and no way to them but the puzzle, and the bait for robots that
/// the rewrite once lost: one link no person sees, hears or reaches, on this
/// page only, which nothing on the site may follow or load ahead.
#[tokio::test]
async fn the_impressum_holds_no_detail_and_keeps_its_bait() {
    const BAIT: &str = "href=\"/site/impressum/kontakt\"";
    let app = app(false);
    for lang in Lang::ALL {
        let path = format!("/{}/impressum", lang.code());
        let (status, headers, html) = get(&app, &path, &[]).await;
        assert_eq!(status, StatusCode::OK, "{path}");
        assert_eq!(headers["x-robots-tag"], "noindex, noarchive, nosnippet");
        assert_eq!(count(&html, BAIT), 1, "{path}: the bait, once");
        let link = tags(&html, "a")
            .into_iter()
            .find(|tag| tag.contains(BAIT))
            .expect("the bait is a link");
        for attribute in [
            "class=\"bait\"",
            "tabindex=\"-1\"",
            "aria-hidden=\"true\"",
            "rel=\"nofollow\"",
            " hidden",
        ] {
            assert!(link.contains(attribute), "{path}: {attribute} in {link}");
        }
        for never in [
            "mailto:",
            "tel:",
            "rel=\"prefetch\"",
            "rel=\"prerender\"",
            "speculationrules",
        ] {
            assert!(!html.contains(never), "{path}: {never}");
        }
        for other in ["", "contact", "privacy", "faq"] {
            let path = format!("/{}/{other}", lang.code());
            let (_, _, html) = get(&app, &path, &[]).await;
            assert!(!html.contains("/site/impressum/kontakt"), "{path}");
        }
    }
    let (_, _, robots) = get(&app, "/robots.txt", &[]).await;
    assert!(robots.contains("Disallow: /site/\n"));
}

/// The security headers every answer carries, a 304 among them.
fn secured(headers: &axum::http::HeaderMap) {
    for name in [
        "content-security-policy",
        "permissions-policy",
        "cross-origin-opener-policy",
        "cross-origin-resource-policy",
        "referrer-policy",
        "x-content-type-options",
        "x-frame-options",
        "strict-transport-security",
    ] {
        assert!(headers.contains_key(name), "{name}");
    }
}

#[tokio::test]
async fn a_page_is_revalidated_by_its_entity_tag() {
    let app = app(true);
    let (status, headers, html) = get(&app, "/en/faq", &[]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers["cache-control"], "no-cache");
    let etag = headers["etag"].to_str().expect("an entity tag").to_owned();
    assert!(
        etag.starts_with('"') && etag.ends_with('"') && etag.len() == 34,
        "{etag}"
    );
    let weak = format!("W/{etag}");
    let listed = format!("\"0000\", {etag}");
    for asked in [etag.as_str(), weak.as_str(), listed.as_str(), "*"] {
        let (status, headers, body) = get(&app, "/en/faq", &[("if-none-match", asked)]).await;
        assert_eq!(status, StatusCode::NOT_MODIFIED, "{asked}");
        assert_eq!(body, "");
        assert_eq!(headers["etag"], etag.as_str());
        assert_eq!(headers["cache-control"], "no-cache");
        assert_eq!(headers.contains_key("x-robots-tag"), !Page::Faq.indexed());
        assert!(!headers.contains_key("content-type"));
        secured(&headers);
    }
    // Another tag, or another language's page, is the page itself.
    let (status, _, again) = get(&app, "/en/faq", &[("if-none-match", "\"0000\"")]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(again, html);
    let (status, headers, _) = get(&app, "/de/faq", &[("if-none-match", etag.as_str())]).await;
    assert_eq!(status, StatusCode::OK);
    assert_ne!(headers["etag"], etag.as_str());
    // A page for another machine has its own tag: a cache's copy for one
    // visitor is never confirmed for another.
    let windows = (
        "user-agent",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/140.0",
    );
    let linux = ("user-agent", "Mozilla/5.0 (X11; Linux x86_64) Chrome/140.0");
    let (_, headers, _) = get(&app, "/en/download", &[windows]).await;
    let tag = headers["etag"].to_str().expect("tag").to_owned();
    let (status, _, _) = get(
        &app,
        "/en/download",
        &[linux, ("if-none-match", tag.as_str())],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, headers, _) = get(
        &app,
        "/en/download",
        &[windows, ("if-none-match", tag.as_str())],
    )
    .await;
    assert_eq!(status, StatusCode::NOT_MODIFIED);
    assert!(
        headers["vary"]
            .to_str()
            .unwrap_or("")
            .contains("User-Agent")
    );
    // Only a 200 is ever confirmed: not the error pages, not a redirect.
    for path in ["/en/no-such-page", "/download"] {
        let (status, _, _) = get(&app, path, &[("if-none-match", "*")]).await;
        assert!(
            status == StatusCode::NOT_FOUND || status == StatusCode::FOUND,
            "{path}"
        );
    }
}

#[tokio::test]
async fn the_documents_the_server_writes_are_revalidated_too() {
    let app = app(true);
    for path in [
        "/sw.js",
        "/favicon.ico",
        "/robots.txt",
        "/sitemap.xml",
        "/.well-known/tdmrep.json",
        "/de/manifest.webmanifest",
    ] {
        let (status, headers, _) = get(&app, path, &[]).await;
        assert_eq!(status, StatusCode::OK, "{path}");
        assert_eq!(headers["cache-control"], "no-cache", "{path}");
        let etag = headers["etag"].to_str().expect("tag").to_owned();
        let (status, headers, body) = get(&app, path, &[("if-none-match", etag.as_str())]).await;
        assert_eq!(status, StatusCode::NOT_MODIFIED, "{path}");
        assert_eq!(body, "");
        assert_eq!(headers["etag"], etag.as_str());
        secured(&headers);
        if path == "/sw.js" {
            assert_eq!(headers["service-worker-allowed"], "/");
        }
    }
}

/// The old React build's administration is gone (12.11.2026): its
/// bookmarks lead to the account area's pages in the visitor's language,
/// by a redirect that is not kept, the feedback's list with its query.
#[tokio::test]
async fn the_old_administrations_addresses_lead_to_its_new_pages() {
    let app = app(true);
    let id = "0123456789abcdef0123456789abcdef";
    for (old, new) in [
        ("/admin", "/de/account/admin/overview".to_owned()),
        ("/admin/", "/de/account/admin/overview".to_owned()),
        ("/admin/users", "/de/account/admin/users".to_owned()),
        (
            &format!("/admin/users/{id}") as &str,
            format!("/de/account/admin/users/{id}"),
        ),
        ("/admin/gateways", "/de/account/admin/gateways".to_owned()),
        ("/admin/audit/", "/de/account/admin/audit".to_owned()),
        ("/admin/legacy", "/de/account/admin/legacy".to_owned()),
        (
            "/admin/feedback?status=new&kind=crash",
            "/de/account/admin/feedback?status=new&kind=crash".to_owned(),
        ),
        ("/admin/reports", "/de/account/admin/feedback".to_owned()),
        (
            &format!("/admin/feedback/{id}") as &str,
            format!("/de/account/admin/feedback/{id}"),
        ),
    ] {
        let (status, headers, _) = get(&app, old, &[("accept-language", "de-DE,de;q=0.9")]).await;
        assert_eq!(status, StatusCode::FOUND, "{old}");
        assert_eq!(headers["location"], new.as_str(), "{old}");
        assert_eq!(headers["cache-control"], "no-cache", "{old}");
        assert_eq!(headers["vary"], "Accept-Language", "{old}");
    }
    // nothing else under it leads into the account area, and nothing that
    // only begins alike (the site's rule for an address without a language
    // may still send it to `/{lang}/…`, where it is no page)
    for path in [
        "/administration",
        "/admin/users/XYZ",
        "/admin/users/0123",
        "/admin/feedback/0123456789ABCDEF0123456789ABCDEF",
        "/admin/settings",
        "/admin/users/0123456789abcdef0123456789abcdef/more",
    ] {
        let (_, headers, _) = get(&app, path, &[]).await;
        let location = headers
            .get("location")
            .and_then(|l| l.to_str().ok())
            .unwrap_or("");
        assert!(!location.contains("/account/admin"), "{path} -> {location}");
    }
}

/// Mail providers fetch the BIMI mark at its fixed address: a 200, as an
/// SVG, its bytes exactly, no redirect to a language, revalidated, without
/// a session and in either mode.
#[tokio::test]
async fn the_bimi_mark_answers_at_its_fixed_address_as_it_is() {
    for fallback in [true, false] {
        let app = app(fallback);
        let (status, headers, body) = get(&app, "/bimi/baylee.svg", &[]).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(headers["content-type"], "image/svg+xml");
        assert_eq!(headers["cache-control"], "no-cache");
        assert!(headers.get("location").is_none());
        assert!(headers.get("set-cookie").is_none());
        assert_eq!(body.as_bytes(), bimi().as_slice());
        secured(&headers);
        // only that address: no language before it, nothing beside it
        for path in [
            "/en/bimi/baylee.svg",
            "/bimi/",
            "/bimi/baylee.svg/",
            "/bimi/other.svg",
        ] {
            let (status, _, body) = get(&app, path, &[]).await;
            assert_ne!(status, StatusCode::OK, "{path}");
            assert_ne!(body.as_bytes(), bimi().as_slice(), "{path}");
        }
    }
}

/// A site folder whose service worker the build also compressed.
fn compressed_site() -> PathBuf {
    let dir = site();
    std::fs::write(dir.join("sw.js"), b"plain").expect("plain");
    std::fs::write(dir.join("sw.js.br"), b"brotli").expect("brotli");
    std::fs::write(dir.join("sw.js.gz"), b"gzip").expect("gzip");
    dir
}

#[tokio::test]
async fn an_encoding_is_sent_only_where_the_browser_takes_it() {
    let config = Config::development(compressed_site(), None);
    let app = router(central_web_server::app(config).expect("app"));
    let mut tags = Vec::new();
    for (accepts, sent) in [
        ("br, gzip", "brotli"),
        ("gzip, br", "brotli"),
        ("br;q=0, gzip", "gzip"),
        ("br;q=0", "plain"),
        ("br;q=0.0, gzip;q=0.000", "plain"),
        ("gzip;q=0.9, br;q=0.4", "gzip"),
        ("*", "brotli"),
        ("*;q=0", "plain"),
        ("gzip, *;q=0", "gzip"),
        ("BR", "brotli"),
        ("brotli", "plain"),
        ("x-gzip", "gzip"),
        ("gzip;q=2", "plain"),
        ("identity", "plain"),
        ("", "plain"),
    ] {
        let (status, headers, body) = get(&app, "/sw.js", &[("accept-encoding", accepts)]).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, sent, "{accepts}");
        let encoding = headers
            .get("content-encoding")
            .map(|v| v.to_str().unwrap_or(""));
        let expected = match sent {
            "brotli" => Some("br"),
            "gzip" => Some("gzip"),
            _ => None,
        };
        assert_eq!(encoding, expected, "{accepts}");
        assert_eq!(headers["vary"], "Accept-Encoding");
        tags.push((sent, headers["etag"].to_str().expect("tag").to_owned()));
    }
    // Each form has its own tag; a tag confirms only the form it names.
    let tag = |form: &str| {
        tags.iter()
            .find(|(sent, _)| *sent == form)
            .map(|(_, tag)| tag.clone())
            .expect("form")
    };
    assert_ne!(tag("brotli"), tag("gzip"));
    assert_ne!(tag("brotli"), tag("plain"));
    let brotli = tag("brotli");
    let (status, _, body) = get(
        &app,
        "/sw.js",
        &[
            ("accept-encoding", "gzip"),
            ("if-none-match", brotli.as_str()),
        ],
    )
    .await;
    assert_eq!((status, body.as_str()), (StatusCode::OK, "gzip"));
    let (status, headers, _) = get(
        &app,
        "/sw.js",
        &[
            ("accept-encoding", "br"),
            ("if-none-match", brotli.as_str()),
        ],
    )
    .await;
    assert_eq!(status, StatusCode::NOT_MODIFIED);
    assert!(!headers.contains_key("content-encoding"));
    assert_eq!(headers["vary"], "Accept-Encoding");
}

#[tokio::test]
async fn only_the_pages_with_a_download_are_kept_per_machine() {
    let mut config = Config::development(site(), Some(data()));
    config.origin = "https://baylee.app".into();
    let state = central_web_server::app(config).expect("app");
    let app = router(Arc::clone(&state));
    let windows = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36";
    let phone = "Mozilla/5.0 (Linux; Android 15; Pixel 9) AppleWebKit/537.36 Mobile";
    // a page that is the same for every machine: one copy, whoever asks
    let (_, headers, first) = get(&app, "/fr/faq", &[("user-agent", windows)]).await;
    let (_, _, second) = get(&app, "/fr/faq", &[("user-agent", phone)]).await;
    assert_eq!(first, second);
    let varies = |headers: &axum::http::HeaderMap| {
        headers
            .get_all("vary")
            .iter()
            .any(|value| value.to_str().unwrap_or("").contains("User-Agent"))
    };
    assert!(!varies(&headers));
    assert_eq!(state.pages.kept(), 1);
    // the download is written for the machine, and says so to caches
    let (_, headers, first) = get(&app, "/fr/download", &[("user-agent", windows)]).await;
    let (_, _, second) = get(&app, "/fr/download", &[("user-agent", phone)]).await;
    assert_ne!(first, second);
    assert!(varies(&headers));
    assert_eq!(state.pages.kept(), 3);
}

#[tokio::test]
async fn a_translated_language_says_what_exists_in_german_and_english_only() {
    let app = app(true);
    // Central's terms: the English text, marked as English, under a note
    let (status, _, terms) = get(&app, "/ja/terms", &[]).await;
    assert_eq!(status, StatusCode::OK);
    assert!(terms.contains("<html lang=\"ja\""));
    assert!(
        terms.contains("<div lang=\"en\">"),
        "the terms are marked as English"
    );
    assert!(
        terms.contains("href=\"/de/terms\""),
        "the German text is one link away"
    );
    let (_, _, german) = get(&app, "/de/terms", &[]).await;
    assert!(german.contains("<div lang=\"de\">") && !german.contains("role=\"note\">Central"));
    // the privacy notice, the legal notice and the Fan Content page are a
    // machine's translation there and say which texts apply
    for page in ["privacy", "impressum", "fan-content"] {
        let (_, _, html) = get(&app, &format!("/zh-hant/{page}"), &[]).await;
        assert!(
            html.contains(&format!("href=\"/de/{page}\""))
                && html.contains(&format!("href=\"/en/{page}\"")),
            "{page}: the German and the English text are named"
        );
        let (_, _, english) = get(&app, &format!("/en/{page}"), &[]).await;
        assert!(!english.contains("class=\"machine-note\""), "{page}");
    }
}

#[tokio::test]
async fn new_data_drops_the_pages_rendered_from_the_old() {
    let mut config = Config::development(site(), Some(data()));
    config.origin = "https://baylee.app".into();
    let state = central_web_server::app(config).expect("app");
    let app = router(Arc::clone(&state));
    let (_, headers, before) = get(&app, "/en/gateways", &[]).await;
    assert!(before.contains("Kitchen"));
    let tag = headers["etag"].to_str().expect("tag").to_owned();
    assert_eq!(state.pages.kept(), 1);
    // The same page again is the kept one.
    let (_, headers, again) = get(&app, "/en/gateways", &[]).await;
    assert_eq!(again, before);
    assert_eq!(headers["etag"], tag.as_str());
    assert_eq!(state.pages.kept(), 1);
    let (_, _, _) = get(&app, "/de/gateways", &[]).await;
    assert_eq!(state.pages.kept(), 2);
    // The same data installed again keeps them.
    install(&state.data, |_| {});
    let (_, _, _) = get(&app, "/en/gateways", &[]).await;
    assert_eq!(state.pages.kept(), 2);
    // New data: the page shows it at once, under another tag, and a cache's
    // copy of the old page is no longer confirmed.
    install(&state.data, |snapshot| {
        if let Some(gateways) = &mut snapshot.gateways {
            gateways.retain(|gateway| gateway.official);
        }
    });
    let (status, headers, after) =
        get(&app, "/en/gateways", &[("if-none-match", tag.as_str())]).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!after.contains("Kitchen"));
    assert!(after.contains("Sanctuary"));
    assert_ne!(headers["etag"], tag.as_str());
    assert_eq!(state.pages.kept(), 1, "the old pages are gone");
}

#[test]
fn a_page_is_a_pure_function_of_what_it_is_rendered_from() {
    // The kept pages rest on this: the same page, language, machine, data
    // and settings render the same bytes, again and on another thread.
    let data = fallback(Some(&data()));
    let assets = Assets::default();
    let machines = [
        machine("", None, None, None),
        machine(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/140.0",
            Some("\"Windows\""),
            Some("\"arm\""),
            Some("\"64\""),
        ),
        machine(
            "Mozilla/5.0 (Linux; Android 15; Pixel 7) Mobile",
            None,
            None,
            None,
        ),
    ];
    let pages: Vec<Page> = Page::LISTED
        .into_iter()
        .chain([
            Page::SignIn,
            Page::Register,
            Page::Reset,
            Page::Offline,
            Page::NotFound,
            Page::Gone,
        ])
        .chain(Section::ROUTED.map(Page::Account))
        .collect();
    for page in pages {
        for lang in Lang::ALL {
            for machine in machines {
                let render_once = || {
                    render(
                        page,
                        &Context {
                            lang,
                            origin: "https://baylee.app",
                            assets: &assets,
                            data: &data,
                            machine,
                            verification: Some("synthetic-value"),
                        },
                    )
                };
                let first = render_once();
                let again = render_once();
                let elsewhere =
                    std::thread::scope(|scope| scope.spawn(render_once).join().expect("thread"));
                assert!(first == again, "{page:?} {lang:?} {machine:?}: again");
                assert!(
                    first == elsewhere,
                    "{page:?} {lang:?} {machine:?}: elsewhere"
                );
            }
        }
    }
}
