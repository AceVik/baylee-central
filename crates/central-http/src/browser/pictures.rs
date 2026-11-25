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
use super::super::in_flight::{Permit, forwarded};
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
use central_api::{Token, picture::parse_key};
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
/// life in a cache that already holds it. `private`, so no shared cache
/// (a proxy or a CDN in front later) keeps it for others after its removal
/// promised a 404 at once; the hour stays for the viewer's own browser,
/// whose copy only that viewer sees. `no-cache` with the `ETag` would ask
/// Central for every avatar on every page instead.
pub(super) const CACHE: &str = "private, max-age=3600";

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

/// Canonical lowercase hyphenated `UUIDv7` as Central writes group IDs.
pub(super) fn parse_group_id(text: &str) -> Option<[u8; 16]> {
    let bytes = text.as_bytes();
    if bytes.len() != 36 {
        return None;
    }
    let mut hex = [0_u8; 32];
    let mut out = 0;
    for (idx, &byte) in bytes.iter().enumerate() {
        match idx {
            8 | 13 | 18 | 23 => {
                if byte != b'-' {
                    return None;
                }
            }
            _ => {
                if !matches!(byte, b'0'..=b'9' | b'a'..=b'f') {
                    return None;
                }
                hex[out] = byte;
                out += 1;
            }
        }
    }
    let hex_str = std::str::from_utf8(&hex).ok()?;
    let id = parse_key(hex_str)?;
    (id[6] >> 4 == 7 && id[8] >> 6 == 2).then_some(id)
}

fn prepare_upload<'a, A>(
    state: &'a Browser<A>,
    parts: &Parts,
) -> Result<(Permit<'a>, Format, usize, Token), Box<Response>> {
    let Some(format) = format(&parts.headers) else {
        return Err(Box::new(refused()));
    };
    if !permitted(parts, &state.site) {
        return Err(Box::new(refused()));
    }
    let Some(client) = forwarded(&parts.headers) else {
        return Err(Box::new(Refusal::NoClient.into_response()));
    };
    let Some(permit) = state.in_flight.admit(client, IN_FLIGHT) else {
        return Err(Box::new(Refusal::Busy.into_response()));
    };
    let Some(length) = length(&parts.headers) else {
        return Err(Box::new(
            (StatusCode::LENGTH_REQUIRED, "request rejected\n").into_response(),
        ));
    };
    if length > format.max_bytes() {
        return Err(Box::new(
            (StatusCode::PAYLOAD_TOO_LARGE, "request rejected\n").into_response(),
        ));
    }
    let Some(cookie) = cookie(&parts.headers) else {
        return Err(Box::new(failure(central_api::Error::Rejected)));
    };
    Ok((permit, format, length, cookie))
}

async fn read_upload_body(body: Body, length: usize) -> Result<Vec<u8>, Box<Response>> {
    match tokio::time::timeout(BODY_TIME, paced(body, length, UPLOAD_PACE)).await {
        Ok(Ok(bytes)) if bytes.len() == length => Ok(bytes.to_vec()),
        Ok(Ok(_)) => Err(Box::new(
            (StatusCode::BAD_REQUEST, "request rejected\n").into_response(),
        )),
        Ok(Err(status)) => Err(Box::new((status, "request rejected\n").into_response())),
        Err(_) => Err(Box::new(timed_out())),
    }
}

pub(super) async fn upload<A: Accounts>(
    State(state): State<Arc<Browser<A>>>,
    request: HttpRequest,
) -> Response {
    let (parts, body) = request.into_parts();
    let (_permit, format, length, cookie) = match prepare_upload(&state, &parts) {
        Ok(prepared) => prepared,
        Err(response) => return *response,
    };
    // The account, its terms, its block, its budget and a slot: all before
    // the body is read.
    let admitted = match state.accounts.admit_picture(&cookie).await {
        Ok(admitted) => admitted,
        Err(error) => return refusal(error),
    };
    let bytes = match read_upload_body(body, length).await {
        Ok(bytes) => bytes,
        Err(response) => return *response,
    };
    match state
        .accounts
        .store_picture(admitted, &cookie, format, bytes)
        .await
    {
        Ok(response) => match central_api::encode(&response) {
            Ok(bytes) => ([(header::CONTENT_TYPE, "application/json")], bytes).into_response(),
            Err(error) => failure(error),
        },
        Err(error) => refusal(error),
    }
}

pub(super) async fn upload_group<A: Accounts>(
    State(state): State<Arc<Browser<A>>>,
    Path(group): Path<String>,
    request: HttpRequest,
) -> Response {
    let (parts, body) = request.into_parts();
    let (_permit, format, length, cookie) = match prepare_upload(&state, &parts) {
        Ok(prepared) => prepared,
        Err(response) => return *response,
    };
    let Some(group) = parse_group_id(&group) else {
        return failure(central_api::Error::Rejected);
    };
    let admitted = match state.accounts.admit_group_picture(&cookie, group).await {
        Ok(admitted) => admitted,
        Err(error) => return refusal(error),
    };
    let bytes = match read_upload_body(body, length).await {
        Ok(bytes) => bytes,
        Err(response) => return *response,
    };
    match state
        .accounts
        .store_group_picture(admitted, &cookie, format, bytes)
        .await
    {
        Ok((key, moving)) => json(
            StatusCode::OK,
            format!(
                r#"{{"key":"{}","moving":{moving}}}"#,
                central_api::picture::key_hex(&key)
            ),
        ),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_s_file_is_its_key_and_one_known_ending() {
        let digits = "0123456789abcdef".repeat(2);
        let key = digits.as_str();
        let bytes = parse_key(key).unwrap();
        for (ending, encoding) in [
            (".png", PictureEncoding::Png),
            (".avif", PictureEncoding::Avif),
            (".sequence.avif", PictureEncoding::Sequence),
        ] {
            let found = file(&format!("{key}{ending}")).unwrap();
            assert!(found == (bytes, encoding), "{ending}");
            for bad in [
                format!("{}{ending}", &key[1..]),
                format!("{key}0{ending}"),
                format!("{}{ending}", key.to_uppercase()),
                format!("../{}{ending}", &key[3..]),
                format!("{key}{ending}/"),
                format!("{key}{ending}\0"),
                format!("{key}{}", ending.to_uppercase()),
                format!("{key}.png{ending}"),
                ending.to_owned(),
            ] {
                assert!(file(&bad).is_none(), "{bad:?}");
            }
        }
        for bad in ["", key, &format!("{key}."), &format!("{key}.svg")] {
            assert!(file(bad).is_none(), "{bad:?}");
        }
    }

    #[test]
    fn an_upload_announces_one_length_and_one_type() {
        let headers = |name: header::HeaderName, values: &[&str]| {
            let mut headers = HeaderMap::new();
            for value in values {
                headers.append(&name, value.parse().unwrap());
            }
            headers
        };
        let length = |values: &[&str]| length(&headers(header::CONTENT_LENGTH, values));
        assert_eq!(length(&["0"]), Some(0));
        assert_eq!(length(&["1048576"]), Some(1_048_576));
        for values in [
            &[][..],
            &["1", "1"],
            &[""],
            &["+1"],
            &["-1"],
            &["1 "],
            &["0x10"],
            &["1,1"],
            &["1e3"],
            &["99999999999999999999999"],
        ] {
            assert_eq!(length(values), None, "{values:?}");
        }
        let format = |values: &[&str]| format(&headers(header::CONTENT_TYPE, values));
        assert_eq!(format(&["image/png"]), Some(Format::Png));
        for values in [
            &[][..],
            &["image/png", "image/png"],
            &["image/png", "text/html"],
            &["text/html"],
            &["image/*"],
        ] {
            assert_eq!(format(values), None, "{values:?}");
        }
    }
}
