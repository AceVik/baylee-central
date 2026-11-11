use std::{
    error::Error,
    future::Future,
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
    time::Duration,
};

use super::{
    browser::Peer,
    in_flight::{Client, InFlight, forwarded},
};
use axum::{
    Router,
    body::{Body, Bytes, HttpBody, to_bytes},
    extract::{Request, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode, header, request::Parts},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use central_api::reports::MAX_RECORD_GZIP;
use http_body_util::{BodyExt, LengthLimitError, Limited};
use tokio::time::Instant;

struct Boundary {
    authority: HeaderValue,
    request_bytes: usize,
    /// The places of bodies that are late or large ([`HEAVY`]).
    heavy: InFlight,
    /// [`GRACE`], shorter in a test.
    grace: Duration,
    /// [`REQUEST_TIME`], shorter in a test.
    request_time: Duration,
}

impl Boundary {
    /// The boundary of a listener that takes `connections` at once.
    fn new(authority: HeaderValue, request_bytes: usize, connections: usize) -> Self {
        let places = (connections / HEAVY_SHARE).max(1);
        Self {
            authority,
            request_bytes,
            // every address in the table holds a place, so no more addresses than places
            heavy: InFlight::new(places, places),
            grace: GRACE,
            request_time: REQUEST_TIME,
        }
    }
}

/// How long any request may take, its body included.
const REQUEST_TIME: Duration = Duration::from_secs(5);
/// How long a report part's upload may take.
pub(crate) const UPLOAD_TIME: Duration = Duration::from_secs(60);
/// How long a body has to arrive before it is a late one. A client sends
/// its body with its headers or right behind them, so a body that is not
/// there after this is slow or withheld.
const GRACE: Duration = Duration::from_millis(250);
/// The largest body that may arrive without a place: the browser listener's
/// largest. A longer one, or one of unknown length, is a large one.
const LIGHT_BYTES: u64 = 65_536;
/// Late or large bodies one client address may have in flight at once.
const HEAVY: u8 = 4;
/// Late and large bodies together hold at most one in this many of the
/// listener's connections (at least one): the others stay free for
/// requests whose small body is there in time, a sign-in's among them.
const HEAVY_SHARE: usize = 4;
/// The native route's fixed answer to every failure (`native.rs`).
pub(crate) const REJECTED: &str = "account request rejected\n";

/// How a long body must keep coming: at least `bytes` in every `window`
/// until it is whole.
#[derive(Clone, Copy)]
pub(crate) struct Pace {
    pub(crate) window: Duration,
    pub(crate) bytes: usize,
}

/// A report part's upload: 16 KiB every five seconds, a line of about
/// 26 kbit/s, far below what its 60 seconds ask of a large part.
pub(crate) const UPLOAD_PACE: Pace = Pace {
    window: Duration::from_secs(5),
    bytes: 16_384,
};

/// A report part's upload (`PUT /v1/reports/{id}/{part}`, `native.rs`).
fn upload(request: &Request) -> bool {
    request.method() == Method::PUT && request.uri().path().starts_with("/v1/reports/")
}

pub(crate) fn router(authority: HeaderValue, request_bytes: usize, connections: usize) -> Router {
    with_routes(authority, request_bytes, connections, Router::new())
}

/// `routes` behind the boundary of a listener that takes `connections` at once.
pub(crate) fn with_routes(
    authority: HeaderValue,
    request_bytes: usize,
    connections: usize,
    routes: Router,
) -> Router {
    guarded(
        Arc::new(Boundary::new(authority, request_bytes, connections)),
        routes,
    )
}

fn guarded(boundary: Arc<Boundary>, routes: Router) -> Router {
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
        return timed(state.request_time, admitted(&state, request, next)).await;
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

pub(crate) fn timed_out() -> Response {
    (StatusCode::REQUEST_TIMEOUT, "request timed out\n").into_response()
}

async fn timed(limit: Duration, response: impl Future<Output = Response>) -> Response {
    tokio::time::timeout(limit, response)
        .await
        .unwrap_or_else(|_| timed_out())
}

/// The address a late or large body's place is counted by: the last
/// `X-Forwarded-For` hop when the peer is the local proxy, which appended
/// it, else the peer itself. It shares the places out and does nothing
/// else: no identity, no credential and no budget follows from it. A
/// client beyond the proxy cannot choose it (the proxy's hop is the last),
/// and a local process that sends its own can take no more than the
/// places of all addresses together.
fn client(parts: &Parts) -> Client {
    match parts.extensions.get::<Peer>() {
        Some(Peer(peer)) if peer.ip().is_loopback() => {
            forwarded(&parts.headers).unwrap_or_else(|| Client::from(peer.ip()))
        }
        Some(Peer(peer)) => Client::from(peer.ip()),
        // A request the server did not accept itself (a test's).
        None => Client::from(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
    }
}

/// Read the whole body, at most the boundary's bytes, before `next` runs.
///
/// A body of at most [`LIGHT_BYTES`] that is there within the grace needs
/// nothing more. One that is not, or a larger one, needs one of its
/// address's [`HEAVY`] places among the boundary's, or the request is
/// refused as busy before another byte is waited for; so bodies that are
/// withheld hold no more than their share of the listener's connections,
/// each for the request's time at most. A late body gives its place back
/// when it has arrived, a large one when its request ends, however it
/// ends: the places bound the large bodies in memory too.
async fn admitted(state: &Boundary, request: Request, next: Next) -> Response {
    let (parts, body) = request.into_parts();
    // Reject declared oversize before waiting for an attacker to send a body.
    // Streaming/chunked bodies are still bounded independently below.
    if oversize(&parts.headers, state.request_bytes) {
        return (StatusCode::PAYLOAD_TOO_LARGE, "request rejected\n").into_response();
    }
    let client = client(&parts);
    let large = body
        .size_hint()
        .upper()
        .is_none_or(|bytes| bytes > LIGHT_BYTES);
    let reading = collect(body, state.request_bytes);
    tokio::pin!(reading);
    let in_time = if large {
        None
    } else {
        tokio::time::timeout(state.grace, &mut reading).await.ok()
    };
    let (read, place) = if let Some(read) = in_time {
        (read, None)
    } else {
        let Some(place) = state.heavy.admit(client, HEAVY) else {
            return (StatusCode::TOO_MANY_REQUESTS, REJECTED).into_response();
        };
        (reading.await, Some(place))
    };
    let bytes = match read {
        Ok(bytes) => bytes,
        Err(status) => return (status, "request rejected\n").into_response(),
    };
    let place = place.filter(|_| large);
    let response = next
        .run(Request::from_parts(parts, Body::from(bytes)))
        .await;
    drop(place);
    response
}

/// Whether `headers` declare a body longer than `limit`, or a length that
/// is none.
fn oversize(headers: &HeaderMap, limit: usize) -> bool {
    headers.get(header::CONTENT_LENGTH).is_some_and(|value| {
        value
            .to_str()
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .is_none_or(|length| length > limit as u64)
    })
}

/// The whole body of a request with `headers`, at most `limit` bytes, or
/// the status that refuses it (413, or 400 for a broken one).
pub(crate) async fn read(
    headers: &HeaderMap,
    body: Body,
    limit: usize,
) -> Result<Bytes, StatusCode> {
    // Reject declared oversize before waiting for an attacker to send a body.
    // Streaming/chunked bodies are still bounded independently below.
    if oversize(headers, limit) {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }
    collect(body, limit).await
}

/// The whole body, at most `limit` bytes, or 413, or 400 for a broken one.
async fn collect(body: Body, limit: usize) -> Result<Bytes, StatusCode> {
    to_bytes(body, limit).await.map_err(|error| {
        if error
            .source()
            .is_some_and(<dyn Error>::is::<LengthLimitError>)
        {
            StatusCode::PAYLOAD_TOO_LARGE
        } else {
            StatusCode::BAD_REQUEST
        }
    })
}

/// The whole body, at most `limit` bytes, as long as it keeps `pace`: a
/// window that ends with the body unfinished and fewer than the pace's
/// bytes ends it with 408, so a body that is withheld or trickles gives up
/// what it holds long before the request's own time. Also 413, and 400 for
/// a broken body.
pub(crate) async fn paced(body: Body, limit: usize, pace: Pace) -> Result<Bytes, StatusCode> {
    let mut body = Limited::new(body, limit);
    // A body in one frame is handed on as it is; a longer one is gathered
    // in one buffer, so many small frames cost no more than their bytes.
    let mut first: Option<Bytes> = None;
    let mut whole: Vec<u8> = Vec::new();
    let mut in_window = 0_usize;
    let mut window_ends = Instant::now() + pace.window;
    loop {
        // A body whose frames are always there never waits, so its window
        // is closed here as well as by the timer.
        let over = Instant::now() >= window_ends && !body.is_end_stream();
        let frame = if over {
            None
        } else {
            tokio::time::timeout_at(window_ends, body.frame())
                .await
                .ok()
        };
        let Some(frame) = frame else {
            if in_window < pace.bytes {
                return Err(StatusCode::REQUEST_TIMEOUT);
            }
            in_window = 0;
            window_ends = Instant::now() + pace.window;
            continue;
        };
        match frame {
            None => break,
            Some(Ok(frame)) => {
                let Ok(data) = frame.into_data() else {
                    continue;
                };
                if data.is_empty() {
                    continue;
                }
                in_window = in_window.saturating_add(data.len());
                if first.is_none() && whole.is_empty() {
                    first = Some(data);
                    continue;
                }
                if let Some(first) = first.take() {
                    grow(&mut whole, first.len().saturating_add(data.len()), limit);
                    whole.extend_from_slice(&first);
                }
                grow(&mut whole, data.len(), limit);
                whole.extend_from_slice(&data);
            }
            Some(Err(error)) => {
                return Err(if error.is::<LengthLimitError>() {
                    StatusCode::PAYLOAD_TOO_LARGE
                } else {
                    StatusCode::BAD_REQUEST
                });
            }
        }
    }
    Ok(first.unwrap_or_else(|| Bytes::from(whole)))
}

/// Room in `whole` for `more` bytes: its capacity doubled, never past
/// `limit` (the body's bound, which what is read never exceeds).
fn grow(whole: &mut Vec<u8>, more: usize, limit: usize) {
    if whole.capacity() - whole.len() >= more {
        return;
    }
    let wanted = whole.len().saturating_add(more);
    let capacity = whole.capacity().saturating_mul(2).min(limit).max(wanted);
    whole.reserve_exact(capacity - whole.len());
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
        router(HeaderValue::from_static("127.0.0.1:28780"), 1024, 32)
    }

    fn request(path: &str) -> axum::http::request::Builder {
        Request::builder()
            .uri(path)
            .header(header::HOST, "127.0.0.1:28780")
    }

    /// A body fed by its test: what is sent arrives, the sender dropped is
    /// the body's end, and nothing sent is a body withheld. `declared` is
    /// the length its request announced, `None` for a body of unknown length.
    struct Fed {
        frames: tokio::sync::mpsc::UnboundedReceiver<Result<Bytes, std::io::Error>>,
        declared: Option<u64>,
    }

    impl hyper::body::Body for Fed {
        type Data = Bytes;
        type Error = std::io::Error;

        fn poll_frame(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Option<Result<hyper::body::Frame<Bytes>, Self::Error>>> {
            self.frames
                .poll_recv(cx)
                .map(|frame| frame.map(|frame| frame.map(hyper::body::Frame::data)))
        }

        fn size_hint(&self) -> hyper::body::SizeHint {
            self.declared
                .map_or_else(hyper::body::SizeHint::default, |bytes| {
                    hyper::body::SizeHint::with_exact(bytes)
                })
        }
    }

    /// A body whose frames are always there and bring nothing.
    struct Hollow;
    impl hyper::body::Body for Hollow {
        type Data = Bytes;
        type Error = std::io::Error;

        fn poll_frame(
            self: std::pin::Pin<&mut Self>,
            _: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Option<Result<hyper::body::Frame<Bytes>, Self::Error>>> {
            std::task::Poll::Ready(Some(Ok(hyper::body::Frame::data(Bytes::new()))))
        }
    }

    type Feed = tokio::sync::mpsc::UnboundedSender<Result<Bytes, std::io::Error>>;

    fn fed(declared: Option<u64>) -> (Feed, Body) {
        let (feed, frames) = tokio::sync::mpsc::unbounded_channel();
        (feed, Body::new(Fed { frames, declared }))
    }

    const TEST_GRACE: Duration = Duration::from_millis(40);
    const TEST_TIME: Duration = Duration::from_millis(600);
    const LOOPBACK: &str = "127.0.0.1:50000";

    /// A boundary for a listener of `connections`, its times shortened, in
    /// front of a route that answers with its body's length; and the
    /// boundary itself, so a test can see its places.
    fn places(connections: usize) -> (Arc<Boundary>, Router) {
        places_before(
            connections,
            Router::new().route(
                "/echo",
                axum::routing::post(|body: Bytes| async move { body.len().to_string() }),
            ),
        )
    }

    fn places_before(connections: usize, routes: Router) -> (Arc<Boundary>, Router) {
        let mut boundary = Boundary::new(
            HeaderValue::from_static("127.0.0.1:28780"),
            1_048_576,
            connections,
        );
        boundary.grace = TEST_GRACE;
        boundary.request_time = TEST_TIME;
        let boundary = Arc::new(boundary);
        (boundary.clone(), guarded(boundary, routes))
    }

    fn client_of(address: &str) -> Client {
        Client::from(address.parse::<IpAddr>().unwrap())
    }

    /// A request as the local proxy passes it on for the client at `address`.
    fn proxied(address: &str, body: Body) -> Request {
        request("/echo")
            .method("POST")
            .header("x-forwarded-for", address)
            .extension(Peer(LOOPBACK.parse().unwrap()))
            .body(body)
            .unwrap()
    }

    /// The request sent in the background, its status when it ends.
    fn sent(app: &Router, request: Request) -> tokio::task::JoinHandle<StatusCode> {
        let app = app.clone();
        tokio::spawn(async move { app.oneshot(request).await.unwrap().status() })
    }

    /// A request of `address` whose small body is announced and withheld.
    fn withheld(app: &Router, address: &str) -> (Feed, tokio::task::JoinHandle<StatusCode>) {
        let (feed, body) = fed(Some(8));
        (feed, sent(app, proxied(address, body)))
    }

    async fn status(app: &Router, request: Request) -> StatusCode {
        app.clone().oneshot(request).await.unwrap().status()
    }

    async fn until(what: impl Fn() -> bool) {
        tokio::time::timeout(Duration::from_secs(2), async {
            while !what() {
                tokio::time::sleep(Duration::from_millis(2)).await;
            }
        })
        .await
        .expect("in time");
    }

    #[tokio::test]
    async fn bodies_withheld_hold_their_address_s_places_while_others_are_served() {
        let (state, app) = places(32);
        let (stalling, other) = ("198.51.100.7", "198.51.100.8");
        let held: Vec<_> = (0..HEAVY).map(|_| withheld(&app, stalling)).collect();
        until(|| state.heavy.count(client_of(stalling)) == HEAVY).await;
        // One more of the same address is refused when its grace ends, long
        // before the request's time, with the native route's busy answer.
        let started = Instant::now();
        let (_feed, fifth) = withheld(&app, stalling);
        assert_eq!(fifth.await.unwrap(), StatusCode::TOO_MANY_REQUESTS);
        assert!(started.elapsed() < TEST_TIME / 2);
        assert_eq!(state.heavy.count(client_of(stalling)), HEAVY);
        // A body that is there needs no place: the same address and
        // another are served meanwhile.
        for address in [stalling, other] {
            let prompt = proxied(address, Body::from("12345678"));
            assert_eq!(status(&app, prompt).await, StatusCode::OK);
        }
        assert_eq!(state.heavy.count(client_of(other)), 0);
        // Another address's late body has a place of its own, and arrives.
        let (feed, late) = withheld(&app, other);
        until(|| state.heavy.count(client_of(other)) == 1).await;
        feed.send(Ok(Bytes::from_static(b"12345678"))).unwrap();
        drop(feed);
        assert_eq!(late.await.unwrap(), StatusCode::OK);
        assert_eq!(state.heavy.count(client_of(other)), 0);
        // The withheld ones end at the request's time and give their places back.
        for (_feed, waiting) in held {
            assert_eq!(waiting.await.unwrap(), StatusCode::REQUEST_TIMEOUT);
        }
        assert_eq!((state.heavy.places(), state.heavy.addresses()), (0, 0));
        let (feed, again) = withheld(&app, stalling);
        until(|| state.heavy.count(client_of(stalling)) == 1).await;
        drop(feed);
        // its body ended short of nothing the route checks: it is answered
        assert_eq!(again.await.unwrap(), StatusCode::OK);
    }

    #[tokio::test]
    async fn late_and_large_bodies_hold_a_quarter_of_the_connections_together() {
        // eight connections: two places
        let (state, app) = places(8);
        let (first, second, third) = ("198.51.100.1", "198.51.100.2", "198.51.100.3");
        let held = [withheld(&app, first), withheld(&app, second)];
        until(|| state.heavy.places() == 2).await;
        // every place is taken: a third address's late body is refused,
        let (_feed, late) = withheld(&app, third);
        assert_eq!(late.await.unwrap(), StatusCode::TOO_MANY_REQUESTS);
        // its large one at once, although all of it is there,
        let started = Instant::now();
        let large = proxied(third, Body::from(vec![b'a'; 100_000]));
        assert_eq!(status(&app, large).await, StatusCode::TOO_MANY_REQUESTS);
        assert!(started.elapsed() < TEST_GRACE);
        // and its small one that is there is served.
        let prompt = proxied(third, Body::from("12345678"));
        assert_eq!(status(&app, prompt).await, StatusCode::OK);
        assert_eq!(state.heavy.places(), 2);
        drop(held);
    }

    #[tokio::test]
    async fn a_large_body_keeps_its_place_until_its_request_ends() {
        let entered = Arc::new(tokio::sync::Notify::new());
        let leave = Arc::new(tokio::sync::Notify::new());
        let (inside, door) = (entered.clone(), leave.clone());
        let (state, app) = places_before(
            32,
            Router::new().route(
                "/echo",
                axum::routing::post(move |body: Bytes| {
                    let (inside, door) = (inside.clone(), door.clone());
                    async move {
                        inside.notify_one();
                        door.notified().await;
                        body.len().to_string()
                    }
                }),
            ),
        );
        let address = "198.51.100.7";
        // A body longer than a light one, and one of unknown length.
        let (feed, unknown) = fed(None);
        feed.send(Ok(Bytes::from_static(b"12345678"))).unwrap();
        drop(feed);
        for body in [Body::from(vec![b'a'; 100_000]), unknown] {
            let waiting = sent(&app, proxied(address, body));
            entered.notified().await;
            // read and with the service, its place still held
            assert_eq!(state.heavy.count(client_of(address)), 1);
            leave.notify_one();
            assert_eq!(waiting.await.unwrap(), StatusCode::OK);
            assert_eq!(state.heavy.places(), 0);
        }
        // A late small body's place is free once the body is there.
        let (feed, late) = withheld(&app, address);
        until(|| state.heavy.count(client_of(address)) == 1).await;
        feed.send(Ok(Bytes::from_static(b"12345678"))).unwrap();
        drop(feed);
        entered.notified().await;
        assert_eq!(state.heavy.places(), 0);
        leave.notify_one();
        assert_eq!(late.await.unwrap(), StatusCode::OK);
        // An address's large bodies are its share at most.
        let held: Vec<_> = (0..HEAVY)
            .map(|_| sent(&app, proxied(address, Body::from(vec![b'a'; 100_000]))))
            .collect();
        until(|| state.heavy.count(client_of(address)) == HEAVY).await;
        let more = proxied(address, Body::from(vec![b'a'; 100_000]));
        assert_eq!(status(&app, more).await, StatusCode::TOO_MANY_REQUESTS);
        for _ in 0..HEAVY {
            leave.notify_one();
        }
        for waiting in held {
            assert_eq!(waiting.await.unwrap(), StatusCode::OK);
        }
        assert_eq!(state.heavy.places(), 0);
    }

    #[tokio::test]
    async fn a_place_is_given_back_however_the_request_ends() {
        let (state, app) = places(32);
        let address = "198.51.100.9";
        let place = || state.heavy.count(client_of(address));
        // a late body that breaks
        let (feed, broken) = withheld(&app, address);
        until(|| place() == 1).await;
        feed.send(Err(std::io::Error::other("synthetic"))).unwrap();
        assert_eq!(broken.await.unwrap(), StatusCode::BAD_REQUEST);
        assert_eq!(place(), 0);
        // one of unknown length that grows past the bound
        let (feed, over) = fed(None);
        let over = sent(&app, proxied(address, over));
        until(|| place() == 1).await;
        feed.send(Ok(Bytes::from(vec![b'a'; 1_048_577]))).unwrap();
        assert_eq!(over.await.unwrap(), StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(place(), 0);
        // one whose connection goes away
        let (_feed, gone) = withheld(&app, address);
        until(|| place() == 1).await;
        gone.abort();
        until(|| place() == 0).await;
        // one never sent: the request's time
        let (_feed, never) = withheld(&app, address);
        assert_eq!(never.await.unwrap(), StatusCode::REQUEST_TIMEOUT);
        assert_eq!((state.heavy.places(), state.heavy.addresses()), (0, 0));
    }

    #[tokio::test]
    async fn the_address_is_the_proxy_s_last_hop_or_else_the_peer() {
        let parts = |peer: Option<&str>, forwarded: &[&str]| {
            let mut builder = request("/echo");
            for value in forwarded {
                builder = builder.header("x-forwarded-for", *value);
            }
            if let Some(peer) = peer {
                builder = builder.extension(Peer(peer.parse().unwrap()));
            }
            builder.body(Body::empty()).unwrap().into_parts().0
        };
        let local = Some(LOOPBACK);
        for (peer, forwarded, expected) in [
            // the proxy's own hop, the last, whatever a client wrote before it
            (local, &["198.51.100.1"][..], "198.51.100.1"),
            (local, &["203.0.113.9, 198.51.100.1"], "198.51.100.1"),
            (local, &["198.51.100.1", "203.0.113.9"], "203.0.113.9"),
            // none, or none that is an address: the peer
            (local, &[], "127.0.0.1"),
            (local, &["unknown"], "127.0.0.1"),
            (Some("[::1]:50000"), &[], "::1"),
            // a peer that is not the local proxy is not believed
            (Some("192.0.2.5:50000"), &["198.51.100.1"], "192.0.2.5"),
            // a request the server did not accept itself
            (None, &["198.51.100.1"], "0.0.0.0"),
        ] {
            assert_eq!(
                client(&parts(peer, forwarded)),
                client_of(expected),
                "{peer:?} {forwarded:?}"
            );
        }
        // Without a forwarded address a request is served as before, and its
        // late body counts by its peer.
        let (state, app) = places(32);
        let direct = |body: Body| {
            request("/echo")
                .method("POST")
                .extension(Peer(LOOPBACK.parse().unwrap()))
                .body(body)
                .unwrap()
        };
        assert_eq!(
            status(&app, direct(Body::from("12345678"))).await,
            StatusCode::OK
        );
        let (feed, body) = fed(Some(8));
        let late = sent(&app, direct(body));
        until(|| state.heavy.count(client_of("127.0.0.1")) == 1).await;
        drop(feed);
        assert_eq!(late.await.unwrap(), StatusCode::OK);
        assert_eq!(state.heavy.places(), 0);
    }

    const TEST_PACE: Pace = Pace {
        window: Duration::from_millis(60),
        bytes: 1_000,
    };

    #[tokio::test]
    async fn a_slow_but_steady_body_is_read_whole() {
        let (feed, body) = fed(Some(20_000));
        let reading = tokio::spawn(paced(body, 20_000, TEST_PACE));
        // twice the pace: a thousand bytes every half window, for ten windows
        for index in 0..20_u8 {
            feed.send(Ok(Bytes::from(vec![index; 1_000]))).unwrap();
            tokio::time::sleep(TEST_PACE.window / 2).await;
        }
        drop(feed);
        let whole = reading.await.unwrap().unwrap();
        assert_eq!(whole.len(), 20_000);
        assert!(
            whole
                .chunks(1_000)
                .zip(0_u8..)
                .all(|(chunk, index)| chunk.iter().all(|byte| *byte == index))
        );
        // A tail shorter than the pace's bytes ends the body, not the pace.
        let (feed, body) = fed(Some(1_200));
        let reading = tokio::spawn(paced(body, 1_200, TEST_PACE));
        feed.send(Ok(Bytes::from(vec![1; 1_000]))).unwrap();
        tokio::time::sleep(TEST_PACE.window / 2).await;
        feed.send(Ok(Bytes::from(vec![2; 200]))).unwrap();
        drop(feed);
        assert_eq!(reading.await.unwrap().unwrap().len(), 1_200);
        // One frame is handed on as it is.
        let whole = paced(Body::from("12345678"), 8, TEST_PACE).await.unwrap();
        assert_eq!(&whole[..], b"12345678");
    }

    #[tokio::test]
    async fn a_body_that_stops_or_trickles_ends_at_its_window() {
        // withheld from the start
        let started = Instant::now();
        let (_feed, body) = fed(Some(20_000));
        assert_eq!(
            paced(body, 20_000, TEST_PACE).await.unwrap_err(),
            StatusCode::REQUEST_TIMEOUT
        );
        assert!(started.elapsed() >= TEST_PACE.window);
        assert!(started.elapsed() < TEST_PACE.window * 4);
        // one that stops after a window that kept the pace
        let (feed, body) = fed(Some(20_000));
        let reading = tokio::spawn(paced(body, 20_000, TEST_PACE));
        feed.send(Ok(Bytes::from(vec![0; 1_500]))).unwrap();
        assert_eq!(
            reading.await.unwrap().unwrap_err(),
            StatusCode::REQUEST_TIMEOUT
        );
        // one that trickles: a byte at a time, never the pace's bytes
        let (feed, body) = fed(Some(20_000));
        let reading = tokio::spawn(paced(body, 20_000, TEST_PACE));
        let started = Instant::now();
        while !reading.is_finished() {
            let _ = feed.send(Ok(Bytes::from_static(b"a")));
            tokio::time::sleep(Duration::from_millis(5)).await;
            assert!(started.elapsed() < TEST_PACE.window * 8, "still read");
        }
        assert_eq!(
            reading.await.unwrap().unwrap_err(),
            StatusCode::REQUEST_TIMEOUT
        );
    }

    #[tokio::test]
    async fn a_paced_body_keeps_its_bound_and_a_broken_one_is_refused() {
        let over = paced(Body::from(vec![0; 2_000]), 1_999, TEST_PACE).await;
        assert_eq!(over.unwrap_err(), StatusCode::PAYLOAD_TOO_LARGE);
        let (feed, body) = fed(None);
        feed.send(Ok(Bytes::from(vec![0; 1_000]))).unwrap();
        feed.send(Err(std::io::Error::other("synthetic"))).unwrap();
        let broken = paced(body, 4_000, TEST_PACE).await;
        assert_eq!(broken.unwrap_err(), StatusCode::BAD_REQUEST);
        // Frames that are always there and bring nothing never let the
        // timer run; the window still ends.
        let started = Instant::now();
        let hollow = paced(Body::new(Hollow), 4_000, TEST_PACE).await;
        assert_eq!(hollow.unwrap_err(), StatusCode::REQUEST_TIMEOUT);
        assert!(started.elapsed() < TEST_PACE.window * 4);
    }

    /// The boundary with a stand-in upload route that says whether the body
    /// was still unread when it ran (the handler, not the boundary, reads it).
    fn uploads() -> Router {
        with_routes(
            HeaderValue::from_static("127.0.0.1:28780"),
            1024,
            32,
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
