//! The animation's encoder (proposal §0.8): ffmpeg with `libaom-av1` as a
//! child of the worker, writing an AVIF image sequence. It is fed only the
//! frames the worker decoded, bounded, composited and averaged itself, as
//! raw RGBA on its standard input, and never sees the uploaded file.
//!
//! No shell; an absolute path from the worker's command line; fixed
//! arguments; no environment; the pipe as its only input protocol and one
//! file as its only output, in a fresh private directory, since ffmpeg's
//! `avif` muxer cannot write to a pipe; one thread for the encoder; and a
//! deadline, after which it is ended. Whatever goes wrong, the picture
//! keeps its still and the answer says why ([`Motion`]).
//!
//! Every frame keeps its own duration: the frames are numbered in
//! milliseconds (`settb`, `setpts`), the encoder counts in milliseconds
//! (`-enc_time_base`), and the input's frame rate is set so that one input
//! frame lasts as long as the last frame, whose duration nothing else
//! gives. The worker then reads the sequence's sample tables and keeps it
//! only when every frame and every duration arrived.
use crate::Animation;
use central_picture::{MAX_SEQUENCE, Motion, SIDE};
use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// How long ffmpeg may take for one animation, all attempts together.
pub const FFMPEG_TIME: Duration = Duration::from_secs(12);
/// Constant rate factors, tried in turn until the sequence fits
/// [`MAX_SEQUENCE`]: 32 gave 50 KB for 50 frames of a photo (proposal §10).
const CRFS: [u8; 3] = [32, 40, 48];
/// `libaom`'s speed: 6 took 2.2 s of CPU for those 50 frames with alpha.
const CPU_USED: &str = "6";
/// The output's name inside the private directory.
const OUTPUT: &str = "sequence.avif";
/// The colour every stored picture uses, as `ravif` writes its stills:
/// BT.601 coefficients at full range, BT.709 primaries, the sRGB curve.
const COLOUR: &str =
    "setparams=colorspace=smpte170m:color_primaries=bt709:color_trc=iec61966-2-1:range=pc";

/// Where ffmpeg is, and how long it may take.
#[derive(Clone, Debug)]
pub struct Ffmpeg {
    program: PathBuf,
    time: Duration,
}

impl Ffmpeg {
    /// The ffmpeg at `program`, which must be an absolute path: the worker
    /// has no environment to search.
    #[must_use]
    pub fn at(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            time: FFMPEG_TIME,
        }
    }

    /// The same ffmpeg with `time` for all attempts of one animation.
    #[must_use]
    pub fn with_time(self, time: Duration) -> Self {
        Self { time, ..self }
    }

    /// Whether this ffmpeg runs and has the `libaom-av1` encoder, asked
    /// within its time.
    #[must_use]
    pub fn probe(&self) -> bool {
        self.probe_until(Instant::now() + self.time)
    }

    fn probe_until(&self, deadline: Instant) -> bool {
        if !self.program.is_absolute() {
            return false;
        }
        let Ok(mut child) = self
            .command(Path::new("/"))
            .args(["-h", "encoder=libaom-av1"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        else {
            return false;
        };
        let stdout = child.stdout.take();
        let reader = std::thread::spawn(move || {
            let mut text = Vec::new();
            if let Some(stdout) = stdout {
                let _ = stdout.take(65_536).read_to_end(&mut text);
            }
            text
        });
        let ended = finish(&mut child, deadline);
        let text = reader.join().unwrap_or_default();
        ended == Some(true) && text.windows(18).any(|w| w == b"Encoder libaom-av1")
    }

    /// The image sequence of `moving`, or why there is none.
    pub(crate) fn encode(&self, moving: &Animation) -> Motion {
        // One deadline for the probe and every attempt.
        let deadline = Instant::now() + self.time;
        if !self.probe_until(deadline) {
            return Motion::NoEncoder;
        }
        let opaque = moving
            .frames
            .iter()
            .all(|frame| frame.rgba.as_chunks::<4>().0.iter().all(|p| p[3] == 255));
        for crf in CRFS {
            match self.attempt(moving, opaque, crf, deadline) {
                Attempt::Made(bytes) => return checked(bytes, moving, opaque),
                Attempt::TooLarge => {}
                Attempt::Failed => return Motion::Failed,
            }
        }
        Motion::TooLarge
    }

    /// One run of ffmpeg at `crf`, within `deadline`.
    fn attempt(&self, moving: &Animation, opaque: bool, crf: u8, deadline: Instant) -> Attempt {
        let Some(scratch) = Scratch::new() else {
            return Attempt::Failed;
        };
        let Ok(mut child) = self
            .command(&scratch.0)
            .args(arguments(moving, opaque, crf))
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        else {
            return Attempt::Failed;
        };
        let Some(mut stdin) = child.stdin.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Attempt::Failed;
        };
        let frames: Vec<u8> = moving
            .frames
            .iter()
            .flat_map(|frame| frame.rgba.iter().copied())
            .collect();
        // Fed on a thread of its own, so an ffmpeg that stops reading cannot
        // hold the worker past the deadline. The thread is never waited for
        // beyond it: whatever still holds the pipe (in production systemd
        // ends the whole unit) cannot hold this one.
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = sender.send(stdin.write_all(&frames).is_ok());
        });
        let ended = finish(&mut child, deadline);
        let fed = ended == Some(true)
            && receiver
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .unwrap_or(false);
        if !fed {
            return Attempt::Failed;
        }
        let mut bytes = Vec::new();
        let read = std::fs::File::open(scratch.0.join(OUTPUT)).and_then(|file| {
            file.take(u64::try_from(MAX_SEQUENCE).unwrap_or(u64::MAX) + 1)
                .read_to_end(&mut bytes)
        });
        match read {
            Ok(_) if bytes.len() > MAX_SEQUENCE => Attempt::TooLarge,
            Ok(_) => Attempt::Made(bytes),
            Err(_) => Attempt::Failed,
        }
    }

    /// ffmpeg with no environment, in `directory`, quiet, never asking.
    fn command(&self, directory: &Path) -> Command {
        let mut command = Command::new(&self.program);
        command.env_clear().current_dir(directory).args([
            "-nostdin",
            "-hide_banner",
            "-nostats",
            "-loglevel",
            "error",
        ]);
        command
    }
}

enum Attempt {
    Made(Vec<u8>),
    TooLarge,
    Failed,
}

/// The arguments of one run: raw RGBA in, every frame at its own time, the
/// colour of the stills, `libaom-av1` on one thread, no metadata, the AVIF
/// image sequence out.
fn arguments(moving: &Animation, opaque: bool, crf: u8) -> Vec<OsString> {
    let last = moving
        .frames
        .last()
        .map_or(100, |frame| frame.delay_ms.max(1));
    // pts(n), in milliseconds: the sum of the delays before frame n
    let pts = (1..moving.frames.len())
        .map(|n| format!("gte(N\\,{n})*{}", moving.frames[n - 1].delay_ms))
        .collect::<Vec<_>>()
        .join("+");
    let timing = format!("[0:v]settb=1/1000,setpts={pts}");
    let colour = format!("scale=out_color_matrix=bt601:out_range=pc,format=yuv420p,{COLOUR}");
    let graph = if opaque {
        format!("{timing},{colour}[colour]")
    } else {
        format!(
            "{timing},split[c][a];[c]{colour}[colour];[a]alphaextract,format=gray,{COLOUR}[alpha]"
        )
    };
    let mut out: Vec<String> = vec![
        "-filter_complex_threads".into(),
        "1".into(),
        "-protocol_whitelist".into(),
        "pipe".into(),
        "-f".into(),
        "rawvideo".into(),
        "-pixel_format".into(),
        "rgba".into(),
        "-video_size".into(),
        format!("{SIDE}x{SIDE}"),
        "-framerate".into(),
        format!("1000/{last}"),
        "-i".into(),
        "pipe:0".into(),
        "-filter_complex".into(),
        graph,
        "-map".into(),
        "[colour]".into(),
    ];
    if !opaque {
        out.extend(["-map".into(), "[alpha]".into()]);
    }
    out.extend(
        [
            "-c:v",
            "libaom-av1",
            "-cpu-used",
            CPU_USED,
            "-b:v",
            "0",
            "-threads",
            "1",
            "-fps_mode",
            "passthrough",
            "-enc_time_base:v",
            "1/1000",
            "-fflags",
            "+bitexact",
            "-flags:v",
            "+bitexact",
            "-map_metadata",
            "-1",
            "-map_chapters",
            "-1",
        ]
        .map(String::from),
    );
    out.extend([
        "-crf".into(),
        crf.to_string(),
        "-loop".into(),
        moving.plays.to_string(),
        "-protocol_whitelist".into(),
        "file".into(),
        "-n".into(),
        "-f".into(),
        "avif".into(),
        OUTPUT.into(),
    ]);
    out.into_iter().map(OsString::from).collect()
}

/// `bytes` as the stored sequence, when its sample tables hold every frame
/// of `moving` with its delay, in one track or, with transparency, two.
fn checked(bytes: Vec<u8>, moving: &Animation, opaque: bool) -> Motion {
    let delays: Vec<u32> = moving.frames.iter().map(|frame| frame.delay_ms).collect();
    match central_picture::sequence_tracks(&bytes) {
        Some(tracks) if tracks.len() == if opaque { 1 } else { 2 } && tracks[0] == delays => {
            Motion::Sequence(bytes)
        }
        _ => Motion::Failed,
    }
}

/// Waits for `child` until `deadline`, then ends it; always reaps it.
/// `Some(success)` when it ended by itself, `None` when it was ended.
fn finish(child: &mut Child, deadline: Instant) -> Option<bool> {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status.success()),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(5)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

/// A fresh directory only this process may enter, removed with everything
/// in it when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Option<Self> {
        let base = std::env::temp_dir();
        for n in 0..64 {
            let path = base.join(format!("central-picture-{}-{n}", std::process::id()));
            let mut builder = std::fs::DirBuilder::new();
            #[cfg(unix)]
            std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
            match builder.create(&path) {
                Ok(()) => return Some(Self(path)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(_) => return None,
            }
        }
        None
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Frame;

    fn moving(delays: &[u32], alpha: u8) -> Animation {
        Animation {
            frames: delays
                .iter()
                .map(|&delay_ms| Frame {
                    rgba: [10, 20, 30, alpha].repeat(256 * 256),
                    delay_ms,
                })
                .collect(),
            plays: 0,
        }
    }

    #[test]
    fn every_frame_is_timed_in_milliseconds_and_the_last_sets_the_rate() {
        let args: Vec<String> = arguments(&moving(&[100, 40, 500], 255), true, 32)
            .into_iter()
            .map(|a| a.into_string().unwrap())
            .collect();
        let at = |flag: &str| args[args.iter().position(|a| a == flag).unwrap() + 1].clone();
        assert_eq!(at("-framerate"), "1000/500");
        assert!(
            at("-filter_complex")
                .starts_with("[0:v]settb=1/1000,setpts=gte(N\\,1)*100+gte(N\\,2)*40,")
        );
        assert_eq!(at("-enc_time_base:v"), "1/1000");
        assert_eq!(at("-crf"), "32");
        assert_eq!(at("-loop"), "0");
        // no alpha track for an opaque picture; one for a transparent one
        assert!(!args.contains(&"[alpha]".to_owned()));
        let alpha: Vec<OsString> = arguments(&moving(&[100, 40], 0), false, 40);
        assert!(alpha.contains(&OsString::from("[alpha]")));
        // nothing but the pipe in, one relative file out, never overwritten
        assert_eq!(args.last().map(String::as_str), Some(OUTPUT));
        assert_eq!(
            args.iter().filter(|a| *a == "-protocol_whitelist").count(),
            2
        );
        assert!(args.contains(&"-n".to_owned()) && !args.contains(&"-y".to_owned()));
    }

    #[test]
    fn a_relative_or_missing_program_has_no_encoder() {
        let frames = moving(&[100, 100], 255);
        assert_eq!(Ffmpeg::at("ffmpeg").encode(&frames), Motion::NoEncoder);
        assert_eq!(
            Ffmpeg::at("/nonexistent/ffmpeg").encode(&frames),
            Motion::NoEncoder
        );
    }

    #[test]
    fn scratch_directories_are_private_and_removed() {
        let scratch = Scratch::new().unwrap();
        let path = scratch.0.clone();
        std::fs::write(path.join(OUTPUT), b"x").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o700);
        }
        let second = Scratch::new().unwrap();
        assert_ne!(second.0, path);
        drop(scratch);
        assert!(!path.exists());
    }
}
