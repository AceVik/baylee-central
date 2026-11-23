use super::{read, verified};
use std::path::Path;

#[test]
fn only_an_exact_local_file_and_lowercase_commitment_are_accepted() {
    let dir = std::env::temp_dir().join(format!("central-advent-file-test-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let root = dir.canonicalize().unwrap();
    let name = Path::new("sample");
    std::fs::write(root.join(name), b"abc").unwrap();
    let hash = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    let passed = verified(&root, name, hash, 3) == Ok(b"abc".to_vec())
        && verified(&root, name, &hash.to_ascii_uppercase(), 3).is_err()
        && verified(&root, name, &hash[..60], 3).is_err()
        && read(&root, name, 2).is_err()
        && read(&root, Path::new("..").join("sample").as_path(), 3).is_err()
        && read(&root, &root.join(name), 3).is_err()
        && read(&root, Path::new("."), 3).is_err();
    #[cfg(unix)]
    let passed = {
        let link = root.join("link");
        std::os::unix::fs::symlink(root.join(name), &link).unwrap();
        let reject = read(&root, Path::new("link"), 3).is_err();
        std::fs::remove_file(link).unwrap();
        passed && reject
    };
    std::fs::remove_file(root.join(name)).unwrap();
    std::fs::remove_dir(dir).unwrap();
    assert!(passed);
}
