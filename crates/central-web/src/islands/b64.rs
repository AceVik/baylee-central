//! Base64 for the two places the chat needs it: the signed header of a
//! sent message (base64url without padding, as on Central's wire) and the
//! peer trust kept in localStorage (standard alphabet with padding, as the
//! previous site wrote it). An encoding, not cryptography.

const STANDARD: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn value(byte: u8, url: bool) -> Option<u32> {
    let value = match byte {
        b'A'..=b'Z' => byte - b'A',
        b'a'..=b'z' => byte - b'a' + 26,
        b'0'..=b'9' => byte - b'0' + 52,
        b'+' if !url => 62,
        b'/' if !url => 63,
        b'-' if url => 62,
        b'_' if url => 63,
        _ => return None,
    };
    Some(u32::from(value))
}

fn decode(text: &str, url: bool) -> Option<Vec<u8>> {
    let body = if url {
        text
    } else {
        // padding to a whole group, as btoa writes it
        if !text.len().is_multiple_of(4) {
            return None;
        }
        text.trim_end_matches('=')
    };
    if text.len() - body.len() > 2 || body.len() % 4 == 1 {
        return None;
    }
    let mut out = Vec::with_capacity(body.len() / 4 * 3 + 2);
    for chunk in body.as_bytes().chunks(4) {
        let mut group = 0u32;
        for (i, &byte) in chunk.iter().enumerate() {
            group |= value(byte, url)? << (18 - 6 * i);
        }
        let [_, a, b, c] = group.to_be_bytes();
        out.push(a);
        if chunk.len() > 2 {
            out.push(b);
        }
        if chunk.len() > 3 {
            out.push(c);
        }
        // canonical: the bits after the last whole byte are zero
        let unused = match chunk.len() {
            2 => group & 0xffff,
            3 => group & 0xff,
            _ => 0,
        };
        if unused != 0 {
            return None;
        }
    }
    Some(out)
}

/// Standard base64 with padding, as `btoa` writes it.
#[must_use]
pub fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let group = chunk
            .iter()
            .enumerate()
            .fold(0u32, |acc, (i, &b)| acc | (u32::from(b) << (16 - 8 * i)));
        for i in 0..4 {
            if i <= chunk.len() {
                let index = usize::try_from((group >> (18 - 6 * i)) & 63).unwrap_or(0);
                out.push(char::from(STANDARD[index]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Standard base64 with padding; `None` for anything else.
#[must_use]
pub fn decode_standard(text: &str) -> Option<Vec<u8>> {
    decode(text, false)
}

/// Unpadded base64url; `None` for anything else.
#[must_use]
pub fn decode_url(text: &str) -> Option<Vec<u8>> {
    decode(text, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        for bytes in [&b""[..], b"f", b"fo", b"foo", b"foob", b"fooba", b"foobar"] {
            assert_eq!(decode_standard(&encode(bytes)).as_deref(), Some(bytes));
        }
        assert_eq!(encode(b"foobar"), "Zm9vYmFy");
        assert_eq!(encode(b"fo"), "Zm8=");
        assert_eq!(encode(&[0xfb, 0xff]), "+/8=");
    }

    #[test]
    fn url_alphabet_without_padding() {
        assert_eq!(decode_url("-_8").as_deref(), Some(&[0xfb, 0xff][..]));
        assert_eq!(decode_url("Zm8"), Some(b"fo".to_vec()));
        assert!(decode_url("Zm8=").is_none());
        assert!(decode_url("+/8").is_none());
    }

    #[test]
    fn refuses_what_is_not_canonical() {
        assert!(decode_standard("Zm8").is_none(), "missing padding");
        assert!(decode_standard("Zm9=").is_none(), "stray bits");
        assert!(decode_standard("Z===").is_none());
        assert!(decode_standard("Zm8*").is_none());
        assert!(decode_url("Z").is_none());
    }
}
