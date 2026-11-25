//! The native push socket, `GET /v1/ws` (`docs/push-socket.md`): one
//! WebSocket per signed-in Baylee client, on which Central says that
//! something concerning the client's own account changed. It carries no
//! content: `ready`, `chat`, `social`, `presence` (the tags of the account's
//! online friends), `table` (the table a friend shares) and `revoked`, each a
//! small JSON text frame. The client reads what changed over `/v1/native`, as
//! it always did; the socket only saves it the polling.
//!
//! A socket is admitted on a connection whose first line asked for exactly
//! this route (`server.rs`), so every other connection keeps its one request
//! and no keep-alive. Admission costs, in order: a well-formed handshake, the
//! address's dials ([`ADDRESS_DIALS`]) and sockets ([`ADDRESS_SOCKETS`]), the
//! listener's ([`SOCKETS`]), a signed native session, and then the account
//! service's own checks (`central-identity`'s `live.rs`). Once upgraded, a
//! socket lives in the listener's set of sockets, not among its connections,
//! with its own idle limit, lifetime and shutdown.
mod frame;

use super::{in_flight::Client, native::failure};
use axum::{
    body::Body,
    extract::Request,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use base64::Engine as _;
use central_api::{
    Error, Token,
    presence::{LONGEST_TABLE_FRAME, Table, table_frame},
};
use central_identity::{IdentityService, Listener, Notice, Place, Presence, news};
use frame::{Code, Ended, Frame, Reader};
use hyper_util::rt::TokioIo;
use sha1::Digest as _;
use std::{
    collections::BTreeMap,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncWrite, AsyncWriteExt},
    net::TcpStream,
    sync::{mpsc, watch},
    time::Instant,
};

/// The first line of a connection that may become a push socket, exactly.
pub(crate) const REQUEST_LINE: &[u8] = b"GET /v1/ws HTTP/1.1\r\n";
const GAME_LINE: &[u8] = b"GET /v1/ranked/socket HTTP/1.1\r\n";
pub(crate) const BROWSER_LINE: &[u8] = b"GET /api/v1/ws HTTP/1.1\r\n";
/// How long a new connection has to show its first line before it is taken
/// for an ordinary one. A client sends its request's head at once.
const FIRST_LINE: Duration = Duration::from_secs(1);

/// Push sockets one listener holds at once. Each is a connection of its own
/// besides the listener's 128 (`central-service`), and the service's file
/// descriptors are 1,024 unless its unit raises them: 512 sockets, both
/// listeners' connections and the database's leave room below that.
pub(crate) const SOCKETS: usize = 512;
/// Push sockets one client address holds at once: a sixteenth, so filling
/// the listener takes sixteen addresses with eight signed-in accounts each
/// (four sockets an account, `central-identity`'s `ACCOUNT_SOCKETS`). An
/// address shared by a household, a club or a small office has room.
pub(crate) const ADDRESS_SOCKETS: u8 = 32;
/// Dials a minute from one client address, admitted or not. A client dials
/// at most every two seconds while its dials fail, and once a minute later;
/// this is thirty clients redialling at their fastest.
pub(crate) const ADDRESS_DIALS: u32 = 60;

/// How long the client has to take the upgrade once it was answered.
const UPGRADE_TIME: Duration = Duration::from_secs(5);
/// How often the server pings. The Baylee client answers each ping (its
/// socket library does) and sends nothing else.
const PING_EVERY: Duration = Duration::from_secs(30);
/// How long a socket may be silent, pongs included, before it is taken for
/// dead and dropped: two pings and a margin.
const IDLE: Duration = Duration::from_secs(75);
/// How long one frame may take to be written; a client that does not read
/// loses its socket rather than hold it.
const WRITE_TIME: Duration = Duration::from_secs(10);
/// How long a closing socket waits for the client's close.
const LINGER: Duration = Duration::from_secs(2);
/// The longest a socket lives: no session lives longer (12 hours after its
/// sign-in), so a socket this old is closed and its client may dial again.
const LIFETIME: Duration = Duration::from_hours(12);
/// The least time between two `presence` frames on one socket: news faster
/// than this waits, and the frame that follows says the latest.
const PRESENCE_GAP: Duration = Duration::from_secs(1);
/// Frames a client may send in [`FRAME_WINDOW`]: a pong for each ping, and
/// pings of its own if it likes, far fewer than this.
const FRAMES: u32 = 16;
const FRAME_WINDOW: Duration = Duration::from_secs(10);
/// How often every socket's session is checked, in one statement for all:
/// with [`CHECK_TIME`], an ended session's socket is told within 70 seconds.
const CHECK_EVERY: Duration = Duration::from_secs(60);
/// How long that check may take.
const CHECK_TIME: Duration = Duration::from_secs(10);
/// Checks failed in a row after which every socket closes: a session that
/// could not be checked for two minutes may have ended.
const CHECK_FAILURES: u32 = 2;

/// The events, exactly as the client reads them. `ready` announces that this
/// Central takes a table with a report and passes it on in `table` frames
/// (`docs/push-socket.md` §"Tables"): a client sends one to no Central that
/// did not say so, since an older one refuses the whole report for it.
const READY: &str = r#"{"event":"ready","tables":true}"#;
const CHAT: &str = r#"{"event":"chat"}"#;
const SOCIAL: &str = r#"{"event":"social"}"#;
const REVOKED: &str = r#"{"event":"revoked"}"#;

/// RFC 6455 § 1.3.
const GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// The longest `presence` frame: every friend an account may have, each
/// with the widest tag (eight hex digits, a positive `i32`) and its state's
/// letter, and the longest choice, fits a frame the client reads.
const _: () = assert!(
    r#"{"event":"presence","online":[],"states":"","me":"offline"}"#.len()
        + central_api::social::MAX_CONTACTS * (r#""7fffffff","#.len() + 1)
        <= frame::TEXT
);
/// The longest `table` frame, for a table within its bounds, fits one too:
/// each friend's table is a frame of its own, so no number of friends at
/// tables can make one longer.
const _: () = assert!(LONGEST_TABLE_FRAME <= frame::TEXT);

/// A socket's work, run in the listener's set of sockets.
pub(crate) type Job = Pin<Box<dyn Future<Output = ()> + Send>>;

/// What the listener gives each connection that asked for the socket.
#[derive(Clone)]
pub(crate) struct Upgrades(Arc<Shared>);

struct Shared {
    jobs: mpsc::Sender<Job>,
    stop: watch::Receiver<bool>,
    held: Mutex<Held>,
    dials: Mutex<super::browser::Book>,
}

/// The sockets held: each address's, and their sum.
#[derive(Default)]
struct Held {
    by_client: BTreeMap<Client, u8>,
    total: usize,
}

/// A socket's place among the listener's and its address's; given back
/// when it is dropped, however the socket ends.
pub(crate) struct Seat {
    shared: Arc<Shared>,
    client: Option<Client>,
}

impl Drop for Seat {
    fn drop(&mut self) {
        let mut held = self
            .shared
            .held
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        held.total = held.total.saturating_sub(1);
        if let Some(client) = self.client
            && let Some(count) = held.by_client.get_mut(&client)
        {
            *count = count.saturating_sub(1);
            if *count == 0 {
                held.by_client.remove(&client);
            }
        }
    }
}

impl Upgrades {
    /// A place for one more socket from `client`, unless the listener or
    /// the address holds all it may. A request whose address is not known
    /// (`boundary.rs`) meets the listener's limit alone.
    fn place(&self, client: Option<Client>) -> Option<Seat> {
        let mut held = self.0.held.lock().unwrap_or_else(PoisonError::into_inner);
        if held.total >= SOCKETS {
            return None;
        }
        if let Some(client) = client {
            let count = held.by_client.entry(client).or_insert(0);
            if *count >= ADDRESS_SOCKETS {
                return None;
            }
            *count += 1;
        }
        held.total += 1;
        Some(Seat {
            shared: Arc::clone(&self.0),
            client,
        })
    }

    /// Spend one of `client`'s dials in its minute.
    fn dial(&self, client: Client) -> bool {
        self.0
            .dials
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .charge(
                client,
                std::time::Instant::now(),
                ADDRESS_DIALS,
                Duration::from_secs(60),
            )
    }
}

/// The listener's side of the sockets: their work as it is admitted, and the
/// switch that stops them.
pub(crate) struct Sockets {
    pub(crate) upgrades: Upgrades,
    pub(crate) jobs: mpsc::Receiver<Job>,
    pub(crate) stop: watch::Sender<bool>,
}

impl Sockets {
    pub(crate) fn new() -> Self {
        // One more than the sockets: the periodic check of their sessions.
        let (jobs, receiver) = mpsc::channel(SOCKETS + 1);
        let (stop, stopped) = watch::channel(false);
        Self {
            upgrades: Upgrades(Arc::new(Shared {
                jobs,
                stop: stopped,
                held: Mutex::default(),
                dials: Mutex::default(),
            })),
            jobs: receiver,
            stop,
        }
    }

    /// Start the periodic check of every socket's session (`CHECK_EVERY`),
    /// which runs until the sockets are stopped.
    pub(crate) fn check(&self, identity: Arc<IdentityService>) {
        let stop = self.upgrades.0.stop.clone();
        // The channel is empty yet, so this always fits.
        let job = check(identity, CHECK_EVERY, CHECK_TIME, stop);
        let _ = self.upgrades.0.jobs.try_send(Box::pin(job));
    }
}

#[cfg(test)]
impl Upgrades {
    /// For the server's tests: a socket on `upgraded` that echoes what it
    /// reads until the sockets stop, then says `bye` and ends.
    pub(crate) fn echo(&self, upgraded: hyper::upgrade::OnUpgrade) {
        use tokio::io::AsyncReadExt;
        let mut stop = self.0.stop.clone();
        let _ = self.0.jobs.try_send(Box::pin(async move {
            let Ok(upgraded) = upgraded.await else {
                return;
            };
            let mut io = TokioIo::new(upgraded);
            let mut buffer = [0_u8; 64];
            loop {
                tokio::select! {
                    _ = stop.changed() => {
                        let _ = io.write_all(b"bye").await;
                        let _ = io.shutdown().await;
                        return;
                    }
                    read = io.read(&mut buffer) => match read {
                        Ok(0) | Err(_) => return,
                        Ok(read) => {
                            if io.write_all(&buffer[..read]).await.is_err() {
                                return;
                            }
                        }
                    }
                }
            }
        }));
    }

    /// For the server's tests: a socket's work that never ends by itself.
    pub(crate) fn stubborn(&self) {
        let _ = self.0.jobs.try_send(Box::pin(std::future::pending::<()>()));
    }
}

/// Whether the connection's first line asks for the push socket, read
/// without taking it from the connection. Anything else, a line that is
/// slow to come included, is an ordinary connection.
pub(crate) async fn asks(stream: &TcpStream) -> bool {
    const LINES: [&[u8]; 3] = [REQUEST_LINE, GAME_LINE, BROWSER_LINE];
    let deadline = Instant::now() + FIRST_LINE;
    let mut seen = [0_u8; GAME_LINE.len()];
    loop {
        let Ok(Ok(read)) = tokio::time::timeout_at(deadline, stream.peek(&mut seen)).await else {
            return false;
        };
        let received = &seen[..read];
        if LINES.iter().any(|line| received.starts_with(line)) {
            return true;
        }
        if read == 0 || !LINES.iter().any(|line| line.starts_with(received)) {
            return false;
        }
        // Peeking again at once would see the same bytes.
        if Instant::now() + Duration::from_millis(10) >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// The refusal of a request that is no handshake this socket takes.
fn invalid() -> Response {
    (StatusCode::BAD_REQUEST, "invalid request\n").into_response()
}

/// Why a handshake is refused.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Refusal {
    /// Not one this socket takes: 400.
    Invalid,
    /// Another version than 13: 426, naming 13 (RFC 6455 § 4.4).
    Version,
    Busy,
}

impl IntoResponse for Refusal {
    fn into_response(self) -> Response {
        match self {
            Self::Invalid => invalid(),
            Self::Busy => failure(Error::Busy),
            Self::Version => (
                StatusCode::UPGRADE_REQUIRED,
                [(header::SEC_WEBSOCKET_VERSION, "13")],
                "invalid request\n",
            )
                .into_response(),
        }
    }
}

/// The one value of a header that must come once.
fn single(headers: &HeaderMap, name: impl header::AsHeaderName) -> Option<&str> {
    let mut values = headers.get_all(name).iter();
    let value = values.next()?;
    if values.next().is_some() {
        return None;
    }
    value.to_str().ok()
}

/// Whether a comma-separated header names `token`, in any of its values.
fn names(headers: &HeaderMap, name: header::HeaderName, token: &str) -> bool {
    headers.get_all(name).iter().any(|value| {
        value.to_str().is_ok_and(|value| {
            value
                .split(',')
                .any(|part| part.trim().eq_ignore_ascii_case(token))
        })
    })
}

/// The accept value for a well-formed opening handshake (RFC 6455 § 4.2.1),
/// or its refusal: a request with a body, without `Upgrade: websocket` and
/// `Connection: upgrade`, with another version than 13 (426, naming 13), or
/// with a key that is not sixteen bytes in canonical Base64.
fn handshake(headers: &HeaderMap) -> Result<HeaderValue, Refusal> {
    if headers.contains_key(header::CONTENT_LENGTH)
        || headers.contains_key(header::TRANSFER_ENCODING)
    {
        return Err(Refusal::Invalid);
    }
    let upgrade = single(headers, header::UPGRADE).is_some_and(|value| {
        value
            .split(',')
            .any(|part| part.trim().eq_ignore_ascii_case("websocket"))
    });
    if !upgrade || !names(headers, header::CONNECTION, "upgrade") {
        return Err(Refusal::Invalid);
    }
    if single(headers, header::SEC_WEBSOCKET_VERSION) != Some("13") {
        return Err(Refusal::Version);
    }
    let key = single(headers, header::SEC_WEBSOCKET_KEY).ok_or(Refusal::Invalid)?;
    accept(key).ok_or(Refusal::Invalid)
}

/// `Sec-WebSocket-Accept` for `key`: Base64 of the SHA-1 of the key and the
/// protocol's GUID. `None` unless the key is sixteen bytes in canonical Base64
/// (the standard engine refuses padding that is not canonical and bits past
/// the last byte, so what it decodes is what it would encode).
pub(super) fn accept(key: &str) -> Option<HeaderValue> {
    let engine = base64::engine::general_purpose::STANDARD;
    if engine.decode(key).ok()?.len() != 16 {
        return None;
    }
    let digest = sha1::Sha1::new()
        .chain_update(key.as_bytes())
        .chain_update(GUID)
        .finalize();
    HeaderValue::from_str(&engine.encode(digest)).ok()
}

/// Service capabilities needed to run an upgraded browser push socket.
pub(crate) trait PushPeer: Send + Sync + 'static {
    fn listen_browser(
        &self,
        cookie: &Token,
        place: Place,
    ) -> impl Future<Output = Result<Listener, Error>> + Send;
    fn presence(&self, listener: &Listener) -> Presence;
    fn listening(&self, listener: &Listener) -> impl Future<Output = Result<bool, Error>> + Send;
}

impl PushPeer for IdentityService {
    fn listen_browser(
        &self,
        cookie: &Token,
        place: Place,
    ) -> impl Future<Output = Result<Listener, Error>> + Send {
        Self::listen_browser(self, cookie, Some(place))
    }

    fn presence(&self, listener: &Listener) -> Presence {
        Self::presence(self, listener)
    }

    fn listening(&self, listener: &Listener) -> impl Future<Output = Result<bool, Error>> + Send {
        Self::listening(self, listener)
    }
}

/// `GET /api/v1/ws` from a same-origin browser session (`browser.rs`):
/// validates the WebSocket handshake, `__Host-central` cookie, and address
/// limits before upgrading to structured browser frames.
pub(crate) async fn dial_browser<P: PushPeer>(
    peer: Arc<P>,
    from: Client,
    cookie: Option<Token>,
    request: Request,
) -> Response {
    let accept = match handshake(request.headers()) {
        Ok(accept) => accept,
        Err(refusal) => return refusal.into_response(),
    };
    let Some(cookie) = cookie else {
        return failure(Error::Rejected);
    };
    let opening = match Opening::admit_accepted(&request, Some(from), accept) {
        Ok(opening) => opening,
        Err(refusal) => return refusal.into_response(),
    };
    let listener = match peer.listen_browser(&cookie, Place::from(from)).await {
        Ok(listener) => listener,
        Err(error) => return failure(error),
    };
    opening.start(request, |place, upgraded, stop| {
        serve_with(peer, listener, place, upgraded, stop, true)
    })
}

/// `GET /v1/ws` from `from` (`native.rs`): admit a socket, or refuse with the
/// native route's vocabulary.
pub(crate) async fn dial(
    identity: &Arc<IdentityService>,
    from: Option<Client>,
    request: Request,
) -> Response {
    let opening = match Opening::admit(&request, from) {
        Ok(opening) => opening,
        Err(refusal) => return refusal.into_response(),
    };
    let Some(bearer) = single(request.headers(), header::AUTHORIZATION) else {
        return failure(Error::Rejected);
    };
    let at = from.map(Place::from);
    let listener = match identity.present(bearer, at) {
        Ok(presented) => identity.listen(&presented, at).await,
        Err(error) => Err(error),
    };
    let listener = match listener {
        Ok(listener) => listener,
        Err(error) => return failure(error),
    };
    let peer = Arc::clone(identity);
    opening.start(request, |place, upgraded, stop| {
        serve_with(peer, listener, place, upgraded, stop, false)
    })
}

/// WebSocket routes share the same listener and address budgets.
pub(super) struct Opening {
    upgrades: Upgrades,
    place: Seat,
    accept: HeaderValue,
}

impl Opening {
    pub(super) fn admit(request: &Request, from: Option<Client>) -> Result<Self, Refusal> {
        let accept = handshake(request.headers())?;
        Self::admit_accepted(request, from, accept)
    }

    fn admit_accepted(
        request: &Request,
        from: Option<Client>,
        accept: HeaderValue,
    ) -> Result<Self, Refusal> {
        let upgrades = request
            .extensions()
            .get::<Upgrades>()
            .cloned()
            .ok_or(Refusal::Invalid)?;
        if from.is_some_and(|client| !upgrades.dial(client)) {
            return Err(Refusal::Busy);
        }
        let place = upgrades.place(from).ok_or(Refusal::Busy)?;
        Ok(Self {
            upgrades,
            place,
            accept,
        })
    }

    pub(super) fn start<F>(
        self,
        mut request: Request,
        serve: impl FnOnce(Seat, hyper::upgrade::OnUpgrade, watch::Receiver<bool>) -> F,
    ) -> Response
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let upgraded = hyper::upgrade::on(&mut request);
        let job = serve(self.place, upgraded, self.upgrades.0.stop.clone());
        if self.upgrades.0.jobs.try_send(Box::pin(job)).is_err() {
            return failure(Error::Busy);
        }
        let mut response = Response::new(Body::empty());
        *response.status_mut() = StatusCode::SWITCHING_PROTOCOLS;
        let headers = response.headers_mut();
        headers.insert(header::UPGRADE, HeaderValue::from_static("websocket"));
        headers.insert(header::CONNECTION, HeaderValue::from_static("upgrade"));
        headers.insert(header::SEC_WEBSOCKET_ACCEPT, self.accept);
        response
    }
}

/// Every non-upgrade answer on a connection that asked for the socket ends
/// it, as every answer on an ordinary connection does (no keep-alive).
pub(crate) fn closing(mut response: Response) -> Response {
    if response.status() != StatusCode::SWITCHING_PROTOCOLS {
        response
            .headers_mut()
            .insert(header::CONNECTION, HeaderValue::from_static("close"));
    }
    response
}

/// One socket, from the upgrade to its end.
async fn serve_with<P: PushPeer>(
    peer: Arc<P>,
    listener: Listener,
    place: Seat,
    upgraded: hyper::upgrade::OnUpgrade,
    stop: watch::Receiver<bool>,
    structured: bool,
) {
    let _place = place;
    let Ok(Ok(upgraded)) = tokio::time::timeout(UPGRADE_TIME, upgraded).await else {
        return;
    };
    let (reader, writer) = tokio::io::split(TokioIo::new(upgraded));
    let account = Served { peer, listener };
    if structured {
        Socket::new_browser(account, reader, writer, stop, Times::SERVED)
            .run()
            .await;
    } else {
        Socket::new(account, reader, writer, stop, Times::SERVED)
            .run()
            .await;
    }
}

/// What a socket asks of the account service: its doorbell, its account's
/// presence and whether its session stands (`central-identity`'s `live.rs`).
trait Account {
    /// The bits raised since the last call ([`news`]).
    fn take(&self) -> u8;
    /// Clear raised bits and drain structured notices for a browser socket.
    fn take_browser(&self) -> (u8, Vec<Notice>) {
        (self.take(), Vec::new())
    }
    /// Update visibility/idle away state signaled over a browser push socket.
    fn set_away(&self, _away: bool) -> bool {
        false
    }
    /// Wait until a bit is raised; cancel-safe.
    fn rung(&self) -> impl Future<Output = ()> + Send;
    /// The socket said `ready`.
    fn ready(&self);
    /// The account's friends shown online, their states, the tables they
    /// share, its own choice.
    fn presence(&self) -> Presence;
    /// Whether the session stands; an error is no answer.
    fn listening(&self) -> impl Future<Output = Result<bool, Error>> + Send;
}

/// The socket's account as the service serves it.
struct Served<P> {
    peer: Arc<P>,
    listener: Listener,
}

impl<P: PushPeer> Account for Served<P> {
    fn take(&self) -> u8 {
        self.listener.take()
    }

    fn take_browser(&self) -> (u8, Vec<Notice>) {
        self.listener.take_browser()
    }

    fn set_away(&self, away: bool) -> bool {
        self.listener.set_away(away)
    }

    fn rung(&self) -> impl Future<Output = ()> + Send {
        self.listener.rung()
    }

    fn ready(&self) {
        self.listener.ready();
    }

    fn presence(&self) -> Presence {
        self.peer.presence(&self.listener)
    }

    fn listening(&self) -> impl Future<Output = Result<bool, Error>> + Send {
        self.peer.listening(&self.listener)
    }
}

/// A socket's times and its client's frame budget; constants when served,
/// shorter in a test.
#[derive(Clone, Copy)]
struct Times {
    ping_every: Duration,
    idle: Duration,
    write: Duration,
    linger: Duration,
    lifetime: Duration,
    check: Duration,
    frames: u32,
    frame_window: Duration,
    presence_gap: Duration,
}

impl Times {
    const SERVED: Self = Self {
        ping_every: PING_EVERY,
        idle: IDLE,
        write: WRITE_TIME,
        linger: LINGER,
        lifetime: LIFETIME,
        check: CHECK_TIME,
        frames: FRAMES,
        frame_window: FRAME_WINDOW,
        presence_gap: PRESENCE_GAP,
    };
}

/// Why a socket ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum End {
    /// Without a word: the connection is gone or dead, or a write failed.
    Drop,
    /// With a close frame, then waiting briefly for the client's.
    Close(Code),
    /// `revoked`, then a close: the session ended.
    Revoked,
    /// The client closed: answer its close with its code, or 1000.
    Answer(Code),
}

struct Socket<A, R, W> {
    account: A,
    reader: R,
    writer: W,
    frames: Reader,
    structured: bool,
    stop: watch::Receiver<bool>,
    times: Times,
    /// The presence last said (its tables apart), and when it or a table was.
    sent: Option<Presence>,
    said_at: Option<Instant>,
    /// The friends' tables last said, by tag.
    tables: BTreeMap<u32, Arc<Table>>,
    /// When a presence held back by [`Times::presence_gap`] is said.
    due: Option<Instant>,
}

impl<A: Account, R: AsyncRead + Unpin, W: AsyncWrite + Unpin> Socket<A, R, W> {
    const fn new(
        account: A,
        reader: R,
        writer: W,
        stop: watch::Receiver<bool>,
        times: Times,
    ) -> Self {
        Self::with_mode(account, reader, writer, stop, times, false)
    }

    const fn new_browser(
        account: A,
        reader: R,
        writer: W,
        stop: watch::Receiver<bool>,
        times: Times,
    ) -> Self {
        Self::with_mode(account, reader, writer, stop, times, true)
    }

    const fn with_mode(
        account: A,
        reader: R,
        writer: W,
        stop: watch::Receiver<bool>,
        times: Times,
        structured: bool,
    ) -> Self {
        Self {
            account,
            reader,
            writer,
            frames: if structured {
                Reader::browser()
            } else {
                Reader::new()
            },
            structured,
            stop,
            times,
            sent: None,
            said_at: None,
            tables: BTreeMap::new(),
            due: None,
        }
    }

    /// Serve the socket to its end; how it ended, for a test.
    async fn run(mut self) -> End {
        let end = self.talk().await;
        self.end(end).await;
        end
    }

    async fn talk(&mut self) -> End {
        if self
            .write(&frame::text(READY).unwrap_or_default())
            .await
            .is_err()
        {
            return End::Drop;
        }
        // Online from here; and its own presence first, before any news.
        self.account.ready();
        if self.presence().await.is_err() {
            return End::Drop;
        }
        let started = Instant::now();
        let mut heard = started;
        let mut ping =
            tokio::time::interval_at(started + self.times.ping_every, self.times.ping_every);
        ping.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut window = (started, 0_u32);
        loop {
            if *self.stop.borrow() {
                return End::Close(Code::GOING_AWAY);
            }
            let (bits, notices) = if self.structured {
                self.account.take_browser()
            } else {
                (self.account.take(), Vec::new())
            };
            if bits != 0 {
                match self.act_with(bits, notices).await {
                    Ok(None) => continue,
                    Ok(Some(end)) => return end,
                    Err(()) => return End::Drop,
                }
            }
            let due = self.due;
            // Each branch but its body is cancel-safe: the doorbell keeps its
            // bits, the reader its bytes, the timers their deadlines.
            tokio::select! {
                biased;
                _ = self.stop.changed() => return End::Close(Code::GOING_AWAY),
                () = self.account.rung() => {}
                () = tokio::time::sleep_until(due.unwrap_or(started)), if due.is_some() => {
                    self.due = None;
                    if self.presence().await.is_err() {
                        return End::Drop;
                    }
                }
                read = self.frames.next(&mut self.reader) => {
                    let now = Instant::now();
                    if now.duration_since(window.0) >= self.times.frame_window {
                        window = (now, 0);
                    }
                    window.1 += 1;
                    if window.1 > self.times.frames {
                        return End::Close(Code::POLICY);
                    }
                    match read {
                        Ok(frame) => {
                            if let Some(end) = self.handle_client_frame(frame, &mut heard, now).await {
                                return end;
                            }
                        }
                        Err(Ended::Gone) => return End::Drop,
                        Err(Ended::Refused(code)) => return End::Close(code),
                    }
                }
                _ = ping.tick() => {
                    if self.write(&frame::PING).await.is_err() {
                        return End::Drop;
                    }
                }
                () = tokio::time::sleep_until(heard + self.times.idle) => return End::Drop,
                () = tokio::time::sleep_until(started + self.times.lifetime) => {
                    return End::Close(Code::GOING_AWAY);
                }
            }
        }
    }

    async fn handle_client_frame(
        &mut self,
        frame: Frame,
        heard: &mut Instant,
        now: Instant,
    ) -> Option<End> {
        match frame {
            Frame::Text(text) => {
                if !self.structured {
                    return Some(End::Close(Code::UNSUPPORTED));
                }
                *heard = now;
                let Some(away) = parse_client_presence(&text) else {
                    return Some(End::Close(Code::UNSUPPORTED));
                };
                let _ = self.account.set_away(away);
                None
            }
            Frame::Ping(payload) => {
                *heard = now;
                self.write(&frame::pong(&payload))
                    .await
                    .is_err()
                    .then_some(End::Drop)
            }
            Frame::Pong => {
                *heard = now;
                None
            }
            Frame::Close(code) => Some(End::Answer(code.unwrap_or(Code::NORMAL))),
        }
    }

    #[cfg(test)]
    async fn act(&mut self, bits: u8) -> Result<Option<End>, ()> {
        self.act_with(bits, Vec::new()).await
    }

    /// Act on the doorbell's bits: an ended session first, then a check of
    /// it, then the news, `social` before `chat` (a client reads both on
    /// `social`), then presence if it changed.
    async fn act_with(&mut self, bits: u8, notices: Vec<Notice>) -> Result<Option<End>, ()> {
        if bits & news::REVOKED != 0 {
            return Ok(Some(End::Revoked));
        }
        if bits & news::STALE != 0 {
            return Ok(Some(End::Close(Code::INTERNAL)));
        }
        if bits & news::CHECK != 0 {
            let checked = tokio::time::timeout(self.times.check, self.account.listening()).await;
            // Only the store's answer that the session is gone revokes it; a
            // store that cannot answer is asked again by the periodic check.
            if matches!(checked, Ok(Ok(false))) {
                return Ok(Some(End::Revoked));
            }
        }
        if self.structured {
            for notice in &notices {
                let text = notice_frame(notice);
                self.write(&frame::text(&text).unwrap_or_default()).await?;
            }
        } else {
            if bits & news::SOCIAL != 0 {
                self.write(&frame::text(SOCIAL).unwrap_or_default()).await?;
            }
            if bits & news::CHAT != 0 {
                self.write(&frame::text(CHAT).unwrap_or_default()).await?;
            }
        }
        if bits & news::PRESENCE != 0 {
            // At most one presence a gap: news within it waits, and what is
            // said then is the latest.
            match self.said_at {
                Some(at) if at.elapsed() < self.times.presence_gap => {
                    self.due.get_or_insert(at + self.times.presence_gap);
                }
                _ => self.presence().await?,
            }
        }
        Ok(None)
    }

    /// Say the account's presence, unless it is what was said last: first a
    /// `table` frame for each friend whose shared table is not the one said
    /// (and one without a table for each that shares none any more), so that
    /// a client has the table when the frame that shows the friend at it
    /// comes; then the `presence` frame if the friends, their states or the
    /// choice changed. All of it is one write, with one write's time: so a
    /// pass costs the socket one wait however many friends' tables changed,
    /// and holds at most 300 table frames (796 bytes each at the longest) and
    /// one presence. Whatever was held back is said with it: the gap begins
    /// anew, and a deadline from before would otherwise send the next change
    /// at once.
    async fn presence(&mut self) -> Result<(), ()> {
        self.due = None;
        let mut now = self.account.presence();
        let tables = std::mem::take(&mut now.tables);
        let mut out = Vec::new();
        for (tag, table) in &tables {
            if self.tables.get(tag) != Some(table) {
                let text = table_frame(*tag, Some(&**table)).ok_or(())?;
                out.extend(frame::text(&text).ok_or(())?);
            }
        }
        for tag in self.tables.keys().filter(|tag| !tables.contains_key(*tag)) {
            let text = table_frame(*tag, None).ok_or(())?;
            out.extend(frame::text(&text).ok_or(())?);
        }
        let changed = self.sent.as_ref() != Some(&now);
        if changed {
            out.extend(frame::text(&presence(&now)).ok_or(())?);
        }
        if out.is_empty() {
            return Ok(());
        }
        self.write(&out).await?;
        self.tables = tables;
        if changed {
            self.sent = Some(now);
        }
        self.said_at = Some(Instant::now());
        Ok(())
    }

    /// Write one whole frame within its time. Never inside a `select!`: a
    /// write cut short would leave half a frame.
    async fn write(&mut self, bytes: &[u8]) -> Result<(), ()> {
        let written = async {
            self.writer.write_all(bytes).await?;
            self.writer.flush().await
        };
        match tokio::time::timeout(self.times.write, written).await {
            Ok(Ok(())) => Ok(()),
            _ => Err(()),
        }
    }

    async fn end(&mut self, end: End) {
        let code = match end {
            End::Drop => return,
            End::Answer(code) => {
                let _ = self.write(&frame::close(code)).await;
                let _ = tokio::time::timeout(self.times.write, self.writer.shutdown()).await;
                return;
            }
            End::Revoked => {
                if self
                    .write(&frame::text(REVOKED).unwrap_or_default())
                    .await
                    .is_err()
                {
                    return;
                }
                Code::POLICY
            }
            End::Close(code) => code,
        };
        if self.write(&frame::close(code)).await.is_err() {
            return;
        }
        // The client's close, or its end, or the linger's.
        let _ = tokio::time::timeout(self.times.linger, async {
            loop {
                match self.frames.next(&mut self.reader).await {
                    Ok(Frame::Close(_)) | Err(_) => break,
                    Ok(_) => {}
                }
            }
        })
        .await;
        let _ = tokio::time::timeout(self.times.write, self.writer.shutdown()).await;
    }
}

/// `{"event":"presence","online":[…],"states":"…","me":"…"}`: each tag as the
/// client writes it (lower-case hexadecimal, at least four digits), then one
/// letter per tag for its state (`central_api::presence::Shown`), then the
/// account's own choice. A client that reads only `online` sees every friend
/// shown, away or busy, as online.
fn presence(now: &Presence) -> String {
    use std::fmt::Write as _;
    let mut text = String::from(r#"{"event":"presence","online":["#);
    for (index, tag) in now.online.iter().enumerate() {
        if index > 0 {
            text.push(',');
        }
        let _ = write!(text, r#""{tag:04x}""#);
    }
    text.push_str(r#"],"states":""#);
    text.extend(now.states.iter().map(|state| state.letter()));
    let _ = write!(text, r#"","me":"{}"}}"#, now.me.name());
    text
}

fn write_json_str(out: &mut String, value: &str) {
    use std::fmt::Write as _;
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str(r#"\""#),
            '\\' => out.push_str(r"\\"),
            '\x08' => out.push_str(r"\b"),
            '\x0c' => out.push_str(r"\f"),
            '\n' => out.push_str(r"\n"),
            '\r' => out.push_str(r"\r"),
            '\t' => out.push_str(r"\t"),
            c if u32::from(c) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn notice_frame(notice: &Notice) -> String {
    use std::fmt::Write as _;
    match notice {
        Notice::ChatMessage { conv, id, at, from } => {
            let mut out = String::from(r#"{"event":"chat","kind":"message","conv":"#);
            write_json_str(&mut out, conv);
            out.push_str(r#","id":"#);
            write_json_str(&mut out, id);
            let _ = write!(out, r#","at":{at}"#);
            if let Some(tag) = from {
                let _ = write!(out, r#","from":"{tag:04x}""#);
            }
            out.push('}');
            if out.len() <= frame::TEXT {
                out
            } else {
                CHAT.to_owned()
            }
        }
        Notice::Thread { action, group } => {
            let mut out = String::from(r#"{"event":"social","kind":"thread","action":"#);
            write_json_str(&mut out, action);
            out.push_str(r#","group":"#);
            write_json_str(&mut out, group);
            out.push('}');
            if out.len() <= frame::TEXT {
                out
            } else {
                SOCIAL.to_owned()
            }
        }
        Notice::FriendRequest { tag } => {
            format!(r#"{{"event":"social","kind":"friend_request","tag":"{tag:04x}"}}"#)
        }
        Notice::FriendAccept { tag, via_invite } => {
            let via = if *via_invite { "invite" } else { "contact" };
            format!(
                r#"{{"event":"social","kind":"friend_accept","tag":"{tag:04x}","via":"{via}"}}"#
            )
        }
        Notice::ContactChanged { tag } => {
            format!(r#"{{"event":"social","kind":"contact_changed","tag":"{tag:04x}"}}"#)
        }
        Notice::ChatSync => CHAT.to_owned(),
        Notice::SocialSync => SOCIAL.to_owned(),
    }
}

fn parse_client_presence(input: &str) -> Option<bool> {
    let mut rest = input.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    if rest.is_empty() {
        return None;
    }
    let mut event_ok = false;
    let mut seen_event = false;
    let mut away: Option<bool> = None;
    loop {
        rest = rest.trim_start();
        let (key, after_key) = parse_json_str_literal(rest)?;
        rest = after_key.trim_start().strip_prefix(':')?.trim_start();
        match key {
            "event" if !seen_event => {
                let (value, after_val) = parse_json_str_literal(rest)?;
                seen_event = true;
                event_ok = value == "presence";
                rest = after_val.trim_start();
            }
            "away" if away.is_none() => {
                if let Some(after_val) = rest.strip_prefix("true") {
                    away = Some(true);
                    rest = after_val.trim_start();
                } else {
                    let after_val = rest.strip_prefix("false")?;
                    away = Some(false);
                    rest = after_val.trim_start();
                }
            }
            _ => return None,
        }
        if rest.is_empty() {
            break;
        }
        rest = rest.strip_prefix(',')?;
        if rest.trim_start().is_empty() {
            return None;
        }
    }
    if event_ok { away } else { None }
}

fn parse_json_str_literal(input: &str) -> Option<(&str, &str)> {
    let after_open = input.strip_prefix('"')?;
    let end = after_open.find('"')?;
    let inner = &after_open[..end];
    if inner.bytes().any(|b| b == b'\\' || b < 0x20) {
        return None;
    }
    Some((inner, &after_open[end + 1..]))
}

/// What the periodic check asks of the account service.
trait Sessions {
    /// Check every socket's session; an error is no answer.
    fn check(&self) -> impl Future<Output = Result<(), Error>> + Send;
    /// The check failed too often: every socket closes without `revoked`.
    fn unchecked(&self);
}

impl Sessions for Arc<IdentityService> {
    fn check(&self) -> impl Future<Output = Result<(), Error>> + Send {
        self.check_listeners()
    }

    fn unchecked(&self) {
        self.listeners_unchecked();
    }
}

/// The periodic check of every socket's session, `every` so often and each
/// within `time`, until the sockets stop; [`CHECK_FAILURES`] in a row that
/// failed or took too long close every socket.
async fn check(
    sessions: impl Sessions,
    every: Duration,
    time: Duration,
    mut stop: watch::Receiver<bool>,
) {
    let mut ticks = tokio::time::interval_at(Instant::now() + every, every);
    ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut failed = 0_u32;
    loop {
        tokio::select! {
            biased;
            _ = stop.changed() => return,
            _ = ticks.tick() => {}
        }
        let checked = tokio::time::timeout(time, sessions.check()).await;
        if matches!(checked, Ok(Ok(()))) {
            failed = 0;
        } else {
            failed += 1;
            if failed >= CHECK_FAILURES {
                sessions.unchecked();
                failed = 0;
            }
        }
    }
}

#[cfg(test)]
mod tests;
