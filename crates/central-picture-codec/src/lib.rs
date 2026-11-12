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

mod av1;
mod decode;
mod encode;
mod exif;
mod header;
mod scale;

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
    let raster = if format == Format::Svg {
        decode::svg(bytes)?
    } else {
        let (width, height) = header::dimensions(format, bytes)?;
        within(width, height)?;
        decode::raster(format, bytes)?
    };
    within(raster.width, raster.height)?;
    let square = scale::square(&raster)?;
    encode::both(&square)
}
