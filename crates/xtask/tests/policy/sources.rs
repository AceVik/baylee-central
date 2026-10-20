//! Root, build-support and hidden files are as much first-party code as
//! `crates/`.

use crate::fixture::Fixture;

const ESCAPE: &str = include_str!("../data/escape.txt");

#[test]
fn native_and_assembly_sources_are_refused_anywhere() {
    for (path, message) in [
        ("escape.c", "Native/assembly source forbidden: escape.c"),
        (
            "tools/native/escape.hpp",
            "Native/assembly source forbidden: escape.hpp",
        ),
        (
            ".hidden/escape.S",
            "Native/assembly source forbidden: escape.S",
        ),
    ] {
        let fixture = Fixture::new();
        fixture.write(path, "int native_escape(void) { return 0; }");
        fixture.rejects(message);
    }
}

#[test]
fn unsafe_code_is_refused_in_any_rust_file() {
    let fixture = Fixture::new();
    fixture.write("tools/escape.rs", ESCAPE);
    fixture.rejects("Forbidden unsafe/assembly marker: escape.rs");
}

#[test]
fn a_build_script_needs_the_forbid_attribute() {
    let fixture = Fixture::new();
    fixture.write("crates/fixture/build.rs", "fn main() {}");
    fixture.rejects("Missing crate-root unsafe prohibition: build-script-build");
}
