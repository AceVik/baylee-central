//! Every decoder and both encoders of profile pictures
//! (`docs/profile-pictures.md`). Only the worker binary links this crate:
//! it runs as a fresh, sandboxed process for each picture, holding no
//! secret, so a bug in a decoder or the encoder reaches nothing of value.
//!
//! [`convert`] takes a file whose format the request declared, checks its
//! first bytes and its header with readers of our own before any decoder
//! runs, decodes the first frame within bounds, applies a JPEG's EXIF
//! orientation, takes the centred square, averages it to 256 × 256 and
//! writes a PNG master (IHDR, IDAT and IEND only) and an AVIF from those
//! pixels alone. Nothing of the file but its pixels reaches either.
#![forbid(unsafe_code)]

mod animation;
mod av1;
mod decode;
mod encode;
mod exif;
mod header;
mod scale;

pub use animation::{
    Animation, Frame, MAX_CANVAS, MAX_COMPOSITED, MAX_FRAMES, MAX_PLAY_MS, MIN_DELAY_MS, SLOW_MS,
    frames,
};
pub use central_picture::{Format, Output, Refusal};

/// Longest side of any picture or frame, in pixels.
pub const MAX_SIDE: u32 = 8_192;
/// Most pixels of any picture: 4096², which a 12-megapixel photo fits.
pub const MAX_PIXELS: u64 = 16_777_216;
/// Shortest side, which the square is cut to.
pub const MIN_SIDE: u32 = 16;

/// Decoded pixels: 8-bit RGBA, not premultiplied, row by row, and how to
/// turn them to stand upright (EXIF orientation 1 to 8; 1 is none).
pub(crate) struct Raster {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) rgba: Vec<u8>,
    pub(crate) orientation: u8,
}

/// The dimensions of a picture are within [`MAX_SIDE`], [`MAX_PIXELS`] and
/// [`MIN_SIDE`].
///
/// # Errors
/// `TooManyPixels` or `TooSmall`.
pub(crate) fn within(width: u32, height: u32) -> Result<(), Refusal> {
    if width > MAX_SIDE || height > MAX_SIDE || u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(Refusal::TooManyPixels);
    }
    if width.min(height) < MIN_SIDE {
        return Err(Refusal::TooSmall);
    }
    Ok(())
}

/// Convert `bytes`, declared as `format`, to the two stored pictures.
///
/// # Errors
/// The [`Refusal`] that names the first check the file failed.
pub fn convert(format: Format, bytes: &[u8]) -> Result<Output, Refusal> {
    // A decoder that panics on a hostile file (two do, found by the tests:
    // `avif-parse` 2.1.0 and `resvg` 0.50.0's turbulence filter) refuses the
    // file; the process stays whole and answers.
    std::panic::catch_unwind(|| checked(format, bytes)).unwrap_or(Err(Refusal::Malformed))
}

fn checked(format: Format, bytes: &[u8]) -> Result<Output, Refusal> {
    if bytes.len() > format.max_bytes() {
        return Err(Refusal::TooLarge);
    }
    match Format::sniff(bytes) {
        Some(found) if found == format => {}
        Some(_) => return Err(Refusal::WrongType),
        None => return Err(Refusal::Unsupported),
    }
    // An AVIF image sequence: `avif-parse` 2.1.0 refuses a file whose
    // major brand is `avis`, and no accepted package decodes AV1 sequences
    // through a safe interface (proposal §0.4). A file that only lists
    // `avis` among its compatible brands gives its still item.
    if format == Format::Avif && bytes.get(8..12) == Some(b"avis".as_slice()) {
        return Err(Refusal::Unsupported);
    }
    if format != Format::Svg {
        let (width, height) = header::dimensions(format, bytes)?;
        within(width, height)?;
        // An animation's still is its first composited frame, so the two
        // cannot disagree (proposal §0.4).
        if let Some(moving) = animation::frames(format, bytes)? {
            let first = scale::Square(moving.frames[0].rgba.clone());
            let output = encode::both(&first)?;
            // Nothing is kept of the animation until its format is chosen.
            let _ = encode::animation(&moving);
            return Ok(output);
        }
    }
    let raster = if format == Format::Svg {
        decode::svg(bytes)?
    } else {
        decode::raster(format, bytes)?
    };
    within(raster.width, raster.height)?;
    let square = scale::square(&raster)?;
    encode::both(&square)
}
