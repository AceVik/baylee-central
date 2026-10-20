//! The fixture's container: created from a pinned image under a random name
//! and an ownership label, read-only with tmpfs storage, bounded memory,
//! CPUs and processes, published on IPv4 loopback only.

use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use super::Fixture;
use super::roles::Secrets;
use crate::fail::{Fail, OrFail, Result};
use crate::process;

/// The official multi-architecture `PostgreSQL` 18 image, by digest, not tag.
const IMAGE: &str =
    "postgres@sha256:77f585114c32fbca283dc835b0596f4e52b51b4c6662d7810b2f4084f60a1873";
pub(super) const LABEL: &str = "baylee.central.test";
/// `PostgreSQL` refuses a key it does not own; the entrypoint copies it into
/// tmpfs as `postgres` first.
const ENTRY: &str = concat!(
    "install -o postgres -g postgres -m 600 /etc/central-tls/server.key /var/run/postgresql/server.key",
    " && exec docker-entrypoint.sh postgres -c ssl=on -c ssl_cert_file=/etc/central-tls/server.crt",
    " -c ssl_key_file=/var/run/postgresql/server.key",
);

pub(super) fn docker() -> Command {
    Command::new("docker")
}

pub(super) fn ensure_image() -> Result<()> {
    let present = docker()
        .args(["image", "inspect", IMAGE])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    if present.is_ok_and(|status| status.success()) {
        return Ok(());
    }
    process::succeed(
        docker().args(["pull", IMAGE]),
        "Pinned PostgreSQL image unavailable",
    )
}

impl Fixture {
    pub(super) fn create(&mut self, tls: &Path, secrets: &Secrets) -> Result<()> {
        let failure = "Creating isolated PostgreSQL fixture failed";
        let mount = format!(
            "type=bind,src={},dst=/etc/central-tls,readonly",
            tls.display()
        );
        let label = format!("{LABEL}=owned");
        let output = process::capture(
            docker()
                .args(["create", "--name", &self.name, "--label", &label])
                .args([
                    "--memory",
                    "512m",
                    "--cpus",
                    "2",
                    "--pids-limit",
                    "128",
                    "--read-only",
                ])
                .args([
                    "--tmpfs",
                    "/var/lib/postgresql:rw,nosuid,nodev,size=268435456",
                ])
                .args([
                    "--tmpfs",
                    "/var/run/postgresql:rw,nosuid,nodev",
                    "--tmpfs",
                    "/tmp:rw,nosuid,nodev",
                ])
                .args(["--mount", &mount, "--publish", "127.0.0.1::5432"])
                .args([
                    "--env",
                    "POSTGRES_PASSWORD",
                    "--env",
                    "PGDATA=/var/lib/postgresql/test",
                ])
                .args(["--entrypoint", "sh", IMAGE, "-c", ENTRY])
                // the only way the admin password leaves this process
                .env("POSTGRES_PASSWORD", &secrets.admin),
            failure,
        )?;
        let id = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        if !output.status.success() {
            return Err(Fail::new(failure));
        }
        if !is_container_id(&id) {
            return Err(Fail::new("Invalid owned container identity"));
        }
        self.container = Some(id);
        Ok(())
    }

    /// Starts the container, waits until it accepts connections, and
    /// returns its port, which must be on IPv4 loopback alone.
    pub(super) fn start(&mut self) -> Result<u16> {
        let id = self
            .container
            .clone()
            .or_fail("Starting PostgreSQL fixture failed")?;
        process::succeed(
            docker().args(["start", &id]).stdout(Stdio::null()),
            "Starting PostgreSQL fixture failed",
        )?;
        let ready = (0..50).any(|_| {
            let ok = docker()
                .args([
                    "exec",
                    &id,
                    "pg_isready",
                    "-q",
                    "-h",
                    "127.0.0.1",
                    "-U",
                    "postgres",
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|s| s.success());
            if !ok {
                thread::sleep(Duration::from_millis(500));
            }
            ok
        });
        if !ready {
            return Err(Fail::new("PostgreSQL fixture did not become ready"));
        }
        let failure = "Fixture must publish exclusively on IPv4 loopback";
        let output = process::capture(docker().args(["port", &id, "5432/tcp"]), failure)?;
        let text = String::from_utf8_lossy(&output.stdout);
        let port = text
            .trim()
            .strip_prefix("127.0.0.1:")
            .and_then(|p| p.parse::<u16>().ok());
        let port = port.filter(|_| output.status.success()).or_fail(failure)?;
        self.port = Some(port);
        Ok(port)
    }
}

pub(super) fn is_container_id(id: &str) -> bool {
    id.len() == 64 && id.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}
