//! `cargo xtask cards-ingest [bulk-path]`: imports Scryfall's `all-cards`
//! catalog into the local `dev-site` PostgreSQL cluster via
//! `central-service --cards-ingest-stdin`.

use std::env;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

use crate::fail::{Fail, OrFail, Result};
use crate::{process, workspace};

const PG_PORT: u16 = 55433;

pub(crate) fn run(root: &Path, bulk_path: Option<&str>) -> Result<()> {
    let dev_dir = root.join("target/dev-site");
    let state_path = dev_dir.join("state.json");
    let raw_state = fs::read_to_string(&state_path).or_fail(
        "Dev-site state missing — start `cargo xtask dev-site` first so PostgreSQL is running",
    )?;
    let state: Value =
        serde_json::from_str(&raw_state).or_fail("Dev-site state.json could not be parsed")?;
    let migrator_pw = state["db_passwords"]["migrator"]
        .as_str()
        .or_fail("Dev-site state missing migrator credential")?;

    let staged_service = dev_dir
        .join("bin/central-service")
        .with_extension(env::consts::EXE_EXTENSION);
    let service_bin = if staged_service.is_file() {
        staged_service
    } else {
        process::succeed(
            Command::new(workspace::cargo())
                .args(["build", "--locked", "--offline", "-p", "central-service"])
                .current_dir(root),
            "Building central-service for cards-ingest failed",
        )?;
        root.join("target/debug/central-service")
            .with_extension(env::consts::EXE_EXTENSION)
    };

    let bulk_abs: Option<PathBuf> = bulk_path.map(|p| {
        let path = PathBuf::from(p);
        if path.is_absolute() {
            path
        } else {
            env::current_dir()
                .unwrap_or_else(|_| root.to_path_buf())
                .join(path)
        }
    });

    let mut payload = json!({
        "database": {
            "loopback_fixture": {
                "port": PG_PORT,
                "user": "central_migrator",
                "password": migrator_pw,
                "name": "central_dev",
            }
        }
    });
    if let Some(path) = bulk_abs {
        payload["bulk_path"] = json!(path);
    }
    let encoded =
        serde_json::to_vec(&payload).or_fail("Serializing cards-ingest settings failed")?;

    let mut cmd = Command::new(&service_bin);
    cmd.arg("--cards-ingest-stdin")
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    let mut child = cmd.spawn().or_fail("Spawning central-service failed")?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(&encoded)
            .and_then(|()| stdin.flush())
            .or_fail("Writing settings to central-service failed")?;
    }
    let status = child
        .wait()
        .or_fail("Waiting for central-service --cards-ingest-stdin failed")?;
    if !status.success() {
        return Err(Fail::new("Card catalog import failed"));
    }
    Ok(())
}
