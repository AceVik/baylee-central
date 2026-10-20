//! Comparing paths as `scripts/check-policy.ps1` does with
//! `GetFullPath(...).StartsWith(...)`: lexically, `..` resolved, never by
//! following a link.

use std::path::{Component, Path, PathBuf};

/// `path` with `.` dropped and every `..` applied to what precedes it.
pub(super) fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Whether `path` is strictly inside `dir`, comparing names exactly or,
/// with `ignore_case`, ASCII-case-insensitively.
pub(super) fn inside(path: &Path, dir: &Path, ignore_case: bool) -> bool {
    let path: Vec<_> = normalize(path)
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    let dir: Vec<_> = normalize(dir)
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    path.len() > dir.len()
        && path.iter().zip(&dir).all(|(a, b)| {
            if ignore_case {
                a.eq_ignore_ascii_case(b)
            } else {
                a == b
            }
        })
}
