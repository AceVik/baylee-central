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
