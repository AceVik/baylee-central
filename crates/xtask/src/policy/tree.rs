//! The walk over the whole reviewed tree: no link anywhere, no native or
//! assembly source, no unsafe marker in a Rust file. Skipped are only tool
//! output: `.git`, `target` and `node_modules` at the root (any case), and
//! `web/node_modules` exactly, as a real directory (a link of that name is
//! refused like any other), whose `.bin` npm fills with links.

use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};

use super::rules;
use crate::fail::{Fail, OrFail, Result};

const ROOT_SKIPS: [&str; 3] = [".git", "target", "node_modules"];

pub(super) fn check(root: &Path) -> Result<()> {
    let web = root.join("web");
    let mut pending = vec![root.to_path_buf()];
    let mut files: Vec<(PathBuf, String)> = Vec::new();
    while let Some(dir) = pending.pop() {
        let mut entries: Vec<_> = fs::read_dir(&dir)
            .or_fail("Source tree could not be read")?
            .collect::<std::io::Result<_>>()
            .or_fail("Source tree could not be read")?;
        entries.sort_by_key(fs::DirEntry::file_name);
        for entry in entries {
            let name = entry.file_name().to_string_lossy().into_owned();
            if dir == root
                && ROOT_SKIPS
                    .iter()
                    .any(|skip| name.eq_ignore_ascii_case(skip))
            {
                continue;
            }
            let meta =
                fs::symlink_metadata(entry.path()).or_fail("Source tree could not be read")?;
            if is_link(&meta) {
                return Err(Fail::new(format!(
                    "Linked source outside the review boundary: {name}"
                )));
            }
            if dir == web && name == "node_modules" && meta.is_dir() {
                continue;
            }
            if meta.is_dir() {
                pending.push(entry.path());
            } else {
                files.push((entry.path(), name));
            }
        }
    }
    // every link is refused before any file is read, as the PowerShell walk does
    for (path, name) in files {
        if rules::is_native(&name) {
            return Err(Fail::new(format!(
                "Native/assembly source forbidden: {name}"
            )));
        }
        if rules::is_rust(&name) {
            let source = fs::read(&path).or_fail("Source tree could not be read")?;
            if rules::has_unsafe_marker(&String::from_utf8_lossy(&source)) {
                return Err(Fail::new(format!(
                    "Forbidden unsafe/assembly marker: {name}"
                )));
            }
        }
    }
    Ok(())
}

/// A symbolic link, or on Windows any reparse point (a junction too).
fn is_link(meta: &Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    meta.file_type().is_symlink()
}
