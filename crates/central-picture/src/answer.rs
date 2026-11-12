//! What the service checks of the worker's answer before it stores it. A
//! worker that an exploit took over still chooses the pixels, so these
//! checks bound the structure, not the content: the PNG master is exactly
//! IHDR, IDAT and IEND at 256 × 256, 8 bits, RGB or RGBA, not interlaced,
//! with every CRC right; the AVIF is exactly the boxes `ftyp` (brand
//! `avif`), `meta` and `mdat`. The digest that reports and decisions key on
//! is computed here, never taken from the worker.
use super::{MAX_AVIF, MAX_PNG, SIDE};
use sha2::{Digest, Sha256};

/// The SHA-256 of a PNG master: what identifies a picture to reports.
#[must_use]
pub fn digest(png: &[u8]) -> [u8; 32] {
    Sha256::digest(png).into()
}

/// A PNG master as the worker must write it.
#[must_use]
pub fn png_master(bytes: &[u8]) -> bool {
    if bytes.len() > MAX_PNG {
        return false;
    }
    let Some(mut rest) = bytes.strip_prefix(b"\x89PNG\r\n\x1a\n") else {
        return false;
    };
    let mut index = 0_usize;
    let mut ended = false;
    while !rest.is_empty() {
        if ended {
            return false;
        }
        let Some((kind, data, after)) = chunk(rest) else {
            return false;
        };
        // IHDR first, then IDAT, at least one, then IEND.
        let fits = match (index, kind) {
            (0, b"IHDR") => ihdr(data),
            (1.., b"IDAT") => !data.is_empty(),
            (2.., b"IEND") => {
                ended = true;
                data.is_empty()
            }
            _ => false,
        };
        if !fits {
            return false;
        }
        index += 1;
        rest = after;
    }
    ended
}

/// IHDR of the master: 256 × 256, 8 bits, RGB (2) or RGBA (6), compression
/// and filter 0, not interlaced.
fn ihdr(data: &[u8]) -> bool {
    let side = SIDE.to_be_bytes();
    data.len() == 13
        && data[..4] == side
        && data[4..8] == side
        && data[8] == 8
        && matches!(data[9], 2 | 6)
        && data[10..13] == [0, 0, 0]
}

/// One chunk: its type, its data and what follows, with its CRC checked.
fn chunk(bytes: &[u8]) -> Option<(&[u8; 4], &[u8], &[u8])> {
    let length = usize::try_from(u32::from_be_bytes(bytes.get(..4)?.try_into().ok()?)).ok()?;
    let kind: &[u8; 4] = bytes.get(4..8)?.try_into().ok()?;
    let data = bytes.get(8..8usize.checked_add(length)?)?;
    let end = 12usize.checked_add(length)?;
    let crc = u32::from_be_bytes(bytes.get(end - 4..end)?.try_into().ok()?);
    (crc32(&bytes[4..end - 4]) == crc).then(|| (kind, data, &bytes[end..]))
}

/// CRC-32 as PNG uses it (ISO 3309, reflected, polynomial 0xEDB88320). A
/// checksum, not cryptography.
fn crc32(bytes: &[u8]) -> u32 {
    const TABLE: [u32; 256] = {
        let mut table = [0_u32; 256];
        let mut n: u32 = 0;
        while n < 256 {
            let mut c = n;
            let mut k = 0;
            while k < 8 {
                c = if c & 1 == 1 {
                    0xedb8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                };
                k += 1;
            }
            table[n as usize] = c;
            n += 1;
        }
        table
    };
    !bytes.iter().fold(u32::MAX, |crc, &byte| {
        TABLE[((crc ^ u32::from(byte)) & 0xff) as usize] ^ (crc >> 8)
    })
}

/// An AVIF as the worker must write it: exactly `ftyp` with the brand
/// `avif`, `meta` and `mdat`, in that order, filling the file.
#[must_use]
pub fn avif_framing(bytes: &[u8]) -> bool {
    if bytes.len() > MAX_AVIF {
        return false;
    }
    let mut rest = bytes;
    for expected in [b"ftyp", b"meta", b"mdat"] {
        let Some(size) = rest
            .get(..4)
            .and_then(|b| b.try_into().ok())
            .map(u32::from_be_bytes)
            .and_then(|size| usize::try_from(size).ok())
        else {
            return false;
        };
        if size < 8 || size > rest.len() || &rest[4..8] != expected {
            return false;
        }
        if expected == b"ftyp" && rest.get(8..12) != Some(b"avif") {
            return false;
        }
        rest = &rest[size..];
    }
    rest.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_chunk(kind: &[u8], data: &[u8]) -> Vec<u8> {
        let mut out = u32::try_from(data.len()).unwrap().to_be_bytes().to_vec();
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        let crc = crc32(&out[4..]);
        out.extend_from_slice(&crc.to_be_bytes());
        out
    }

    fn header(side: u32, depth: u8, colour: u8, interlace: u8) -> Vec<u8> {
        let mut data = side.to_be_bytes().to_vec();
        data.extend_from_slice(&side.to_be_bytes());
        data.extend_from_slice(&[depth, colour, 0, 0, interlace]);
        data
    }

    fn png(chunks: &[Vec<u8>]) -> Vec<u8> {
        let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
        for c in chunks {
            out.extend_from_slice(c);
        }
        out
    }

    #[test]
    fn crc_matches_the_standard_check_value() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn only_the_master_shape_passes() {
        let ihdr = raw_chunk(b"IHDR", &header(256, 8, 6, 0));
        let idat = raw_chunk(b"IDAT", b"x");
        let iend = raw_chunk(b"IEND", b"");
        assert!(png_master(&png(&[
            ihdr.clone(),
            idat.clone(),
            iend.clone()
        ])));
        assert!(png_master(&png(&[
            raw_chunk(b"IHDR", &header(256, 8, 2, 0)),
            idat.clone(),
            idat.clone(),
            iend.clone()
        ])));
        let refused = [
            png(&[ihdr.clone(), iend.clone()]),
            png(&[idat.clone(), iend.clone()]),
            png(&[ihdr.clone(), idat.clone()]),
            png(&[ihdr.clone(), idat.clone(), iend.clone(), iend.clone()]),
            png(&[ihdr.clone(), idat.clone(), iend.clone(), idat.clone()]),
            png(&[
                ihdr.clone(),
                raw_chunk(b"tEXt", b"a\0<script>"),
                idat.clone(),
                iend.clone(),
            ]),
            png(&[
                ihdr.clone(),
                raw_chunk(b"eXIf", b"MM"),
                idat.clone(),
                iend.clone(),
            ]),
            png(&[
                raw_chunk(b"IHDR", &header(512, 8, 6, 0)),
                idat.clone(),
                iend.clone(),
            ]),
            png(&[
                raw_chunk(b"IHDR", &header(256, 16, 6, 0)),
                idat.clone(),
                iend.clone(),
            ]),
            png(&[
                raw_chunk(b"IHDR", &header(256, 8, 3, 0)),
                idat.clone(),
                iend.clone(),
            ]),
            png(&[
                raw_chunk(b"IHDR", &header(256, 8, 6, 1)),
                idat.clone(),
                iend.clone(),
            ]),
            png(&[ihdr.clone(), raw_chunk(b"IDAT", b""), iend.clone()]),
            png(&[ihdr.clone(), idat.clone(), raw_chunk(b"IEND", b"x")]),
            [
                png(&[ihdr.clone(), idat.clone(), iend.clone()]),
                b"<html>".to_vec(),
            ]
            .concat(),
            b"GIF89a".to_vec(),
        ];
        for (n, bytes) in refused.iter().enumerate() {
            assert!(!png_master(bytes), "case {n}");
        }
        // one bit anywhere in a chunk breaks its CRC
        let good = png(&[ihdr, idat, iend]);
        for at in 8..good.len() {
            let mut bad = good.clone();
            bad[at] ^= 1;
            assert!(!png_master(&bad), "bit at {at}");
        }
        assert!(!png_master(&vec![0; MAX_PNG + 1]));
    }

    fn boxed(kind: &[u8], data: &[u8]) -> Vec<u8> {
        let mut out = u32::try_from(data.len() + 8)
            .unwrap()
            .to_be_bytes()
            .to_vec();
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        out
    }

    #[test]
    fn only_the_three_avif_boxes_pass() {
        let ftyp = boxed(b"ftyp", b"avif\0\0\0\0avifmif1");
        let meta = boxed(b"meta", &[0; 20]);
        let mdat = boxed(b"mdat", &[1; 30]);
        assert!(avif_framing(
            &[ftyp.clone(), meta.clone(), mdat.clone()].concat()
        ));
        for bad in [
            [meta.clone(), ftyp.clone(), mdat.clone()].concat(),
            [ftyp.clone(), meta.clone()].concat(),
            [
                ftyp.clone(),
                meta.clone(),
                mdat.clone(),
                boxed(b"free", b""),
            ]
            .concat(),
            [
                ftyp.clone(),
                boxed(b"Exif", b""),
                meta.clone(),
                mdat.clone(),
            ]
            .concat(),
            [boxed(b"ftyp", b"heic\0\0\0\0"), meta.clone(), mdat.clone()].concat(),
            [ftyp.clone(), meta.clone(), mdat[..mdat.len() - 1].to_vec()].concat(),
            [ftyp, meta, mdat, b"x".to_vec()].concat(),
        ] {
            assert!(!avif_framing(&bad));
        }
    }

    #[test]
    fn the_digest_is_sha256() {
        assert_eq!(
            digest(b"abc")[..4],
            [0xba, 0x78, 0x16, 0xbf],
            "the FIPS 180-2 test vector"
        );
    }
}
