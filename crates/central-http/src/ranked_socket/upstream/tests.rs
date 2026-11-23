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
        "GET /games/{}/ws?ticket={ticket}&protocol=69 HTTP/1.1\r\n",
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
