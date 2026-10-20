//! The fixture's roles: a migration login and a runtime login with random
//! passwords, a `NOINHERIT` login that is a member of the migrator (to prove
//! membership grants nothing), and two databases closed to `PUBLIC`.

use super::Fixture;
use super::container::docker;
use crate::fail::{Fail, OrFail, Result};
use crate::{process, random};

/// The fixture's passwords: generated here, given only through environment
/// variables and standard input. No `Debug`: they are never printed.
pub(super) struct Secrets {
    pub(super) admin: String,
    pub(super) migrator: String,
    pub(super) runtime: String,
}

impl Secrets {
    pub(super) fn new() -> Result<Self> {
        Ok(Self {
            admin: random::hex(32)?,
            migrator: random::hex(32)?,
            runtime: random::hex(32)?,
        })
    }
}

impl Fixture {
    /// The roles and databases, from SQL on standard input; whatever `psql`
    /// says is withheld, since it may echo a password.
    pub(super) fn bootstrap(&self, secrets: &Secrets) -> Result<()> {
        let id = self
            .container
            .as_deref()
            .or_fail("Fixture role bootstrap failed (diagnostics withheld)")?;
        let sql = format!(
            "CREATE ROLE central_migrator LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE
  NOREPLICATION NOBYPASSRLS PASSWORD '{migrator}';
CREATE ROLE central_runtime LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE
  NOREPLICATION NOBYPASSRLS PASSWORD '{runtime}';
CREATE ROLE central_inherited LOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE
  NOREPLICATION NOBYPASSRLS PASSWORD '{runtime}';
GRANT central_migrator TO central_inherited;
CREATE DATABASE central_test OWNER central_migrator;
CREATE DATABASE central_native_test OWNER central_migrator;
\\connect central_native_test
REVOKE CREATE, TEMP ON DATABASE central_native_test FROM PUBLIC;
REVOKE CREATE ON SCHEMA public FROM PUBLIC;
\\connect central_test
REVOKE CREATE, TEMP ON DATABASE central_test FROM PUBLIC;
REVOKE CREATE ON SCHEMA public FROM PUBLIC;
",
            migrator = secrets.migrator,
            runtime = secrets.runtime,
        );
        let failure = "Fixture role bootstrap failed (diagnostics withheld)";
        let output = process::feed(
            docker().args([
                "exec",
                "-i",
                id,
                "psql",
                "-X",
                "-v",
                "ON_ERROR_STOP=1",
                "-U",
                "postgres",
                "-d",
                "postgres",
            ]),
            sql.as_bytes(),
            failure,
        )?;
        if !output.status.success() {
            return Err(Fail::new(failure));
        }
        let version = process::capture(
            docker().args([
                "exec",
                id,
                "psql",
                "-X",
                "-At",
                "-U",
                "postgres",
                "-c",
                "SHOW server_version",
            ]),
            failure,
        )?;
        println!(
            "Testing real PostgreSQL {} with isolated migration/runtime roles.",
            String::from_utf8_lossy(&version.stdout).trim()
        );
        Ok(())
    }
}
