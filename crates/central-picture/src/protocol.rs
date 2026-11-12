//! The worker protocol: one request on the worker's standard input, one
//! answer on its standard output, then the worker exits. Both are framed by
//! a magic, a version and lengths, and both are read with bounds.
//!
//! Request: `BCPQ`, version 2, the format's byte, the length (u32, big
//! endian), the file. Answer: `BCPA`, version 2, then either 0, the AVIF's
//! length and bytes, the PNG's length and bytes and the [`Motion`]'s byte
//! (for a sequence followed by its length and bytes), or 1 and a refusal's
//! byte. Nothing may follow either.
use super::{Format, MAX_AVIF, MAX_PNG, MAX_SEQUENCE};
use std::io::Read;

const REQUEST_MAGIC: &[u8; 4] = b"BCPQ";
const ANSWER_MAGIC: &[u8; 4] = b"BCPA";
const VERSION: u8 = 2;
const HEADER: usize = 10;

/// The largest request: its header and the largest file.
pub const MAX_REQUEST: usize = HEADER + super::MAX_UPLOAD;
/// The largest answer: its header and every picture at its bound.
pub const MAX_ANSWER: usize = 6 + 4 + MAX_AVIF + 4 + MAX_PNG + 1 + 4 + MAX_SEQUENCE;

/// Why a picture was not taken. The closed list the page has words for;
/// nothing of the file or of a decoder's message is ever passed on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// Not a format Central takes.
    Unsupported,
    /// The file's own bytes say another format than the request did.
    WrongType,
    /// More bytes than the format may have.
    TooLarge,
    /// More pixels, or a longer side, than a picture may have.
    TooManyPixels,
    /// A side shorter than a picture may have.
    TooSmall,
    /// The decoder refused the file: truncated, damaged or not what it says.
    Malformed,
    /// The conversion failed for a reason of Central's own: the worker
    /// crashed, ran out of time or answered something it may not.
    Failed,
}

impl Refusal {
    /// Every refusal.
    pub const ALL: [Self; 7] = [
        Self::Unsupported,
        Self::WrongType,
        Self::TooLarge,
        Self::TooManyPixels,
        Self::TooSmall,
        Self::Malformed,
        Self::Failed,
    ];

    /// The name the upload route answers with.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unsupported => "unsupported",
            Self::WrongType => "wrong_type",
            Self::TooLarge => "too_large",
            Self::TooManyPixels => "too_many_pixels",
            Self::TooSmall => "too_small",
            Self::Malformed => "malformed",
            Self::Failed => "failed",
        }
    }

    const fn code(self) -> u8 {
        match self {
            Self::Unsupported => 1,
            Self::WrongType => 2,
            Self::TooLarge => 3,
            Self::TooManyPixels => 4,
            Self::TooSmall => 5,
            Self::Malformed => 6,
            Self::Failed => 7,
        }
    }

    fn from_code(code: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|refusal| refusal.code() == code)
    }
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::error::Error for Refusal {}

/// The worker's pictures, as it made them; the service checks each
/// ([`crate::png_master`], [`crate::avif_framing`],
/// [`crate::sequence_tracks`]) before it keeps them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Output {
    /// The still's AVIF.
    pub avif: Vec<u8>,
    /// The still's PNG master and fallback.
    pub png: Vec<u8>,
    /// Whether the picture moves, and what became of that.
    pub motion: Motion,
}

/// What became of a picture's movement. An animation that could not be
/// encoded is kept as its still, its first frame: the upload never fails
/// for its animation alone, and the service learns why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Motion {
    /// A still: the file has one frame.
    Still,
    /// An animation, and its AVIF image sequence.
    Sequence(Vec<u8>),
    /// An animation kept as its still: no ffmpeg with `libaom-av1` here.
    NoEncoder,
    /// An animation kept as its still: the encoder failed, ran out of
    /// time, or made what the checks refuse.
    Failed,
    /// An animation kept as its still: no quality fit [`MAX_SEQUENCE`].
    TooLarge,
}

impl Motion {
    const fn code(&self) -> u8 {
        match self {
            Self::Still => 0,
            Self::Sequence(_) => 1,
            Self::NoEncoder => 2,
            Self::Failed => 3,
            Self::TooLarge => 4,
        }
    }

    /// The name logs and the upload's answer use; nothing of the picture.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Still => "still",
            Self::Sequence(_) => "sequence",
            Self::NoEncoder => "no_encoder",
            Self::Failed => "failed",
            Self::TooLarge => "too_large",
        }
    }
}

/// The request for `bytes` of `format`.
///
/// # Errors
/// `TooLarge` beyond the format's bound.
pub fn request(format: Format, bytes: &[u8]) -> Result<Vec<u8>, Refusal> {
    if bytes.len() > format.max_bytes() {
        return Err(Refusal::TooLarge);
    }
    let length = u32::try_from(bytes.len()).map_err(|_| Refusal::TooLarge)?;
    let mut out = Vec::with_capacity(HEADER + bytes.len());
    out.extend_from_slice(REQUEST_MAGIC);
    out.push(VERSION);
    out.push(format.code());
    out.extend_from_slice(&length.to_be_bytes());
    out.extend_from_slice(bytes);
    Ok(out)
}

/// The request on `input`, read to its end and no further than its bound.
///
/// # Errors
/// `Malformed` for a frame that is broken, cut short or followed by
/// anything; `TooLarge` for a length beyond the format's bound, refused
/// before a byte of the file is read.
pub fn read_request(input: &mut impl Read) -> Result<(Format, Vec<u8>), Refusal> {
    let mut header = [0; HEADER];
    input
        .read_exact(&mut header)
        .map_err(|_| Refusal::Malformed)?;
    if &header[..4] != REQUEST_MAGIC || header[4] != VERSION {
        return Err(Refusal::Malformed);
    }
    let format = Format::from_code(header[5]).ok_or(Refusal::Unsupported)?;
    let length = u32::from_be_bytes([header[6], header[7], header[8], header[9]]);
    let length = usize::try_from(length).map_err(|_| Refusal::TooLarge)?;
    if length > format.max_bytes() {
        return Err(Refusal::TooLarge);
    }
    let mut bytes = Vec::with_capacity(length);
    // One byte more than announced shows a frame that goes on.
    let wanted = u64::try_from(length).map_err(|_| Refusal::TooLarge)? + 1;
    input
        .take(wanted)
        .read_to_end(&mut bytes)
        .map_err(|_| Refusal::Malformed)?;
    if bytes.len() != length {
        return Err(Refusal::Malformed);
    }
    Ok((format, bytes))
}

/// The answer frame for `result`.
#[must_use]
pub fn answer(result: &Result<Output, Refusal>) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(ANSWER_MAGIC);
    out.push(VERSION);
    match result {
        Ok(output) => {
            out.push(0);
            push_part(&mut out, &output.avif);
            push_part(&mut out, &output.png);
            out.push(output.motion.code());
            if let Motion::Sequence(sequence) = &output.motion {
                push_part(&mut out, sequence);
            }
        }
        Err(refusal) => {
            out.push(1);
            out.push(refusal.code());
        }
    }
    out
}

/// The answer in `bytes`, which must be exactly one frame within its
/// bounds; `None` for anything else.
#[must_use]
pub fn read_answer(bytes: &[u8]) -> Option<Result<Output, Refusal>> {
    if bytes.len() > MAX_ANSWER {
        return None;
    }
    let rest = bytes.strip_prefix(ANSWER_MAGIC)?;
    let (&version, rest) = rest.split_first()?;
    let (&kind, rest) = rest.split_first()?;
    if version != VERSION {
        return None;
    }
    match kind {
        0 => {
            let (avif, rest) = part(rest, MAX_AVIF)?;
            let (png, rest) = part(rest, MAX_PNG)?;
            let (&code, rest) = rest.split_first()?;
            let (motion, rest) = match code {
                0 => (Motion::Still, rest),
                1 => {
                    let (sequence, rest) = part(rest, MAX_SEQUENCE)?;
                    (Motion::Sequence(sequence.to_vec()), rest)
                }
                2 => (Motion::NoEncoder, rest),
                3 => (Motion::Failed, rest),
                4 => (Motion::TooLarge, rest),
                _ => return None,
            };
            rest.is_empty().then(|| {
                Ok(Output {
                    avif: avif.to_vec(),
                    png: png.to_vec(),
                    motion,
                })
            })
        }
        1 => match rest {
            [code] => Some(Err(Refusal::from_code(*code)?)),
            _ => None,
        },
        _ => None,
    }
}

/// `part` with its length (u32, big endian) in front. A part beyond u32 is
/// beyond every bound and refused by the reader as broken.
fn push_part(out: &mut Vec<u8>, part: &[u8]) {
    let length = u32::try_from(part.len()).unwrap_or(u32::MAX);
    out.extend_from_slice(&length.to_be_bytes());
    out.extend_from_slice(part);
}

/// A length-prefixed part of at most `bound` bytes, and what follows it.
fn part(bytes: &[u8], bound: usize) -> Option<(&[u8], &[u8])> {
    let length = u32::from_be_bytes(bytes.get(..4)?.try_into().ok()?);
    let length = usize::try_from(length).ok()?;
    if length > bound {
        return None;
    }
    let rest = &bytes[4..];
    (rest.len() >= length).then(|| rest.split_at(length))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn a_request_round_trips_and_nothing_else_is_read() {
        let framed = request(Format::Gif, b"GIF89a").unwrap();
        assert_eq!(&framed[..6], b"BCPQ\x02\x03");
        assert_eq!(
            read_request(&mut Cursor::new(&framed)).unwrap(),
            (Format::Gif, b"GIF89a".to_vec())
        );
        // a length that lies, either way, and anything after the frame
        let mut longer = framed.clone();
        longer[9] += 1;
        assert_eq!(
            read_request(&mut Cursor::new(&longer)),
            Err(Refusal::Malformed)
        );
        let mut shorter = framed.clone();
        shorter[9] -= 1;
        assert_eq!(
            read_request(&mut Cursor::new(&shorter)),
            Err(Refusal::Malformed)
        );
        let mut trailing = framed.clone();
        trailing.push(0);
        assert_eq!(
            read_request(&mut Cursor::new(&trailing)),
            Err(Refusal::Malformed)
        );
        // every cut of the frame
        for end in 0..framed.len() {
            assert!(
                read_request(&mut Cursor::new(&framed[..end])).is_err(),
                "{end}"
            );
        }
    }

    #[test]
    fn a_request_header_is_checked_before_its_body() {
        let mut framed = request(Format::Png, b"x").unwrap();
        framed[0] = b'X';
        assert_eq!(
            read_request(&mut Cursor::new(&framed)),
            Err(Refusal::Malformed)
        );
        let mut framed = request(Format::Png, b"x").unwrap();
        // the first version, and one to come
        framed[4] = 1;
        assert_eq!(
            read_request(&mut Cursor::new(&framed)),
            Err(Refusal::Malformed)
        );
        framed[4] = 3;
        assert_eq!(
            read_request(&mut Cursor::new(&framed)),
            Err(Refusal::Malformed)
        );
        let mut framed = request(Format::Png, b"x").unwrap();
        framed[5] = 0;
        assert_eq!(
            read_request(&mut Cursor::new(&framed)),
            Err(Refusal::Unsupported)
        );
        // an announced length beyond the format's bound, with no body at all
        let huge = [b"BCPQ\x02\x06".as_slice(), &u32::MAX.to_be_bytes()].concat();
        assert_eq!(
            read_request(&mut Cursor::new(&huge)),
            Err(Refusal::TooLarge)
        );
        let svg = [b"BCPQ\x02\x06".as_slice(), &(1_048_577_u32).to_be_bytes()].concat();
        assert_eq!(read_request(&mut Cursor::new(&svg)), Err(Refusal::TooLarge));
        assert_eq!(
            request(Format::Svg, &vec![b' '; 1_048_577]),
            Err(Refusal::TooLarge)
        );
    }

    #[test]
    fn answers_round_trip_and_bounds_hold() {
        let motions = [
            Motion::Still,
            Motion::Sequence(vec![3; 90]),
            Motion::NoEncoder,
            Motion::Failed,
            Motion::TooLarge,
        ];
        for motion in motions {
            let output = Output {
                avif: vec![1; 40],
                png: vec![2; 70],
                motion,
            };
            assert_eq!(read_answer(&answer(&Ok(output.clone()))), Some(Ok(output)));
        }
        for refusal in Refusal::ALL {
            assert_eq!(read_answer(&answer(&Err(refusal))), Some(Err(refusal)));
        }
        let good = answer(&Ok(Output {
            avif: vec![1; 3],
            png: vec![2; 3],
            motion: Motion::Sequence(vec![3; 3]),
        }));
        for end in 0..good.len() {
            assert_eq!(read_answer(&good[..end]), None, "{end}");
        }
        let mut trailing = good.clone();
        trailing.push(0);
        assert_eq!(read_answer(&trailing), None);
        // a motion the protocol has no name for
        let mut unknown = answer(&Ok(Output {
            avif: vec![1; 3],
            png: vec![2; 3],
            motion: Motion::Still,
        }));
        *unknown.last_mut().unwrap() = 5;
        assert_eq!(read_answer(&unknown), None);
        for (avif, sequence) in [(MAX_AVIF + 1, 3), (3, MAX_SEQUENCE + 1)] {
            let too_big = answer(&Ok(Output {
                avif: vec![1; avif],
                png: vec![2; 3],
                motion: Motion::Sequence(vec![3; sequence]),
            }));
            assert_eq!(read_answer(&too_big), None);
        }
        assert_eq!(read_answer(b"BCPA\x02\x01\x00"), None);
        assert_eq!(read_answer(b"BCPA\x02\x02"), None);
        // the first version is no longer read
        assert_eq!(read_answer(b"BCPA\x01\x01\x01"), None);
        assert_eq!(read_answer(&vec![0; MAX_ANSWER + 1]), None);
    }
}
