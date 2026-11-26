//! Addresses: every page lives under its language (`/en/…`, `/de/…`); an
//! address without one (`/`, the old `/download`, the client's `/register`,
//! the app's `/account`) is sent on to the visitor's language. Every
//! response carries the security headers; HTML is never cached unchecked:
//! it is revalidated by its entity tag (`If-None-Match`, 304).

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::{Body, Bytes};
use axum::extract::{Request, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, Uri, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use central_web::data::{Machine, Section, machine};
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
    /// How often a page was opened, where the deployment counts it.
    pub views: Option<Arc<crate::views::Views>>,
}

/// Most pages kept for one snapshot of the data, and their most bytes
/// together. A page is about 25 KB (the start page about 75 KB). Only the
/// start page, the download and the account's overview depend on the
/// visitor's machine ([`varies`]); every other page is kept once per
/// language. Some forty pages in eleven languages, and those three for
/// each of the at most 28 machines told apart (seven platforms, read or
/// assumed, phone or not), are about 1,360 pages and 55 MB: both bounds
/// hold every page there can be, so no request can
/// fill the table with rare pages and leave the common ones to be rendered
/// each time. Past either bound a page is rendered for its request and not
/// kept.
const PAGES: usize = 2_048;
const PAGE_BYTES: usize = 64 * 1024 * 1024;

/// Whether a page is written for the visitor's machine: the three with the
/// download for it (the start page, the download page and the account's
/// overview). They say so to caches (`Vary`), and only they are kept per
/// machine.
const fn varies(page: Page) -> bool {
    matches!(
        page,
        Page::Home | Page::Download | Page::Account(Section::Home)
    )
}

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
    policy(https, "'self'")
}

/// The signed-in account area's policy (`docs/profile-pictures.md` and
/// signed-in chat card previews): `blob:` pictures chosen on the person's
/// device, and Scryfall's image host for opt-out chat card previews (no scripts,
/// styles, or `connect-src` allowed there). Only `/<lang>/account*` sends it.
#[must_use]
pub fn csp_profile(https: bool) -> String {
    policy(https, "'self' blob: https://cards.scryfall.io")
}

/// Whether `path` is inside the signed-in account area, `/{lang}/account` or
/// `/{lang}/account/*`, so `blob:` picture previews remain permitted across
/// soft navigation into the profile picture editor.
fn account_page(path: &str) -> bool {
    path.strip_prefix('/')
        .and_then(|rest| rest.split_once('/'))
        .is_some_and(|(lang, rest)| {
            Lang::from_code(lang).is_some() && (rest == "account" || rest.starts_with("account/"))
        })
}

fn policy(https: bool, images: &str) -> String {
    let mut policy = format!(
        "default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self'; img-src {images}; \
         media-src 'self'; font-src 'self'; connect-src 'self'; worker-src 'self'; manifest-src 'self'; \
         form-action 'none'; base-uri 'none'; frame-ancestors 'none'; object-src 'none'; \
         require-trusted-types-for 'script'; trusted-types baylee baylee-sw baylee-nav",
    );
    if https {
        policy.push_str("; upgrade-insecure-requests");
    }
    policy
}

/// What the browser may use: nothing the site does not need.
pub const PERMISSIONS: &str = "camera=(), microphone=(), geolocation=(), payment=(), usb=(), serial=(), \
     bluetooth=(), hid=(), midi=(), display-capture=()";

async fn security(State(app): State<Arc<App>>, request: Request, next: Next) -> Response {
    let https = app.config.origin.starts_with("https://");
    let profile = account_page(request.uri().path());
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    let mut set = |name: &'static str, value: &str| {
        if let Ok(value) = HeaderValue::from_str(value) {
            headers.insert(name, value);
        }
    };
    set(
        "content-security-policy",
        &if profile {
            csp_profile(https)
        } else {
            csp(https)
        },
    );
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
    app.data.read().map_or_default(|data| Arc::clone(&data))
}

fn page(app: &App, headers: &HeaderMap, page: Page, lang: Lang) -> Response {
    // A page that is the same for every machine is rendered for none in
    // particular, and so kept once.
    let machine = if varies(page) {
        machine(
            header_text(headers, "user-agent").unwrap_or(""),
            header_text(headers, "sec-ch-ua-platform"),
            header_text(headers, "sec-ch-ua-arch"),
            header_text(headers, "sec-ch-ua-bitness"),
        )
    } else {
        Machine {
            platform: None,
            sure: false,
            mobile: false,
        }
    };
    let data = snapshot(app);
    let key = (page, lang, machine);
    // one account's page in the administration has an address per id: it
    // is rendered for its request and never kept
    let found = if page.kept() {
        app.pages.find(key, &data)
    } else {
        None
    };
    let rendered = found.unwrap_or_else(|| {
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
        if page.kept() {
            app.pages.keep(key, &data, &rendered);
        }
        rendered
    });
    let status = match page {
        Page::NotFound => StatusCode::NOT_FOUND,
        Page::Gone => StatusCode::GONE,
        _ => StatusCode::OK,
    };
    // a page that exists, opened by a browser for a person
    if status == StatusCode::OK
        && let Some(views) = &app.views
    {
        views.count(headers);
    }
    let mut extra: Vec<(&'static str, &str)> = Vec::new();
    if varies(page) {
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
const MOVED: [&str; 16] = [
    "download",
    "gateways",
    "contact",
    "donate",
    "impressum",
    "privacy",
    "terms",
    "faq",
    "fan-content",
    "register",
    "signin",
    "reset",
    "account",
    "admin",
    "player",
    "invite",
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

/// Where an address of the old React build's administration (`/admin…`,
/// removed on 12.11.2026) lives now, after `/{lang}/`: its overview, the
/// accounts' list and one account, the gateways, the audit, the legacy
/// accounts, and the feedback's list (with its query, whose form the new
/// list kept) and one report. An id is 32 lowercase hex digits, as the old
/// build wrote it; any other address under `/admin` is no page.
fn old_admin(path: &str, query: Option<&str>) -> Option<String> {
    let rest = path.strip_prefix("/admin")?;
    let rest = rest.strip_suffix('/').unwrap_or(rest);
    let hex =
        |id: &str| id.len() == 32 && id.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
    let with_query = |to: String| match query {
        Some(query) if !query.is_empty() => format!("{to}?{query}"),
        _ => to,
    };
    let mut parts = rest.split('/');
    // the path began with "/admin": what follows must be empty or "/…"
    if parts.next() != Some("") {
        return None;
    }
    match (parts.next(), parts.next(), parts.next()) {
        (None, None, None) => Some("account/admin/overview".to_owned()),
        (Some(page @ ("users" | "gateways" | "audit" | "legacy")), None, None) => {
            Some(format!("account/admin/{page}"))
        }
        (Some("users"), Some(id), None) if hex(id) => Some(format!("account/admin/users/{id}")),
        (Some("feedback" | "reports"), None, None) => {
            Some(with_query("account/admin/feedback".to_owned()))
        }
        (Some("feedback" | "reports"), Some(id), None) if hex(id) => {
            Some(with_query(format!("account/admin/feedback/{id}")))
        }
        _ => None,
    }
}

/// Maximum raw byte length of a percent-encoded query value accepted by
/// `/cards/suggest`.
const MAX_RAW_QUERY_BYTES: usize = 256;

fn query_param<'a>(uri: &'a Uri, name: &str) -> Option<&'a str> {
    uri.query()?.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        (key == name).then_some(value)
    })
}

const fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Decodes a query value (`+` as space, `%XX` as raw byte), returning `None`
/// if it is overlong, malformed, or not valid UTF-8.
fn percent_decode(raw: &str) -> Option<String> {
    if raw.len() > MAX_RAW_QUERY_BYTES {
        return None;
    }
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' => {
                let hi = hex_nibble(*bytes.get(i + 1)?)?;
                let lo = hex_nibble(*bytes.get(i + 2)?)?;
                out.push((hi << 4) | lo);
                i += 3;
            }
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

fn cards_suggest(uri: &Uri) -> Response {
    let query = query_param(uri, "q")
        .and_then(percent_decode)
        .unwrap_or_default();
    let names = crate::cards::suggest(&query, crate::cards::DEFAULT_LIMIT);
    let body = serde_json::to_string(&names).unwrap_or_else(|_| "[]".to_owned());
    let tag = etag(body.as_bytes());
    let mut response = body.into_response();
    let map = response.headers_mut();
    map.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json; charset=utf-8"),
    );
    map.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=3600"),
    );
    map.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    if let Some(tag) = tag {
        map.insert(header::ETAG, tag);
    }
    response
}

const SECURITY_AUDIO_PREFIX: &str = "/static/audio/security/";
const SECURITY_AUDIO_FILES: [(&str, &str); 6] = [
    ("cues.json", "application/json; charset=utf-8"),
    ("intro.opus", "audio/ogg; codecs=opus"),
    ("signin.opus", "audio/ogg; codecs=opus"),
    ("memory.opus", "audio/ogg; codecs=opus"),
    ("chat.opus", "audio/ogg; codecs=opus"),
    ("pairing.opus", "audio/ogg; codecs=opus"),
];
const EMPTY_CUES_JSON: &[u8] = b"{\"available\":false,\"parts\":{}}\n";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ByteRange {
    Bounds(usize, usize),
    Unsatisfiable,
}

/// Parses a single `Range: bytes=...` specification for `len` bytes.
fn parse_byte_range(header_value: &str, len: usize) -> Option<ByteRange> {
    let spec = header_value.trim().strip_prefix("bytes=")?;
    if spec.contains(',') {
        return None;
    }
    let (start_str, end_str) = spec.split_once('-')?;
    if len == 0 {
        return Some(ByteRange::Unsatisfiable);
    }
    if start_str.is_empty() {
        let suffix = end_str.parse::<usize>().ok().filter(|n| *n > 0)?;
        let start = len.saturating_sub(suffix);
        return Some(ByteRange::Bounds(start, len - 1));
    }
    let start = start_str.parse::<usize>().ok()?;
    if start >= len {
        return Some(ByteRange::Unsatisfiable);
    }
    if end_str.is_empty() {
        return Some(ByteRange::Bounds(start, len - 1));
    }
    let end = end_str.parse::<usize>().ok()?;
    if end < start {
        return Some(ByteRange::Unsatisfiable);
    }
    Some(ByteRange::Bounds(start, end.min(len - 1)))
}

fn read_security_audio_bytes(site: &Path, lang: Lang, filename: &'static str) -> Option<Vec<u8>> {
    let built = site
        .join("static")
        .join("audio")
        .join("security")
        .join(lang.code())
        .join(filename);
    if built.is_file()
        && let Ok(bytes) = std::fs::read(&built)
    {
        return Some(bytes);
    }
    let dev = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../central-web/static/audio/security")
        .join(lang.code())
        .join(filename);
    if dev.is_file()
        && let Ok(bytes) = std::fs::read(&dev)
    {
        return Some(bytes);
    }
    None
}

fn security_audio_response(site: &Path, path: &str, headers: &HeaderMap) -> Option<Response> {
    let rest = path.strip_prefix(SECURITY_AUDIO_PREFIX)?;
    let (lang_raw, file_raw) = rest.split_once('/')?;
    let lang = Lang::from_code(lang_raw)?;
    let (filename, kind) = SECURITY_AUDIO_FILES
        .iter()
        .copied()
        .find(|(allowed, _)| *allowed == file_raw)?;

    let (bytes, cache_control): (Vec<u8>, &'static str) =
        if let Some(bytes) = read_security_audio_bytes(site, lang, filename) {
            let cache = if filename == "cues.json" {
                "no-cache"
            } else {
                "public, max-age=3600"
            };
            (bytes, cache)
        } else if filename == "cues.json" {
            (EMPTY_CUES_JSON.to_vec(), "no-cache")
        } else {
            return None;
        };

    let total = bytes.len();
    let tag = etag(&bytes);
    if let Some(range_hdr) = header_text(headers, "range")
        && let Some(parsed) = parse_byte_range(range_hdr, total)
    {
        return Some(match parsed {
            ByteRange::Unsatisfiable => {
                let mut response = Response::new(Body::empty());
                *response.status_mut() = StatusCode::RANGE_NOT_SATISFIABLE;
                let map = response.headers_mut();
                map.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
                if let Ok(val) = HeaderValue::from_str(&format!("bytes */{total}")) {
                    map.insert(header::CONTENT_RANGE, val);
                }
                response
            }
            ByteRange::Bounds(start, end) => {
                let slice = Bytes::copy_from_slice(&bytes[start..=end]);
                let mut response = Response::new(Body::from(slice));
                *response.status_mut() = StatusCode::PARTIAL_CONTENT;
                let map = response.headers_mut();
                map.insert(header::CONTENT_TYPE, HeaderValue::from_static(kind));
                map.insert(
                    header::CACHE_CONTROL,
                    HeaderValue::from_static(cache_control),
                );
                map.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
                map.insert(
                    header::X_CONTENT_TYPE_OPTIONS,
                    HeaderValue::from_static("nosniff"),
                );
                if let Ok(val) = HeaderValue::from_str(&format!("bytes {start}-{end}/{total}")) {
                    map.insert(header::CONTENT_RANGE, val);
                }
                if let Some(tag) = tag {
                    map.insert(header::ETAG, tag);
                }
                response
            }
        });
    }

    let mut response = Response::new(Body::from(bytes));
    let map = response.headers_mut();
    map.insert(header::CONTENT_TYPE, HeaderValue::from_static(kind));
    map.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(cache_control),
    );
    map.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    map.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    if let Some(tag) = tag {
        map.insert(header::ETAG, tag);
    }
    Some(response)
}

fn special_route(app: &App, headers: &HeaderMap, uri: &Uri) -> Option<Response> {
    if uri.path().starts_with(SECURITY_AUDIO_PREFIX) {
        return Some(
            security_audio_response(&app.config.site, uri.path(), headers)
                .unwrap_or_else(|| StatusCode::NOT_FOUND.into_response()),
        );
    }
    match uri.path() {
        "/cards/suggest" => Some(cards_suggest(uri)),
        "/robots.txt" => Some(text(
            "text/plain; charset=utf-8",
            generated::robots(&app.config.origin),
        )),
        "/sitemap.xml" => Some(text(
            "application/xml; charset=utf-8",
            generated::sitemap(&app.config.origin, &snapshot(app)),
        )),
        "/funding.json" => Some(text(
            "application/json",
            generated::funding(&app.config.origin, &snapshot(app)),
        )),
        "/.well-known/tdmrep.json" => Some(text("application/json", generated::TDMREP.to_owned())),
        "/datenschutz" | "/de/datenschutz" => Some(redirect(
            StatusCode::MOVED_PERMANENTLY,
            "/de/privacy",
            false,
        )),
        "/agb" | "/de/agb" => Some(redirect(StatusCode::MOVED_PERMANENTLY, "/de/terms", false)),
        "/" => Some(redirect(
            StatusCode::FOUND,
            &format!("/{}/", negotiated(headers, uri).code()),
            true,
        )),
        _ => None,
    }
}

fn answer(app: &App, headers: &HeaderMap, uri: &Uri) -> Response {
    let path = uri.path();
    if let Some(response) = file(app, path, headers) {
        return response;
    }
    // the old React build's administration, bookmarked: its pages live in
    // the account area now
    if let Some(rest) = old_admin(path, uri.query()) {
        let to = format!("/{}/{rest}", negotiated(headers, uri).code());
        return redirect(StatusCode::FOUND, &to, true);
    }
    if let Some(response) = special_route(app, headers, uri) {
        return response;
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
    // Not found, in the language the browser asks for: the answer depends
    // on that header and says so, as the redirects above do.
    let lang = Lang::negotiate(header_text(headers, "accept-language").unwrap_or(""));
    let mut response = page(app, headers, Page::NotFound, lang);
    response
        .headers_mut()
        .insert(header::VARY, HeaderValue::from_static("Accept-Language"));
    response
}

/// The router: one handler for every address, a ten-second deadline, and
/// the security headers on every answer, the deadline's own included (the
/// layer added last is the outermost).
pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .fallback(any)
        .layer(tower_http::timeout::TimeoutLayer::with_status_code(
            StatusCode::SERVICE_UNAVAILABLE,
            std::time::Duration::from_secs(10),
        ))
        .layer(middleware::from_fn_with_state(Arc::clone(&app), security))
        .with_state(app)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_account_area_allows_blob_pictures() {
        for allowed in [
            "/en/account",
            "/en/account/",
            "/en/account/profile",
            "/pt-br/account/profile/",
            "/en/account/privacy",
            "/en/account/admin/feedback",
        ] {
            assert!(account_page(allowed), "{allowed}");
        }
        for other in [
            "/xx/account/profile",
            "/account/profile",
            "/en/accountfoo",
            "/en/chat",
            "/en/",
        ] {
            assert!(!account_page(other), "{other}");
        }
        assert!(csp_profile(true).contains("img-src 'self' blob: https://cards.scryfall.io;"));
        assert!(csp(true).contains("img-src 'self';"));
        assert!(csp(true).contains("media-src 'self';"));
        assert!(csp_profile(true).contains("media-src 'self';"));
        assert!(!csp(false).contains("blob:"));
        assert!(!csp(false).contains("scryfall"));
        // the rest of the policy is the same
        assert_eq!(
            csp_profile(false).replace(" blob: https://cards.scryfall.io", ""),
            csp(false)
        );
    }

    #[test]
    fn security_audio_validates_paths_and_supports_ranges() {
        assert_eq!(
            parse_byte_range("bytes=0-3", 10),
            Some(ByteRange::Bounds(0, 3))
        );
        assert_eq!(
            parse_byte_range("bytes=5-", 10),
            Some(ByteRange::Bounds(5, 9))
        );
        assert_eq!(
            parse_byte_range("bytes=-4", 10),
            Some(ByteRange::Bounds(6, 9))
        );
        assert_eq!(
            parse_byte_range("bytes=10-", 10),
            Some(ByteRange::Unsatisfiable)
        );
        assert_eq!(
            parse_byte_range("bytes=6-3", 10),
            Some(ByteRange::Unsatisfiable)
        );

        let root =
            std::env::temp_dir().join(format!("baylee-routes-audio-test-{}", std::process::id()));
        let en_dir = root.join("static/audio/security/en");
        std::fs::create_dir_all(&en_dir).unwrap();
        let fake_opus = b"OggS-fake-opus-data";
        std::fs::write(en_dir.join("intro.opus"), fake_opus).unwrap();

        let headers = HeaderMap::new();
        // Traversal or unknown files/languages must never resolve.
        assert!(
            security_audio_response(&root, "/static/audio/security/../en/intro.opus", &headers)
                .is_none()
        );
        assert!(
            security_audio_response(&root, "/static/audio/security/en/secret.txt", &headers)
                .is_none()
        );
        assert!(
            security_audio_response(&root, "/static/audio/security/xx/intro.opus", &headers)
                .is_none()
        );

        // Missing cues.json gracefully serves an empty manifest without 404 console noise.
        let stub = security_audio_response(&root, "/static/audio/security/en/cues.json", &headers)
            .expect("cues fallback");
        assert_eq!(stub.status(), StatusCode::OK);
        assert_eq!(
            stub.headers()[header::CONTENT_TYPE],
            "application/json; charset=utf-8"
        );

        // Present Opus file supports full download and partial byte-range requests.
        let full = security_audio_response(&root, "/static/audio/security/en/intro.opus", &headers)
            .expect("intro opus");
        assert_eq!(full.status(), StatusCode::OK);
        assert_eq!(
            full.headers()[header::CONTENT_TYPE],
            "audio/ogg; codecs=opus"
        );
        assert_eq!(full.headers()[header::ACCEPT_RANGES], "bytes");

        let mut ranged_headers = HeaderMap::new();
        ranged_headers.insert(header::RANGE, HeaderValue::from_static("bytes=4-7"));
        let part = security_audio_response(
            &root,
            "/static/audio/security/en/intro.opus",
            &ranged_headers,
        )
        .expect("ranged opus");
        assert_eq!(part.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(
            part.headers()[header::CONTENT_RANGE],
            format!("bytes 4-7/{}", fake_opus.len()).as_str()
        );

        assert!(MOVED.contains(&"invite"));
        let _ = std::fs::remove_dir_all(&root);
    }

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

    #[test]
    fn percent_decode_handles_valid_and_malformed_queries() {
        assert_eq!(percent_decode("sol+r").as_deref(), Some("sol r"));
        assert_eq!(percent_decode("sol%20r").as_deref(), Some("sol r"));
        assert_eq!(
            percent_decode("Lim-D%C3%BBl").as_deref(),
            Some("Lim-D\u{00fb}l")
        );
        assert_eq!(percent_decode("%ZZ"), None);
        assert_eq!(percent_decode("%2"), None);
        assert_eq!(percent_decode("%FF%FE"), None);
        assert_eq!(percent_decode(&"a".repeat(MAX_RAW_QUERY_BYTES + 1)), None);
        let uri: Uri = "/cards/suggest?foo=1&q=sol%20r&bar=2".parse().unwrap();
        assert_eq!(query_param(&uri, "q"), Some("sol%20r"));
    }

    #[tokio::test]
    async fn cards_suggest_answers_json_with_cache_and_nosniff_headers() {
        let uri: Uri = "/cards/suggest?q=sol%20r".parse().unwrap();
        let response = cards_suggest(&uri);
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[header::CONTENT_TYPE],
            "application/json; charset=utf-8"
        );
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "public, max-age=3600"
        );
        assert_eq!(
            response.headers()[header::X_CONTENT_TYPE_OPTIONS],
            "nosniff"
        );
        let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .unwrap();
        let names: Vec<String> = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(names.first().map(String::as_str), Some("Sol Ring"));

        let empty: Uri = "/cards/suggest?q=%ZZ".parse().unwrap();
        let bytes = axum::body::to_bytes(cards_suggest(&empty).into_body(), 64 * 1024)
            .await
            .unwrap();
        assert_eq!(&bytes[..], b"[]");
    }
}
