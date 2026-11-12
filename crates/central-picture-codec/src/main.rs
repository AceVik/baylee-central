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
//! `--ffmpeg` names the ffmpeg for animations (default `/usr/bin/ffmpeg`,
//! Ubuntu's); `--no-ffmpeg` keeps every animation as its still; `--probe`
//! only says whether that ffmpeg has `libaom-av1`, for the operator, and
//! exits 0 when it has.
#![forbid(unsafe_code)]

use central_picture_codec::{Ffmpeg, Refusal, convert_with};
use std::io::Write;
use std::process::ExitCode;
use std::time::{Duration, Instant};

/// The conversion's own stack: generous, since a decoder may recurse.
const STACK: usize = 64 * 1024 * 1024;
/// Ubuntu's ffmpeg, which the production server installs.
const FFMPEG: &str = "/usr/bin/ffmpeg";
/// The latest ffmpeg may end, from the worker's start: inside the
/// service's `central_picture::TIME` (20 s) with room to answer, so an
/// animation that runs late loses only its sequence, never its still.
const ANSWER_BY: Duration = Duration::from_secs(17);

/// What the command line asks.
struct Options {
    ffmpeg: Option<Ffmpeg>,
    probe: bool,
}

fn options() -> Option<Options> {
    let mut ffmpeg = Some(Ffmpeg::at(FFMPEG));
    let mut probe = false;
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        match arg.to_str()? {
            "--ffmpeg" => ffmpeg = Some(Ffmpeg::at(args.next()?)),
            "--no-ffmpeg" => ffmpeg = None,
            "--probe" => probe = true,
            _ => return None,
        }
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
        let found = options.ffmpeg.as_ref().is_some_and(Ffmpeg::probe);
        println!(
            "animations: {}",
            if found {
                "ffmpeg with libaom-av1 found"
            } else {
                "no ffmpeg with libaom-av1; animations are kept as stills"
            }
        );
        return if found {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }
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
