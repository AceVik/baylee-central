//! `central-picture-worker`: one picture per process
//! (`docs/profile-pictures.md`). It reads one request from standard input,
//! writes one answer to standard output and exits; it reads no
//! configuration, opens no file and no socket, and writes nothing else. In
//! production systemd starts it for each connection to the picture socket,
//! in a sandbox without network, secrets or a user of its own; in
//! development the service starts it as a child.
#![forbid(unsafe_code)]

use central_picture_codec::{Refusal, convert};
use std::io::Write;
use std::process::ExitCode;

/// The conversion's own stack: generous, since a decoder may recurse.
const STACK: usize = 64 * 1024 * 1024;

fn main() -> ExitCode {
    // A decoder's panic becomes a refusal; its message, which may quote the
    // file, is printed nowhere.
    std::panic::set_hook(Box::new(|_| {}));
    let mut input = std::io::stdin().lock();
    let result = central_picture::read_request(&mut input).and_then(|(format, bytes)| {
        std::thread::Builder::new()
            .stack_size(STACK)
            .spawn(move || convert(format, &bytes))
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
