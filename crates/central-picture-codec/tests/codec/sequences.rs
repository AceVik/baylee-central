//! Animations encoded by ffmpeg (`src/ffmpeg.rs`, proposal §0.8).
//!
//! - **Stand-ins** (`/bin/sh` scripts that answer the probe, then
//!   misbehave) run everywhere with a shell: no encoder, a crash, a hang,
//!   nothing written, garbage, too few frames, wrong durations, too large.
//!   Every one keeps the still and says why.
//! - **The real encoders** need an ffmpeg with `libaom-av1`:
//!   `PICTURE_FFMPEG` names it (default `/usr/bin/ffmpeg`; on a host
//!   without one, `tests/ffmpeg/ffmpeg` runs Ubuntu 26.04's in a
//!   container). Without one these tests say SKIPPED on standard error,
//!   unless `PICTURE_REQUIRE_FFMPEG=1`, which CI sets: then they fail.
//! - **The fallback** runs the same ffmpeg behind a stand-in that hides its
//!   `libaom-av1`, so the worker takes `libsvtav1`; it says SKIPPED where
//!   that ffmpeg has no `libsvtav1` either, unless `PICTURE_REQUIRE_SVT=1`,
//!   which CI sets (Ubuntu 24.04's ffmpeg links SVT-AV1 1.7): then it
//!   fails. Homebrew's ffmpeg has only `libsvtav1`
//!   (`PICTURE_FFMPEG=/opt/homebrew/bin/ffmpeg`).
use crate::support::{fixture, valid};
use central_picture::{Format, Motion, Refusal, sequence_tracks};
use central_picture_codec::{Encoder, Ffmpeg, Found, convert_with};
use std::borrow::Cow;
use std::time::{Duration, Instant};

/// An opaque GIF of 64 × 64 frames, one colour each, with `delays` in
/// hundredths of a second.
fn gif(delays: &[u16]) -> Vec<u8> {
    let palette = [255, 0, 0, 0, 0, 255, 0, 255, 0];
    let mut out = Vec::new();
    {
        let mut encoder = gif::Encoder::new(&mut out, 64, 64, &palette).unwrap();
        encoder.set_repeat(gif::Repeat::Infinite).unwrap();
        for (n, &delay) in delays.iter().enumerate() {
            let frame = gif::Frame {
                width: 64,
                height: 64,
                buffer: Cow::Owned(vec![u8::try_from(n % 3).unwrap(); 64 * 64]),
                delay,
                ..gif::Frame::default()
            };
            encoder.write_frame(&frame).unwrap();
        }
    }
    out
}

/// An APNG of 64 × 64 frames with a transparent left half, `delays` in
/// milliseconds.
fn apng(delays: &[u16]) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, 64, 64);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .set_animated(u32::try_from(delays.len()).unwrap(), 0)
            .unwrap();
        let mut writer = encoder.write_header().unwrap();
        for (n, &delay) in delays.iter().enumerate() {
            writer.set_frame_delay(delay, 1000).unwrap();
            let shade = u8::try_from((n * 40) % 200).unwrap();
            let pixels: Vec<u8> = (0..64 * 64)
                .flat_map(|i| {
                    if i % 64 < 32 {
                        [0, 0, 0, 0]
                    } else {
                        [shade, 128, 255 - shade, 255]
                    }
                })
                .collect();
            writer.write_image_data(&pixels).unwrap();
        }
        writer.finish().unwrap();
    }
    out
}

/// What a stand-in answers when asked for `libaom-av1`, as ffmpeg does.
#[cfg(unix)]
const AOM_HELP: &str =
    "Encoder libaom-av1 [libaom AV1]:\\n    Supported pixel formats: yuv420p yuv444p gray\\n";

/// A stand-in ffmpeg: a script that answers the probe for `libaom-av1` as
/// ffmpeg does and has no other encoder, then runs `encode` with the
/// arguments, the last of which is the output.
#[cfg(unix)]
fn stand_in(name: &str, encode: &str) -> Ffmpeg {
    script(
        name,
        &format!(
            "for a; do last=$a; done\n\
             case \"$*\" in *encoder=libaom-av1*) printf '{AOM_HELP}'; exit 0;; *encoder=*) exit 1;; esac\n\
             {encode}"
        ),
    )
}

/// An executable `/bin/sh` script of `body` in the temporary directory.
#[cfg(unix)]
fn script(name: &str, body: &str) -> Ffmpeg {
    use std::os::unix::fs::PermissionsExt;
    let path = std::env::temp_dir().join(format!(
        "central-picture-test-{}-{name}.sh",
        std::process::id()
    ));
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    Ffmpeg::at(path).with_time(Duration::from_secs(8))
}

/// The motion of `bytes` converted with `ffmpeg`; the still is always there.
fn motion(format: Format, bytes: &[u8], ffmpeg: &Ffmpeg) -> Motion {
    let output = convert_with(format, bytes, Some(ffmpeg)).unwrap();
    valid(&output);
    output.motion
}

/// No directory of this process's encodes is left behind. Other tests of
/// this process may be encoding at the same time (with `PICTURE_FFMPEG` set,
/// as in CI, every animated conversion does), and each removes its own
/// directory when it ends: so the check waits until none is there, and only
/// a directory that stays for 30 s counts as left behind.
fn nothing_left() {
    let prefix = format!("central-picture-{}-", std::process::id());
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let left: Vec<_> = std::fs::read_dir(std::env::temp_dir())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().starts_with(&prefix))
            .map(|entry| entry.path())
            .collect();
        if left.is_empty() {
            return;
        }
        assert!(Instant::now() < deadline, "{left:?}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(unix)]
#[test]
fn every_failure_of_the_encoder_keeps_the_still_and_says_why() {
    // the fixture: libaom's six 64 px frames of 100, 40, 250, 130, 60 and
    // 500 ms, with an alpha track
    let fixture_path = std::env::temp_dir().join(format!(
        "central-picture-test-{}-sequence.avif",
        std::process::id()
    ));
    std::fs::write(&fixture_path, fixture("sequence.avif")).unwrap();
    let copy = format!("cat > /dev/null; cp '{}' \"$last\"", fixture_path.display());
    let six = apng(&[100, 40, 250, 130, 60, 500]);
    let three = apng(&[100, 40, 250]);
    let slower = apng(&[100, 40, 250, 130, 60, 400]);
    let still = gif(&[10]);
    let cases: [(&str, &str, &[u8], Motion); 9] = [
        ("crash", "cat > /dev/null; exit 1", &six, Motion::Failed),
        ("silent", "cat > /dev/null; exit 0", &six, Motion::Failed),
        (
            "garbage",
            "cat > /dev/null; echo '<html>' > \"$last\"",
            &six,
            Motion::Failed,
        ),
        ("unread", "exit 0", &six, Motion::Failed),
        ("fewer", &copy, &three, Motion::Failed),
        ("slower", &copy, &slower, Motion::Failed),
        // no environment, and a private directory of its own
        (
            "careful",
            &format!(
                "[ -z \"$HOME$USER$TMPDIR$LANG\" ] && [ \"$last\" = sequence.avif ] && [ ! -e sequence.avif ] || exit 1; {copy}"
            ),
            &six,
            Motion::Sequence(fixture("sequence.avif")),
        ),
        (
            "large",
            "cat > /dev/null; head -c 600000 /dev/zero > \"$last\"",
            &six,
            Motion::TooLarge,
        ),
        // a still never reaches the encoder
        ("still", "exit 1", &still, Motion::Still),
    ];
    for (name, encode, file, expected) in cases {
        let format = if name == "still" {
            Format::Gif
        } else {
            Format::Png
        };
        assert_eq!(
            motion(format, file, &stand_in(name, encode)),
            expected,
            "{name}"
        );
    }
    // one that hangs is ended at its time, long before it would end (the
    // bound leaves room for a loaded machine's decoding and still)
    let started = Instant::now();
    assert_eq!(
        motion(Format::Png, &six, &stand_in("hang", "sleep 90")),
        Motion::Failed
    );
    assert!(
        started.elapsed() < Duration::from_secs(40),
        "{:?}",
        started.elapsed()
    );
    // none there, or without an AV1 encoder
    assert_eq!(
        motion(Format::Png, &six, &Ffmpeg::at("/nonexistent/ffmpeg")),
        Motion::NoEncoder
    );
    let without = script(
        "without",
        "case \"$*\" in *encoder=*) echo \"Codec is not an encoder\"; exit 0;; esac; exit 1",
    );
    assert_eq!(without.probe(), None);
    assert_eq!(motion(Format::Png, &six, &without), Motion::NoEncoder);
    // only an encoder without alpha: an opaque animation is encoded with
    // it, a transparent one keeps its still
    let svt_help =
        "Encoder libsvtav1 [SVT-AV1]:\\n    Supported pixel formats: yuv420p yuv420p10le\\n";
    let copy_gif = format!(
        "cat > /dev/null; case \"$*\" in *libsvtav1*) cp '{}' \"$last\";; *) exit 1;; esac",
        opaque_fixture().display()
    );
    let only_svt = script(
        "only-svt",
        &format!(
            "for a; do last=$a; done\n\
             case \"$*\" in *encoder=libsvtav1*) printf '{svt_help}'; exit 0;; *encoder=*) exit 1;; esac\n\
             {copy_gif}"
        ),
    );
    assert_eq!(
        only_svt.probe(),
        Some(Found {
            encoder: Encoder::Svt,
            alpha: false
        })
    );
    assert_eq!(motion(Format::Png, &six, &only_svt), Motion::NoEncoder);
    assert!(matches!(
        motion(Format::Gif, &gif(&[10, 30, 150]), &only_svt),
        Motion::Sequence(_)
    ));
    nothing_left();
}

/// A real sequence of one track for an opaque GIF of 100, 300 and 1500 ms,
/// made by `libsvtav1` (`tests/fixtures/README.md`), written where a
/// stand-in can copy it.
#[cfg(unix)]
fn opaque_fixture() -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "central-picture-test-{}-opaque.avif",
        std::process::id()
    ));
    std::fs::write(&path, fixture("opaque-sequence.avif")).unwrap();
    path
}

/// The ffmpeg the real tests use: `PICTURE_FFMPEG`, else Ubuntu's.
fn real_path() -> std::ffi::OsString {
    std::env::var_os("PICTURE_FFMPEG").unwrap_or_else(|| "/usr/bin/ffmpeg".into())
}

/// The ffmpeg with `libaom-av1` the real tests use, if there is one.
fn real() -> Option<Ffmpeg> {
    let ffmpeg = Ffmpeg::at(real_path());
    if ffmpeg.probe().map(|found| found.encoder) == Some(Encoder::Aom) {
        return Some(ffmpeg);
    }
    assert!(
        std::env::var_os("PICTURE_REQUIRE_FFMPEG").is_none_or(|v| v != "1"),
        "PICTURE_REQUIRE_FFMPEG=1, but there is no ffmpeg with libaom-av1 (PICTURE_FFMPEG)"
    );
    eprintln!(
        "SKIPPED: no ffmpeg with libaom-av1; set PICTURE_FFMPEG (see tests/ffmpeg/Dockerfile)"
    );
    None
}

/// Every `colr` box of type `nclx`: primaries, transfer, matrix, full range.
fn colours(file: &[u8]) -> Vec<[u8; 7]> {
    file.windows(8)
        .enumerate()
        .filter(|(_, w)| w == b"colrnclx")
        .map(|(at, _)| file[at + 8..at + 15].try_into().unwrap())
        .collect()
}

#[test]
fn libaom_keeps_every_frame_its_duration_and_the_still_s_colour() {
    let Some(ffmpeg) = real() else { return };
    // an opaque GIF (one track) and a transparent APNG (two), each with a
    // long last frame, which an encoder's own timing would lose
    let cases: [(Format, Vec<u8>, Vec<u32>, usize); 2] = [
        (Format::Gif, gif(&[10, 30, 150]), vec![100, 300, 1500], 1),
        (
            Format::Png,
            apng(&[100, 40, 250, 900]),
            vec![100, 40, 250, 900],
            2,
        ),
    ];
    for (format, file, delays, tracks) in cases {
        let output = convert_with(format, &file, Some(&ffmpeg)).unwrap();
        valid(&output);
        let Motion::Sequence(sequence) = &output.motion else {
            panic!("{format:?}: {:?}", output.motion);
        };
        assert_eq!(&sequence[8..12], b"avis");
        assert_eq!(
            sequence_tracks(sequence),
            Some(vec![delays; tracks]),
            "{format:?}"
        );
        // one colour description for the still and every track: BT.709
        // primaries, the sRGB curve, BT.601 coefficients, full range
        let mut seen = colours(&output.avif);
        seen.extend(colours(sequence));
        assert!(seen.len() >= 2, "{format:?}: {seen:?}");
        assert!(
            seen.iter().all(|c| *c == [0, 1, 0, 13, 0, 6, 0x80]),
            "{format:?}: {seen:?}"
        );
    }
    nothing_left();
}

/// The real ffmpeg behind a stand-in that hides its `libaom-av1`, if it has
/// `libsvtav1`: the host the fallback is for.
#[cfg(unix)]
fn without_libaom() -> Option<(Ffmpeg, std::path::PathBuf)> {
    let real = real_path();
    let real = real.to_str().expect("PICTURE_FFMPEG is UTF-8");
    assert!(!real.contains('\''), "{real}");
    let ffmpeg = script(
        "no-libaom",
        &format!(
            "case \"$*\" in *encoder=libaom-av1*) echo \"Codec 'libaom-av1' is not recognized by FFmpeg.\"; exit 1;; esac\n\
             exec '{real}' \"$@\""
        ),
    )
    .with_time(central_picture_codec::FFMPEG_TIME);
    let path = std::env::temp_dir().join(format!(
        "central-picture-test-{}-no-libaom.sh",
        std::process::id()
    ));
    match ffmpeg.probe() {
        Some(Found {
            encoder: Encoder::Svt,
            alpha: false,
        }) => Some((ffmpeg, path)),
        other => {
            assert!(
                std::env::var_os("PICTURE_REQUIRE_SVT").is_none_or(|v| v != "1"),
                "PICTURE_REQUIRE_SVT=1, but {real} has no libsvtav1 ({other:?})"
            );
            eprintln!("SKIPPED: {real} has no libsvtav1 ({other:?}); set PICTURE_FFMPEG");
            None
        }
    }
}

#[cfg(unix)]
#[test]
fn without_libaom_svt_encodes_opaque_animations_and_the_service_keeps_them() {
    let Some((ffmpeg, path)) = without_libaom() else {
        return;
    };
    // an opaque GIF with a long last frame, and one of as many frames as
    // the worker keeps (100), every one different
    let cases: [(Vec<u8>, Vec<u32>); 2] = [
        (gif(&[10, 30, 150]), vec![100, 300, 1500]),
        (sliding_gif(&[5; 100]), vec![50; 100]),
    ];
    for (file, delays) in cases {
        let output = convert_with(Format::Gif, &file, Some(&ffmpeg)).unwrap();
        valid(&output);
        let Motion::Sequence(sequence) = &output.motion else {
            panic!("{} frames: {:?}", delays.len(), output.motion);
        };
        assert_eq!(&sequence[8..12], b"avis");
        assert_eq!(sequence_tracks(sequence), Some(vec![delays]));
        // the stills' colour description in the sequence too
        let mut seen = colours(&output.avif);
        seen.extend(colours(sequence));
        assert!(seen.len() >= 2, "{seen:?}");
        assert!(
            seen.iter().all(|c| *c == [0, 1, 0, 13, 0, 6, 0x80]),
            "{seen:?}"
        );
    }
    // a transparent one keeps its still: SVT-AV1 writes no alpha track
    assert_eq!(
        convert_with(Format::Png, &apng(&[100, 40, 250]), Some(&ffmpeg))
            .unwrap()
            .motion,
        Motion::NoEncoder
    );
    // through the worker binary and the service's checks of its answer
    let worker = central_picture::Worker::program_with(
        env!("CARGO_BIN_EXE_central-picture-worker"),
        vec!["--ffmpeg".into(), path.into()],
    );
    let converted = worker.convert(Format::Gif, &gif(&[10, 20])).unwrap();
    let sequence = converted.sequence().expect("a sequence");
    assert_eq!(sequence_tracks(sequence), Some(vec![vec![100, 200]]));
    assert_eq!(
        converted.sha256,
        central_picture::digest(&converted.png, Some(sequence))
    );
    nothing_left();
}

/// An opaque GIF of 256 × 256 frames of a diagonal gradient that slides by
/// three steps a frame, with `delays` in hundredths of a second.
fn sliding_gif(delays: &[u16]) -> Vec<u8> {
    let palette: Vec<u8> = (0..=255_u8).flat_map(|v| [v, 255 - v, v / 2]).collect();
    let mut out = Vec::new();
    {
        let mut encoder = gif::Encoder::new(&mut out, 256, 256, &palette).unwrap();
        encoder.set_repeat(gif::Repeat::Infinite).unwrap();
        for (n, &delay) in delays.iter().enumerate() {
            let buffer: Vec<u8> = (0..256 * 256)
                .map(|i| u8::try_from((i % 256 + i / 256 + n * 3) % 256).unwrap())
                .collect();
            let frame = gif::Frame {
                width: 256,
                height: 256,
                buffer: Cow::Owned(buffer),
                delay,
                ..gif::Frame::default()
            };
            encoder.write_frame(&frame).unwrap();
        }
    }
    out
}

#[test]
fn the_worker_binary_encodes_with_the_ffmpeg_its_command_line_names() {
    let Some(_) = real() else { return };
    let path = real_path();
    let worker = central_picture::Worker::program_with(
        env!("CARGO_BIN_EXE_central-picture-worker"),
        vec!["--ffmpeg".into(), path],
    );
    let converted = worker.convert(Format::Gif, &gif(&[10, 20])).unwrap();
    let sequence = converted.sequence().expect("a sequence");
    assert_eq!(sequence_tracks(sequence), Some(vec![vec![100, 200]]));
    assert_eq!(
        converted.sha256,
        central_picture::digest(&converted.png, Some(sequence))
    );
    // a still stays a still, and a refusal stays a refusal
    let still = worker.convert(Format::Gif, &gif(&[10])).unwrap();
    assert_eq!(still.motion, Motion::Still);
    assert_eq!(
        worker.convert(Format::Gif, b"GIF89a"),
        Err(Refusal::Malformed)
    );
}
