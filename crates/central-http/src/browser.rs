//! The website's browser listener (`docs/web-accounts.md`): one route,
//! `POST /api/v1`, behind `baylee.app`'s Caddy, which passes `/api/*` with the
//! `Host` header unchanged. Default-deny: every refusal here happens before the
//! identity service is called, and no CORS header is ever sent. The session
//! token travels only in the `__Host-central` cookie.
use axum::{
    Router,
    body::to_bytes,
    extract::{Request as HttpRequest, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode, Uri, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::post,
};
use central_api::{Error, Token, wire::Request};
use central_identity::{BrowserReply, IdentityService, browser_operation};
use std::{
    collections::BTreeMap,
    future::Future,
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tower_http::timeout::TimeoutLayer;
use zeroize::Zeroizing;

/// The one site a browser listener serves: the `Host` its proxy passes on and
/// the page's exact `Origin`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserOrigin {
    host: String,
    origin: String,
}

impl BrowserOrigin {
    /// `https://baylee.app`, the deployed service's only site.
    #[must_use]
    pub fn live() -> Self {
        Self {
            host: "baylee.app".into(),
            origin: "https://baylee.app".into(),
        }
    }

    /// A page served from an explicit numeric loopback `http` origin with a
    /// port, for local runs of the development launcher. Every other check of
    /// the listener stays as it is.
    ///
    /// # Errors
    /// Rejects what [`central_api::Endpoint::development`] rejects: another
    /// scheme, a DNS name, a remote address, a missing port, credentials, a
    /// path, a query or a fragment.
    pub fn development(value: &str) -> Result<Self, Error> {
        let endpoint = central_api::Endpoint::development(value)?;
        let origin = endpoint.origin().to_owned();
        let host = origin
            .strip_prefix("http://")
            .ok_or(Error::Rejected)?
            .to_owned();
        Ok(Self { host, origin })
    }

    /// The exact `Origin` served.
    #[must_use]
    pub fn origin(&self) -> &str {
        &self.origin
    }
}
/// The session cookie's name; `__Host-` makes the browser refuse it unless it
/// is `Secure`, has `Path=/` and no `Domain`, so no other site sets it.
const COOKIE: &str = "__Host-central";
/// The cookie's lifetime: the session's cap, 12 hours after the sign-in that
/// set it. The session itself expires sooner unless it is renewed
/// (`RenewSession`), and an expired session's cookie finds no session, so a
/// renewal never has to set the cookie again.
const COOKIE_SECONDS: u32 = 43_200;
/// Largest body: the browser operations are small (the initial document is a
/// few hundred bytes sealed).
pub const REQUEST_BYTES: usize = 65_536;

/// The peer of a connection, attached by the server to each of its requests.
#[derive(Clone, Copy)]
pub(crate) struct Peer(pub(crate) SocketAddr);

/// What answers browser requests: the identity service, or a test's stand-in.
pub(crate) trait Accounts: Send + Sync + 'static {
    fn browser(
        &self,
        request: Request,
        cookie: Option<&Token>,
    ) -> impl Future<Output = Result<BrowserReply, Error>> + Send;
}

impl Accounts for IdentityService {
    fn browser(
        &self,
        request: Request,
        cookie: Option<&Token>,
    ) -> impl Future<Output = Result<BrowserReply, Error>> + Send {
        self.handle_browser(request, cookie)
    }
}

struct Browser<A> {
    accounts: Arc<A>,
    site: BrowserOrigin,
    budgets: Mutex<Budgets>,
}

pub(crate) fn router<A: Accounts>(accounts: Arc<A>, site: BrowserOrigin) -> Router {
    let state = Arc::new(Browser {
        accounts,
        site,
        budgets: Mutex::default(),
    });
    Router::new()
        .route("/api/v1", post(call::<A>))
        .fallback(|| async { (StatusCode::NOT_FOUND, "not found\n") })
        .with_state(state)
        .layer(middleware::from_fn(bounded))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(5),
        ))
        // Outermost: refusals and timeouts also receive the security headers.
        .layer(middleware::from_fn(super::boundary::headers))
}

async fn bounded(request: HttpRequest, next: Next) -> Response {
    super::boundary::bounded(request, REQUEST_BYTES, next).await
}

fn refused() -> Response {
    (StatusCode::FORBIDDEN, "request not permitted\n").into_response()
}

async fn call<A: Accounts>(State(state): State<Arc<Browser<A>>>, request: HttpRequest) -> Response {
    let (parts, body) = request.into_parts();
    let headers = &parts.headers;
    // Only the local proxy reaches this listener; its forwarded address counts.
    let local = parts
        .extensions
        .get::<Peer>()
        .is_some_and(|Peer(peer)| peer.ip().is_loopback());
    if !local || !permitted(&parts.uri, headers, &state.site) {
        return refused();
    }
    // Already read whole, within the bound, by `bounded`.
    let Ok(body) = to_bytes(body, REQUEST_BYTES).await else {
        return (StatusCode::PAYLOAD_TOO_LARGE, "request rejected\n").into_response();
    };
    let Some(client) = client(headers) else {
        return (StatusCode::BAD_REQUEST, "invalid request\n").into_response();
    };
    let Ok(request) = central_api::decode::<Request>(&body) else {
        return failure(Error::Rejected);
    };
    if !browser_operation(&request) {
        return refused();
    }
    // The guard is gone before anything is awaited.
    let charged = match state.budgets.lock() {
        Ok(mut budgets) => budgets.charge(&request, client, Instant::now()),
        Err(_) => return failure(Error::Unavailable),
    };
    if !charged {
        return failure(Error::Busy);
    }
    let logout = matches!(request, Request::Logout);
    let deletion = matches!(request, Request::DeleteAccountFinish { .. });
    let cookie = cookie(headers);
    let result = state.accounts.browser(request, cookie.as_ref()).await;
    // A deleted account's session is gone with it.
    let deleted = deletion && result.is_ok();
    let mut response = match result.and_then(|reply| {
        let bytes = central_api::encode(&reply.response)?;
        Ok((bytes, reply.session))
    }) {
        Ok((bytes, session)) => {
            let mut response =
                ([(header::CONTENT_TYPE, "application/json")], bytes).into_response();
            if let Some(token) = session {
                set_cookie(&mut response, Some(&token));
            }
            response
        }
        Err(error) => failure(error),
    };
    if logout || deleted {
        // Signing out always clears the cookie, also when the session was gone.
        set_cookie(&mut response, None);
    }
    response
}

/// The fixed public vocabulary of the native route; nothing of the request or
/// of the cause reaches the page.
fn failure(error: Error) -> Response {
    let status = match error {
        Error::Rejected => StatusCode::UNAUTHORIZED,
        Error::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        Error::Conflict => StatusCode::CONFLICT,
        Error::Busy => StatusCode::TOO_MANY_REQUESTS,
        Error::TermsRequired => StatusCode::FORBIDDEN,
    };
    (status, "account request rejected\n").into_response()
}

/// Exactly one value, equal byte for byte to `expected`.
fn exactly(headers: &HeaderMap, name: impl header::AsHeaderName + Copy, expected: &str) -> bool {
    let mut values = headers.get_all(name).iter();
    values
        .next()
        .is_some_and(|value| value.as_bytes() == expected.as_bytes())
        && values.next().is_none()
}

/// The request comes from the website's own page, in a way no other site's
/// page can produce: the exact `Host` and `Origin`, same-origin fetch metadata
/// when sent, JSON, the custom header that forces a cross-origin preflight
/// (which no CORS answer ever grants), and no bearer.
fn permitted(uri: &Uri, headers: &HeaderMap, site: &BrowserOrigin) -> bool {
    let fetch_site = HeaderName::from_static("sec-fetch-site");
    let mut sites = headers.get_all(&fetch_site).iter();
    let same_origin = match (sites.next(), sites.next()) {
        (None, _) => true,
        (Some(value), None) => value.as_bytes() == b"same-origin",
        (Some(_), Some(_)) => false,
    };
    uri.scheme().is_none()
        && uri.authority().is_none()
        && exactly(headers, &header::HOST, &site.host)
        && exactly(headers, &header::ORIGIN, &site.origin)
        && same_origin
        && exactly(headers, &header::CONTENT_TYPE, "application/json")
        && exactly(headers, &HeaderName::from_static("x-baylee-central"), "1")
        && !headers.contains_key(header::AUTHORIZATION)
}

/// The client's address: the last `X-Forwarded-For` hop, which the local proxy
/// appended. IPv6 counts by its /64, the block one subscriber usually holds.
fn client(headers: &HeaderMap) -> Option<Client> {
    let last = headers.get_all("x-forwarded-for").iter().next_back()?;
    let hop = last
        .to_str()
        .ok()?
        .rsplit(',')
        .next()?
        .trim_matches([' ', '\t']);
    Some(Client::from(hop.parse::<IpAddr>().ok()?))
}

/// The session cookie, when exactly one well-formed `__Host-central` is sent;
/// anything else is no session.
fn cookie(headers: &HeaderMap) -> Option<Token> {
    let mut found = None;
    for value in headers.get_all(header::COOKIE) {
        for pair in value.to_str().ok()?.split(';') {
            let Some((name, value)) = pair.trim_matches(' ').split_once('=') else {
                continue;
            };
            if name == COOKIE {
                if found.is_some() {
                    return None;
                }
                found = Some(Token::from_hex(value).ok()?);
            }
        }
    }
    found
}

/// Set the session cookie, or clear it with `None`.
fn set_cookie(response: &mut Response, token: Option<&Token>) {
    let mut text = Zeroizing::new(String::with_capacity(128));
    text.push_str(COOKIE);
    text.push('=');
    if let Some(token) = token {
        text.push_str(&token.hex());
    }
    text.push_str("; Path=/; Secure; HttpOnly; SameSite=Strict; Max-Age=");
    text.push_str(&token.map_or(0, |_| COOKIE_SECONDS).to_string());
    if let Ok(mut value) = HeaderValue::from_str(&text) {
        value.set_sensitive(true);
        response.headers_mut().insert(header::SET_COOKIE, value);
    }
}

/// A client address as the budgets count it.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Client {
    V4([u8; 4]),
    V6([u8; 8]),
}

impl From<IpAddr> for Client {
    fn from(ip: IpAddr) -> Self {
        match ip.to_canonical() {
            IpAddr::V4(ip) => Self::V4(ip.octets()),
            IpAddr::V6(ip) => {
                let mut prefix = [0; 8];
                prefix.copy_from_slice(&ip.octets()[..8]);
                Self::V6(prefix)
            }
        }
    }
}

/// Most addresses one budget remembers; past it the oldest window goes, so a
/// crowd of addresses cannot lock new ones out.
const ADDRESSES: usize = 4_096;
const MINUTE: Duration = Duration::from_secs(60);
const HOUR: Duration = Duration::from_secs(3_600);

/// The per-address budgets, in memory only; an address is forgotten once its
/// window has passed.
#[derive(Default)]
struct Budgets {
    register: Book,
    discover: Book,
    login: Book,
    directory: Book,
    reset: Book,
}

impl Budgets {
    fn charge(&mut self, request: &Request, client: Client, now: Instant) -> bool {
        match request {
            Request::Register { .. } => self.register.charge(client, now, 5, HOUR),
            Request::Discover { .. } => self.discover.charge(client, now, 30, MINUTE),
            Request::LoginStart { .. } => self.login.charge(client, now, 30, MINUTE),
            // The public directory needs no session; a page reads it a few
            // times, so a minute's budget leaves room for paging.
            Request::GatewayDirectory { .. } => self.directory.charge(client, now, 60, MINUTE),
            // A reset needs a code; a few tries an hour leave room for typing it.
            Request::ResetStart { .. } => self.reset.charge(client, now, 10, HOUR),
            _ => true,
        }
    }
}

#[derive(Default)]
struct Book(BTreeMap<Client, (Instant, u32)>);

impl Book {
    fn charge(&mut self, client: Client, now: Instant, limit: u32, period: Duration) -> bool {
        self.0
            .retain(|_, (start, _)| now.duration_since(*start) < period);
        if !self.0.contains_key(&client) && self.0.len() >= ADDRESSES {
            let oldest = self
                .0
                .iter()
                .min_by_key(|(_, (start, _))| *start)
                .map(|(client, _)| *client);
            if let Some(oldest) = oldest {
                self.0.remove(&oldest);
            }
        }
        let (_, used) = self.0.entry(client).or_insert((now, 0));
        if *used >= limit {
            return false;
        }
        *used += 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use central_api::{
        terms::TermsSubject,
        wire::{Identity, Response as Answer, TicketPurpose},
    };
    use tower::ServiceExt;

    /// Records each call; answers `Done`, or a browser session for `LoginFinish`.
    #[derive(Default)]
    struct Mock {
        calls: Mutex<Vec<(String, Option<[u8; 32]>)>>,
        fail: Option<Error>,
    }

    const SESSION: [u8; 32] = [0xab; 32];

    impl Accounts for Mock {
        fn browser(
            &self,
            request: Request,
            cookie: Option<&Token>,
        ) -> impl Future<Output = Result<BrowserReply, Error>> + Send {
            self.calls
                .lock()
                .unwrap()
                .push((operation(&request), cookie.map(Token::digest)));
            let reply = match (self.fail, request) {
                (Some(error), _) => Err(error),
                (None, Request::LoginFinish { .. }) => Ok(BrowserReply {
                    response: Answer::BrowserSession {
                        identity: Identity {
                            account: [7; 16],
                            epoch: 1,
                        },
                        username: "synthetic".into(),
                        wrapped_master: vec![1; 73],
                        expires_at: 1,
                    },
                    session: Some(Token::from_secret_bytes(Zeroizing::new(SESSION))),
                }),
                (None, _) => Ok(BrowserReply {
                    response: Answer::Done,
                    session: None,
                }),
            };
            std::future::ready(reply)
        }
    }

    fn operation(request: &Request) -> String {
        let text = String::from_utf8(central_api::encode(request).unwrap()).unwrap();
        let start = text.find(r#""operation":""#).unwrap() + 13;
        text[start..].split('"').next().unwrap().to_owned()
    }

    fn body(request: &Request) -> String {
        String::from_utf8(central_api::encode(request).unwrap()).unwrap()
    }

    fn discover() -> String {
        body(&Request::Discover {
            username: "synthetic".into(),
        })
    }

    const LOOPBACK: &str = "127.0.0.1:50000";

    /// The headers the website's page sends through its proxy.
    fn base() -> Vec<(&'static str, String)> {
        [
            ("host", "baylee.app"),
            ("origin", "https://baylee.app"),
            ("sec-fetch-site", "same-origin"),
            ("content-type", "application/json"),
            ("x-baylee-central", "1"),
            ("x-forwarded-for", "198.51.100.7"),
        ]
        .into_iter()
        .map(|(name, value)| (name, value.to_owned()))
        .collect()
    }

    /// `base()` with every `name` header replaced by `values` (none: removed).
    fn with(name: &'static str, values: &[&str]) -> Vec<(&'static str, String)> {
        let mut headers: Vec<_> = base().into_iter().filter(|(n, _)| *n != name).collect();
        headers.extend(values.iter().map(|value| (name, (*value).to_owned())));
        headers
    }

    struct Sent {
        status: StatusCode,
        headers: HeaderMap,
        body: String,
    }

    async fn send_to(
        app: Router,
        method: &str,
        uri: &str,
        headers: &[(&'static str, String)],
        body: String,
        peer: Option<&str>,
    ) -> Sent {
        let mut builder = HttpRequest::builder().method(method).uri(uri);
        for (name, value) in headers {
            builder = builder.header(*name, value);
        }
        if let Some(peer) = peer {
            builder = builder.extension(Peer(peer.parse().unwrap()));
        }
        let response = app
            .oneshot(builder.body(Body::from(body)).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = to_bytes(response.into_body(), 1 << 20).await.unwrap();
        // No response of this listener ever grants a cross-origin read, and
        // every one carries the security headers.
        assert!(
            headers
                .keys()
                .all(|name| !name.as_str().starts_with("access-control-"))
        );
        assert_eq!(headers[header::CACHE_CONTROL], "no-store");
        assert_eq!(headers["x-content-type-options"], "nosniff");
        Sent {
            status,
            headers,
            body: String::from_utf8(bytes.to_vec()).unwrap(),
        }
    }

    /// One listener with its budgets, and the stand-in behind it.
    fn listener() -> (Router, Arc<Mock>) {
        let mock = Arc::new(Mock::default());
        (router(mock.clone(), BrowserOrigin::live()), mock)
    }

    async fn send(app: &Router, headers: &[(&'static str, String)], body: String) -> Sent {
        send_to(
            app.clone(),
            "POST",
            "/api/v1",
            headers,
            body,
            Some(LOOPBACK),
        )
        .await
    }

    fn calls(mock: &Mock) -> usize {
        mock.calls.lock().unwrap().len()
    }

    #[tokio::test]
    async fn the_page_s_own_request_is_served() {
        let (app, mock) = listener();
        let sent = send(&app, &base(), discover()).await;
        assert_eq!(sent.status, StatusCode::OK);
        assert_eq!(sent.body, r#"{"result":"done"}"#);
        assert!(sent.headers.get(header::SET_COOKIE).is_none());
        // Fetch metadata is optional: older browsers do not send it.
        let sent = send(&app, &with("sec-fetch-site", &[]), discover()).await;
        assert_eq!(sent.status, StatusCode::OK);
        assert_eq!(calls(&mock), 2);
    }

    #[tokio::test]
    async fn every_header_condition_refuses_before_the_service() {
        let (app, mock) = listener();
        let bearer = Token::generate().unwrap().bearer();
        let cases: Vec<(&'static str, Vec<&str>)> = vec![
            ("host", vec![]),
            ("host", vec!["evil.example"]),
            ("host", vec!["baylee.app:443"]),
            ("host", vec!["Baylee.app"]),
            ("host", vec!["baylee.app", "baylee.app"]),
            ("origin", vec![]),
            ("origin", vec!["null"]),
            ("origin", vec!["https://evil.example"]),
            ("origin", vec!["https://baylee.app/"]),
            ("origin", vec!["http://baylee.app"]),
            ("origin", vec!["https://baylee.app:443"]),
            ("origin", vec!["https://baylee.app.evil.example"]),
            ("origin", vec!["https://central.baylee.app"]),
            ("origin", vec!["https://baylee.app", "https://baylee.app"]),
            ("sec-fetch-site", vec!["cross-site"]),
            ("sec-fetch-site", vec!["same-site"]),
            ("sec-fetch-site", vec!["none"]),
            ("sec-fetch-site", vec!["same-origin", "same-origin"]),
            ("content-type", vec![]),
            ("content-type", vec!["text/plain"]),
            ("content-type", vec!["application/json; charset=utf-8"]),
            ("content-type", vec!["application/x-www-form-urlencoded"]),
            ("content-type", vec!["application/json", "application/json"]),
            ("x-baylee-central", vec![]),
            ("x-baylee-central", vec!["0"]),
            ("x-baylee-central", vec!["true"]),
            ("x-baylee-central", vec!["1", "1"]),
            ("authorization", vec![bearer.as_str()]),
        ];
        for (name, values) in &cases {
            let sent = send(&app, &with(name, values), discover()).await;
            assert_eq!(sent.status, StatusCode::FORBIDDEN, "{name}: {values:?}");
            assert_eq!(sent.body, "request not permitted\n");
            assert!(sent.headers.get(header::SET_COOKIE).is_none());
        }
        // Only the loopback proxy reaches the listener.
        for peer in [Some("203.0.113.9:4444"), Some("[2001:db8::1]:4444"), None] {
            let sent = send_to(app.clone(), "POST", "/api/v1", &base(), discover(), peer).await;
            assert_eq!(sent.status, StatusCode::FORBIDDEN, "{peer:?}");
        }
        let absolute = send_to(
            router(mock.clone(), BrowserOrigin::live()),
            "POST",
            "https://baylee.app/api/v1",
            &base(),
            discover(),
            Some(LOOPBACK),
        )
        .await;
        assert_eq!(absolute.status, StatusCode::FORBIDDEN);
        assert_eq!(calls(&mock), 0, "no refused request reached the service");
    }

    #[tokio::test]
    async fn only_the_browser_operations_and_only_the_route_are_served() {
        let (app, mock) = listener();
        let identity = Identity {
            account: [7; 16],
            epoch: 1,
        };
        for request in [
            Request::ReadDocument,
            Request::CreateObject,
            Request::ConfirmRestore { revision: 1 },
            Request::GatewayTicket {
                gateway: central_api::OFFICIAL_ORIGIN.into(),
                purpose: TicketPurpose::Admit,
            },
            Request::LegacyReserve {
                legacy_id: "x".into(),
                username: "synthetic".into(),
                display_name: None,
                tag: None,
            },
            Request::LegacyClose {
                legacy_id: "x".into(),
                identity,
            },
            Request::LegacyProfile {
                legacy_id: "x".into(),
                display_name: Some("Synthetic".into()),
                tag: 13,
            },
            Request::GatewayProfile {
                lease: Token::generate().unwrap(),
                purpose: TicketPurpose::Admit,
            },
            Request::AcceptTerms {
                subject: TermsSubject::Gateway {
                    origin: central_api::OFFICIAL_ORIGIN.into(),
                },
                version: "1".into(),
                digest: [0; 32],
            },
        ] {
            let sent = send(&app, &base(), body(&request)).await;
            assert_eq!(
                sent.status,
                StatusCode::FORBIDDEN,
                "{}",
                operation(&request)
            );
        }
        // Malformed or unknown requests are refused like the native route does.
        for text in [
            "",
            "{",
            r#"{"operation":"reset_everything"}"#,
            r#"{"operation":"discover","username":"synthetic","password":"never"}"#,
        ] {
            let sent = send(&app, &base(), text.into()).await;
            assert_eq!(sent.status, StatusCode::UNAUTHORIZED, "{text}");
            assert_eq!(sent.body, "account request rejected\n");
        }
        let oversized = send(&app, &base(), " ".repeat(REQUEST_BYTES + 1)).await;
        assert_eq!(oversized.status, StatusCode::PAYLOAD_TOO_LARGE);
        for (method, uri, status) in [
            ("GET", "/api/v1", StatusCode::METHOD_NOT_ALLOWED),
            ("OPTIONS", "/api/v1", StatusCode::METHOD_NOT_ALLOWED),
            ("POST", "/api/v2", StatusCode::NOT_FOUND),
            ("POST", "/v1/native", StatusCode::NOT_FOUND),
            ("POST", "/api/v1/", StatusCode::NOT_FOUND),
        ] {
            let sent = send_to(
                app.clone(),
                method,
                uri,
                &base(),
                discover(),
                Some(LOOPBACK),
            )
            .await;
            assert_eq!(sent.status, status, "{method} {uri}");
        }
        assert_eq!(calls(&mock), 0);
    }

    #[tokio::test]
    async fn the_client_address_is_the_proxy_s_last_hop() {
        let (app, mock) = listener();
        for values in [
            vec![],
            vec!["not-an-address"],
            vec!["198.51.100.7:443"],
            vec!["[2001:db8::1]"],
            vec!["198.51.100.7, garbage"],
            vec!["198.51.100.7", "garbage"],
            vec!["198.51.100.7,"],
        ] {
            let sent = send(&app, &with("x-forwarded-for", &values), discover()).await;
            assert_eq!(sent.status, StatusCode::BAD_REQUEST, "{values:?}");
        }
        assert_eq!(calls(&mock), 0);
        // A forged first hop changes nothing: the last one is counted.
        let register = body(&Request::Register {
            username: "synthetic".into(),
            beta_key: "CENTRAL-0000-0000-0000-0000".into(),
            display_name: None,
        });
        for forged in 0..5 {
            let hops = format!("203.0.113.{forged}, 198.51.100.20");
            let sent = send(&app, &with("x-forwarded-for", &[&hops]), register.clone()).await;
            assert_eq!(sent.status, StatusCode::OK);
        }
        let sent = send(
            &app,
            &with("x-forwarded-for", &["203.0.113.99", "198.51.100.20"]),
            register,
        )
        .await;
        assert_eq!(sent.status, StatusCode::TOO_MANY_REQUESTS);
    }

    fn hex(byte: u8) -> String {
        format!("{byte:02x}").repeat(32)
    }

    fn cookie_header(value: &str) -> Vec<(&'static str, String)> {
        let mut headers = base();
        headers.push(("cookie", value.to_owned()));
        headers
    }

    #[tokio::test]
    async fn the_session_travels_only_in_the_cookie() {
        let (app, mock) = listener();
        let finish = body(&Request::LoginFinish {
            attempt: Token::generate().unwrap(),
            message: vec![0; 32],
        });
        let sent = send(&app, &base(), finish).await;
        assert_eq!(sent.status, StatusCode::OK);
        assert_eq!(sent.headers.get_all(header::SET_COOKIE).iter().count(), 1);
        assert_eq!(
            sent.headers[header::SET_COOKIE],
            format!(
                "__Host-central={}; Path=/; Secure; HttpOnly; SameSite=Strict; Max-Age=43200",
                hex(0xab)
            )
        );
        assert!(sent.body.starts_with(r#"{"result":"browser_session""#));
        assert!(!sent.body.contains(&hex(0xab)) && !sent.body.contains("token"));
        // The cookie reaches the service as the session; anything ambiguous is none.
        let session = Token::from_secret_bytes(Zeroizing::new(SESSION)).digest();
        let status = body(&Request::SessionStatus);
        for (cookie, expected) in [
            (format!("__Host-central={}", hex(0xab)), Some(session)),
            (
                format!("theme=dark; __Host-central={}; lang=de", hex(0xab)),
                Some(session),
            ),
            (
                format!("__Host-central={}; __Host-central={}", hex(0xab), hex(0xab)),
                None,
            ),
            (format!("__Host-central={}", hex(0xab).to_uppercase()), None),
            (format!("__host-central={}", hex(0xab)), None),
            (format!("__Host-central={}", &hex(0xab)[1..]), None),
            (format!("__Host-central=\"{}\"", hex(0xab)), None),
            ("__Host-central=".to_owned(), None),
            ("central=1".to_owned(), None),
        ] {
            mock.calls.lock().unwrap().clear();
            let sent = send(&app, &cookie_header(&cookie), status.clone()).await;
            assert_eq!(sent.status, StatusCode::OK);
            assert!(sent.headers.get(header::SET_COOKIE).is_none());
            assert_eq!(mock.calls.lock().unwrap()[0].1, expected, "{cookie}");
        }
        // Two Cookie lines with the session in one of them still name one session.
        let mut headers = cookie_header("theme=dark");
        headers.push(("cookie", format!("__Host-central={}", hex(0xab))));
        mock.calls.lock().unwrap().clear();
        send(&app, &headers, status).await;
        assert_eq!(mock.calls.lock().unwrap()[0].1, Some(session));
    }

    #[tokio::test]
    async fn a_deletion_clears_the_cookie_only_once_it_is_done() {
        let cleared = "__Host-central=; Path=/; Secure; HttpOnly; SameSite=Strict; Max-Age=0";
        let finish = || {
            body(&Request::DeleteAccountFinish {
                attempt: Token::from_secret_bytes(Zeroizing::new([2; 32])),
                proof: vec![3],
            })
        };
        let headers = cookie_header(&format!("__Host-central={}", hex(1)));
        let (app, _) = listener();
        let sent = send(&app, &headers, finish()).await;
        assert_eq!(sent.status, StatusCode::OK);
        assert_eq!(sent.headers[header::SET_COOKIE], cleared);
        // A refused deletion (a wrong password) keeps the session and its cookie.
        let app = router(
            Arc::new(Mock {
                fail: Some(Error::Rejected),
                ..Mock::default()
            }),
            BrowserOrigin::live(),
        );
        let sent = send(&app, &headers, finish()).await;
        assert_eq!(sent.status, StatusCode::UNAUTHORIZED);
        assert!(!sent.headers.contains_key(header::SET_COOKIE));
        // The start leaves the cookie alone.
        let (app, _) = listener();
        let sent = send(
            &app,
            &headers,
            body(&Request::DeleteAccountStart { login: vec![1] }),
        )
        .await;
        assert_eq!(sent.status, StatusCode::OK);
        assert!(!sent.headers.contains_key(header::SET_COOKIE));
    }

    #[tokio::test]
    async fn signing_out_clears_the_cookie_even_when_the_session_is_gone() {
        let cleared = "__Host-central=; Path=/; Secure; HttpOnly; SameSite=Strict; Max-Age=0";
        let (app, _) = listener();
        let sent = send(
            &app,
            &cookie_header(&format!("__Host-central={}", hex(1))),
            body(&Request::Logout),
        )
        .await;
        assert_eq!(sent.status, StatusCode::OK);
        assert_eq!(sent.headers[header::SET_COOKIE], cleared);
        let mock = Arc::new(Mock {
            fail: Some(Error::Rejected),
            ..Mock::default()
        });
        let app = router(mock.clone(), BrowserOrigin::live());
        let sent = send(&app, &base(), body(&Request::Logout)).await;
        assert_eq!(sent.status, StatusCode::UNAUTHORIZED);
        assert_eq!(sent.headers[header::SET_COOKIE], cleared);
        // Errors keep the native route's fixed vocabulary and set no cookie.
        for (error, status) in [
            (Error::Rejected, StatusCode::UNAUTHORIZED),
            (Error::Unavailable, StatusCode::SERVICE_UNAVAILABLE),
            (Error::Conflict, StatusCode::CONFLICT),
            (Error::Busy, StatusCode::TOO_MANY_REQUESTS),
            (Error::TermsRequired, StatusCode::FORBIDDEN),
        ] {
            let app = router(
                Arc::new(Mock {
                    fail: Some(error),
                    ..Mock::default()
                }),
                BrowserOrigin::live(),
            );
            let finish = body(&Request::LoginFinish {
                attempt: Token::generate().unwrap(),
                message: vec![],
            });
            let sent = send(&app, &base(), finish).await;
            assert_eq!(sent.status, status);
            assert_eq!(sent.body, "account request rejected\n");
            assert!(sent.headers.get(header::SET_COOKIE).is_none());
        }
    }

    #[tokio::test]
    async fn registration_and_sign_in_are_budgeted_per_address() {
        let (app, mock) = listener();
        let from = |address: &str| with("x-forwarded-for", &[address]);
        let register = body(&Request::Register {
            username: "synthetic".into(),
            beta_key: "CENTRAL-0000-0000-0000-0000".into(),
            display_name: None,
        });
        for _ in 0..5 {
            assert_eq!(
                send(&app, &from("198.51.100.7"), register.clone())
                    .await
                    .status,
                StatusCode::OK
            );
        }
        let sent = send(&app, &from("198.51.100.7"), register.clone()).await;
        assert_eq!(sent.status, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(sent.body, "account request rejected\n");
        // The same address, mapped into IPv6, is the same client.
        let mapped = send(&app, &from("::ffff:198.51.100.7"), register.clone()).await;
        assert_eq!(mapped.status, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            send(&app, &from("198.51.100.8"), register.clone())
                .await
                .status,
            StatusCode::OK
        );
        // IPv6 counts by /64.
        for host in 1..=5 {
            let address = format!("2001:db8:1:2::{host}");
            assert_eq!(
                send(&app, &from(&address), register.clone()).await.status,
                StatusCode::OK
            );
        }
        let same_block = send(
            &app,
            &from("2001:db8:1:2:ffff:ffff:ffff:ffff"),
            register.clone(),
        )
        .await;
        assert_eq!(same_block.status, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            send(&app, &from("2001:db8:1:3::1"), register).await.status,
            StatusCode::OK
        );
        // Discovery and sign-in starts have budgets of their own, 30 a minute each.
        let start = body(&Request::LoginStart {
            identity: Identity {
                account: [7; 16],
                epoch: 1,
            },
            message: vec![0; 96],
        });
        for (request, other) in [(discover(), start.clone()), (start, discover())] {
            let address = if other == discover() {
                "198.51.100.30"
            } else {
                "198.51.100.31"
            };
            for _ in 0..30 {
                assert_eq!(
                    send(&app, &from(address), request.clone()).await.status,
                    StatusCode::OK
                );
            }
            assert_eq!(
                send(&app, &from(address), request.clone()).await.status,
                StatusCode::TOO_MANY_REQUESTS
            );
            assert_eq!(
                send(&app, &from(address), other).await.status,
                StatusCode::OK
            );
        }
        // Refused requests reach no service; the others did.
        assert_eq!(calls(&mock), 5 + 1 + 5 + 1 + 30 + 1 + 30 + 1);
    }

    #[tokio::test]
    async fn the_directory_is_served_without_a_session_and_budgeted_per_address() {
        let (app, mock) = listener();
        let from = |address: &str| with("x-forwarded-for", &[address]);
        let directory = body(&Request::GatewayDirectory { after: None });
        assert_eq!(
            directory,
            r#"{"operation":"gateway_directory","after":null}"#
        );
        // The page's own request, with no cookie, as `credentials: "omit"` sends it.
        let sent = send(&app, &from("198.51.100.40"), directory.clone()).await;
        assert_eq!(sent.status, StatusCode::OK);
        assert_eq!(sent.body, r#"{"result":"done"}"#);
        assert!(sent.headers.get(header::SET_COOKIE).is_none());
        assert_eq!(
            mock.calls.lock().unwrap()[0],
            ("gateway_directory".to_owned(), None)
        );
        // Without the page's marks it is refused like every other operation.
        let mut unmarked = from("198.51.100.40");
        unmarked.retain(|(name, _)| *name != "x-baylee-central");
        assert_eq!(
            send(&app, &unmarked, directory.clone()).await.status,
            StatusCode::FORBIDDEN
        );
        // 60 a minute from one address, then 429; the other budgets and the
        // other addresses are untouched.
        for _ in 1..60 {
            assert_eq!(
                send(&app, &from("198.51.100.40"), directory.clone())
                    .await
                    .status,
                StatusCode::OK
            );
        }
        let sent = send(&app, &from("198.51.100.40"), directory.clone()).await;
        assert_eq!(sent.status, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(sent.body, "account request rejected\n");
        assert_eq!(
            send(&app, &from("198.51.100.40"), discover()).await.status,
            StatusCode::OK
        );
        assert_eq!(
            send(&app, &from("198.51.100.41"), directory).await.status,
            StatusCode::OK
        );
        assert_eq!(calls(&mock), 60 + 1 + 1);
        // The window ends after a minute.
        let mut budgets = Budgets::default();
        let client = Client::from("198.51.100.42".parse::<IpAddr>().unwrap());
        let request = Request::GatewayDirectory { after: None };
        let now = Instant::now();
        for _ in 0..60 {
            assert!(budgets.charge(&request, client, now));
        }
        assert!(!budgets.charge(&request, client, now + Duration::from_secs(59)));
        assert!(budgets.charge(&request, client, now + MINUTE));
    }

    #[test]
    fn a_reset_has_ten_starts_an_hour_per_address() {
        let mut budgets = Budgets::default();
        let client = Client::from("198.51.100.43".parse::<IpAddr>().unwrap());
        let request = Request::ResetStart {
            username: "synthetic".into(),
            code: central_api::reset_code::ResetCode::generate().unwrap(),
            registration: vec![],
        };
        let now = Instant::now();
        for _ in 0..10 {
            assert!(budgets.charge(&request, client, now));
        }
        assert!(!budgets.charge(&request, client, now + Duration::from_secs(3_599)));
        // The other budgets are apart.
        assert!(budgets.charge(
            &Request::Discover {
                username: "synthetic".into()
            },
            client,
            now
        ));
        assert!(budgets.charge(&request, client, now + HOUR));
    }

    #[test]
    fn a_budget_window_ends_and_forgets_its_address() {
        let now = Instant::now();
        let mut book = Book::default();
        let client = Client::from("198.51.100.7".parse::<IpAddr>().unwrap());
        for _ in 0..5 {
            assert!(book.charge(client, now, 5, HOUR));
        }
        assert!(!book.charge(client, now + Duration::from_secs(3_599), 5, HOUR));
        assert!(book.charge(client, now + HOUR, 5, HOUR));
        // Another address's request forgets every window that has passed.
        let other = Client::from("198.51.100.8".parse::<IpAddr>().unwrap());
        assert!(book.charge(other, now + HOUR * 3, 5, HOUR));
        assert_eq!(book.0.len(), 1);
    }

    #[test]
    fn a_crowd_of_addresses_cannot_lock_a_new_one_out() {
        let now = Instant::now();
        let mut book = Book::default();
        for index in 0..u32::try_from(ADDRESSES).unwrap() {
            let address = IpAddr::from(std::net::Ipv4Addr::from(0x0a00_0000 + index));
            let at = now + Duration::from_millis(u64::from(index));
            assert!(book.charge(Client::from(address), at, 5, HOUR));
        }
        assert_eq!(book.0.len(), ADDRESSES);
        let newcomer = Client::from("198.51.100.7".parse::<IpAddr>().unwrap());
        assert!(book.charge(newcomer, now + Duration::from_secs(10), 5, HOUR));
        assert_eq!(book.0.len(), ADDRESSES);
        // The oldest window made room; the newest are kept.
        let oldest = Client::from(IpAddr::from(std::net::Ipv4Addr::from(0x0a00_0000)));
        assert!(!book.0.contains_key(&oldest));
    }

    fn site(host: &str, origin: &str) -> Vec<(&'static str, String)> {
        let mut headers: Vec<_> = base()
            .into_iter()
            .filter(|(name, _)| *name != "host" && *name != "origin")
            .collect();
        headers.push(("host", host.to_owned()));
        headers.push(("origin", origin.to_owned()));
        headers
    }

    #[tokio::test]
    async fn a_development_listener_serves_its_own_loopback_page_only() {
        let mock = Arc::new(Mock::default());
        let development = BrowserOrigin::development("http://127.0.0.1:5173/").unwrap();
        assert_eq!(development.origin(), "http://127.0.0.1:5173");
        let app = router(mock.clone(), development);
        let sent = send(
            &app,
            &site("127.0.0.1:5173", "http://127.0.0.1:5173"),
            discover(),
        )
        .await;
        assert_eq!(sent.status, StatusCode::OK);
        for (host, origin) in [
            ("baylee.app", "https://baylee.app"),
            ("127.0.0.1:5173", "https://baylee.app"),
            ("baylee.app", "http://127.0.0.1:5173"),
            ("127.0.0.1:5174", "http://127.0.0.1:5174"),
            ("localhost:5173", "http://localhost:5173"),
            ("127.0.0.1:5173", "http://127.0.0.1:5173/"),
        ] {
            let sent = send(&app, &site(host, origin), discover()).await;
            assert_eq!(sent.status, StatusCode::FORBIDDEN, "{host} {origin}");
        }
        // Every other condition still holds on a development listener.
        let mut unmarked = site("127.0.0.1:5173", "http://127.0.0.1:5173");
        unmarked.retain(|(name, _)| *name != "x-baylee-central");
        assert_eq!(
            send(&app, &unmarked, discover()).await.status,
            StatusCode::FORBIDDEN
        );
        assert_eq!(calls(&mock), 1);
        // The deployed listener does not serve the development page.
        let (live, live_mock) = listener();
        let sent = send(
            &live,
            &site("127.0.0.1:5173", "http://127.0.0.1:5173"),
            discover(),
        )
        .await;
        assert_eq!(sent.status, StatusCode::FORBIDDEN);
        assert_eq!(calls(&live_mock), 0);
        assert_eq!(BrowserOrigin::live().origin(), "https://baylee.app");
    }

    #[test]
    fn a_development_origin_is_numeric_loopback_http_with_a_port() {
        for origin in [
            "https://127.0.0.1:5173",
            "http://localhost:5173",
            "http://127.0.0.1",
            "http://127.0.0.1:0",
            "http://192.168.1.2:5173",
            "http://203.0.113.9:5173",
            "http://127.0.0.1:5173/app",
            "http://127.0.0.1:5173/?x=1",
            "http://user@127.0.0.1:5173",
            "https://baylee.app",
            "baylee.app",
            "",
        ] {
            assert!(BrowserOrigin::development(origin).is_err(), "{origin}");
        }
        assert_eq!(
            BrowserOrigin::development("http://[::1]:5173")
                .unwrap()
                .origin(),
            "http://[::1]:5173"
        );
    }
}
