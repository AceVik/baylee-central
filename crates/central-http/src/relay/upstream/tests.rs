//! What a game host may answer, and what may be asked of it.
use super::*;
use central_api::ranked::replica::SetupBytes;
use std::{
    io::{Read as _, Write as _},
    net::TcpListener,
    thread,
};

fn secret(c: char) -> Secret {
    Secret::new(std::iter::repeat_n(c, 64).collect()).unwrap()
}

fn fence() -> Fence {
    Fence {
        match_id: [1; 16],
        generation: 3,
    }
}

fn play(deck: &str) -> Result<Post, Error> {
    let call = Call::Play {
        bearer: secret('a'),
        deck: SetupBytes::new(deck.as_bytes().to_vec())?,
    };
    prepare(call, fence(), None, None)
}

fn deck(cards: &[&str], sideboard: &[&str], commanders: &[&str]) -> String {
    let rows = |rows: &[&str]| central_api::encode(&rows).unwrap();
    format!(
        r#"{{"format":"standard","cards":{},"sideboard":{},"commanders":{}}}"#,
        String::from_utf8(rows(cards)).unwrap(),
        String::from_utf8(rows(sideboard)).unwrap(),
        String::from_utf8(rows(commanders)).unwrap(),
    )
}

#[test]
fn a_deck_row_is_a_count_and_a_name_without_notes() {
    assert!(play(&deck(&["60 Forest"], &["1 Island"], &["Any commander"])).is_ok());
    let long = format!("1 {}", "x".repeat(2046));
    assert!(play(&deck(&[&long], &[], &[])).is_ok());
    let longer = format!("1 {}", "x".repeat(2047));
    for row in [
        "",
        "Forest",
        "0 Forest",
        "-1 Forest",
        "1.5 Forest",
        "4294967296 Forest",
        "1 ",
        " 1 Forest",
        "1 Forest ",
        "1 Forest # mine",
        "1 Forest\n1 Island",
        "1 For\u{0}est",
        "1 For\u{85}est",
        "1\tForest",
        &longer,
    ] {
        assert!(play(&deck(&[row], &[], &[])).is_err(), "{row:?}");
        assert!(play(&deck(&["1 Forest"], &[row], &[])).is_err(), "{row:?}");
    }
    for commander in ["", " A", "A ", "A # note", "A\r\nB"] {
        assert!(
            play(&deck(&["1 Forest"], &[], &[commander])).is_err(),
            "{commander:?}"
        );
    }
    // A deck without cards is none, whatever its sideboard holds.
    assert!(play(&deck(&[], &["1 Forest"], &["A"])).is_err());
    for body in [
        "",
        "null",
        "[]",
        r#"{"format":"vintage","cards":["1 Forest"],"sideboard":[],"commanders":[]}"#,
        r#"{"format":"standard","cards":["1 Forest"],"sideboard":[]}"#,
        r#"{"format":"standard","cards":["1 Forest"],"sideboard":[],"commanders":[]}x"#,
    ] {
        assert!(play(body).is_err(), "{body}");
    }
}

#[test]
fn a_call_without_its_own_proof_or_outside_its_game_is_not_prepared() {
    let game = [3; 16];
    // An admission is made of Central's own proof, which a caller cannot supply.
    assert!(prepare(Call::Admit {}, fence(), None, None).is_err());
    for bearer in ["a".repeat(63), "A".repeat(64), "a".repeat(65)] {
        let odd = Secret::new(bearer).unwrap();
        for call in [
            Call::Play {
                bearer: odd.clone(),
                deck: SetupBytes::new(deck(&["1 Forest"], &[], &[]).into_bytes()).unwrap(),
            },
            Call::Reclaim {
                bearer: odd.clone(),
                game,
            },
            Call::SocketTicket {
                seat_token: odd.clone(),
                game,
            },
        ] {
            assert!(prepare(call, fence(), Some(game), None).is_err());
        }
    }
    for bound in [None, Some([4; 16])] {
        for call in [
            Call::Reclaim {
                bearer: secret('a'),
                game,
            },
            Call::SocketTicket {
                seat_token: secret('a'),
                game,
            },
        ] {
            assert!(prepare(call, fence(), bound, None).is_err());
        }
    }
    let post = prepare(
        Call::SocketTicket {
            seat_token: secret('b'),
            game,
        },
        fence(),
        Some(game),
        None,
    )
    .unwrap();
    assert_eq!(post.path, "/ws-ticket");
    assert_eq!(post.bearer, Some(secret('b')));
}

#[test]
fn a_host_s_grant_is_exactly_the_shape_of_the_call_it_answers() {
    let token = "c".repeat(64);
    let admitted =
        |token: &str, renew: &str| format!(r#"{{"token":"{token}","renew_in":{renew}}}"#);
    let chair = |token: &str, seat: &str| {
        format!(r#"{{"game_id":"g","seat":{seat},"seat_token":"{token}"}}"#)
    };
    let ticket = |token: &str, ttl: &str| format!(r#"{{"ticket":"{token}","expires_in":{ttl}}}"#);
    let ok = |op, body: &str| parse(op, body.as_bytes()).is_ok();

    for renew in ["1", "1800"] {
        assert!(ok(Operation::Admit, &admitted(&token, renew)), "{renew}");
    }
    for renew in ["0", "1801", "-1", "1.0", "null", "18446744073709551616"] {
        assert!(!ok(Operation::Admit, &admitted(&token, renew)), "{renew}");
    }
    for seat in ["0", "1"] {
        assert!(ok(Operation::Play, &chair(&token, seat)), "{seat}");
    }
    for seat in ["2", "255", "256", "-1", "\"0\""] {
        assert!(!ok(Operation::Play, &chair(&token, seat)), "{seat}");
    }
    let socket = || Operation::Socket([3; 16]);
    assert!(ok(socket(), &ticket(&token, "1")));
    for ttl in ["0", "-1", "0.5"] {
        assert!(!ok(socket(), &ticket(&token, ttl)), "{ttl}");
    }
    // A credential of any other shape is not kept.
    for bad in ["", "c", &token[1..], &token.to_uppercase(), "c d"] {
        assert!(!ok(Operation::Admit, &admitted(bad, "60")), "{bad:?}");
        assert!(!ok(Operation::Play, &chair(bad, "0")), "{bad:?}");
        assert!(!ok(socket(), &ticket(bad, "60")), "{bad:?}");
    }
    // No answer serves another call, and none carries more than its own fields.
    let bodies = [
        admitted(&token, "60"),
        chair(&token, "0"),
        ticket(&token, "60"),
    ];
    let operations = [Operation::Admit, Operation::Play, socket()];
    for (i, operation) in operations.into_iter().enumerate() {
        for (j, body) in bodies.iter().enumerate() {
            assert_eq!(ok(operation, body), i == j, "{i} {j}");
            let extra = body.replacen('{', r#"{"message":"hello","#, 1);
            assert!(!ok(operation, &extra), "{extra}");
        }
    }
}

/// What one host that answers `response` to everything leads to.
fn answered(call: &Post, response: &str) -> Result<ResultBody, Error> {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let response = response.to_owned();
    let host = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut seen = Vec::new();
        let mut chunk = [0; 4096];
        // The whole request: its head, then the body its length announces.
        loop {
            let read = stream.read(&mut chunk).unwrap();
            assert_ne!(read, 0, "the request ended early");
            seen.extend(&chunk[..read]);
            let text = String::from_utf8_lossy(&seen).to_lowercase();
            if let Some((head, body)) = text.split_once("\r\n\r\n") {
                let length: usize = head
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length: "))
                    .map_or(0, |length| length.parse().unwrap());
                if body.len() >= length {
                    break;
                }
            }
        }
        // Central may stop reading an answer that is too long.
        let _ = stream.write_all(response.as_bytes());
    });
    let result = send(&origin, true, call);
    host.join().unwrap();
    result
}

fn response(status: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

#[test]
fn only_known_refusals_cross_and_never_with_the_host_s_words() {
    let game = [3; 16];
    let play = play(&deck(&["1 Forest"], &[], &[])).unwrap();
    let ticket = prepare(
        Call::SocketTicket {
            seat_token: secret('a'),
            game,
        },
        fence(),
        Some(game),
        None,
    )
    .unwrap();
    for status in [400, 401, 403, 404, 409, 413, 426, 429, 503] {
        for call in [&play, &ticket] {
            assert!(
                matches!(
                    answered(call, &response(&format!("{status} X"), "operator text")),
                    Ok(ResultBody::Status(found)) if found == status
                ),
                "{status}"
            );
        }
    }
    // Waiting for the second deck is an answer to a deck alone.
    let waiting = response("202 Accepted", "");
    assert!(matches!(
        answered(&play, &waiting),
        Ok(ResultBody::Status(202))
    ));
    assert!(matches!(
        answered(&ticket, &waiting),
        Err(Error::Unavailable)
    ));
    for status in [201, 204, 302, 307, 402, 418, 500, 502] {
        let found = answered(
            &play,
            &format!(
                "HTTP/1.1 {status} X\r\nLocation: http://127.0.0.1:9/\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            ),
        );
        assert!(matches!(found, Err(Error::Unavailable)), "{status}");
    }
}

#[test]
fn a_grant_is_read_to_4096_bytes_and_no_further() {
    let game = [3; 16];
    let ticket = prepare(
        Call::SocketTicket {
            seat_token: secret('a'),
            game,
        },
        fence(),
        Some(game),
        None,
    )
    .unwrap();
    let grant = format!(r#"{{"ticket":"{}","expires_in":15}}"#, "d".repeat(64));
    let padded = |length: usize| format!("{grant}{}", " ".repeat(length - grant.len()));
    assert!(matches!(
        answered(&ticket, &response("200 OK", &padded(4096))),
        Ok(ResultBody::Socket { ttl, game: found, .. })
            if ttl == Duration::from_secs(15) && found == game
    ));
    assert!(matches!(
        answered(&ticket, &response("200 OK", &padded(4097))),
        Err(Error::Unavailable)
    ));
}
