//! The bound of the RUSTSEC-2026-0459 exception (`libsqlite3-sys`, owner,
//! 12.11.2026, `docs/dependency-proposal/sqlite-advisory.md`): `cargo
//! audit` ignores that advisory only while the package is in `Cargo.lock`
//! and in no graph a build of this workspace resolves, with every feature
//! of every member, for every target, with build and dev dependencies. A
//! build that takes it, or a lock file that holds another version or none,
//! fails here, and the ignore has to be reviewed or removed.

use std::{fs, path::Path, process::Command};

use crate::fail::{Fail, OrFail, Result};
use crate::{process, workspace};

/// The advisory's package and the one version the exception was given for.
const PACKAGE: &str = "libsqlite3-sys";
const VERSION: &str = "0.37.0";
/// The package that would bring it into a build.
const DRIVER: &str = "sqlx-sqlite";

const BUILT: &str =
    "libsqlite3-sys is in a build of the workspace; RUSTSEC-2026-0459 may no longer be ignored";
const MOVED: &str = "libsqlite3-sys 0.37.0 is no longer what Cargo.lock holds; review or remove the RUSTSEC-2026-0459 ignore in .cargo/audit.toml";
const FAILURE: &str = "SQLite advisory scope could not be read";

pub(crate) fn check(root: &Path) -> Result<()> {
    let lock = fs::read_to_string(root.join("Cargo.lock")).or_fail(FAILURE)?;
    if locked_versions(&lock, PACKAGE) != [VERSION] {
        return Err(Fail::new(MOVED));
    }
    let output = process::capture(
        Command::new(workspace::cargo())
            .args([
                "tree",
                "--locked",
                "--offline",
                "--workspace",
                "--all-features",
                "--target",
                "all",
                "--edges",
                "all",
                "--prefix",
                "none",
                "--format",
                "{p}",
            ])
            .current_dir(root),
        FAILURE,
    )?;
    if !output.status.success() {
        return Err(Fail::new(FAILURE));
    }
    let tree = String::from_utf8(output.stdout).or_fail(FAILURE)?;
    // a tree that does not name the workspace's own crates is no proof
    if !names(&tree, "central-api") {
        return Err(Fail::new(FAILURE));
    }
    if names(&tree, PACKAGE) || names(&tree, DRIVER) {
        return Err(Fail::new(BUILT));
    }
    println!(
        "SQLite advisory scope passed (libsqlite3-sys 0.37.0 in Cargo.lock only, in no build)."
    );
    Ok(())
}

/// The versions `Cargo.lock` holds of `name`.
fn locked_versions<'a>(lock: &'a str, name: &str) -> Vec<&'a str> {
    let wanted = format!("name = \"{name}\"");
    let mut versions = Vec::new();
    let mut lines = lock.lines();
    while let Some(line) = lines.next() {
        if line.trim() != wanted {
            continue;
        }
        if let Some(version) = lines
            .next()
            .and_then(|next| next.trim().strip_prefix("version = \""))
            .and_then(|rest| rest.strip_suffix('"'))
        {
            versions.push(version);
        }
    }
    versions
}

/// Whether a line of `cargo tree --prefix none --format {p}` names `name`.
fn names(tree: &str, name: &str) -> bool {
    tree.lines()
        .any(|line| line.split_once(' ').is_some_and(|(first, _)| first == name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lock_file_is_read_by_name_and_version() {
        let lock = "[[package]]\nname = \"libsqlite3-sys\"\nversion = \"0.37.0\"\nsource = \"x\"\n\n[[package]]\nname = \"libsqlite3-sys-extra\"\nversion = \"1.0.0\"\n";
        assert_eq!(locked_versions(lock, PACKAGE), ["0.37.0"]);
        let two = format!("{lock}\n[[package]]\nname = \"libsqlite3-sys\"\nversion = \"0.39.0\"\n");
        assert_eq!(locked_versions(&two, PACKAGE), ["0.37.0", "0.39.0"]);
        assert_eq!(locked_versions("", PACKAGE), Vec::<&str>::new());
    }

    #[test]
    fn a_tree_names_a_package_by_its_first_word_only() {
        let tree =
            "central-api v0.1.0 (/src/crates/central-api)\nsqlx v0.9.0\nsqlx-postgres v0.9.0 (*)\n";
        assert!(names(tree, "central-api") && names(tree, "sqlx"));
        assert!(!names(tree, DRIVER) && !names(tree, PACKAGE));
        assert!(names("libsqlite3-sys v0.37.0 (*)\n", PACKAGE));
        assert!(!names("libsqlite3-sys-extra v1.0.0\n", PACKAGE));
    }
}
