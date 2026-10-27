//! Fail-closed bounds for the unreachable RUSTSEC-2026-0387 arithmetic.
//! See docs/crypto-proposal/bigint-advisory.md before updating the review.

mod graph;
mod sources;

use std::{path::Path, process::Command};

use crate::fail::{Fail, OrFail, Result};
use crate::{process, workspace};

pub(crate) fn check(root: &Path) -> Result<()> {
    let output = process::capture(
        Command::new(workspace::cargo())
            .args([
                "metadata",
                "--locked",
                "--offline",
                "--all-features",
                "--format-version",
                "1",
            ])
            .current_dir(root),
        "Crypto scope metadata failed",
    )?;
    if !output.status.success() {
        return Err(Fail::new("Crypto scope metadata failed"));
    }
    let metadata =
        serde_json::from_slice(&output.stdout).or_fail("Crypto scope metadata failed")?;
    let manifest = graph::verify(&metadata)?;
    let expected = root.join("crates/central-crypto/Cargo.toml");
    if Path::new(manifest)
        .canonicalize()
        .or_fail("Crypto manifest unavailable")?
        != expected
            .canonicalize()
            .or_fail("Crypto manifest unavailable")?
    {
        return Err(Fail::new(
            "Crypto crate moved; review RUSTSEC-2026-0387 again",
        ));
    }
    sources::verify(&root.join("crates/central-crypto"))?;
    println!("Crypto advisory scope passed (reviewed Ristretto sources and dependency graph).");
    Ok(())
}
