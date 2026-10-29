//! The gate: formatting, the policy with its compiler
//! probe, clippy, every test (this crate's policy regressions among them),
//! the doc tests and the release build, in that order, stopping at the
//! first that fails.
//!
//! `--client` keeps formatting, the policy and the crypto scope, and runs
//! clippy, the tests and the doc tests only for what the Baylee client links
//! (`client`), without the release build: the server is built on Linux
//! x86-64 alone (owner, 29.10.2026).

use std::path::Path;
use std::process::Command;

use crate::fail::Result;
use crate::process;
use crate::{client, crypto_scope, policy, workspace};

fn cargo(root: &Path, args: &[&str], failure: &str) -> Result<()> {
    process::succeed(
        Command::new(workspace::cargo())
            .args(args)
            .current_dir(root),
        failure,
    )
}

/// `cargo <command> <scope> <args>`: the scope is `--workspace` or a `-p`
/// for each package.
fn scoped(root: &Path, command: &str, scope: &[&str], args: &[&str], failure: &str) -> Result<()> {
    let line: Vec<&str> = [command, "--locked"]
        .into_iter()
        .chain(scope.iter().copied())
        .chain(args.iter().copied())
        .collect();
    cargo(root, &line, failure)
}

pub(crate) fn run(root: &Path, client: bool) -> Result<()> {
    let packages = if client {
        let packages = client::packages(root)?;
        println!("Client gate packages: {}", packages.join(", "));
        packages
    } else {
        Vec::new()
    };
    let scope: Vec<&str> = if client {
        packages
            .iter()
            .flat_map(|name| ["-p", name.as_str()])
            .collect()
    } else {
        vec!["--workspace"]
    };
    cargo(root, &["fmt", "--all", "--", "--check"], "Format failed")?;
    policy::check(root, true)?;
    println!("{}", policy::PASSED);
    crypto_scope::check(root)?;
    scoped(
        root,
        "clippy",
        &scope,
        &["--all-targets", "--all-features", "--", "-D", "warnings"],
        "Clippy failed",
    )?;
    scoped(
        root,
        "test",
        &scope,
        &["--all-targets", "--all-features"],
        "Tests failed",
    )?;
    scoped(
        root,
        "test",
        &scope,
        &["--all-features", "--doc"],
        "Documentation tests failed",
    )?;
    if client {
        println!("Client gate passed: format, policy, clippy, tests and doc tests.");
        return Ok(());
    }
    scoped(
        root,
        "build",
        &scope,
        &["--all-features", "--release"],
        "Release build failed",
    )?;
    println!("Gate passed: format, policy, clippy, tests, doc tests and release build.");
    Ok(())
}
