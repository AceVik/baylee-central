//! What the moderation asks of Central: `AdminRequest`'s report actions on
//! the browser surface (`POST /api/v1`, `central::call`), each answer taken
//! only in the shape its request expects; and the two parts Central keeps
//! as gzip (the client's details, a game's record), read from their own
//! route (`POST /api/v1/reports/<hex>/<part>`) no further than their bound.

use central_api::admin::{AdminAnswer, AdminRequest};
use central_api::reports::{
    BulkAction, CrashGroup, DayCount, MAX_PAGE, Neighbours, ReportAdmin, ReportChange,
    ReportDetail, ReportFacets, ReportFilter, ReportSort, ReportSummary, SavedView,
};
use central_api::wire::{Request, Response};

use super::super::central::{Failure, call};
use super::query::Asked;

/// Sends one administrator's request; `Ok(None)` is Central's `Done`.
async fn ask(request: AdminRequest) -> Result<Option<AdminAnswer>, Failure> {
    match call(&Request::Admin { request }).await? {
        Response::Admin { answer } => Ok(Some(answer)),
        Response::Done => Ok(None),
        _ => Err(Failure::Malformed),
    }
}

/// An answer of another kind than the request's: for a read, nothing to
/// show; for a change, nothing to tell what happened.
const OTHER: Failure = Failure::Malformed;

/// A page of reports and how many the filter matches.
pub async fn reports(asked: &Asked) -> Result<(Vec<ReportSummary>, u64), Failure> {
    let limit = asked.limit.min(MAX_PAGE);
    match ask(AdminRequest::Reports {
        filter: asked.filter.clone(),
        sort: asked.sort,
        descending: asked.descending,
        offset: asked.offset,
        limit,
    })
    .await?
    {
        Some(AdminAnswer::Reports { reports, total })
            if reports.len() <= usize::try_from(limit).unwrap_or(usize::MAX) =>
        {
            Ok((reports, total))
        }
        _ => Err(OTHER),
    }
}

/// The counts per dimension under `filter`.
pub async fn facets(filter: ReportFilter) -> Result<ReportFacets, Failure> {
    match ask(AdminRequest::ReportFacets { filter }).await? {
        Some(AdminAnswer::ReportFacets { facets }) => Ok(facets),
        _ => Err(OTHER),
    }
}

/// One report.
pub async fn report(report: [u8; 16]) -> Result<ReportDetail, Failure> {
    match ask(AdminRequest::Report { report }).await? {
        Some(AdminAnswer::Report { report: found }) if found.summary.id == report => Ok(*found),
        _ => Err(OTHER),
    }
}

/// A report's place in the list `filter`, `sort`, `descending`.
pub async fn neighbours(
    report: [u8; 16],
    filter: ReportFilter,
    sort: ReportSort,
    descending: bool,
) -> Result<Neighbours, Failure> {
    match ask(AdminRequest::ReportNeighbours {
        report,
        filter,
        sort,
        descending,
    })
    .await?
    {
        Some(AdminAnswer::Neighbours { neighbours }) => Ok(neighbours),
        _ => Err(OTHER),
    }
}

/// The crashes under `filter`, by group.
pub async fn crash_groups(filter: ReportFilter) -> Result<Vec<CrashGroup>, Failure> {
    match ask(AdminRequest::CrashGroups { filter }).await? {
        Some(AdminAnswer::CrashGroups { groups }) => Ok(groups),
        _ => Err(OTHER),
    }
}

/// Reports per day and kind.
pub async fn stats() -> Result<Vec<DayCount>, Failure> {
    match ask(AdminRequest::ReportStats).await? {
        Some(AdminAnswer::ReportStats { days }) => Ok(days),
        _ => Err(OTHER),
    }
}

/// The administrators a report may be assigned to.
pub async fn admins() -> Result<Vec<ReportAdmin>, Failure> {
    match ask(AdminRequest::ReportAdmins).await? {
        Some(AdminAnswer::ReportAdmins { admins }) => Ok(admins),
        _ => Err(OTHER),
    }
}

/// The reading administrator's saved lists.
pub async fn views(request: AdminRequest) -> Result<Vec<SavedView>, Failure> {
    match ask(request).await? {
        Some(AdminAnswer::ReportViews { views }) => Ok(views),
        _ => Err(OTHER),
    }
}

/// A change to one report, read at `revision`; the report as it is now.
pub async fn set_report(
    report: [u8; 16],
    change: ReportChange,
    revision: u32,
) -> Result<ReportDetail, Failure> {
    one(AdminRequest::SetReport {
        report,
        change,
        expected_revision: Some(revision),
    })
    .await
}

/// A request answered with the report.
pub async fn one(request: AdminRequest) -> Result<ReportDetail, Failure> {
    match ask(request).await? {
        Some(AdminAnswer::Report { report }) => Ok(*report),
        _ => Err(OTHER),
    }
}

/// A selection's action; how many it changed.
pub async fn bulk(reports: Vec<[u8; 16]>, bulk: BulkAction) -> Result<u32, Failure> {
    match ask(AdminRequest::ReportsBulk { reports, bulk }).await? {
        Some(AdminAnswer::Changed { changed }) => Ok(changed),
        _ => Err(OTHER),
    }
}

/// A change Central answers with `Done`.
pub async fn done(request: AdminRequest) -> Result<(), Failure> {
    match ask(request).await? {
        None => Ok(()),
        Some(_) => Err(OTHER),
    }
}

/// Where this browser's account stands, asked after Central refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Standing {
    /// Nobody is signed in any more.
    SignedOut,
    /// Signed in, without the role.
    NotAdmin,
    /// Signed in with the role; the password sign-in at these Unix seconds.
    Admin(i64),
    /// Not known.
    Unknown,
}

/// Asks Central where this browser's account stands.
pub async fn standing() -> Standing {
    match call(&Request::AccountSummary).await {
        Err(Failure::Rejected) => Standing::SignedOut,
        Ok(Response::AccountSummary { admin: false, .. }) => Standing::NotAdmin,
        Ok(Response::AccountSummary {
            admin: true,
            signed_in_at,
            ..
        }) => Standing::Admin(signed_in_at),
        _ => Standing::Unknown,
    }
}

/// A part's bytes as Central keeps them.
pub struct PartBytes {
    /// The gzip.
    pub gzip: Vec<u8>,
}

/// Why a part could not be read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartFailure {
    /// Central's refusal or no answer.
    Central(Failure),
    /// More than its bound, or no gzip.
    Shape,
}

/// The part `part` (`client` or `record`) of report `report`, at most
/// `max` bytes of gzip.
#[cfg(target_arch = "wasm32")]
pub async fn part(report: [u8; 16], part: &str, max: usize) -> Result<PartBytes, PartFailure> {
    use leptos::prelude::window;
    use leptos::wasm_bindgen::JsCast;
    use leptos::web_sys;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{ReferrerPolicy, RequestCache, RequestCredentials, RequestInit, RequestRedirect};

    let lost = PartFailure::Central(Failure::Network);
    let url = format!("/api/v1/reports/{}/{part}", super::text::hex(&report));
    let init = RequestInit::new();
    init.set_method("POST");
    init.set_credentials(RequestCredentials::SameOrigin);
    init.set_redirect(RequestRedirect::Error);
    init.set_referrer_policy(ReferrerPolicy::NoReferrer);
    init.set_cache(RequestCache::NoStore);
    let request = web_sys::Request::new_with_str_and_init(&url, &init).map_err(|_| lost)?;
    for (name, value) in [
        ("Content-Type", "application/json"),
        ("X-Baylee-Central", "1"),
    ] {
        request.headers().set(name, value).map_err(|_| lost)?;
    }
    let response: web_sys::Response = JsFuture::from(window().fetch_with_request(&request))
        .await
        .map_err(|_| lost)?
        .dyn_into()
        .map_err(|_| lost)?;
    let status = response.status();
    if status != 200 {
        return Err(PartFailure::Central(match status {
            401 => Failure::Rejected,
            403 => Failure::Forbidden,
            409 => Failure::Conflict,
            429 => Failure::Busy,
            500..=599 => Failure::Unavailable,
            _ => Failure::Malformed,
        }));
    }
    let headers = response.headers();
    if headers.get("content-type").ok().flatten().as_deref() != Some("application/gzip") {
        return Err(PartFailure::Shape);
    }
    let origin = headers.get("x-record-origin").ok().flatten();
    let fits = match part {
        "client" => origin.is_none(),
        _ => matches!(origin.as_deref(), Some("gateway" | "client")),
    };
    if !fits {
        return Err(PartFailure::Shape);
    }
    let declared = headers
        .get("content-length")
        .ok()
        .flatten()
        .and_then(|n| n.parse::<usize>().ok());
    if declared.is_some_and(|n| n > max) {
        return Err(PartFailure::Shape);
    }
    let body = browser::get(response.as_ref(), "body").ok_or(PartFailure::Shape)?;
    let gzip = browser::read_stream(&body, max)
        .await
        .map_err(|too| if too { PartFailure::Shape } else { lost })?;
    // gzip's magic, which Central checked at intake
    if gzip.len() < 18 || gzip[0] != 0x1f || gzip[1] != 0x8b {
        return Err(PartFailure::Shape);
    }
    Ok(PartBytes { gzip })
}

/// On the server nothing is read: the client part is not sent, and a
/// record is nothing the page can take.
#[cfg(not(target_arch = "wasm32"))]
pub fn part(
    _report: [u8; 16],
    part: &str,
    _max: usize,
) -> std::future::Ready<Result<PartBytes, PartFailure>> {
    std::future::ready(Err(if part == "client" {
        PartFailure::Central(Failure::Network)
    } else {
        PartFailure::Shape
    }))
}

/// `gzip` inflated by the browser's `DecompressionStream`, refused past
/// `max` bytes of output, counted while it streams; `Err(true)` for too
/// much, `Err(false)` for no gzip.
#[cfg(target_arch = "wasm32")]
pub async fn inflate(gzip: &[u8], max: usize) -> Result<Vec<u8>, bool> {
    let stream = browser::inflating(gzip).ok_or(false)?;
    browser::read_stream(&stream, max).await
}

/// On the server nothing is inflated.
#[cfg(not(target_arch = "wasm32"))]
pub fn inflate(_gzip: &[u8], _max: usize) -> std::future::Ready<Result<Vec<u8>, bool>> {
    std::future::ready(Err(false))
}

/// The browser's own pieces, reached through `js_sys` alone (as `clip`
/// does), so no further `web-sys` feature is needed.
#[cfg(target_arch = "wasm32")]
pub mod browser {
    use leptos::prelude::*;
    use leptos::wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::JsFuture;

    /// `name` of the object `of`, if it is one.
    pub fn get(of: &JsValue, name: &str) -> Option<JsValue> {
        js_sys::Reflect::get(of, &JsValue::from_str(name))
            .ok()
            .filter(|value| !value.is_undefined() && !value.is_null())
    }

    fn method(of: &JsValue, name: &str) -> Option<js_sys::Function> {
        get(of, name)?.dyn_into::<js_sys::Function>().ok()
    }

    fn construct(name: &str, args: &js_sys::Array) -> Option<JsValue> {
        let make = method(&js_sys::global(), name)?;
        js_sys::Reflect::construct(&make, args).ok()
    }

    /// A `Blob` of `bytes` typed `mime`.
    pub fn blob(bytes: &[u8], mime: &str) -> Option<JsValue> {
        let parts = js_sys::Array::of1(&js_sys::Uint8Array::from(bytes));
        let options = js_sys::Object::new();
        js_sys::Reflect::set(
            &options,
            &JsValue::from_str("type"),
            &JsValue::from_str(mime),
        )
        .ok()?;
        construct("Blob", &js_sys::Array::of2(&parts, &options))
    }

    /// The stream of `gzip` inflated: `Blob.stream()` through a
    /// `DecompressionStream("gzip")`.
    pub fn inflating(gzip: &[u8]) -> Option<JsValue> {
        let source = blob(gzip, "application/gzip")?;
        let stream = method(&source, "stream")?.call0(&source).ok()?;
        let inflate = construct(
            "DecompressionStream",
            &js_sys::Array::of1(&JsValue::from_str("gzip")),
        )?;
        method(&stream, "pipeThrough")?
            .call1(&stream, &inflate)
            .ok()
    }

    /// Every chunk of `stream`, refused past `max` bytes as they come
    /// (`Err(true)`, the stream cancelled); `Err(false)` when it fails.
    pub async fn read_stream(stream: &JsValue, max: usize) -> Result<Vec<u8>, bool> {
        let reader = method(stream, "getReader")
            .and_then(|f| f.call0(stream).ok())
            .ok_or(false)?;
        let read = method(&reader, "read").ok_or(false)?;
        let mut out: Vec<u8> = Vec::new();
        loop {
            let promise = read
                .call0(&reader)
                .ok()
                .and_then(|p| p.dyn_into::<js_sys::Promise>().ok())
                .ok_or(false)?;
            let step = JsFuture::from(promise).await.map_err(|_| false)?;
            if get(&step, "done").and_then(|d| d.as_bool()) == Some(true) {
                return Ok(out);
            }
            let chunk = get(&step, "value")
                .and_then(|v| v.dyn_into::<js_sys::Uint8Array>().ok())
                .ok_or(false)?;
            let length = usize::try_from(chunk.length()).unwrap_or(usize::MAX);
            if out.len().saturating_add(length) > max {
                if let Some(cancel) = method(&reader, "cancel") {
                    let _ = cancel.call0(&reader);
                }
                return Err(true);
            }
            let start = out.len();
            out.resize(start + length, 0);
            chunk.copy_to(&mut out[start..]);
        }
    }

    /// Hands `bytes` to the browser as the file `name`: a `blob:` address
    /// made for the click, given back half a minute later.
    pub fn save(name: &str, bytes: &[u8], mime: &str) -> bool {
        let Some(file) = blob(bytes, mime) else {
            return false;
        };
        let Some(url_type) = get(&js_sys::global(), "URL") else {
            return false;
        };
        let Some(address) = method(&url_type, "createObjectURL")
            .and_then(|f| f.call1(&url_type, &file).ok())
            .and_then(|a| a.as_string())
        else {
            return false;
        };
        let (Some(body), Ok(link)) = (document().body(), document().create_element("a")) else {
            return false;
        };
        let _ = link.set_attribute("href", &address);
        let _ = link.set_attribute("download", name);
        let _ = link.set_attribute("hidden", "");
        if body.append_child(&link).is_ok()
            && let Ok(link) = link.dyn_into::<leptos::web_sys::HtmlElement>()
        {
            link.click();
            link.remove();
        }
        let _ = set_timeout(
            move || {
                if let Some(url_type) = get(&js_sys::global(), "URL")
                    && let Some(revoke) = method(&url_type, "revokeObjectURL")
                {
                    let _ = revoke.call1(&url_type, &JsValue::from_str(&address));
                }
            },
            std::time::Duration::from_secs(30),
        );
        true
    }

    /// Draws the checked PNG `png` into the canvas `canvas`: decoded by
    /// `createImageBitmap`, never through an address, so the page's
    /// `img-src 'self'` stays as it is.
    pub async fn draw(canvas: leptos::web_sys::HtmlCanvasElement, png: &[u8]) -> bool {
        let drawn = async {
            let file = blob(png, "image/png")?;
            let global = js_sys::global();
            let promise = method(&global, "createImageBitmap")?
                .call1(&global, &file)
                .ok()?
                .dyn_into::<js_sys::Promise>()
                .ok()?;
            let bitmap = JsFuture::from(promise).await.ok()?;
            let canvas: &JsValue = canvas.as_ref();
            let context = method(canvas, "getContext")?
                .call1(canvas, &JsValue::from_str("2d"))
                .ok()
                .filter(|c| !c.is_null())?;
            let draw = method(&context, "drawImage")?;
            draw.call3(&context, &bitmap, &JsValue::from(0), &JsValue::from(0))
                .ok()?;
            if let Some(close) = method(&bitmap, "close") {
                let _ = close.call0(&bitmap);
            }
            Some(())
        };
        drawn.await.is_some()
    }
}

/// On the server nothing is saved or drawn.
#[cfg(not(target_arch = "wasm32"))]
pub mod browser {
    /// Nothing to save on the server.
    pub fn save(_name: &str, _bytes: &[u8], _mime: &str) -> bool {
        false
    }
}
