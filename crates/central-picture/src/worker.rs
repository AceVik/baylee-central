//! The service's side of the worker: a fresh process per picture, as a
//! child in development and tests, through systemd's socket activation in
//! production (`ops/central-picture.socket`). Either way the worker gets the
//! request, nothing else, and [`TIME`] to answer; a worker that takes longer,
//! answers more than [`MAX_ANSWER`] or anything malformed is ended and the
//! picture is refused. Blocking: the service calls it on a blocking thread.
use super::{Format, MAX_ANSWER, Motion, Output, Refusal, protocol};
use std::{
    ffi::OsString,
    io::{Read, Write},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

/// How long a conversion may take, start to answer: the decoders, the
/// still's encoders and ffmpeg's encoding of an animation, which the worker
/// itself ends after `central_picture_codec::FFMPEG_TIME`.
pub const TIME: Duration = Duration::from_secs(20);

/// Where the worker is.
#[derive(Clone, Debug)]
pub struct Worker(Kind);

#[derive(Clone, Debug)]
enum Kind {
    /// A child process of this program, with no environment.
    Program {
        program: PathBuf,
        args: Vec<OsString>,
    },
    /// A socket systemd accepts on, starting a worker for each connection.
    #[cfg(unix)]
    Socket(PathBuf),
}

/// A conversion's result, checked: every picture within its bounds and
/// framing, and the digest as this process computed it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Converted {
    /// The still's AVIF.
    pub avif: Vec<u8>,
    /// The still's PNG master and fallback.
    pub png: Vec<u8>,
    /// Whether it moves: with a [`Motion::Sequence`], the image sequence.
    pub motion: Motion,
    /// [`crate::digest`] of `png` and the sequence.
    pub sha256: [u8; 32],
}

impl Converted {
    /// The image sequence, when the picture moves and one was made.
    #[must_use]
    pub fn sequence(&self) -> Option<&[u8]> {
        match &self.motion {
            Motion::Sequence(sequence) => Some(sequence),
            _ => None,
        }
    }
}

impl Worker {
    /// The worker binary at `program`, started as a child for each picture.
    #[must_use]
    pub fn program(program: impl Into<PathBuf>) -> Self {
        Self::program_with(program, Vec::new())
    }

    /// The worker binary at `program`, started with `args` (such as
    /// `--ffmpeg PATH`) for each picture.
    #[must_use]
    pub fn program_with(program: impl Into<PathBuf>, args: Vec<OsString>) -> Self {
        Self(Kind::Program {
            program: program.into(),
            args,
        })
    }

    /// The listening socket at `path`, whose every connection systemd hands
    /// to a fresh, sandboxed worker (`ops/central-picture@.service`).
    #[cfg(unix)]
    #[must_use]
    pub fn socket(path: impl Into<PathBuf>) -> Self {
        Self(Kind::Socket(path.into()))
    }

    /// Convert `bytes`, which the request declared as `format`.
    ///
    /// # Errors
    /// `Unsupported` or `WrongType` when the bytes are no format or another
    /// one than declared, checked here before any worker runs; the worker's
    /// own refusal; `Failed` when the worker could not be reached, ran out
    /// of time, or answered anything but one well-formed frame whose
    /// pictures pass [`crate::png_master`] and [`crate::avif_framing`].
    pub fn convert(&self, format: Format, bytes: &[u8]) -> Result<Converted, Refusal> {
        match Format::sniff(bytes) {
            Some(found) if found == format => {}
            Some(_) => return Err(Refusal::WrongType),
            None => return Err(Refusal::Unsupported),
        }
        let request = protocol::request(format, bytes)?;
        let raw = match &self.0 {
            Kind::Program { program, args } => run_program(program, args, request)?,
            #[cfg(unix)]
            Kind::Socket(path) => run_socket(path, &request)?,
        };
        let Output { avif, png, motion } = protocol::read_answer(&raw).ok_or(Refusal::Failed)??;
        if !crate::png_master(&png) || !crate::avif_framing(&avif) {
            return Err(Refusal::Failed);
        }
        let sequence = match &motion {
            Motion::Sequence(sequence) => Some(sequence.as_slice()),
            _ => None,
        };
        // A worker that answers a sequence the checks refuse may be anyone's:
        // the whole picture is refused, as for a broken still.
        if sequence.is_some_and(|sequence| !crate::sequence_framing(sequence)) {
            return Err(Refusal::Failed);
        }
        let sha256 = crate::digest(&png, sequence);
        Ok(Converted {
            avif,
            png,
            motion,
            sha256,
        })
    }
}

/// Start `program` with no environment and its working directory at the
/// file system's root, hand it `request`, and read its answer within
/// [`TIME`]. The child is ended and waited for on every path.
fn run_program(program: &PathBuf, args: &[OsString], request: Vec<u8>) -> Result<Vec<u8>, Refusal> {
    let mut command = Command::new(program);
    command
        .args(args)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(unix)]
    command.current_dir("/");
    let mut child = command.spawn().map_err(|_| Refusal::Failed)?;
    let deadline = Instant::now() + TIME;
    let result = exchange(&mut child, request, deadline);
    if result.is_err() {
        let _ = child.kill();
    }
    // Reaped in every case, so no process outlives the request.
    let _ = child.wait();
    result
}

fn exchange(child: &mut Child, request: Vec<u8>, deadline: Instant) -> Result<Vec<u8>, Refusal> {
    let mut stdin = child.stdin.take().ok_or(Refusal::Failed)?;
    let stdout = child.stdout.take().ok_or(Refusal::Failed)?;
    // Both ends run on their own threads, so a worker that neither reads nor
    // writes cannot hold this one past the deadline; ending the child closes
    // its pipes, and with them both threads.
    thread::spawn(move || {
        let _ = stdin.write_all(&request);
    });
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut answer = Vec::new();
        let read = stdout
            .take(u64::try_from(MAX_ANSWER).unwrap_or(u64::MAX) + 1)
            .read_to_end(&mut answer);
        let _ = sender.send(read.map(|_| answer));
    });
    let answer = receiver
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .map_err(|_| Refusal::Failed)?
        .map_err(|_| Refusal::Failed)?;
    if answer.len() > MAX_ANSWER {
        return Err(Refusal::Failed);
    }
    // The answer is whole once the worker closed its output; it must also
    // have exited, successfully, before the deadline.
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(answer),
            Ok(Some(_)) | Err(_) => return Err(Refusal::Failed),
            Ok(None) if Instant::now() >= deadline => return Err(Refusal::Failed),
            Ok(None) => thread::sleep(Duration::from_millis(5)),
        }
    }
}

/// Hand `request` to the worker behind the socket at `path` and read its
/// answer within [`TIME`].
#[cfg(unix)]
fn run_socket(path: &PathBuf, request: &[u8]) -> Result<Vec<u8>, Refusal> {
    use std::os::unix::net::UnixStream;
    let deadline = Instant::now() + TIME;
    let remaining = || {
        let left = deadline.saturating_duration_since(Instant::now());
        (!left.is_zero()).then_some(left).ok_or(Refusal::Failed)
    };
    let mut stream = UnixStream::connect(path).map_err(|_| Refusal::Failed)?;
    stream
        .set_write_timeout(Some(remaining()?))
        .map_err(|_| Refusal::Failed)?;
    stream.write_all(request).map_err(|_| Refusal::Failed)?;
    stream
        .shutdown(std::net::Shutdown::Write)
        .map_err(|_| Refusal::Failed)?;
    let mut answer = Vec::new();
    let mut buffer = vec![0; 64 * 1024];
    loop {
        stream
            .set_read_timeout(Some(remaining()?))
            .map_err(|_| Refusal::Failed)?;
        let read = stream.read(&mut buffer).map_err(|_| Refusal::Failed)?;
        if read == 0 {
            return Ok(answer);
        }
        answer.extend_from_slice(&buffer[..read]);
        if answer.len() > MAX_ANSWER {
            return Err(Refusal::Failed);
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    /// A stand-in worker: `sh -c script`, with the request on its input.
    fn shell(script: &str) -> Worker {
        Worker(Kind::Program {
            program: "/bin/sh".into(),
            args: vec!["-c".into(), script.into()],
        })
    }

    const GIF: &[u8] = b"GIF89a\x01\x00\x01\x00";

    #[test]
    fn the_declared_type_must_match_before_any_worker_runs() {
        // /nonexistent is never started: the bytes decide first
        let worker = Worker::program("/nonexistent/worker");
        assert_eq!(worker.convert(Format::Png, GIF), Err(Refusal::WrongType));
        assert_eq!(
            worker.convert(Format::Png, b"<html>"),
            Err(Refusal::Unsupported)
        );
        assert_eq!(worker.convert(Format::Gif, GIF), Err(Refusal::Failed));
    }

    #[test]
    fn a_worker_that_crashes_hangs_or_babbles_is_ended_and_refused() {
        let cases = [
            // exits with failure, says nothing
            "exit 3",
            // crashes after reading
            "cat > /dev/null; kill -SEGV $$",
            // echoes the request back: not an answer
            "cat",
            // answers more than any answer may be
            "exec yes",
            // a refusal frame, then exits with failure
            "cat > /dev/null; printf 'BCPA\\002\\001\\006'; exit 1",
        ];
        for script in cases {
            let started = Instant::now();
            assert_eq!(
                shell(script).convert(Format::Gif, GIF),
                Err(Refusal::Failed),
                "{script}"
            );
            assert!(started.elapsed() < TIME, "{script}");
        }
    }

    #[test]
    fn a_worker_that_hangs_is_killed_at_the_deadline() {
        let started = Instant::now();
        assert_eq!(
            shell("exec sleep 60").convert(Format::Gif, GIF),
            Err(Refusal::Failed)
        );
        let took = started.elapsed();
        assert!(
            took >= TIME && took < TIME + Duration::from_secs(3),
            "{took:?}"
        );
    }

    #[test]
    fn a_worker_s_refusal_is_passed_on_and_its_environment_is_empty() {
        assert_eq!(
            shell("cat > /dev/null; printf 'BCPA\\002\\001\\006'").convert(Format::Gif, GIF),
            Err(Refusal::Malformed)
        );
        // anything in the environment beyond what the shell sets itself
        // would be refused below; an empty one gives the refusal
        let probe = "cat > /dev/null; if [ -z \"$HOME$USER$PATH_TO_SECRET$CENTRAL_DATABASE_PASSWORD\" ]; \
                     then printf 'BCPA\\002\\001\\005'; else printf 'BCPA\\002\\001\\001'; fi";
        assert_eq!(
            shell(probe).convert(Format::Gif, GIF),
            Err(Refusal::TooSmall)
        );
        // and it runs at the file system's root
        let root = "cat > /dev/null; if [ \"$(pwd)\" = / ]; then printf 'BCPA\\002\\001\\005'; fi";
        assert_eq!(
            shell(root).convert(Format::Gif, GIF),
            Err(Refusal::TooSmall)
        );
    }

    #[test]
    fn a_well_framed_answer_that_fails_its_checks_is_refused() {
        // A frame with pictures of the right sizes that are not a PNG
        // master and an AVIF: a worker cannot pass off anything else.
        let script = "cat > /dev/null; printf 'BCPA\\002\\000\\000\\000\\000\\001X\\000\\000\\000\\001Y\\000'";
        assert_eq!(
            shell(script).convert(Format::Gif, GIF),
            Err(Refusal::Failed)
        );
    }

    #[test]
    fn a_socket_nobody_listens_on_fails() {
        assert_eq!(
            Worker::socket("/nonexistent/picture.sock").convert(Format::Gif, GIF),
            Err(Refusal::Failed)
        );
    }
}
