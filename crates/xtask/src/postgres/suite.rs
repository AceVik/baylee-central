//! The ignored database tests of `central-postgres` and `central-client`
//! against the fixture, with no ambient database credential, for at most
//! five minutes per suite, and both suites required to have run. Compilation
//! and a slower native suite must not consume the store suite's entire deadline.

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use super::roles::Secrets;
use crate::fail::{Fail, Result};
use crate::process::Watched;
use crate::workspace;

const PASSED: &str = "test result: ok. 1 passed; 0 failed";

pub(super) fn run(root: &Path, port: u16, secrets: &Secrets, tls: &Path) -> Result<()> {
    for package in ["central-client", "central-postgres"] {
        run_suite(root, port, secrets, tls, package)?;
    }
    Ok(())
}

fn run_suite(root: &Path, port: u16, secrets: &Secrets, tls: &Path, package: &str) -> Result<()> {
    println!("Running {package}'s PostgreSQL integration suite.");
    let mut command = Command::new(workspace::cargo());
    command
        .args(["test", "--locked", "-p", package, "--test", "postgres"])
        .args(["--", "--ignored", "--test-threads=1"])
        .current_dir(root)
        .env_remove("POSTGRES_PASSWORD")
        .env_remove("PGPASSWORD")
        .env_remove("DATABASE_URL")
        .env("CENTRAL_TEST_FIXTURE", "isolated-v1")
        .env("CENTRAL_TEST_PORT", port.to_string())
        .env("CENTRAL_TEST_MIGRATOR_PASSWORD", &secrets.migrator)
        .env("CENTRAL_TEST_RUNTIME_PASSWORD", &secrets.runtime)
        .env("CENTRAL_TEST_LEGACY_PASSWORD", &secrets.legacy)
        .env("CENTRAL_TEST_CA_PEM", tls.join("ca.pem"))
        .env("CENTRAL_TEST_WRONG_CA_PEM", tls.join("wrong.pem"));
    let tests = Watched::start(&mut command, "PostgreSQL integration tests failed")?;
    let (status, stdout, stderr) = tests.finish(
        Duration::from_secs(300),
        "PostgreSQL integration tests timed out",
    )?;
    let stdout = String::from_utf8_lossy(&stdout);
    println!("{stdout}");
    if !status.success() {
        println!("{}", String::from_utf8_lossy(&stderr));
        return Err(Fail::new("PostgreSQL integration tests failed"));
    }
    if stdout.matches(PASSED).count() != 1 {
        return Err(Fail::new(
            "Expected PostgreSQL integration suite did not run",
        ));
    }
    Ok(())
}
