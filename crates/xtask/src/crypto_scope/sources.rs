//! Fingerprint the reviewed crate, including new files, with portable LF text.

use std::{collections::BTreeMap, fmt::Write, fs, path::Path};

use sha2::{Digest, Sha256};

use crate::fail::{Fail, OrFail, Result};

const REVIEWED: &str = "ddbb34985086f0d853bc6f23db4c8b9acd4e11ffd853d5fba976d6806762d87f";
const FAILURE: &str = "Crypto sources changed; review RUSTSEC-2026-0387 again";

pub(super) fn verify(root: &Path) -> Result<()> {
    if fingerprint(root)? != REVIEWED {
        return Err(Fail::new(FAILURE));
    }
    Ok(())
}

fn fingerprint(root: &Path) -> Result<String> {
    let mut files = BTreeMap::new();
    read(root, "", &mut files)?;
    let mut hash = Sha256::new();
    for (name, content) in files {
        for field in [name.as_bytes(), content.as_bytes()] {
            hash.update((field.len() as u64).to_be_bytes());
            hash.update(field);
        }
    }
    let mut hex = String::with_capacity(64);
    for byte in hash.finalize() {
        write!(hex, "{byte:02x}").or_fail(FAILURE)?;
    }
    Ok(hex)
}

fn read(path: &Path, relative: &str, files: &mut BTreeMap<String, String>) -> Result<()> {
    let kind = fs::symlink_metadata(path).or_fail(FAILURE)?.file_type();
    if kind.is_symlink() {
        return Err(Fail::new(FAILURE));
    }
    if kind.is_dir() {
        for entry in fs::read_dir(path).or_fail(FAILURE)? {
            let entry = entry.or_fail(FAILURE)?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| Fail::new(FAILURE))?;
            let name = if relative.is_empty() {
                name
            } else {
                format!("{relative}/{name}")
            };
            read(&entry.path(), &name, files)?;
        }
    } else if kind.is_file() {
        files.insert(
            relative.to_owned(),
            fs::read_to_string(path)
                .or_fail(FAILURE)?
                .replace("\r\n", "\n"),
        );
    } else {
        return Err(Fail::new(FAILURE));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
