//! `central-web-crypto`'s WebAssembly module and its JavaScript glue, which
//! are committed under `web/src/central-wasm/` so the website builds without
//! Rust (`docs/xtask.md`, `docs/web-accounts.md` §"Browser crypto").
//!
//! The glue comes from the wasm-bindgen CLI at the crate's own version,
//! installed once with `cargo install --locked` into the target directory's
//! `wasm-bindgen-cli/<version>`. The build remaps the repository and Cargo's
//! home out of the module, so it holds no local path. It runs on x86-64
//! Linux (glibc) only, CI's host: cargo folds the build host into every
//! crate's `-C metadata`, which changes symbol names and with them the
//! module's layout, so another host builds other bytes (the container recipe
//! in `docs/xtask.md` serves every other machine). `wasm` writes the files
//! and their SHA-256 sums; `wasm --check` builds into a temporary directory
//! and requires the committed files to equal that build byte for byte.

use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

use crate::fail::{Fail, OrFail, Result, after_cleanup};
use crate::owned::OwnedDir;
use crate::{process, random, workspace};

/// The version of the `wasm-bindgen` dependency and of the CLI that must
/// match it.
pub const BINDGEN: &str = "0.2.129";
/// The committed files, relative to the repository.
pub const OUTPUT: &str = "web/src/central-wasm";
/// What wasm-bindgen writes for the crate, in the order the sums list them.
pub const GENERATED: [&str; 4] = [
    "central_web_crypto.d.ts",
    "central_web_crypto.js",
    "central_web_crypto_bg.wasm",
    "central_web_crypto_bg.wasm.d.ts",
];
/// The sums file beside them.
pub const SUMS: &str = "SHA256SUMS";
/// Files in the committed directory that the build does not write.
const KEPT: [&str; 1] = ["README.md"];

const TARGET: &str = "wasm32-unknown-unknown";
const PROFILE: &str = "wasm";

/// Builds the module and writes the glue and its sums into the tree, or with
/// `check` compares a fresh build with what is committed.
///
/// # Errors
/// The step that failed, or with `check` the first difference.
pub(crate) fn run(root: &Path, check: bool) -> Result<()> {
    if !canonical_host() {
        return Err(Fail::new(
            "The WebAssembly module is built on x86-64 Linux only; elsewhere use the container (docs/xtask.md)",
        ));
    }
    let target = target_dir(root);
    let bindgen = cli(&target)?;
    let module = build(root, &target)?;
    if !check {
        let out = root.join(OUTPUT);
        bindgen_into(&bindgen, &module, &out)?;
        fs::write(out.join(SUMS), sums(&out)?).or_fail("WebAssembly sums could not be written")?;
        println!("WebAssembly module and glue written to {OUTPUT}.");
        return Ok(());
    }
    let fresh = OwnedDir::new("central-wasm-")?;
    let outcome = bindgen_into(&bindgen, &module, fresh.path())
        .and_then(|()| compare(fresh.path(), &root.join(OUTPUT)));
    after_cleanup(outcome, fresh.remove())?;
    println!("Committed WebAssembly module and glue match a fresh build.");
    Ok(())
}

/// The host whose build is the committed one: CI's.
fn canonical_host() -> bool {
    cfg!(all(
        target_os = "linux",
        target_arch = "x86_64",
        target_env = "gnu"
    ))
}

/// Cargo's target directory: `CARGO_TARGET_DIR` (a container's own, so it
/// never replaces the host's binaries), else `target`.
fn target_dir(root: &Path) -> PathBuf {
    env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), |dir| root.join(dir))
}

/// The pinned wasm-bindgen CLI, installed on first use.
fn cli(target: &Path) -> Result<PathBuf> {
    let dir = target.join("wasm-bindgen-cli").join(BINDGEN);
    let program = dir
        .join("bin")
        .join(format!("wasm-bindgen{}", env::consts::EXE_SUFFIX));
    if !version_matches(&program) {
        process::succeed(
            Command::new(workspace::cargo())
                .args(["install", "--locked", "--force", "wasm-bindgen-cli"])
                .arg("--version")
                .arg(format!("={BINDGEN}"))
                .arg("--root")
                .arg(&dir),
            "wasm-bindgen CLI could not be installed",
        )?;
    }
    if !version_matches(&program) {
        return Err(Fail::new("wasm-bindgen CLI has the wrong version"));
    }
    Ok(program)
}

fn version_matches(program: &Path) -> bool {
    Command::new(program)
        .arg("--version")
        .output()
        .is_ok_and(|out| {
            out.status.success()
                && out.stdout.trim_ascii() == format!("wasm-bindgen {BINDGEN}").as_bytes()
        })
}

/// Cargo's home, whose registry sources the module would otherwise name.
fn cargo_home() -> Result<PathBuf> {
    if let Some(home) = env::var_os("CARGO_HOME") {
        return Ok(PathBuf::from(home));
    }
    let user = env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .or_fail("Cargo home could not be found")?;
    Ok(PathBuf::from(user).join(".cargo"))
}

/// The remapping flags, as `CARGO_ENCODED_RUSTFLAGS` (separated by 0x1f).
fn remap_flags(root: &Path, home: &Path) -> OsString {
    let mut flags = OsString::from("--remap-path-prefix=");
    flags.push(home);
    flags.push("=/cargo\u{1f}--remap-path-prefix=");
    flags.push(root);
    flags.push("=/central");
    flags
}

/// Builds the crate for the browser; the path of the module.
fn build(root: &Path, target: &Path) -> Result<PathBuf> {
    let home = cargo_home()?;
    process::succeed(
        Command::new(workspace::cargo())
            .args([
                "build",
                "--locked",
                "-p",
                "central-web-crypto",
                "--target",
                TARGET,
                "--profile",
                PROFILE,
            ])
            .env("CARGO_ENCODED_RUSTFLAGS", remap_flags(root, &home))
            .current_dir(root),
        "WebAssembly build failed",
    )?;
    let module = target
        .join(TARGET)
        .join(PROFILE)
        .join("central_web_crypto.wasm");
    let bytes = fs::read(&module).or_fail("WebAssembly build failed")?;
    for local in [root, home.as_path()] {
        if contains(&bytes, &with_separator(local)) {
            return Err(Fail::new("WebAssembly module names a local path"));
        }
    }
    Ok(module)
}

/// `dir` and a separator, as a path inside it starts (`/home/ci/.cargo/`):
/// so a URL like `https://central.baylee.app` is not taken for `/central`.
fn with_separator(dir: &Path) -> Vec<u8> {
    let mut bytes = dir.to_string_lossy().into_owned().into_bytes();
    bytes.push(std::path::MAIN_SEPARATOR as u8);
    bytes
}

/// Whether `needle` occurs in `haystack`.
pub fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

fn bindgen_into(bindgen: &Path, module: &Path, out: &Path) -> Result<()> {
    process::succeed(
        Command::new(bindgen)
            .args([
                "--target",
                "web",
                "--remove-name-section",
                "--remove-producers-section",
                "--out-dir",
            ])
            .arg(out)
            .arg(module),
        "wasm-bindgen failed",
    )
}

/// `SHA256SUMS` for the generated files in `dir`: `<hex>  <name>` lines.
fn sums(dir: &Path) -> Result<String> {
    let mut text = String::new();
    for name in GENERATED {
        let bytes = fs::read(dir.join(name)).or_fail("WebAssembly glue is incomplete")?;
        text.push_str(&random::to_hex(&Sha256::digest(&bytes)));
        text.push_str("  ");
        text.push_str(name);
        text.push('\n');
    }
    Ok(text)
}

/// The committed directory holds exactly the fresh build's files, equal byte
/// for byte, their sums, and the files kept by hand.
fn compare(fresh: &Path, committed: &Path) -> Result<()> {
    let mut names: Vec<String> = fs::read_dir(committed)
        .or_fail("Committed WebAssembly glue is missing")?
        .map(|entry| entry.map(|e| e.file_name().to_string_lossy().into_owned()))
        .collect::<std::io::Result<_>>()
        .or_fail("Committed WebAssembly glue is missing")?;
    names.sort();
    let mut expected: Vec<String> = GENERATED
        .iter()
        .chain(&[SUMS])
        .chain(&KEPT)
        .map(|name| (*name).to_owned())
        .collect();
    expected.sort();
    if names != expected {
        return Err(Fail::new(
            "Committed WebAssembly glue has other files than the build",
        ));
    }
    for name in GENERATED {
        let built = fs::read(fresh.join(name)).or_fail("wasm-bindgen failed")?;
        let kept =
            fs::read(committed.join(name)).or_fail("Committed WebAssembly glue is missing")?;
        if built != kept {
            return Err(Fail::new(difference(name, &kept, &built)));
        }
    }
    let kept = fs::read_to_string(committed.join(SUMS))
        .or_fail("Committed WebAssembly sums are missing")?;
    if kept != sums(fresh)? {
        return Err(Fail::new(
            "Committed WebAssembly sums differ from a fresh build",
        ));
    }
    Ok(())
}

/// Where a committed file and a fresh build part: sizes and the first
/// differing byte, so a mismatch on another machine says where to look.
fn difference(name: &str, kept: &[u8], built: &[u8]) -> String {
    let at = kept
        .iter()
        .zip(built)
        .position(|(a, b)| a != b)
        .unwrap_or_else(|| kept.len().min(built.len()));
    format!(
        "Committed {name} differs from a fresh build ({} bytes committed, {} built, first difference at byte {at})",
        kept.len(),
        built.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_difference_names_both_sizes_and_the_first_byte() {
        assert_eq!(
            difference("m.wasm", b"abcd", b"abXd"),
            "Committed m.wasm differs from a fresh build (4 bytes committed, 4 built, first difference at byte 2)"
        );
        assert_eq!(
            difference("m.wasm", b"ab", b"abc"),
            "Committed m.wasm differs from a fresh build (2 bytes committed, 3 built, first difference at byte 2)"
        );
    }

    #[test]
    fn a_directory_is_found_only_as_the_start_of_a_path() {
        let root = with_separator(Path::new("/central"));
        assert!(!contains(b"https://central.baylee.app", &root));
        // a path inside it, spelled with this platform's separator
        let inside = Path::new("/central").join("crates").join("lib.rs");
        let mut haystack = b"\0".to_vec();
        haystack.extend_from_slice(inside.to_string_lossy().as_bytes());
        assert!(contains(&haystack, &root));
    }

    #[test]
    fn local_paths_are_found_anywhere_in_the_module() {
        assert!(contains(
            b"\0\0/home/ci/.cargo/registry/src",
            b"/home/ci/.cargo"
        ));
        assert!(contains(b"/home/ci/.cargo", b"/home/ci/.cargo"));
        assert!(!contains(b"/cargo/registry/src", b"/home/ci/.cargo"));
        assert!(!contains(b"anything", b""));
    }

    #[test]
    fn remapping_names_both_prefixes_in_one_encoded_list() {
        let flags = remap_flags(Path::new("/w/central"), Path::new("/h/.cargo"));
        let flags = flags.to_string_lossy();
        let parts: Vec<&str> = flags.split('\u{1f}').collect();
        assert_eq!(
            parts,
            [
                "--remap-path-prefix=/h/.cargo=/cargo",
                "--remap-path-prefix=/w/central=/central"
            ]
        );
    }

    #[test]
    fn a_tree_differing_by_one_byte_or_one_file_is_refused() {
        let fresh = OwnedDir::new("central-wasm-test-").unwrap();
        let committed = OwnedDir::new("central-wasm-test-").unwrap();
        for name in GENERATED {
            fs::write(fresh.path().join(name), name).unwrap();
            fs::write(committed.path().join(name), name).unwrap();
        }
        fs::write(committed.path().join(SUMS), sums(fresh.path()).unwrap()).unwrap();
        fs::write(committed.path().join("README.md"), "kept").unwrap();
        compare(fresh.path(), committed.path()).unwrap();

        fs::write(
            committed.path().join(GENERATED[2]),
            "central_web_crypto_bg.wasM",
        )
        .unwrap();
        assert!(compare(fresh.path(), committed.path()).is_err());
        fs::write(committed.path().join(GENERATED[2]), GENERATED[2]).unwrap();

        fs::write(committed.path().join(SUMS), "0  central_web_crypto.js\n").unwrap();
        assert!(compare(fresh.path(), committed.path()).is_err());
        fs::write(committed.path().join(SUMS), sums(fresh.path()).unwrap()).unwrap();

        fs::write(committed.path().join("extra.js"), "").unwrap();
        assert!(compare(fresh.path(), committed.path()).is_err());
        fs::remove_file(committed.path().join("extra.js")).unwrap();
        compare(fresh.path(), committed.path()).unwrap();

        fresh.remove().unwrap();
        committed.remove().unwrap();
    }
}
