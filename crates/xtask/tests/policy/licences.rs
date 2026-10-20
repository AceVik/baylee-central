use crate::fixture::{Fixture, MANIFEST};

const SOURCE_AVAILABLE: &str = "license = \"LicenseRef-Baylee-Central-Source-Available-1.0\"\n";
const AGPL: &str = "license = \"AGPL-3.0-only\"\n";
const UNKNOWN: &str = "Package licence must be AGPL-3.0-only or \
                       LicenseRef-Baylee-Central-Source-Available-1.0: policy-fixture";

/// No licence, or one the repository does not use, is refused by name.
#[test]
fn a_member_declares_one_of_the_two_licences() {
    let fixture = Fixture::new();
    fixture.write(
        "crates/fixture/Cargo.toml",
        &MANIFEST.replace(SOURCE_AVAILABLE, ""),
    );
    fixture.rejects(UNKNOWN);
    fixture.write(
        "crates/fixture/Cargo.toml",
        &MANIFEST.replace(SOURCE_AVAILABLE, "license = \"MIT\"\n"),
    );
    fixture.rejects(UNKNOWN);
    fixture.write(
        "crates/fixture/Cargo.toml",
        &MANIFEST.replace(SOURCE_AVAILABLE, AGPL),
    );
    fixture.accepts();
}

/// The Baylee client links the AGPL crates, so they may not build on
/// source-available ones; their own tests may.
#[test]
fn an_agpl_member_builds_only_on_agpl_members() {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]
members = [\"crates/fixture\", \"crates/server\"]
resolver = \"3\"
[workspace.lints.rust]
unsafe_code = \"forbid\"
",
    );
    let server = MANIFEST.replace("policy-fixture", "policy-server");
    fixture.write("crates/server/Cargo.toml", &server);
    fixture.write("crates/server/src/lib.rs", "#![forbid(unsafe_code)]\n");
    let agpl = MANIFEST.replace(SOURCE_AVAILABLE, AGPL);
    let depends = |section: &str| {
        fixture.write(
            "crates/fixture/Cargo.toml",
            &format!("{agpl}[{section}]\npolicy-server = {{ path = \"../server\" }}\n"),
        );
        fixture.lock();
    };
    depends("dependencies");
    fixture
        .rejects("AGPL package depends on source-available code: policy-fixture -> policy-server");
    depends("build-dependencies");
    fixture
        .rejects("AGPL package depends on source-available code: policy-fixture -> policy-server");
    depends("dev-dependencies");
    fixture.accepts();
    fixture.write(
        "crates/server/Cargo.toml",
        &server.replace(SOURCE_AVAILABLE, AGPL),
    );
    depends("dependencies");
    fixture.accepts();
}
