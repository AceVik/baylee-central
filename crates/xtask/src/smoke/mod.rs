//! The development listener's smoke test (`scripts/smoke-dev.ps1`): start
//! the release `central-service --dev` on a free loopback port, check its
//! HTTP boundary (`http.rs`), then stop it and prove the process and the
//! listener are gone, also when a check failed.

mod http;

use std::env;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crate::fail::{Fail, OrFail, Result, after_cleanup};
use crate::{process, socket};

const LISTENING: &str = "central: development listener at http://127.0.0.1:";

pub(crate) fn run(root: &Path, exercise_failure: bool) -> Result<()> {
    let binary = root
        .join("target/release/central-service")
        .with_extension(env::consts::EXE_EXTENSION);
    if !binary.is_file() {
        return Err(Fail::new("Build the release executable first"));
    }
    let mut command = Command::new(&binary);
    command.arg("--dev").current_dir(root);
    for (key, _) in env::vars_os() {
        if key
            .to_string_lossy()
            .to_ascii_uppercase()
            .starts_with("BAYLEE_CENTRAL_")
        {
            command.env_remove(key);
        }
    }
    command
        .env("BAYLEE_CENTRAL_LISTEN", "127.0.0.1:0")
        .env("BAYLEE_CENTRAL_REQUEST_BYTES", "1024")
        .env("BAYLEE_CENTRAL_CONCURRENT_REQUESTS", "2");
    let mut child = process::spawn_grouped(&mut command, "Cannot start owned development process")?;
    let mut port = None;
    let outcome = check(&mut child, &mut port, exercise_failure);
    let cleanup = stop(&mut child, port);
    after_cleanup(outcome, cleanup)?;
    println!(
        "Development smoke passed: HTTP boundaries, header deadline and verified process/listener cleanup."
    );
    Ok(())
}

fn check(child: &mut Child, port: &mut Option<u16>, exercise_failure: bool) -> Result<()> {
    let listening = first_line(child)?;
    let found = listening
        .strip_prefix(LISTENING)
        .and_then(|rest| rest.parse::<u16>().ok())
        .or_fail("Unexpected development startup result")?;
    *port = Some(found);
    if exercise_failure {
        return Err(Fail::new("Synthetic development failure"));
    }
    http::check_boundary(found)?;
    http::check_header_deadline(found)
}

/// The first line the service prints, within 15 seconds; the rest of its
/// output is drained so the service never blocks on a full pipe. A service
/// that ends without a line is an unexpected result, not a timeout.
fn first_line(child: &mut Child) -> Result<String> {
    let stdout = child
        .stdout
        .take()
        .or_fail("Development startup timed out")?;
    if let Some(stderr) = child.stderr.take() {
        process::drain(stderr);
    }
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut lines = BufReader::new(stdout).lines();
        let _ = sender.send(lines.next().and_then(std::result::Result::ok));
        for _ in lines {}
    });
    receiver
        .recv_timeout(Duration::from_secs(15))
        .or_fail("Development startup timed out")?
        .or_fail("Unexpected development startup result")
}

/// Stops the service and proves it is gone: the process, then the port.
fn stop(child: &mut Child, port: Option<u16>) -> Result<()> {
    let mut failures = Vec::new();
    if process::stop_tree(child).is_err() {
        failures.push("Owned development process cleanup could not be verified");
    }
    if port.is_some_and(|port| !socket::refused(port)) {
        failures.push("Owned development listener closure could not be verified");
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(Fail::new(failures.join("; ")))
    }
}
