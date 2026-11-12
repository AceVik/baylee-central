//! Animated GIF, WebP and APNG (proposal §0.4): frames, disposal and
//! blending, loops, and the bounds against frame bombs, huge canvases of
//! tiny frames and zero delays; and that a picture's still is its
//! animation's first frame.
use crate::support::{BOUND, decode_png, fixture, timed, valid};
use central_picture::Format;
use central_picture_codec::{Animation, MAX_FRAMES, MAX_PLAY_MS, SLOW_MS, frames};
use std::borrow::Cow;
use std::time::Instant;

const RED: [u8; 4] = [255, 0, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];
const GREEN: [u8; 4] = [0, 255, 0, 255];
const CLEAR: [u8; 4] = [0, 0, 0, 0];
/// Palette: red, blue, green, and 3 the transparent index.
const PALETTE: [u8; 12] = [255, 0, 0, 0, 0, 255, 0, 255, 0, 0, 0, 0];

/// One GIF frame: its rectangle, the index it is filled with, its
/// disposal and its delay in hundredths.
struct Part {
    at: (u16, u16, u16, u16),
    fill: u8,
    dispose: gif::DisposalMethod,
    delay: u16,
}

fn gif_parts(width: u16, height: u16, parts: &[Part], repeat: Option<gif::Repeat>) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut encoder = gif::Encoder::new(&mut out, width, height, &PALETTE).unwrap();
        if let Some(repeat) = repeat {
            encoder.set_repeat(repeat).unwrap();
        }
        for part in parts {
            let (left, top, w, h) = part.at;
            let frame = gif::Frame {
                left,
                top,
                width: w,
                height: h,
                buffer: Cow::Owned(vec![part.fill; usize::from(w) * usize::from(h)]),
                dispose: part.dispose,
                delay: part.delay,
                transparent: Some(3),
                ..gif::Frame::default()
            };
            encoder.write_frame(&frame).unwrap();
        }
    }
    out
}

fn part(at: (u16, u16, u16, u16), fill: u8, dispose: gif::DisposalMethod) -> Part {
    Part {
        at,
        fill,
        dispose,
        delay: 10,
    }
}

/// The pixel at (x, y) of a 256² frame.
fn at(rgba: &[u8], x: usize, y: usize) -> [u8; 4] {
    rgba[(y * 256 + x) * 4..][..4].try_into().unwrap()
}

fn moving(format: Format, bytes: &[u8]) -> Animation {
    let started = Instant::now();
    let found = frames(format, bytes).unwrap().expect("an animation");
    assert!(started.elapsed() < BOUND);
    found
}

#[test]
fn gif_frames_are_drawn_and_disposed_as_the_format_says() {
    use gif::DisposalMethod::{Background, Keep, Previous};
    let file = gif_parts(
        64,
        64,
        &[
            part((0, 0, 64, 64), 0, Keep),
            // blue top-left quarter, cleared after it was shown
            part((0, 0, 32, 32), 1, Background),
            // green bottom-right quarter on what is left
            part((32, 32, 32, 32), 2, Keep),
            // blue everywhere, then restored to the canvas before it
            part((0, 0, 64, 64), 1, Previous),
            // a transparent frame: the canvas as it was
            part((0, 0, 64, 64), 3, Keep),
        ],
        None,
    );
    let a = moving(Format::Gif, &file);
    assert_eq!(a.frames.len(), 5);
    assert_eq!(a.plays, 1, "no loop extension: once");
    assert_eq!(at(&a.frames[0].rgba, 200, 200), RED);
    assert_eq!(at(&a.frames[1].rgba, 20, 20), BLUE);
    assert_eq!(at(&a.frames[1].rgba, 200, 200), RED);
    assert_eq!(
        at(&a.frames[2].rgba, 20, 20),
        CLEAR,
        "disposed to the background"
    );
    assert_eq!(at(&a.frames[2].rgba, 200, 200), GREEN);
    assert_eq!(at(&a.frames[3].rgba, 20, 20), BLUE);
    assert_eq!(
        at(&a.frames[4].rgba, 20, 20),
        CLEAR,
        "restored to before the blue"
    );
    assert_eq!(at(&a.frames[4].rgba, 200, 200), GREEN);
    assert!(a.frames.iter().all(|f| f.delay_ms == 100));
}

#[test]
fn gif_loops_and_short_delays_are_kept_as_browsers_play_them() {
    use gif::DisposalMethod::Keep;
    let parts = [
        Part {
            at: (0, 0, 16, 16),
            fill: 0,
            dispose: Keep,
            delay: 0,
        },
        Part {
            at: (0, 0, 16, 16),
            fill: 1,
            dispose: Keep,
            delay: 1,
        },
        Part {
            at: (0, 0, 16, 16),
            fill: 2,
            dispose: Keep,
            delay: 50,
        },
    ];
    let forever = moving(
        Format::Gif,
        &gif_parts(16, 16, &parts, Some(gif::Repeat::Infinite)),
    );
    assert_eq!(forever.plays, 0);
    let delays: Vec<u32> = forever.frames.iter().map(|f| f.delay_ms).collect();
    assert_eq!(delays, [SLOW_MS, SLOW_MS, 500]);
    let thrice = moving(
        Format::Gif,
        &gif_parts(16, 16, &parts, Some(gif::Repeat::Finite(2))),
    );
    assert_eq!(thrice.plays, 3);
    // one frame is no animation
    let still = gif_parts(16, 16, &parts[..1], None);
    assert!(frames(Format::Gif, &still).unwrap().is_none());
}

#[test]
fn frame_bombs_keep_their_beginning_within_the_bounds() {
    use gif::DisposalMethod::Keep;
    // a thousand 1×1 frames without delay
    let bomb: Vec<Part> = (0..1_000)
        .map(|n| Part {
            at: (0, 0, 1, 1),
            fill: u8::try_from(n % 3).unwrap(),
            dispose: Keep,
            delay: 0,
        })
        .collect();
    let a = moving(
        Format::Gif,
        &gif_parts(64, 64, &bomb, Some(gif::Repeat::Infinite)),
    );
    assert_eq!(a.frames.len(), MAX_FRAMES);
    assert!(a.frames.iter().map(|f| f.delay_ms).sum::<u32>() <= MAX_PLAY_MS);
    // a long play: frames of five seconds, cut at ten
    let long: Vec<Part> = (0..5)
        .map(|n| Part {
            at: (0, 0, 8, 8),
            fill: u8::try_from(n % 3).unwrap(),
            dispose: Keep,
            delay: 500,
        })
        .collect();
    let cut = moving(Format::Gif, &gif_parts(16, 16, &long, None));
    assert_eq!(cut.frames.len(), 2);
    assert_eq!(
        cut.frames.iter().map(|f| f.delay_ms).sum::<u32>(),
        MAX_PLAY_MS
    );
}

#[test]
fn huge_canvases_of_tiny_frames_are_bounded_by_the_work() {
    use gif::DisposalMethod::Keep;
    let tiny: Vec<Part> = (0..300)
        .map(|n| Part {
            at: (0, 0, 1, 1),
            fill: u8::try_from(n % 3).unwrap(),
            dispose: Keep,
            delay: 5,
        })
        .collect();
    // above the animation's canvas: a still
    let started = Instant::now();
    let wide = gif_parts(4_096, 2_048, &tiny, None);
    assert!(frames(Format::Gif, &wide).unwrap().is_none());
    assert!(started.elapsed() < BOUND);
    // the largest canvas: as many frames as 200 million composited pixels allow
    let large = moving(Format::Gif, &gif_parts(2_048, 2_048, &tiny, None));
    assert_eq!(large.frames.len(), 47);
    // and the whole conversion stays within the bound, as a still
    if let Ok(output) = timed(Format::Gif, &wide) {
        valid(&output);
    }
}

/// One APNG frame: its rectangle, colour, blending and disposal.
type ApngPart = ((u32, u32, u32, u32), [u8; 4], png::BlendOp, png::DisposeOp);

/// An APNG of 64 × 64 frames: `parts` of (rectangle, colour, blend, dispose).
fn apng(parts: &[ApngPart], separate_default: Option<[u8; 4]>) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, 64, 64);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .set_animated(u32::try_from(parts.len()).unwrap(), 0)
            .unwrap();
        if separate_default.is_some() {
            encoder.set_sep_def_img(true).unwrap();
        }
        let mut writer = encoder.write_header().unwrap();
        if let Some(colour) = separate_default {
            writer.write_image_data(&colour.repeat(64 * 64)).unwrap();
        }
        for &((x, y, w, h), colour, blend, dispose) in parts {
            writer.set_frame_dimension(w, h).unwrap();
            writer.set_frame_position(x, y).unwrap();
            writer.set_blend_op(blend).unwrap();
            writer.set_dispose_op(dispose).unwrap();
            writer.set_frame_delay(1, 4).unwrap();
            let pixels = colour.repeat(usize::try_from(w * h).unwrap());
            writer.write_image_data(&pixels).unwrap();
        }
        writer.finish().unwrap();
    }
    out
}

#[test]
fn apng_frames_blend_and_dispose_as_the_specification_says() {
    use png::{BlendOp::*, DisposeOp::*};
    let half_blue = [0, 0, 255, 128];
    let file = apng(
        &[
            ((0, 0, 64, 64), RED, Source, None),
            // half-transparent blue over the red's top-left quarter, cleared after
            ((0, 0, 32, 32), half_blue, Over, Background),
            // replaced outright in the bottom-right quarter, restored after
            ((32, 32, 32, 32), half_blue, Source, Previous),
            // nothing new: what the disposals left
            ((0, 0, 1, 1), [0, 0, 0, 0], Over, None),
        ],
        Option::None,
    );
    let a = moving(Format::Png, &file);
    assert_eq!(a.frames.len(), 4);
    assert_eq!(a.plays, 0);
    assert!(a.frames.iter().all(|f| f.delay_ms == 250));
    // half blue over red
    assert_eq!(at(&a.frames[1].rgba, 20, 20), [127, 0, 128, 255], "over");
    assert_eq!(at(&a.frames[2].rgba, 20, 20), CLEAR, "cleared");
    assert_eq!(at(&a.frames[2].rgba, 200, 200), half_blue, "source");
    assert_eq!(at(&a.frames[3].rgba, 200, 200), RED, "restored");
    assert_eq!(at(&a.frames[3].rgba, 20, 20), CLEAR);
}

#[test]
fn an_apng_s_still_is_its_first_frame_not_its_separate_default_image() {
    use png::{BlendOp::Source, DisposeOp::None};
    let file = apng(
        &[
            ((0, 0, 64, 64), BLUE, Source, None),
            ((0, 0, 64, 64), GREEN, Source, None),
        ],
        Some(RED),
    );
    let a = moving(Format::Png, &file);
    assert_eq!(a.frames.len(), 2);
    assert_eq!(at(&a.frames[0].rgba, 100, 100), BLUE);
    let output = timed(Format::Png, &file).unwrap();
    let (_, _, still) = decode_png(&output.png);
    assert_eq!(still, a.frames[0].rgba, "the still and the animation agree");
}

#[test]
fn an_apng_that_claims_more_frames_than_it_has_ends_with_its_last() {
    use png::{BlendOp::Source, DisposeOp::None};
    let mut file = apng(
        &[
            ((0, 0, 64, 64), RED, Source, None),
            ((0, 0, 64, 64), GREEN, Source, None),
        ],
        Option::None,
    );
    // acTL's frame count, and its CRC made right
    let actl = file.windows(4).position(|w| w == b"acTL").unwrap();
    file[actl + 4..actl + 8].copy_from_slice(&10_000_u32.to_be_bytes());
    let crc = crate::support::crc32(&file[actl..actl + 12]);
    file[actl + 12..actl + 16].copy_from_slice(&crc.to_be_bytes());
    let a = moving(Format::Png, &file);
    assert_eq!(a.frames.len(), 2);
    // and a long one keeps ten seconds of its quarter-second frames
    let many: Vec<_> = (0..250)
        .map(|n| {
            (
                (0, 0, 64, 64),
                if n % 2 == 0 { RED } else { BLUE },
                Source,
                None,
            )
        })
        .collect();
    let kept = moving(Format::Png, &apng(&many, Option::None));
    assert_eq!(
        kept.frames.len(),
        usize::try_from(MAX_PLAY_MS / 250).unwrap()
    );
    assert!(kept.frames.len() < MAX_FRAMES);
}

#[test]
fn webp_and_the_still_of_every_animation_agree() {
    let a = moving(Format::Webp, &fixture("animated.webp"));
    assert_eq!(a.frames.len(), 2);
    assert_eq!(at(&a.frames[0].rgba, 128, 128), RED);
    // lossless, but img2webp's blue reads back as 254
    assert!(
        at(&a.frames[1].rgba, 128, 128)
            .iter()
            .zip(BLUE)
            .all(|(x, y)| x.abs_diff(y) <= 2)
    );
    for (format, file) in [
        (Format::Webp, fixture("animated.webp")),
        (Format::Gif, crate::support::gif_of(48, 48, 3)),
    ] {
        let first = moving(format, &file).frames.remove(0);
        let output = timed(format, &file).unwrap();
        assert_eq!(decode_png(&output.png).2, first.rgba, "{format:?}");
    }
}
