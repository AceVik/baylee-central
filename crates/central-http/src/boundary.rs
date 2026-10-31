use std::{error::Error, future::Future, sync::Arc, time::Duration};

use axum::{
    Router,
    body::{Body, to_bytes},
    extract::{Request, State},
    http::{HeaderValue, Method, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use central_api::reports::MAX_RECORD_GZIP;

struct Boundary {
    authority: HeaderValue,
    request_bytes: usize,
}

/// How long any request may take, its body included.
const REQUEST_TIME: Duration = Duration::from_secs(5);
/// How long a report part's upload may take.
pub(crate) const UPLOAD_TIME: Duration = Duration::from_secs(60);

/// A report part's upload (`PUT /v1/reports/{id}/{part}`, `native.rs`).
fn upload(request: &Request) -> bool {
    request.method() == Method::PUT && request.uri().path().starts_with("/v1/reports/")
}

pub(crate) fn router(authority: HeaderValue, request_bytes: usize) -> Router {
    with_routes(authority, request_bytes, Router::new())
}

pub(crate) fn with_routes(authority: HeaderValue, request_bytes: usize, routes: Router) -> Router {
    let boundary = Arc::new(Boundary {
        authority,
        request_bytes,
    });
    Router::new()
        .merge(routes)
        .route("/health/live", get(|| async { "alive\n" }))
        .route(
            "/health/ready",
            get(|| async { (StatusCode::SERVICE_UNAVAILABLE, "not ready\n") }),
        )
        .fallback(|| async { (StatusCode::NOT_FOUND, "not found\n") })
        // Its own time limit per request (a report part's upload is longer).
        .layer(middleware::from_fn_with_state(boundary, inspect))
        // Outermost: timeout and rejection responses also receive these headers.
        .layer(middleware::from_fn(headers))
}

async fn inspect(State(state): State<Arc<Boundary>>, request: Request, next: Next) -> Response {
    let hosts = request.headers().get_all(header::HOST);
    if hosts.iter().count() != 1 || hosts.iter().next() != Some(&state.authority) {
        return (StatusCode::BAD_REQUEST, "invalid request\n").into_response();
    }
    if request.uri().scheme().is_some() || request.uri().authority().is_some() {
        return (StatusCode::BAD_REQUEST, "invalid request\n").into_response();
    }
    // Native API only; no CORS, cookies, forwarded identity or browser origins.
    // The website has its own listener and reviewed policy (`browser.rs`).
    if request.headers().contains_key(header::ORIGIN)
        || request.headers().contains_key("sec-fetch-site")
    {
        return (StatusCode::FORBIDDEN, "request not permitted\n").into_response();
    }
    if !upload(&request) {
        return timed(REQUEST_TIME, bounded(request, state.request_bytes, next)).await;
    }
    // A part's upload declares its length, within the largest part; its
    // handler checks the credential and the announcement before it reads
    // the body (exactly the announced length), so nothing is read here.
    let declared = request
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());
    match declared {
        None => (StatusCode::LENGTH_REQUIRED, "request rejected\n").into_response(),
        Some(length) if length > u64::from(MAX_RECORD_GZIP) => {
            (StatusCode::PAYLOAD_TOO_LARGE, "request rejected\n").into_response()
        }
        Some(_) => timed(UPLOAD_TIME, next.run(request)).await,
    }
}

async fn timed(limit: Duration, response: impl Future<Output = Response>) -> Response {
    tokio::time::timeout(limit, response)
        .await
        .unwrap_or_else(|_| (StatusCode::REQUEST_TIMEOUT, "request timed out\n").into_response())
}

/// Read the whole body, at most `limit` bytes, before `next` runs.
pub(crate) async fn bounded(request: Request, limit: usize, next: Next) -> Response {
    // Reject declared oversize before waiting for an attacker to send a body.
    // Streaming/chunked bodies are still bounded independently below.
    if request
        .headers()
        .get(header::CONTENT_LENGTH)
        .is_some_and(|value| {
            value
                .to_str()
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
                .is_none_or(|length| length > limit as u64)
        })
    {
        return (StatusCode::PAYLOAD_TOO_LARGE, "request rejected\n").into_response();
    }
    let (parts, body) = request.into_parts();
    let bytes = match to_bytes(body, limit).await {
        Ok(bytes) => bytes,
        Err(error) => {
            let status = if error
                .source()
                .is_some_and(<dyn Error>::is::<http_body_util::LengthLimitError>)
            {
                StatusCode::PAYLOAD_TOO_LARGE
            } else {
                StatusCode::BAD_REQUEST
            };
            return (status, "request rejected\n").into_response();
        }
    };
    next.run(Request::from_parts(parts, Body::from(bytes)))
        .await
}

pub(crate) async fn headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    for (name, value) in [
        ("cache-control", "no-store"),
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "no-referrer"),
        (
            "content-security-policy",
            "default-src 'none'; frame-ancestors 'none'; base-uri 'none'",
        ),
        ("x-frame-options", "DENY"),
        (
            "permissions-policy",
            "camera=(), microphone=(), geolocation=()",
        ),
    ] {
        headers.insert(name, HeaderValue::from_static(value));
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower::ServiceExt;

    fn app() -> Router {
        router(HeaderValue::from_static("127.0.0.1:28780"), 1024)
    }

    fn request(path: &str) -> axum::http::request::Builder {
        Request::builder()
            .uri(path)
            .header(header::HOST, "127.0.0.1:28780")
    }

    /// The boundary with a stand-in upload route that says whether the body
    /// was still unread when it ran (the handler, not the boundary, reads it).
    fn uploads() -> Router {
        with_routes(
            HeaderValue::from_static("127.0.0.1:28780"),
            1024,
            Router::new().route(
                "/v1/reports/{report}/{part}",
                axum::routing::put(|request: Request| async move {
                    let unread = to_bytes(request.into_body(), 4096).await.map(|b| b.len());
                    format!("read here: {unread:?}")
                }),
            ),
        )
    }

    #[tokio::test]
    async fn an_upload_declares_its_length_within_the_largest_part_and_is_read_by_its_handler() {
        let put = |length: Option<&str>| {
            let mut builder =
                request("/v1/reports/0192d0c57b3e7a1f8c2d3e4f5a6b7c8d/record").method("PUT");
            if let Some(length) = length {
                builder = builder.header(header::CONTENT_LENGTH, length);
            }
            builder.body(Body::from(vec![0x1f; 2048])).unwrap()
        };
        let over = (u64::from(MAX_RECORD_GZIP) + 1).to_string();
        assert_eq!(
            uploads().oneshot(put(None)).await.unwrap().status(),
            StatusCode::LENGTH_REQUIRED
        );
        assert_eq!(
            uploads().oneshot(put(Some(&over))).await.unwrap().status(),
            StatusCode::PAYLOAD_TOO_LARGE
        );
        assert_eq!(
            uploads()
                .oneshot(put(Some("not a number")))
                .await
                .unwrap()
                .status(),
            StatusCode::LENGTH_REQUIRED
        );
        // over the 1 KiB every other request has, and the boundary leaves it to the handler
        let response = uploads().oneshot(put(Some("2048"))).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let said = to_bytes(response.into_body(), 1024).await.unwrap();
        assert_eq!(&said[..], b"read here: Ok(2048)");
        // every other path keeps the small bound: a POST of the same size is refused
        let post = request("/v1/native")
            .method("POST")
            .header(header::CONTENT_LENGTH, "2048")
            .body(Body::from(vec![0; 2048]))
            .unwrap();
        assert_eq!(
            uploads().oneshot(post).await.unwrap().status(),
            StatusCode::PAYLOAD_TOO_LARGE
        );
    }

    #[tokio::test]
    async fn liveness_is_not_readiness_or_authentication() {
        for (path, status) in [
            ("/health/live", StatusCode::OK),
            ("/health/ready", StatusCode::SERVICE_UNAVAILABLE),
            ("/auth/login", StatusCode::NOT_FOUND),
            ("/v1/training", StatusCode::NOT_FOUND),
        ] {
            let response = app()
                .oneshot(request(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), status);
            assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
            assert_eq!(response.headers()["x-content-type-options"], "nosniff");
            assert!(response.headers().get(header::SET_COOKIE).is_none());
        }
    }

    #[tokio::test]
    async fn rejects_missing_foreign_and_duplicate_host_headers() {
        for hosts in [
            vec![],
            vec!["attacker.invalid"],
            vec!["127.0.0.1:28780", "attacker.invalid"],
            vec!["127.0.0.1:28780", "127.0.0.1:28780"],
        ] {
            let mut builder = Request::builder().uri("/health/live");
            for host in hosts {
                builder = builder.header(header::HOST, host);
            }
            let response = app()
                .oneshot(builder.body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
            assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        }
    }

    #[tokio::test]
    async fn rejects_absolute_uri_and_browser_origins() {
        let absolute = request("http://attacker.invalid/health/live")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            app().oneshot(absolute).await.unwrap().status(),
            StatusCode::BAD_REQUEST
        );
        for (name, value) in [
            ("origin", "null"),
            ("origin", "https://attacker.invalid"),
            ("sec-fetch-site", "same-origin"),
        ] {
            let request = request("/health/live")
                .header(name, value)
                .body(Body::empty())
                .unwrap();
            assert_eq!(
                app().oneshot(request).await.unwrap().status(),
                StatusCode::FORBIDDEN
            );
        }
    }

    #[tokio::test]
    async fn bounds_body_even_without_content_length_and_does_not_echo_it() {
        let canary = "SECRET_PASSWORD_CANARY";
        let body = canary.repeat(100);
        let response = app()
            .oneshot(request("/health/live").body(Body::from(body)).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        let bytes = to_bytes(response.into_body(), 1024).await.unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains(canary));
    }

    #[tokio::test]
    async fn never_accepts_identity_from_headers() {
        let request = request("/admin")
            .header("x-baylee-admin", "owner")
            .header("authorization", "Bearer SECRET_TOKEN_CANARY")
            .body(Body::empty())
            .unwrap();
        let response = app().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let bytes = to_bytes(response.into_body(), 1024).await.unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("CANARY"));
    }
}
