use super::{
    first_ticket,
    frames::{From, Header, forward, mask},
};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _, duplex};

fn frame(first: u8, body: &[u8], key: Option<[u8; 4]>) -> Vec<u8> {
    let n = u8::try_from(body.len()).unwrap();
    assert!(n <= 125);
    let mut out = vec![first, n | if key.is_some() { 128 } else { 0 }];
    if let Some(key) = key {
        out.extend(key);
        let mut bytes = body.to_owned();
        mask(&mut bytes, key, 0);
        out.extend(bytes);
    } else {
        out.extend(body);
    }
    out
}

#[tokio::test]
async fn only_one_complete_masked_lowercase_ticket_authenticates() {
    let key = Some([4, 3, 2, 1]);
    let body = [b'b'; 64];
    let ticket = frame(0x82, &body, key);
    assert_eq!(*first_ticket(&mut &ticket[..]).await.unwrap(), body);
    for bad in [
        frame(0x81, &body, key),
        frame(0x02, &body, key),
        frame(0x82, &body, None),
        frame(0x82, &body[..63], key),
        frame(0x82, &[b'F'; 64], key),
    ] {
        assert!(first_ticket(&mut &bad[..]).await.is_err());
    }
}

#[tokio::test]
async fn preserves_fragmented_data_and_interleaved_controls_in_both_directions() {
    for from in [From::Native, From::Gateway] {
        let key = (from == From::Native).then_some([1, 2, 3, 4]);
        let data = [(0x02, "first"), (0x89, "ping"), (0x80, "second")];
        let mut input = Vec::new();
        for (op, body) in data {
            input.extend(frame(op, body.as_bytes(), key));
        }
        input.extend(frame(0x88, &[], key));
        let (write, mut read) = duplex(512);
        forward(&input[..], write, from).await.unwrap();
        for (op, body) in data {
            let head = Header::read(&mut read, from).await.unwrap();
            assert_eq!(head.opcode, op & 15);
            assert_eq!(head.fin, op & 128 != 0);
            let mut bytes = vec![0; head.length];
            read.read_exact(&mut bytes).await.unwrap();
            mask(&mut bytes, head.mask, 0);
            assert_eq!(bytes, body.as_bytes());
        }
    }
}

#[tokio::test]
async fn frame_roles_sequence_and_fragment_total_are_bounded_before_copy() {
    let key = Some([1, 2, 3, 4]);
    let mut input = frame(0x02, b"abc", key);
    input.extend([0x80, 0xff]);
    input.extend(65536_u64.to_be_bytes());
    input.extend([0; 4]);
    for bad in [
        input,
        frame(0x80, b"unexpected continuation", key),
        frame(0x82, b"wrong mask", None),
        vec![0x82, 0xfe, 0, 1],
        vec![0x09, 0x80],
    ] {
        assert_eq!(
            forward(&bad[..], tokio::io::sink(), From::Native)
                .await
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::InvalidData
        );
    }
    let (mut socket, read) = duplex(128);
    socket.write_all(&[0x82, 0x7f]).await.unwrap();
    socket
        .write_all(&(16 * 1024 * 1024 + 1u64).to_be_bytes())
        .await
        .unwrap();
    assert!(
        forward(read, tokio::io::sink(), From::Gateway)
            .await
            .is_err()
    );
}

const KEY: [u8; 4] = [0x5a, 0x01, 0xff, 0x80];

/// A frame of any length with the shortest length code, masked as `from` must.
fn sized(first: u8, body: &[u8], from: From) -> Vec<u8> {
    let masked = from == From::Native;
    let bit = if masked { 0x80 } else { 0 };
    let mut out = vec![first];
    match body.len() {
        n @ 0..=125 => out.push(bit | u8::try_from(n).unwrap()),
        n @ 126..=0xffff => {
            out.push(bit | 0x7e);
            out.extend(u16::try_from(n).unwrap().to_be_bytes());
        }
        n => {
            out.push(bit | 0x7f);
            out.extend((n as u64).to_be_bytes());
        }
    }
    if masked {
        out.extend(KEY);
    }
    let start = out.len();
    out.extend(body);
    if masked {
        mask(&mut out[start..], KEY, 0);
    }
    out
}

async fn refused(input: &[u8], from: From) -> bool {
    forward(input, tokio::io::sink(), from)
        .await
        .is_err_and(|error| error.kind() == std::io::ErrorKind::InvalidData)
}

#[tokio::test]
async fn a_header_is_read_only_in_its_one_canonical_form() {
    // First octet: FIN, no reserved bit, and a binary, continuation or control opcode.
    for first in 0..=u8::MAX {
        let opcode = first & 15;
        let legal = first & 0x70 == 0
            && matches!(opcode, 0 | 2 | 8 | 9 | 10)
            && (opcode < 8 || first & 0x80 != 0);
        let input = sized(first, b"", From::Native);
        assert_eq!(
            Header::read(&mut &input[..], From::Native).await.is_ok(),
            legal,
            "{first:#x}"
        );
    }
    // The other role's masking is never accepted, in either direction.
    let native = sized(0x82, b"x", From::Native);
    let gateway = sized(0x82, b"x", From::Gateway);
    assert!(Header::read(&mut &native[..], From::Gateway).await.is_err());
    assert!(Header::read(&mut &gateway[..], From::Native).await.is_err());
}

#[tokio::test]
async fn a_length_has_its_shortest_code_and_a_limit_by_direction() {
    let long = |opcode: u8, marker: u8, length: &[u8], masked: bool| {
        let mut out = vec![opcode, marker | if masked { 0x80 } else { 0 }];
        out.extend(length);
        if masked {
            out.extend(KEY);
        }
        out
    };
    for (marker, length, native, gateway) in [
        (126, &125_u16.to_be_bytes()[..], false, false),
        (126, &126_u16.to_be_bytes()[..], true, true),
        (126, &u16::MAX.to_be_bytes()[..], true, true),
        (127, &u64::from(u16::MAX).to_be_bytes()[..], false, false),
        (127, &(64 * 1024_u64).to_be_bytes()[..], true, true),
        (127, &(64 * 1024 + 1_u64).to_be_bytes()[..], false, true),
        (127, &(16 * 1024 * 1024_u64).to_be_bytes()[..], false, true),
        (
            127,
            &(16 * 1024 * 1024 + 1_u64).to_be_bytes()[..],
            false,
            false,
        ),
        (127, &(1_u64 << 63).to_be_bytes()[..], false, false),
        (127, &u64::MAX.to_be_bytes()[..], false, false),
    ] {
        for (from, accepted) in [(From::Native, native), (From::Gateway, gateway)] {
            let masked = from == From::Native;
            let input = long(0x82, marker, length, masked);
            let read = Header::read(&mut &input[..], from).await;
            assert_eq!(read.is_ok(), accepted, "{marker} {length:x?}");
            // A control frame never carries an extended length.
            let ping = long(0x89, marker, length, masked);
            assert!(Header::read(&mut &ping[..], from).await.is_err());
        }
    }
    let most = sized(0x89, &[7; 125], From::Native);
    assert_eq!(
        Header::read(&mut &most[..], From::Native)
            .await
            .unwrap()
            .length,
        125
    );
}

#[tokio::test]
async fn every_length_code_is_forwarded_whole_under_a_mask_of_central_s_own() {
    for from in [From::Native, From::Gateway] {
        for length in [0, 1, 125, 126, 8191, 8192, 8193, 0xffff, 0x1_0000] {
            let body: Vec<u8> = (0..length)
                .map(|i| u8::try_from(i % 251).unwrap())
                .collect();
            let mut input = sized(0x82, &body, from);
            input.extend(sized(0x88, b"", from));
            let mut output = Vec::new();
            forward(&input[..], &mut output, from).await.unwrap();
            let mut rest = &output[..];
            let head = Header::read(&mut rest, from).await.unwrap();
            assert_eq!((head.fin, head.opcode, head.length), (true, 2, length));
            let mut body_out = rest.to_vec();
            mask(&mut body_out, head.mask, 0);
            assert!(body_out == body, "{length} bytes changed on the way");
        }
    }
    let mut keys = std::collections::BTreeSet::new();
    let frames: Vec<u8> = (0..8)
        .flat_map(|_| sized(0x82, b"same", From::Native))
        .chain(sized(0x88, b"", From::Native))
        .collect();
    let mut output = Vec::new();
    forward(&frames[..], &mut output, From::Native)
        .await
        .unwrap();
    let mut rest = &output[..];
    for _ in 0..8 {
        let head = Header::read(&mut rest, From::Native).await.unwrap();
        keys.insert(head.mask);
        rest = &rest[head.length..];
    }
    assert!(!keys.contains(&KEY), "the native mask reached the gateway");
    assert!(keys.len() > 1, "one mask for every frame");
}

#[tokio::test]
async fn a_close_ends_the_relay_and_neither_its_reason_nor_what_follows_crosses() {
    for from in [From::Native, From::Gateway] {
        let mut input = sized(0x88, b"\x03\xe8operator text", from);
        input.extend(sized(0x82, b"after the close", from));
        let mut output = Vec::new();
        forward(&input[..], &mut output, from).await.unwrap();
        assert!(output.is_empty(), "{output:x?}");
    }
    // In the middle of a fragmented message too.
    let mut input = sized(0x02, b"half", From::Gateway);
    input.extend(sized(0x88, b"", From::Gateway));
    let mut output = Vec::new();
    forward(&input[..], &mut output, From::Gateway)
        .await
        .unwrap();
    assert_eq!(output, sized(0x02, b"half", From::Gateway));
}

#[tokio::test]
async fn a_truncated_frame_is_an_error_and_never_a_clean_end() {
    let whole = sized(0x82, &[9; 200], From::Native);
    for cut in 0..whole.len() {
        assert!(
            forward(&whole[..cut], tokio::io::sink(), From::Native)
                .await
                .is_err(),
            "cut after {cut}"
        );
    }
}

#[tokio::test]
async fn fragments_are_one_message_to_the_limit() {
    let close = sized(0x88, b"", From::Native);
    let message = |lengths: &[usize]| {
        let mut out = Vec::new();
        for (index, &length) in lengths.iter().enumerate() {
            let opcode = if index == 0 { 2 } else { 0 };
            let fin = if index + 1 == lengths.len() { 0x80 } else { 0 };
            out.extend(sized(fin | opcode, &vec![1; length], From::Native));
        }
        out.extend(&close);
        out
    };
    let limit = 64 * 1024;
    assert!(!refused(&message(&[limit / 2, limit / 2]), From::Native).await);
    assert!(refused(&message(&[limit / 2, limit / 2, 1]), From::Native).await);
    assert!(refused(&message(&[limit, 1]), From::Native).await);
    // Empty fragments cost a frame each: 128 are a message, 129 are not.
    assert!(!refused(&message(&[0; 128]), From::Native).await);
    assert!(refused(&message(&[0; 129]), From::Native).await);
    // A finished message frees the count for the next one.
    let mut two = message(&[0; 128]);
    two.truncate(two.len() - close.len());
    two.extend(message(&[0; 128]));
    assert!(!refused(&two, From::Native).await);
}

#[tokio::test]
async fn a_message_is_never_opened_inside_another() {
    for from in [From::Native, From::Gateway] {
        for (first, second) in [(0x02, 0x82), (0x02, 0x02), (0x82, 0x80), (0x82, 0x00)] {
            let mut input = sized(first, b"a", from);
            input.extend(sized(second, b"b", from));
            assert!(refused(&input, from).await, "{first:#x} {second:#x}");
        }
    }
}

#[tokio::test]
async fn a_flood_of_frames_or_bytes_ends_the_relay() {
    let close = sized(0x88, b"", From::Native);
    let flood = |count: usize, body: &[u8]| {
        let mut out: Vec<u8> = sized(0x82, body, From::Native).repeat(count);
        out.extend(&close);
        out
    };
    // The closing frame counts as one of the 512 a second.
    assert!(!refused(&flood(511, b""), From::Native).await);
    assert!(refused(&flood(512, b""), From::Native).await);
    let most = vec![1; 64 * 1024];
    assert!(!refused(&flood(8, &most), From::Native).await);
    let mut over = flood(8, &most);
    over.truncate(over.len() - close.len());
    over.extend(flood(1, b"x"));
    assert!(refused(&over, From::Native).await);
}

#[tokio::test]
async fn a_ticket_is_one_whole_binary_message_of_64_hex_digits() {
    let good = sized(0x82, &[b'0'; 64], From::Native);
    assert!(first_ticket(&mut &good[..]).await.is_ok());
    for (first, body) in [
        (0x82, vec![b'a'; 65]),
        (0x82, vec![b'g'; 64]),
        (0x82, vec![b'/'; 64]),
        (0x82, vec![b':'; 64]),
        (0x82, vec![b'`'; 64]),
        (0x82, vec![0; 64]),
        (0x82, vec![]),
        (0x80, vec![b'a'; 64]),
        (0x89, vec![b'a'; 64]),
        (0x88, vec![b'a'; 64]),
    ] {
        let input = sized(first, &body, From::Native);
        assert!(
            first_ticket(&mut &input[..]).await.is_err(),
            "{first:#x} {body:x?}"
        );
    }
    for cut in 0..good.len() {
        assert!(first_ticket(&mut &good[..cut]).await.is_err(), "{cut}");
    }
    // One wrong digit at either end is enough.
    for index in [0, 63] {
        let mut body = [b'0'; 64];
        body[index] = b'A';
        let input = sized(0x82, &body, From::Native);
        assert!(first_ticket(&mut &input[..]).await.is_err());
    }
}

/// After the ticket: a grant Central issued for a person's session keeps
/// the socket on its address; one for an AI runner's session moves it to
/// the runners' budget, which takes one relay a seat, the newer in the
/// older's place, and a ticket for the account's seventeenth seat closes.
#[tokio::test]
async fn a_ticket_decides_the_budget_and_only_central_s_grant_names_a_runner() {
    use super::{Grant, budget, ws};
    let home = crate::in_flight::Client::from(std::net::IpAddr::from([198, 51, 100, 7]));
    let sockets = ws::Sockets::new();
    let upgrades = &sockets.upgrades;
    let mut person = upgrades.admitted(home).unwrap();
    assert!(budget(&Grant::synthetic(None, [1; 16], 0), &mut person).is_some());
    assert_eq!(
        upgrades.counted(home),
        1,
        "a person's relay stays on its address"
    );
    let ai = [9; 16];
    let mut relays = Vec::new();
    for game in 0..16 {
        let mut relay = upgrades.admitted(home).unwrap();
        let replaced = budget(&Grant::synthetic(Some(ai), [game; 16], 1), &mut relay).unwrap();
        relays.push((relay, replaced));
    }
    assert_eq!(upgrades.counted(home), 1, "the runner's sixteen left it");
    let mut over = upgrades.admitted(home).unwrap();
    assert!(budget(&Grant::synthetic(Some(ai), [16; 16], 1), &mut over).is_none());
    let mut again = upgrades.admitted(home).unwrap();
    let _kept = budget(&Grant::synthetic(Some(ai), [0; 16], 1), &mut again).unwrap();
    let (_old, old_replaced) = relays.remove(0);
    tokio::time::timeout(std::time::Duration::from_secs(1), old_replaced.wait())
        .await
        .expect("the seat's older relay is told to close");
}
