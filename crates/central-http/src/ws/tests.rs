use super::*;
use std::sync::atomic::{AtomicU8, Ordering};
use tokio::{
    io::{AsyncReadExt, DuplexStream},
    sync::Notify,
};

type Pairs = Vec<(&'static str, String)>;

fn headers(pairs: &Pairs) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for (name, value) in pairs {
        headers.append(*name, HeaderValue::from_str(value).unwrap());
    }
    headers
}

/// RFC 6455 § 1.3's example: the client's nonce is "the sample nonce", sent
/// in Base64.
fn sample() -> String {
    base64::engine::general_purpose::STANDARD.encode(b"the sample nonce")
}

/// A handshake as the Baylee client sends it, with the RFC's nonce.
fn good() -> Pairs {
    vec![
        ("upgrade", "websocket".into()),
        ("connection", "Upgrade".into()),
        ("sec-websocket-version", "13".into()),
        (header::SEC_WEBSOCKET_KEY.as_str(), sample()),
    ]
}

#[test]
fn the_accept_value_is_rfc_6455_s() {
    // RFC 6455 § 1.3's example and its answer.
    assert_eq!(sample(), "dGhlIHNhbXBsZSBub25jZQ==");
    assert_eq!(
        handshake(&headers(&good())).ok(),
        Some(HeaderValue::from_static("s3pPLMBiTxaQ9kYGzzhZRbK+xOo="))
    );
    // A client's Connection may name more than the upgrade, in any case.
    let mut more = good();
    more[1] = ("connection", "keep-alive, upgrade".into());
    assert!(handshake(&headers(&more)).is_ok());
    more[0] = ("upgrade", "WebSocket".into());
    assert!(handshake(&headers(&more)).is_ok());
}

#[test]
fn every_other_handshake_is_refused_before_anything_is_asked() {
    let nonce = header::SEC_WEBSOCKET_KEY.as_str();
    let without =
        |name: &str| -> Pairs { good().into_iter().filter(|(n, _)| *n != name).collect() };
    let with = |name: &'static str, value: String| -> Pairs {
        let mut pairs = good();
        pairs.push((name, value));
        pairs
    };
    let replaced = |name: &'static str, value: String| -> Pairs {
        good()
            .into_iter()
            .map(|(n, v)| {
                if n == name {
                    (n, value.clone())
                } else {
                    (n, v)
                }
            })
            .collect()
    };
    // the sample with its last character changed: still 24 characters, no
    // longer the canonical spelling of sixteen bytes
    let mut uncanonical = sample();
    uncanonical.replace_range(21..22, "R");
    for pairs in [
        without("upgrade"),
        without("connection"),
        without(nonce),
        replaced("upgrade", "h2c".into()),
        replaced("connection", "keep-alive".into()),
        // two Upgrade headers, two nonces
        with("upgrade", "websocket".into()),
        with(nonce, sample()),
        // a body, announced either way
        with("content-length", "0".into()),
        with("content-length", "5".into()),
        with("transfer-encoding", "chunked".into()),
        // nonces: not Base64, fifteen bytes, seventeen, not canonical
        replaced(nonce, "not base64 at all!!!!!==".into()),
        replaced(nonce, "A".repeat(20)),
        replaced(nonce, format!("{}=", "A".repeat(23))),
        replaced(nonce, uncanonical),
    ] {
        let refused = handshake(&headers(&pairs)).unwrap_err();
        assert_eq!(refused, Refusal::Invalid, "{pairs:?}");
        assert_eq!(refused.into_response().status(), StatusCode::BAD_REQUEST);
    }
    // Another version: 426, naming the one this server speaks.
    for version in [
        replaced("sec-websocket-version", "8".into()),
        without("sec-websocket-version"),
    ] {
        let refused = handshake(&headers(&version)).unwrap_err().into_response();
        assert_eq!(refused.status(), StatusCode::UPGRADE_REQUIRED);
        assert_eq!(
            refused
                .headers()
                .get(header::SEC_WEBSOCKET_VERSION)
                .unwrap(),
            "13"
        );
    }
}

#[test]
fn presence_names_tags_as_the_client_does() {
    assert_eq!(presence(&[]), r#"{"event":"presence","online":[]}"#);
    assert_eq!(
        presence(&[0x2a, 0x1000, 0x7fff_ffff]),
        r#"{"event":"presence","online":["002a","1000","7fffffff"]}"#
    );
    // the widest presence an account can have fits the client's frames
    let widest = vec![0x7fff_ffff; central_api::social::MAX_CONTACTS];
    assert!(presence(&widest).len() <= frame::TEXT);
    assert!(frame::text(&presence(&widest)).is_some());
}

#[test]
fn only_a_non_upgrade_answer_says_close() {
    let closed = closing(StatusCode::UNAUTHORIZED.into_response());
    assert_eq!(closed.headers().get(header::CONNECTION).unwrap(), "close");
    let mut upgrade = Response::new(Body::empty());
    *upgrade.status_mut() = StatusCode::SWITCHING_PROTOCOLS;
    upgrade
        .headers_mut()
        .insert(header::CONNECTION, HeaderValue::from_static("upgrade"));
    assert_eq!(
        closing(upgrade).headers().get(header::CONNECTION).unwrap(),
        "upgrade"
    );
}

#[tokio::test]
async fn only_the_exact_first_line_asks_for_the_socket() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let asked = |parts: Vec<&'static [u8]>, pause: Duration| {
        let listener = &listener;
        async move {
            let mut client = TcpStream::connect(address).await.unwrap();
            let (server, _) = listener.accept().await.unwrap();
            let sending = async {
                for part in parts {
                    client.write_all(part).await.unwrap();
                    tokio::time::sleep(pause).await;
                }
            };
            let (asked, ()) = tokio::join!(asks(&server), sending);
            // what was looked at is still there for hyper
            let mut first = [0; 4];
            if asked {
                server.peek(&mut first).await.unwrap();
                assert_eq!(&first, b"GET ");
            }
            asked
        }
    };
    let pause = Duration::from_millis(30);
    assert!(asked(vec![b"GET /v1/ws HTTP/1.1\r\nHost: x\r\n\r\n"], pause).await);
    // the line in pieces, within the time a first line has
    assert!(asked(vec![b"GET /v1", b"/ws HT", b"TP/1.1\r\n"], pause).await);
    for line in [
        &b"GET /v1/ws?x=1 HTTP/1.1\r\n"[..],
        b"GET /v1/wss HTTP/1.1\r\n",
        b"POST /v1/ws HTTP/1.1\r\n",
        b"GET /v1/ws HTTP/1.0\r\n",
        b"get /v1/ws HTTP/1.1\r\n",
        b"GET /v1/native HTTP/1.1\r\n",
    ] {
        assert!(!asked(vec![line], pause).await, "{line:?}");
    }
    // a line that never comes whole is an ordinary connection's
    let started = Instant::now();
    assert!(!asked(vec![b"GET /v1/w"], FIRST_LINE * 2).await);
    assert!(started.elapsed() < FIRST_LINE * 3);
}

/// An account as a test drives it.
#[derive(Default)]
struct Fake {
    bits: AtomicU8,
    bell: Notify,
    ready: AtomicU8,
    online: std::sync::Mutex<Vec<u32>>,
    /// What a check of the session answers.
    standing: std::sync::Mutex<Option<Result<bool, Error>>>,
}

impl Fake {
    fn raise(&self, bits: u8) {
        self.bits.fetch_or(bits, Ordering::AcqRel);
        self.bell.notify_one();
    }
}

impl Account for Arc<Fake> {
    fn take(&self) -> u8 {
        self.bits.swap(0, Ordering::AcqRel)
    }

    fn rung(&self) -> impl Future<Output = ()> + Send {
        self.bell.notified()
    }

    fn ready(&self) {
        self.ready.fetch_add(1, Ordering::AcqRel);
    }

    fn presence(&self) -> Vec<u32> {
        self.online.lock().unwrap().clone()
    }

    fn listening(&self) -> impl Future<Output = Result<bool, Error>> + Send {
        let standing = self.standing.lock().unwrap().unwrap_or(Ok(true));
        async move { standing }
    }
}

const SHORT: Times = Times {
    ping_every: Duration::from_millis(100),
    idle: Duration::from_millis(250),
    write: Duration::from_millis(300),
    linger: Duration::from_millis(100),
    lifetime: Duration::from_secs(30),
    check: Duration::from_millis(300),
    frames: 16,
    frame_window: Duration::from_secs(10),
};

/// A socket served over an in-memory connection: the fake account, the
/// client's end, the stop switch and the socket's task.
struct Harness {
    account: Arc<Fake>,
    client: DuplexStream,
    stop: watch::Sender<bool>,
    task: tokio::task::JoinHandle<End>,
}

fn served(times: Times, buffer: usize) -> Harness {
    let account = Arc::new(Fake::default());
    let (client, server) = tokio::io::duplex(buffer);
    let (reader, writer) = tokio::io::split(server);
    let (stop, stopped) = watch::channel(false);
    let socket = Socket::new(Arc::clone(&account), reader, writer, stopped, times);
    Harness {
        account,
        client,
        stop,
        task: tokio::spawn(socket.run()),
    }
}

/// What the server sent: a text, a ping, a close with its code, or the end.
#[derive(Debug, PartialEq, Eq)]
enum Heard {
    Text(String),
    Ping,
    Pong(Vec<u8>),
    Close(u16),
    End,
}

async fn hear(client: &mut DuplexStream) -> Heard {
    let mut head = [0; 2];
    if client.read_exact(&mut head).await.is_err() {
        return Heard::End;
    }
    assert_eq!(head[1] & 0x80, 0, "a server never masks");
    let length = match head[1] & 0x7f {
        126 => {
            let mut wide = [0; 2];
            client.read_exact(&mut wide).await.unwrap();
            usize::from(u16::from_be_bytes(wide))
        }
        127 => panic!("no frame of ours is that long"),
        short => usize::from(short),
    };
    let mut payload = vec![0; length];
    client.read_exact(&mut payload).await.unwrap();
    match head[0] {
        0x81 => Heard::Text(String::from_utf8(payload).unwrap()),
        0x89 => Heard::Ping,
        0x8a => Heard::Pong(payload),
        0x88 => Heard::Close(u16::from_be_bytes([payload[0], payload[1]])),
        other => panic!("unexpected frame {other:#x}"),
    }
}

/// A client's masked frame.
fn sent(first: u8, payload: &[u8]) -> Vec<u8> {
    let mask = [0x11, 0x22, 0x33, 0x44];
    let mut frame = vec![first, 0x80 | u8::try_from(payload.len()).unwrap()];
    frame.extend_from_slice(&mask);
    frame.extend(payload.iter().enumerate().map(|(i, b)| b ^ mask[i % 4]));
    frame
}

fn text(event: &str) -> Heard {
    Heard::Text(event.to_owned())
}

async fn opened(times: Times) -> Harness {
    let mut socket = served(times, 4096);
    assert_eq!(hear(&mut socket.client).await, text(READY));
    assert_eq!(
        hear(&mut socket.client).await,
        text(r#"{"event":"presence","online":[]}"#)
    );
    assert_eq!(socket.account.ready.load(Ordering::Acquire), 1);
    socket
}

#[tokio::test]
async fn ready_then_presence_then_only_news_that_was_raised() {
    let mut socket = opened(SHORT).await;
    socket.account.raise(news::CHAT);
    assert_eq!(hear(&mut socket.client).await, text(CHAT));
    socket.account.raise(news::SOCIAL | news::CHAT);
    assert_eq!(hear(&mut socket.client).await, text(SOCIAL));
    assert_eq!(hear(&mut socket.client).await, text(CHAT));
    // presence only when it changed
    socket.account.raise(news::PRESENCE);
    *socket.account.online.lock().unwrap() = vec![0x2a];
    socket.account.raise(news::PRESENCE);
    assert_eq!(
        hear(&mut socket.client).await,
        text(r#"{"event":"presence","online":["002a"]}"#)
    );
    socket.account.raise(news::PRESENCE);
    socket.account.raise(news::CHAT);
    assert_eq!(
        hear(&mut socket.client).await,
        text(CHAT),
        "no repeated presence"
    );
    socket.stop.send(true).unwrap();
    assert_eq!(hear(&mut socket.client).await, Heard::Close(1001));
    assert_eq!(socket.task.await.unwrap(), End::Close(Code::GOING_AWAY));
}

#[tokio::test]
async fn a_burst_of_news_is_one_frame_of_each_kind() {
    let mut socket = opened(SHORT).await;
    for _ in 0..10_000 {
        socket.account.raise(news::CHAT);
        socket.account.raise(news::SOCIAL);
    }
    assert_eq!(hear(&mut socket.client).await, text(SOCIAL));
    assert_eq!(hear(&mut socket.client).await, text(CHAT));
    // then nothing until the next ping
    assert_eq!(hear(&mut socket.client).await, Heard::Ping);
    socket.stop.send(true).unwrap();
    let _ = socket.task.await;
}

#[tokio::test]
async fn news_while_a_frame_is_half_read_loses_none_of_its_bytes() {
    let mut socket = opened(SHORT).await;
    let ping = sent(0x89, b"half-read");
    socket.client.write_all(&ping[..5]).await.unwrap();
    tokio::time::sleep(Duration::from_millis(20)).await;
    // The read is in flight; a push wins the select and is written.
    socket.account.raise(news::CHAT);
    assert_eq!(hear(&mut socket.client).await, text(CHAT));
    socket.client.write_all(&ping[5..]).await.unwrap();
    assert_eq!(
        hear(&mut socket.client).await,
        Heard::Pong(b"half-read".to_vec())
    );
    socket.stop.send(true).unwrap();
    let _ = socket.task.await;
}

#[tokio::test]
async fn an_ended_session_says_revoked_and_closes() {
    let mut socket = opened(SHORT).await;
    socket.account.raise(news::REVOKED | news::CHAT);
    assert_eq!(hear(&mut socket.client).await, text(REVOKED));
    assert_eq!(hear(&mut socket.client).await, Heard::Close(1008));
    socket
        .client
        .write_all(&sent(0x88, &1000_u16.to_be_bytes()))
        .await
        .unwrap();
    assert_eq!(hear(&mut socket.client).await, Heard::End);
    assert_eq!(socket.task.await.unwrap(), End::Revoked);
}

#[tokio::test]
async fn a_check_revokes_only_on_the_store_s_word() {
    // the store cannot answer: no revocation, the socket goes on
    let mut socket = opened(SHORT).await;
    *socket.account.standing.lock().unwrap() = Some(Err(Error::Unavailable));
    socket.account.raise(news::CHECK | news::CHAT);
    assert_eq!(hear(&mut socket.client).await, text(CHAT));
    // the session stands: nothing
    *socket.account.standing.lock().unwrap() = Some(Ok(true));
    socket.account.raise(news::CHECK | news::SOCIAL);
    assert_eq!(hear(&mut socket.client).await, text(SOCIAL));
    // it does not
    *socket.account.standing.lock().unwrap() = Some(Ok(false));
    socket.account.raise(news::CHECK | news::CHAT);
    assert_eq!(hear(&mut socket.client).await, text(REVOKED));
    assert_eq!(hear(&mut socket.client).await, Heard::Close(1008));
    assert_eq!(socket.task.await.unwrap(), End::Revoked);
}

#[tokio::test]
async fn what_cannot_be_kept_true_closes_without_revoked() {
    let mut socket = opened(SHORT).await;
    socket.account.raise(news::STALE | news::CHAT);
    assert_eq!(hear(&mut socket.client).await, Heard::Close(1011));
    assert_eq!(socket.task.await.unwrap(), End::Close(Code::INTERNAL));
}

#[tokio::test]
async fn a_silent_client_is_dropped_and_an_answering_one_is_kept() {
    // answers every ping: still there after three idle limits
    let mut socket = opened(SHORT).await;
    let started = Instant::now();
    while started.elapsed() < SHORT.idle * 3 {
        assert_eq!(hear(&mut socket.client).await, Heard::Ping);
        socket.client.write_all(&sent(0x8a, b"")).await.unwrap();
    }
    assert!(!socket.task.is_finished());
    socket.stop.send(true).unwrap();
    let _ = socket.task.await;
    // answers nothing: pinged, then dropped without a word once idle
    let mut silent = opened(SHORT).await;
    let mut pings = 0;
    loop {
        match hear(&mut silent.client).await {
            Heard::Ping => pings += 1,
            Heard::End => break,
            other => panic!("unexpected {other:?}"),
        }
    }
    assert!((1..=3).contains(&pings), "{pings}");
    assert_eq!(silent.task.await.unwrap(), End::Drop);
    // dropped at the idle limit itself, not at the next ping after it
    let times = Times {
        ping_every: Duration::from_millis(200),
        idle: Duration::from_millis(250),
        ..SHORT
    };
    let mut silent = opened(times).await;
    let started = Instant::now();
    while hear(&mut silent.client).await != Heard::End {}
    let held = started.elapsed();
    assert!(
        held >= times.idle && held < times.ping_every * 2,
        "{held:?}"
    );
}

#[tokio::test]
async fn frames_the_client_may_not_send_close_the_socket() {
    for (frame, code) in [
        (sent(0x81, br#"{"event":"chat"}"#), 1003),
        (vec![0x89, 0x00], 1002),
        (sent(0x8b, b""), 1002),
    ] {
        let mut socket = opened(SHORT).await;
        socket.client.write_all(&frame).await.unwrap();
        assert_eq!(hear(&mut socket.client).await, Heard::Close(code));
        assert_eq!(socket.task.await.unwrap(), End::Close(Code(code)));
    }
    // more frames in a window than allowed
    let mut socket = opened(SHORT).await;
    for _ in 0..=SHORT.frames {
        socket.client.write_all(&sent(0x8a, b"")).await.unwrap();
    }
    assert_eq!(hear(&mut socket.client).await, Heard::Close(1008));
    assert_eq!(socket.task.await.unwrap(), End::Close(Code::POLICY));
}

#[tokio::test]
async fn a_client_s_close_is_answered_and_ends_the_socket() {
    let mut socket = opened(SHORT).await;
    socket
        .client
        .write_all(&sent(0x88, &1000_u16.to_be_bytes()))
        .await
        .unwrap();
    assert_eq!(hear(&mut socket.client).await, Heard::Close(1000));
    assert_eq!(hear(&mut socket.client).await, Heard::End);
    assert_eq!(socket.task.await.unwrap(), End::Answer);
}

#[tokio::test]
async fn a_client_that_does_not_read_loses_its_socket() {
    // A buffer smaller than the first frame: the write never completes.
    let socket = served(SHORT, 8);
    let ended = tokio::time::timeout(SHORT.write * 4, socket.task)
        .await
        .expect("the write's time ends it")
        .unwrap();
    assert_eq!(ended, End::Drop);
    assert_eq!(
        socket.account.ready.load(Ordering::Acquire),
        0,
        "never online"
    );
    drop(socket.client);
}

#[tokio::test]
async fn a_socket_outliving_its_time_closes() {
    let times = Times {
        lifetime: Duration::from_millis(150),
        ..SHORT
    };
    let mut socket = opened(times).await;
    loop {
        match hear(&mut socket.client).await {
            Heard::Ping => socket.client.write_all(&sent(0x8a, b"")).await.unwrap(),
            Heard::Close(code) => {
                assert_eq!(code, 1001);
                break;
            }
            other => panic!("unexpected {other:?}"),
        }
    }
    assert_eq!(socket.task.await.unwrap(), End::Close(Code::GOING_AWAY));
}

#[tokio::test]
async fn a_socket_started_after_the_stop_closes_at_once() {
    let mut socket = served(SHORT, 4096);
    socket.stop.send(true).unwrap();
    assert_eq!(hear(&mut socket.client).await, text(READY));
    let _presence = hear(&mut socket.client).await;
    assert_eq!(hear(&mut socket.client).await, Heard::Close(1001));
    assert_eq!(socket.task.await.unwrap(), End::Close(Code::GOING_AWAY));
}

#[test]
fn an_address_holds_its_share_and_all_together_the_listener_s() {
    let sockets = Sockets::new();
    let client = |last: u8| Some(Client::from(std::net::IpAddr::from([198, 51, 100, last])));
    let mut held: Vec<Seat> = (0..ADDRESS_SOCKETS)
        .map(|_| sockets.upgrades.place(client(1)).unwrap())
        .collect();
    assert!(sockets.upgrades.place(client(1)).is_none(), "its share");
    assert!(sockets.upgrades.place(client(2)).is_some(), "another's");
    held.pop();
    assert!(sockets.upgrades.place(client(1)).is_some());
    drop(held);
    // the listener's limit, whatever the addresses (and for none known)
    let all: Vec<Seat> = (0..SOCKETS)
        .map(|index| {
            let last = u8::try_from(index / usize::from(ADDRESS_SOCKETS)).unwrap();
            sockets.upgrades.place(client(last)).unwrap()
        })
        .collect();
    assert!(sockets.upgrades.place(client(250)).is_none());
    assert!(sockets.upgrades.place(None).is_none());
    drop(all);
    let held = sockets.upgrades.0.held.lock().unwrap();
    assert_eq!((held.total, held.by_client.len()), (0, 0));
}

#[test]
fn an_address_dials_sixty_times_a_minute() {
    let sockets = Sockets::new();
    let client = Client::from(std::net::IpAddr::from([198, 51, 100, 9]));
    for _ in 0..ADDRESS_DIALS {
        assert!(sockets.upgrades.dial(client));
    }
    assert!(!sockets.upgrades.dial(client));
    let other = Client::from(std::net::IpAddr::from([198, 51, 100, 10]));
    assert!(sockets.upgrades.dial(other));
}
