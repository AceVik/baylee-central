//! A synthetic HTTP listener records exactly the exchange emitted at the game host.
use super::upstream;
use central_api::{
    SignedToken, Token,
    ranked::replica::{
        Fence, SetupBytes,
        relay::{Call, Secret},
    },
};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener},
    thread,
    time::Duration,
};
use zeroize::Zeroizing;

fn fence() -> Fence {
    Fence {
        match_id: [1; 16],
        generation: 3,
    }
}
fn secret() -> Secret {
    Secret::new("a".repeat(64)).unwrap()
}

#[test]
fn upstream_sees_only_relay_peer_fixed_path_and_scoped_admission() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let credential = Token::generate().unwrap();
    let json = format!(
        r#"{{"token":"{}","renew_in":60}}"#,
        credential.hex().as_str()
    );
    let worker = thread::spawn(move || {
        let (mut stream, peer) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut received = Vec::new();
        let mut buffer = [0; 2048];
        let (header, body) = loop {
            let n = stream.read(&mut buffer).unwrap();
            assert!(n > 0 && received.len() + n < 8192);
            received.extend_from_slice(&buffer[..n]);
            if let Some(end) = received.windows(4).position(|part| part == b"\r\n\r\n") {
                let header = std::str::from_utf8(&received[..end])
                    .unwrap()
                    .to_ascii_lowercase();
                let length: usize = header
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length: "))
                    .unwrap()
                    .parse()
                    .unwrap();
                if received.len() == end + 4 + length {
                    break (header, received[end + 4..].to_vec());
                }
            }
        };
        write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{json}",json.len()).unwrap();
        (header, body, peer)
    });
    let call = upstream::prepare(
        Call::Admit {},
        fence(),
        None,
        Some(&SignedToken::new(Zeroizing::new("ranked.test.proof".into())).unwrap()),
    )
    .unwrap();
    assert!(upstream::send(&format!("http://{address}"), true, &call).is_ok());
    let (header, body, peer): (String, Vec<u8>, SocketAddr) = worker.join().unwrap();
    assert!(peer.ip().is_loopback());
    assert!(header.starts_with("post /auth/ranked/login http/1.1\r\n"));
    for forbidden in [
        "forwarded:",
        "x-forwarded-for:",
        "x-real-ip:",
        "authorization:",
        "cookie:",
        "origin:",
        "referer:",
    ] {
        assert!(!header.contains(forbidden));
    }
    assert_eq!(body, br#"{"token":"ranked.test.proof"}"#);
}

#[test]
fn dispatch_refuses_notes_extra_fields_cross_game_and_explicit_remote_targets() {
    for bad in [
        r#"{"format":"standard","cards":["1 Forest # private"],"sideboard":[],"commanders":[]}"#,
        r#"{"format":"standard","cards":["1 Forest"],"sideboard":[],"commanders":[],"owner":"private"}"#,
    ] {
        let call = Call::Play {
            bearer: secret(),
            deck: SetupBytes::new(bad.as_bytes().to_vec()).unwrap(),
        };
        assert!(upstream::prepare(call, fence(), None, None).is_err());
    }
    let call = Call::SocketTicket {
        seat_token: secret(),
        game: [3; 16],
    };
    assert!(upstream::prepare(call, fence(), Some([4; 16]), None).is_err());
    for (url, dev) in [
        ("http://203.0.113.15:9021", true),
        ("http://127.0.0.1:9021", false),
        ("http://localhost:9021", true),
        ("https://host.example/path", false),
    ] {
        let call = upstream::prepare(
            Call::Reclaim {
                bearer: secret(),
                game: [3; 16],
            },
            fence(),
            Some([3; 16]),
            None,
        )
        .unwrap();
        assert!(upstream::send(url, dev, &call).is_err());
    }
}
