//! The `PostgreSQL` integration fixture (`scripts/test-postgres.ps1`): a
//! pinned `PostgreSQL` 18 in an owned, read-only container on a loopback port,
//! serving TLS from a synthetic authority (`tls.rs`), with migration and
//! runtime roles under random passwords (`container.rs`); then the ignored
//! database tests of `central-postgres` and `central-client` against it
//! (`suite.rs`). Afterwards the container, its port and the TLS material
//! are removed and each removal is proved, also when a step failed.
//!
//! The passwords reach Docker and the tests only through environment
//! variables and standard input, never an argument, and are never printed.

mod cleanup;
mod container;
mod roles;
mod suite;
mod tls;

use std::path::Path;

use crate::fail::{Fail, Result, after_cleanup};
use crate::owned::OwnedDir;
use crate::random;

/// What the fixture owns, for its cleanup.
pub(super) struct Fixture {
    name: String,
    container: Option<String>,
    port: Option<u16>,
    tls: Option<OwnedDir>,
}

pub(crate) fn run(root: &Path, exercise_failure: bool) -> Result<()> {
    let name = format!("central-test-{}", random::hex(8)?);
    println!("Owned PostgreSQL fixture: {name}");
    let mut fixture = Fixture {
        name,
        container: None,
        port: None,
        tls: None,
    };
    let outcome = exercise(&mut fixture, root, exercise_failure);
    let cleanup = fixture.clean();
    after_cleanup(outcome, cleanup)?;
    println!("PostgreSQL fixture passed and its owned container/listener were removed.");
    Ok(())
}

fn exercise(fixture: &mut Fixture, root: &Path, exercise_failure: bool) -> Result<()> {
    container::ensure_image()?;
    let tls = fixture.tls.insert(tls::authority()?);
    let secrets = roles::Secrets::new()?;
    let tls_path = tls.path().to_path_buf();
    fixture.create(&tls_path, &secrets)?;
    let port = fixture.start()?;
    if exercise_failure {
        return Err(Fail::new("Synthetic fixture failure"));
    }
    fixture.bootstrap(&secrets)?;
    suite::run(root, port, &secrets, &tls_path)
}
