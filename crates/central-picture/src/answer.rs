//! What the service checks of the worker's answer before it stores it. A
//! worker that an exploit took over still chooses the pixels, so these
//! checks bound the structure, not the content: the PNG master is exactly
//! IHDR, IDAT and IEND at 256 × 256, 8 bits, RGB or RGBA, not interlaced,
//! with every CRC right; the AVIF is exactly the boxes `ftyp` (brand
//! `avif`), `meta` and `mdat`. The digest that reports and decisions key on
//! is computed here, never taken from the worker.
use super::{MAX_AVIF, MAX_PNG, SIDE};
use sha2::{Digest, Sha256};

/// What identifies a stored picture to reports: the SHA-256 of its PNG
/// master, followed by its image sequence when it moves, so a report names
/// the animation that was shown and not only its first frame.
#[must_use]
pub fn digest(png: &[u8], sequence: Option<&[u8]>) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(png);
    if let Some(sequence) = sequence {
        hasher.update(sequence);
    }
    hasher.finalize().into()
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
        let body = &rest[8..size];
        if (expected == b"meta" && !items(body)) || (expected == b"mdat" && !bitstream(body)) {
            return false;
        }
        rest = &rest[size..];
    }
    rest.is_empty()
}

/// A `meta` box's body as the worker's encoders write it: a handler, the
/// primary item, its location, the items and their properties, and for
/// transparency a reference. Exactly one AV1 item, or one and its alpha
/// auxiliary; every image 256 × 256; a colour given as `nclx` only. No
/// Exif or XMP item, no ICC profile, nothing else.
pub(crate) fn items(meta: &[u8]) -> bool {
    use crate::sequence::boxes;
    let Some(children) = meta.get(4..).and_then(boxes) else {
        return false;
    };
    let (mut count, mut referenced, mut auxiliary) = (0, false, false);
    for (kind, body) in &children {
        match *kind {
            b"hdlr" | b"pitm" | b"iloc" => {}
            b"iref" => referenced = true,
            b"iinf" => match item_types(body) {
                Some(types)
                    if (1..=2).contains(&types.len()) && types.iter().all(|t| t == b"av01") =>
                {
                    count = types.len();
                }
                _ => return false,
            },
            b"iprp" => match properties(body) {
                Some(alpha) => auxiliary = alpha,
                None => return false,
            },
            _ => return false,
        }
    }
    count == 1 || (count == 2 && referenced && auxiliary)
}

/// The item types an `iinf` lists.
fn item_types(iinf: &[u8]) -> Option<Vec<[u8; 4]>> {
    let at = if *iinf.first()? == 0 { 6 } else { 8 };
    crate::sequence::boxes(iinf.get(at..)?)?
        .iter()
        .map(|(kind, infe)| {
            let at = match (*kind, infe.first()?) {
                (b"infe", &2) => 8,
                (b"infe", &3) => 10,
                _ => return None,
            };
            infe.get(at..at + 4)?.try_into().ok()
        })
        .collect()
}

/// Whether an `iprp`'s properties are only what the encoders write, every
/// size 256 × 256 and every colour `nclx`; and whether one says alpha.
fn properties(iprp: &[u8]) -> Option<bool> {
    let mut alpha = false;
    for (kind, body) in crate::sequence::boxes(iprp)? {
        match kind {
            b"ipma" => {}
            b"ipco" => {
                for (property, value) in crate::sequence::boxes(body)? {
                    match property {
                        b"ispe" if value.get(4..12) == Some(&[0, 0, 1, 0, 0, 0, 1, 0][..]) => {}
                        b"colr" if value.get(..4) == Some(b"nclx".as_slice()) => {}
                        b"auxC" => alpha = true,
                        b"pixi" | b"av1C" => {}
                        _ => return None,
                    }
                }
            }
            _ => return None,
        }
    }
    Some(alpha)
}

/// An `mdat`'s AV1: every sequence header allows frames of at most 256 px a
/// side, and there is one.
pub(crate) fn bitstream(mdat: &[u8]) -> bool {
    crate::av1::largest(mdat).is_some_and(|(width, height)| width <= 256 && height <= 256)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn full(kind: &[u8], body: &[u8]) -> Vec<u8> {
        boxed(kind, &[&[0, 0, 0, 0][..], body].concat())
    }

    /// `infe` entries (version 2) of these item types.
    pub(crate) fn still_items(alpha: bool) -> Vec<u8> {
        let mut types: Vec<&[u8; 4]> = vec![b"av01"];
        if alpha {
            types.push(b"av01");
        }
        let entries: Vec<u8> = types
            .iter()
            .zip(1_u16..)
            .flat_map(|(kind, id)| {
                let body = [
                    &[2, 0, 0, 0][..],
                    &id.to_be_bytes(),
                    &[0, 0],
                    &kind[..],
                    b"\0",
                ]
                .concat();
                boxed(b"infe", &body)
            })
            .collect();
        full(
            b"iinf",
            &[
                &u16::try_from(types.len()).unwrap().to_be_bytes()[..],
                &entries,
            ]
            .concat(),
        )
    }

    /// An `iprp` with an `ispe` of `side`, a `colr` of `colour`'s type, and
    /// for alpha an `auxC`.
    pub(crate) fn properties_of(side: u32, colour: &[u8], alpha: bool) -> Vec<u8> {
        let mut ipco = full(b"ispe", &[side.to_be_bytes(), side.to_be_bytes()].concat());
        ipco.extend(boxed(
            b"colr",
            &[colour, &[0, 1, 0, 13, 0, 6, 0x80]].concat(),
        ));
        if alpha {
            ipco.extend(full(
                b"auxC",
                b"urn:mpeg:mpegB:cicp:systems:auxiliary:alpha\0",
            ));
        }
        [boxed(b"ipco", &ipco), full(b"ipma", &[0, 0, 0, 0])].concat()
    }

    /// A `meta` box's body: a handler, a primary item, a location, `items`,
    /// `properties` and, for alpha, a reference.
    pub(crate) fn meta_body(items: &[u8], properties: &[u8], reference: bool) -> Vec<u8> {
        let mut body = vec![0, 0, 0, 0];
        body.extend(full(b"hdlr", b"\0\0\0\0pict\0\0\0\0\0\0\0\0\0\0\0\0\0"));
        body.extend(full(b"pitm", &[0, 1]));
        body.extend(full(b"iloc", &[0x44, 0, 0, 0]));
        body.extend_from_slice(items);
        body.extend(boxed(b"iprp", properties));
        if reference {
            body.extend(full(b"iref", b"\0\0\0\x0eauxl\0\x02\0\x01\0\x01"));
        }
        body
    }

    #[test]
    fn a_still_s_items_properties_and_bitstream_are_only_what_the_encoders_write() {
        let ftyp = boxed(b"ftyp", b"avif\0\0\0\0avifmif1");
        let file = |meta: Vec<u8>, mdat: Vec<u8>| {
            [ftyp.clone(), boxed(b"meta", &meta), boxed(b"mdat", &mdat)].concat()
        };
        let av1 = crate::av1::tests::sequence(8, 8);
        let good = meta_body(
            &still_items(false),
            &properties_of(256, b"nclx", false),
            false,
        );
        assert!(avif_framing(&file(good.clone(), av1.clone())));
        let alpha = meta_body(&still_items(true), &properties_of(256, b"nclx", true), true);
        assert!(avif_framing(&file(alpha, av1.clone())));
        let exif_item = {
            let mut items = still_items(true);
            let at = items.windows(4).rposition(|w| w == b"av01").unwrap();
            items[at..at + 4].copy_from_slice(b"Exif");
            items
        };
        let bad = [
            // a size other than 256
            meta_body(
                &still_items(false),
                &properties_of(512, b"nclx", false),
                false,
            ),
            // an ICC profile
            meta_body(
                &still_items(false),
                &properties_of(256, b"prof", false),
                false,
            ),
            // an Exif item beside the image
            meta_body(&exif_item, &properties_of(256, b"nclx", false), false),
            // two items without an alpha auxiliary
            meta_body(
                &still_items(true),
                &properties_of(256, b"nclx", false),
                false,
            ),
            // XMP as a box of its own
            [good.clone(), boxed(b"xml ", b"<x:xmpmeta/>")].concat(),
        ];
        for meta in bad {
            assert!(!avif_framing(&file(meta, av1.clone())));
        }
        // an AV1 sequence header that allows frames larger than 256 px
        assert!(!avif_framing(&file(
            good.clone(),
            crate::av1::tests::sequence(9, 8)
        )));
        // no sequence header at all
        assert!(!avif_framing(&file(good, vec![0x12, 0x00])));
    }

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
        let meta = boxed(
            b"meta",
            &meta_body(
                &still_items(false),
                &properties_of(256, b"nclx", false),
                false,
            ),
        );
        let mdat = boxed(b"mdat", &crate::av1::tests::sequence(8, 8));
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
    fn the_digest_is_sha256_of_the_master_and_the_sequence() {
        assert_eq!(
            digest(b"abc", None)[..4],
            [0xba, 0x78, 0x16, 0xbf],
            "the FIPS 180-2 test vector"
        );
        // the sequence follows the master
        assert_eq!(digest(b"ab", Some(b"c")), digest(b"abc", None));
        assert_ne!(digest(b"abc", Some(b"d")), digest(b"abc", None));
    }
}
