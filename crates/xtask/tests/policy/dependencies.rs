use crate::fixture::Fixture;

/// The version of `zeroize` the workspace's own lock file holds. It is in
/// every cache `cargo fetch --locked` filled, CI's fresh one included, so the
/// fixture resolves offline whatever version the workspace moves to (a
/// version written here went missing from CI's cache with an upgrade).
fn locked_zeroize() -> String {
    let lock = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock"),
    )
    .expect("the workspace's lock file");
    let mut versions = Vec::new();
    let mut lines = lock.lines();
    while let Some(line) = lines.next() {
        if line == "name = \"zeroize\"" {
            let version = lines
                .next()
                .and_then(|next| next.strip_prefix("version = \""))
                .and_then(|rest| rest.strip_suffix('"'))
                .expect("a version after the name");
            versions.push(version.to_owned());
        }
    }
    assert_eq!(versions.len(), 1, "one zeroize in the lock file");
    versions.remove(0)
}

/// Cargo features are case-sensitive; an approval in another case is none.
/// Uses a crate the workspace already depends on, at the version its lock
/// file holds, so it is cached for offline use.
#[test]
fn dependency_features_are_approved_by_exact_name() {
    let version = locked_zeroize();
    let fixture = Fixture::new();
    fixture.manifest(&format!(
        r#"[dependencies]
zeroize = {{ version = "={version}", default-features = false, features = ["alloc"] }}
"#
    ));
    fixture.approve(&format!(
        r#"{{"packages":["zeroize@{version}"],"features":{{"zeroize@{version}":["ALLOC"]}}}}"#
    ));
    fixture.lock();
    fixture.rejects(&format!(
        "Unapproved dependency feature: zeroize@{version}/alloc"
    ));
    fixture.approve(&format!(
        r#"{{"packages":["zeroize@{version}"],"features":{{"zeroize@{version}":["alloc"]}}}}"#
    ));
    fixture.accepts();
    fixture.approve(r#"{"packages":[],"features":{}}"#);
    fixture.rejects(&format!("Dependency not approved: zeroize@{version}"));
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
