//! The repository's code policy (AGENTS.md, "Safety and dependencies"), with
//! the rules and messages the former `scripts/check-policy.ps1` had:
//!
//! - the workspace forbids `unsafe`; every member inherits the workspace
//!   lints, is not published, declares one of the repository's two licences
//!   (an AGPL member building only on AGPL members), lives in the reviewed
//!   tree and outside the tool output, and every target's root says
//!   `#![forbid(unsafe_code)]`;
//! - the whole tree holds no link, no C/C++ or assembly source, and no
//!   unsafe block, inline assembly or unsafe allowance in a Rust file; skipped are only
//!   tool output: `.git`, `target` and `node_modules` at the root, and
//!   `web/node_modules`, npm's install of the web app;
//! - every dependency outside the workspace comes from crates.io, at a
//!   version and with features `docs/dependency-proposal/approved-packages.json`
//!   approves;
//! - optionally, rustc itself rejects direct and generated `unsafe` under the
//!   forbid lint.

pub(crate) mod metadata;
mod paths;
mod probe;
pub mod rules;
mod tree;

use std::fs;
use std::path::Path;

use crate::fail::{Fail, OrFail, Result};

/// What the check prints when the tree passes.
pub const PASSED: &str = "Policy checks passed (first-party code and approved package versions).";

/// Checks the workspace at `root`; with `compiler_probe`, also that the
/// compiler rejects unsafe code under the forbid lint.
///
/// # Errors
/// The first rule the tree breaks, in that rule's fixed sentence.
pub fn check(root: &Path, compiler_probe: bool) -> Result<()> {
    let members = metadata::read(root, false)?;
    let manifest = fs::read_to_string(root.join("Cargo.toml"))
        .or_fail("Workspace manifest could not be read")?;
    if !rules::workspace_forbids_unsafe(&manifest) {
        return Err(Fail::new("Workspace unsafe prohibition is missing"));
    }
    metadata::check_members(&members)?;
    tree::check(&members.workspace_root)?;
    let graph = metadata::read(root, true)?;
    let approved = fs::read_to_string(root.join("docs/dependency-proposal/approved-packages.json"))
        .or_fail("Approved package inventory could not be read")?;
    metadata::check_dependencies(&graph, &approved)?;
    if compiler_probe {
        probe::run(root)?;
    }
    Ok(())
}
