use super::*;
use tokio::io::AsyncWriteExt;

/// A client's frame: `first` byte, masked with `mask`, carrying `payload`.
fn masked(first: u8, payload: &[u8], mask: [u8; 4]) -> Vec<u8> {
    let mut frame = vec![first, 0x80 | u8::try_from(payload.len()).unwrap()];
    frame.extend_from_slice(&mask);
    frame.extend(
        payload
            .iter()
            .enumerate()
            .map(|(index, byte)| byte ^ mask[index % 4]),
    );
    frame
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

/// Everything `bytes` reads as, frame by frame, until it ends.
fn read_all(bytes: &[u8]) -> Vec<Result<Frame, Ended>> {
    runtime().block_on(async {
        let mut source = bytes;
        let mut reader = Reader::new();
        let mut frames = Vec::new();
        loop {
            let frame = reader.next(&mut source).await;
            let last = frame.is_err();
            frames.push(frame);
            if last {
                return frames;
            }
        }
    })
}

#[test]
fn control_frames_are_read_and_unmasked() {
    // RFC 6455 § 5.7: a masked "Hello" ping, and the pong a server returns.
    let ping = [
        0x89, 0x85, 0x37, 0xfa, 0x21, 0x3d, 0x7f, 0x9f, 0x4d, 0x51, 0x58,
    ];
    let mut bytes = ping.to_vec();
    bytes.extend(masked(0x8a, b"", [1, 2, 3, 4]));
    bytes.extend(masked(0x88, &1000_u16.to_be_bytes(), [9, 8, 7, 6]));
    bytes.extend(masked(0x88, b"", [0; 4]));
    assert_eq!(
        read_all(&bytes),
        vec![
            Ok(Frame::Ping(b"Hello".to_vec())),
            Ok(Frame::Pong),
            Ok(Frame::Close),
            Ok(Frame::Close),
            Err(Ended::Gone),
        ]
    );
    assert_eq!(pong(b"Hello"), [0x8a, 0x05, b'H', b'e', b'l', b'l', b'o']);
    // the longest control frame fits the reader exactly
    let longest = masked(0x89, &[0x55; 125], [3; 4]);
    assert_eq!(longest.len(), LONGEST);
    assert_eq!(read_all(&longest)[0], Ok(Frame::Ping(vec![0x55; 125])));
}

#[test]
fn frames_the_socket_does_not_take_end_it_before_their_payload() {
    for (bytes, code) in [
        // a text frame, a binary one, a continuation: no data from the client
        (masked(0x81, b"hi", [1; 4]), Code::UNSUPPORTED),
        (masked(0x82, b"hi", [1; 4]), Code::UNSUPPORTED),
        (masked(0x80, b"hi", [1; 4]), Code::UNSUPPORTED),
        // a fragment of a text, announced long: refused at its header
        (vec![0x01, 0xff, 0, 0, 0, 0, 0, 1, 0, 0], Code::UNSUPPORTED),
        // reserved opcodes and bits
        (masked(0x83, b"", [1; 4]), Code::PROTOCOL),
        (masked(0x8b, b"", [1; 4]), Code::PROTOCOL),
        (masked(0xc9, b"", [1; 4]), Code::PROTOCOL),
        // an unfinished control frame
        (masked(0x09, b"", [1; 4]), Code::PROTOCOL),
        // unmasked
        (vec![0x89, 0x00], Code::PROTOCOL),
        // a control frame announced longer than 125 bytes, either way
        (vec![0x89, 0xfe, 0x00, 0x7e], Code::PROTOCOL),
        (vec![0x89, 0xff, 0, 0, 0, 0, 0, 0, 0, 0x7e], Code::PROTOCOL),
        // a close with half a code
        (masked(0x88, &[3], [1; 4]), Code::PROTOCOL),
    ] {
        assert_eq!(
            read_all(&bytes).first(),
            Some(&Err(Ended::Refused(code))),
            "{bytes:02x?}"
        );
    }
}

#[test]
fn a_frame_read_in_pieces_and_a_read_cut_short_lose_nothing() {
    runtime().block_on(async {
        let (mut client, mut server) = tokio::io::duplex(64);
        let mut reader = Reader::new();
        let frame = masked(0x89, b"piecewise", [5, 6, 7, 8]);
        for byte in &frame[..7] {
            client.write_all(&[*byte]).await.unwrap();
            // The read is begun and dropped before the frame is whole, as a
            // `select!` drops it when another branch wins.
            let cut = tokio::time::timeout(
                std::time::Duration::from_millis(5),
                reader.next(&mut server),
            )
            .await;
            assert!(cut.is_err(), "not whole yet");
        }
        client.write_all(&frame[7..]).await.unwrap();
        assert_eq!(
            reader.next(&mut server).await,
            Ok(Frame::Ping(b"piecewise".to_vec()))
        );
        // two frames in one write are two frames
        let mut two = masked(0x8a, b"", [1; 4]);
        two.extend(masked(0x89, b"x", [2; 4]));
        client.write_all(&two).await.unwrap();
        assert_eq!(reader.next(&mut server).await, Ok(Frame::Pong));
        assert_eq!(
            reader.next(&mut server).await,
            Ok(Frame::Ping(b"x".to_vec()))
        );
        drop(client);
        assert_eq!(reader.next(&mut server).await, Err(Ended::Gone));
    });
}

#[test]
fn the_server_s_frames_are_rfc_6455_s() {
    // RFC 6455 § 5.7: an unmasked "Hello"
    assert_eq!(
        text("Hello").unwrap(),
        [0x81, 0x05, 0x48, 0x65, 0x6c, 0x6c, 0x6f]
    );
    // 126 bytes and more take the two-byte length
    let long = "x".repeat(126);
    let frame = text(&long).unwrap();
    assert_eq!(&frame[..4], &[0x81, 126, 0x00, 126]);
    assert_eq!(frame.len(), 4 + 126);
    let longest = "y".repeat(TEXT);
    assert_eq!(&text(&longest).unwrap()[..4], &[0x81, 126, 0x10, 0x00]);
    assert_eq!(text(&"z".repeat(TEXT + 1)), None);
    assert_eq!(PING, [0x89, 0x00]);
    assert_eq!(close(Code::POLICY), [0x88, 0x02, 0x03, 0xf0]);
    assert_eq!(close(Code::GOING_AWAY), [0x88, 0x02, 0x03, 0xe9]);
    // a pong never says more than a control frame may
    assert_eq!(pong(&[1; 200]).len(), 2 + 125);
}
