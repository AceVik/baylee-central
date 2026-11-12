//! The client against a stand-in mailer: a TCP listener on an ephemeral
//! loopback port that answers the connections a test scripted. Every test
//! stops its stand-in and sees the port closed. Addresses, names and tokens
//! are synthetic.
use super::*;
use central_api::{email::MailCode, reset_code::ResetCode};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::mpsc,
    thread,
    time::Instant,
};

const SITE: &str = "https://baylee.example";
const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// One request as the stand-in read it.
struct Seen {
    head: String,
    body: Vec<u8>,
}

impl Seen {
    /// The value of the header `name`, which the request has once.
    fn header(&self, name: &str) -> &str {
        let mut values = self.head.lines().skip(1).filter_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.eq_ignore_ascii_case(name).then_some(value.trim())
        });
        let value = values.next().unwrap_or_else(|| panic!("no {name} header"));
        assert!(values.next().is_none(), "{name} twice");
        value
    }

    fn json(&self) -> serde_json::Value {
        serde_json::from_slice(&self.body).unwrap()
    }
}

/// What the stand-in does with a connection after reading its request.
enum Script {
    /// Write these bytes and close.
    Answer(String),
    /// Keep the connection open and silent until the test lets go.
    Hold(mpsc::Receiver<()>),
}

struct Standin {
    address: SocketAddr,
    seen: mpsc::Receiver<Seen>,
    served: thread::JoinHandle<()>,
}

impl Standin {
    /// Serve one connection for each entry of `script`, in order, then stop
    /// listening.
    fn start(script: Vec<Script>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (tell, seen) = mpsc::channel();
        let served = thread::spawn(move || {
            for step in script {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(10)))
                    .unwrap();
                tell.send(read(&mut stream)).unwrap();
                match step {
                    Script::Answer(answer) => stream.write_all(answer.as_bytes()).unwrap(),
                    Script::Hold(until) => until.recv().unwrap(),
                }
            }
        });
        Self {
            address,
            seen,
            served,
        }
    }

    fn answering(status: &str, body: &str) -> Self {
        Self::start(vec![Script::Answer(answer(status, body))])
    }

    fn client(&self) -> Client {
        let endpoint = format!("http://{}", self.address);
        Client::new(&endpoint, SITE, Token::new(TOKEN.into()).unwrap(), false).unwrap()
    }

    /// Wait for the stand-in to end, see that its port is closed, and
    /// return what it read.
    fn stop(self) -> Vec<Seen> {
        self.served.join().unwrap();
        assert!(
            TcpStream::connect_timeout(&self.address, Duration::from_secs(1)).is_err(),
            "the stand-in's port is closed"
        );
        self.seen.try_iter().collect()
    }
}

/// Read one request: its head, and as many bytes of body as it announces.
fn read(stream: &mut TcpStream) -> Seen {
    let mut bytes = Vec::new();
    let mut chunk = [0; 4096];
    let end = loop {
        if let Some(at) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break at + 4;
        }
        let count = stream.read(&mut chunk).unwrap();
        assert!(count > 0, "the request ended inside its head");
        bytes.extend_from_slice(&chunk[..count]);
    };
    let head = String::from_utf8(bytes[..end].to_vec()).unwrap();
    let length: usize = head
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse().unwrap())
        })
        .expect("a body with a length");
    while bytes.len() < end + length {
        let count = stream.read(&mut chunk).unwrap();
        assert!(count > 0, "the request ended inside its body");
        bytes.extend_from_slice(&chunk[..count]);
    }
    Seen {
        head,
        body: bytes[end..].to_vec(),
    }
}

fn answer(status: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nCache-Control: no-store\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

fn confirm(code: &MailCode) -> Mail {
    Mail::confirm("player@example.org", [0xab; 16], "de", code, 15)
}

#[test]
fn a_mail_is_one_request_as_the_mailer_asks_for_it() {
    let queued = r#"{"id":"1","status":"queued"}"#;
    let mailer = Standin::start(vec![
        Script::Answer(answer("202 Accepted", queued)),
        Script::Answer(answer("202 Accepted", queued)),
        Script::Answer(answer("202 Accepted", queued)),
    ]);
    let client = mailer.client();
    let code = MailCode::generate().unwrap();
    let reset = ResetCode::generate().unwrap();
    assert_eq!(client.send(&confirm(&code)), Outcome::Sent);
    assert_eq!(
        client.send(&Mail::reset(
            "player@example.org",
            [0xab; 16],
            "en",
            "player.one",
            &reset,
            60
        )),
        Outcome::Sent
    );
    assert_eq!(
        client.send(&Mail::removed("player@example.org", [0xab; 16], "en")),
        Outcome::Sent
    );
    let host = mailer.address.to_string();
    let seen = mailer.stop();
    assert_eq!(seen.len(), 3);
    let mut keys = Vec::new();
    for request in &seen {
        assert_eq!(request.head.lines().next(), Some("POST /v1/send HTTP/1.1"));
        assert_eq!(request.header("host"), host);
        assert_eq!(request.header("authorization"), format!("Bearer {TOKEN}"));
        assert_eq!(request.header("content-type"), "application/json");
        let key = request.header("idempotency-key");
        assert!(key.len() == 32 && key.bytes().all(|b| b.is_ascii_hexdigit()));
        keys.push(key);
        // The token is in its header and nowhere else; no code is in the head.
        assert_eq!(request.head.matches(TOKEN).count(), 1);
        assert!(!String::from_utf8_lossy(&request.body).contains(TOKEN));
        for secret in [code.display(), reset.display()] {
            assert!(!request.head.contains(secret.as_str()));
        }
        let body = request.json();
        assert_eq!(body["to"], "player@example.org");
        assert_eq!(
            body["limit_key"],
            "account:abababababababababababababababab"
        );
        for field in ["subject", "text", "html"] {
            assert!(!body[field].as_str().unwrap().trim().is_empty(), "{field}");
        }
    }
    assert!(keys[0] != keys[1] && keys[1] != keys[2] && keys[0] != keys[2]);
    // The fields the mailer knows and no other; a code's life is the
    // mail's, and a notice has none.
    let fields = |request: &Seen| -> Vec<String> {
        request
            .json()
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect()
    };
    let with_life = ["html", "limit_key", "subject", "text", "to", "ttl_secs"];
    assert_eq!(fields(&seen[0]), with_life);
    assert_eq!(fields(&seen[1]), with_life);
    assert_eq!(
        fields(&seen[2]),
        ["html", "limit_key", "subject", "text", "to"]
    );
    assert_eq!(seen[0].json()["ttl_secs"], 900);
    assert_eq!(seen[1].json()["ttl_secs"], 3600);
    let text = |request: &Seen| request.json()["text"].as_str().unwrap().to_owned();
    assert!(text(&seen[0]).contains(code.display().as_str()));
    assert!(text(&seen[0]).contains("Der Code gilt 15 Minuten."));
    assert!(text(&seen[1]).contains(reset.display().as_str()));
    assert!(text(&seen[1]).contains("https://baylee.example/en/reset"));
}

#[test]
fn the_mailers_answers_become_outcomes() {
    let long = format!(
        r#"{{"error":"invalid_address","field":"{}"}}"#,
        "x".repeat(5000)
    );
    for (status, body, expected) in [
        (
            "202 Accepted",
            r#"{"id":"1","status":"queued"}"#,
            Outcome::Sent,
        ),
        ("200 OK", r#"{"id":"1","status":"sent"}"#, Outcome::Sent),
        (
            "429 Too Many Requests",
            r#"{"error":"key_limited"}"#,
            Outcome::Busy,
        ),
        (
            "429 Too Many Requests",
            r#"{"error":"recipient_limited"}"#,
            Outcome::Busy,
        ),
        (
            "503 Service Unavailable",
            r#"{"error":"queue_full"}"#,
            Outcome::Busy,
        ),
        (
            "400 Bad Request",
            r#"{"error":"invalid_address","field":"to"}"#,
            Outcome::Rejected,
        ),
        (
            "400 Bad Request",
            r#"{"error":"invalid_ttl","field":"ttl_secs"}"#,
            Outcome::Unavailable,
        ),
        (
            "400 Bad Request",
            r#"{"error":"invalid_json"}"#,
            Outcome::Unavailable,
        ),
        ("400 Bad Request", "invalid_address", Outcome::Unavailable),
        ("400 Bad Request", "", Outcome::Unavailable),
        // A refusal longer than the client reads says nothing.
        ("400 Bad Request", long.as_str(), Outcome::Unavailable),
        (
            "401 Unauthorized",
            r#"{"error":"unauthorized"}"#,
            Outcome::Unavailable,
        ),
        (
            "413 Content Too Large",
            r#"{"error":"too_large"}"#,
            Outcome::Unavailable,
        ),
        (
            "500 Internal Server Error",
            r#"{"error":"internal"}"#,
            Outcome::Unavailable,
        ),
        // Statuses the mailer does not give for a mail are no success.
        ("201 Created", r#"{"id":"1"}"#, Outcome::Unavailable),
        ("204 No Content", "", Outcome::Unavailable),
        (
            "404 Not Found",
            r#"{"error":"invalid_address"}"#,
            Outcome::Unavailable,
        ),
    ] {
        let mailer = Standin::answering(status, body);
        let outcome = mailer
            .client()
            .send(&confirm(&MailCode::generate().unwrap()));
        assert_eq!(outcome, expected, "{status} {body:.40}");
        assert_eq!(mailer.stop().len(), 1);
    }
}

#[test]
fn a_redirect_is_not_followed_and_a_long_head_is_not_read() {
    // Where the redirect points, nobody is asked: the listener sees no
    // connection.
    let elsewhere = TcpListener::bind("127.0.0.1:0").unwrap();
    elsewhere.set_nonblocking(true).unwrap();
    let target = elsewhere.local_addr().unwrap();
    for status in [
        "301 Moved Permanently",
        "302 Found",
        "307 Temporary Redirect",
    ] {
        let mailer = Standin::start(vec![Script::Answer(format!(
            "HTTP/1.1 {status}\r\nLocation: http://{target}/v1/send\r\nContent-Length: 0\r\n\
             Connection: close\r\n\r\n"
        ))]);
        let outcome = mailer
            .client()
            .send(&confirm(&MailCode::generate().unwrap()));
        assert_eq!(outcome, Outcome::Unavailable, "{status}");
        assert_eq!(mailer.stop().len(), 1);
        assert!(
            elsewhere.accept().is_err(),
            "{status}: the redirect was followed"
        );
    }
    drop(elsewhere);
    assert!(TcpStream::connect_timeout(&target, Duration::from_secs(1)).is_err());
    // A head past the bound, which would say the mail was taken.
    let mailer = Standin::start(vec![Script::Answer(format!(
        "HTTP/1.1 202 Accepted\r\nX-Filler: {}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        "x".repeat(HEAD_BYTES)
    ))]);
    let outcome = mailer
        .client()
        .send(&confirm(&MailCode::generate().unwrap()));
    assert_eq!(outcome, Outcome::Unavailable);
    mailer.stop();
}

#[test]
fn no_mailer_and_a_silent_one_are_unavailable_within_the_bound() {
    // Nothing listens: the port of a listener that is gone.
    let gone = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", gone.local_addr().unwrap());
    drop(gone);
    let client = Client::new(&endpoint, SITE, Token::new(TOKEN.into()).unwrap(), false).unwrap();
    let asked = Instant::now();
    assert_eq!(
        client.send(&confirm(&MailCode::generate().unwrap())),
        Outcome::Unavailable
    );
    assert!(asked.elapsed() < CONNECT + Duration::from_secs(1));
    // A mailer that takes the request and says nothing: the whole request
    // ends at its bound.
    let (release, until) = mpsc::channel();
    let mailer = Standin::start(vec![Script::Hold(until)]);
    let asked = Instant::now();
    assert_eq!(
        mailer
            .client()
            .send(&confirm(&MailCode::generate().unwrap())),
        Outcome::Unavailable
    );
    let waited = asked.elapsed();
    assert!(
        waited + Duration::from_secs(1) >= OVERALL && waited < OVERALL + Duration::from_secs(3),
        "{waited:?}"
    );
    release.send(()).unwrap();
    assert_eq!(mailer.stop().len(), 1);
}

#[test]
fn the_agent_reaches_the_configured_address_and_no_other() {
    // A listener that would see a connection, at an address the client was
    // not made for.
    let other = TcpListener::bind("127.0.0.1:0").unwrap();
    other.set_nonblocking(true).unwrap();
    let other_port = other.local_addr().unwrap().port();
    let client = Client::new(
        "http://127.0.0.1:9",
        SITE,
        Token::new(TOKEN.into()).unwrap(),
        false,
    )
    .unwrap();
    let asked = Instant::now();
    for url in [
        format!("http://127.0.0.1:{other_port}/v1/send"),
        format!("http://localhost:{other_port}/v1/send"),
        format!("http://[::1]:{other_port}/v1/send"),
        "http://localhost:9/v1/send".into(),
        "http://127.0.0.1/v1/send".into(),
        "https://127.0.0.1:9/v1/send".into(),
        "http://192.0.2.1:9/v1/send".into(),
        "http://mailer.test:9/v1/send".into(),
    ] {
        assert!(client.agent.post(&url).send("x").is_err(), "{url}");
    }
    assert!(
        asked.elapsed() < Duration::from_secs(2),
        "refused without a connection or a lookup"
    );
    assert!(other.accept().is_err(), "nothing was connected to");
    // A resolver made for an address that is no loopback answers nothing.
    let stray = Loopback("192.0.2.1:9".parse().unwrap());
    let agent = Agent::with_parts(
        Agent::config_builder().build(),
        DefaultConnector::default(),
        stray,
    );
    assert!(agent.post("http://192.0.2.1:9/v1/send").send("x").is_err());
}

#[test]
fn an_endpoint_that_is_not_loopback_http_is_refused_at_start() {
    let token = || Token::new(TOKEN.into()).unwrap();
    for endpoint in [
        "http://127.0.0.1:28795",
        "http://127.0.0.2:1",
        "http://[::1]:28795",
    ] {
        assert!(
            Client::new(endpoint, SITE, token(), false).is_ok(),
            "{endpoint}"
        );
    }
    for endpoint in [
        "",
        "127.0.0.1:28795",
        "https://127.0.0.1:28795",
        "HTTP://127.0.0.1:28795",
        "http://localhost:28795",
        "http://mailer.test:28795",
        "http://127.0.0.1",
        "http://127.0.0.1:0",
        "http://127.0.0.1:28795/",
        "http://127.0.0.1:28795/v1/send",
        "http://127.0.0.1:28795?x=1",
        "http://user@127.0.0.1:28795",
        "http://0.0.0.0:28795",
        "http://10.0.0.1:28795",
        "http://192.0.2.1:25",
        "http://[::]:28795",
        "http://[::ffff:127.0.0.1]:28795",
        "http://[2001:db8::1]:28795",
        "http://[::1%1]:28795",
        "http://127.1:28795",
        "http://2130706433:28795",
        " http://127.0.0.1:28795",
        "http://127.0.0.1:28795 ",
    ] {
        assert_eq!(
            Client::new(endpoint, SITE, token(), false).err(),
            Some(Error::Rejected),
            "{endpoint}"
        );
    }
}

#[test]
fn the_site_is_a_canonical_origin() {
    let token = || Token::new(TOKEN.into()).unwrap();
    let mailer = "http://127.0.0.1:28795";
    assert!(Client::new(mailer, "https://baylee.example", token(), false).is_ok());
    assert!(Client::new(mailer, "http://127.0.0.1:28782", token(), true).is_ok());
    for (site, development) in [
        ("", false),
        ("baylee.example", false),
        ("http://baylee.example", false),
        ("https://baylee.example/", false),
        ("https://baylee.example/en", false),
        ("https://user@baylee.example", false),
        ("https://baylee.example?x=1", false),
        ("https://BAYLEE.example", false),
        ("https://baylee.example.", false),
        ("http://127.0.0.1:28782", false),
        ("https://baylee.example", true),
        ("http://localhost:28782", true),
        ("http://192.0.2.1:28782", true),
    ] {
        assert_eq!(
            Client::new(mailer, site, token(), development).err(),
            Some(Error::Rejected),
            "{site}"
        );
    }
}

#[test]
fn a_token_is_a_header_value_and_never_printed() {
    let token = Token::new(TOKEN.into()).unwrap();
    assert_eq!(format!("{token:?}"), "Token([REDACTED])");
    let client = Client::new("http://127.0.0.1:28795", SITE, token, false).unwrap();
    let printed = format!("{client:?}");
    assert_eq!(printed, "Client([REDACTED])");
    assert!(!printed.contains(TOKEN) && !printed.contains("127.0.0.1"));
    assert!(Token::new("A-._~+/=".repeat(4)).is_ok());
    assert!(Token::new("a".repeat(TOKEN_MAX)).is_ok());
    let spaced = format!("{} {}", &TOKEN[..31], &TOKEN[..31]);
    let broken = format!("{TOKEN}\r\nX-Other: 1");
    for bad in [
        String::new(),
        "a".repeat(TOKEN_MIN - 1),
        "a".repeat(TOKEN_MAX + 1),
        spaced,
        broken,
        format!("{TOKEN}\n"),
        format!("Bearer {TOKEN}"),
        "\u{e9}".repeat(TOKEN_MIN),
        format!("{}\"", &TOKEN[..40]),
    ] {
        assert_eq!(Token::new(bad).err(), Some(Error::Rejected));
    }
}
