//! Removing what the fixture owns, each removal proved: the container (only
//! one that carries this run's name, the ownership label and the created
//! id, confirmed gone by a second inventory), the port, the TLS material.

use std::process::Stdio;

use super::Fixture;
use super::container::{LABEL, docker, is_container_id};
use crate::fail::{Fail, OrFail, Result};
use crate::{process, socket};

impl Fixture {
    /// Removes what the fixture owns and proves each removal; every step runs
    /// even when an earlier one failed.
    pub(super) fn clean(&mut self) -> Result<()> {
        let mut failures = Vec::new();
        if self.remove_container().is_err() {
            failures.push("Owned PostgreSQL container cleanup could not be verified");
        }
        if self.port.is_some_and(|port| !socket::refused(port)) {
            failures.push("Owned PostgreSQL listener closure could not be verified");
        }
        if self.tls.take().is_some_and(|dir| dir.remove().is_err()) {
            failures.push("Synthetic TLS material cleanup could not be verified");
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(Fail::new(failures.join("; ")))
        }
    }

    /// Found by the name chosen before `create` (which also covers an
    /// interrupted create), removed only if it carries the ownership label
    /// and is the container this run created.
    fn remove_container(&self) -> Result<()> {
        let failure = "container cleanup";
        let filter = format!("name=^/{}$", self.name);
        let format = format!("{{{{.ID}}}} {{{{.Label \"{LABEL}\"}}}}");
        let found = process::capture(
            docker().args([
                "ps",
                "-a",
                "--no-trunc",
                "--filter",
                &filter,
                "--format",
                &format,
            ]),
            failure,
        )?;
        if !found.status.success() {
            return Err(Fail::new(failure));
        }
        let text = String::from_utf8_lossy(&found.stdout).trim().to_owned();
        if text.is_empty() {
            return Ok(());
        }
        let owned = text
            .strip_suffix(" owned")
            .filter(|id| is_container_id(id))
            .or_fail(failure)?;
        if self.container.as_deref().is_some_and(|id| id != owned) {
            return Err(Fail::new(failure));
        }
        process::succeed(
            docker()
                .args(["rm", "--force", "--volumes", owned])
                .stdout(Stdio::null()),
            failure,
        )?;
        let filter = format!("id={owned}");
        let remaining = process::capture(
            docker().args([
                "ps",
                "-a",
                "--no-trunc",
                "--filter",
                &filter,
                "--format",
                "{{.ID}}",
            ]),
            failure,
        )?;
        if remaining.status.success()
            && String::from_utf8_lossy(&remaining.stdout).trim().is_empty()
        {
            Ok(())
        } else {
            Err(Fail::new(failure))
        }
    }
}
