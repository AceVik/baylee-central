//! `picture-fuzz FORMAT SECONDS [SEED]`: seeded mutation fuzzing of one
//! decoder entry point of the picture worker (`docs/profile-pictures.md`),
//! for as long as asked. Not coverage-guided: libFuzzer would need a
//! foreign-function entry point, which this repository does not write
//! (AGENTS.md). It keeps every mutation that converts as a further seed, so
//! it wanders deeper than the seeds alone.
//!
//! Each case must end, within the bound, in a refusal or a pair that passes
//! the service's own checks; anything else is written to `findings/` and
//! counted. AVIF goes through the worker binary (its path in
//! `PICTURE_WORKER`), since `rav1d` may abort the process.
#![forbid(unsafe_code)]

use central_picture::{Format, Output, Refusal};
use std::process::ExitCode;
use std::time::{Duration, Instant};

const BOUND: Duration = Duration::from_secs(9);

struct Xorshift(u64);

impl Xorshift {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(n.max(1)).unwrap_or(1)).unwrap_or(0)
    }

    fn byte(&mut self) -> u8 {
        self.next().to_le_bytes()[0]
    }
}

fn mutate(rng: &mut Xorshift, seed: &[u8]) -> Vec<u8> {
    let mut bytes = seed.to_vec();
    for _ in 0..=rng.below(8) {
        if bytes.is_empty() {
            bytes.push(0);
        }
        let at = rng.below(bytes.len());
        match rng.below(7) {
            0 => bytes[at] ^= 1 << rng.below(8),
            1 => bytes[at] = rng.byte(),
            2 => bytes.insert(at, rng.byte()),
            3 if bytes.len() > 16 => {
                let end = (at + 1 + rng.below(32)).min(bytes.len());
                bytes.drain(at..end);
            }
            4 => {
                let value = [0_u32, 1, 0x7fff_ffff, u32::MAX, 0x0001_0000, 0x4000][rng.below(6)];
                let end = (at + 4).min(bytes.len());
                let len = end - at;
                bytes[at..end].copy_from_slice(&value.to_be_bytes()[..len]);
            }
            5 => {
                let len = rng.below(256).min(bytes.len() - at);
                let run = bytes[at..at + len].to_vec();
                let to = rng.below(bytes.len());
                bytes.splice(to..to, run);
            }
            _ => bytes.truncate(at.max(1)),
        }
    }
    bytes
}

/// PNG chunks with their CRCs made right, so damage reaches the inflater.
fn recrc(png: &[u8]) -> Vec<u8> {
    if !png.starts_with(b"\x89PNG\r\n\x1a\n") {
        return png.to_vec();
    }
    let mut out = png[..8].to_vec();
    let mut rest = &png[8..];
    while rest.len() >= 12 {
        let length = usize::try_from(u32::from_be_bytes([rest[0], rest[1], rest[2], rest[3]]))
            .unwrap_or(usize::MAX);
        if length > rest.len() - 12 {
            break;
        }
        let kind_and_data = &rest[4..8 + length];
        out.extend_from_slice(&rest[..8 + length]);
        out.extend_from_slice(&crc32(kind_and_data).to_be_bytes());
        rest = &rest[12 + length..];
    }
    out.extend_from_slice(rest);
    out
}

fn crc32(bytes: &[u8]) -> u32 {
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

fn quadrant(x: u32, y: u32, side: u32) -> [u8; 4] {
    if x < side / 2 && y < side / 2 {
        [255, 0, 0, 255]
    } else {
        [0, 64, 255, 200]
    }
}

/// Seeds drawn with the encoders in the graph (a two-frame GIF and APNG
/// among them), and the codec's committed JPEG and WebP fixtures, an
/// animated WebP among them.
fn seeds(format: Format) -> Vec<Vec<u8>> {
    let side = 40_u32;
    let rgba: Vec<u8> = (0..side * side)
        .flat_map(|i| quadrant(i % side, i / side, side))
        .collect();
    match format {
        Format::Png => {
            let mut out = Vec::new();
            {
                let mut encoder = png::Encoder::new(&mut out, side, side);
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                if let Ok(mut writer) = encoder.write_header() {
                    let _ = writer.write_image_data(&rgba);
                }
            }
            // an APNG of two frames, the second the first's bytes reversed
            let mut moving = Vec::new();
            {
                let mut encoder = png::Encoder::new(&mut moving, side, side);
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                if encoder.set_animated(2, 0).is_ok()
                    && let Ok(mut writer) = encoder.write_header()
                {
                    let _ = writer.write_image_data(&rgba);
                    let reversed: Vec<u8> = rgba.iter().rev().copied().collect();
                    let _ = writer.write_image_data(&reversed);
                }
            }
            vec![out, moving]
        }
        Format::Gif => {
            let palette = [255, 0, 0, 0, 0, 255];
            let mut out = Vec::new();
            {
                let side = u16::try_from(side).unwrap_or(40);
                if let Ok(mut encoder) = gif::Encoder::new(&mut out, side, side, &palette) {
                    for n in 0..2_u8 {
                        let frame = gif::Frame {
                            width: side,
                            height: side,
                            buffer: std::borrow::Cow::Owned(vec![n; usize::from(side) * usize::from(side)]),
                            ..gif::Frame::default()
                        };
                        let _ = encoder.write_frame(&frame);
                    }
                }
            }
            vec![out]
        }
        Format::Webp => {
            let mut out = Vec::new();
            let _ = image_webp::WebPEncoder::new(&mut out).encode(&rgba, side, side, image_webp::ColorType::Rgba8);
            let mut seeds = fixtures(&["lossy.webp", "animated.webp"]);
            seeds.push(out);
            seeds
        }
        Format::Avif => {
            let pixels: Vec<ravif::RGBA8> = rgba
                .as_chunks::<4>()
                .0
                .iter()
                .map(|&[r, g, b, a]| ravif::RGBA8::new(r, g, b, a))
                .collect();
            let side = usize::try_from(side).unwrap_or(40);
            ravif::Encoder::new()
                .with_speed(10)
                .with_num_threads(Some(1))
                .encode_rgba(ravif::Img::new(&pixels[..], side, side))
                .map(|encoded| vec![encoded.avif_file])
                .unwrap_or_default()
        }
        Format::Bmp | Format::Ico => {
            let picture = image::RgbaImage::from_raw(side, side, rgba).unwrap_or_default();
            let mut out = std::io::Cursor::new(Vec::new());
            let kind = if format == Format::Bmp { image::ImageFormat::Bmp } else { image::ImageFormat::Ico };
            let _ = picture.write_to(&mut out, kind);
            vec![out.into_inner()]
        }
        Format::Svg => vec![
            b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 40 40'><defs><linearGradient id='g'><stop offset='0' stop-color='red'/><stop offset='1' stop-color='blue'/></linearGradient><filter id='f'><feGaussianBlur stdDeviation='2'/></filter></defs><rect width='40' height='40' fill='url(#g)'/><circle cx='20' cy='20' r='9' filter='url(#f)' fill='#0f0' opacity='.5'/><path d='M0 0L40 40Q20 0 0 40Z' stroke='#000'/><g transform='rotate(30 20 20)'><use href='#c'/></g></svg>".to_vec(),
        ],
        Format::Jpeg => fixtures(&["quadrant.jpg", "progressive.jpg", "grey.jpg"]),
    }
}

/// The codec's committed fixtures of these names.
fn fixtures(names: &[&str]) -> Vec<Vec<u8>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../crates/central-picture-codec/tests/fixtures");
    names
        .iter()
        .filter_map(|name| std::fs::read(root.join(name)).ok())
        .collect()
}

fn convert(
    format: Format,
    bytes: &[u8],
    worker: Option<&central_picture::Worker>,
) -> Result<Output, Refusal> {
    match worker {
        Some(worker) => worker.convert(format, bytes).map(|c| Output {
            avif: c.avif,
            png: c.png,
            motion: c.motion,
        }),
        // uncaught, so a decoder's panic is a finding, not a refusal
        None => central_picture_codec::convert_uncaught(format, bytes),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let names = ["png", "jpeg", "gif", "webp", "avif", "svg", "bmp", "ico"];
    let Some(at) = args
        .get(1)
        .and_then(|name| names.iter().position(|n| n == name))
    else {
        eprintln!("usage: picture-fuzz {} SECONDS [SEED]", names.join("|"));
        return ExitCode::FAILURE;
    };
    let format = Format::ALL[at];
    let seconds: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(60);
    let seed: u64 = args
        .get(3)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0x5eed_0000 + u64::try_from(at).unwrap_or(0));
    let worker = (format == Format::Avif).then(|| {
        // no ffmpeg: what is fuzzed here is the decoders
        central_picture::Worker::program_with(
            std::env::var("PICTURE_WORKER")
                .unwrap_or_else(|_| "../target/release/central-picture-worker".into()),
            vec!["--no-ffmpeg".into()],
        )
    });
    std::panic::set_hook(Box::new(|_| {}));
    let mut corpus = seeds(format);
    if corpus.is_empty() {
        eprintln!("no seed for {}", names[at]);
        return ExitCode::FAILURE;
    }
    let mut rng = Xorshift(seed | 1);
    let (mut cases, mut accepted, mut findings) = (0_u64, 0_u64, 0_u64);
    let mut refused = std::collections::BTreeMap::<&str, u64>::new();
    let end = Instant::now() + Duration::from_secs(seconds);
    let place = std::env::var("PICTURE_FINDINGS").unwrap_or_else(|_| "findings".into());
    let _ = std::fs::create_dir_all(&place);
    while Instant::now() < end {
        let parent = corpus[rng.below(corpus.len())].clone();
        let mut bytes = mutate(&mut rng, &parent);
        if format == Format::Png {
            bytes = recrc(&bytes);
        }
        let started = Instant::now();
        let result = std::panic::catch_unwind(|| convert(format, &bytes, worker.as_ref()));
        let took = started.elapsed();
        cases += 1;
        let finding = match &result {
            Err(_) => Some("panic"),
            Ok(_) if took > BOUND => Some("slow"),
            Ok(Ok(output))
                if !central_picture::png_master(&output.png)
                    || !central_picture::avif_framing(&output.avif) =>
            {
                Some("invalid")
            }
            Ok(Ok(_)) => {
                accepted += 1;
                if corpus.len() < 256 {
                    corpus.push(bytes.clone());
                }
                None
            }
            Ok(Err(refusal)) => {
                *refused.entry(refusal.as_str()).or_default() += 1;
                None
            }
        };
        if let Some(kind) = finding {
            findings += 1;
            let _ = std::fs::write(format!("{place}/{}-{kind}-{cases}.bin", names[at]), &bytes);
        }
    }
    println!(
        "{}: {cases} cases in {seconds} s, {accepted} converted, refused {refused:?}, {findings} findings, corpus {}",
        names[at],
        corpus.len()
    );
    if findings == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
