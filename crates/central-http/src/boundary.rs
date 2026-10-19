use std::{error::Error, sync::Arc, time::Duration};

use axum::{
    Router,
    body::{Body, to_bytes},
    extract::{Request, State},
    http::{HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use tower_http::timeout::TimeoutLayer;

#[derive(Clone)]
struct Boundary {
    authority: HeaderValue,
    request_bytes: usize,
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
        .layer(middleware::from_fn_with_state(boundary, inspect))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(5),
        ))
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
    // Native development API only; no CORS, cookies, forwarded identity or browser origins.
    // Same-origin website integration requires an explicit reviewed policy later.
    if request.headers().contains_key(header::ORIGIN)
        || request.headers().contains_key("sec-fetch-site")
    {
        return (StatusCode::FORBIDDEN, "request not permitted\n").into_response();
    }
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
                .is_none_or(|length| length > state.request_bytes as u64)
        })
    {
        return (StatusCode::PAYLOAD_TOO_LARGE, "request rejected\n").into_response();
    }
    let (parts, body) = request.into_parts();
    let bytes = match to_bytes(body, state.request_bytes).await {
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

async fn headers(request: Request, next: Next) -> Response {
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
