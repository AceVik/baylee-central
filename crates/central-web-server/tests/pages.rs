//! Every page in every language, as a browser and a search engine get it:
//! status, headers, one `h1`, canonical address and languages, sharing
//! card, structured data that parses, no inline style or script, pictures
//! with their size, the Fan Content notice, and nothing from another origin.
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use central_web::site::Page;
use central_web::words::Lang;
use central_web_server::config::Config;
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
        std::fs::write(dir.join(name), b"x").expect("root file");
    }
    dir
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

fn check_page(html: &str, lang: Lang, page: Page) {
    let code = lang.code();
    assert!(
        html.starts_with("<!doctype html><html lang=\""),
        "{code} {page:?}: a doctype and html lang"
    );
    assert!(
        html.contains(&format!("<html lang=\"{code}\"")),
        "{page:?}: lang"
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
        let away = anchor.contains(" href=\"https://")
            && !anchor.contains(" href=\"https://baylee.app/");
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
            other.code(),
            page.path(other)
        );
        assert!(
            html.contains(&alternate),
            "{code} {page:?}: hreflang {}",
            other.code()
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
    let notice = match lang {
        Lang::En => "is unofficial Fan Content permitted under the",
        Lang::De => "ist ein inoffizieller Fan-Inhalt im Rahmen der",
    };
    assert!(
        html.contains(notice),
        "{code} {page:?}: the Fan Content notice"
    );
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
    assert!(html.contains("Official"), "the official gateway is marked");
    let official = html.find("Sanctuary").expect("official");
    let other = html.find("Kitchen").expect("other");
    assert!(official < other, "official first");
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
            &[],
            StatusCode::MOVED_PERMANENTLY,
            Some("/en/#cards"),
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
        "/en/account",
        "/de/account/profile",
        "/en/account/security",
        "/en/account/privacy",
        "/de/account/gateways",
    ] {
        let (status, headers, html) = get(&app, path, &[]).await;
        assert_eq!(status, StatusCode::OK, "{path}");
        assert_eq!(count(&html, "<h1"), 1, "{path}: one h1");
        assert!(html.contains("<leptos-island"), "{path}: an island");
        assert!(
            html.contains("<script type=\"module\" src=\"/assets/islands.js\""),
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
