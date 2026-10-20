//! Git for the scanner's synthetic repository, with nothing of the caller's.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::fail::{OrFail, Result};
use crate::process;

const SEEDING: &str = "Secret-scanner fixture initialization failed";

/// Git with nothing of the caller's: no system or global configuration, no
/// hooks, no signing, no `GIT_*` variable.
pub(super) struct Git {
    empty: PathBuf,
}

impl Git {
    pub(super) fn isolated(fixture: &Path) -> Result<Self> {
        let empty = fixture.join("empty");
        fs::create_dir(&empty).or_fail(SEEDING)?;
        fs::write(empty.join("config"), "").or_fail(SEEDING)?;
        Ok(Self { empty })
    }

    fn command(&self) -> Command {
        let mut command = Command::new("git");
        for (name, _) in env::vars_os() {
            if name
                .to_string_lossy()
                .to_ascii_uppercase()
                .starts_with("GIT_")
            {
                command.env_remove(name);
            }
        }
        command
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", self.empty.join("config"))
            .arg("-c")
            .arg(format!("core.hooksPath={}", self.empty.display()))
            .args(["-c", "commit.gpgSign=false"])
            .args([
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
            ])
            .stdout(Stdio::null());
        command
    }

    pub(super) fn init(&self, repo: &Path) -> Result<()> {
        let template = format!("--template={}", self.empty.display());
        let mut command = self.command();
        command.args(["init", "--quiet", &template]).arg(repo);
        process::succeed(&mut command, SEEDING)
    }

    pub(super) fn run(&self, repo: &Path, failure: &str, args: &[&str]) -> Result<()> {
        let mut command = self.command();
        command.arg("-C").arg(repo).args(args);
        process::succeed(&mut command, failure)
    }

    pub(super) fn commit(&self, repo: &Path, message: &str, failure: &str) -> Result<()> {
        self.run(repo, failure, &["commit", "--quiet", "-m", message])
    }
}
