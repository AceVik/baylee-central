use crate::fixture::Fixture;

/// Cargo features are case-sensitive; an approval in another case is none.
/// Uses a crate the workspace already depends on, cached for offline use.
#[test]
fn dependency_features_are_approved_by_exact_name() {
    let fixture = Fixture::new();
    fixture.manifest(
        r#"[dependencies]
zeroize = { version = "=1.9.1", default-features = false, features = ["alloc"] }
"#,
    );
    fixture.approve(r#"{"packages":["zeroize@1.9.1"],"features":{"zeroize@1.9.1":["ALLOC"]}}"#);
    fixture.lock();
    fixture.rejects("Unapproved dependency feature: zeroize@1.9.1/alloc");
    fixture.approve(r#"{"packages":["zeroize@1.9.1"],"features":{"zeroize@1.9.1":["alloc"]}}"#);
    fixture.accepts();
    fixture.approve(r#"{"packages":[],"features":{}}"#);
    fixture.rejects("Dependency not approved: zeroize@1.9.1");
}

/// A path dependency outside the workspace would escape every check.
#[test]
fn a_local_dependency_must_be_a_member() {
    let fixture = Fixture::new();
    let helper = fixture.outside().join("unreviewed-helper");
    std::fs::create_dir_all(helper.join("src")).expect("a helper");
    std::fs::write(
        helper.join("Cargo.toml"),
        "[package]\nname = \"unreviewed-helper\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("a manifest");
    std::fs::write(helper.join("src/lib.rs"), "pub fn unreviewed() {}").expect("a source");
    fixture.manifest(
        "[dependencies]\nunreviewed-helper = { path = \"../../../unreviewed-helper\" }\n",
    );
    fixture.lock();
    fixture.rejects("Local dependency outside reviewed workspace: unreviewed-helper");
}

/// Linux has two distinct package paths whose IDs differ only by case; a
/// case-insensitive membership test would take the foreign one as a member.
#[cfg(target_os = "linux")]
#[test]
fn package_ids_differing_only_in_case_stay_apart() {
    let fixture = Fixture::new();
    let foreign = fixture.outside().join("REVIEWED/crates/fixture");
    std::fs::create_dir_all(foreign.join("src")).expect("a helper");
    std::fs::write(
        foreign.join("Cargo.toml"),
        "[package]\nname = \"POLICY-FIXTURE\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("a manifest");
    std::fs::write(foreign.join("src/lib.rs"), "pub fn unreviewed() {}").expect("a source");
    fixture.manifest(
        r#"[dependencies]
unreviewed-case-helper = { package = "POLICY-FIXTURE", path = "../../../REVIEWED/crates/fixture" }
"#,
    );
    fixture.lock();
    fixture.rejects("Local dependency outside reviewed workspace: POLICY-FIXTURE");
}
