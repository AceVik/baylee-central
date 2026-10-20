//! A synthetic workspace in a temporary directory: one member, its manifest
//! inheriting the workspace lints, an empty approval inventory, a lockfile.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use xtask::owned::OwnedDir;
use xtask::policy;

const WORKSPACE: &str = "[workspace]
members = [\"crates/fixture\"]
resolver = \"3\"
[workspace.lints.rust]
unsafe_code = \"forbid\"
";
pub const MANIFEST: &str = "[package]
name = \"policy-fixture\"
version = \"0.1.0\"
edition = \"2024\"
publish = false
[lints]
workspace = true
";

pub struct Fixture {
    dir: OwnedDir,
    root: PathBuf,
}

impl Fixture {
    pub fn new() -> Self {
        let dir = OwnedDir::new("central-policy-").expect("a temporary directory");
        let root = dir.path().join("reviewed");
        let fixture = Self { dir, root };
        fixture.write("Cargo.toml", WORKSPACE);
        fixture.write("crates/fixture/Cargo.toml", MANIFEST);
        fixture.write("crates/fixture/src/lib.rs", "#![forbid(unsafe_code)]\n");
        fixture.approve(r#"{"packages":[],"features":{}}"#);
        fixture.lock();
        fixture
    }

    /// Beside the reviewed tree, never inside it.
    pub fn outside(&self) -> PathBuf {
        self.dir.path().to_path_buf()
    }

    pub fn write(&self, path: &str, text: &str) {
        let file = self.root.join(path);
        fs::create_dir_all(file.parent().expect("a parent")).expect("a directory");
        fs::write(file, text).expect("a file");
    }

    pub fn manifest(&self, extra: &str) {
        self.write("crates/fixture/Cargo.toml", &format!("{MANIFEST}{extra}"));
    }

    pub fn approve(&self, inventory: &str) {
        self.write("docs/dependency-proposal/approved-packages.json", inventory);
    }

    /// Resolves the graph offline, so `--locked` itself is never the refusal.
    pub fn lock(&self) {
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let status = Command::new(cargo)
            .args(["generate-lockfile", "--offline"])
            .current_dir(&self.root)
            .status()
            .expect("cargo");
        assert!(status.success(), "the synthetic workspace resolves");
    }

    /// A directory link at `path` (under the reviewed tree) to a directory
    /// outside it.
    pub fn link(&self, path: &str) -> PathBuf {
        let target = self.dir.path().join("linked-source");
        fs::create_dir_all(&target).expect("a link target");
        let link = self.root.join(path);
        fs::create_dir_all(link.parent().expect("a parent")).expect("a directory");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &link).expect("a symbolic link");
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&target, &link)
            .expect("a directory link (needs the symlink privilege)");
        link
    }

    pub fn check(&self) -> Result<(), String> {
        policy::check(&self.root, false).map_err(|fail| fail.message().to_owned())
    }

    pub fn accepts(&self) {
        assert_eq!(self.check(), Ok(()));
    }

    pub fn rejects(&self, message: &str) {
        assert_eq!(self.check(), Err(message.to_owned()));
    }
}
