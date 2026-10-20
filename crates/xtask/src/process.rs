//! Running programs: never through a shell, the arguments as a list, a
//! deadline where a program may hang, and the whole process tree stopped
//! when it runs out. A secret goes to a child only through its environment
//! or its standard input, never its arguments.

use std::io::{Read, Write};
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::fail::{Fail, OrFail, Result};

/// Runs `command` with this process's output and fails with `failure` unless
/// it succeeds.
pub(crate) fn succeed(command: &mut Command, failure: &str) -> Result<()> {
    let status = command.stdin(Stdio::null()).status().or_fail(failure)?;
    if status.success() {
        Ok(())
    } else {
        Err(Fail::new(failure))
    }
}

/// Runs `command`, its output captured; whether it succeeded is the caller's.
pub(crate) fn capture(command: &mut Command, failure: &str) -> Result<Output> {
    command.stdin(Stdio::null()).output().or_fail(failure)
}

/// Runs `command` with `input` on its standard input and its output captured.
pub(crate) fn feed(command: &mut Command, input: &[u8], failure: &str) -> Result<Output> {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .or_fail(failure)?;
    // the child is waited for even when writing fails, so it is never left behind
    let wrote = child.stdin.take().map(|mut stdin| stdin.write_all(input));
    let output = child.wait_with_output().or_fail(failure)?;
    match wrote {
        Some(Ok(())) => Ok(output),
        _ => Err(Fail::new(failure)),
    }
}

/// A started child whose output is read on threads, so neither pipe fills.
pub(crate) struct Watched {
    child: Child,
    stdout: Option<JoinHandle<Vec<u8>>>,
    stderr: Option<JoinHandle<Vec<u8>>>,
}

impl Watched {
    /// Starts `command` in a process group of its own (on Unix), so the
    /// whole tree can be stopped.
    pub(crate) fn start(command: &mut Command, failure: &str) -> Result<Self> {
        let mut child = spawn_grouped(command, failure)?;
        let stdout = child.stdout.take().map(drain);
        let stderr = child.stderr.take().map(drain);
        Ok(Self {
            child,
            stdout,
            stderr,
        })
    }

    /// Waits up to `limit`; past it, stops the tree and fails with `timeout`.
    pub(crate) fn finish(
        mut self,
        limit: Duration,
        timeout: &str,
    ) -> Result<(ExitStatus, Vec<u8>, Vec<u8>)> {
        let status = wait_until(&mut self.child, limit)?;
        let Some(status) = status else {
            self.stop()?;
            return Err(Fail::new(timeout));
        };
        let stdout = self
            .stdout
            .take()
            .map(|t| t.join().unwrap_or_default())
            .unwrap_or_default();
        let stderr = self
            .stderr
            .take()
            .map(|t| t.join().unwrap_or_default())
            .unwrap_or_default();
        Ok((status, stdout, stderr))
    }

    /// Stops the child and everything it started, and waits for it.
    pub(crate) fn stop(&mut self) -> Result<()> {
        stop_tree(&mut self.child)
    }
}

impl Drop for Watched {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            let _ = stop_tree(&mut self.child);
        }
    }
}

/// Starts `command` with piped output, in a process group of its own on
/// Unix, so `stop_tree` reaches everything it starts.
pub(crate) fn spawn_grouped(command: &mut Command, failure: &str) -> Result<Child> {
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(command, 0);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .or_fail(failure)
}

/// Reads `pipe` to its end on a thread of its own.
pub(crate) fn drain(mut pipe: impl Read + Send + 'static) -> JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = pipe.read_to_end(&mut bytes);
        bytes
    })
}

/// The child's exit status if it ends within `limit`, else `None`.
pub(crate) fn wait_until(child: &mut Child, limit: Duration) -> Result<Option<ExitStatus>> {
    let end = Instant::now() + limit;
    loop {
        if let Some(status) = child
            .try_wait()
            .or_fail("Owned process state unavailable")?
        {
            return Ok(Some(status));
        }
        if Instant::now() >= end {
            return Ok(None);
        }
        thread::sleep(Duration::from_millis(50));
    }
}

/// Stops `child` and its descendants (its process group on Unix, its tree
/// on Windows) and waits up to ten seconds for it to be gone.
pub(crate) fn stop_tree(child: &mut Child) -> Result<()> {
    if child
        .try_wait()
        .or_fail("Owned process state unavailable")?
        .is_none()
    {
        let pid = child.id().to_string();
        #[cfg(unix)]
        let _ = Command::new("kill")
            .args(["-KILL", "--", &format!("-{pid}")])
            .stderr(Stdio::null())
            .status();
        #[cfg(windows)]
        let _ = Command::new("taskkill")
            .args(["/PID", &pid, "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let _ = child.kill();
    }
    match wait_until(child, Duration::from_secs(10))? {
        Some(_) => Ok(()),
        None => Err(Fail::new("Owned process could not be stopped")),
    }
}
