//! The native push socket, `GET /v1/ws` (`docs/push-socket.md`): one
//! WebSocket per signed-in Baylee client, on which Central says that
//! something concerning the client's own account changed. It carries no
//! content: `ready`, `chat`, `social`, `presence` (the tags of the account's
//! online friends) and `revoked`, each a small JSON text frame. The client
//! reads what changed over `/v1/native`, as it always did; the socket only
//! saves it the polling.
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
use central_api::Error;
use central_identity::{IdentityService, Listener, Place, news};
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
/// Frames a client may send in [`FRAME_WINDOW`]: a pong for each ping, and
/// pings of its own if it likes, far fewer than this.
const FRAMES: u32 = 16;
const FRAME_WINDOW: Duration = Duration::from_secs(10);
/// How often every socket's session is checked, in one statement for all.
const CHECK_EVERY: Duration = Duration::from_secs(60);
/// How long that check may take.
const CHECK_TIME: Duration = Duration::from_secs(10);
/// Checks failed in a row after which every socket closes: a session that
/// could not be checked for two minutes may have ended.
const CHECK_FAILURES: u32 = 2;

/// The events, exactly as the client reads them.
const READY: &str = r#"{"event":"ready"}"#;
const CHAT: &str = r#"{"event":"chat"}"#;
const SOCIAL: &str = r#"{"event":"social"}"#;
const REVOKED: &str = r#"{"event":"revoked"}"#;

/// RFC 6455 § 1.3.
const GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// The longest `presence` frame: every friend an account may have, each
/// with the widest tag (eight hex digits, a positive `i32`), fits a frame
/// the client reads.
const _: () = assert!(
    r#"{"event":"presence","online":[]}"#.len()
        + central_api::social::MAX_CONTACTS * r#""7fffffff","#.len()
        <= frame::TEXT
);

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
        let _ = self
            .upgrades
            .0
            .jobs
            .try_send(Box::pin(check(identity, stop)));
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
    let deadline = Instant::now() + FIRST_LINE;
    let mut seen = [0_u8; REQUEST_LINE.len()];
    loop {
        let Ok(Ok(read)) = tokio::time::timeout_at(deadline, stream.peek(&mut seen)).await else {
            return false;
        };
        if read == 0 || !REQUEST_LINE.starts_with(&seen[..read]) {
            return false;
        }
        if read == REQUEST_LINE.len() {
            return true;
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
enum Refusal {
    /// Not one this socket takes: 400.
    Invalid,
    /// Another version than 13: 426, naming 13 (RFC 6455 § 4.4).
    Version,
}

impl IntoResponse for Refusal {
    fn into_response(self) -> Response {
        match self {
            Self::Invalid => invalid(),
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
/// protocol's GUID. `None` unless the key is sixteen bytes in canonical Base64.
fn accept(key: &str) -> Option<HeaderValue> {
    let engine = base64::engine::general_purpose::STANDARD;
    let decoded = engine.decode(key).ok()?;
    if decoded.len() != 16 || engine.encode(&decoded) != key {
        return None;
    }
    let digest = sha1::Sha1::new()
        .chain_update(key.as_bytes())
        .chain_update(GUID)
        .finalize();
    HeaderValue::from_str(&engine.encode(digest)).ok()
}

/// `GET /v1/ws` from `from` (`native.rs`): admit a socket, or refuse with the
/// native route's vocabulary.
pub(crate) async fn dial(
    identity: &Arc<IdentityService>,
    from: Option<Client>,
    mut request: Request,
) -> Response {
    // Only a connection that asked in its first line can be upgraded.
    let Some(upgrades) = request.extensions().get::<Upgrades>().cloned() else {
        return invalid();
    };
    let accept = match handshake(request.headers()) {
        Ok(accept) => accept,
        Err(refusal) => return refusal.into_response(),
    };
    if let Some(client) = from
        && !upgrades.dial(client)
    {
        return failure(Error::Busy);
    }
    let Some(place) = upgrades.place(from) else {
        return failure(Error::Busy);
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
    let on_upgrade = hyper::upgrade::on(&mut request);
    let job = serve(
        Arc::clone(identity),
        listener,
        place,
        on_upgrade,
        upgrades.0.stop.clone(),
    );
    if upgrades.0.jobs.try_send(Box::pin(job)).is_err() {
        return failure(Error::Busy);
    }
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::SWITCHING_PROTOCOLS;
    let headers = response.headers_mut();
    headers.insert(header::UPGRADE, HeaderValue::from_static("websocket"));
    headers.insert(header::CONNECTION, HeaderValue::from_static("upgrade"));
    headers.insert(header::SEC_WEBSOCKET_ACCEPT, accept);
    response
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
async fn serve(
    identity: Arc<IdentityService>,
    listener: Listener,
    place: Seat,
    upgraded: hyper::upgrade::OnUpgrade,
    stop: watch::Receiver<bool>,
) {
    let _place = place;
    let Ok(Ok(upgraded)) = tokio::time::timeout(UPGRADE_TIME, upgraded).await else {
        return;
    };
    let (reader, writer) = tokio::io::split(TokioIo::new(upgraded));
    let account = Served { identity, listener };
    Socket::new(account, reader, writer, stop, Times::SERVED)
        .run()
        .await;
}

/// What a socket asks of the account service: its doorbell, its account's
/// presence and whether its session stands (`central-identity`'s `live.rs`).
trait Account {
    /// The bits raised since the last call ([`news`]).
    fn take(&self) -> u8;
    /// Wait until a bit is raised; cancel-safe.
    fn rung(&self) -> impl Future<Output = ()> + Send;
    /// The socket said `ready`.
    fn ready(&self);
    /// The account's online friends' tags, in order.
    fn presence(&self) -> Vec<u32>;
    /// Whether the session stands; an error is no answer.
    fn listening(&self) -> impl Future<Output = Result<bool, Error>> + Send;
}

/// The socket's account as the service serves it.
struct Served {
    identity: Arc<IdentityService>,
    listener: Listener,
}

impl Account for Served {
    fn take(&self) -> u8 {
        self.listener.take()
    }

    fn rung(&self) -> impl Future<Output = ()> + Send {
        self.listener.rung()
    }

    fn ready(&self) {
        self.listener.ready();
    }

    fn presence(&self) -> Vec<u32> {
        self.identity.presence(&self.listener)
    }

    fn listening(&self) -> impl Future<Output = Result<bool, Error>> + Send {
        self.identity.listening(&self.listener)
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
    /// The client closed: answer its close.
    Answer,
}

struct Socket<A, R, W> {
    account: A,
    reader: R,
    writer: W,
    frames: Reader,
    stop: watch::Receiver<bool>,
    times: Times,
    /// The presence last said.
    sent: Option<Vec<u32>>,
}

impl<A: Account, R: AsyncRead + Unpin, W: AsyncWrite + Unpin> Socket<A, R, W> {
    const fn new(
        account: A,
        reader: R,
        writer: W,
        stop: watch::Receiver<bool>,
        times: Times,
    ) -> Self {
        Self {
            account,
            reader,
            writer,
            frames: Reader::new(),
            stop,
            times,
            sent: None,
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
            let bits = self.account.take();
            if bits != 0 {
                match self.act(bits).await {
                    Ok(None) => continue,
                    Ok(Some(end)) => return end,
                    Err(()) => return End::Drop,
                }
            }
            // Each branch but its body is cancel-safe: the doorbell keeps its
            // bits, the reader its bytes, the timers their deadlines.
            tokio::select! {
                biased;
                _ = self.stop.changed() => return End::Close(Code::GOING_AWAY),
                () = self.account.rung() => {}
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
                        Ok(Frame::Ping(payload)) => {
                            heard = now;
                            if self.write(&frame::pong(&payload)).await.is_err() {
                                return End::Drop;
                            }
                        }
                        Ok(Frame::Pong) => heard = now,
                        Ok(Frame::Close) => return End::Answer,
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

    /// Act on the doorbell's bits: an ended session first, then a check of
    /// it, then the news, `social` before `chat` (a client reads both on
    /// `social`), then presence if it changed.
    async fn act(&mut self, bits: u8) -> Result<Option<End>, ()> {
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
        if bits & news::SOCIAL != 0 {
            self.write(&frame::text(SOCIAL).unwrap_or_default()).await?;
        }
        if bits & news::CHAT != 0 {
            self.write(&frame::text(CHAT).unwrap_or_default()).await?;
        }
        if bits & news::PRESENCE != 0 {
            self.presence().await?;
        }
        Ok(None)
    }

    /// Say the account's online friends, unless they are what was said last.
    async fn presence(&mut self) -> Result<(), ()> {
        let online = self.account.presence();
        if self.sent.as_ref() == Some(&online) {
            return Ok(());
        }
        let text = presence(&online);
        self.write(&frame::text(&text).ok_or(())?).await?;
        self.sent = Some(online);
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
            End::Answer => {
                let _ = self.write(&frame::close(Code::NORMAL)).await;
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
                    Ok(Frame::Close) | Err(_) => break,
                    Ok(_) => {}
                }
            }
        })
        .await;
        let _ = tokio::time::timeout(self.times.write, self.writer.shutdown()).await;
    }
}

/// `{"event":"presence","online":[…]}` with each tag as the client writes
/// it: lower-case hexadecimal, at least four digits.
fn presence(online: &[u32]) -> String {
    use std::fmt::Write as _;
    let mut text = String::from(r#"{"event":"presence","online":["#);
    for (index, tag) in online.iter().enumerate() {
        if index > 0 {
            text.push(',');
        }
        let _ = write!(text, r#""{tag:04x}""#);
    }
    text.push_str("]}");
    text
}

/// The periodic check of every socket's session until the sockets stop.
async fn check(identity: Arc<IdentityService>, mut stop: watch::Receiver<bool>) {
    let mut every = tokio::time::interval_at(Instant::now() + CHECK_EVERY, CHECK_EVERY);
    every.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut failed = 0_u32;
    loop {
        tokio::select! {
            biased;
            _ = stop.changed() => return,
            _ = every.tick() => {}
        }
        let checked = tokio::time::timeout(CHECK_TIME, identity.check_listeners()).await;
        if matches!(checked, Ok(Ok(()))) {
            failed = 0;
        } else {
            failed += 1;
            if failed >= CHECK_FAILURES {
                identity.listeners_unchecked();
                failed = 0;
            }
        }
    }
}

#[cfg(test)]
mod tests;
