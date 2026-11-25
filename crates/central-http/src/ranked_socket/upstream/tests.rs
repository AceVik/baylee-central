use super::*;
use tokio::{io::AsyncBufReadExt as _, net::TcpListener};

#[test]
fn live_destinations_require_exclusively_public_pinned_answers() {
    let good = "8.8.8.8:443".parse().unwrap();
    let local = "127.0.0.1:443".parse().unwrap();
    let mapped = "[::ffff:192.168.1.10]:443".parse().unwrap();
    assert_eq!(public_addresses([good].into_iter()).unwrap(), [good]);
    for bad in [
        vec![],
        vec![good, local],
        vec![mapped, good],
        vec![good; 33],
    ] {
        assert!(public_addresses(bad.into_iter()).is_err());
    }
}

#[test]
fn refused_negotiation_and_redirects_never_open_another_origin() {
    // RFC 6455's public handshake example, expressed as its original text.
    let key = STANDARD.encode(b"the sample nonce");
    let good = "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: keep-alive, Upgrade\r\nSec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n";
    assert!(verify(format!("{good}\r\n").as_bytes(), &key).is_ok());
    for bad in [
        good.replace("101", "307"),
        good.replace("+xOo=", "abcd="),
        format!("{good}Sec-WebSocket-Extensions: permessage-deflate\r\n"),
        format!("{good}Upgrade: websocket\r\n"),
    ] {
        assert!(verify(format!("{bad}\r\n").as_bytes(), &key).is_err());
    }
}

#[tokio::test]
async fn host_receives_only_fixed_upgrade_headers_and_game_bytes_survive_read_ahead() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let id = [8; 16];
    let ticket = "e".repeat(64);
    let expected = format!(
        "GET /games/{}/ws?ticket={ticket}&protocol=70 HTTP/1.1\r\n",
        game_id(&id)
    );
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = BufReader::new(stream);
        let mut line = String::new();
        socket.read_line(&mut line).await.unwrap();
        assert_eq!(line, expected);
        let mut names = Vec::new();
        let mut key = String::new();
        loop {
            line.clear();
            socket.read_line(&mut line).await.unwrap();
            if line == "\r\n" {
                break;
            }
            let (name, value) = line.split_once(':').unwrap();
            names.push(name.to_owned());
            if name == "Sec-WebSocket-Key" {
                key = value.trim().to_owned();
            }
        }
        assert_eq!(
            names,
            [
                "Host",
                "Connection",
                "Upgrade",
                "Sec-WebSocket-Version",
                "Sec-WebSocket-Key"
            ]
        );
        let accept = ws::accept(&key).unwrap();
        let reply = format!(
            "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {}\r\n\r\n",
            accept.to_str().unwrap()
        );
        let mut bytes = reply.into_bytes();
        bytes.extend([0x82, 1, 42]);
        socket.write_all(&bytes).await.unwrap();
    });
    let mut host = connect(&format!("http://{addr}"), true, &id, &ticket)
        .await
        .unwrap();
    let mut bytes = [0; 3];
    host.io.read_exact(&mut bytes).await.unwrap();
    assert_eq!(bytes, [0x82, 1, 42]);
    server.await.unwrap();
}

#[test]
fn an_answer_that_is_not_exactly_one_plain_upgrade_is_refused() {
    let key = STANDARD.encode(b"the sample nonce");
    let accept = "Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n";
    let status = "HTTP/1.1 101 Switching Protocols\r\n";
    let head = |status: &str, lines: &[&str]| format!("{status}{}\r\n", lines.concat());
    let (upgrade, connection) = ("Upgrade: websocket\r\n", "Connection: Upgrade\r\n");
    assert!(
        verify(
            head(status, &[upgrade, connection, accept]).as_bytes(),
            &key
        )
        .is_ok()
    );
    // Header names and tokens are compared without case, the accept value is not.
    assert!(
        verify(
            head(
                status,
                &[
                    "UPGRADE: WebSocket\r\n",
                    "connection: x, uPgRaDe\r\n",
                    accept
                ]
            )
            .as_bytes(),
            &key
        )
        .is_ok()
    );
    for lines in [
        vec![connection, accept],
        vec![upgrade, accept],
        vec![upgrade, connection],
        vec![upgrade, connection, accept, accept],
        vec![upgrade, connection, connection, accept],
        vec![upgrade, "Connection: keep-alive\r\n", accept],
        vec![upgrade, "Connection: upgraded\r\n", accept],
        vec!["Upgrade: websocket, h2c\r\n", connection, accept],
        vec!["Upgrade: h2c\r\n", connection, accept],
        vec![upgrade, connection, accept, "Content-Length: 0\r\n"],
        vec![
            upgrade,
            connection,
            accept,
            "transfer-encoding: chunked\r\n",
        ],
        vec![
            upgrade,
            connection,
            accept,
            "Sec-WebSocket-Protocol: other\r\n",
        ],
        vec![
            upgrade,
            connection,
            "Sec-WebSocket-Accept: S3PPLMBITXAQ9KYGZZHZRBK+XOO=\r\n",
        ],
    ] {
        assert!(
            verify(head(status, &lines).as_bytes(), &key).is_err(),
            "{lines:?}"
        );
    }
    let lines = [upgrade, connection, accept];
    for status in [
        "HTTP/1.0 101 Switching Protocols\r\n",
        "HTTP/1.1 100 Continue\r\n",
        "HTTP/1.1 200 OK\r\n",
        "HTTP/1.1 302 Found\r\nLocation: https://elsewhere.example/\r\n",
        "GET / HTTP/1.1\r\n",
    ] {
        assert!(
            verify(head(status, &lines).as_bytes(), &key).is_err(),
            "{status}"
        );
    }
    // Cut short, and with more headers than are read.
    let whole = head(status, &lines);
    assert!(verify(&whole.as_bytes()[..whole.len() - 2], &key).is_err());
    let many = "X-Other: 1\r\n".repeat(30);
    let mut lines = vec![upgrade, connection, accept, many.as_str()];
    assert!(verify(head(status, &lines).as_bytes(), &key).is_err());
    lines.pop();
    let few = "X-Other: 1\r\n".repeat(29);
    lines.push(&few);
    assert!(verify(head(status, &lines).as_bytes(), &key).is_ok());
}

/// Whether the host is refused at once, rather than dialled and waited for.
async fn declined(origin: &str, development: bool, ticket: &str) -> bool {
    let dial = connect(origin, development, &[8; 16], ticket);
    tokio::time::timeout(std::time::Duration::from_secs(2), dial)
        .await
        .is_ok_and(|result| result.is_err())
}

#[tokio::test]
async fn a_ticket_or_origin_that_is_not_plain_never_leads_to_a_connection() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    let good = "e".repeat(64);
    let local = format!("http://127.0.0.1:{port}");
    let tickets = [
        String::new(),
        "e".repeat(63),
        "e".repeat(65),
        "E".repeat(64),
        format!("{}\r\n", "e".repeat(62)),
        format!("{}&protocol=1", "e".repeat(53)),
        format!("{} HTTP/1.1", "e".repeat(55)),
        "é".repeat(32),
    ];
    for ticket in &tickets {
        assert!(declined(&local, true, ticket).await, "{ticket:?}");
    }
    for (origin, development) in [
        // Plain HTTP outside development.
        (local.clone(), false),
        // HTTPS to this machine, in development too.
        (format!("https://127.0.0.1:{port}"), false),
        (format!("https://127.0.0.1:{port}"), true),
        (format!("https://localhost:{port}"), false),
        (format!("https://[::ffff:127.0.0.1]:{port}"), false),
        (format!("http://localhost:{port}"), true),
        (format!("http://127.0.0.1:{port}/games"), true),
        (format!("http://user@127.0.0.1:{port}"), true),
        (format!("ws://127.0.0.1:{port}"), true),
    ] {
        assert!(declined(&origin, development, &good).await, "{origin}");
    }
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}
