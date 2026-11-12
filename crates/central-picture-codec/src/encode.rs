//! The two stored pictures, made from the square's pixels alone: a PNG
//! master (IHDR, IDAT and IEND; RGB when opaque, RGBA otherwise; the same
//! bytes for the same pixels) and an AVIF (`ravif` with `rav1e`, one
//! thread, no metadata).
use super::scale::{SIDE, Square};
use super::{Output, Refusal};
use central_picture::{MAX_AVIF, MAX_PNG};

/// AVIF quality: 80 keeps a face's detail at 256 px in 6–16 KB for photos
/// (`docs/dependency-proposal/profile-picture.md` §4.2). A picture that
/// still exceeds [`MAX_AVIF`] is encoded again at the lower ones.
const QUALITIES: [f32; 3] = [80.0, 60.0, 40.0];
/// AVIF speed: 8 costs about a quarter second of CPU and gives files
/// 10–24 % smaller than 10 (§4.2).
const SPEED: u8 = 8;

/// Both pictures of `square`.
///
/// # Errors
/// `Failed` when an encoder fails or no quality fits the AVIF's bound.
pub(crate) fn both(square: &Square) -> Result<Output, Refusal> {
    let png = png_master(&square.0)?;
    let avif = avif(&square.0)?;
    Ok(Output { avif, png })
}

fn png_master(rgba: &[u8]) -> Result<Vec<u8>, Refusal> {
    let (pixels4, _) = rgba.as_chunks::<4>();
    let opaque = pixels4.iter().all(|p| p[3] == 255);
    let (colour, pixels): (png::ColorType, Vec<u8>) = if opaque {
        (
            png::ColorType::Rgb,
            pixels4.iter().flat_map(|p| [p[0], p[1], p[2]]).collect(),
        )
    } else {
        (png::ColorType::Rgba, rgba.to_vec())
    };
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, SIDE, SIDE);
        encoder.set_color(colour);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::High);
        let mut writer = encoder.write_header().map_err(|_| Refusal::Failed)?;
        writer
            .write_image_data(&pixels)
            .map_err(|_| Refusal::Failed)?;
        writer.finish().map_err(|_| Refusal::Failed)?;
    }
    if out.len() > MAX_PNG {
        return Err(Refusal::Failed);
    }
    Ok(out)
}

fn avif(rgba: &[u8]) -> Result<Vec<u8>, Refusal> {
    let pixels: Vec<ravif::RGBA8> = rgba
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| ravif::RGBA8::new(p[0], p[1], p[2], p[3]))
        .collect();
    let side = usize::try_from(SIDE).map_err(|_| Refusal::Failed)?;
    for quality in QUALITIES {
        let encoded = ravif::Encoder::new()
            .with_quality(quality)
            .with_alpha_quality(quality)
            .with_speed(SPEED)
            .with_num_threads(Some(1))
            .encode_rgba(ravif::Img::new(&pixels[..], side, side))
            .map_err(|_| Refusal::Failed)?;
        if encoded.avif_file.len() <= MAX_AVIF {
            return Ok(encoded.avif_file);
        }
    }
    Err(Refusal::Failed)
}

/// The animated variant of `moving`: none yet. Its stored format is the
/// owner's choice (proposal §0.6 and §10); the frames reach this one
/// function decoded, bounded, composited and averaged to 256 px, so the
/// chosen encoder drops in here and nowhere else.
pub(crate) fn animation(moving: &crate::Animation) -> Option<Vec<u8>> {
    let _ = moving;
    None
}
