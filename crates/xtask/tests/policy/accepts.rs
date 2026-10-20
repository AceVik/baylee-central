use crate::fixture::Fixture;

#[test]
fn a_clean_workspace_passes() {
    Fixture::new().accepts();
}

#[test]
fn the_workspace_must_forbid_unsafe() {
    let fixture = Fixture::new();
    let denies = "[workspace]
members = [\"crates/fixture\"]
resolver = \"3\"
[workspace.lints.rust]
unsafe_code = \"deny\"
";
    fixture.write("Cargo.toml", denies);
    fixture.rejects("Workspace unsafe prohibition is missing");
}

#[test]
fn every_member_inherits_the_workspace_lints() {
    let fixture = Fixture::new();
    let manifest = crate::fixture::MANIFEST.replace("[lints]\nworkspace = true\n", "");
    fixture.write("crates/fixture/Cargo.toml", &manifest);
    fixture.rejects("Missing inherited lints: policy-fixture");
}
