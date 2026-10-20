//! A directory a task makes for itself under the system's temporary
//! directory, with a random name, and removes again. Removal checks first
//! that the path is still under the temporary directory, and never follows
//! a link inside it (`std::fs::remove_dir_all` removes links, not targets).

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::fail::{Fail, OrFail, Result};
use crate::random;

/// A directory this process made and owns.
#[derive(Debug)]
pub struct OwnedDir {
    path: Option<PathBuf>,
}

impl OwnedDir {
    /// A new, empty directory named `prefix` and 16 random hex digits.
    ///
    /// # Errors
    /// When there is no randomness or the directory cannot be made (also when
    /// the name exists already: it is never reused).
    pub fn new(prefix: &str) -> Result<Self> {
        let path = env::temp_dir().join(format!("{prefix}{}", random::hex(8)?));
        fs::create_dir(&path).or_fail("Temporary directory could not be created")?;
        Ok(Self { path: Some(path) })
    }

    /// Where it is.
    ///
    /// # Panics
    /// Never: the path is only taken by `remove`, which consumes `self`.
    #[must_use]
    pub fn path(&self) -> &Path {
        self.path
            .as_deref()
            .expect("an owned directory has a path until removed")
    }

    /// Removes it, saying so when that fails.
    ///
    /// # Errors
    /// When the path is no longer under the temporary directory or cannot be
    /// removed.
    pub fn remove(mut self) -> Result<()> {
        match self.path.take() {
            Some(path) => remove_owned(&path),
            None => Ok(()),
        }
    }
}

impl Drop for OwnedDir {
    fn drop(&mut self) {
        if let Some(path) = self.path.take() {
            // best effort on an early return; `remove` is the checked way
            let _ = remove_owned(&path);
        }
    }
}

fn remove_owned(path: &Path) -> Result<()> {
    let temp = env::temp_dir()
        .canonicalize()
        .or_fail("Temporary directory unavailable")?;
    let parent = path.parent().and_then(|p| p.canonicalize().ok());
    if parent.as_deref() != Some(temp.as_path()) {
        return Err(Fail::new("Unsafe temporary cleanup path"));
    }
    if fs::symlink_metadata(path).is_err() {
        return Ok(());
    }
    fs::remove_dir_all(path).or_fail("Temporary directory could not be removed")
}
