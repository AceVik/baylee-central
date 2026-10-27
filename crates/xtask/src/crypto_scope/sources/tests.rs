use super::*;
use crate::owned::OwnedDir;

#[test]
fn source_addition_removal_rename_and_edit_change_the_review() {
    let fixture = OwnedDir::new("central-crypto-scope-").unwrap();
    let root = fixture.path();
    fs::write(root.join("lib.rs"), "fixed suite\n").unwrap();
    let before = fingerprint(root).unwrap();
    fs::write(root.join("lib.rs"), "fixed suite\r\n").unwrap();
    assert_eq!(fingerprint(root).unwrap(), before);
    fs::write(root.join("lib.rs"), "different suite\n").unwrap();
    assert_ne!(fingerprint(root).unwrap(), before);
    fs::write(root.join("lib.rs"), "fixed suite\n").unwrap();
    fs::write(root.join("extra.rs"), "new consumer\n").unwrap();
    assert_ne!(fingerprint(root).unwrap(), before);
    fs::remove_file(root.join("extra.rs")).unwrap();
    assert_eq!(fingerprint(root).unwrap(), before);
    fs::rename(root.join("lib.rs"), root.join("moved.rs")).unwrap();
    assert_ne!(fingerprint(root).unwrap(), before);
    fs::remove_file(root.join("moved.rs")).unwrap();
    assert_ne!(fingerprint(root).unwrap(), before);
    assert!(verify(root).is_err());
    fixture.remove().unwrap();
}

#[cfg(unix)]
#[test]
fn rejects_links_instead_of_hashing_outside_sources() {
    let fixture = OwnedDir::new("central-crypto-scope-link-").unwrap();
    std::os::unix::fs::symlink("/nonexistent", fixture.path().join("lib.rs")).unwrap();
    assert!(fingerprint(fixture.path()).is_err());
    fixture.remove().unwrap();
}
