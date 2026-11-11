//! Addresses: every page lives under its language (`/en/…`, `/de/…`); an
//! address without one (`/`, the old `/download`, the client's `/register`,
//! the app's `/account`) is sent on to the visitor's language. Every
//! response carries the security headers; HTML is never cached unchecked:
//! it is revalidated by its entity tag (`If-None-Match`, 304).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::{Body, Bytes};
use axum::extract::{Request, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, Uri, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use central_web::data::{Machine, machine};
use central_web::site::{self, Assets, Context, Page, Snapshot};
use central_web::words::Lang;

use crate::config::Config;
use crate::fetch::Shared;
use crate::files::{Files, Stored, etag};
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
    /// The pages rendered from it.
    pub pages: Pages,
}

/// Most pages kept for one snapshot of the data, and their most bytes
/// together. A page is about 20 KB (the start page about 70 KB) and
/// depends on its language and the visitor's machine: some forty pages in
/// two languages for the dozen or so machines told apart fit well within
/// both. Past either bound a page is rendered for its request and not kept.
const PAGES: usize = 1_024;
const PAGE_BYTES: usize = 32 * 1024 * 1024;

/// A page as rendered: its bytes and their entity tag.
struct Rendered {
    body: Bytes,
    etag: Option<HeaderValue>,
}

/// One page in one language, as rendered for each machine (compared whole).
type ByMachine = Vec<(Machine, Arc<Rendered>)>;

/// The pages rendered from one snapshot of the data. A page is a pure
/// function of itself, its language, the visitor's machine, the data and
/// the settings (origin, assets, verification: fixed for the process), so
/// each is rendered once per snapshot. A new snapshot is a new `Arc`
/// (`fetch.rs`), and finding one drops every page rendered from the old.
#[derive(Default)]
pub struct Pages(Mutex<Kept>);

#[derive(Default)]
struct Kept {
    data: Option<Arc<Snapshot>>,
    pages: HashMap<(Page, Lang), ByMachine>,
    count: usize,
    bytes: usize,
}

impl Kept {
    fn from(&self, data: &Arc<Snapshot>) -> bool {
        self.data
            .as_ref()
            .is_some_and(|kept| Arc::ptr_eq(kept, data))
    }

    fn get(&self, page: Page, lang: Lang, machine: Machine) -> Option<&Arc<Rendered>> {
        self.pages
            .get(&(page, lang))?
            .iter()
            .find(|(kept, _)| *kept == machine)
            .map(|(_, rendered)| rendered)
    }
}

impl Pages {
    /// How many pages are kept now.
    #[must_use]
    pub fn kept(&self) -> usize {
        self.0.lock().map_or(0, |kept| kept.count)
    }

    /// The page rendered for `machine` from `data`, if it is kept. Data
    /// that is not the kept pages' own replaces them, all of them.
    fn find(
        &self,
        (page, lang, machine): (Page, Lang, Machine),
        data: &Arc<Snapshot>,
    ) -> Option<Arc<Rendered>> {
        // A poisoned table renders every page afresh.
        let mut kept = self.0.lock().ok()?;
        if !kept.from(data) {
            *kept = Kept {
                data: Some(Arc::clone(data)),
                ..Kept::default()
            };
            return None;
        }
        kept.get(page, lang, machine).map(Arc::clone)
    }

    /// Keep `rendered` unless the data changed while it was rendered or the
    /// table is full.
    fn keep(
        &self,
        (page, lang, machine): (Page, Lang, Machine),
        data: &Arc<Snapshot>,
        rendered: &Arc<Rendered>,
    ) {
        let Ok(mut kept) = self.0.lock() else {
            return;
        };
        let bytes = kept.bytes.saturating_add(rendered.body.len());
        if !kept.from(data)
            || kept.count >= PAGES
            || bytes > PAGE_BYTES
            || kept.get(page, lang, machine).is_some()
        {
            return;
        }
        kept.bytes = bytes;
        kept.count += 1;
        kept.pages
            .entry((page, lang))
            .or_default()
            .push((machine, Arc::clone(rendered)));
    }
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

fn html(status: StatusCode, page: &Rendered, headers: &[(&'static str, &str)]) -> Response {
    let mut response = (status, page.body.clone()).into_response();
    let map = response.headers_mut();
    map.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    map.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    if let Some(etag) = &page.etag {
        map.insert(header::ETAG, etag.clone());
    }
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
    let tag = etag(body.as_bytes());
    let mut response = body.into_response();
    let map = response.headers_mut();
    map.insert(header::CONTENT_TYPE, HeaderValue::from_static(kind));
    map.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    if let Some(tag) = tag {
        map.insert(header::ETAG, tag);
    }
    response
}

fn header_text<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

/// The data as it is now; replacing it never changes this one.
fn snapshot(app: &App) -> Arc<Snapshot> {
    app.data
        .read()
        .map(|data| Arc::clone(&data))
        .unwrap_or_default()
}

fn page(app: &App, headers: &HeaderMap, page: Page, lang: Lang) -> Response {
    let machine = machine(
        header_text(headers, "user-agent").unwrap_or(""),
        header_text(headers, "sec-ch-ua-platform"),
        header_text(headers, "sec-ch-ua-arch"),
        header_text(headers, "sec-ch-ua-bitness"),
    );
    let data = snapshot(app);
    let key = (page, lang, machine);
    let rendered = app.pages.find(key, &data).unwrap_or_else(|| {
        let cx = Context {
            lang,
            origin: &app.config.origin,
            assets: &app.assets,
            data: &data,
            machine,
            verification: app.config.verification.as_deref(),
        };
        let body = Bytes::from(site::render(page, &cx));
        let rendered = Arc::new(Rendered {
            etag: etag(&body),
            body,
        });
        app.pages.keep(key, &data, &rendered);
        rendered
    });
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
    html(status, &rendered, &extra)
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

/// A qvalue (RFC 9110 §12.4.2) in thousandths: `0` to `0.999` or `1` to
/// `1.000`; anything else is `None`.
fn qvalue(text: &str) -> Option<u16> {
    let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
    if fraction.len() > 3 || !fraction.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    match whole {
        "0" => fraction
            .bytes()
            .chain(std::iter::repeat(b'0'))
            .take(3)
            .try_fold(0_u16, |sum, digit| {
                sum.checked_mul(10)?.checked_add(u16::from(digit - b'0'))
            }),
        "1" if fraction.bytes().all(|b| b == b'0') => Some(1000),
        _ => None,
    }
}

/// How much the request takes `coding` (`br`, `gzip`), in thousandths, by
/// its `Accept-Encoding` (RFC 9110 §12.5.3): the coding's own weight, else
/// that of `*`, else none. `q=0` refuses a coding, and so does a weight
/// that does not parse; names are compared without case.
fn quality(headers: &HeaderMap, coding: &str) -> u16 {
    let (mut own, mut any) = (None, None);
    let values = headers
        .get_all(header::ACCEPT_ENCODING)
        .iter()
        .filter_map(|value| value.to_str().ok());
    for item in values.flat_map(|value| value.split(',')) {
        let mut parts = item.split(';');
        let name = parts.next().unwrap_or("").trim();
        if name.is_empty() {
            continue;
        }
        let mut weight = Some(1000);
        for parameter in parts {
            let (key, value) = parameter.split_once('=').unwrap_or((parameter, ""));
            if key.trim().eq_ignore_ascii_case("q") {
                weight = qvalue(value.trim());
            }
        }
        let weight = weight.unwrap_or(0);
        let named = name.eq_ignore_ascii_case(coding)
            || (coding == "gzip" && name.eq_ignore_ascii_case("x-gzip"));
        if named {
            own = Some(own.map_or(weight, |own: u16| own.max(weight)));
        } else if name == "*" {
            any = Some(weight);
        }
    }
    own.or(any).unwrap_or(0)
}

fn file(app: &App, path: &str, headers: &HeaderMap) -> Option<Response> {
    let file = app.files.get(path)?;
    let br = file.brotli.as_ref().map_or(0, |_| quality(headers, "br"));
    let gzip = file.gzip.as_ref().map_or(0, |_| quality(headers, "gzip"));
    let (stored, encoding): (&Stored, _) = match (&file.brotli, &file.gzip) {
        (Some(brotli), _) if br > 0 && br >= gzip => (brotli, Some("br")),
        (_, Some(compressed)) if gzip > 0 => (compressed, Some("gzip")),
        _ => (&file.plain, None),
    };
    let mut response = Response::new(Body::from(stored.bytes.clone()));
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
    if let Some(etag) = &stored.etag {
        map.insert(header::ETAG, etag.clone());
    }
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

/// Whether the request's `If-None-Match` names `etag`, or `*`. The
/// comparison is the weak one (RFC 9110 §13.1.2): `W/"x"` names `"x"`.
fn names(headers: &HeaderMap, etag: &HeaderValue) -> bool {
    let etag = etag.as_bytes();
    headers
        .get_all(header::IF_NONE_MATCH)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .any(|tag| tag == "*" || tag.strip_prefix("W/").unwrap_or(tag).as_bytes() == etag)
}

/// `response`, or 304 without a body when it is a 200 with an entity tag
/// the request's `If-None-Match` names: the cache's copy is the very bytes.
/// The 304 keeps every header but those of the body itself, and the
/// security headers are added to it as to every answer.
fn conditional(headers: &HeaderMap, response: Response) -> Response {
    let unchanged = response.status() == StatusCode::OK
        && response
            .headers()
            .get(header::ETAG)
            .is_some_and(|etag| names(headers, etag));
    if !unchanged {
        return response;
    }
    let (mut parts, _) = response.into_parts();
    parts.status = StatusCode::NOT_MODIFIED;
    for name in [
        header::CONTENT_TYPE,
        header::CONTENT_ENCODING,
        header::CONTENT_LENGTH,
    ] {
        parts.headers.remove(name);
    }
    Response::from_parts(parts, Body::empty())
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
    conditional(&headers, answer(&app, &headers, &uri))
}

fn answer(app: &App, headers: &HeaderMap, uri: &Uri) -> Response {
    let path = uri.path();
    if let Some(response) = file(app, path, headers) {
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
            return text(
                "application/xml; charset=utf-8",
                generated::sitemap(&app.config.origin, &snapshot(app)),
            );
        }
        "/.well-known/tdmrep.json" => {
            return text("application/json", generated::TDMREP.to_owned());
        }
        "/" => {
            return redirect(
                StatusCode::FOUND,
                &format!("/{}/", negotiated(headers, uri).code()),
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
            Some(found) => page(app, headers, found, lang),
            None => page(app, headers, Page::NotFound, lang),
        };
    }
    if MOVED.contains(&first) {
        let lang = negotiated(headers, uri);
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
        // Chosen by the visitor's language: never permanent, and it varies.
        let lang = negotiated(headers, uri);
        return redirect(StatusCode::FOUND, &format!("/{}/#cards", lang.code()), true);
    }
    let lang = Lang::negotiate(header_text(headers, "accept-language").unwrap_or(""));
    page(app, headers, Page::NotFound, lang)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn accepts(values: &[&str]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for value in values {
            headers.append(
                header::ACCEPT_ENCODING,
                HeaderValue::from_str(value).unwrap(),
            );
        }
        headers
    }

    #[test]
    fn qvalues_parse_as_the_grammar_says() {
        for (text, value) in [
            ("0", Some(0)),
            ("0.", Some(0)),
            ("0.000", Some(0)),
            ("0.5", Some(500)),
            ("0.05", Some(50)),
            ("0.999", Some(999)),
            ("1", Some(1000)),
            ("1.000", Some(1000)),
            ("1.001", None),
            ("1.5", None),
            ("0.0001", None),
            ("2", None),
            ("-0", None),
            (".5", None),
            ("0.5x", None),
            ("", None),
        ] {
            assert_eq!(qvalue(text), value, "{text}");
        }
    }

    #[test]
    fn an_encoding_counts_by_its_own_weight_and_q_0_refuses_it() {
        let cases: [(&[&str], u16, u16); 17] = [
            (&["br, gzip"], 1000, 1000),
            (&["br;q=0, gzip"], 0, 1000),
            (&["br; q=0.000"], 0, 0),
            (&["BR"], 1000, 0),
            (&["brotli"], 0, 0),
            (&["gzip;q=0.8, br;q=0.5"], 500, 800),
            (&["*"], 1000, 1000),
            (&["*;q=0"], 0, 0),
            (&["gzip, *;q=0"], 0, 1000),
            (&["br;q=0, *"], 0, 1000),
            (&["x-gzip"], 0, 1000),
            (&["gzip;q=1.5"], 0, 0),
            (&["gzip;q"], 0, 0),
            (&["gzip;level=9"], 0, 1000),
            (&["gzip;q=0", "br"], 1000, 0),
            (&[""], 0, 0),
            (&[], 0, 0),
        ];
        for (values, br, gzip) in cases {
            let headers = accepts(values);
            assert_eq!(quality(&headers, "br"), br, "{values:?} br");
            assert_eq!(quality(&headers, "gzip"), gzip, "{values:?} gzip");
        }
    }

    #[test]
    fn if_none_match_takes_a_list_star_and_weak_tags() {
        let etag = HeaderValue::from_static("\"0123abcd\"");
        let named = |value: &str| {
            let mut headers = HeaderMap::new();
            headers.insert(header::IF_NONE_MATCH, HeaderValue::from_str(value).unwrap());
            names(&headers, &etag)
        };
        assert!(named("\"0123abcd\""));
        assert!(named("W/\"0123abcd\""));
        assert!(named("\"x\", \"0123abcd\""));
        assert!(named("*"));
        assert!(!named("\"0123abcd"));
        assert!(!named("0123abcd"));
        assert!(!named("\"0123abce\""));
        assert!(!names(&HeaderMap::new(), &etag));
    }
}
