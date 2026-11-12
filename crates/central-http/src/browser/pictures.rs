//! Profile pictures on the browser listener (`docs/profile-pictures.md`).
//!
//! `PUT /api/v1/picture` takes one picture as the raw body, its format in
//! `Content-Type`. Everything about the request is checked on its headers
//! alone, before a byte of the body is read or the service asked: the local
//! proxy, the site's exact `Host` and `Origin`, same-origin fetch metadata
//! when sent, the custom header (which, like the picture's media type,
//! forces a CORS preflight no answer ever grants), no bearer, the client's
//! address and its place in flight, and a `Content-Length` within the
//! format's bound. Then the session's account is admitted (terms, block,
//! budget, an upload slot), and only then the body is read, within
//! [`BODY_TIME`] and at a pace, to its announced length exactly.
//!
//! `GET /api/v1/pictures/{key}.avif` and `.png` serve a picture's still,
//! and `{key}.sequence.avif` the image sequence of one that moves, to anyone,
//! signed in or not (owner, 12.11.2026), by its random key: same-origin or
//! direct requests only, without a bearer, within a budget per address.
//! Every refusal and every unknown key is the same 404, so no key's
//! existence is confirmed. The answers carry their own headers: a sandbox
//! CSP, `nosniff`, CORP same-origin and at most an hour in any cache.
use super::super::boundary::{UPLOAD_PACE, paced};
use super::super::in_flight::forwarded;
use super::{
    Accounts, Browser, IN_FLIGHT, MINUTE, Peer, Refusal, cookie, exactly, failure, refused,
    timed_out,
};
use axum::{
    body::Body,
    extract::{Path, Request as HttpRequest, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header, request::Parts},
    response::{IntoResponse, Response},
};
use central_api::picture::parse_key;
use central_identity::{PictureEncoding, UploadRefusal};
use central_picture::Format;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

/// How long an upload's body may take, its pace held throughout.
pub(super) const BODY_TIME: Duration = Duration::from_secs(20);
/// An upload from its headers to its answer: the body, the worker's own
/// time (`central_picture::TIME`, an animation's encoding included) and
/// the store.
pub(crate) const UPLOAD_TIME: Duration = Duration::from_secs(50);
/// How long a picture's read may take.
pub(super) const SERVE_TIME: Duration = Duration::from_secs(5);
/// Reads of pictures one address may have in flight, in a table of their
/// own, so a page full of pictures never takes the places of its calls.
pub(super) const READS_IN_FLIGHT: u8 = 16;
/// Reads of pictures one address may make in a minute.
pub(super) const READS_PER_MINUTE: u32 = 600;
/// How long a browser may keep a picture: the bound on a removed picture's
/// life in a cache that already holds it.
pub(super) const CACHE: &str = "public, max-age=3600";

/// An upload's format, from its `Content-Type`: exactly one value naming one.
fn format(headers: &HeaderMap) -> Option<Format> {
    let mut values = headers.get_all(header::CONTENT_TYPE).iter();
    let value = values.next()?.to_str().ok()?;
    if values.next().is_some() {
        return None;
    }
    Format::from_content_type(value)
}

/// The upload comes from the site's own page, as `POST /api/v1` must, with
/// a picture's media type instead of JSON.
fn permitted(parts: &Parts, site: &super::BrowserOrigin) -> bool {
    let fetch_site = HeaderName::from_static("sec-fetch-site");
    let mut sites = parts.headers.get_all(&fetch_site).iter();
    let same_origin = match (sites.next(), sites.next()) {
        (None, _) => true,
        (Some(value), None) => value.as_bytes() == b"same-origin",
        (Some(_), Some(_)) => false,
    };
    let local = parts
        .extensions
        .get::<Peer>()
        .is_some_and(|Peer(peer)| peer.ip().is_loopback());
    local
        && parts.uri.scheme().is_none()
        && parts.uri.authority().is_none()
        && exactly(&parts.headers, &header::HOST, &site.host)
        && exactly(&parts.headers, &header::ORIGIN, &site.origin)
        && same_origin
        && exactly(
            &parts.headers,
            &HeaderName::from_static("x-baylee-central"),
            "1",
        )
        && !parts.headers.contains_key(header::AUTHORIZATION)
}

/// The announced length: exactly one well-formed `Content-Length`.
fn length(headers: &HeaderMap) -> Option<usize> {
    let mut values = headers.get_all(header::CONTENT_LENGTH).iter();
    let value = values.next()?.to_str().ok()?;
    if values.next().is_some() || value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}

fn json(status: StatusCode, body: String) -> Response {
    (status, [(header::CONTENT_TYPE, "application/json")], body).into_response()
}

/// A refusal of the upload: the account vocabulary as `POST /api/v1`
/// answers it, a block (403), or the picture's own refusal (422), each
/// with its name for the page. Never 401 for a picture: the page reads 401
/// as an ended session.
fn refusal(refusal: UploadRefusal) -> Response {
    match refusal {
        UploadRefusal::Account(error) => failure(error),
        UploadRefusal::Blocked => json(StatusCode::FORBIDDEN, r#"{"refusal":"blocked"}"#.into()),
        UploadRefusal::Picture(refused) => json(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!(r#"{{"refusal":"{}"}}"#, refused.as_str()),
        ),
    }
}

pub(super) async fn upload<A: Accounts>(
    State(state): State<Arc<Browser<A>>>,
    request: HttpRequest,
) -> Response {
    let (parts, body) = request.into_parts();
    let Some(format) = format(&parts.headers) else {
        return refused();
    };
    if !permitted(&parts, &state.site) {
        return refused();
    }
    let Some(client) = forwarded(&parts.headers) else {
        return Refusal::NoClient.into_response();
    };
    let Some(_permit) = state.in_flight.admit(client, IN_FLIGHT) else {
        return Refusal::Busy.into_response();
    };
    let Some(length) = length(&parts.headers) else {
        return (StatusCode::LENGTH_REQUIRED, "request rejected\n").into_response();
    };
    if length > format.max_bytes() {
        return (StatusCode::PAYLOAD_TOO_LARGE, "request rejected\n").into_response();
    }
    let Some(cookie) = cookie(&parts.headers) else {
        return failure(central_api::Error::Rejected);
    };
    // The account, its terms, its block, its budget and a slot: all before
    // the body is read.
    let admitted = match state.accounts.admit_picture(&cookie).await {
        Ok(admitted) => admitted,
        Err(error) => return refusal(error),
    };
    let bytes = match tokio::time::timeout(BODY_TIME, paced(body, length, UPLOAD_PACE)).await {
        Ok(Ok(bytes)) if bytes.len() == length => bytes,
        Ok(Ok(_)) => return (StatusCode::BAD_REQUEST, "request rejected\n").into_response(),
        Ok(Err(status)) => return (status, "request rejected\n").into_response(),
        Err(_) => return timed_out(),
    };
    match state
        .accounts
        .store_picture(admitted, &cookie, format, bytes.to_vec())
        .await
    {
        Ok(response) => match central_api::encode(&response) {
            Ok(bytes) => ([(header::CONTENT_TYPE, "application/json")], bytes).into_response(),
            Err(error) => failure(error),
        },
        Err(error) => refusal(error),
    }
}

/// A picture's path segment: its key and encoding.
fn file(name: &str) -> Option<([u8; 16], PictureEncoding)> {
    let (key, encoding) = if let Some(key) = name.strip_suffix(".sequence.avif") {
        (key, PictureEncoding::Sequence)
    } else if let Some(key) = name.strip_suffix(".avif") {
        (key, PictureEncoding::Avif)
    } else {
        (name.strip_suffix(".png")?, PictureEncoding::Png)
    };
    Some((parse_key(key)?, encoding))
}

/// The one answer to every refused or unknown read.
fn not_found() -> Response {
    let mut response = (StatusCode::NOT_FOUND, "not found\n").into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

/// A read from the site's page or by the address itself: the local proxy,
/// the exact `Host`, fetch metadata of the same origin or none, no bearer.
fn readable(parts: &Parts, site: &super::BrowserOrigin) -> bool {
    let fetch_site = HeaderName::from_static("sec-fetch-site");
    let mut sites = parts.headers.get_all(&fetch_site).iter();
    let ours = match (sites.next(), sites.next()) {
        (None, _) => true,
        (Some(value), None) => matches!(value.as_bytes(), b"same-origin" | b"none"),
        (Some(_), Some(_)) => false,
    };
    let local = parts
        .extensions
        .get::<Peer>()
        .is_some_and(|Peer(peer)| peer.ip().is_loopback());
    local
        && parts.uri.scheme().is_none()
        && parts.uri.authority().is_none()
        && exactly(&parts.headers, &header::HOST, &site.host)
        && ours
        && !parts.headers.contains_key(header::AUTHORIZATION)
}

pub(super) async fn serve<A: Accounts>(
    State(state): State<Arc<Browser<A>>>,
    Path(name): Path<String>,
    request: HttpRequest,
) -> Response {
    let (parts, _) = request.into_parts();
    let (Some((key, encoding)), true) = (file(&name), readable(&parts, &state.site)) else {
        return not_found();
    };
    let Some(client) = forwarded(&parts.headers) else {
        return not_found();
    };
    let Some(_permit) = state.picture_flight.admit(client, READS_IN_FLIGHT) else {
        return failure(central_api::Error::Busy);
    };
    let charged = match state.budgets.lock() {
        Ok(mut budgets) => {
            budgets
                .pictures
                .charge(client, Instant::now(), READS_PER_MINUTE, MINUTE)
        }
        Err(_) => return failure(central_api::Error::Unavailable),
    };
    if !charged {
        return failure(central_api::Error::Busy);
    }
    let (bytes, digest) = match state.accounts.picture_bytes(key, encoding).await {
        Ok(Some(found)) => found,
        Ok(None) => return not_found(),
        Err(error) => return failure(error),
    };
    let tag = central_api::picture::hex(&digest);
    let etag = format!("\"{tag}\"");
    let matched = parts
        .headers
        .get_all(header::IF_NONE_MATCH)
        .iter()
        .any(|value| value.as_bytes() == etag.as_bytes());
    let mut response = if matched {
        StatusCode::NOT_MODIFIED.into_response()
    } else {
        Body::from(bytes).into_response()
    };
    let headers = response.headers_mut();
    let kind = match encoding {
        PictureEncoding::Avif | PictureEncoding::Sequence => "image/avif",
        PictureEncoding::Png => "image/png",
    };
    for (name, value) in [
        (header::CONTENT_TYPE, kind),
        (header::CACHE_CONTROL, CACHE),
        (header::CONTENT_DISPOSITION, "inline"),
    ] {
        headers.insert(name, HeaderValue::from_static(value));
    }
    if let Ok(value) = HeaderValue::from_str(&etag) {
        headers.insert(header::ETAG, value);
    }
    response
}

/// The security headers of every picture answer, refusals included; the
/// cache directive is the handler's own.
pub(super) async fn headers(request: HttpRequest, next: axum::middleware::Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    for (name, value) in [
        ("x-content-type-options", "nosniff"),
        ("content-security-policy", "default-src 'none'; sandbox"),
        ("cross-origin-resource-policy", "same-origin"),
        ("referrer-policy", "no-referrer"),
        ("x-frame-options", "DENY"),
    ] {
        headers.insert(name, HeaderValue::from_static(value));
    }
    headers
        .entry(header::CACHE_CONTROL)
        .or_insert(HeaderValue::from_static("no-store"));
    response
}
