//! AVIF image sequences (`avis`) as the worker must write them
//! (`docs/profile-pictures.md`, "Animations"): ffmpeg's `avif` muxer with
//! `libaom-av1`, one colour track and, for a picture with transparency, one
//! alpha track. The worker checks every sequence it made against the
//! frames it sent; the service checks the framing and the sample tables
//! again before it keeps one.
use super::{MAX_FRAMES, MAX_SEQUENCE};

/// Every track's sample durations in milliseconds, when `bytes` is an
/// image sequence as the worker writes it: exactly `ftyp` (major brand
/// `avis`), `meta`, `moov` and `mdat`, in that order and filling the file;
/// one movie header; one or two tracks, each with a media header, a sample
/// table of 2 to [`MAX_FRAMES`] samples and durations that agree across
/// tracks. `None` for anything else.
#[must_use]
pub fn sequence_tracks(bytes: &[u8]) -> Option<Vec<Vec<u32>>> {
    if bytes.len() > MAX_SEQUENCE {
        return None;
    }
    let top = boxes(bytes)?;
    let kinds: Vec<&[u8; 4]> = top.iter().map(|(kind, _)| *kind).collect();
    if kinds != [b"ftyp", b"meta", b"moov", b"mdat"] || top[0].1.get(..4)? != b"avis" {
        return None;
    }
    // the still in its meta, and every frame, as the worker writes them
    if !crate::answer::items(top[1].1) || !crate::answer::bitstream(top[3].1) {
        return None;
    }
    let moov = boxes(top[2].1)?;
    if moov.iter().filter(|(kind, _)| *kind == b"mvhd").count() != 1 {
        return None;
    }
    let tracks = moov
        .iter()
        .filter(|(kind, _)| *kind == b"trak")
        .map(|(_, body)| track(body))
        .collect::<Option<Vec<_>>>()?;
    let first = tracks.first()?;
    (matches!(tracks.len(), 1 | 2) && tracks.iter().all(|t| t == first)).then_some(tracks)
}

/// Whether `bytes` is an image sequence as the worker writes it.
#[must_use]
pub fn sequence_framing(bytes: &[u8]) -> bool {
    sequence_tracks(bytes).is_some()
}

/// One track's sample durations (ms): `mdia` with `mdhd` and
/// `minf`/`stbl` with `stts` and `stsz`, whose counts agree.
fn track(trak: &[u8]) -> Option<Vec<u32>> {
    let mdia = child(trak, b"mdia")?;
    let mdhd = child(mdia, b"mdhd")?;
    let timescale = match mdhd.first()? {
        0 => be32(mdhd, 12)?,
        1 => be32(mdhd, 20)?,
        _ => return None,
    };
    if timescale == 0 {
        return None;
    }
    let stbl = child(child(mdia, b"minf")?, b"stbl")?;
    let stts = child(stbl, b"stts")?;
    let stsz = child(stbl, b"stsz")?;
    let samples = usize::try_from(be32(stsz, 8)?).ok()?;
    let entries = usize::try_from(be32(stts, 4)?).ok()?;
    if !(2..=MAX_FRAMES).contains(&samples) || entries > samples {
        return None;
    }
    let mut durations = Vec::with_capacity(samples);
    for entry in 0..entries {
        let count = usize::try_from(be32(stts, 8 + entry * 8)?).ok()?;
        let delta = u64::from(be32(stts, 12 + entry * 8)?);
        if count > samples - durations.len() {
            return None;
        }
        let ms =
            u32::try_from((delta * 1000 + u64::from(timescale) / 2) / u64::from(timescale)).ok()?;
        durations.extend(std::iter::repeat_n(ms, count));
    }
    (durations.len() == samples).then_some(durations)
}

/// The boxes that fill `bytes`: 32-bit sizes only, each at least its
/// header and within what is left.
pub(crate) fn boxes(mut bytes: &[u8]) -> Option<Vec<(&[u8; 4], &[u8])>> {
    let mut out = Vec::new();
    while !bytes.is_empty() {
        let size = usize::try_from(be32(bytes, 0)?).ok()?;
        let kind: &[u8; 4] = bytes.get(4..8)?.try_into().ok()?;
        if size < 8 || size > bytes.len() {
            return None;
        }
        out.push((kind, &bytes[8..size]));
        bytes = &bytes[size..];
    }
    Some(out)
}

/// The body of the one child box of `kind` in `parent`.
fn child<'a>(parent: &'a [u8], kind: &[u8]) -> Option<&'a [u8]> {
    let mut found = boxes(parent)?
        .into_iter()
        .filter(|(k, _)| k.as_slice() == kind);
    let (_, body) = found.next()?;
    found.next().is_none().then_some(body)
}

fn be32(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boxed(kind: &[u8], body: &[u8]) -> Vec<u8> {
        let mut out = u32::try_from(body.len() + 8)
            .unwrap()
            .to_be_bytes()
            .to_vec();
        out.extend_from_slice(kind);
        out.extend_from_slice(body);
        out
    }

    fn full(kind: &[u8], fields: &[u32]) -> Vec<u8> {
        let mut body = vec![0, 0, 0, 0];
        for field in fields {
            body.extend_from_slice(&field.to_be_bytes());
        }
        boxed(kind, &body)
    }

    /// A track of `durations` (ms) at a time scale of 16,000.
    fn trak(durations: &[u32]) -> Vec<u8> {
        let mut stts = vec![u32::try_from(durations.len()).unwrap()];
        for d in durations {
            stts.extend([1, d * 16]);
        }
        let stbl = boxed(
            b"stbl",
            &[
                full(b"stts", &stts),
                full(b"stsz", &[0, u32::try_from(durations.len()).unwrap()]),
            ]
            .concat(),
        );
        let mdia = boxed(
            b"mdia",
            &[full(b"mdhd", &[0, 0, 16_000, 0]), boxed(b"minf", &stbl)].concat(),
        );
        boxed(b"trak", &mdia)
    }

    fn file(traks: &[Vec<u8>]) -> Vec<u8> {
        let moov = boxed(b"moov", &[full(b"mvhd", &[0; 24]), traks.concat()].concat());
        [
            boxed(b"ftyp", b"avis\0\0\0\0avisavif"),
            boxed(
                b"meta",
                &crate::answer::tests::meta_body(
                    &crate::answer::tests::still_items(false),
                    &crate::answer::tests::properties_of(256, b"nclx", false),
                    false,
                ),
            ),
            moov,
            boxed(b"mdat", &crate::av1::tests::sequence(8, 8)),
        ]
        .concat()
    }

    #[test]
    fn durations_are_read_per_track() {
        let delays = [100, 40, 250, 500];
        let one = file(&[trak(&delays)]);
        assert_eq!(sequence_tracks(&one), Some(vec![delays.to_vec()]));
        let two = file(&[trak(&delays), trak(&delays)]);
        assert_eq!(sequence_tracks(&two).map(|t| t.len()), Some(2));
        assert!(sequence_framing(&two));
    }

    #[test]
    fn anything_else_is_refused() {
        let delays = [100, 40];
        // tracks that disagree, none, three
        assert!(!sequence_framing(&file(&[trak(&delays), trak(&[100, 41])])));
        assert!(!sequence_framing(&file(&[])));
        assert!(!sequence_framing(&file(&[
            trak(&delays),
            trak(&delays),
            trak(&delays)
        ])));
        // one frame, and more than the bound
        assert!(!sequence_framing(&file(&[trak(&[100])])));
        assert!(!sequence_framing(&file(&[trak(&[100; MAX_FRAMES + 1])])));
        // a still's brand, a box out of order, anything after the file
        let good = file(&[trak(&delays)]);
        let mut still = good.clone();
        still[8..12].copy_from_slice(b"avif");
        assert!(!sequence_framing(&still));
        let mut appended = good.clone();
        appended.extend_from_slice(b"<html>");
        assert!(!sequence_framing(&appended));
        for end in 0..good.len() {
            assert!(!sequence_framing(&good[..end]), "cut at {end}");
        }
        // a sample table that claims more samples than its durations
        let mut lying = good.clone();
        let at = lying.windows(4).position(|w| w == b"stsz").unwrap() + 12;
        lying[at..at + 4].copy_from_slice(&3_u32.to_be_bytes());
        assert!(!sequence_framing(&lying));
        // larger than the bound
        let big = [good.clone(), vec![0; MAX_SEQUENCE]].concat();
        assert!(!sequence_framing(&big));
    }
}
