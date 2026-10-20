//! Where the repository is, and which cargo to call.

use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::fail::{OrFail, Result};

/// The repository root: two levels above this crate's manifest.
pub(crate) fn root() -> Result<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .or_fail("Repository root could not be found")
}

/// The cargo that runs this task (cargo sets `CARGO`), else the one on `PATH`.
pub(crate) fn cargo() -> OsString {
    env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"))
}
