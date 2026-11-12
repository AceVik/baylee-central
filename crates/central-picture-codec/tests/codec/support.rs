//! Builders of synthetic pictures, valid and hostile, for the codec's tests.
//! Nothing here comes from a person: every file is drawn by these functions
//! or by the tools named in `fixtures/README.md` from drawn pixels.
use central_picture::{Format, Output, Refusal};
use std::borrow::Cow;
use std::time::{Duration, Instant};

/// How long one conversion may take in these tests: well inside the
/// worker's ten seconds, on a loaded machine too.
pub(crate) const BOUND: Duration = Duration::from_secs(8);

/// Convert, with the time bound checked.
pub(crate) fn timed(format: Format, bytes: &[u8]) -> Result<Output, Refusal> {
    let started = Instant::now();
    let result = central_picture_codec::convert(format, bytes);
    let took = started.elapsed();
    assert!(took < BOUND, "{format:?} took {took:?}");
    result
}

/// A converted pair that passes the service's checks, decoded back.
pub(crate) fn valid(output: &Output) -> Vec<u8> {
    assert!(central_picture::png_master(&output.png), "png master");
    assert!(central_picture::avif_framing(&output.avif), "avif framing");
    let (width, height, rgba) = decode_png(&output.png);
    assert_eq!((width, height), (256, 256));
    rgba
}

/// A PNG as 8-bit RGBA.
pub(crate) fn decode_png(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(
        png::Transformations::normalize_to_color8() | png::Transformations::ALPHA,
    );
    let mut reader = decoder.read_info().unwrap();
    let mut buffer = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buffer).unwrap();
    buffer.truncate(info.buffer_size());
    let rgba = match info.color_type {
        png::ColorType::Rgba => buffer,
        png::ColorType::GrayscaleAlpha => buffer
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        other => panic!("{other:?}"),
    };
    (info.width, info.height, rgba)
}

/// The RGBA pixel at (x, y) of a 256² picture.
pub(crate) fn at(rgba: &[u8], x: usize, y: usize) -> [u8; 4] {
    rgba[(y * 256 + x) * 4..][..4].try_into().unwrap()
}

/// Two colours are within `tolerance` per channel (lossy formats).
pub(crate) fn near(a: [u8; 4], b: [u8; 4], tolerance: u8) -> bool {
    a.iter().zip(b).all(|(x, y)| x.abs_diff(y) <= tolerance)
}

/// CRC-32 (ISO 3309), for chunks built by hand.
pub(crate) fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                0xedb8_8320 ^ (crc >> 1)
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// A PNG chunk with its CRC.
pub(crate) fn chunk(kind: &[u8], data: &[u8]) -> Vec<u8> {
    let mut out = u32::try_from(data.len()).unwrap().to_be_bytes().to_vec();
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let crc = crc32(&out[4..]);
    out.extend_from_slice(&crc.to_be_bytes());
    out
}

/// The chunks of a PNG, each as (type, data).
pub(crate) fn chunks(png: &[u8]) -> Vec<([u8; 4], Vec<u8>)> {
    let mut rest = &png[8..];
    let mut out = Vec::new();
    while rest.len() >= 12 {
        let length = usize::try_from(u32::from_be_bytes(rest[..4].try_into().unwrap())).unwrap();
        out.push((rest[4..8].try_into().unwrap(), rest[8..8 + length].to_vec()));
        rest = &rest[12 + length..];
    }
    out
}

/// A PNG from chunks.
pub(crate) fn assemble(chunks: &[Vec<u8>]) -> Vec<u8> {
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    for c in chunks {
        out.extend_from_slice(c);
    }
    out
}

/// IHDR data.
pub(crate) fn ihdr(width: u32, height: u32, depth: u8, colour: u8, interlace: u8) -> Vec<u8> {
    let mut data = width.to_be_bytes().to_vec();
    data.extend_from_slice(&height.to_be_bytes());
    data.extend_from_slice(&[depth, colour, 0, 0, interlace]);
    data
}

/// A PNG of `width` × `height` drawn by `pixel`, in the given colour type
/// and depth (samples of `pixel` are taken as 8-bit RGBA and reduced).
pub(crate) fn png_of(
    width: u32,
    height: u32,
    colour: png::ColorType,
    depth: png::BitDepth,
    interlaced: bool,
    pixel: impl Fn(u32, u32) -> [u8; 4],
) -> Vec<u8> {
    let bits = match depth {
        png::BitDepth::One => 1,
        png::BitDepth::Two => 2,
        png::BitDepth::Four => 4,
        png::BitDepth::Eight => 8,
        png::BitDepth::Sixteen => 16,
    };
    let mut rows = Vec::new();
    for y in 0..height {
        let mut row_bits: Vec<(u16, u8)> = Vec::new();
        for x in 0..width {
            let [r, g, b, a] = pixel(x, y);
            let scale = |v: u8| -> u16 {
                if bits == 16 {
                    u16::from(v) * 257
                } else {
                    u16::from(v) >> (8 - bits)
                }
            };
            let samples: Vec<u8> = match colour {
                // grey is the red value, and so is a palette index
                png::ColorType::Grayscale | png::ColorType::Indexed => vec![r],
                png::ColorType::GrayscaleAlpha => vec![r, a],
                png::ColorType::Rgb => vec![r, g, b],
                png::ColorType::Rgba => vec![r, g, b, a],
            };
            for s in samples {
                let value = if matches!(colour, png::ColorType::Indexed) {
                    u16::from(s) & ((1 << bits) - 1)
                } else {
                    scale(s)
                };
                row_bits.push((value, bits));
            }
        }
        let mut row = Vec::new();
        let (mut acc, mut filled) = (0_u32, 0_u8);
        for (value, n) in row_bits {
            if n == 16 {
                row.extend_from_slice(&value.to_be_bytes());
                continue;
            }
            acc = (acc << n) | u32::from(value);
            filled += n;
            if filled == 8 {
                row.push(u8::try_from(acc).unwrap());
                acc = 0;
                filled = 0;
            }
        }
        if filled > 0 {
            row.push(u8::try_from(acc << (8 - filled)).unwrap());
        }
        rows.extend_from_slice(&row);
    }
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(colour);
        encoder.set_depth(depth);
        if matches!(colour, png::ColorType::Indexed) {
            let palette: Vec<u8> = (0..=255_u8).flat_map(|i| [i, 255 - i, i / 2]).collect();
            let entries = 1usize << bits;
            encoder.set_palette(palette[..entries * 3].to_vec());
        }
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&rows).unwrap();
        writer.finish().unwrap();
    }
    if interlaced {
        // The png encoder writes no interlaced file; this one is made by
        // the decoder-independent Adam7 writer below.
        return adam7(width, height, colour, bits, &out);
    }
    out
}

/// An interlaced copy of a non-interlaced 8-bit PNG (filter 0 on every
/// pass row), for the interlace test.
fn adam7(width: u32, height: u32, colour: png::ColorType, bits: u8, plain: &[u8]) -> Vec<u8> {
    assert_eq!(bits, 8, "only 8-bit interlaced test files");
    let channels = match colour {
        png::ColorType::Grayscale | png::ColorType::Indexed => 1,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
    };
    let (_, _, rgba) = {
        let mut decoder = png::Decoder::new(std::io::Cursor::new(plain));
        decoder.set_transformations(png::Transformations::IDENTITY);
        let mut reader = decoder.read_info().unwrap();
        let mut buffer = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut buffer).unwrap();
        buffer.truncate(info.buffer_size());
        (info.width, info.height, buffer)
    };
    let passes = [
        (0, 0, 8, 8),
        (4, 0, 8, 8),
        (0, 4, 4, 8),
        (2, 0, 4, 4),
        (0, 2, 2, 4),
        (1, 0, 2, 2),
        (0, 1, 1, 2),
    ];
    let mut raw = Vec::new();
    for (x0, y0, dx, dy) in passes {
        for y in (y0..height).step_by(dy) {
            let xs: Vec<u32> = (x0..width).step_by(dx).collect();
            if xs.is_empty() {
                continue;
            }
            raw.push(0);
            for x in xs {
                let at = usize::try_from((y * width + x) * channels).unwrap();
                raw.extend_from_slice(&rgba[at..at + usize::try_from(channels).unwrap()]);
            }
        }
    }
    let colour_byte = match colour {
        png::ColorType::Grayscale => 0,
        png::ColorType::Rgb => 2,
        png::ColorType::Indexed => 3,
        png::ColorType::GrayscaleAlpha => 4,
        png::ColorType::Rgba => 6,
    };
    let mut kept: Vec<Vec<u8>> = vec![chunk(b"IHDR", &ihdr(width, height, 8, colour_byte, 1))];
    for (kind, data) in chunks(plain) {
        if &kind == b"PLTE" || &kind == b"tRNS" {
            kept.push(chunk(&kind, &data));
        }
    }
    kept.push(chunk(b"IDAT", &zlib_stored(&raw)));
    kept.push(chunk(b"IEND", b""));
    assemble(&kept)
}

/// A zlib stream of stored (uncompressed) blocks, with its Adler-32.
pub(crate) fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = if data.is_empty() {
        vec![&[]]
    } else {
        data.chunks(65_535).collect()
    };
    for (i, block) in blocks.iter().enumerate() {
        out.push(u8::from(i + 1 == blocks.len()));
        let length = u16::try_from(block.len()).unwrap();
        out.extend_from_slice(&length.to_le_bytes());
        out.extend_from_slice(&(!length).to_le_bytes());
        out.extend_from_slice(block);
    }
    let (mut a, mut b) = (1_u32, 0_u32);
    for &byte in data {
        a = (a + u32::from(byte)) % 65_521;
        b = (b + a) % 65_521;
    }
    out.extend_from_slice(&((b << 16) | a).to_be_bytes());
    out
}

/// A zlib stream that inflates to `rows` × (`row` + 1) zero bytes, made by
/// the png encoder's own compressor from a grey picture of zeros, fed row
/// by row so the picture never exists in memory here.
pub(crate) fn zero_bomb(row: u32, rows: u32) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, row, rows);
        encoder.set_color(png::ColorType::Grayscale);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::High);
        let mut writer = encoder.write_header().unwrap();
        let mut stream = writer.stream_writer().unwrap();
        let zeros = vec![0; usize::try_from(row).unwrap()];
        for _ in 0..rows {
            std::io::Write::write_all(&mut stream, &zeros).unwrap();
        }
        stream.finish().unwrap();
    }
    chunks(&out)
        .into_iter()
        .filter(|(kind, _)| kind == b"IDAT")
        .flat_map(|(_, data)| data)
        .collect()
}

/// A GIF of `frames` frames on a `width` × `height` screen, frame n filled
/// with palette colour n (0 red, 1 blue, 2 green).
pub(crate) fn gif_of(width: u16, height: u16, frames: usize) -> Vec<u8> {
    let palette = vec![255, 0, 0, 0, 0, 255, 0, 255, 0, 0, 0, 0];
    let mut out = Vec::new();
    {
        let mut encoder = gif::Encoder::new(&mut out, width, height, &palette).unwrap();
        for n in 0..frames {
            let index = u8::try_from(n % 3).unwrap();
            let frame = gif::Frame {
                width,
                height,
                buffer: Cow::Owned(vec![index; usize::from(width) * usize::from(height)]),
                delay: 10,
                ..gif::Frame::default()
            };
            encoder.write_frame(&frame).unwrap();
        }
    }
    out
}

/// A lossless WebP drawn by `pixel`.
pub(crate) fn webp_of(width: u32, height: u32, pixel: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
    let mut rgba = Vec::new();
    for y in 0..height {
        for x in 0..width {
            rgba.extend_from_slice(&pixel(x, y));
        }
    }
    let mut out = Vec::new();
    image_webp::WebPEncoder::new(&mut out)
        .encode(&rgba, width, height, image_webp::ColorType::Rgba8)
        .unwrap();
    out
}

/// An AVIF made by the same encoder Central writes with.
pub(crate) fn avif_of(
    width: usize,
    height: usize,
    pixel: impl Fn(usize, usize) -> [u8; 4],
) -> Vec<u8> {
    let mut pixels = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let [r, g, b, a] = pixel(x, y);
            pixels.push(ravif::RGBA8::new(r, g, b, a));
        }
    }
    ravif::Encoder::new()
        .with_quality(90.0)
        .with_speed(10)
        .with_num_threads(Some(1))
        .encode_rgba(ravif::Img::new(&pixels[..], width, height))
        .unwrap()
        .avif_file
}

/// A BMP or ICO drawn by `pixel`, written by `image`'s own encoders.
pub(crate) fn image_of(
    format: image::ImageFormat,
    width: u32,
    height: u32,
    pixel: impl Fn(u32, u32) -> [u8; 4],
) -> Vec<u8> {
    let picture = image::RgbaImage::from_fn(width, height, |x, y| image::Rgba(pixel(x, y)));
    let mut out = std::io::Cursor::new(Vec::new());
    picture.write_to(&mut out, format).unwrap();
    out.into_inner()
}

/// The quadrant picture: top-left quarter red, the rest blue.
pub(crate) fn quadrant(width: u32, height: u32) -> impl Fn(u32, u32) -> [u8; 4] {
    move |x, y| {
        if x < width / 2 && y < height / 2 {
            [255, 0, 0, 255]
        } else {
            [0, 0, 255, 255]
        }
    }
}

/// A committed fixture (see `fixtures/README.md`).
pub(crate) fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read(path).unwrap()
}

/// A fixed-seed xorshift: mutations that are the same on every run.
pub(crate) struct Xorshift(pub u64);

impl Xorshift {
    pub(crate) fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    pub(crate) fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(n.max(1)).unwrap()).unwrap()
    }
}

/// The worker binary with no ffmpeg: every animation keeps its still.
pub(crate) fn worker_without_ffmpeg() -> central_picture::Worker {
    central_picture::Worker::program_with(
        env!("CARGO_BIN_EXE_central-picture-worker"),
        vec!["--no-ffmpeg".into()],
    )
}

/// Convert through the worker binary, as the service does: a decoder that
/// aborts the process (rav1d 1.1.0 does on some damaged AV1, from inside its
/// C ABI, where a panic cannot unwind) ends in `Failed` and harms nothing.
pub(crate) fn through_worker(format: Format, bytes: &[u8]) -> Result<Output, Refusal> {
    let started = Instant::now();
    let result = worker_without_ffmpeg()
        .convert(format, bytes)
        .map(|converted| Output {
            avif: converted.avif,
            png: converted.png,
            motion: converted.motion,
        });
    let took = started.elapsed();
    assert!(
        took < central_picture::TIME + Duration::from_secs(3),
        "{format:?} took {took:?}"
    );
    result
}
