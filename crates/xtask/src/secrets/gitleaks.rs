//! gitleaks 8.30.1, downloaded once into `target/gitleaks` and checked
//! against a pinned SHA-256 on every run. The Linux and Windows x64 pins are
//! the ones `scripts/scan-secrets.ps1` carries; the others are from the
//! release's own checksum file, whose entries for those two match them.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

use crate::fail::{Fail, OrFail, Result};
use crate::{process, random};

const VERSION: &str = "8.30.1";

/// (os, arch, asset suffix, SHA-256)
const BUILDS: [(&str, &str, &str, &str); 5] = [
    (
        "linux",
        "x86_64",
        "linux_x64.tar.gz",
        "551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb",
    ),
    (
        "linux",
        "aarch64",
        "linux_arm64.tar.gz",
        "e4a487ee7ccd7d3a7f7ec08657610aa3606637dab924210b3aee62570fb4b080",
    ),
    (
        "windows",
        "x86_64",
        "windows_x64.zip",
        "d29144deff3a68aa93ced33dddf84b7fdc26070add4aa0f4513094c8332afc4e",
    ),
    (
        "macos",
        "aarch64",
        "darwin_arm64.tar.gz",
        "b40ab0ae55c505963e365f271a8d3846efbc170aa17f2607f13df610a9aeb6a5",
    ),
    (
        "macos",
        "x86_64",
        "darwin_x64.tar.gz",
        "dfe101a4db2255fc85120ac7f3d25e4342c3c20cf749f2c20a18081af1952709",
    ),
];

/// The checked gitleaks binary for this platform.
pub(super) fn fetch(root: &Path) -> Result<PathBuf> {
    let (os, _, suffix, expected) = BUILDS
        .iter()
        .find(|(os, arch, _, _)| *os == std::env::consts::OS && *arch == std::env::consts::ARCH)
        .or_fail("Secret scanner has no pinned build for this platform")?;
    let dir = root.join("target/gitleaks");
    fs::create_dir_all(&dir).or_fail("Secret scanner download failed")?;
    let asset = format!("gitleaks_{VERSION}_{suffix}");
    let archive = dir.join(&asset);
    if !archive.is_file() {
        let url =
            format!("https://github.com/gitleaks/gitleaks/releases/download/v{VERSION}/{asset}");
        process::succeed(
            Command::new("curl")
                .args(["-fsSL", "--proto", "=https", "--tlsv1.2", "-o"])
                .arg(&archive)
                .arg(&url),
            "Secret scanner download failed",
        )?;
    }
    let bytes = fs::read(&archive).or_fail("Secret scanner download failed")?;
    if random::to_hex(&Sha256::digest(&bytes)) != *expected {
        return Err(Fail::new("Secret scanner archive checksum mismatch"));
    }
    // Windows' build is a zip, which `tar -xf` (bsdtar, as Windows ships) unpacks
    let flags = if *os == "windows" { "-xf" } else { "-xzf" };
    process::succeed(
        Command::new("tar")
            .arg(flags)
            .arg(&archive)
            .arg("-C")
            .arg(&dir),
        "Secret scanner extraction failed",
    )?;
    let binary = dir
        .join("gitleaks")
        .with_extension(std::env::consts::EXE_EXTENSION);
    if binary.is_file() {
        Ok(binary)
    } else {
        Err(Fail::new("Secret scanner extraction failed"))
    }
}
