//! Animated pictures (`docs/dependency-proposal/profile-picture.md` §0.4):
//! the frames of an animated GIF, WebP or APNG, decoded one by one within
//! bounds, composited by each format's own disposal and blending rules on
//! one canvas, and each reduced to the 256 px square at once, so no more
//! than one canvas (and, for "restore to previous", its copy) is ever held.
//!
//! Every bound stops the decoding instead of refusing: a picture of more
//! frames, or a longer play, keeps its beginning. A file whose frames
//! cannot be read, or that has one frame, is no animation; the still path
//! then decides about it.
use super::{Format, Refusal, scale};
use std::io::Cursor;

/// Most frames kept: the bound of every stored sequence.
pub const MAX_FRAMES: usize = central_picture::MAX_FRAMES;
/// Longest play kept, in milliseconds.
pub const MAX_PLAY_MS: u32 = 10_000;
/// Largest canvas an animation may have; a larger one is taken as a still.
pub const MAX_CANVAS: u64 = 2_048 * 2_048;
/// Most canvas pixels composited and averaged over all frames: the work an
/// animation may cause.
pub const MAX_COMPOSITED: u64 = 200_000_000;
/// A frame shorter than this plays for [`SLOW_MS`], as browsers do.
pub const MIN_DELAY_MS: u32 = 20;
/// See [`MIN_DELAY_MS`].
pub const SLOW_MS: u32 = 100;

/// One frame: 256 × 256 RGBA, upright, and how long it shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    /// The pixels.
    pub rgba: Vec<u8>,
    /// Milliseconds.
    pub delay_ms: u32,
}

/// An animation's frames and how often it plays (0: for ever).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Animation {
    /// At least two frames.
    pub frames: Vec<Frame>,
    /// How often the whole plays; 0 for ever.
    pub plays: u32,
}

/// The frames, as the bounds let them be collected.
struct Collector {
    width: u32,
    height: u32,
    frames: Vec<Frame>,
    played: u32,
    composited: u64,
}

impl Collector {
    /// `None` for a canvas larger than [`MAX_CANVAS`] or empty.
    fn new(width: u32, height: u32) -> Option<Self> {
        let pixels = u64::from(width) * u64::from(height);
        (pixels > 0 && pixels <= MAX_CANVAS).then_some(Collector {
            width,
            height,
            frames: Vec::new(),
            played: 0,
            composited: 0,
        })
    }

    fn pixels(&self) -> usize {
        usize::try_from(u64::from(self.width) * u64::from(self.height)).unwrap_or(usize::MAX)
    }

    /// Keeps `canvas` as the next frame; `false` once a bound says stop.
    fn push(&mut self, canvas: &[u8], delay_ms: u32) -> Result<bool, Refusal> {
        let cost = u64::from(self.width) * u64::from(self.height);
        if self.frames.len() >= MAX_FRAMES
            || self.played >= MAX_PLAY_MS
            || self.composited + cost > MAX_COMPOSITED
        {
            return Ok(false);
        }
        let delay = if delay_ms < MIN_DELAY_MS {
            SLOW_MS
        } else {
            delay_ms
        };
        let delay = delay.min(MAX_PLAY_MS - self.played);
        let square = scale::square_of(self.width, self.height, canvas, 1)?;
        self.frames.push(Frame {
            rgba: square.0,
            delay_ms: delay,
        });
        self.played += delay;
        self.composited += cost;
        Ok(true)
    }

    fn finish(self, plays: u32) -> Option<Animation> {
        (self.frames.len() >= 2).then_some(Animation {
            frames: self.frames,
            plays,
        })
    }
}

/// The animation in `bytes` of `format`, if it is one within the bounds.
///
/// # Errors
/// `Failed` only when a frame cannot be averaged (never for a file's
/// content); every problem of the file itself ends the animation instead.
pub fn frames(format: Format, bytes: &[u8]) -> Result<Option<Animation>, Refusal> {
    match format {
        Format::Gif => gif(bytes),
        Format::Webp => webp(bytes),
        Format::Png => apng(bytes),
        _ => Ok(None),
    }
}

/// The rectangle `(left, top, width, height)` of `frame_width` ×
/// `frame_height` inside a `width` × `height` canvas, clipped to it.
fn clipped(
    (left, top, frame_width, frame_height): (u32, u32, u32, u32),
    width: u32,
    height: u32,
) -> (usize, usize, usize, usize) {
    let to = |v: u32| usize::try_from(v).unwrap_or(usize::MAX);
    let w = frame_width.min(width.saturating_sub(left));
    let h = frame_height.min(height.saturating_sub(top));
    (to(left.min(width)), to(top.min(height)), to(w), to(h))
}

/// Clears the rectangle to transparent black.
fn clear(canvas: &mut [u8], width: usize, (x, y, w, h): (usize, usize, usize, usize)) {
    for row in y..y + h {
        let start = (row * width + x) * 4;
        if let Some(line) = canvas.get_mut(start..start + w * 4) {
            line.fill(0);
        }
    }
}

/// GIF with `gif`: each frame drawn where it says, its transparent pixels
/// leaving the canvas as it was; then disposed: kept, cleared to
/// transparent, or restored to the canvas before it.
fn gif(bytes: &[u8]) -> Result<Option<Animation>, Refusal> {
    let mut options = gif::DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::RGBA);
    options.set_memory_limit(gif::MemoryLimit::Bytes(
        std::num::NonZeroU64::new(MAX_CANVAS * 4).ok_or(Refusal::Failed)?,
    ));
    let Ok(mut decoder) = options.read_info(Cursor::new(bytes)) else {
        return Ok(None);
    };
    let (width, height) = (u32::from(decoder.width()), u32::from(decoder.height()));
    let Some(mut collector) = Collector::new(width, height) else {
        return Ok(None);
    };
    let screen = usize::try_from(width).map_err(|_| Refusal::Failed)?;
    let mut canvas = vec![0_u8; collector.pixels() * 4];
    while let Ok(Some(frame)) = decoder.read_next_frame() {
        let (fw, fh) = (u32::from(frame.width), u32::from(frame.height));
        if frame.buffer.len() != usize::try_from(fw * fh * 4).unwrap_or(0) {
            break;
        }
        let area = clipped(
            (u32::from(frame.left), u32::from(frame.top), fw, fh),
            width,
            height,
        );
        let previous = (frame.dispose == gif::DisposalMethod::Previous).then(|| canvas.clone());
        let (x, y, w, h) = area;
        let source_width = usize::try_from(fw).unwrap_or(0);
        for row in 0..h {
            for column in 0..w {
                let from = (row * source_width + column) * 4;
                let pixel = &frame.buffer[from..from + 4];
                if pixel[3] != 0 {
                    let to = ((y + row) * screen + x + column) * 4;
                    canvas[to..to + 4].copy_from_slice(pixel);
                }
            }
        }
        let delay = u32::from(frame.delay) * 10;
        let dispose = frame.dispose;
        if !collector.push(&canvas, delay)? {
            break;
        }
        match (dispose, previous) {
            (gif::DisposalMethod::Background, _) => clear(&mut canvas, screen, area),
            (gif::DisposalMethod::Previous, Some(before)) => canvas = before,
            _ => {}
        }
    }
    // `Finite(0)` is a GIF without a loop extension: it plays once.
    let plays = match decoder.repeat() {
        gif::Repeat::Infinite => 0,
        gif::Repeat::Finite(n) => u32::from(n).saturating_add(1),
    };
    Ok(collector.finish(plays))
}

/// WebP with `image-webp`, which composites its frames itself (blending
/// and disposal onto its canvas): each frame comes whole.
fn webp(bytes: &[u8]) -> Result<Option<Animation>, Refusal> {
    let Ok(mut decoder) = image_webp::WebPDecoder::new(Cursor::new(bytes)) else {
        return Ok(None);
    };
    if !decoder.is_animated() {
        return Ok(None);
    }
    decoder.set_memory_limit(usize::try_from(MAX_CANVAS * 4 * 2).unwrap_or(usize::MAX));
    let (width, height) = decoder.dimensions();
    let Some(mut collector) = Collector::new(width, height) else {
        return Ok(None);
    };
    let Some(size) = decoder.output_buffer_size() else {
        return Ok(None);
    };
    let alpha = decoder.has_alpha();
    let plays = match decoder.loop_count() {
        image_webp::LoopCount::Forever => 0,
        image_webp::LoopCount::Times(n) => u32::from(n.get()),
    };
    let mut buffer = vec![0_u8; size];
    for _ in 0..decoder.num_frames() {
        let Ok(delay) = decoder.read_frame(&mut buffer) else {
            break;
        };
        let keep = if alpha {
            collector.push(&buffer, delay)?
        } else {
            let rgba: Vec<u8> = buffer
                .as_chunks::<3>()
                .0
                .iter()
                .flat_map(|p| [p[0], p[1], p[2], 255])
                .collect();
            collector.push(&rgba, delay)?
        };
        if !keep {
            break;
        }
    }
    Ok(collector.finish(plays))
}

/// One straight-alpha pixel `source` over `target` (APNG's `OVER`).
fn over(target: &mut [u8], source: &[u8]) {
    let sa = u32::from(source[3]);
    if sa == 255 {
        target.copy_from_slice(source);
        return;
    }
    if sa == 0 {
        return;
    }
    let da = u32::from(target[3]);
    let inverse = 255 - sa;
    // Both weights scaled by 255.
    let weight = sa * 255 + da * inverse;
    for c in 0..3 {
        let mixed = u32::from(source[c]) * sa * 255 + u32::from(target[c]) * da * inverse;
        target[c] = u8::try_from((mixed + weight / 2) / weight).unwrap_or(u8::MAX);
    }
    target[3] = u8::try_from((weight + 127) / 255).unwrap_or(u8::MAX);
}

/// One decoded APNG frame as `png` gives it: rows of `line` bytes, grey and
/// alpha (2 channels) or RGBA (4), and how it meets the canvas.
struct Part<'a> {
    pixels: &'a [u8],
    line: usize,
    channels: usize,
    blend: png::BlendOp,
}

/// Lays `part` on `canvas` (`screen` pixels wide) at `area`.
fn draw(canvas: &mut [u8], screen: usize, area: (usize, usize, usize, usize), part: &Part<'_>) {
    let (x, y, w, h) = area;
    for row in 0..h {
        let line = part.pixels.get(row * part.line..).unwrap_or_default();
        for column in 0..w {
            let pixel: [u8; 4] = if part.channels == 4 {
                let rgba = &line[column * 4..column * 4 + 4];
                [rgba[0], rgba[1], rgba[2], rgba[3]]
            } else {
                let grey = &line[column * 2..column * 2 + 2];
                [grey[0], grey[0], grey[0], grey[1]]
            };
            let to = ((y + row) * screen + x + column) * 4;
            let target = &mut canvas[to..to + 4];
            match part.blend {
                png::BlendOp::Source => target.copy_from_slice(&pixel),
                png::BlendOp::Over => over(target, &pixel),
            }
        }
    }
}

/// APNG with `png`: the frames of its `fcTL` chunks (the default image only
/// when it is one of them), each a part of the canvas that replaces
/// (`SOURCE`) or is laid over (`OVER`) what is there, then disposed: kept,
/// cleared, or restored (a first frame's "restore" clears, as the
/// specification says).
fn apng(bytes: &[u8]) -> Result<Option<Animation>, Refusal> {
    let mut decoder = png::Decoder::new_with_limits(
        Cursor::new(bytes),
        png::Limits {
            bytes: usize::try_from(MAX_CANVAS * 4 * 2).unwrap_or(usize::MAX),
        },
    );
    decoder.ignore_checksums(false);
    decoder.set_ignore_text_chunk(true);
    decoder.set_ignore_iccp_chunk(true);
    decoder.set_transformations(
        png::Transformations::normalize_to_color8() | png::Transformations::ALPHA,
    );
    let Ok(mut reader) = decoder.read_info() else {
        return Ok(None);
    };
    let Some(control) = reader.info().animation_control else {
        return Ok(None);
    };
    if control.num_frames < 2 {
        return Ok(None);
    }
    let (width, height) = (reader.info().width, reader.info().height);
    let Some(mut collector) = Collector::new(width, height) else {
        return Ok(None);
    };
    let default_is_frame = reader.info().frame_control.is_some();
    let Some(size) = reader.output_buffer_size() else {
        return Ok(None);
    };
    let screen = usize::try_from(width).map_err(|_| Refusal::Failed)?;
    let mut buffer = vec![0_u8; size];
    let mut canvas = vec![0_u8; collector.pixels() * 4];
    let mut skip_default = !default_is_frame;
    while let Ok(output) = reader.next_frame(&mut buffer) {
        if skip_default {
            skip_default = false;
            continue;
        }
        let Some(control) = reader.info().frame_control else {
            break;
        };
        let channels = match output.color_type {
            png::ColorType::GrayscaleAlpha => 2,
            png::ColorType::Rgba => 4,
            _ => break,
        };
        if output.bit_depth != png::BitDepth::Eight
            || (control.width, control.height) != (output.width, output.height)
            || u64::from(control.x_offset) + u64::from(control.width) > u64::from(width)
            || u64::from(control.y_offset) + u64::from(control.height) > u64::from(height)
        {
            break;
        }
        let area = clipped(
            (
                control.x_offset,
                control.y_offset,
                control.width,
                control.height,
            ),
            width,
            height,
        );
        let first = collector.frames.is_empty();
        let dispose = match control.dispose_op {
            png::DisposeOp::Previous if first => png::DisposeOp::Background,
            other => other,
        };
        let previous = (dispose == png::DisposeOp::Previous).then(|| canvas.clone());
        let part = Part {
            pixels: &buffer,
            line: output.line_size,
            channels,
            blend: control.blend_op,
        };
        draw(&mut canvas, screen, area, &part);
        let denominator = if control.delay_den == 0 {
            100
        } else {
            u32::from(control.delay_den)
        };
        let delay = u32::from(control.delay_num) * 1_000 / denominator;
        if !collector.push(&canvas, delay)? {
            break;
        }
        match (dispose, previous) {
            (png::DisposeOp::Background, _) => clear(&mut canvas, screen, area),
            (png::DisposeOp::Previous, Some(before)) => canvas = before,
            _ => {}
        }
    }
    Ok(collector.finish(control.num_plays))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn over_mixes_straight_alpha() {
        let mut target = [0, 0, 255, 255];
        over(&mut target, &[255, 0, 0, 128]);
        assert_eq!(target, [128, 0, 127, 255]);
        let mut empty = [0, 0, 0, 0];
        over(&mut empty, &[10, 20, 30, 40]);
        assert_eq!(empty, [10, 20, 30, 40]);
        let mut kept = [1, 2, 3, 4];
        over(&mut kept, &[9, 9, 9, 0]);
        assert_eq!(kept, [1, 2, 3, 4]);
    }

    #[test]
    fn rectangles_are_clipped_to_the_canvas() {
        assert_eq!(clipped((10, 10, 100, 100), 50, 40), (10, 10, 40, 30));
        assert_eq!(clipped((60, 0, 10, 10), 50, 40), (50, 0, 0, 10));
        assert_eq!(clipped((0, 0, 0, 0), 50, 40), (0, 0, 0, 0));
    }

    #[test]
    fn the_collector_keeps_its_bounds() {
        assert!(Collector::new(0, 10).is_none());
        assert!(Collector::new(2_049, 2_048).is_none());
        let mut c = Collector::new(16, 16).unwrap();
        let canvas = vec![255; 16 * 16 * 4];
        for _ in 0..MAX_FRAMES {
            assert!(c.push(&canvas, 0).unwrap());
        }
        assert!(!c.push(&canvas, 0).unwrap());
        // short delays play slow
        assert!(c.frames.iter().all(|f| f.delay_ms == SLOW_MS));
        let mut long = Collector::new(16, 16).unwrap();
        assert!(long.push(&canvas, 9_000).unwrap());
        assert!(long.push(&canvas, 9_000).unwrap());
        assert!(!long.push(&canvas, 9_000).unwrap());
        assert_eq!(long.played, MAX_PLAY_MS);
        assert_eq!(long.frames[1].delay_ms, 1_000);
        let one = Collector::new(16, 16).unwrap();
        assert!(one.finish(0).is_none());
    }
}
