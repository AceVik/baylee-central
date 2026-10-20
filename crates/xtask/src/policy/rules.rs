//! The policy's text rules, each the regular expression of
//! `scripts/check-policy.ps1` (PowerShell's `-match` ignores case, so these
//! do too).

use std::sync::LazyLock;

use regex::Regex;

fn rule(pattern: &str) -> Regex {
    Regex::new(pattern)
        .unwrap_or_else(|_| unreachable!("the policy's patterns are fixed and valid"))
}

static WORKSPACE_FORBIDS: LazyLock<Regex> =
    LazyLock::new(|| rule(r#"(?mi)^unsafe_code\s*=\s*"forbid"\s*$"#));
static INHERITS_LINTS: LazyLock<Regex> =
    LazyLock::new(|| rule(r"(?msi)\[lints\]\s*workspace\s*=\s*true"));
static ROOT_FORBIDS: LazyLock<Regex> =
    LazyLock::new(|| rule(r"(?mi)^#!\[forbid\(unsafe_code\)\]\s*$"));
static UNSAFE_MARKERS: LazyLock<[Regex; 3]> = LazyLock::new(|| {
    [
        rule(r"(?i)\bunsafe\s*(\{|fn\b|impl\b|trait\b|extern\b)"),
        rule(r"(?i)\b(asm|global_asm|llvm_asm)\s*!"),
        rule(r"(?i)#!\[allow\(unsafe_code\)\]"),
    ]
});

/// Native or assembly sources, which no first-party code may carry.
const NATIVE: [&str; 10] = ["s", "asm", "c", "cpp", "cc", "cxx", "h", "hh", "hpp", "hxx"];

/// The workspace manifest sets `unsafe_code = "forbid"` on a line of its own.
#[must_use]
pub fn workspace_forbids_unsafe(manifest: &str) -> bool {
    WORKSPACE_FORBIDS.is_match(manifest)
}

/// A member's manifest says `[lints] workspace = true`.
#[must_use]
pub fn inherits_lints(manifest: &str) -> bool {
    INHERITS_LINTS.is_match(manifest)
}

/// A target's root file has `#![forbid(unsafe_code)]` on a line of its own.
#[must_use]
pub fn forbids_unsafe_at_root(source: &str) -> bool {
    ROOT_FORBIDS.is_match(source)
}

/// A Rust file contains `unsafe` code, inline assembly or an allowance for
/// unsafe code (textually: comments and strings count too).
#[must_use]
pub fn has_unsafe_marker(source: &str) -> bool {
    UNSAFE_MARKERS.iter().any(|marker| marker.is_match(source))
}

/// The extension after the last dot of a file name (a name that starts with
/// its only dot, such as `.c`, has one), lowercased.
#[must_use]
pub fn extension(name: &str) -> Option<String> {
    name.rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase())
}

/// A file name that is a native or assembly source.
#[must_use]
pub fn is_native(name: &str) -> bool {
    extension(name).is_some_and(|ext| NATIVE.contains(&ext.as_str()))
}

/// A file name that is a Rust source.
#[must_use]
pub fn is_rust(name: &str) -> bool {
    extension(name).as_deref() == Some("rs")
}
