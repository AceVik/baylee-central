//! `central-picture-worker`: one picture per process
//! (`docs/profile-pictures.md`). It reads one request from standard input,
//! writes one answer to standard output and exits. It reads no
//! configuration but its command line, opens no socket and writes nothing
//! else. For an animation it starts ffmpeg as its own child, fed raw
//! frames, writing one file in a private directory that is removed after.
//! In production systemd starts it for each connection to the picture
//! socket, in a sandbox without network, secrets or a user of its own; in
//! development the service starts it as a child.
//!
//! ```text
//! central-picture-worker [--ffmpeg PATH | --no-ffmpeg] [--probe]
//! ```
//!
//! `--ffmpeg` names the ffmpeg for animations (default: the first of
//! [`FFMPEGS`] that is a file, else `/usr/bin/ffmpeg`, Ubuntu's, which the
//! production unit names itself); `--no-ffmpeg` keeps every animation as
//! its still; `--probe` only says which AV1 encoder that ffmpeg has
//! (`libaom-av1`, else `libsvtav1`, which keeps transparent animations as
//! stills), for the operator, and exits 0 when it has one.
#![forbid(unsafe_code)]

use central_picture_codec::{Encoder, Ffmpeg, Found, Refusal, convert_with};
use std::io::Write;
use std::process::ExitCode;
use std::time::{Duration, Instant};

/// The conversion's own stack: generous, since a decoder may recurse.
const STACK: usize = 64 * 1024 * 1024;
/// Where the worker looks for ffmpeg without `--ffmpeg`: Ubuntu's, which
/// the production server installs (and its unit names anyway), then
/// Homebrew's on Apple Silicon and on Intel, for a development Central on
/// a Mac, whose service starts the worker without `--ffmpeg` unless told
/// one. Fixed absolute paths only: the worker has no environment to search.
const FFMPEGS: [&str; 3] = [
    "/usr/bin/ffmpeg",
    "/opt/homebrew/bin/ffmpeg",
    "/usr/local/bin/ffmpeg",
];

/// The first of [`FFMPEGS`] that is a file, else Ubuntu's.
fn default_ffmpeg() -> Ffmpeg {
    let found = FFMPEGS
        .into_iter()
        .find(|path| std::path::Path::new(path).is_file())
        .unwrap_or(FFMPEGS[0]);
    Ffmpeg::at(found)
}
/// The latest ffmpeg may end, from the worker's start: inside the
/// service's `central_picture::TIME` (20 s) with room to answer, so an
/// animation that runs late loses only its sequence, never its still.
const ANSWER_BY: Duration = Duration::from_secs(17);
/// The worker's own end, inside the service's 20 s: past it the process
/// exits unanswered, whatever a decoder is doing, so no worker outlives the
/// slot the service holds for it.
const END_BY: Duration = Duration::from_secs(19);

/// What the command line asks.
struct Options {
    ffmpeg: Option<Ffmpeg>,
    probe: bool,
}

fn options() -> Option<Options> {
    let mut ffmpeg = None;
    let mut chosen = false;
    let mut probe = false;
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        match arg.to_str()? {
            "--ffmpeg" => {
                ffmpeg = Some(Ffmpeg::at(args.next()?));
                chosen = true;
            }
            "--no-ffmpeg" => {
                ffmpeg = None;
                chosen = true;
            }
            "--probe" => probe = true,
            _ => return None,
        }
    }
    // Looked for only when the command line named none.
    if !chosen {
        ffmpeg = Some(default_ffmpeg());
    }
    Some(Options { ffmpeg, probe })
}

fn main() -> ExitCode {
    let started = Instant::now();
    let Some(options) = options() else {
        eprintln!("usage: central-picture-worker [--ffmpeg PATH | --no-ffmpeg] [--probe]");
        return ExitCode::FAILURE;
    };
    if options.probe {
        let found = options.ffmpeg.as_ref().and_then(Ffmpeg::probe);
        println!("animations: {}", said(found));
        return if found.is_some() {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }
    std::thread::spawn(move || {
        std::thread::sleep(END_BY.saturating_sub(started.elapsed()));
        std::process::exit(2);
    });
    // A decoder's panic becomes a refusal; its message, which may quote the
    // file, is printed nowhere.
    std::panic::set_hook(Box::new(|_| {}));
    let mut input = std::io::stdin().lock();
    let result = central_picture::read_request(&mut input).and_then(|(format, bytes)| {
        let ffmpeg = options.ffmpeg.map(|f| f.until(started + ANSWER_BY));
        std::thread::Builder::new()
            .stack_size(STACK)
            .spawn(move || convert_with(format, &bytes, ffmpeg.as_ref()))
            .map_err(|_| Refusal::Failed)?
            .join()
            .unwrap_or(Err(Refusal::Malformed))
    });
    let answer = central_picture::answer(&result);
    let mut output = std::io::stdout().lock();
    if output
        .write_all(&answer)
        .and_then(|()| output.flush())
        .is_err()
    {
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

/// What `--probe` found, for the operator (`docs/operations.md`).
fn said(found: Option<Found>) -> String {
    let names = Encoder::ALL.map(Encoder::name).join(", ");
    match found {
        Some(Found {
            encoder,
            alpha: true,
        }) => format!("ffmpeg with {} found", encoder.name()),
        Some(Found {
            encoder,
            alpha: false,
        }) => format!(
            "ffmpeg with {} found; transparent animations are kept as stills (no {})",
            encoder.name(),
            Encoder::Aom.name()
        ),
        None => format!("no ffmpeg with an AV1 encoder ({names}); animations are kept as stills"),
    }
}
