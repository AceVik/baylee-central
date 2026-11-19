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
    Ffmpeg::at(first_file(&FFMPEGS))
}

/// The first of `paths` that is a file, else the first of them.
fn first_file<'a>(paths: &[&'a str]) -> &'a str {
    paths
        .iter()
        .copied()
        .find(|path| std::path::Path::new(path).is_file())
        .or_else(|| paths.first().copied())
        .unwrap_or_default()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_a_command_line_ffmpeg_the_first_that_is_a_file_is_taken() {
        let dir =
            std::env::temp_dir().join(format!("central-picture-first-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("folder")).unwrap();
        let file = dir.join("ffmpeg");
        std::fs::write(&file, b"").unwrap();
        let (missing, folder) = (dir.join("missing"), dir.join("folder"));
        let (missing, folder, file) = (
            missing.to_str().unwrap(),
            folder.to_str().unwrap(),
            file.to_str().unwrap(),
        );
        // a missing path and a directory are passed over
        assert_eq!(first_file(&[missing, folder, file]), file);
        // none there: the first, Ubuntu's, as before
        assert_eq!(first_file(&[missing, folder]), missing);
        assert_eq!(first_file(&[]), "");
        std::fs::remove_dir_all(&dir).unwrap();
        // the list is Ubuntu's first, then Homebrew's, all absolute
        assert_eq!(FFMPEGS[0], "/usr/bin/ffmpeg");
        assert!(FFMPEGS.iter().all(|path| path.starts_with('/')));
    }

    #[test]
    fn the_probe_names_the_encoder_and_what_it_cannot_do() {
        let aom = Found {
            encoder: Encoder::Aom,
            alpha: true,
        };
        let svt = Found {
            encoder: Encoder::Svt,
            alpha: false,
        };
        // the operator's check after a deploy reads this line as it is
        assert_eq!(said(Some(aom)), "ffmpeg with libaom-av1 found");
        assert_eq!(
            said(Some(svt)),
            "ffmpeg with libsvtav1 found; transparent animations are kept as stills (no libaom-av1)"
        );
        assert_eq!(
            said(None),
            "no ffmpeg with an AV1 encoder (libaom-av1, libsvtav1); animations are kept as stills"
        );
    }
}
