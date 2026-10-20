//! Proof that the scanner's configuration still finds what it must
//! (`scripts/test-secret-scanner.ps1`): in a synthetic repository, three
//! random credentials, one in an ordinary file, one in a `*.local.json` and
//! one under `target/`. The working-tree configuration must report the
//! ordinary one and only that (its path exclusions hold); after the three are
//! deleted, the history configuration must still report all three (history
//! has no exclusions). Git runs with no system or user configuration, hooks
//! or signing, and none of the caller's Git variables.

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::Value;

use super::git::Git;
use super::{REROUTED, ROUTING, reject_rerouting};
use crate::fail::{Fail, OrFail, Result, after_cleanup};
use crate::owned::OwnedDir;
use crate::random;

const PATHS: [&str; 3] = ["config.local.json", "target/config.json", "source.txt"];
const SEEDING: &str = "Secret-scanner fixture initialization failed";

pub(super) fn run(project: &Path, binary: &Path) -> Result<()> {
    for name in ROUTING {
        let refused = reject_rerouting([name.into()]).map_err(|f| f.message().to_owned());
        if refused != Err(REROUTED.to_owned()) {
            return Err(Fail::new(
                "Secret scanner accepted overridden repository scope",
            ));
        }
    }
    let fixture = OwnedDir::new("central-secret-scan-")?;
    let outcome = exercise(project, binary, fixture.path());
    after_cleanup(outcome, fixture.remove())?;
    println!(
        "Secret-scanner regressions passed: committed/deleted local metadata and output paths are scanned."
    );
    Ok(())
}

fn exercise(project: &Path, binary: &Path, fixture: &Path) -> Result<()> {
    let git = Git::isolated(fixture)?;
    let repo = fixture.join("repository");
    git.init(&repo)?;
    seed(&repo)?;
    git.run(
        &repo,
        "Secret-scanner fixture staging failed",
        &["add", "--force", "."],
    )?;
    git.commit(
        &repo,
        "Synthetic scanner fixtures",
        "Secret-scanner fixture commit failed",
    )?;

    let scanner = Scanner {
        binary,
        fixture,
        repo: &repo,
    };
    let working = scanner
        .scan("dir", &project.join(".gitleaks.toml"))
        .or_fail("Working-tree scanner missed its positive control")?;
    if working.len() != 1 || relative(&repo, &working[0]) != "source.txt" {
        return Err(Fail::new("Working-tree exclusions changed unexpectedly"));
    }

    let mut remove = vec!["rm", "--quiet", "--force", "--"];
    remove.extend(PATHS);
    git.run(&repo, "Secret-scanner fixture deletion failed", &remove)?;
    git.commit(
        &repo,
        "Remove synthetic credentials from current tree",
        "Secret-scanner fixture deletion commit failed",
    )?;
    let history = scanner
        .scan("git", &project.join(".gitleaks-history.toml"))
        .or_fail("History scanner missed deleted synthetic credentials")?;
    if history.len() != PATHS.len() {
        return Err(Fail::new("History scanner retained a path exclusion"));
    }
    if PATHS
        .iter()
        .any(|path| !history.iter().any(|file| file == path))
    {
        return Err(Fail::new(
            "History scanner skipped a synthetic credential path",
        ));
    }
    Ok(())
}

/// A random credential in each of the three paths.
fn seed(repo: &Path) -> Result<()> {
    for path in PATHS {
        let file = repo.join(path);
        fs::create_dir_all(file.parent().or_fail(SEEDING)?).or_fail(SEEDING)?;
        let canary = random::hex(32)?.to_uppercase();
        fs::write(&file, format!("{{\"api_key\":\"{canary}\"}}")).or_fail(SEEDING)?;
    }
    Ok(())
}

struct Scanner<'a> {
    binary: &'a Path,
    fixture: &'a Path,
    repo: &'a Path,
}

impl Scanner<'_> {
    /// The files the scanner reports, or `None` unless it exits with 1 (leaks
    /// found) and writes a readable report. Its output is withheld, redacted.
    fn scan(&self, mode: &str, config: &Path) -> Option<Vec<String>> {
        let report = self.fixture.join(format!("{mode}-report.json"));
        let mut command = Command::new(self.binary);
        command.arg(mode).arg(self.repo);
        if mode == "git" {
            command.arg("--log-opts=--all");
        }
        let status = command
            .arg("--config")
            .arg(config)
            .args(["--redact=100", "--no-banner", "--report-format=json"])
            .arg(format!("--report-path={}", report.display()))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .ok()?;
        if status.code() != Some(1) {
            return None;
        }
        let findings: Value = serde_json::from_slice(&fs::read(report).ok()?).ok()?;
        let files = findings
            .as_array()?
            .iter()
            .map(|f| f["File"].as_str().map(str::to_owned));
        files.collect()
    }
}

/// A reported file (relative to the repository or absolute, where the
/// temporary directory may be reported through a link, as on macOS) as a
/// path relative to the repository, with `/`.
fn relative(repo: &Path, file: &str) -> String {
    let canonical = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    let path = canonical(&repo.join(file));
    path.strip_prefix(canonical(repo)).map_or_else(
        |_| file.to_owned(),
        |p| p.to_string_lossy().replace('\\', "/"),
    )
}
