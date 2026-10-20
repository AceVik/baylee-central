//! Whether rustc rejects `unsafe` under `#![forbid(unsafe_code)]`, directly,
//! from a macro, and from an `include!`d file: the guarantee the crate-root
//! check relies on.

use std::fs;
use std::path::Path;
use std::process::Command;

use crate::fail::{Fail, OrFail, Result};
use crate::owned::OwnedDir;
use crate::process;

// The probes are text files, not Rust: the tree's marker scan reads every
// `.rs` file, and these must contain the very code it forbids. They are
// compiled only here, in a temporary directory, and must fail.
const PROBES: [&str; 3] = [
    include_str!("../../probes/direct.txt"),
    include_str!("../../probes/macro.txt"),
    include_str!("../../probes/include.txt"),
];
const GENERATED: &str = include_str!("../../probes/generated.txt");
const FAILURE: &str = "Compiler did not reject the direct/generated unsafe probe";

/// Compiles each probe with the repository's toolchain (run from `root`, so
/// `rust-toolchain.toml` picks it) and wants every one refused.
pub(super) fn run(root: &Path) -> Result<()> {
    let dir = OwnedDir::new("central-guard-")?;
    fs::write(dir.path().join("generated.rs"), GENERATED).or_fail(FAILURE)?;
    let source = dir.path().join("probe.rs");
    for probe in PROBES {
        fs::write(&source, probe).or_fail(FAILURE)?;
        let output = process::capture(
            Command::new("rustc")
                .arg(&source)
                .arg("--out-dir")
                .arg(dir.path())
                .current_dir(root),
            FAILURE,
        )?;
        let diagnostic = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let refused = diagnostic.lines().any(|line| {
            let line = line.to_lowercase();
            line.find("usage of an")
                .is_some_and(|at| line[at..].contains("unsafe"))
        });
        if output.status.success() || !refused {
            return Err(Fail::new(FAILURE));
        }
    }
    dir.remove()
}
