//! Header readers of our own: each format's dimensions, read before any
//! decoder runs, so a picture that claims too many pixels is refused before
//! a decoder allocates for it. The decoders check again with their own
//! limits; these readers only look and never trust a length past the file.
use super::{Format, Refusal};

/// The width and height `bytes` claim, as their own header says.
///
/// # Errors
/// `Malformed` when the header is not there or not whole.
pub(crate) fn dimensions(format: Format, bytes: &[u8]) -> Result<(u32, u32), Refusal> {
    match format {
        Format::Png => png(bytes),
        Format::Jpeg => jpeg(bytes),
        Format::Gif => gif(bytes),
        Format::Webp => webp(bytes),
        Format::Avif => avif(bytes),
        Format::Bmp => bmp(bytes.get(14..).ok_or(Refusal::Malformed)?),
        Format::Ico => ico(bytes),
        Format::Svg => None,
    }
    .ok_or(Refusal::Malformed)
}

fn be32(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn le32(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn le16(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}

fn be16(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}

/// IHDR, which must be the first chunk and 13 bytes long.
fn png(bytes: &[u8]) -> Option<(u32, u32)> {
    (be32(bytes, 8)? == 13 && bytes.get(12..16)? == b"IHDR")
        .then(|| Some((be32(bytes, 16)?, be32(bytes, 20)?)))?
}

/// The first start-of-frame marker, found by walking the segments.
fn jpeg(bytes: &[u8]) -> Option<(u32, u32)> {
    let mut at = 2;
    loop {
        // Fill bytes (0xFF) may precede a marker.
        while *bytes.get(at)? == 0xff && *bytes.get(at + 1)? == 0xff {
            at += 1;
        }
        if *bytes.get(at)? != 0xff {
            return None;
        }
        let marker = *bytes.get(at + 1)?;
        match marker {
            // Markers without a length.
            0x01 | 0xd0..=0xd7 => at += 2,
            // End of image or start of scan before any frame: no frame.
            0xd9 | 0xda => return None,
            0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf => {
                let height = be16(bytes, at + 5)?;
                let width = be16(bytes, at + 7)?;
                return Some((u32::from(width), u32::from(height)));
            }
            _ => {
                let length = usize::from(be16(bytes, at + 2)?);
                if length < 2 {
                    return None;
                }
                at = at.checked_add(2 + length)?;
            }
        }
    }
}

/// The logical screen; every frame is checked against it as decoded.
fn gif(bytes: &[u8]) -> Option<(u32, u32)> {
    Some((u32::from(le16(bytes, 6)?), u32::from(le16(bytes, 8)?)))
}

/// The first chunk: a lossy frame header, a lossless header or the
/// extended canvas.
fn webp(bytes: &[u8]) -> Option<(u32, u32)> {
    let kind = bytes.get(12..16)?;
    let data = bytes.get(20..)?;
    match kind {
        b"VP8 " => {
            // Frame tag (3 bytes), start code, then 14-bit sizes.
            if data.get(3..6)? != [0x9d, 0x01, 0x2a] {
                return None;
            }
            let width = le16(data, 6)? & 0x3fff;
            let height = le16(data, 8)? & 0x3fff;
            Some((u32::from(width), u32::from(height)))
        }
        b"VP8L" => {
            if *data.first()? != 0x2f {
                return None;
            }
            let bits = le32(data, 1)?;
            Some(((bits & 0x3fff) + 1, ((bits >> 14) & 0x3fff) + 1))
        }
        b"VP8X" => {
            let width = le32(&[data.get(4..7)?, &[0]].concat(), 0)? + 1;
            let height = le32(&[data.get(7..10)?, &[0]].concat(), 0)? + 1;
            Some((width, height))
        }
        _ => None,
    }
}

/// The largest `ispe` (image spatial extents) property under
/// `meta`/`iprp`/`ipco`: every item's claimed size is bounded, alpha
/// included. The AV1 data itself is bounded again in `av1.rs`.
fn avif(bytes: &[u8]) -> Option<(u32, u32)> {
    let meta = child(bytes, *b"meta")?.get(4..)?;
    let ipco = child(child(meta, *b"iprp")?, *b"ipco")?;
    let mut largest: Option<(u32, u32)> = None;
    for (kind, body) in boxes(ipco) {
        if kind == *b"ispe" {
            let size = (be32(body, 4)?, be32(body, 8)?);
            largest = Some(largest.map_or(size, |(w, h)| (w.max(size.0), h.max(size.1))));
        }
    }
    largest
}

/// The body of the first box of type `kind` in `bytes`.
fn child(bytes: &[u8], kind: [u8; 4]) -> Option<&[u8]> {
    boxes(bytes)
        .find(|(found, _)| *found == kind)
        .map(|(_, body)| body)
}

/// The boxes in `bytes`, each with its type and body, until one does not fit.
fn boxes(mut bytes: &[u8]) -> impl Iterator<Item = ([u8; 4], &[u8])> {
    std::iter::from_fn(move || {
        let size = be32(bytes, 0)?;
        let kind: [u8; 4] = bytes.get(4..8)?.try_into().ok()?;
        let (start, end) = match size {
            // To the end of the enclosing box.
            0 => (8, bytes.len()),
            1 => {
                let large = u64::from_be_bytes(bytes.get(8..16)?.try_into().ok()?);
                (16, usize::try_from(large).ok()?)
            }
            n => (8, usize::try_from(n).ok()?),
        };
        if end < start || end > bytes.len() {
            return None;
        }
        let body = &bytes[start..end];
        bytes = &bytes[end..];
        Some((kind, body))
    })
}

/// A DIB header: the old 12-byte core header's 16-bit sizes, or the 32-bit
/// signed ones of every later header (a negative height is top-down).
fn bmp(dib: &[u8]) -> Option<(u32, u32)> {
    match le32(dib, 0)? {
        12 => Some((u32::from(le16(dib, 4)?), u32::from(le16(dib, 6)?))),
        16..=124 => {
            let width = i32::from_le_bytes(dib.get(4..8)?.try_into().ok()?);
            let height = i32::from_le_bytes(dib.get(8..12)?.try_into().ok()?);
            Some((width.unsigned_abs(), height.unsigned_abs()))
        }
        _ => None,
    }
}

/// The largest entry of an icon: each entry's own data is read, a PNG's
/// IHDR or a DIB header (whose height counts the mask too), since the
/// directory's byte-sized sizes say nothing of either.
fn ico(bytes: &[u8]) -> Option<(u32, u32)> {
    let count = usize::from(le16(bytes, 4)?);
    let mut largest = (0, 0);
    for entry in 0..count {
        let record = 6 + 16 * entry;
        let size = usize::try_from(le32(bytes, record + 8)?).ok()?;
        let offset = usize::try_from(le32(bytes, record + 12)?).ok()?;
        let data = bytes.get(offset..offset.checked_add(size)?)?;
        let (width, height) = if data.starts_with(b"\x89PNG\r\n\x1a\n") {
            png(data)?
        } else {
            let (width, height) = bmp(data)?;
            (width, height / 2)
        };
        if u64::from(width) * u64::from(height) > u64::from(largest.0) * u64::from(largest.1) {
            largest = (width, height);
        }
    }
    (largest != (0, 0)).then_some(largest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_reads_ihdr_only_when_it_comes_first() {
        let mut file = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
        file.extend_from_slice(&30_000_u32.to_be_bytes());
        file.extend_from_slice(&20_u32.to_be_bytes());
        assert_eq!(png(&file), Some((30_000, 20)));
        let mut other = file.clone();
        other[12..16].copy_from_slice(b"tEXt");
        assert_eq!(png(&other), None);
        let mut long = file.clone();
        long[11] = 14;
        assert_eq!(png(&long), None);
        assert_eq!(png(&file[..20]), None);
    }

    #[test]
    fn jpeg_walks_to_its_frame_and_stops_at_a_scan() {
        let app0 = [0xff, 0xe0, 0, 4, 0, 0];
        let sof2 = [0xff, 0xc2, 0, 11, 8, 0x75, 0x30, 0x00, 0x20, 3];
        let file = [&[0xff, 0xd8][..], &app0, &[0xff, 0xff], &sof2].concat();
        assert_eq!(jpeg(&file), Some((32, 30_000)));
        let scan_first = [&[0xff, 0xd8][..], &[0xff, 0xda, 0, 2], &sof2].concat();
        assert_eq!(jpeg(&scan_first), None);
        // a segment length that runs past the end, or below its own size
        let past = [&[0xff, 0xd8][..], &[0xff, 0xe1, 0xff, 0xff]].concat();
        assert_eq!(jpeg(&past), None);
        let zero = [&[0xff, 0xd8][..], &[0xff, 0xe1, 0, 0], &sof2].concat();
        assert_eq!(jpeg(&zero), None);
        for end in 0..file.len() {
            assert!(
                jpeg(&file[..end]).is_none_or(|d| d == (32, 30_000)),
                "{end}"
            );
        }
    }

    #[test]
    fn webp_reads_each_kind_of_first_chunk() {
        let riff = |kind: &[u8; 4], data: &[u8]| {
            [b"RIFF\0\0\0\0WEBP".as_slice(), kind, &[0, 0, 0, 0], data].concat()
        };
        let lossy = riff(
            b"VP8 ",
            &[0, 0, 0, 0x9d, 0x01, 0x2a, 0x10, 0x00, 0x20, 0xc0],
        );
        assert_eq!(webp(&lossy), Some((16, 32)));
        let bits: u32 = 0x63 | (0x31 << 14);
        let lossless = riff(b"VP8L", &[[0x2f].as_slice(), &bits.to_le_bytes()].concat());
        assert_eq!(webp(&lossless), Some((100, 50)));
        let extended = riff(b"VP8X", &[0, 0, 0, 0, 0xff, 0xff, 0xff, 0x0f, 0, 0]);
        assert_eq!(webp(&extended), Some((16_777_216, 16)));
        assert_eq!(webp(&riff(b"ALPH", &[0; 10])), None);
        assert_eq!(webp(&riff(b"VP8 ", &[0, 0, 0, 1, 2, 3, 0, 0, 0, 0])), None);
    }

    #[test]
    fn avif_takes_the_largest_ispe_and_survives_broken_boxes() {
        let boxed = |kind: &[u8; 4], body: &[u8]| {
            [
                &u32::try_from(body.len() + 8).unwrap().to_be_bytes()[..],
                kind,
                body,
            ]
            .concat()
        };
        let ispe = |w: u32, h: u32| {
            boxed(
                b"ispe",
                &[&[0, 0, 0, 0][..], &w.to_be_bytes(), &h.to_be_bytes()].concat(),
            )
        };
        let ipco = boxed(b"ipco", &[ispe(64, 48), ispe(9_000, 10)].concat());
        let meta = boxed(
            b"meta",
            &[&[0, 0, 0, 0][..], &boxed(b"iprp", &ipco)].concat(),
        );
        let file = [boxed(b"ftyp", b"avif\0\0\0\0"), meta].concat();
        assert_eq!(avif(&file), Some((9_000, 48)));
        for end in 0..file.len() {
            assert!(avif(&file[..end]).is_none() || end == file.len());
        }
        // a box size past the end ends the walk instead of reading past it
        let mut lying = file.clone();
        lying[3] = 0xff;
        assert_eq!(avif(&lying), None);
    }

    #[test]
    fn bmp_and_ico_read_their_headers() {
        let mut dib = 40_u32.to_le_bytes().to_vec();
        dib.extend_from_slice(&100_i32.to_le_bytes());
        dib.extend_from_slice(&(-50_i32).to_le_bytes());
        assert_eq!(bmp(&dib), Some((100, 50)));
        let core = [12, 0, 0, 0, 0x10, 0, 0x20, 0];
        assert_eq!(bmp(&core), Some((16, 32)));
        assert_eq!(bmp(&[200, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]), None);
        // an icon whose one entry is a BMP of 32 × (2 × 32)
        let mut entry = 40_u32.to_le_bytes().to_vec();
        entry.extend_from_slice(&32_i32.to_le_bytes());
        entry.extend_from_slice(&64_i32.to_le_bytes());
        let mut icon = vec![0, 0, 1, 0, 1, 0, 32, 32, 0, 0, 1, 0, 32, 0];
        icon.extend_from_slice(&u32::try_from(entry.len()).unwrap().to_le_bytes());
        icon.extend_from_slice(&22_u32.to_le_bytes());
        icon.extend_from_slice(&entry);
        assert_eq!(ico(&icon), Some((32, 32)));
        // an offset past the end
        let mut past = icon.clone();
        past[18] = 0xff;
        assert_eq!(ico(&past), None);
    }
}
