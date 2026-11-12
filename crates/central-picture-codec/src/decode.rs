//! The decoders, one per format, each bounded by its own limits besides
//! the header check before it, each giving the first frame as 8-bit RGBA.
//! Text, colour profiles and every other kind of metadata are skipped or
//! dropped; only a JPEG's orientation is read.
use super::{MAX_PIXELS, MAX_SIDE, Raster, Refusal, av1, exif, within};
use std::io::Cursor;

/// What any decoder may allocate: the largest RGBA picture and as much
/// again for its working buffers.
const MEMORY: u64 = MAX_PIXELS * 4 * 2;

fn bytes_of(memory: u64) -> usize {
    usize::try_from(memory).unwrap_or(usize::MAX)
}

/// The raster formats.
pub(crate) fn raster(format: super::Format, bytes: &[u8]) -> Result<Raster, Refusal> {
    use super::Format;
    match format {
        Format::Png => png(bytes),
        Format::Jpeg => jpeg(bytes),
        Format::Gif => gif(bytes),
        Format::Webp => webp(bytes),
        Format::Avif => avif(bytes),
        Format::Bmp => image(bytes, ::image::ImageFormat::Bmp),
        Format::Ico => image(bytes, ::image::ImageFormat::Ico),
        Format::Svg => svg(bytes),
    }
}

fn upright(width: u32, height: u32, rgba: Vec<u8>) -> Result<Raster, Refusal> {
    within(width, height)?;
    let expected = usize::try_from(u64::from(width) * u64::from(height) * 4)
        .map_err(|_| Refusal::TooManyPixels)?;
    if rgba.len() != expected {
        return Err(Refusal::Malformed);
    }
    Ok(Raster {
        width,
        height,
        rgba,
        orientation: 1,
    })
}

/// `pixels` of `channels` (1 grey, 2 grey and alpha, 3 RGB, 4 RGBA) as
/// RGBA; RGBA is handed on without a copy.
fn to_rgba(pixels: Vec<u8>, channels: usize) -> Result<Vec<u8>, Refusal> {
    if channels == 4 {
        return Ok(pixels);
    }
    let mut out = Vec::new();
    out.try_reserve_exact(pixels.len() / channels * 4)
        .map_err(|_| Refusal::TooManyPixels)?;
    for p in pixels.chunks_exact(channels) {
        match *p {
            [g] => out.extend_from_slice(&[g, g, g, 255]),
            [g, a] => out.extend_from_slice(&[g, g, g, a]),
            [r, g, b] => out.extend_from_slice(&[r, g, b, 255]),
            _ => return Err(Refusal::Malformed),
        }
    }
    Ok(out)
}

/// PNG with `png`: its default image (an APNG's first frame or the image
/// non-animated viewers show), every bit depth normalised to 8 with alpha,
/// text and ICC chunks skipped unread, CRCs and Adler-32 checked.
fn png(bytes: &[u8]) -> Result<Raster, Refusal> {
    let mut decoder = png::Decoder::new_with_limits(
        Cursor::new(bytes),
        png::Limits {
            bytes: bytes_of(MEMORY),
        },
    );
    // CRCs and the zlib stream's Adler-32 are checked (png skips the
    // Adler-32 unless told).
    decoder.ignore_checksums(false);
    decoder.set_ignore_text_chunk(true);
    decoder.set_ignore_iccp_chunk(true);
    decoder.set_transformations(
        png::Transformations::normalize_to_color8() | png::Transformations::ALPHA,
    );
    let mut reader = decoder.read_info().map_err(|_| Refusal::Malformed)?;
    let (width, height) = (reader.info().width, reader.info().height);
    within(width, height)?;
    let size = reader.output_buffer_size().ok_or(Refusal::TooManyPixels)?;
    let mut buffer = Vec::new();
    buffer
        .try_reserve_exact(size)
        .map_err(|_| Refusal::TooManyPixels)?;
    buffer.resize(size, 0);
    let frame = reader
        .next_frame(&mut buffer)
        .map_err(|_| Refusal::Malformed)?;
    if frame.bit_depth != png::BitDepth::Eight || (frame.width, frame.height) != (width, height) {
        return Err(Refusal::Malformed);
    }
    buffer.truncate(frame.buffer_size());
    let channels = match frame.color_type {
        png::ColorType::Grayscale => 1,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        png::ColorType::Indexed => return Err(Refusal::Malformed),
    };
    // A row may be padded; then take each row's pixels only. Unpadded, the
    // buffer is the picture, without a copy (a 4096² RGBA is 64 MiB).
    let row = usize::try_from(width).map_err(|_| Refusal::Malformed)? * channels;
    let rows = usize::try_from(height).map_err(|_| Refusal::Malformed)?;
    let stride = frame.line_size;
    let pixels = if stride == row {
        buffer.truncate(row * rows);
        buffer
    } else {
        let mut pixels = Vec::with_capacity(row * rows);
        for line in buffer.chunks(stride.max(1)).take(rows) {
            pixels.extend_from_slice(line.get(..row).ok_or(Refusal::Malformed)?);
        }
        pixels
    };
    upright(width, height, to_rgba(pixels, channels)?)
}

/// JPEG with `zune-jpeg` (scalar, no SIMD), baseline or progressive, grey,
/// YCbCr or CMYK, to RGBA, with its EXIF orientation kept for the scaler.
fn jpeg(bytes: &[u8]) -> Result<Raster, Refusal> {
    let side = usize::try_from(MAX_SIDE).unwrap_or(usize::MAX);
    let options = zune_core::options::DecoderOptions::default()
        .set_max_width(side)
        .set_max_height(side)
        .jpeg_set_out_colorspace(zune_core::colorspace::ColorSpace::RGBA);
    let mut decoder = zune_jpeg::JpegDecoder::new_with_options(
        zune_core::bytestream::ZCursor::new(bytes),
        options,
    );
    decoder.decode_headers().map_err(|_| Refusal::Malformed)?;
    let (width, height) = decoder.dimensions().ok_or(Refusal::Malformed)?;
    let width = u32::try_from(width).map_err(|_| Refusal::TooManyPixels)?;
    let height = u32::try_from(height).map_err(|_| Refusal::TooManyPixels)?;
    within(width, height)?;
    let orientation = decoder.exif().map_or(1, |tiff| exif::orientation(tiff));
    let pixels = decoder.decode().map_err(|_| Refusal::Malformed)?;
    let mut raster = upright(width, height, pixels)?;
    raster.orientation = orientation;
    Ok(raster)
}

/// GIF with `gif`: the first frame, drawn at its place on the logical
/// screen, which is transparent elsewhere.
fn gif(bytes: &[u8]) -> Result<Raster, Refusal> {
    let mut options = gif::DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::RGBA);
    options.set_memory_limit(gif::MemoryLimit::Bytes(
        std::num::NonZeroU64::new(MAX_PIXELS * 4).ok_or(Refusal::Failed)?,
    ));
    let mut decoder = options
        .read_info(Cursor::new(bytes))
        .map_err(|_| Refusal::Malformed)?;
    let (width, height) = (u32::from(decoder.width()), u32::from(decoder.height()));
    within(width, height)?;
    let (screen_width, screen_height) =
        (usize::from(decoder.width()), usize::from(decoder.height()));
    let frame = decoder
        .read_next_frame()
        .map_err(|_| Refusal::Malformed)?
        .ok_or(Refusal::Malformed)?;
    let mut canvas = vec![0; screen_width * screen_height * 4];
    let (left, top) = (usize::from(frame.left), usize::from(frame.top));
    let frame_width = usize::from(frame.width);
    if frame.buffer.len() != frame_width * usize::from(frame.height) * 4 {
        return Err(Refusal::Malformed);
    }
    for (y, row) in frame.buffer.chunks(frame_width.max(1) * 4).enumerate() {
        let target_y = top + y;
        if target_y >= screen_height {
            break;
        }
        let visible = frame_width.min(screen_width.saturating_sub(left));
        let start = (target_y * screen_width + left) * 4;
        if let Some(target) = canvas.get_mut(start..start + visible * 4) {
            target.copy_from_slice(&row[..visible * 4]);
        }
    }
    upright(width, height, canvas)
}

/// WebP with `image-webp`: lossy, lossless or extended; an animation's
/// first frame on its canvas.
fn webp(bytes: &[u8]) -> Result<Raster, Refusal> {
    let mut decoder =
        image_webp::WebPDecoder::new(Cursor::new(bytes)).map_err(|_| Refusal::Malformed)?;
    decoder.set_memory_limit(bytes_of(MEMORY));
    let (width, height) = decoder.dimensions();
    within(width, height)?;
    let size = decoder.output_buffer_size().ok_or(Refusal::TooManyPixels)?;
    let mut buffer = vec![0; size];
    decoder
        .read_image(&mut buffer)
        .map_err(|_| Refusal::Malformed)?;
    let channels = if decoder.has_alpha() { 4 } else { 3 };
    upright(width, height, to_rgba(buffer, channels)?)
}

/// AVIF with `avif-decode` (`avif-parse` and `rav1d`): the primary item and
/// its alpha, each AV1 item bounded by its sequence header first.
fn avif(bytes: &[u8]) -> Result<Raster, Refusal> {
    let parsed = avif_parse::read_avif(&mut &bytes[..]).map_err(|_| Refusal::Malformed)?;
    av1::bounded(&parsed.primary_item)?;
    if let Some(alpha) = &parsed.alpha_item {
        av1::bounded(alpha)?;
    }
    drop(parsed);
    let image = avif_decode::Decoder::from_avif(bytes)
        .and_then(avif_decode::Decoder::to_image)
        .map_err(|_| Refusal::Malformed)?;
    let dims = |w: usize, h: usize| -> Result<(u32, u32), Refusal> {
        let width = u32::try_from(w).map_err(|_| Refusal::TooManyPixels)?;
        let height = u32::try_from(h).map_err(|_| Refusal::TooManyPixels)?;
        within(width, height)?;
        Ok((width, height))
    };
    let (width, height, rgba) = match image {
        avif_decode::Image::Rgb8(img) => {
            let (w, h) = dims(img.width(), img.height())?;
            let rgba = img.pixels().flat_map(|p| [p.r, p.g, p.b, 255]).collect();
            (w, h, rgba)
        }
        avif_decode::Image::Rgba8(img) => {
            let (w, h) = dims(img.width(), img.height())?;
            let rgba = img.pixels().flat_map(|p| [p.r, p.g, p.b, p.a]).collect();
            (w, h, rgba)
        }
        avif_decode::Image::Rgb16(img) => {
            let (w, h) = dims(img.width(), img.height())?;
            let rgba = img
                .pixels()
                .flat_map(|p| [high(p.r), high(p.g), high(p.b), 255])
                .collect();
            (w, h, rgba)
        }
        avif_decode::Image::Rgba16(img) => {
            let (w, h) = dims(img.width(), img.height())?;
            let rgba = img
                .pixels()
                .flat_map(|p| [high(p.r), high(p.g), high(p.b), high(p.a)])
                .collect();
            (w, h, rgba)
        }
        avif_decode::Image::Gray8(img) => {
            let (w, h) = dims(img.width(), img.height())?;
            let rgba = img
                .pixels()
                .flat_map(|p| {
                    let g = p.value();
                    [g, g, g, 255]
                })
                .collect();
            (w, h, rgba)
        }
        avif_decode::Image::Gray16(img) => {
            let (w, h) = dims(img.width(), img.height())?;
            let rgba = img
                .pixels()
                .flat_map(|p| {
                    let g = high(p.value());
                    [g, g, g, 255]
                })
                .collect();
            (w, h, rgba)
        }
    };
    upright(width, height, rgba)
}

/// The high byte of a 16-bit sample.
fn high(sample: u16) -> u8 {
    sample.to_be_bytes()[0]
}

/// BMP and ICO with `image` (only its bmp and ico features; an icon's PNG
/// entries go through `png`), within `image`'s own limits.
fn image(bytes: &[u8], format: ::image::ImageFormat) -> Result<Raster, Refusal> {
    let mut reader = ::image::ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = ::image::Limits::default();
    limits.max_image_width = Some(MAX_SIDE);
    limits.max_image_height = Some(MAX_SIDE);
    limits.max_alloc = Some(MEMORY);
    reader.limits(limits);
    let decoded = reader.decode().map_err(|error| match error {
        ::image::ImageError::Limits(_) => Refusal::TooManyPixels,
        _ => Refusal::Malformed,
    })?;
    let rgba = decoded.into_rgba8();
    let (width, height) = rgba.dimensions();
    upright(width, height, rgba.into_raw())
}

/// The deepest nesting and the most elements an SVG may have; checked by a
/// scan of our own before `usvg` parses it, whose conversion recurses.
const SVG_DEPTH: usize = 128;
const SVG_ELEMENTS: usize = 20_000;

/// SVG with `resvg` (no text, no fonts, no svgz, no raster images): the
/// centred square of its own size, rasterised straight at 256 × 256. No
/// reference is resolved: images, fonts and files stay unloaded.
pub(crate) fn svg(bytes: &[u8]) -> Result<Raster, Refusal> {
    use resvg::{tiny_skia, usvg};
    // An entity declared anywhere, even where the scan would not look.
    if bytes.windows(8).any(|w| w == b"<!ENTITY") {
        return Err(Refusal::Unsupported);
    }
    shallow(bytes)?;
    let options = usvg::Options {
        resources_dir: None,
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_data: Box::new(|_, _, _| None),
            resolve_string: Box::new(|_, _| None),
        },
        ..usvg::Options::default()
    };
    let tree = usvg::Tree::from_data(bytes, &options).map_err(|_| Refusal::Malformed)?;
    let size = tree.size();
    let shortest = size.width().min(size.height());
    if !shortest.is_finite() || shortest <= 0.0 {
        return Err(Refusal::Malformed);
    }
    let target = f32::from(u16::try_from(super::scale::SIDE).map_err(|_| Refusal::Failed)?);
    let scale = target / shortest;
    let transform = tiny_skia::Transform::from_row(
        scale,
        0.0,
        0.0,
        scale,
        -(size.width() - shortest) / 2.0 * scale,
        -(size.height() - shortest) / 2.0 * scale,
    );
    let mut pixmap =
        tiny_skia::Pixmap::new(super::scale::SIDE, super::scale::SIDE).ok_or(Refusal::Failed)?;
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    // tiny-skia keeps premultiplied pixels.
    let rgba = pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect();
    upright(super::scale::SIDE, super::scale::SIDE, rgba)
}

/// Nesting and element count of `bytes`, by a scan that understands
/// comments, CDATA, processing instructions and declarations well enough
/// to skip them. Anything it cannot follow the parser refuses anyway.
fn shallow(bytes: &[u8]) -> Result<(), Refusal> {
    let text = std::str::from_utf8(bytes).map_err(|_| Refusal::Malformed)?;
    let mut depth: usize = 0;
    let mut elements: usize = 0;
    let mut rest = text;
    while let Some(at) = rest.find('<') {
        rest = &rest[at..];
        let skip_to = |rest: &str, end: &str| rest.find(end).map(|i| i + end.len());
        let advance = if rest.starts_with("<!--") {
            skip_to(rest, "-->")
        } else if rest.starts_with("<![CDATA[") {
            skip_to(rest, "]]>")
        } else if rest.starts_with("<?") {
            skip_to(rest, "?>")
        } else if rest.starts_with("<!") {
            // A document type: an internal subset could declare entities,
            // whose expansion no element or depth count here would see (a
            // 25 KB file took 548 MiB in the review). None is needed for
            // a picture, so any subset is refused.
            match (rest.find('['), rest.find('>')) {
                (Some(open), Some(close)) if open < close => return Err(Refusal::Unsupported),
                _ => skip_to(rest, ">"),
            }
        } else if rest.starts_with("</") {
            depth = depth.saturating_sub(1);
            skip_to(rest, ">")
        } else {
            let end = rest.find('>').ok_or(Refusal::Malformed)?;
            elements += 1;
            if !rest[..end].ends_with('/') {
                depth += 1;
            }
            if depth > SVG_DEPTH || elements > SVG_ELEMENTS {
                return Err(Refusal::TooManyPixels);
            }
            Some(end + 1)
        };
        rest = &rest[advance.ok_or(Refusal::Malformed)?..];
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_svg_scan_bounds_depth_and_count() {
        let nested = |n: usize| format!("<svg>{}{}</svg>", "<g>".repeat(n), "</g>".repeat(n));
        assert_eq!(shallow(nested(100).as_bytes()), Ok(()));
        assert_eq!(shallow(nested(200).as_bytes()), Err(Refusal::TooManyPixels));
        let flat = format!("<svg>{}</svg>", "<rect/>".repeat(SVG_ELEMENTS + 1));
        assert_eq!(shallow(flat.as_bytes()), Err(Refusal::TooManyPixels));
        let flat = format!("<svg>{}</svg>", "<rect/>".repeat(1000));
        assert_eq!(shallow(flat.as_bytes()), Ok(()));
        // comments and CDATA are not elements
        let hidden = format!(
            "<svg><!--{}--><![CDATA[{}]]></svg>",
            "<g>".repeat(500),
            "<g>".repeat(500)
        );
        assert_eq!(shallow(hidden.as_bytes()), Ok(()));
        assert_eq!(shallow(b"<svg><g"), Err(Refusal::Malformed));
        assert_eq!(shallow(b"<svg><!-- open"), Err(Refusal::Malformed));
        assert_eq!(shallow(b"\xff<svg/>"), Err(Refusal::Malformed));
    }

    #[test]
    fn channels_become_rgba() {
        assert_eq!(to_rgba(vec![7], 1).unwrap(), [7, 7, 7, 255]);
        assert_eq!(to_rgba(vec![7, 9], 2).unwrap(), [7, 7, 7, 9]);
        assert_eq!(to_rgba(vec![1, 2, 3], 3).unwrap(), [1, 2, 3, 255]);
        assert_eq!(to_rgba(vec![1, 2, 3, 4], 4).unwrap(), [1, 2, 3, 4]);
        assert_eq!(high(0xabcd), 0xab);
    }
}
