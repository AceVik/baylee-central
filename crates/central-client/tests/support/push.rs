//! The native push socket, `GET /v1/ws`, over real TCP against the fixture's
//! account listener and PostgreSQL (`docs/push-socket.md`). No approved
//! WebSocket client exists, so this is a small one of its own: it masks every
//! frame, as a client must, answers pings, and reads only what RFC 6455
//! allows a server to send.
use super::{
    Http,
    browser::Browser,
    password::{self, NEW, OLD, Registered},
};
use central_api::{
    Bearer, Error, SignedToken, Token,
    owner::OwnerRequest,
    picture::{MAX_PICTURE_TAGS, PictureReason, TaggedPicture},
    presence::{Activity, Choice, Table, table_frame},
    social::{ContactAction, SocialRequest, SocialResponse},
    terms::TermsSubject,
    wire::{Identity, Request, Response},
};
use central_client::Transport;
use central_crypto::opaque::Login;
use sea_orm::{ConnectionTrait, DatabaseConnection};
use std::{
    collections::BTreeSet,
    io::{ErrorKind, Read, Write},
    net::{SocketAddr, TcpStream},
    time::{Duration, Instant},
};
use tokio::runtime::Runtime;

/// RFC 6455 § 1.3's example key, and the accept value it must come back with.
const KEY: &str = "dGhlIHNhbXBsZSBub25jZQ==";
const ACCEPT: &str = "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=";
/// How long a socket is watched to show it hears nothing: longer than the
/// second a socket waits between two presence frames, so a frame it holds
/// back is heard here rather than merged into the next one a test expects.
const QUIET: Duration = Duration::from_millis(1_250);

const READY: &str = r#"{"event":"ready","tables":true}"#;
const SOCIAL: &str = r#"{"event":"social"}"#;
const REVOKED: &str = r#"{"event":"revoked"}"#;

/// A presence frame of friends shown online with nothing to say of them,
/// for an account that chose to be online.
fn presence(tags: &[u32]) -> String {
    let friends: Vec<(u32, char)> = tags.iter().map(|tag| (*tag, 'o')).collect();
    presence_of(&friends, "online")
}

/// A presence frame: each friend's tag and state letter, and the own choice.
fn presence_of(friends: &[(u32, char)], me: &str) -> String {
    let named: Vec<String> = friends
        .iter()
        .map(|(tag, _)| format!(r#""{tag:04x}""#))
        .collect();
    let states: String = friends.iter().map(|(_, state)| *state).collect();
    format!(
        r#"{{"event":"presence","online":[{}],"states":"{states}","me":"{me}"}}"#,
        named.join(",")
    )
}

/// What the server sent.
#[derive(Debug, PartialEq, Eq)]
enum Heard {
    Text(String),
    Close(u16),
    /// The connection ended.
    End,
    /// Nothing within the read's time.
    Nothing,
}

/// An open push socket.
struct Socket {
    stream: TcpStream,
}

/// The answer to a dial that was not upgraded: its status and head.
#[derive(Debug)]
struct Refused {
    status: u16,
    head: String,
}

/// Dial with `lines` (each ending in CRLF) after the request line and `Host`.
fn dial_with(address: SocketAddr, line: &str, lines: &str, body: &[u8]) -> Result<Socket, Refused> {
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(3)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    write!(stream, "{line}\r\nHost: {address}\r\n{lines}\r\n").unwrap();
    stream.write_all(body).unwrap();
    // The head byte by byte, so that no frame behind it is read with it.
    let mut head = Vec::new();
    let mut byte = [0_u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        match stream.read(&mut byte) {
            Ok(1) => head.push(byte[0]),
            _ => break,
        }
        assert!(head.len() < 8192, "a head this long");
    }
    let head = String::from_utf8(head).unwrap();
    let status: u16 = head
        .split_whitespace()
        .nth(1)
        .and_then(|status| status.parse().ok())
        .unwrap_or(0);
    if status != 101 {
        return Err(Refused { status, head });
    }
    let fields: Vec<(String, String)> = head
        .lines()
        .skip(1)
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_owned()))
        .collect();
    let field = |name: &str| -> Vec<&str> {
        fields
            .iter()
            .filter(|(n, _)| n == name)
            .map(|(_, value)| value.as_str())
            .collect()
    };
    assert_eq!(field("upgrade"), ["websocket"], "{head}");
    // the upgrade, and no `close` that a proxy or a client could act on
    assert_eq!(field("connection"), ["upgrade"], "{head}");
    assert_eq!(field("sec-websocket-accept"), [ACCEPT], "{head}");
    Ok(Socket { stream })
}

/// The handshake the Baylee client sends, and `extra` lines.
fn handshake(bearer: Option<&str>, extra: &str) -> String {
    let mut lines = format!(
        "Upgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: {KEY}\r\n{extra}"
    );
    if let Some(bearer) = bearer {
        lines.push_str("Authorization: ");
        lines.push_str(bearer);
        lines.push_str("\r\n");
    }
    lines
}

fn dial(http: &Http, bearer: &str) -> Result<Socket, Refused> {
    dial_with(
        http.address,
        "GET /v1/ws HTTP/1.1",
        &handshake(Some(bearer), ""),
        b"",
    )
}

/// A client's frame, masked.
fn masked(first: u8, payload: &[u8]) -> Vec<u8> {
    let mask = [0x5a, 0xa5, 0x3c, 0xc3];
    let mut frame = vec![first, 0x80 | u8::try_from(payload.len()).unwrap()];
    frame.extend_from_slice(&mask);
    frame.extend(payload.iter().enumerate().map(|(i, b)| b ^ mask[i % 4]));
    frame
}

impl Socket {
    /// The next frame, within `wait`; pings are answered and not shown.
    fn hear(&mut self, wait: Duration) -> Heard {
        let deadline = Instant::now() + wait;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Heard::Nothing;
            }
            self.stream.set_read_timeout(Some(left)).unwrap();
            let mut head = [0_u8; 2];
            match self.stream.read_exact(&mut head) {
                Ok(()) => {}
                Err(error)
                    if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) =>
                {
                    return Heard::Nothing;
                }
                Err(_) => return Heard::End,
            }
            // a frame begun is read whole
            self.stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            assert_eq!(head[1] & 0x80, 0, "a server never masks");
            assert_eq!(head[0] & 0x70, 0, "no extension was agreed");
            let length = match head[1] & 0x7f {
                126 => {
                    let mut wide = [0; 2];
                    self.stream.read_exact(&mut wide).unwrap();
                    usize::from(u16::from_be_bytes(wide))
                }
                127 => panic!("no event is that long"),
                short => usize::from(short),
            };
            assert!(length <= 4096, "the client reads at most 4096 bytes");
            let mut payload = vec![0; length];
            self.stream.read_exact(&mut payload).unwrap();
            match head[0] {
                0x81 => return Heard::Text(String::from_utf8(payload).unwrap()),
                0x89 => self.send(&masked(0x8a, &payload)),
                0x88 => {
                    return Heard::Close(u16::from_be_bytes([payload[0], payload[1]]));
                }
                other => panic!("a frame the server does not send: {other:#x}"),
            }
        }
    }

    fn send(&mut self, frame: &[u8]) {
        let _ = self.stream.write_all(frame);
    }

    /// The next text, which must come.
    fn text(&mut self) -> String {
        match self.hear(Duration::from_secs(5)) {
            Heard::Text(text) => text,
            other => panic!("a text, not {other:?}"),
        }
    }

    /// The next `count` texts, in any order.
    fn texts(&mut self, count: usize) -> BTreeSet<String> {
        (0..count).map(|_| self.text()).collect()
    }

    /// Nothing comes for a while.
    fn quiet(&mut self) {
        assert_eq!(self.hear(QUIET), Heard::Nothing);
    }

    /// The socket still stands: a ping of the client's own is answered.
    fn answers_a_ping(&mut self) {
        self.send(&masked(0x89, b"still there"));
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut head = [0_u8; 2];
        self.stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        loop {
            assert!(Instant::now() < deadline, "no pong");
            self.stream.read_exact(&mut head).unwrap();
            let mut payload = vec![0; usize::from(head[1] & 0x7f)];
            self.stream.read_exact(&mut payload).unwrap();
            match head[0] {
                0x8a => {
                    assert_eq!(payload, b"still there");
                    return;
                }
                // the server's own ping, answered
                0x89 => self.send(&masked(0x8a, &payload)),
                other => panic!("a frame other than the pong: {other:#x}"),
            }
        }
    }

    /// Opened: `ready`, then the presence it reports.
    fn opened(self, online: &[u32]) -> Self {
        self.opened_as(&presence(online))
    }

    /// Opened: `ready`, then exactly `presence`.
    fn opened_as(mut self, presence: &str) -> Self {
        assert_eq!(self.text(), READY);
        assert_eq!(self.text(), presence);
        self
    }

    /// `revoked`, then a close for its policy, then the end.
    fn revoked(mut self) {
        assert_eq!(self.text(), REVOKED);
        assert_eq!(self.hear(Duration::from_secs(5)), Heard::Close(1008));
        self.send(&masked(0x88, &1000_u16.to_be_bytes()));
        assert_eq!(self.hear(Duration::from_secs(5)), Heard::End);
    }

    /// The client closes, and the server answers.
    fn close(mut self) {
        self.send(&masked(0x88, &1000_u16.to_be_bytes()));
        assert_eq!(self.hear(Duration::from_secs(5)), Heard::Close(1000));
        assert_eq!(self.hear(Duration::from_secs(5)), Heard::End);
    }
}

/// A signed-in native account with its terms accepted: its identity, tag
/// and bearer header.
struct Player {
    identity: Identity,
    tag: u32,
    token: SignedToken,
}

impl Player {
    fn bearer(&self) -> String {
        Bearer::from(&self.token).header().as_str().to_owned()
    }

    fn ask(&self, http: &Http, request: &Request) -> Result<Response, Error> {
        http.exchange(request, Some((&self.token).into()))
    }

    fn contact(&self, http: &Http, other: &Self, change: ContactAction) {
        assert!(matches!(
            self.ask(
                http,
                &Request::Social {
                    request: SocialRequest::Contact {
                        tag: other.tag,
                        change,
                    },
                },
            ),
            Ok(Response::Social { .. })
        ));
    }

    /// Signed in anew: the native session replaces the one before.
    fn again(&mut self, http: &Http) {
        self.token = password::sign_in(http, self.identity, OLD).0;
    }
}

fn accept(http: &Http, token: &SignedToken) {
    let Response::TermsStatus {
        current: Some(current),
        ..
    } = http
        .exchange(&Request::TermsStatus, Some(token.into()))
        .unwrap()
    else {
        panic!("terms in force");
    };
    http.exchange(
        &Request::AcceptTerms {
            subject: TermsSubject::Central,
            version: current.version,
            digest: current.digest,
        },
        Some(token.into()),
    )
    .unwrap();
}

fn tag(runtime: &Runtime, operator: &DatabaseConnection, identity: Identity) -> u32 {
    let row = runtime
        .block_on(
            operator.query_one_raw(sea_orm::Statement::from_sql_and_values(
                sea_orm::DbBackend::Postgres,
                "SELECT tag FROM central_identity.accounts WHERE id = $1",
                vec![sea_orm::prelude::Uuid::from_bytes(identity.account).into()],
            )),
        )
        .unwrap()
        .unwrap();
    u32::try_from(row.try_get::<i32>("", "tag").unwrap()).unwrap()
}

pub(crate) fn run(
    http: &Http,
    browser_address: SocketAddr,
    runtime: &Runtime,
    operator: &DatabaseConnection,
) {
    // Their own address, for their registrations' budget.
    let home = Browser {
        address: browser_address,
        client: "198.51.100.160",
    };
    let mut players: Vec<Player> = ["push-alice", "push-bob", "push-carol"]
        .into_iter()
        .map(|name| {
            let Registered { identity, .. } = password::register(name, http, &home);
            let (token, _) = password::sign_in(http, identity, OLD);
            Player {
                identity,
                tag: tag(runtime, operator, identity),
                token,
            }
        })
        .collect();
    // C has not accepted the terms yet: no socket, and no choice; its
    // choice can be read, and a choice's budget is spent before the terms are
    // looked at, so it cannot be turned into statements.
    let refused = dial(http, &players[2].bearer()).err().unwrap();
    assert_eq!(refused.status, 403, "{}", refused.head);
    assert!(matches!(
        players[2].ask(http, &Request::Presence),
        Ok(Response::Presence {
            choice: Choice::Online
        })
    ));
    for _ in 1..central_identity::SESSION_CHOICES {
        let choice = Choice::Offline;
        assert_eq!(
            players[2].ask(http, &Request::SetPresence { choice }).err(),
            Some(Error::TermsRequired)
        );
    }
    let choice = Choice::Offline;
    assert_eq!(
        players[2].ask(http, &Request::SetPresence { choice }).err(),
        Some(Error::Busy)
    );
    // a new session, with a budget of its own
    players[2].again(http);
    for player in &players {
        accept(http, &player.token);
    }
    refusals(http, &players[0]);
    limits(http, &players[0]);
    pictures(runtime, operator, http, [&players[0], &players[1]]);
    let [a, b, c] = &mut players[..] else {
        unreachable!()
    };
    let cookie = addressing(http, &home, [a, b, c]);
    status(http, &home, &cookie, [a, b, c]);
    tables(http, [a, b, c]);
    revocation(http, &home, runtime, operator, [a, b, c], &cookie);
}

/// Every dial that is not a signed-in client's handshake is refused.
fn refusals(http: &Http, a: &Player) {
    let bearer = a.bearer();
    let refused = |line: &str, lines: &str, body: &[u8]| -> Refused {
        dial_with(http.address, line, lines, body).err().unwrap()
    };
    let line = "GET /v1/ws HTTP/1.1";
    // no bearer, two bearers
    assert_eq!(refused(line, &handshake(None, ""), b"").status, 401);
    let twice = format!("Authorization: {bearer}\r\n");
    assert_eq!(
        refused(line, &handshake(Some(&bearer), &twice), b"").status,
        401
    );
    // a gateway's credential, a made-up signed token
    let credential = format!("Bearer {}", Token::generate().unwrap().hex().as_str());
    assert_eq!(
        refused(line, &handshake(Some(&credential), ""), b"").status,
        401
    );
    assert_eq!(
        refused(line, &handshake(Some("Bearer a.b.c"), ""), b"").status,
        401
    );
    // a browser's Origin, before anything else
    let page = refused(
        line,
        &handshake(Some(&bearer), "Origin: https://baylee.app\r\n"),
        b"",
    );
    assert_eq!(page.status, 403);
    // a body
    assert_eq!(
        refused(
            line,
            &handshake(Some(&bearer), "Content-Length: 5\r\n"),
            b"hello"
        )
        .status,
        400
    );
    // another version: 426, naming 13
    let version = refused(
        line,
        &handshake(Some(&bearer), "")
            .replace("Sec-WebSocket-Version: 13", "Sec-WebSocket-Version: 8"),
        b"",
    );
    assert_eq!(version.status, 426);
    assert!(
        version
            .head
            .to_ascii_lowercase()
            .contains("sec-websocket-version: 13")
    );
    // a key that is not sixteen bytes
    assert_eq!(
        refused(
            line,
            &handshake(Some(&bearer), "").replace(KEY, "c2hvcnQ="),
            b""
        )
        .status,
        400
    );
    // not an upgrade at all
    assert_eq!(
        refused(line, &format!("Authorization: {bearer}\r\n"), b"").status,
        400
    );
    // the route on a connection whose first line asked for more than it
    assert_eq!(
        refused(
            "GET /v1/ws?client=1 HTTP/1.1",
            &handshake(Some(&bearer), ""),
            b""
        )
        .status,
        400
    );
    // a refusal ends its connection: no second request is answered
    let mut stream = TcpStream::connect_timeout(&http.address, Duration::from_secs(3)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    write!(
        stream,
        "GET /v1/ws HTTP/1.1\r\nHost: {0}\r\n\r\nGET /health/live HTTP/1.1\r\nHost: {0}\r\n\r\n",
        http.address
    )
    .unwrap();
    let mut answer = String::new();
    let _ = stream.read_to_string(&mut answer);
    assert_eq!(answer.matches("HTTP/1.1 ").count(), 1, "{answer}");
    assert!(answer.to_ascii_lowercase().contains("connection: close"));
}

/// Four sockets an account; sixty dials a minute an address.
/// Picture keys over the native listener with a native bearer, as the game
/// client asks them (M6, `docs/profile-pictures.md`): answered by the
/// website's rules, and no report filed here. B's picture is written as the
/// service would store one that moves; A and B are strangers yet, so A's
/// asking for B's tag spends one lookup.
fn pictures(runtime: &Runtime, operator: &DatabaseConnection, http: &Http, [a, b]: [&Player; 2]) {
    let account = sea_orm::prelude::Uuid::from_bytes(b.identity.account);
    let key = [0x7c_u8; 16];
    runtime
        .block_on(operator.execute_unprepared(&format!(
            "INSERT INTO central_identity.pictures (account_id, key, avif, png, sequence, sha256, side) \
             VALUES ('{account}', decode(repeat('7c', 16), 'hex'), decode(repeat('01', 64), 'hex'), \
             decode(repeat('02', 100), 'hex'), decode(repeat('03', 64), 'hex'), \
             decode(repeat('04', 32), 'hex'), 256)"
        )))
        .unwrap();
    let keys = |tags: Vec<u32>| match a.ask(
        http,
        &Request::Social {
            request: SocialRequest::Pictures { tags },
        },
    ) {
        Ok(Response::Social {
            response: SocialResponse::Pictures { pictures },
        }) => Ok(pictures),
        Ok(other) => panic!("pictures expected: {other:?}"),
        Err(error) => Err(error),
    };
    // one's own tag, free and without a picture; a stranger's that moves
    assert_eq!(keys(vec![a.tag]), Ok(vec![]));
    assert_eq!(
        keys(vec![b.tag, a.tag]),
        Ok(vec![TaggedPicture {
            tag: b.tag,
            key,
            moving: true,
        }])
    );
    // the bound on tags; and a report is the website's
    assert_eq!(keys(vec![1; MAX_PICTURE_TAGS + 1]), Err(Error::Rejected));
    let report = Request::Social {
        request: SocialRequest::ReportPicture {
            tag: b.tag,
            key,
            reason: PictureReason::Hate,
            note: None,
        },
    };
    assert_eq!(a.ask(http, &report).err(), Some(Error::Rejected));
    runtime
        .block_on(operator.execute_unprepared(&format!(
            "DELETE FROM central_identity.pictures WHERE account_id = '{account}'"
        )))
        .unwrap();
}

fn limits(http: &Http, a: &Player) {
    let held: Vec<Socket> = (0..4)
        .map(|_| dial(http, &a.bearer()).unwrap().opened(&[]))
        .collect();
    let fifth = dial(http, &a.bearer()).err().unwrap();
    assert_eq!(fifth.status, 429, "{}", fifth.head);
    for socket in held {
        socket.close();
    }
    // An address's dials are counted before anything is asked of the
    // service: refused ones too.
    let from = "X-Forwarded-For: 198.51.100.161\r\n";
    for _ in 0..60 {
        let refused = dial_with(
            http.address,
            "GET /v1/ws HTTP/1.1",
            &handshake(None, from),
            b"",
        )
        .err()
        .unwrap();
        assert_eq!(refused.status, 401);
    }
    let spent = dial_with(
        http.address,
        "GET /v1/ws HTTP/1.1",
        &handshake(Some(&a.bearer()), from),
        b"",
    )
    .err()
    .unwrap();
    assert_eq!(spent.status, 429);
}

/// News reaches exactly the accounts it concerns, from either listener;
/// presence follows friends' sockets. The website's cookie of B's, for what
/// follows.
fn addressing(http: &Http, home: &Browser, [a, b, c]: [&mut Player; 3]) -> Token {
    let mut alice = dial(http, &a.bearer()).unwrap().opened(&[]);
    let mut carol = dial(http, &c.bearer()).unwrap().opened(&[]);
    // A asks B: A's own news; C hears nothing
    a.contact(http, b, ContactAction::Request);
    assert_eq!(alice.text(), SOCIAL);
    carol.quiet();
    // B, not yet A's friend, sees nobody online
    let mut bob = dial(http, &b.bearer()).unwrap().opened(&[]);
    // B accepts: both are told, and see each other online; C nothing
    b.contact(http, a, ContactAction::Accept);
    assert_eq!(
        alice.texts(2),
        BTreeSet::from([SOCIAL.to_owned(), presence(&[b.tag])])
    );
    assert_eq!(
        bob.texts(2),
        BTreeSet::from([SOCIAL.to_owned(), presence(&[a.tag])])
    );
    carol.quiet();
    // B's second socket changes nothing for A, nor does its closing
    let second = dial(http, &b.bearer()).unwrap().opened(&[a.tag]);
    alice.quiet();
    second.close();
    alice.quiet();
    // B's last socket closes: A sees B go
    bob.close();
    assert_eq!(alice.text(), presence(&[]));
    // and come back
    let mut bob = dial(http, &b.bearer()).unwrap().opened(&[a.tag]);
    assert_eq!(alice.text(), presence(&[b.tag]));
    // A change made on the website reaches the native sockets it concerns.
    let cookie = website_sign_in(http, home, b.identity);
    let asked = home
        .answer(
            &Request::Social {
                request: SocialRequest::Contact {
                    tag: c.tag,
                    change: ContactAction::Request,
                },
            },
            Some(&cookie),
        )
        .unwrap();
    assert!(matches!(asked, Response::Social { .. }));
    assert_eq!(bob.text(), SOCIAL);
    assert_eq!(carol.text(), SOCIAL);
    alice.quiet();
    // B ends the friendship: both are told and see each other gone; C nothing
    b.contact(http, a, ContactAction::Remove);
    assert_eq!(
        alice.texts(2),
        BTreeSet::from([SOCIAL.to_owned(), presence(&[])])
    );
    assert_eq!(
        bob.texts(2),
        BTreeSet::from([SOCIAL.to_owned(), presence(&[])])
    );
    carol.quiet();
    // A blocks and unblocks C, who is no contact of A's, and drops its
    // connection as soon as the request is out, again and again: C's socket
    // hears nothing and stays open (nothing of C's is read for a block, so
    // there is no reading to cut short and no time to measure)
    for change in [ContactAction::Block, ContactAction::Unblock].repeat(3) {
        fire_and_forget(
            http,
            a,
            &Request::Social {
                request: SocialRequest::Contact { tag: c.tag, change },
            },
        );
    }
    a.contact(http, c, ContactAction::Unblock);
    carol.quiet();
    carol.answers_a_ping();
    for socket in [alice, bob, carol] {
        socket.close();
    }
    cookie
}

/// The chosen status and what a client says it is doing: an account that
/// chose offline is never seen, not as it connects, on a second device or
/// while it reports, and its friends see only how it is shown changing.
fn status(http: &Http, home: &Browser, cookie: &Token, [a, b, c]: [&mut Player; 3]) {
    let choose = |player: &Player, choice: Choice| {
        assert!(matches!(
            player.ask(http, &Request::SetPresence { choice }),
            Ok(Response::Presence { choice: stored }) if stored == choice
        ));
    };
    let report = |player: &Player, activity: Option<Activity>, idle: bool| {
        assert!(matches!(
            player.ask(
                http,
                &Request::ReportPresence {
                    activity,
                    idle,
                    table: None,
                },
            ),
            Ok(Response::Done)
        ));
    };
    // A and C become friends; A is online
    a.contact(http, c, ContactAction::Request);
    c.contact(http, a, ContactAction::Accept);
    let mut alice = dial(http, &a.bearer()).unwrap().opened(&[]);
    // C chooses offline before it connects, then connects twice and reports
    choose(c, Choice::Offline);
    let carol = dial(http, &c.bearer()).unwrap();
    let mut carol = carol.opened_as(&presence_of(&[(a.tag, 'o')], "offline"));
    let second = dial(http, &c.bearer()).unwrap();
    let second = second.opened_as(&presence_of(&[(a.tag, 'o')], "offline"));
    // playing and idle while offline: still nothing
    report(c, Some(Activity::Playing), true);
    second.close();
    alice.quiet();
    // a report from a session without a socket: answered alike, nothing kept
    report(b, Some(Activity::Table), true);
    // what C chooses reaches A only as it changes how C is shown
    choose(c, Choice::Online);
    assert_eq!(alice.text(), presence_of(&[(c.tag, 'G')], "online"));
    assert_eq!(carol.text(), presence_of(&[(a.tag, 'o')], "online"));
    report(c, Some(Activity::Table), false);
    assert_eq!(alice.text(), presence_of(&[(c.tag, 't')], "online"));
    choose(c, Choice::Afk);
    assert_eq!(alice.text(), presence_of(&[(c.tag, 'T')], "online"));
    assert_eq!(carol.text(), presence_of(&[(a.tag, 'o')], "afk"));
    // the choice read back, natively and on the website (B's)
    assert!(matches!(
        c.ask(http, &Request::Presence),
        Ok(Response::Presence {
            choice: Choice::Afk
        })
    ));
    assert!(matches!(
        home.answer(&Request::Presence, Some(cookie)),
        Ok(Response::Presence {
            choice: Choice::Online
        })
    ));
    choose(c, Choice::Offline);
    assert_eq!(alice.text(), presence_of(&[], "online"));
    // its own sockets still hear its choice, and its friends as before
    assert_eq!(carol.text(), presence_of(&[(a.tag, 'o')], "offline"));
    carol.close();
    alice.quiet();
    // back to what the other scenarios expect
    choose(c, Choice::Online);
    alice.quiet();
    a.contact(http, c, ContactAction::Remove);
    assert_eq!(alice.text(), SOCIAL);
    alice.close();
}

/// A shared table (`docs/push-socket.md` §"Tables"): a friend hears the table
/// before the presence that shows the account at it, then each change of it
/// alone; one out of its bounds is dropped and the activity heard; nothing of
/// it reaches anybody while the account chose offline, nor a stranger, nor
/// across a block; and it goes with the socket.
fn tables(http: &Http, [a, b, c]: [&mut Player; 3]) {
    let report = |player: &Player, activity: Option<Activity>, table: Option<Table>| {
        assert!(matches!(
            player.ask(
                http,
                &Request::ReportPresence {
                    activity,
                    idle: false,
                    table,
                },
            ),
            Ok(Response::Done)
        ));
    };
    let choose = |player: &Player, choice: Choice| {
        assert!(matches!(
            player.ask(http, &Request::SetPresence { choice }),
            Ok(Response::Presence { choice: stored }) if stored == choice
        ));
    };
    let table_text =
        |player: &Player, table: Option<&Table>| table_frame(player.tag, table).unwrap();
    let shared = Table {
        origin: "https://gateway.example".into(),
        game: "01937d4e-7b1c-7000-8000-000000000001".into(),
        name: "Friday \"night\" \\ draft".into(),
        seats: 4,
        taken: 1,
    };
    // A and C are friends; B is a stranger to C
    a.contact(http, c, ContactAction::Request);
    c.contact(http, a, ContactAction::Accept);
    let mut alice = dial(http, &a.bearer()).unwrap().opened(&[]);
    let mut bob = dial(http, &b.bearer()).unwrap().opened(&[]);
    let mut carol = dial(http, &c.bearer()).unwrap().opened(&[a.tag]);
    assert_eq!(alice.text(), presence(&[c.tag]));
    // C sits down at a table it shares: A hears the table, then C at a table
    report(c, Some(Activity::Table), Some(shared.clone()));
    assert_eq!(alice.text(), table_text(c, Some(&shared)));
    assert_eq!(alice.text(), presence_of(&[(c.tag, 't')], "online"));
    // a seat taken: the table alone
    let fuller = Table {
        taken: 2,
        ..shared.clone()
    };
    report(c, Some(Activity::Table), Some(fuller.clone()));
    assert_eq!(alice.text(), table_text(c, Some(&fuller)));
    alice.quiet();
    // a table out of its bounds is dropped, and the game C plays is heard
    let elsewhere = Table {
        origin: "http://gateway.example".into(),
        ..shared.clone()
    };
    report(c, Some(Activity::Playing), Some(elsewhere));
    assert_eq!(alice.text(), table_text(c, None));
    assert_eq!(alice.text(), presence_of(&[(c.tag, 'g')], "online"));
    report(c, Some(Activity::Playing), Some(shared.clone()));
    assert_eq!(alice.text(), table_text(c, Some(&shared)));
    // C's own socket and the stranger hear nothing of it
    carol.quiet();
    bob.quiet();
    // C chooses offline: A sees C go with its table, and then nothing of
    // what C shares
    choose(c, Choice::Offline);
    assert_eq!(alice.text(), table_text(c, None));
    assert_eq!(alice.text(), presence_of(&[], "online"));
    assert_eq!(carol.text(), presence_of(&[(a.tag, 'o')], "offline"));
    report(c, Some(Activity::Table), Some(fuller.clone()));
    alice.quiet();
    // shown again: the table it sits at now
    choose(c, Choice::Online);
    assert_eq!(alice.text(), table_text(c, Some(&fuller)));
    assert_eq!(alice.text(), presence_of(&[(c.tag, 't')], "online"));
    assert_eq!(carol.text(), presence_of(&[(a.tag, 'o')], "online"));
    // C blocks A: the friendship and the table end for A, and what C shares
    // from then on never reaches A
    c.contact(http, a, ContactAction::Block);
    assert_eq!(
        alice.texts(3),
        BTreeSet::from([
            SOCIAL.to_owned(),
            table_text(c, None),
            presence_of(&[], "online")
        ])
    );
    assert_eq!(
        carol.texts(2),
        BTreeSet::from([SOCIAL.to_owned(), presence_of(&[], "online")])
    );
    report(c, Some(Activity::Table), Some(shared.clone()));
    alice.quiet();
    bob.quiet();
    // back to what the other scenarios expect: no contact between them
    c.contact(http, a, ContactAction::Unblock);
    assert_eq!(carol.text(), SOCIAL);
    report(c, None, None);
    for socket in [alice, bob, carol] {
        socket.close();
    }
}

/// A sign-in on the website, for its cookie.
fn website_sign_in(http: &Http, home: &Browser, identity: Identity) -> Token {
    let (login, message) = Login::start(OLD.into(), password::context(http, identity)).unwrap();
    let Response::LoginChallenge { attempt, message } = home
        .answer(&Request::LoginStart { identity, message }, None)
        .unwrap()
    else {
        panic!("a challenge");
    };
    let result = login.finish(&message).unwrap();
    home.call(
        &Request::LoginFinish {
            attempt,
            message: result.finalization,
        },
        None,
    )
    .unwrap()
    .session
    .unwrap()
}

/// Every end of a session closes its sockets with `revoked`: a sign-out, a
/// sign-in that replaced it, its end on the website, a new password, and its
/// expiry, which the periodic check finds.
fn revocation(
    http: &Http,
    home: &Browser,
    runtime: &Runtime,
    operator: &DatabaseConnection,
    [a, b, c]: [&mut Player; 3],
    cookie: &Token,
) {
    // A signs out
    let alice = dial(http, &a.bearer()).unwrap().opened(&[]);
    let mut carol = dial(http, &c.bearer()).unwrap().opened(&[]);
    a.ask(http, &Request::Logout).unwrap();
    alice.revoked();
    carol.quiet();
    // C signs in again: its old session's socket ends
    c.again(http);
    carol.revoked();
    // B's client session is ended on the website
    let bob = dial(http, &b.bearer()).unwrap().opened(&[]);
    let ended = home.answer(
        &Request::Owner {
            request: OwnerRequest::EndClientSession,
        },
        Some(cookie),
    );
    assert!(matches!(ended, Ok(Response::Done)));
    bob.revoked();
    // A new password, set by the client itself through the SDK
    let recording = super::tokens::Recording::new(http);
    let mut session = central_client::login(&recording, a.identity, OLD.into()).unwrap();
    session.confirm_restored(http).unwrap();
    a.token = recording.taken();
    let alice = dial(http, &a.bearer()).unwrap().opened(&[]);
    session
        .change_password(http, OLD.into(), NEW.into())
        .unwrap();
    alice.revoked();
    session.logout(http).unwrap();
    // C's session expires: nothing in this service ended it, the periodic
    // check (run here at once rather than in a minute) finds it
    let carol = dial(http, &c.bearer()).unwrap().opened(&[]);
    runtime
        .block_on(operator.execute_raw(sea_orm::Statement::from_sql_and_values(
            sea_orm::DbBackend::Postgres,
            "UPDATE central_identity.sessions SET created_at = pg_catalog.clock_timestamp() - interval '1 hour',
                 expires_at = pg_catalog.clock_timestamp() - interval '1 second'
             WHERE account_id = $1 AND kind = 'native'",
            vec![sea_orm::prelude::Uuid::from_bytes(c.identity.account).into()],
        )))
        .unwrap();
    runtime.block_on(http.identity.check_listeners()).unwrap();
    carol.revoked();
    // an ended session dials no more
    assert_eq!(dial(http, &c.bearer()).err().unwrap().status, 401);
    // B deletes its account: its socket ends with it
    let recording = super::tokens::Recording::new(http);
    let mut session = central_client::login(&recording, b.identity, OLD.into()).unwrap();
    session.confirm_restored(http).unwrap();
    b.token = recording.taken();
    let bob = dial(http, &b.bearer()).unwrap().opened(&[]);
    let deletion = session.start_deletion(http, OLD.into()).unwrap();
    session.delete_account(http, deletion).unwrap();
    bob.revoked();
    assert_eq!(dial(http, &b.bearer()).err().unwrap().status, 401);
}

/// Send `request` with `player`'s session on a connection of its own and
/// close the connection at once, without waiting for the answer: a caller
/// that goes away while its request is served.
fn fire_and_forget(http: &Http, player: &Player, request: &Request) {
    let body = central_api::encode(request).unwrap();
    let mut stream = TcpStream::connect_timeout(&http.address, Duration::from_secs(3)).unwrap();
    write!(
        stream,
        "POST /v1/native HTTP/1.1\r\nHost: {}\r\n{}: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAuthorization: {}\r\n\r\n",
        http.address,
        central_api::PROTOCOL_HEADER,
        central_api::PROTOCOL_VERSION,
        body.len(),
        player.bearer(),
    )
    .unwrap();
    stream.write_all(&body).unwrap();
    drop(stream);
}
