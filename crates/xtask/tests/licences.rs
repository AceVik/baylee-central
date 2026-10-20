//! The repository's licence split (`LICENSE.md` § 6): the three crates the
//! Baylee client links stay AGPL-3.0-only, everything else takes the
//! workspace's source-available licence.
#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};

/// Linked by the Baylee client, so AGPL: `LICENSE.md` § 6 names them.
const AGPL_CRATES: [&str; 3] = ["central-api", "central-client", "central-crypto"];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

#[test]
fn the_workspace_licence_is_the_source_available_one() {
    let manifest = read(&root().join("Cargo.toml"));
    assert!(
        manifest.contains("license = \"LicenseRef-Baylee-Central-Source-Available-1.0\"\n"),
        "workspace licence"
    );
    for file in ["LICENSE.md", "LICENSE-AGPL"] {
        assert!(root().join(file).is_file(), "{file}");
    }
    let licence = read(&root().join("LICENSE.md"));
    for name in AGPL_CRATES {
        assert!(
            licence.contains(&format!("`crates/{name}`")),
            "§ 6 names {name}"
        );
    }
}

#[test]
fn only_the_crates_the_client_links_are_agpl() {
    let crates = fs::read_dir(root().join("crates")).expect("crates/");
    let mut seen = 0;
    for entry in crates {
        let dir = entry.expect("an entry").path();
        let manifest = dir.join("Cargo.toml");
        if !manifest.is_file() {
            continue;
        }
        let name = dir
            .file_name()
            .and_then(|name| name.to_str())
            .expect("a crate directory name");
        let text = read(&manifest);
        if AGPL_CRATES.contains(&name) {
            seen += 1;
            assert!(
                text.contains("\nlicense = \"AGPL-3.0-only\"\n"),
                "{name} is AGPL"
            );
        } else {
            assert!(
                text.contains("\nlicense.workspace = true\n"),
                "{name} inherits"
            );
        }
    }
    assert_eq!(seen, AGPL_CRATES.len(), "every AGPL crate exists");
}
