//! The secret scan: the working tree and the
//! full history of this repository, with a pinned gitleaks (`gitleaks.rs`)
//! whose configuration is first proved on synthetic credentials
//! (`regression.rs`). Git variables that would point Git at another
//! repository are refused before Git runs at all.

mod git;
mod gitleaks;
mod regression;

use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::fail::{Fail, OrFail, Result};
use crate::process;

/// Git's variables that change which repository, index or objects Git uses.
pub const ROUTING: [&str; 10] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_COMMON_DIR",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_SHALLOW_FILE",
    "GIT_REPLACE_REF_BASE",
    "GIT_NAMESPACE",
    "GIT_REFERENCE_BACKEND",
];

/// The message for an environment that reroutes Git.
pub const REROUTED: &str = "Secret history scan rejects overridden Git repository scope";

/// Refuses an environment (names, in any case) that sets one of [`ROUTING`].
///
/// # Errors
/// [`REROUTED`], when one is set.
pub fn reject_rerouting<I: IntoIterator<Item = OsString>>(names: I) -> Result<()> {
    for name in names {
        let name = name.to_string_lossy().to_ascii_uppercase();
        if ROUTING.contains(&name.as_str()) {
            return Err(Fail::new(REROUTED));
        }
    }
    Ok(())
}

pub(crate) fn run(root: &Path) -> Result<()> {
    reject_rerouting(env::vars_os().map(|(name, _)| name))?;
    verify_repository(root)?;
    let binary = gitleaks::fetch(root)?;
    regression::run(root, &binary)?;
    process::succeed(
        Command::new(&binary)
            .args(["dir", "."])
            .args(["--config", ".gitleaks.toml", "--redact", "--no-banner"])
            .current_dir(root),
        "Secret scan failed",
    )?;
    process::succeed(
        Command::new(&binary)
            .args(["git", ".", "--log-opts=--all"])
            .args([
                "--config",
                ".gitleaks-history.toml",
                "--redact",
                "--no-banner",
            ])
            .current_dir(root),
        "Secret history scan failed",
    )?;
    println!("Secret scans passed: the working tree and the full history.");
    Ok(())
}

/// Git resolves this very checkout, and it carries its whole history.
fn verify_repository(root: &Path) -> Result<()> {
    let identity = "Secret history repository identity could not be verified";
    let top = process::capture(
        Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .current_dir(root),
        identity,
    )?;
    if !top.status.success() {
        return Err(Fail::new(identity));
    }
    let top = PathBuf::from(String::from_utf8_lossy(&top.stdout).trim())
        .canonicalize()
        .or_fail(identity)?;
    let same = if cfg!(windows) {
        top.to_string_lossy()
            .eq_ignore_ascii_case(&root.to_string_lossy())
    } else {
        top == root
    };
    if !same {
        return Err(Fail::new(
            "Secret history scan resolved a different repository",
        ));
    }
    let shallow = process::capture(
        Command::new("git")
            .args(["rev-parse", "--is-shallow-repository"])
            .current_dir(root),
        identity,
    )?;
    if !shallow.status.success() || String::from_utf8_lossy(&shallow.stdout).trim() != "false" {
        return Err(Fail::new("Secret history scan requires full Git history"));
    }
    Ok(())
}
