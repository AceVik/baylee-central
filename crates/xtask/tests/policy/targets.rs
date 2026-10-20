use crate::fixture::Fixture;

#[test]
fn a_target_cannot_hide_in_tool_output() {
    let fixture = Fixture::new();
    fixture.write("target/fixture.rs", "#![forbid(unsafe_code)]");
    fixture.manifest("[lib]\npath = \"../../target/fixture.rs\"\n");
    fixture.rejects("Source target inside excluded output directory: policy_fixture");
}

#[test]
fn a_target_cannot_live_outside_the_reviewed_tree() {
    let fixture = Fixture::new();
    let outside = fixture.outside().join("outside.rs");
    std::fs::write(&outside, "#![forbid(unsafe_code)]").expect("a file");
    fixture.manifest("[lib]\npath = \"../../../outside.rs\"\n");
    fixture.rejects("Source target outside reviewed root: policy_fixture");
}

#[test]
fn a_member_is_never_published() {
    for publish in ["", "publish = [\"crates-io\"]\n"] {
        let fixture = Fixture::new();
        let manifest = crate::fixture::MANIFEST.replace("publish = false\n", publish);
        fixture.write("crates/fixture/Cargo.toml", &manifest);
        fixture.rejects("Package publication must be disabled: policy-fixture");
    }
}
