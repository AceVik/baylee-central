//! The gate: formatting, the policy with its compiler
//! probe, clippy, every test (this crate's policy regressions among them),
//! the doc tests and the release build, in that order, stopping at the
//! first that fails.

use std::path::Path;
use std::process::Command;

use crate::fail::Result;
use crate::process;
use crate::{crypto_scope, policy, workspace};

fn cargo(root: &Path, args: &[&str], failure: &str) -> Result<()> {
    process::succeed(
        Command::new(workspace::cargo())
            .args(args)
            .current_dir(root),
        failure,
    )
}

pub(crate) fn run(root: &Path) -> Result<()> {
    cargo(root, &["fmt", "--all", "--", "--check"], "Format failed")?;
    policy::check(root, true)?;
    println!("{}", policy::PASSED);
    crypto_scope::check(root)?;
    cargo(
        root,
        &[
            "clippy",
            "--locked",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ],
        "Clippy failed",
    )?;
    cargo(
        root,
        &[
            "test",
            "--locked",
            "--workspace",
            "--all-targets",
            "--all-features",
        ],
        "Tests failed",
    )?;
    cargo(
        root,
        &["test", "--locked", "--workspace", "--all-features", "--doc"],
        "Documentation tests failed",
    )?;
    cargo(
        root,
        &[
            "build",
            "--locked",
            "--workspace",
            "--all-features",
            "--release",
        ],
        "Release build failed",
    )?;
    println!("Gate passed: format, policy, clippy, tests, doc tests and release build.");
    Ok(())
}
