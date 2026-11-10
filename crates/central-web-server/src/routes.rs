//! Addresses: every page lives under its language (`/en/…`, `/de/…`); an
//! address without one (`/`, the old `/download`, the client's `/register`,
//! the app's `/account`) is sent on to the visitor's language. Every
//! response carries the security headers; HTML is never cached unchecked.

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, Uri, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use central_web::data::machine;
use central_web::site::{self, Assets, Context, Page};
use central_web::words::Lang;

use crate::config::Config;
use crate::fetch::Shared;
use crate::files::Files;
use crate::generated;

/// Everything a request needs.
pub struct App {
    /// The settings.
    pub config: Config,
    /// The build's assets.
    pub assets: Assets,
    /// Their bytes.
    pub files: Files,
    /// The public data.
    pub data: Shared,
}

/// The content security policy (`docs/proposals/frontend-rewrite.md` §3):
/// nothing from another origin, no inline script or style, WebAssembly
/// compiled by our own scripts only, Trusted Types for every script URL.
#[must_use]
pub fn csp(https: bool) -> String {
    let mut policy = String::from(
        "default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self'; img-src 'self'; \
         font-src 'self'; connect-src 'self'; worker-src 'self'; manifest-src 'self'; form-action 'none'; \
         base-uri 'none'; frame-ancestors 'none'; object-src 'none'; \
         require-trusted-types-for 'script'; trusted-types baylee baylee-sw",
    );
    if https {
        policy.push_str("; upgrade-insecure-requests");
    }
    policy
}

/// What the browser may use: nothing the site does not need.
pub const PERMISSIONS: &str = "camera=(), microphone=(), geolocation=(), payment=(), usb=(), serial=(), \
     bluetooth=(), hid=(), midi=(), display-capture=(), browsing-topics=(), interest-cohort=()";

async fn security(State(app): State<Arc<App>>, request: Request, next: Next) -> Response {
    let https = app.config.origin.starts_with("https://");
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    let mut set = |name: &'static str, value: &str| {
        if let Ok(value) = HeaderValue::from_str(value) {
            headers.insert(name, value);
        }
    };
    set("content-security-policy", &csp(https));
    set("permissions-policy", PERMISSIONS);
    set("cross-origin-opener-policy", "same-origin");
    set("cross-origin-resource-policy", "same-origin");
    set("referrer-policy", "no-referrer");
    set("x-content-type-options", "nosniff");
    set("x-frame-options", "DENY");
    if https {
        set("strict-transport-security", "max-age=31536000");
    }
    response
}

fn html(status: StatusCode, body: String, headers: &[(&'static str, &str)]) -> Response {
    let mut response = (status, body).into_response();
    let map = response.headers_mut();
    map.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    map.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    for (name, value) in headers {
        if let Ok(value) = HeaderValue::from_str(value) {
            map.insert(*name, value);
        }
    }
    response
}

fn redirect(status: StatusCode, to: &str, vary: bool) -> Response {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = status;
    if let Ok(location) = HeaderValue::from_str(to) {
        response.headers_mut().insert(header::LOCATION, location);
    }
    if vary {
        response
            .headers_mut()
            .insert(header::VARY, HeaderValue::from_static("Accept-Language"));
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    }
    response
}

fn text(kind: &'static str, body: String) -> Response {
    let mut response = body.into_response();
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(kind));
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    response
}

fn header_text<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

fn page(app: &App, headers: &HeaderMap, page: Page, lang: Lang) -> Response {
    let machine = machine(
        header_text(headers, "user-agent").unwrap_or(""),
        header_text(headers, "sec-ch-ua-platform"),
        header_text(headers, "sec-ch-ua-arch"),
        header_text(headers, "sec-ch-ua-bitness"),
    );
    let data = app.data.read().map(|data| data.clone()).unwrap_or_default();
    let cx = Context {
        lang,
        origin: &app.config.origin,
        assets: &app.assets,
        data: &data,
        machine,
        verification: app.config.verification.as_deref(),
    };
    let body = site::render(page, &cx);
    let status = match page {
        Page::NotFound => StatusCode::NOT_FOUND,
        Page::Gone => StatusCode::GONE,
        _ => StatusCode::OK,
    };
    let mut extra: Vec<(&'static str, &str)> = Vec::new();
    if matches!(page, Page::Home | Page::Download) {
        extra.push((
            "accept-ch",
            "Sec-CH-UA-Platform, Sec-CH-UA-Arch, Sec-CH-UA-Bitness",
        ));
        extra.push((
            "vary",
            "User-Agent, Sec-CH-UA-Platform, Sec-CH-UA-Arch, Sec-CH-UA-Bitness",
        ));
    }
    if !page.indexed() {
        extra.push(("x-robots-tag", "noindex, noarchive, nosnippet"));
    }
    html(status, body, &extra)
}

/// Old and unprefixed paths that live on under a language.
const MOVED: [&str; 12] = [
    "download",
    "gateways",
    "contact",
    "impressum",
    "privacy",
    "terms",
    "faq",
    "fan-content",
    "register",
    "signin",
    "account",
    "admin",
];

fn file(app: &App, path: &str, headers: &HeaderMap) -> Option<Response> {
    let file = app.files.get(path)?;
    let accepts = header_text(headers, "accept-encoding").unwrap_or("");
    let (bytes, encoding) = match (&file.brotli, &file.gzip) {
        (Some(brotli), _) if accepts.contains("br") => (brotli.clone(), Some("br")),
        (_, Some(gzip)) if accepts.contains("gzip") => (gzip.clone(), Some("gzip")),
        _ => (file.bytes.clone(), None),
    };
    let mut response = Response::new(Body::from(bytes));
    let map = response.headers_mut();
    map.insert(header::CONTENT_TYPE, HeaderValue::from_static(file.kind));
    map.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(if file.immutable {
            "public, max-age=31536000, immutable"
        } else {
            "no-cache"
        }),
    );
    if file.brotli.is_some() || file.gzip.is_some() {
        map.insert(header::VARY, HeaderValue::from_static("Accept-Encoding"));
    }
    if let Some(encoding) = encoding {
        map.insert(header::CONTENT_ENCODING, HeaderValue::from_static(encoding));
    }
    if path == "/sw.js" {
        map.insert("service-worker-allowed", HeaderValue::from_static("/"));
    }
    Some(response)
}

fn negotiated(headers: &HeaderMap, uri: &Uri) -> Lang {
    let asked = uri.query().and_then(|query| {
        query
            .split('&')
            .find_map(|pair| pair.strip_prefix("lang="))
            .and_then(Lang::from_code)
    });
    asked.unwrap_or_else(|| Lang::negotiate(header_text(headers, "accept-language").unwrap_or("")))
}

async fn any(
    State(app): State<Arc<App>>,
    method: Method,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if method != Method::GET && method != Method::HEAD {
        let mut response = StatusCode::METHOD_NOT_ALLOWED.into_response();
        response
            .headers_mut()
            .insert(header::ALLOW, HeaderValue::from_static("GET, HEAD"));
        return response;
    }
    let path = uri.path();
    if let Some(response) = file(&app, path, &headers) {
        return response;
    }
    match path {
        "/robots.txt" => {
            return text(
                "text/plain; charset=utf-8",
                generated::robots(&app.config.origin),
            );
        }
        "/sitemap.xml" => {
            let data = app.data.read().map(|data| data.clone()).unwrap_or_default();
            return text(
                "application/xml; charset=utf-8",
                generated::sitemap(&app.config.origin, &data),
            );
        }
        "/.well-known/tdmrep.json" => {
            return text("application/json", generated::TDMREP.to_owned());
        }
        "/" => {
            return redirect(
                StatusCode::FOUND,
                &format!("/{}/", negotiated(&headers, &uri).code()),
                true,
            );
        }
        _ => {}
    }
    let trimmed = path.trim_start_matches('/');
    let (first, rest) = trimmed.split_once('/').unwrap_or((trimmed, ""));
    if let Some(lang) = Lang::from_code(first) {
        if rest == "manifest.webmanifest" {
            return text(
                "application/manifest+json",
                generated::manifest(lang, &app.assets),
            );
        }
        if rest.ends_with('/') && !rest.is_empty() {
            return redirect(
                StatusCode::MOVED_PERMANENTLY,
                &format!("/{first}/{}", rest.trim_end_matches('/')),
                false,
            );
        }
        if rest == "cards" || rest.starts_with("cards/") {
            return redirect(
                StatusCode::MOVED_PERMANENTLY,
                &format!("/{first}/#cards"),
                false,
            );
        }
        return match Page::from_slug(rest) {
            Some(found) => page(&app, &headers, found, lang),
            None => page(&app, &headers, Page::NotFound, lang),
        };
    }
    if MOVED.contains(&first) {
        let lang = negotiated(&headers, &uri);
        // The query stays (`/account/chat?with=…`), but for the language.
        let kept: Vec<&str> = uri
            .query()
            .into_iter()
            .flat_map(|query| query.split('&'))
            .filter(|pair| !pair.is_empty() && !pair.starts_with("lang="))
            .collect();
        let query = if kept.is_empty() {
            String::new()
        } else {
            format!("?{}", kept.join("&"))
        };
        return redirect(
            StatusCode::FOUND,
            &format!("/{}/{trimmed}{query}", lang.code()),
            true,
        );
    }
    if first == "cards" {
        let lang = negotiated(&headers, &uri);
        return redirect(
            StatusCode::MOVED_PERMANENTLY,
            &format!("/{}/#cards", lang.code()),
            false,
        );
    }
    let lang = Lang::negotiate(header_text(&headers, "accept-language").unwrap_or(""));
    page(&app, &headers, Page::NotFound, lang)
}

/// The router: one handler for every address, the security headers on
/// every answer, a ten-second deadline.
pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .fallback(any)
        .layer(middleware::from_fn_with_state(Arc::clone(&app), security))
        .layer(tower_http::timeout::TimeoutLayer::with_status_code(
            StatusCode::SERVICE_UNAVAILABLE,
            std::time::Duration::from_secs(10),
        ))
        .with_state(app)
}
