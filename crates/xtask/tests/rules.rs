//! The policy's text rules, case by case. The cases that contain unsafe code
//! are data files: the policy scans every `.rs` file for exactly that text.
#![forbid(unsafe_code)]

use xtask::policy::rules;

#[test]
fn every_marker_is_found_and_ordinary_code_is_not() {
    for line in include_str!("data/markers.txt").lines() {
        assert!(rules::has_unsafe_marker(line), "{line}");
    }
    for line in include_str!("data/clean.txt").lines() {
        assert!(!rules::has_unsafe_marker(line), "{line}");
    }
}

#[test]
fn the_workspace_and_crate_roots_say_forbid_on_a_line_of_their_own() {
    assert!(rules::workspace_forbids_unsafe(
        "[workspace.lints.rust]\nunsafe_code = \"forbid\"\r\n"
    ));
    assert!(!rules::workspace_forbids_unsafe("unsafe_code = \"deny\"\n"));
    assert!(!rules::workspace_forbids_unsafe(
        "# unsafe_code = \"forbid\"\n"
    ));
    assert!(rules::forbids_unsafe_at_root(
        "//! docs\n#![forbid(unsafe_code)]\nfn main() {}"
    ));
    assert!(!rules::forbids_unsafe_at_root(
        "// #![forbid(unsafe_code)]\n"
    ));
    assert!(!rules::forbids_unsafe_at_root("#![deny(unsafe_code)]\n"));
}

#[test]
fn members_inherit_lints_across_lines() {
    assert!(rules::inherits_lints("[lints]\nworkspace = true\n"));
    assert!(rules::inherits_lints("[lints]\n\n  workspace=true"));
    assert!(!rules::inherits_lints(
        "[lints.rust]\nunsafe_code = \"forbid\"\n"
    ));
}

#[test]
fn native_sources_are_known_by_extension_in_any_case() {
    for name in [
        "a.c", "a.S", "a.asm", "b.HPP", "x.cc", "y.cxx", "z.hh", ".c",
    ] {
        assert!(rules::is_native(name), "{name}");
    }
    for name in ["Makefile", "a.rs", "c", "a.css", "a.cs"] {
        assert!(!rules::is_native(name), "{name}");
    }
    assert!(rules::is_rust("lib.rs"));
    assert!(rules::is_rust("LIB.RS"));
    assert!(!rules::is_rust("rs"));
}
