//! The website's browser listener (`docs/web-accounts.md`): one route,
//! `POST /api/v1`, behind `baylee.app`'s Caddy, which passes `/api/*` with the
//! `Host` header unchanged. Default-deny: every refusal here happens before the
//! identity service is called, and no CORS header is ever sent. The session
//! token travels only in the `__Host-central` cookie.
use super::in_flight::{Client, InFlight, Permit, forwarded};

mod pictures;
use axum::{
    Router,
    body::{Body, Bytes},
    extract::{Path, Request as HttpRequest, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode, Uri, header, request::Parts},
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use central_api::{
    Error, Token,
    reports::{Part, RecordOrigin},
    wire::{Request, Response as Answer},
};
use central_identity::{
    BrowserReply, IdentityService, PictureEncoding, Place, Seen, UploadRefusal, browser_operation,
};
pub(crate) use pictures::UPLOAD_TIME as PICTURE_UPLOAD_TIME;
use std::{
    collections::BTreeMap,
    future::Future,
    net::SocketAddr,
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

/// A picture's bytes and the digest of its PNG master, when its key names one.
type StoredPicture = Option<(Vec<u8>, [u8; 32])>;

/// What answers browser requests: the identity service, or a test's stand-in.
pub(crate) trait Accounts: Send + Sync + 'static {
    /// `place` is the client address the request came from: the service
    /// counts guesses at passwords and requests without a session by it,
    /// and nothing else follows from it. `seen` is what the listener saw
    /// of a request for a registration challenge (`seen.rs`), and nothing
    /// for every other request.
    fn browser(
        &self,
        request: Request,
        cookie: Option<&Token>,
        place: Place,
        seen: Seen,
    ) -> impl Future<Output = Result<BrowserReply, Error>> + Send;

    /// A report's part as stored, for an administrator's session (`docs/reports.md`).
    fn report_part(
        &self,
        cookie: Option<&Token>,
        report: [u8; 16],
        part: Part,
    ) -> impl Future<Output = Result<(Vec<u8>, Option<RecordOrigin>), Error>> + Send;

    /// An admitted picture upload, holding its slot.
    type Admitted: Send + 'static;

    /// Admit a picture upload by the session `cookie`, before its body is
    /// read (`docs/profile-pictures.md`).
    fn admit_picture(
        &self,
        cookie: &Token,
    ) -> impl Future<Output = Result<Self::Admitted, UploadRefusal>> + Send;

    /// Convert and store an admitted upload; the answer is `MyPicture`.
    fn store_picture(
        &self,
        admitted: Self::Admitted,
        cookie: &Token,
        format: central_picture::Format,
        bytes: Vec<u8>,
    ) -> impl Future<Output = Result<Answer, UploadRefusal>> + Send;

    /// A picture's bytes and digest by its key, for anyone.
    fn picture_bytes(
        &self,
        key: [u8; 16],
        encoding: PictureEncoding,
    ) -> impl Future<Output = Result<StoredPicture, Error>> + Send;
}

impl Accounts for IdentityService {
    fn browser(
        &self,
        request: Request,
        cookie: Option<&Token>,
        place: Place,
        seen: Seen,
    ) -> impl Future<Output = Result<BrowserReply, Error>> + Send {
        self.handle_browser_from(request, cookie, Some(place), seen)
    }

    fn report_part(
        &self,
        cookie: Option<&Token>,
        report: [u8; 16],
        part: Part,
    ) -> impl Future<Output = Result<(Vec<u8>, Option<RecordOrigin>), Error>> + Send {
        IdentityService::report_part(self, cookie, report, part)
    }

    type Admitted = central_identity::Admitted;

    fn admit_picture(
        &self,
        cookie: &Token,
    ) -> impl Future<Output = Result<Self::Admitted, UploadRefusal>> + Send {
        IdentityService::admit_picture(self, cookie)
    }

    fn store_picture(
        &self,
        admitted: Self::Admitted,
        cookie: &Token,
        format: central_picture::Format,
        bytes: Vec<u8>,
    ) -> impl Future<Output = Result<Answer, UploadRefusal>> + Send {
        IdentityService::store_picture(self, admitted, cookie, format, bytes)
    }

    fn picture_bytes(
        &self,
        key: [u8; 16],
        encoding: PictureEncoding,
    ) -> impl Future<Output = Result<StoredPicture, Error>> + Send {
        IdentityService::picture_bytes(self, key, encoding)
    }
}

struct Browser<A> {
    accounts: Arc<A>,
    site: BrowserOrigin,
    budgets: Mutex<Budgets>,
    in_flight: InFlight,
    /// Reads of pictures in flight, apart from the calls' places.
    picture_flight: InFlight,
}

/// How long a request's body may take to arrive once its headers passed;
/// the page's bodies are small, so a slow one is a stalled one.
const BODY_TIME: Duration = Duration::from_secs(2);
/// How long the service may take to answer once the body is read.
const ANSWER_TIME: Duration = Duration::from_secs(5);
/// Requests one client address may have in flight at once, its body's
/// wait included. A page's islands ask one thing when they start (the
/// account summary) and the chat works strictly in turn, so four leave
/// room for a few tabs; an address shared by many people meets the limit
/// sooner, as it meets the budgets.
const IN_FLIGHT: u8 = 4;
/// Most addresses with a request in flight; the listener's connection cap
/// (at most 128) keeps the table far below it.
const IN_FLIGHT_ADDRESSES: usize = 1_024;

pub(crate) fn router<A: Accounts>(accounts: Arc<A>, site: BrowserOrigin) -> Router {
    routes(Arc::new(Browser {
        accounts,
        site,
        budgets: Mutex::default(),
        in_flight: in_flight(),
        picture_flight: in_flight(),
    }))
}

/// An empty table of the places in flight.
fn in_flight() -> InFlight {
    InFlight::new(IN_FLIGHT_ADDRESSES, usize::MAX)
}

fn routes<A: Accounts>(state: Arc<Browser<A>>) -> Router {
    let calls = Router::new()
        .route("/api/v1", post(call::<A>))
        .route("/api/v1/reports/{report}/{part}", post(report_part::<A>))
        // A backstop only: the body and the answer each have their own
        // deadline, the body's after the checks on the headers alone.
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            BODY_TIME.saturating_add(ANSWER_TIME),
        ));
    // A picture's upload: its body's own time, then the worker's.
    let upload = Router::new()
        .route("/api/v1/picture", put(pictures::upload::<A>))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            pictures::UPLOAD_TIME,
        ));
    let api = calls
        .merge(upload)
        // Nothing of a request to another path or with another method is read.
        .fallback(|| async { (StatusCode::NOT_FOUND, "not found\n") })
        .with_state(Arc::clone(&state))
        // Outermost: refusals and timeouts also receive the security headers.
        .layer(middleware::from_fn(super::boundary::headers));
    // Pictures, to anyone, with headers of their own (`pictures.rs`).
    let served = Router::new()
        .route("/api/v1/pictures/{file}", get(pictures::serve::<A>))
        .with_state(state)
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            pictures::SERVE_TIME,
        ))
        .layer(middleware::from_fn(pictures::headers));
    api.merge(served)
}

fn refused() -> Response {
    (StatusCode::FORBIDDEN, "request not permitted\n").into_response()
}

fn timed_out() -> Response {
    (StatusCode::REQUEST_TIMEOUT, "request timed out\n").into_response()
}

/// Why a request is refused before the service is asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Refusal {
    /// Not the site's own page through its proxy (403).
    NotPermitted,
    /// No well-formed client address (400).
    NoClient,
    /// The address has [`IN_FLIGHT`] requests in flight already, or the
    /// table is full (429).
    Busy,
    /// The body is too large or broken (413 or 400).
    Body(StatusCode),
    /// The body did not arrive within [`BODY_TIME`] (408).
    TimedOut,
}

impl IntoResponse for Refusal {
    fn into_response(self) -> Response {
        match self {
            Self::NotPermitted => refused(),
            Self::NoClient => (StatusCode::BAD_REQUEST, "invalid request\n").into_response(),
            Self::Busy => failure(Error::Busy),
            Self::Body(status) => (status, "request rejected\n").into_response(),
            Self::TimedOut => timed_out(),
        }
    }
}

/// What a request must pass on its headers alone, before its body is read
/// or the service is asked: the loopback proxy as the peer, the site's exact
/// headers, the client's address, and a place among that address's
/// requests in flight, held until the request ends however it ends.
fn admit<'a, A>(state: &'a Browser<A>, parts: &Parts) -> Result<Permit<'a>, Refusal> {
    // Only the local proxy reaches this listener; its forwarded address counts.
    let local = parts
        .extensions
        .get::<Peer>()
        .is_some_and(|Peer(peer)| peer.ip().is_loopback());
    if !local || !permitted(&parts.uri, &parts.headers, &state.site) {
        return Err(Refusal::NotPermitted);
    }
    let client = forwarded(&parts.headers).ok_or(Refusal::NoClient)?;
    state
        .in_flight
        .admit(client, IN_FLIGHT)
        .ok_or(Refusal::Busy)
}

/// The body, within [`REQUEST_BYTES`] and [`BODY_TIME`].
async fn read_body(parts: &Parts, body: Body) -> Result<Bytes, Refusal> {
    tokio::time::timeout(
        BODY_TIME,
        super::boundary::read(&parts.headers, body, REQUEST_BYTES),
    )
    .await
    .map_err(|_| Refusal::TimedOut)?
    .map_err(Refusal::Body)
}

async fn call<A: Accounts>(State(state): State<Arc<Browser<A>>>, request: HttpRequest) -> Response {
    let (parts, body) = request.into_parts();
    let permit = match admit(&state, &parts) {
        Ok(permit) => permit,
        Err(refusal) => return refusal.into_response(),
    };
    let body = match read_body(&parts, body).await {
        Ok(body) => body,
        Err(refusal) => return refusal.into_response(),
    };
    let headers = &parts.headers;
    let Ok(request) = central_api::decode::<Request>(&body) else {
        return failure(Error::Rejected);
    };
    if !browser_operation(&request) {
        return refused();
    }
    // The guard is gone before anything is awaited.
    let Ok(charged) = state
        .budgets
        .lock()
        .map(|mut budgets| budgets.admit(&request, permit.client, headers, Instant::now()))
    else {
        return failure(Error::Unavailable);
    };
    let Some(seen) = charged else {
        return failure(Error::Busy);
    };
    let logout = matches!(request, Request::Logout);
    let deletion = matches!(request, Request::DeleteAccountFinish { .. });
    let cookie = cookie(headers);
    let Ok(result) = tokio::time::timeout(
        ANSWER_TIME,
        state
            .accounts
            .browser(request, cookie.as_ref(), permit.client.into(), seen),
    )
    .await
    else {
        return timed_out();
    };
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

/// A report's part for an administrator's page (`docs/reports.md`): the
/// same proofs as `POST /api/v1` (the local proxy, the site's exact headers,
/// a client address, its place in flight, its budget), an administrator's
/// cookie, and the gzip as stored. The page unpacks it; nothing here does.
async fn report_part<A: Accounts>(
    State(state): State<Arc<Browser<A>>>,
    Path((report, part)): Path<(String, String)>,
    request: HttpRequest,
) -> Response {
    let (parts, body) = request.into_parts();
    let permit = match admit(&state, &parts) {
        Ok(permit) => permit,
        Err(refusal) => return refusal.into_response(),
    };
    // Nothing is asked of the body, but it keeps the listener's bounds.
    if let Err(refusal) = read_body(&parts, body).await {
        return refusal.into_response();
    }
    let (Some(report), Ok(part)) = (hex_id(&report), Part::from_stored(&part)) else {
        return failure(Error::Rejected);
    };
    let charged = match state.budgets.lock() {
        Ok(mut budgets) => budgets
            .parts
            .charge(permit.client, Instant::now(), 120, MINUTE),
        Err(_) => return failure(Error::Unavailable),
    };
    if !charged {
        return failure(Error::Busy);
    }
    let cookie = cookie(&parts.headers);
    let answer = state.accounts.report_part(cookie.as_ref(), report, part);
    match tokio::time::timeout(ANSWER_TIME, answer).await {
        Ok(Ok((gzip, origin))) => {
            let mut response = ([(header::CONTENT_TYPE, "application/gzip")], gzip).into_response();
            if let Some(origin) = origin {
                response.headers_mut().insert(
                    HeaderName::from_static("x-record-origin"),
                    HeaderValue::from_static(origin.as_str()),
                );
            }
            response
        }
        Ok(Err(error)) => failure(error),
        Err(_) => timed_out(),
    }
}

/// An id as a path segment: 32 lowercase hex digits.
pub(crate) fn hex_id(text: &str) -> Option<[u8; 16]> {
    if text.len() != 32 || !text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
        return None;
    }
    let mut id = [0; 16];
    for (i, byte) in id.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(id)
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

/// Most addresses one budget remembers; past it the oldest window goes, so a
/// crowd of addresses cannot lock new ones out.
const ADDRESSES: usize = 4_096;
pub(crate) const MINUTE: Duration = Duration::from_secs(60);
pub(crate) const HOUR: Duration = Duration::from_secs(3_600);
const DAY: Duration = Duration::from_hours(24);

/// The per-address budgets, in memory only; an address is forgotten once its
/// window has passed.
#[derive(Default)]
struct Budgets {
    register: Book,
    register_day: Book,
    challenge: Book,
    discover: Book,
    login: Book,
    directory: Book,
    reset: Book,
    /// Requests for a reset code by mail.
    reset_mail: Book,
    pair_start: Book,
    pair_status: Book,
    /// A report's parts an administrator's page reads.
    parts: Book,
    /// Pictures read (`pictures.rs`).
    pictures: Book,
    /// Registration challenges by the block of addresses around the caller
    /// ([`Client::block`]): counted for the challenge's difficulty, and
    /// never a reason to refuse.
    near: Book,
}

/// How much haste `count` challenges in an hour are: one more for each of
/// `steps` they pass, 0 to 3.
fn haste(count: u32, steps: [u32; 3]) -> u8 {
    steps
        .iter()
        .fold(0, |haste, step| haste + u8::from(count > *step))
}

/// Challenges an address may ask for in an hour and count as a few: a
/// person asks for one with each try, and registers five times at most.
const OWN_STEPS: [u32; 3] = [5, 10, 15];
/// The same for the block around it, which many people share.
const NEAR_STEPS: [u32; 3] = [20, 60, 200];

impl Budgets {
    fn charge(&mut self, request: &Request, client: Client, now: Instant) -> bool {
        match request {
            // Open registration (`docs/registration-abuse.md`): five an hour
            // and ten a day; a refused one spends no daily unit.
            Request::Register { .. } => {
                self.register.charge(client, now, 5, HOUR)
                    && self.register_day.charge(client, now, 10, DAY)
            }
            // A challenge for each attempt, with room for a few retries;
            // one budget for both ways to ask for it.
            Request::RegisterChallenge | Request::RegisterChallengeFor { .. } => {
                self.challenge.charge(client, now, 20, HOUR)
            }
            Request::Discover { .. } => self.discover.charge(client, now, 30, MINUTE),
            Request::LoginStart { .. } => self.login.charge(client, now, 30, MINUTE),
            // The public directory needs no session; a page reads it a few
            // times, so a minute's budget leaves room for paging. The public
            // figures are read beside it and spend from the same budget.
            Request::GatewayDirectory { .. } | Request::PublicFigures { .. } => {
                self.directory.charge(client, now, 60, MINUTE)
            }
            // A reset needs a code; a few tries an hour leave room for typing it.
            Request::ResetStart { .. } => self.reset.charge(client, now, 10, HOUR),
            // A reset code by mail (`docs/email.md`): the same few an hour
            // as on the account listener, whatever the name.
            Request::ResetMail { .. } => {
                self.reset_mail
                    .charge(client, now, super::addresses::RESET_MAIL, HOUR)
            }
            // A pairing is a click on "Play in Baylee"; its page asks for the
            // claim about once a second for at most five minutes.
            Request::PairStart => self.pair_start.charge(client, now, 10, HOUR),
            Request::PairStatus { .. } => self.pair_status.charge(client, now, 120, MINUTE),
            _ => true,
        }
    }

    /// Charges `request` to its budget: `None` when that is spent. For a
    /// registration challenge within its budget, what the listener saw of
    /// it (`seen.rs`), with the haste of its address and of the block
    /// around it this hour; for every other request nothing.
    fn admit(
        &mut self,
        request: &Request,
        client: Client,
        headers: &HeaderMap,
        now: Instant,
    ) -> Option<Seen> {
        if !self.charge(request, client, now) {
            return None;
        }
        if !matches!(
            request,
            Request::RegisterChallenge | Request::RegisterChallengeFor { .. }
        ) {
            return Some(Seen::default());
        }
        let own = haste(self.challenge.used(client), OWN_STEPS);
        let near = haste(self.near.count(client.block(), now, HOUR), NEAR_STEPS);
        Some(super::seen::glance(headers, own.max(near)))
    }
}

/// One budget: each address's window (its start and the units spent), and
/// when the windows that had passed were last forgotten.
#[derive(Default)]
pub(crate) struct Book(BTreeMap<Client, (Instant, u32)>, Option<Instant>);

impl Book {
    pub(crate) fn charge(
        &mut self,
        client: Client,
        now: Instant,
        limit: u32,
        period: Duration,
    ) -> bool {
        let used = self.window(client, now, period);
        if *used >= limit {
            return false;
        }
        *used += 1;
        true
    }

    /// One more unit of `client`, never refused: its units in this window,
    /// this one included.
    fn count(&mut self, client: Client, now: Instant, period: Duration) -> u32 {
        let used = self.window(client, now, period);
        *used = used.saturating_add(1);
        *used
    }

    /// The units `client` spent in the window it has.
    fn used(&self, client: Client) -> u32 {
        self.0.get(&client).map_or(0, |(_, used)| *used)
    }

    /// The units `client` spent in its window, which starts now when it has
    /// none; the windows that passed are forgotten and room is made first.
    fn window(&mut self, client: Client, now: Instant, period: Duration) -> &mut u32 {
        // The caller's own window, once passed, starts again.
        if self
            .0
            .get(&client)
            .is_some_and(|(start, _)| now.duration_since(*start) >= period)
        {
            self.0.remove(&client);
        }
        let known = self.0.contains_key(&client);
        // Every other passed window is forgotten once a period, and whenever
        // a newcomer finds the book full; not on every request.
        let due = self
            .1
            .is_none_or(|swept| now.duration_since(swept) >= period);
        if due || (!known && self.0.len() >= ADDRESSES) {
            self.0
                .retain(|_, (start, _)| now.duration_since(*start) < period);
            self.1 = Some(now);
        }
        if !known && self.0.len() >= ADDRESSES {
            let oldest = self
                .0
                .iter()
                .min_by_key(|(_, (start, _))| *start)
                .map(|(client, _)| *client);
            if let Some(oldest) = oldest {
                self.0.remove(&oldest);
            }
        }
        &mut self.0.entry(client).or_insert((now, 0)).1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use axum::http::HeaderValue;
    use central_api::{
        terms::TermsSubject,
        wire::{Identity, Response as Answer, TicketPurpose},
    };
    use std::net::IpAddr;
    use tower::ServiceExt;

    /// Records each call; answers `Done`, or a browser session for `LoginFinish`.
    #[derive(Default)]
    struct Mock {
        calls: Mutex<Vec<(String, Option<[u8; 32]>)>>,
        /// What the listener saw of each request, as it handed it over.
        seen: Mutex<Vec<Seen>>,
        /// The place each call came from, as the service is told it.
        places: Mutex<Vec<Place>>,
        fail: Option<Error>,
        /// Never answer, as a service that hangs.
        stall: bool,
        /// How a picture upload is refused, if it is.
        picture: Option<UploadRefusal>,
    }

    const SESSION: [u8; 32] = [0xab; 32];

    impl Accounts for Mock {
        fn browser(
            &self,
            request: Request,
            cookie: Option<&Token>,
            place: Place,
            seen: Seen,
        ) -> impl Future<Output = Result<BrowserReply, Error>> + Send {
            self.places.lock().unwrap().push(place);
            self.calls
                .lock()
                .unwrap()
                .push((operation(&request), cookie.map(Token::digest)));
            self.seen.lock().unwrap().push(seen);
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
            let stall = self.stall;
            async move {
                if stall {
                    std::future::pending::<()>().await;
                }
                reply
            }
        }

        fn report_part(
            &self,
            cookie: Option<&Token>,
            report: [u8; 16],
            part: Part,
        ) -> impl Future<Output = Result<(Vec<u8>, Option<RecordOrigin>), Error>> + Send {
            self.calls.lock().unwrap().push((
                format!("report_part {} {}", report[15], part.as_str()),
                cookie.map(Token::digest),
            ));
            std::future::ready(match self.fail {
                Some(error) => Err(error),
                None => Ok((
                    vec![0x1f, 0x8b, 8, 0],
                    (part == Part::Record).then_some(RecordOrigin::Client),
                )),
            })
        }

        type Admitted = ();

        fn admit_picture(
            &self,
            cookie: &Token,
        ) -> impl Future<Output = Result<(), UploadRefusal>> + Send {
            self.calls
                .lock()
                .unwrap()
                .push(("admit_picture".into(), Some(cookie.digest())));
            std::future::ready(match self.picture {
                Some(refusal @ (UploadRefusal::Account(_) | UploadRefusal::Blocked)) => {
                    Err(refusal)
                }
                _ => Ok(()),
            })
        }

        fn store_picture(
            &self,
            (): (),
            cookie: &Token,
            format: central_picture::Format,
            bytes: Vec<u8>,
        ) -> impl Future<Output = Result<Answer, UploadRefusal>> + Send {
            self.calls.lock().unwrap().push((
                format!("store_picture {format:?} {}", bytes.len()),
                Some(cookie.digest()),
            ));
            std::future::ready(match self.picture {
                Some(refusal @ UploadRefusal::Picture(_)) => Err(refusal),
                _ => Ok(Answer::MyPicture {
                    picture: None,
                    notice: None,
                    block: None,
                }),
            })
        }

        fn picture_bytes(
            &self,
            key: [u8; 16],
            encoding: PictureEncoding,
        ) -> impl Future<Output = Result<StoredPicture, Error>> + Send {
            self.calls
                .lock()
                .unwrap()
                .push((format!("picture_bytes {encoding:?}"), None));
            std::future::ready(match self.fail {
                Some(error) => Err(error),
                None => Ok((key == PICTURE).then(|| (b"picture bytes".to_vec(), [0xcd; 32]))),
            })
        }
    }

    /// The key the stand-in has a picture for.
    const PICTURE: [u8; 16] = [0x5a; 16];

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

    /// A report part read as the page reads it: status, headers and the raw bytes.
    async fn read_part(
        app: &Router,
        uri: &str,
        headers: &[(&'static str, String)],
        peer: &str,
    ) -> (StatusCode, HeaderMap, Vec<u8>) {
        let mut builder = HttpRequest::builder().method("POST").uri(uri);
        for (name, value) in headers {
            builder = builder.header(*name, value);
        }
        let response = app
            .clone()
            .oneshot(
                builder
                    .extension(Peer(peer.parse().unwrap()))
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, headers) = (response.status(), response.headers().clone());
        assert_eq!(headers[header::CACHE_CONTROL], "no-store");
        let bytes = to_bytes(response.into_body(), 1 << 20).await.unwrap();
        (status, headers, bytes.to_vec())
    }

    #[tokio::test]
    async fn a_report_part_is_the_page_s_own_read_with_a_cookie_and_comes_as_stored() {
        let (app, mock) = listener();
        let id = "0192d0c57b3e7a1f8c2d3e4f5a6b7c8d";
        let cookie = with("cookie", &[&format!("{COOKIE}={}", "ab".repeat(32))]);
        let (status, headers, body) = read_part(
            &app,
            &format!("/api/v1/reports/{id}/record"),
            &cookie,
            LOOPBACK,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(headers[header::CONTENT_TYPE], "application/gzip");
        assert_eq!(headers["x-record-origin"], "client");
        assert_eq!(
            body,
            [0x1f, 0x8b, 8, 0],
            "the bytes as stored, never unpacked"
        );
        let (_, headers, _) = read_part(
            &app,
            &format!("/api/v1/reports/{id}/client"),
            &cookie,
            LOOPBACK,
        )
        .await;
        assert!(!headers.contains_key("x-record-origin"));
        assert_eq!(calls(&mock), 2);
        assert_eq!(
            mock.calls.lock().unwrap()[0].1,
            Some(Token::from_hex(&"ab".repeat(32)).unwrap().digest())
        );
        // the site's proofs, the address and a well-formed path, or nothing is asked
        for (uri, headers, peer) in [
            (
                format!("/api/v1/reports/{id}/record"),
                with("x-baylee-central", &[]),
                LOOPBACK,
            ),
            (
                format!("/api/v1/reports/{id}/record"),
                with("origin", &["https://evil.example"]),
                LOOPBACK,
            ),
            (
                format!("/api/v1/reports/{id}/record"),
                cookie.clone(),
                "203.0.113.9:4444",
            ),
            (
                format!("/api/v1/reports/{id}/record"),
                with("x-forwarded-for", &[]),
                LOOPBACK,
            ),
            (
                format!("/api/v1/reports/{}/record", id.to_uppercase()),
                cookie.clone(),
                LOOPBACK,
            ),
            (
                format!("/api/v1/reports/{id}/screenshot"),
                cookie.clone(),
                LOOPBACK,
            ),
        ] {
            let (status, _, _) = read_part(&app, &uri, &headers, peer).await;
            assert!(status.is_client_error(), "{uri} {status}");
        }
        assert_eq!(calls(&mock), 2);
    }

    #[tokio::test]
    async fn report_parts_have_their_own_budget_per_address() {
        let (app, _) = listener();
        let uri = "/api/v1/reports/0192d0c57b3e7a1f8c2d3e4f5a6b7c8d/client";
        for _ in 0..120 {
            assert_eq!(
                read_part(&app, uri, &base(), LOOPBACK).await.0,
                StatusCode::OK
            );
        }
        assert_eq!(
            read_part(&app, uri, &base(), LOOPBACK).await.0,
            StatusCode::TOO_MANY_REQUESTS
        );
        // another address still reads, and a sign-in is not charged by the reads
        assert_eq!(
            read_part(
                &app,
                uri,
                &with("x-forwarded-for", &["198.51.100.8"]),
                LOOPBACK
            )
            .await
            .0,
            StatusCode::OK
        );
        assert_eq!(send(&app, &base(), discover()).await.status, StatusCode::OK);
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
            nonce: String::new(),
            counters: Vec::new(),
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
        // The service is told the same address, for its guesses at
        // passwords: the last hop, never a forged one; IPv6 by its /64.
        let told = |mock: &Mock| mock.places.lock().unwrap().clone();
        assert_eq!(told(&mock).len(), 5);
        assert!(
            told(&mock)
                .iter()
                .all(|place| *place == Place::V4([198, 51, 100, 20]))
        );
        let hops = ["203.0.113.9, 2001:db8:1:2:3:4:5:6"];
        let sent = send(&app, &with("x-forwarded-for", &hops), discover()).await;
        assert_eq!(sent.status, StatusCode::OK);
        assert!(
            told(&mock).last()
                == Some(&Place::V6([0x20, 0x01, 0x0d, 0xb8, 0x00, 0x01, 0x00, 0x02]))
        );
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
            nonce: String::new(),
            counters: Vec::new(),
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

    #[tokio::test]
    async fn the_figures_are_served_without_a_session_from_the_directory_s_budget() {
        let (app, mock) = listener();
        let from = |address: &str| with("x-forwarded-for", &[address]);
        let figures = body(&Request::PublicFigures { after: None });
        assert_eq!(figures, r#"{"operation":"public_figures","after":null}"#);
        let directory = body(&Request::GatewayDirectory { after: None });
        let sent = send(&app, &from("198.51.100.50"), figures.clone()).await;
        assert_eq!(sent.status, StatusCode::OK);
        assert!(sent.headers.get(header::SET_COOKIE).is_none());
        assert_eq!(
            mock.calls.lock().unwrap()[0],
            ("public_figures".to_owned(), None)
        );
        // Without the page's marks they are refused like every other operation.
        let mut unmarked = from("198.51.100.50");
        unmarked.retain(|(name, _)| *name != "x-baylee-central");
        assert_eq!(
            send(&app, &unmarked, figures.clone()).await.status,
            StatusCode::FORBIDDEN
        );
        // One budget of 60 a minute for the two public reads of an address:
        // 30 of each spend it, and then neither is served.
        for sent in 1..60 {
            let next = if sent % 2 == 0 { &figures } else { &directory };
            assert_eq!(
                send(&app, &from("198.51.100.50"), next.clone())
                    .await
                    .status,
                StatusCode::OK
            );
        }
        for refused in [&figures, &directory] {
            let sent = send(&app, &from("198.51.100.50"), refused.clone()).await;
            assert_eq!(sent.status, StatusCode::TOO_MANY_REQUESTS);
            assert_eq!(sent.body, "account request rejected\n");
        }
        // The other budgets and the other addresses are untouched.
        assert_eq!(
            send(&app, &from("198.51.100.50"), discover()).await.status,
            StatusCode::OK
        );
        assert_eq!(
            send(&app, &from("198.51.100.51"), figures).await.status,
            StatusCode::OK
        );
        assert_eq!(calls(&mock), 60 + 1 + 1);
        // The window ends after a minute, for both together.
        let mut budgets = Budgets::default();
        let client = Client::from("198.51.100.52".parse::<IpAddr>().unwrap());
        let now = Instant::now();
        let read = Request::PublicFigures { after: None };
        for _ in 0..60 {
            assert!(budgets.charge(&read, client, now));
        }
        assert!(!budgets.charge(&read, client, now + Duration::from_secs(59)));
        assert!(!budgets.charge(
            &Request::GatewayDirectory { after: None },
            client,
            now + Duration::from_secs(59)
        ));
        assert!(budgets.charge(&read, client, now + MINUTE));
    }

    #[test]
    fn registration_has_five_an_hour_ten_a_day_and_its_own_challenges() {
        let mut budgets = Budgets::default();
        let client = Client::from("198.51.100.44".parse::<IpAddr>().unwrap());
        let register = Request::Register {
            username: "synthetic".into(),
            display_name: None,
            nonce: String::new(),
            counters: Vec::new(),
        };
        let now = Instant::now();
        for _ in 0..5 {
            assert!(budgets.charge(&register, client, now));
        }
        assert!(!budgets.charge(&register, client, now));
        for _ in 0..5 {
            assert!(budgets.charge(&register, client, now + HOUR));
        }
        // ten in the day: the next hour's budget is there, the day's is not
        assert!(!budgets.charge(&register, client, now + HOUR * 2));
        assert!(budgets.charge(&register, client, now + DAY));
        for _ in 0..20 {
            assert!(budgets.charge(&Request::RegisterChallenge, client, now));
        }
        assert!(!budgets.charge(&Request::RegisterChallenge, client, now));
        assert!(budgets.charge(&Request::RegisterChallenge, client, now + HOUR));
    }

    /// Both ways to ask for a registration challenge spend the one budget:
    /// the newer one is no way around it.
    #[test]
    fn both_ways_to_ask_for_a_challenge_share_its_budget() {
        let mut budgets = Budgets::default();
        let client = Client::from("198.51.100.45".parse::<IpAddr>().unwrap());
        let now = Instant::now();
        let plain = Request::RegisterChallenge;
        let reported = Request::RegisterChallengeFor { signals: 0 };
        for _ in 0..10 {
            assert!(budgets.charge(&plain, client, now));
            assert!(budgets.charge(&reported, client, now));
        }
        assert!(!budgets.charge(&reported, client, now));
        assert!(!budgets.charge(&plain, client, now));
        let headers = HeaderMap::new();
        assert_eq!(budgets.admit(&reported, client, &headers, now), None);
        assert!(budgets.charge(&reported, client, now + HOUR));
    }

    /// The headers of a person's browser, as its page sends them.
    fn person() -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in [
            (
                "user-agent",
                "Mozilla/5.0 (X11; Linux x86_64; rv:130.0) Gecko/20100101 Firefox/130.0",
            ),
            ("accept-language", "de-DE,de;q=0.9"),
            ("sec-fetch-site", "same-origin"),
        ] {
            headers.insert(name, HeaderValue::from_static(value));
        }
        headers
    }

    /// An address's own haste: a person asks for one challenge with each
    /// try and may register five times an hour; more than that is counted,
    /// and never refused before the budget's twenty.
    #[test]
    fn an_address_s_challenges_past_a_few_count_as_haste() {
        let mut budgets = Budgets::default();
        let client = Client::from("198.51.100.46".parse::<IpAddr>().unwrap());
        let now = Instant::now();
        let headers = person();
        let mut seen = Vec::new();
        for index in 0..20 {
            // either way to ask counts the same
            let request = if index % 2 == 0 {
                Request::RegisterChallenge
            } else {
                Request::RegisterChallengeFor { signals: 0 }
            };
            let glance = budgets
                .admit(&request, client, &headers, now)
                .expect("within the budget");
            assert_eq!(
                glance,
                Seen {
                    frequent: glance.frequent,
                    ..Seen::default()
                },
                "nothing else is seen of a person's browser"
            );
            seen.push(glance.frequent);
        }
        let expected: Vec<u8> = [[0; 5], [1; 5], [2; 5], [3; 5]].concat();
        assert_eq!(seen, expected);
        assert_eq!(
            budgets.admit(&Request::RegisterChallenge, client, &headers, now),
            None
        );
        // the next hour starts with a few again
        let later = budgets.admit(&Request::RegisterChallenge, client, &headers, now + HOUR);
        assert_eq!(later, Some(Seen::default()));
    }

    /// One who holds a block of addresses is a new address with each
    /// request: the block around an address is counted too (an IPv6 /48, an
    /// IPv4 /24), for the difficulty only. Nobody is refused for it.
    #[test]
    fn a_block_of_addresses_is_counted_and_never_refused() {
        let now = Instant::now();
        let headers = person();
        for (blocks, other) in [
            (
                (0..300u16)
                    .map(|index| format!("2001:db8:77:{index:x}::1"))
                    .collect::<Vec<_>>(),
                "2001:db8:78::1",
            ),
            (
                // an IPv4 /24 has 256 addresses: each asks twice
                (0..300u16)
                    .map(|index| format!("203.0.113.{}", index % 150))
                    .collect::<Vec<_>>(),
                "203.0.112.9",
            ),
        ] {
            let mut budgets = Budgets::default();
            let mut haste = Vec::new();
            for address in &blocks {
                let client = Client::from(address.parse::<IpAddr>().unwrap());
                let seen = budgets
                    .admit(&Request::RegisterChallenge, client, &headers, now)
                    .expect("never refused for its block");
                haste.push(seen.frequent);
            }
            assert_eq!(&haste[..20], &[0; 20], "a few");
            assert_eq!((haste[20], haste[59]), (1, 1));
            assert_eq!((haste[60], haste[199]), (2, 2));
            assert_eq!((haste[200], haste[299]), (3, 3));
            // another block is another count, and the hour ends this one
            let elsewhere = Client::from(other.parse::<IpAddr>().unwrap());
            let seen = budgets.admit(&Request::RegisterChallenge, elsewhere, &headers, now);
            assert_eq!(seen, Some(Seen::default()), "{other}");
            let first = Client::from(blocks[0].parse::<IpAddr>().unwrap());
            let seen = budgets.admit(&Request::RegisterChallenge, first, &headers, now + HOUR);
            assert_eq!(seen, Some(Seen::default()));
        }
        // Only challenges are counted by their block, and only they are looked at.
        let mut budgets = Budgets::default();
        let client = Client::from("203.0.113.5".parse::<IpAddr>().unwrap());
        let discover = Request::Discover {
            username: "synthetic".into(),
        };
        for _ in 0..30 {
            let seen = budgets.admit(&discover, client, &HeaderMap::new(), now);
            assert_eq!(seen, Some(Seen::default()));
        }
        assert!(budgets.near.0.is_empty());
    }

    #[test]
    fn haste_steps_at_its_three_marks() {
        assert_eq!(haste(0, OWN_STEPS), 0);
        assert_eq!(haste(5, OWN_STEPS), 0);
        assert_eq!(haste(6, OWN_STEPS), 1);
        assert_eq!(haste(11, OWN_STEPS), 2);
        assert_eq!(haste(16, OWN_STEPS), 3);
        assert_eq!(haste(u32::MAX, NEAR_STEPS), 3);
        assert_eq!(haste(200, NEAR_STEPS), 2);
        let block = |text: &str| Client::from(text.parse::<IpAddr>().unwrap()).block();
        assert_eq!(block("2001:db8:77:1::1"), block("2001:db8:77:ffff::2"));
        assert_ne!(block("2001:db8:77:1::1"), block("2001:db8:78:1::1"));
        assert_eq!(block("203.0.113.1"), block("203.0.113.254"));
        assert_ne!(block("203.0.113.1"), block("203.0.112.1"));
        // an IPv4 address written as IPv6 is the IPv4 address
        assert_eq!(block("::ffff:203.0.113.1"), block("203.0.113.9"));
    }

    /// What the listener hands the service of a request for a registration
    /// challenge: plain facts of its headers, the same for both ways to
    /// ask, and nothing of any other request.
    #[tokio::test]
    async fn the_service_is_told_what_a_challenge_request_looked_like() {
        let (app, mock) = listener();
        let plain = body(&Request::RegisterChallenge);
        let reported = body(&Request::RegisterChallengeFor { signals: 1 });
        // A script with the listener's own headers and no browser's.
        let script = Seen {
            no_language: true,
            agent: central_identity::Agent::Missing,
            ..Seen::default()
        };
        assert_eq!(send(&app, &base(), plain.clone()).await.status, 200);
        assert_eq!(send(&app, &base(), reported.clone()).await.status, 200);
        // A person's browser through the proxy.
        let mut browser = base();
        browser.push((
            "user-agent",
            "Mozilla/5.0 (X11; Linux x86_64; rv:130.0) Gecko/20100101 Firefox/130.0".into(),
        ));
        browser.push(("accept-language", "en-GB,en;q=0.9".into()));
        assert_eq!(send(&app, &browser, plain.clone()).await.status, 200);
        assert_eq!(send(&app, &browser, reported.clone()).await.status, 200);
        // A tool that names itself, and an older browser without fetch metadata.
        let mut tool = browser.clone();
        tool.retain(|(name, _)| *name != "user-agent" && *name != "sec-fetch-site");
        tool.push(("user-agent", "curl/8.7.1".into()));
        assert_eq!(send(&app, &tool, plain.clone()).await.status, 200);
        // Another request is not looked at, whatever it sends.
        assert_eq!(send(&app, &base(), discover()).await.status, 200);
        let seen = mock.seen.lock().unwrap().clone();
        assert_eq!(
            seen,
            [
                script,
                script,
                Seen::default(),
                Seen::default(),
                Seen {
                    no_fetch_site: true,
                    agent: central_identity::Agent::Tool,
                    ..Seen::default()
                },
                Seen::default(),
            ]
        );
        // The sixth challenge of the hour from the address is haste.
        assert_eq!(send(&app, &browser, plain).await.status, 200);
        assert_eq!(mock.seen.lock().unwrap().last().unwrap().frequent, 1);
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
    fn a_reset_by_mail_has_five_requests_an_hour_per_address_whatever_the_name() {
        let mut budgets = Budgets::default();
        let client = Client::from("198.51.100.44".parse::<IpAddr>().unwrap());
        let asked = |name: &str| Request::ResetMail {
            username: name.into(),
            lang: "en".into(),
        };
        let now = Instant::now();
        // Five, counted by the address and not by the name asked about.
        for name in ["one", "two", "three", "four", "five"] {
            assert!(budgets.charge(&asked(name), client, now));
        }
        for name in ["one", "six"] {
            assert!(!budgets.charge(&asked(name), client, now + Duration::from_secs(3_599)));
        }
        // Another address is served, and so is a reset's start from this one.
        let other = Client::from("198.51.100.45".parse::<IpAddr>().unwrap());
        assert!(budgets.charge(&asked("one"), other, now));
        assert!(budgets.charge(
            &Request::ResetStart {
                username: "synthetic".into(),
                code: central_api::reset_code::ResetCode::generate().unwrap(),
                registration: vec![],
            },
            client,
            now
        ));
        assert!(budgets.charge(&asked("one"), client, now + HOUR));
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

    /// A body that never sends a byte: a client that stops after its headers.
    struct Stalled;

    impl hyper::body::Body for Stalled {
        type Data = Bytes;
        type Error = std::convert::Infallible;

        fn poll_frame(
            self: std::pin::Pin<&mut Self>,
            _: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Option<Result<hyper::body::Frame<Bytes>, Self::Error>>> {
            std::task::Poll::Pending
        }
    }

    /// The listener's state and its routes, so a test can see the table.
    fn listener_with(mock: Mock) -> (Arc<Browser<Mock>>, Router) {
        let state = Arc::new(Browser {
            accounts: Arc::new(mock),
            site: BrowserOrigin::live(),
            budgets: Mutex::default(),
            in_flight: in_flight(),
            picture_flight: in_flight(),
        });
        (state.clone(), routes(state))
    }

    fn from(address: &str) -> Vec<(&'static str, String)> {
        with("x-forwarded-for", &[address])
    }

    fn client_of(address: &str) -> Client {
        Client::from(address.parse::<IpAddr>().unwrap())
    }

    /// A request whose body never comes, sent in the background.
    fn stall(app: &Router, address: &str) -> tokio::task::JoinHandle<StatusCode> {
        let mut builder = HttpRequest::builder()
            .method("POST")
            .uri("/api/v1")
            .extension(Peer(LOOPBACK.parse().unwrap()));
        for (name, value) in from(address) {
            builder = builder.header(name, value);
        }
        let request = builder.body(Body::new(Stalled)).unwrap();
        let app = app.clone();
        tokio::spawn(async move { app.oneshot(request).await.unwrap().status() })
    }

    async fn until(what: impl Fn() -> bool) {
        tokio::time::timeout(Duration::from_secs(2), async {
            while !what() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("in time");
    }

    #[tokio::test]
    async fn a_stalled_body_holds_no_more_than_its_address_s_places() {
        let (state, app) = listener_with(Mock::default());
        let stalled = "198.51.100.7";
        let waiting: Vec<_> = (0..IN_FLIGHT).map(|_| stall(&app, stalled)).collect();
        until(|| state.in_flight.count(client_of(stalled)) == IN_FLIGHT).await;
        // One more from the same address is refused at once, before its body
        // is read, the service asked or a budget charged.
        let started = Instant::now();
        let refused = send(&app, &from(stalled), discover()).await;
        assert_eq!(refused.status, StatusCode::TOO_MANY_REQUESTS);
        assert!(started.elapsed() < BODY_TIME);
        assert_eq!(state.in_flight.count(client_of(stalled)), IN_FLIGHT);
        // Another address is served meanwhile.
        let other = send(&app, &from("198.51.100.8"), discover()).await;
        assert_eq!(other.status, StatusCode::OK);
        assert_eq!(state.in_flight.count(client_of("198.51.100.8")), 0);
        // The stalled bodies time out on their own deadline and give their places back.
        for waiting in waiting {
            assert_eq!(waiting.await.unwrap(), StatusCode::REQUEST_TIMEOUT);
        }
        assert_eq!(state.in_flight.addresses(), 0);
        let again = send(&app, &from(stalled), discover()).await;
        assert_eq!(again.status, StatusCode::OK);
        assert_eq!(
            calls(&state.accounts),
            2,
            "the stalled requests asked nothing"
        );
        let budgets = state.budgets.lock().unwrap();
        assert_eq!(
            budgets
                .discover
                .0
                .get(&client_of(stalled))
                .map(|(_, used)| *used),
            Some(1),
            "only the served request spent a unit"
        );
    }

    #[tokio::test]
    async fn a_place_in_flight_is_given_back_however_the_request_ends() {
        let (state, app) = listener_with(Mock::default());
        let address = "198.51.100.9";
        let register = body(&Request::Register {
            username: "synthetic".into(),
            nonce: String::new(),
            counters: Vec::new(),
            display_name: None,
        });
        // answered, malformed, not a browser operation, too large, over budget
        let mut statuses = Vec::new();
        for text in [
            discover(),
            "{".to_owned(),
            body(&Request::ReadDocument),
            " ".repeat(REQUEST_BYTES + 1),
        ] {
            statuses.push(send(&app, &from(address), text).await.status);
            assert_eq!(state.in_flight.addresses(), 0);
        }
        for _ in 0..6 {
            statuses.push(send(&app, &from(address), register.clone()).await.status);
            assert_eq!(state.in_flight.addresses(), 0);
        }
        assert_eq!(
            statuses[..4],
            [
                StatusCode::OK,
                StatusCode::UNAUTHORIZED,
                StatusCode::FORBIDDEN,
                StatusCode::PAYLOAD_TOO_LARGE
            ]
        );
        assert_eq!(statuses[9], StatusCode::TOO_MANY_REQUESTS);
        // More requests than places, one after another: none is refused for a
        // place an earlier one kept.
        for _ in 0..(usize::from(IN_FLIGHT) * 3) {
            assert_eq!(
                send(&app, &from(address), discover()).await.status,
                StatusCode::OK
            );
        }
        // The service's own deadline: an answer that never comes times out
        // and gives the place back too.
        let (state, app) = listener_with(Mock {
            stall: true,
            ..Mock::default()
        });
        let started = Instant::now();
        let hung = send(&app, &from(address), discover()).await;
        assert_eq!(hung.status, StatusCode::REQUEST_TIMEOUT);
        assert!(started.elapsed() >= ANSWER_TIME);
        assert_eq!(state.in_flight.addresses(), 0);
    }

    #[test]
    fn a_full_table_refuses_newcomers_and_never_evicts_a_place() {
        let table = in_flight();
        let mut held = Vec::new();
        for index in 0..u32::try_from(IN_FLIGHT_ADDRESSES).unwrap() {
            let client = Client::from(IpAddr::from(std::net::Ipv4Addr::from(0x0a00_0000 + index)));
            held.push(table.admit(client, IN_FLIGHT).expect("a place"));
        }
        let newcomer = client_of("198.51.100.7");
        assert!(table.admit(newcomer, IN_FLIGHT).is_none());
        // An address already in the table still has its other places.
        let first = Client::from(IpAddr::from(std::net::Ipv4Addr::from(0x0a00_0000)));
        for _ in 1..IN_FLIGHT {
            held.push(table.admit(first, IN_FLIGHT).expect("its own place"));
        }
        assert!(table.admit(first, IN_FLIGHT).is_none());
        assert_eq!(table.count(first), IN_FLIGHT);
        drop(held);
        assert_eq!(table.addresses(), 0);
        assert!(table.admit(newcomer, IN_FLIGHT).is_some());
    }

    /// A request over a real socket from the loopback proxy, its headers
    /// as the page's, from `address`, declaring `length` bytes and sending `sent`.
    fn raw(address: &str, length: usize, sent: &str) -> String {
        request(&from(address), length, sent)
    }

    fn request(headers: &[(&'static str, String)], length: usize, sent: &str) -> String {
        use std::fmt::Write as _;
        let mut text = String::from("POST /api/v1 HTTP/1.1\r\n");
        for (name, value) in headers {
            let _ = write!(text, "{name}: {value}\r\n");
        }
        let _ = write!(text, "content-length: {length}\r\n\r\n{sent}");
        text
    }

    fn answer(socket: &mut std::net::TcpStream) -> String {
        use std::io::Read as _;
        let mut answer = Vec::new();
        let _ = socket.read_to_end(&mut answer);
        String::from_utf8_lossy(&answer).into_owned()
    }

    #[tokio::test]
    async fn stalled_bodies_cannot_take_the_listener_s_connections() {
        use std::io::Write as _;
        let config = central_policy::config::Config::from_pairs([
            ("BAYLEE_CENTRAL_LISTEN", "127.0.0.1:0"),
            ("BAYLEE_CENTRAL_CONCURRENT_REQUESTS", "8"),
        ])
        .unwrap();
        let bound = super::super::DevelopmentServer::bind(config).await.unwrap();
        let address = bound.local_addr();
        let (state, app) = listener_with(Mock::default());
        let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
        let running = tokio::spawn(bound.run_router(app, Duration::from_secs(10), async {
            let _ = stopped.await;
        }));
        let connect = move || {
            let socket =
                std::net::TcpStream::connect_timeout(&address, Duration::from_secs(2)).unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            socket
        };
        // Four stalled bodies from one address, and one that goes away.
        let mut stalled: Vec<_> = (0..IN_FLIGHT)
            .map(|_| {
                let mut socket = connect();
                socket
                    .write_all(raw("203.0.113.5", 64, "{").as_bytes())
                    .unwrap();
                socket
            })
            .collect();
        let stalled_client = client_of("203.0.113.5");
        until(|| state.in_flight.count(stalled_client) == IN_FLIGHT).await;
        let (served, refused, whole) = tokio::task::spawn_blocking(move || {
            let mut gone = connect();
            gone.write_all(raw("203.0.113.6", 64, "{").as_bytes())
                .unwrap();
            drop(gone);
            // The fifth from the stalled address is answered at once.
            let started = Instant::now();
            let mut fifth = connect();
            fifth
                .write_all(raw("203.0.113.5", 64, "").as_bytes())
                .unwrap();
            let refused = (answer(&mut fifth), started.elapsed());
            // A proxy sends the headers and the whole body at once: the
            // refusal still arrives whole, its body unread.
            let body = discover();
            let large = " ".repeat(60_000);
            let mut whole = Vec::new();
            for (headers, sent) in [
                (from("203.0.113.5"), body.as_str()),
                (from("203.0.113.5"), large.as_str()),
                (with("origin", &["https://evil.example"]), body.as_str()),
                (with("origin", &["https://evil.example"]), large.as_str()),
            ] {
                let mut socket = connect();
                let _ = socket.write_all(request(&headers, sent.len(), sent).as_bytes());
                whole.push(answer(&mut socket));
            }
            // Another address is served while the four wait.
            let mut other = connect();
            other
                .write_all(raw("203.0.113.7", body.len(), &body).as_bytes())
                .unwrap();
            (answer(&mut other), refused, whole)
        })
        .await
        .unwrap();
        assert!(refused.0.starts_with("HTTP/1.1 429"), "{}", refused.0);
        for (answer, status) in whole.iter().zip(["429", "429", "403", "403"]) {
            assert!(
                answer.starts_with(&format!("HTTP/1.1 {status}")),
                "{status}: {answer}"
            );
        }
        assert!(refused.1 < BODY_TIME, "{:?}", refused.1);
        assert!(served.starts_with("HTTP/1.1 200"), "{served}");
        // The one whose connection went away has given its place back.
        until(|| state.in_flight.count(client_of("203.0.113.6")) == 0).await;
        // The stalled ones end on the body's deadline, not the connection's.
        let answers =
            tokio::task::spawn_blocking(move || stalled.iter_mut().map(answer).collect::<Vec<_>>())
                .await
                .unwrap();
        for answer in answers {
            assert!(answer.starts_with("HTTP/1.1 408"), "{answer}");
        }
        assert_eq!(state.in_flight.addresses(), 0);
        stop.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(5), running)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(calls(&state.accounts), 1);
    }

    #[test]
    fn a_caller_s_own_passed_window_starts_again_between_sweeps() {
        let now = Instant::now();
        let mut book = Book::default();
        let client = |last: u8| Client::from(IpAddr::from([198, 51, 100, last]));
        let (first, quiet, late, other) = (client(1), client(2), client(3), client(4));
        // The first request sweeps and stamps the book.
        assert!(book.charge(first, now, 5, HOUR));
        assert!(book.charge(quiet, now + HOUR * 2 / 5, 5, HOUR));
        let opened = now + HOUR / 2;
        for _ in 0..5 {
            assert!(book.charge(late, opened, 5, HOUR));
        }
        assert!(!book.charge(late, opened, 5, HOUR));
        // A period after the stamp the next request sweeps: only the first
        // window has passed.
        assert!(book.charge(other, now + HOUR, 5, HOUR));
        assert!(!book.0.contains_key(&first));
        assert!(book.0.contains_key(&quiet));
        assert!(!book.charge(late, now + HOUR, 5, HOUR));
        // Before the next sweep is due, the late window passes: its own
        // address finds it passed and starts again, though nothing swept.
        let passed = opened + HOUR;
        assert!(book.charge(late, passed, 5, HOUR));
        assert_eq!(book.0.get(&late), Some(&(passed, 1)));
        // The quiet address's window has passed too and waits for the sweep.
        assert!(book.0.contains_key(&quiet));
        assert!(book.charge(other, now + HOUR * 2, 5, HOUR));
        assert!(!book.0.contains_key(&quiet));
        assert_eq!(book.0.len(), 2);
        assert_eq!(book.0.get(&other), Some(&(now + HOUR * 2, 1)));
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

    mod pictures {
        use super::*;
        use central_picture::Refusal as PictureRefusal;

        /// Headers, the peer and the expected status of an upload.
        type UploadCase = (Vec<(&'static str, String)>, &'static str, StatusCode);
        /// A path, headers and the peer of a read.
        type ReadCase = (String, Vec<(&'static str, &'static str)>, &'static str);

        /// The headers the profile page sends with a picture.
        fn upload_headers(kind: &str, length: usize) -> Vec<(&'static str, String)> {
            let mut headers = with("content-type", &[kind]);
            headers.push(("content-length", length.to_string()));
            headers.push(("cookie", format!("__Host-central={}", "ab".repeat(32))));
            headers
        }

        /// A body that never comes: a handler that reads it waits for ever.
        struct Withheld;

        impl hyper::body::Body for Withheld {
            type Data = Bytes;
            type Error = std::io::Error;

            fn poll_frame(
                self: std::pin::Pin<&mut Self>,
                _: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Option<Result<hyper::body::Frame<Bytes>, Self::Error>>>
            {
                std::task::Poll::Pending
            }
        }

        async fn put(
            app: &Router,
            headers: &[(&'static str, String)],
            body: Body,
            peer: &str,
        ) -> (StatusCode, HeaderMap, String) {
            let mut builder = HttpRequest::builder().method("PUT").uri("/api/v1/picture");
            for (name, value) in headers {
                builder = builder.header(*name, value);
            }
            let response = tokio::time::timeout(
                Duration::from_secs(3),
                app.clone().oneshot(
                    builder
                        .extension(Peer(peer.parse().unwrap()))
                        .body(body)
                        .unwrap(),
                ),
            )
            .await
            .expect("answered without reading a withheld body")
            .unwrap();
            let (status, headers) = (response.status(), response.headers().clone());
            assert_eq!(headers[header::CACHE_CONTROL], "no-store");
            let bytes = to_bytes(response.into_body(), 1 << 20).await.unwrap();
            (status, headers, String::from_utf8(bytes.to_vec()).unwrap())
        }

        #[tokio::test]
        async fn every_header_refusal_comes_before_the_body_and_the_service() {
            let png = upload_headers("image/png", 1_000);
            let mut cases: Vec<UploadCase> = vec![
                (
                    png.iter()
                        .filter(|(n, _)| *n != "origin")
                        .cloned()
                        .collect(),
                    LOOPBACK,
                    StatusCode::FORBIDDEN,
                ),
                (
                    png.iter()
                        .filter(|(n, _)| *n != "x-baylee-central")
                        .cloned()
                        .collect(),
                    LOOPBACK,
                    StatusCode::FORBIDDEN,
                ),
                (
                    png.iter()
                        .filter(|(n, _)| *n != "content-length")
                        .cloned()
                        .collect(),
                    LOOPBACK,
                    StatusCode::LENGTH_REQUIRED,
                ),
                (
                    png.iter()
                        .filter(|(n, _)| *n != "x-forwarded-for")
                        .cloned()
                        .collect(),
                    LOOPBACK,
                    StatusCode::BAD_REQUEST,
                ),
                (
                    upload_headers("application/json", 10),
                    LOOPBACK,
                    StatusCode::FORBIDDEN,
                ),
                (
                    upload_headers("image/heic", 10),
                    LOOPBACK,
                    StatusCode::FORBIDDEN,
                ),
                (
                    upload_headers("image/png; charset=x", 10),
                    LOOPBACK,
                    StatusCode::FORBIDDEN,
                ),
                (
                    upload_headers("image/png", 8 * 1024 * 1024 + 1),
                    LOOPBACK,
                    StatusCode::PAYLOAD_TOO_LARGE,
                ),
                (
                    upload_headers("image/svg+xml", 1024 * 1024 + 1),
                    LOOPBACK,
                    StatusCode::PAYLOAD_TOO_LARGE,
                ),
                (png.clone(), "192.0.2.1:50000", StatusCode::FORBIDDEN),
            ];
            for (name, value) in [
                ("host", "evil.example"),
                ("origin", "https://evil.example"),
                ("sec-fetch-site", "cross-site"),
                ("authorization", "Bearer x"),
            ] {
                let mut headers: Vec<_> = png.iter().filter(|(n, _)| *n != name).cloned().collect();
                headers.push((name, value.to_owned()));
                cases.push((headers, LOOPBACK, StatusCode::FORBIDDEN));
            }
            let mut extra_length = png.clone();
            extra_length.push(("content-length", "1".into()));
            cases.push((extra_length, LOOPBACK, StatusCode::LENGTH_REQUIRED));
            for (headers, peer, expected) in cases {
                let (app, mock) = listener();
                let (status, ..) = put(&app, &headers, Body::new(Withheld), peer).await;
                assert_eq!(status, expected, "{headers:?}");
                assert_eq!(calls(&mock), 0, "{headers:?}");
            }
            // without a cookie the service is not asked either
            let (app, mock) = listener();
            let no_cookie: Vec<_> = png
                .iter()
                .filter(|(n, _)| *n != "cookie")
                .cloned()
                .collect();
            let (status, ..) = put(&app, &no_cookie, Body::new(Withheld), LOOPBACK).await;
            assert_eq!(status, StatusCode::UNAUTHORIZED);
            assert_eq!(calls(&mock), 0);
        }

        #[tokio::test]
        async fn the_account_is_admitted_before_its_body_is_read() {
            for (refusal, expected, body) in [
                (
                    UploadRefusal::Blocked,
                    StatusCode::FORBIDDEN,
                    r#"{"refusal":"blocked"}"#,
                ),
                (
                    UploadRefusal::Account(Error::Busy),
                    StatusCode::TOO_MANY_REQUESTS,
                    "account request rejected\n",
                ),
                (
                    UploadRefusal::Account(Error::TermsRequired),
                    StatusCode::FORBIDDEN,
                    "account request rejected\n",
                ),
                (
                    UploadRefusal::Account(Error::Rejected),
                    StatusCode::UNAUTHORIZED,
                    "account request rejected\n",
                ),
            ] {
                let mock = Arc::new(Mock {
                    picture: Some(refusal),
                    ..Mock::default()
                });
                let app = router(mock.clone(), BrowserOrigin::live());
                let (status, _, text) = put(
                    &app,
                    &upload_headers("image/png", 1_000),
                    Body::new(Withheld),
                    LOOPBACK,
                )
                .await;
                assert_eq!((status, text.as_str()), (expected, body), "{refusal:?}");
                let names: Vec<String> = mock
                    .calls
                    .lock()
                    .unwrap()
                    .iter()
                    .map(|(n, _)| n.clone())
                    .collect();
                assert_eq!(names, ["admit_picture"]);
            }
        }

        #[tokio::test]
        async fn a_picture_is_read_to_its_length_and_its_refusals_are_named() {
            let (app, mock) = listener();
            let (status, headers, text) = put(
                &app,
                &upload_headers("image/png", 12),
                Body::from(vec![7; 12]),
                LOOPBACK,
            )
            .await;
            assert_eq!(status, StatusCode::OK, "{text}");
            assert_eq!(headers[header::CONTENT_TYPE], "application/json");
            assert_eq!(
                text,
                r#"{"result":"my_picture","picture":null,"notice":null,"block":null}"#
            );
            let names: Vec<String> = mock
                .calls
                .lock()
                .unwrap()
                .iter()
                .map(|(n, _)| n.clone())
                .collect();
            assert_eq!(names, ["admit_picture", "store_picture Png 12"]);
            // fewer bytes than announced, and more
            let (app, _) = listener();
            let (status, ..) = put(
                &app,
                &upload_headers("image/png", 12),
                Body::from(vec![7; 5]),
                LOOPBACK,
            )
            .await;
            assert_eq!(status, StatusCode::BAD_REQUEST);
            let (status, ..) = put(
                &app,
                &upload_headers("image/png", 12),
                Body::from(vec![7; 13]),
                LOOPBACK,
            )
            .await;
            assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
            for refusal in PictureRefusal::ALL {
                let mock = Arc::new(Mock {
                    picture: Some(UploadRefusal::Picture(refusal)),
                    ..Mock::default()
                });
                let app = router(mock, BrowserOrigin::live());
                let (status, headers, text) = put(
                    &app,
                    &upload_headers("image/jpeg", 3),
                    Body::from(vec![1; 3]),
                    LOOPBACK,
                )
                .await;
                assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
                assert_eq!(headers[header::CONTENT_TYPE], "application/json");
                assert_eq!(text, format!(r#"{{"refusal":"{}"}}"#, refusal.as_str()));
            }
        }

        #[tokio::test]
        async fn a_body_that_stalls_ends_at_its_pace() {
            let (app, mock) = listener();
            let mut builder = HttpRequest::builder().method("PUT").uri("/api/v1/picture");
            for (name, value) in upload_headers("image/png", 1_000) {
                builder = builder.header(name, value);
            }
            let started = Instant::now();
            let response = app
                .oneshot(
                    builder
                        .extension(Peer(LOOPBACK.parse().unwrap()))
                        .body(Body::new(Withheld))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::REQUEST_TIMEOUT);
            assert!(started.elapsed() < Duration::from_secs(8));
            let names: Vec<String> = mock
                .calls
                .lock()
                .unwrap()
                .iter()
                .map(|(n, _)| n.clone())
                .collect();
            assert_eq!(names, ["admit_picture"]);
        }

        async fn get_picture(
            app: &Router,
            method: &str,
            uri: &str,
            headers: &[(&'static str, &str)],
            peer: &str,
        ) -> (StatusCode, HeaderMap, Vec<u8>) {
            let mut builder = HttpRequest::builder().method(method).uri(uri);
            for (name, value) in headers {
                builder = builder.header(*name, *value);
            }
            let response = app
                .clone()
                .oneshot(
                    builder
                        .extension(Peer(peer.parse().unwrap()))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            let (status, headers) = (response.status(), response.headers().clone());
            let bytes = to_bytes(response.into_body(), 1 << 20).await.unwrap();
            (status, headers, bytes.to_vec())
        }

        const PAGE: [(&str, &str); 3] = [
            ("host", "baylee.app"),
            ("sec-fetch-site", "same-origin"),
            ("x-forwarded-for", "198.51.100.7"),
        ];

        fn uri(key: [u8; 16], extension: &str) -> String {
            format!(
                "/api/v1/pictures/{}.{extension}",
                central_api::picture::key_hex(&key)
            )
        }

        #[tokio::test]
        async fn a_picture_is_served_to_anyone_with_its_own_headers() {
            let (app, mock) = listener();
            for (extension, kind) in [
                ("avif", "image/avif"),
                ("png", "image/png"),
                ("sequence.avif", "image/avif"),
            ] {
                let (status, headers, body) =
                    get_picture(&app, "GET", &uri(PICTURE, extension), &PAGE, LOOPBACK).await;
                assert_eq!(status, StatusCode::OK);
                assert_eq!(body, b"picture bytes");
                let expected = [
                    ("content-type", kind),
                    ("cache-control", "private, max-age=3600"),
                    ("content-disposition", "inline"),
                    ("x-content-type-options", "nosniff"),
                    ("content-security-policy", "default-src 'none'; sandbox"),
                    ("cross-origin-resource-policy", "same-origin"),
                    ("referrer-policy", "no-referrer"),
                    ("x-frame-options", "DENY"),
                ];
                for (name, value) in expected {
                    assert_eq!(headers[name], value, "{name}");
                }
                let etag = format!("\"{}\"", "cd".repeat(32));
                assert_eq!(headers[header::ETAG], etag.as_str());
                assert!(!headers.contains_key(header::SET_COOKIE));
                assert!(
                    headers
                        .keys()
                        .all(|n| !n.as_str().starts_with("access-control-"))
                );
                // a direct visit, signed out, and a revalidation
                let direct = [
                    ("host", "baylee.app"),
                    ("sec-fetch-site", "none"),
                    ("x-forwarded-for", "198.51.100.7"),
                ];
                assert_eq!(
                    get_picture(&app, "GET", &uri(PICTURE, extension), &direct, LOOPBACK)
                        .await
                        .0,
                    StatusCode::OK
                );
                let mut again = PAGE.to_vec();
                again.push(("if-none-match", etag.as_str()));
                let (status, headers, body) =
                    get_picture(&app, "GET", &uri(PICTURE, extension), &again, LOOPBACK).await;
                assert_eq!((status, body.len()), (StatusCode::NOT_MODIFIED, 0));
                assert_eq!(headers["cache-control"], "private, max-age=3600");
            }
            assert!(
                mock.calls
                    .lock()
                    .unwrap()
                    .iter()
                    .all(|(_, cookie)| cookie.is_none())
            );
            // each file asked for by its own encoding
            let asked: Vec<String> = mock
                .calls
                .lock()
                .unwrap()
                .iter()
                .map(|(call, _)| call.clone())
                .collect();
            for encoding in ["Avif", "Png", "Sequence"] {
                assert!(
                    asked.contains(&format!("picture_bytes {encoding}")),
                    "{encoding}"
                );
            }
        }

        #[tokio::test]
        async fn every_refused_or_unknown_read_is_the_same_404() {
            let (app, _) = listener();
            let good = uri(PICTURE, "png");
            let unknown = uri([1; 16], "png");
            let upper = good
                .to_uppercase()
                .replace("/API/V1/PICTURES/", "/api/v1/pictures/")
                .replace(".PNG", ".png");
            let reference = get_picture(&app, "GET", &unknown, &PAGE, LOOPBACK).await;
            assert_eq!(reference.0, StatusCode::NOT_FOUND);
            assert_eq!(reference.1["cache-control"], "no-store");
            let mut cases: Vec<ReadCase> = vec![
                (
                    format!(
                        "/api/v1/pictures/{}",
                        central_api::picture::key_hex(&PICTURE)
                    ),
                    PAGE.to_vec(),
                    LOOPBACK,
                ),
                (good.replace(".png", ".svg"), PAGE.to_vec(), LOOPBACK),
                // another sequence's name, and an unknown key's sequence
                (
                    good.replace(".png", ".moving.avif"),
                    PAGE.to_vec(),
                    LOOPBACK,
                ),
                (
                    good.replace(".png", ".sequence.png"),
                    PAGE.to_vec(),
                    LOOPBACK,
                ),
                (uri([1; 16], "sequence.avif"), PAGE.to_vec(), LOOPBACK),
                (upper, PAGE.to_vec(), LOOPBACK),
                (good.clone(), PAGE.to_vec(), "192.0.2.1:50000"),
                (
                    good.clone(),
                    vec![
                        ("host", "evil.example"),
                        ("x-forwarded-for", "198.51.100.7"),
                    ],
                    LOOPBACK,
                ),
            ];
            for (name, value) in [
                ("sec-fetch-site", "cross-site"),
                ("sec-fetch-site", "same-site"),
                ("authorization", "Bearer x"),
            ] {
                let mut headers = PAGE.to_vec();
                headers.retain(|(n, _)| *n != name);
                headers.push((name, value));
                cases.push((good.clone(), headers, LOOPBACK));
            }
            for (uri, headers, peer) in cases {
                let (status, headers_out, body) =
                    get_picture(&app, "GET", &uri, &headers, peer).await;
                assert_eq!(
                    (status, body.as_slice()),
                    (reference.0, reference.2.as_slice()),
                    "{uri} {headers:?}"
                );
                assert_eq!(headers_out["cache-control"], "no-store");
                assert_eq!(
                    headers_out["content-security-policy"],
                    "default-src 'none'; sandbox"
                );
            }
        }

        #[tokio::test]
        async fn other_methods_are_refused_and_reads_have_a_budget() {
            let (app, _) = listener();
            let good = uri(PICTURE, "avif");
            for method in ["POST", "PUT", "DELETE", "PATCH"] {
                assert_eq!(
                    get_picture(&app, method, &good, &PAGE, LOOPBACK).await.0,
                    StatusCode::METHOD_NOT_ALLOWED,
                    "{method}"
                );
            }
            for method in ["GET", "POST", "DELETE"] {
                assert_eq!(
                    get_picture(&app, method, "/api/v1/picture", &PAGE, LOOPBACK)
                        .await
                        .0,
                    StatusCode::METHOD_NOT_ALLOWED,
                    "{method}"
                );
            }
            for _ in 0..super::super::pictures::READS_PER_MINUTE {
                assert_eq!(
                    get_picture(&app, "GET", &good, &PAGE, LOOPBACK).await.0,
                    StatusCode::OK
                );
            }
            assert_eq!(
                get_picture(&app, "GET", &good, &PAGE, LOOPBACK).await.0,
                StatusCode::TOO_MANY_REQUESTS
            );
            // another address keeps its own budget, and the calls their own
            let other = [
                ("host", "baylee.app"),
                ("sec-fetch-site", "same-origin"),
                ("x-forwarded-for", "198.51.100.8"),
            ];
            assert_eq!(
                get_picture(&app, "GET", &good, &other, LOOPBACK).await.0,
                StatusCode::OK
            );
            assert_eq!(send(&app, &base(), discover()).await.status, StatusCode::OK);
        }
    }
}
