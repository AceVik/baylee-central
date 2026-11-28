//! Profile-guided builds (`cargo xtask pgo`, `docs/xtask.md` §"Profile-guided
//! builds") of the production Linux servers.
//!
//! What is built from a profile is what its training stands for:
//! `central-web`, the server-rendered website ([`train`]: every page in
//! every language, the redirects, the generated files, the card
//! suggestions and the hashed assets), and `central-picture-worker`, every
//! picture format the service takes. Under a profile, code the training
//! never ran counts as cold and is optimised for size (LLVM's profile-guided
//! size optimisation), so a thin training is worse than none.
//! `central-service`, whose work (sign-in, its database, the push socket)
//! needs a database to train against, is built the same way without a
//! profile ([`PLAIN`]) until a training of its real requests exists.
//!
//! The steps, each of which stops the task when it fails:
//!
//! 1. Build the profiled binaries instrumented, in `target/pgo-gen`.
//! 2. Train them ([`train`]): the counts go to `<out>/profraw`.
//! 3. Merge the counts into `<out>/central.profdata`, and write the toolchain
//!    that made it beside it (`central.profdata.rustc`).
//! 4. Build them from the profile in an emptied `target/pgo-use`, and
//!    `central-service` without one in `target/pgo-plain`. The flag names
//!    the same file whatever it holds, so cargo cannot see a new profile,
//!    and a crate no commit touched would keep the code an older profile
//!    made.
//! 5. Require the profile to have taken: the functions the compiler reports
//!    without counts, against the profile's own total ([`took`]). A profile
//!    that matches nothing still builds working binaries; it is a build
//!    without a profile under another name.
//!
//! Then `<out>/dist/` gets the three binaries and `SHA256SUMS.txt`. With
//! `--compare` the profiled binaries are measured against the build the
//! deploy hook makes today ([`compare`]), a report and not a gate. With
//! `--profile-use <file>` steps 1 to 3 are skipped for a profile made
//! elsewhere, which must name this toolchain.
//!
//! Every build passes `--target` with the host's triple, so the flags reach
//! what ships and no build script or procedural macro: they would otherwise
//! be instrumented as well and write counts while the build runs.

mod compare;
mod train;

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use sha2::{Digest, Sha256};

use crate::fail::{Fail, OrFail, Result};
use crate::{process, random, workspace};

const DEFAULT_OUT: &str = "target/pgo-release";
const DEFAULT_CPU: &str = "x86-64-v3";

/// The cargo profile of everything this task ships (`Cargo.toml`): fat
/// link-time optimisation and one codegen unit.
const PROFILE: &str = "release-pgo";

/// The binaries built from the profile: package and binary.
const PROFILED: [(&str, &str); 2] = [
    ("central-web-server", "central-web"),
    ("central-picture-codec", "central-picture-worker"),
];

/// The binary built the same way without a profile (module docs).
const PLAIN: [(&str, &str); 1] = [("central-service", "central-service")];

/// What the compiler says of a function it found no counts for, when asked
/// (`-pgo-warn-missing-function`), and of one whose code changed since the
/// counts were taken.
const UNMATCHED: [&str; 2] = ["no profile data available for function", "hash mismatch"];

/// The share of the profile's functions the compiler may find no counts for.
/// Both builds select the same packages with the same flags, so a profile
/// that took matches nearly every function; one whose crate hashes differ
/// matches almost none.
const MOST_UNMATCHED_PERCENT: u64 = 5;

/// Lines of the compiler's report of unmatched functions shown in the log;
/// the rest are counted.
const SHOWN_UNMATCHED: u64 = 3;

/// Command options for `cargo xtask pgo`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// Output directory for `central.profdata` and `dist/` binaries.
    pub out: String,
    /// Target CPU microarchitecture passed to `-C target-cpu=...`.
    pub target_cpu: String,
    /// Only collect and merge `.profraw` into `<out>/central.profdata`; skip stage-4 build.
    pub profile_only: bool,
    /// Build from this profile instead of training one (steps 4 and 5 only).
    pub profile_use: Option<String>,
    /// Measure the profiled binaries against the deploy hook's build.
    pub compare: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            out: DEFAULT_OUT.to_owned(),
            target_cpu: DEFAULT_CPU.to_owned(),
            profile_only: false,
            profile_use: None,
            compare: false,
        }
    }
}

/// How a build is given the profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FlagMode<'a> {
    /// Instrumented, writing counts into this directory.
    Generate(&'a Path),
    /// Optimised from this merged profile.
    Use(&'a Path),
    /// No profile.
    Plain,
}

/// One machine's builds: the checkout, the host's triple (`--target`) and
/// the CPU the binaries are for.
struct Builds<'a> {
    root: &'a Path,
    triple: &'a str,
    target_cpu: &'a str,
}

impl Builds<'_> {
    /// Builds `bins` under `profile` into `target_dir` and answers the
    /// number of functions the compiler reported without counts.
    fn build(
        &self,
        target_dir: &Path,
        profile: &str,
        bins: &[(&str, &str)],
        mode: FlagMode<'_>,
    ) -> Result<u64> {
        let base_flags = env::var("RUSTFLAGS").ok();
        let encoded = encoded_rustflags(base_flags.as_deref(), self.target_cpu, mode)?;
        let mut cmd = Command::new(workspace::cargo());
        cmd.current_dir(self.root)
            .args([
                "build",
                "--locked",
                "--profile",
                profile,
                "--target",
                self.triple,
            ])
            .arg("--target-dir")
            .arg(target_dir)
            .args(selection(bins))
            .env_remove("RUSTFLAGS")
            .env("CARGO_ENCODED_RUSTFLAGS", encoded)
            .env("CARGO_TERM_COLOR", "never")
            .stdin(Stdio::null())
            .stderr(Stdio::piped());
        let mut child = cmd.spawn().or_fail("PGO cargo build could not start")?;
        let unmatched = match child.stderr.take() {
            Some(stderr) => report(BufReader::new(stderr), &mut std::io::stderr()),
            None => 0,
        };
        let status = child.wait().or_fail("PGO cargo build failed")?;
        if status.success() {
            Ok(unmatched)
        } else {
            Err(Fail::new("PGO cargo build failed"))
        }
    }

    /// Where `target_dir`'s binaries of `profile` are.
    fn bin_dir(&self, target_dir: &Path, profile: &str) -> PathBuf {
        target_dir.join(self.triple).join(profile)
    }
}

/// Validates a `-C target-cpu=...` selector (no whitespace or shell metacharacters).
#[must_use]
pub fn valid_target_cpu(raw: &str) -> bool {
    !raw.is_empty()
        && raw.len() <= 64
        && raw
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}

/// Validates an `--out` directory path string (non-empty, NUL-free).
#[must_use]
pub fn valid_out_dir(raw: &str) -> bool {
    !raw.trim().is_empty() && !raw.bytes().any(|b| b == 0)
}

pub(crate) fn run(root: &Path, opts: &Options) -> Result<()> {
    let llvm_profdata = find_llvm_profdata()?;
    let rustc = toolchain()?;
    let triple = host_triple(&rustc).or_fail("rustc names no host triple")?;
    let builds = Builds {
        root,
        triple: &triple,
        target_cpu: &opts.target_cpu,
    };
    let out_dir = resolve_path(root, &opts.out);
    fs::create_dir_all(&out_dir).or_fail("PGO output directory could not be prepared")?;

    let profile = if let Some(raw) = &opts.profile_use {
        let path = resolve_path(root, raw)
            .canonicalize()
            .or_fail("The PGO profile to build from is missing")?;
        let stamp = fs::read_to_string(stamp_path(&path))
            .or_fail("The PGO profile names no toolchain (central.profdata.rustc)")?;
        if stamp.trim() != rustc.trim() {
            return Err(Fail::new(
                "The PGO profile was made by another toolchain than this one",
            ));
        }
        println!("PGO: building from {}", path.display());
        path
    } else {
        let path = collect(&builds, &llvm_profdata, &out_dir, &rustc)?;
        if opts.profile_only {
            println!("PGO profile collection finished (--profile-only).");
            return Ok(());
        }
        path
    };
    let total = profile_functions(&llvm_profdata, &profile)?;

    println!(
        "PGO step 4/5: building from the profile (FatLTO, target-cpu={})...",
        opts.target_cpu
    );
    let use_target = root.join("target/pgo-use");
    reset_dir(&use_target, "PGO build directory could not be emptied")?;
    let unmatched = builds.build(&use_target, PROFILE, &PROFILED, FlagMode::Use(&profile))?;
    let plain_target = root.join("target/pgo-plain");
    builds.build(&plain_target, PROFILE, &PLAIN, FlagMode::Plain)?;

    println!("PGO step 5/5: {unmatched} of the profile's {total} functions without counts");
    took(unmatched, total)?;

    let profiled = builds.bin_dir(&use_target, PROFILE);
    let plain = builds.bin_dir(&plain_target, PROFILE);
    let dist_dir = out_dir.join("dist");
    stage_dist(
        &[
            (profiled.as_path(), &PROFILED[..]),
            (plain.as_path(), &PLAIN[..]),
        ],
        &dist_dir,
    )?;
    println!("PGO release binaries staged under {}", dist_dir.display());
    if opts.compare {
        compare::run(&builds, &profiled)?;
    }
    Ok(())
}

/// Steps 1 to 3: the instrumented build, its training and the merged
/// profile with its toolchain beside it.
fn collect(
    builds: &Builds<'_>,
    llvm_profdata: &Path,
    out_dir: &Path,
    rustc: &str,
) -> Result<PathBuf> {
    let profraw_dir = out_dir.join("profraw");
    reset_dir(&profraw_dir, "PGO profile directory could not be prepared")?;
    let profraw_abs = profraw_dir
        .canonicalize()
        .or_fail("PGO profile directory could not be resolved")?;
    let gen_target = builds.root.join("target/pgo-gen");

    println!(
        "PGO step 1/5: building instrumented binaries (target-cpu={}, profile={PROFILE})...",
        builds.target_cpu
    );
    builds.build(
        &gen_target,
        PROFILE,
        &PROFILED,
        FlagMode::Generate(&profraw_abs),
    )?;

    println!("PGO step 2/5: training...");
    train::run(
        builds.root,
        &builds.bin_dir(&gen_target, PROFILE),
        &llvm_profile_pattern(&profraw_abs)?,
    )?;
    let raw_files = collect_profraw(&profraw_abs)?;
    if raw_files.is_empty() {
        return Err(Fail::new(
            "PGO workload produced no non-empty .profraw files",
        ));
    }

    println!(
        "PGO step 3/5: merging {} .profraw files via {}...",
        raw_files.len(),
        llvm_profdata.display()
    );
    let profdata_path = out_dir.join("central.profdata");
    merge_profdata(llvm_profdata, &raw_files, &profdata_path)?;
    fs::write(stamp_path(&profdata_path), rustc).or_fail("PGO toolchain stamp unwritable")?;
    let profdata_abs = profdata_path
        .canonicalize()
        .or_fail("Merged PGO profile missing after merge")?;
    let profdata_bytes = fs::metadata(&profdata_abs)
        .or_fail("Merged PGO profile unreadable")?
        .len();
    println!(
        "Merged PGO profile: {} ({} KiB)",
        profdata_abs.display(),
        profdata_bytes / 1024
    );
    Ok(profdata_abs)
}

/// Whether the profile took: at most [`MOST_UNMATCHED_PERCENT`] of its
/// functions may be left without counts.
fn took(unmatched: u64, total: u64) -> Result<()> {
    if total == 0 {
        return Err(Fail::new("The PGO profile holds no functions"));
    }
    if unmatched.saturating_mul(100) > total.saturating_mul(MOST_UNMATCHED_PERCENT) {
        return Err(Fail::new(format!(
            "The PGO profile did not take: {unmatched} of its {total} functions have no counts \
             (at most {MOST_UNMATCHED_PERCENT} % may)"
        )));
    }
    Ok(())
}

/// Copies a cargo build's output to `echo` line by line and answers how many
/// lines report a function without counts ([`UNMATCHED`]); of those only
/// the first [`SHOWN_UNMATCHED`] are shown.
fn report(input: impl BufRead, echo: &mut impl Write) -> u64 {
    let mut unmatched = 0_u64;
    for line in input.lines() {
        let Ok(line) = line else {
            break;
        };
        if UNMATCHED.iter().any(|said| line.contains(said)) {
            unmatched += 1;
            if unmatched > SHOWN_UNMATCHED {
                continue;
            }
        }
        let _ = writeln!(echo, "{line}");
    }
    if unmatched > SHOWN_UNMATCHED {
        let _ = writeln!(
            echo,
            "... and {} more functions without counts",
            unmatched - SHOWN_UNMATCHED
        );
    }
    unmatched
}

/// `-p` for each package and `--bin` for each binary of `bins`.
fn selection(bins: &[(&str, &str)]) -> Vec<String> {
    let mut args = Vec::with_capacity(bins.len() * 4);
    for (package, _) in bins {
        args.push("-p".to_owned());
        args.push((*package).to_owned());
    }
    for (_, bin) in bins {
        args.push("--bin".to_owned());
        args.push((*bin).to_owned());
    }
    args
}

fn resolve_path(root: &Path, raw: &str) -> PathBuf {
    let p = Path::new(raw.trim());
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        root.join(p)
    }
}

fn reset_dir(dir: &Path, failure: &str) -> Result<()> {
    if dir.exists() {
        fs::remove_dir_all(dir).or_fail(failure)?;
    }
    fs::create_dir_all(dir).or_fail(failure)
}

/// The file beside `profile` that names the toolchain which made it.
fn stamp_path(profile: &Path) -> PathBuf {
    let mut name = profile.as_os_str().to_os_string();
    name.push(".rustc");
    PathBuf::from(name)
}

fn rustc() -> std::ffi::OsString {
    env::var_os("RUSTC").unwrap_or_else(|| "rustc".into())
}

/// `rustc -vV`: the toolchain's version, commit, host and LLVM version. A
/// profile is read only by the compiler that made it.
fn toolchain() -> Result<String> {
    let out = process::capture(
        Command::new(rustc()).arg("-vV"),
        "rustc unavailable while naming the toolchain",
    )?;
    if !out.status.success() {
        return Err(Fail::new("rustc unavailable while naming the toolchain"));
    }
    String::from_utf8(out.stdout).or_fail("rustc named its toolchain in something not UTF-8")
}

/// The `host:` line of `rustc -vV`.
fn host_triple(verbose: &str) -> Option<String> {
    verbose
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .map(|host| host.trim().to_owned())
        .filter(|host| !host.is_empty())
}

/// The number of functions in a merged profile (`llvm-profdata show`).
fn profile_functions(llvm_profdata: &Path, profile: &Path) -> Result<u64> {
    let out = process::capture(
        Command::new(llvm_profdata).arg("show").arg(profile),
        "llvm-profdata show failed",
    )?;
    if !out.status.success() {
        return Err(Fail::new("llvm-profdata show failed"));
    }
    total_functions(&String::from_utf8_lossy(&out.stdout))
        .or_fail("llvm-profdata show named no total of functions")
}

/// `Total functions: N` of `llvm-profdata show`'s summary.
fn total_functions(summary: &str) -> Option<u64> {
    summary.lines().find_map(|line| {
        line.trim()
            .strip_prefix("Total functions:")
            .and_then(|n| n.trim().parse().ok())
    })
}

fn profdata_candidates(target_libdir: &Path, sysroot: Option<&Path>) -> Vec<PathBuf> {
    let bin_name = if cfg!(windows) {
        "llvm-profdata.exe"
    } else {
        "llvm-profdata"
    };
    let mut out = Vec::new();
    if let Some(target_root) = target_libdir.parent() {
        out.push(target_root.join("bin").join(bin_name));
    }
    if let Some(root) = sysroot {
        let rustlib = root.join("lib/rustlib");
        if let Ok(entries) = fs::read_dir(&rustlib) {
            for entry in entries.flatten() {
                out.push(entry.path().join("bin").join(bin_name));
            }
        }
    }
    out
}

fn find_llvm_profdata() -> Result<PathBuf> {
    let libdir_out = process::capture(
        Command::new(rustc()).args(["--print", "target-libdir"]),
        "rustc unavailable while locating llvm-profdata",
    )?;
    let target_libdir = PathBuf::from(String::from_utf8_lossy(&libdir_out.stdout).trim());
    let sysroot_out = process::capture(
        Command::new(rustc()).args(["--print", "sysroot"]),
        "rustc sysroot unavailable",
    )?;
    let sysroot = PathBuf::from(String::from_utf8_lossy(&sysroot_out.stdout).trim());
    for candidate in profdata_candidates(&target_libdir, Some(&sysroot)) {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(Fail::new(
        "llvm-profdata not found in active rustc toolchain (install `rustup component add llvm-tools`)",
    ))
}

fn encoded_rustflags(
    existing: Option<&str>,
    target_cpu: &str,
    mode: FlagMode<'_>,
) -> Result<String> {
    let mut flags: Vec<String> = Vec::new();
    if let Some(prev) = existing {
        for part in prev.split_whitespace() {
            if !part.is_empty() {
                flags.push(part.to_owned());
            }
        }
    }
    flags.push("-C".to_owned());
    flags.push(format!("target-cpu={target_cpu}"));
    match mode {
        FlagMode::Generate(dir) => {
            let s = dir.to_str().or_fail("Non-UTF-8 PGO profile directory")?;
            flags.push("-C".to_owned());
            flags.push(format!("profile-generate={s}"));
        }
        FlagMode::Use(file) => {
            let s = file.to_str().or_fail("Non-UTF-8 PGO profile file path")?;
            flags.push("-C".to_owned());
            flags.push(format!("profile-use={s}"));
            flags.push("-C".to_owned());
            flags.push("llvm-args=-pgo-warn-missing-function".to_owned());
        }
        FlagMode::Plain => {}
    }
    Ok(flags.join("\x1f"))
}

fn free_loopback_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").or_fail("Failed to bind ephemeral port")?;
    Ok(listener
        .local_addr()
        .or_fail("Failed to read bound port")?
        .port())
}

fn bin_path(bin_dir: &Path, name: &str) -> PathBuf {
    bin_dir
        .join(name)
        .with_extension(env::consts::EXE_EXTENSION)
}

fn llvm_profile_pattern(profraw_dir: &Path) -> Result<String> {
    let dir = profraw_dir
        .to_str()
        .or_fail("Non-UTF-8 PGO profile directory")?;
    Ok(format!("{dir}/%p-%m.profraw"))
}

fn wait_ready_line(child: &mut Child, prefix: &str, name: &str) -> Result<String> {
    let stdout = child.stdout.take().or_fail("Server stdout missing")?;
    if let Some(stderr) = child.stderr.take() {
        let _ = process::drain(stderr);
    }
    let prefix_owned = prefix.to_owned();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut lines = BufReader::new(stdout).lines();
        let mut matched = None;
        for line in lines.by_ref() {
            let Ok(text) = line else {
                break;
            };
            if text.contains(&prefix_owned) {
                matched = Some(text);
                break;
            }
        }
        let _ = tx.send(matched);
        for _ in lines {}
    });
    rx.recv_timeout(Duration::from_secs(20))
        .or_fail(&format!("{name} startup timed out"))?
        .or_fail(&format!("{name} exited before signaling ready"))
}

/// Stops a server with `SIGINT` to its process group, so an instrumented
/// one writes its counts; a server that is gone or will not stop fails.
fn graceful_stop(child: &mut Child, name: &str) -> Result<()> {
    if let Ok(Some(status)) = child.try_wait() {
        return if status.success() {
            Ok(())
        } else {
            Err(Fail::new(format!("{name} exited unexpectedly during PGO")))
        };
    }
    #[cfg(unix)]
    {
        let pid = child.id().to_string();
        let _ = Command::new("kill")
            .args(["-INT", "--", &format!("-{pid}")])
            .stderr(Stdio::null())
            .status();
        match process::wait_until(child, Duration::from_secs(12))? {
            Some(status) if status.success() => Ok(()),
            Some(_) => Err(Fail::new(format!(
                "{name} returned non-zero status on SIGINT"
            ))),
            None => {
                let _ = process::stop_tree(child);
                Err(Fail::new(format!(
                    "{name} did not stop cleanly on SIGINT to flush PGO counters"
                )))
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = name;
        process::stop_tree(child)
    }
}

fn http_agent() -> ureq::Agent {
    let cfg = ureq::config::Config::builder()
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(10)))
        .build();
    ureq::Agent::new_with_config(cfg)
}

fn collect_profraw(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).or_fail("PGO profraw directory unreadable")? {
        let entry = entry.or_fail("PGO profraw entry unreadable")?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("profraw")
            && entry.metadata().is_ok_and(|m| m.is_file() && m.len() > 0)
        {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn merge_profdata(llvm_profdata: &Path, raw: &[PathBuf], out: &Path) -> Result<()> {
    let mut cmd = Command::new(llvm_profdata);
    cmd.arg("merge").arg("-o").arg(out);
    for file in raw {
        cmd.arg(file);
    }
    process::succeed(&mut cmd, "llvm-profdata merge failed")
}

/// A build's binary directory and the binaries of it that ship.
type Staged<'a> = (&'a Path, &'a [(&'a str, &'a str)]);

/// Copies each source's binaries to `dist_dir` and writes their SHA-256 sums.
fn stage_dist(sources: &[Staged<'_>], dist_dir: &Path) -> Result<()> {
    reset_dir(dist_dir, "PGO dist directory could not be prepared")?;
    let mut sums = String::new();
    for (bin_dir, bins) in sources {
        for (_, name) in *bins {
            let src = bin_path(bin_dir, name);
            if !src.is_file() {
                return Err(Fail::new(format!("Final PGO binary missing: {name}")));
            }
            let filename = src.file_name().or_fail("Binary filename missing")?;
            let dst = dist_dir.join(filename);
            fs::copy(&src, &dst).or_fail("Failed copying PGO binary to dist")?;
            let bytes = fs::read(&dst).or_fail("Failed reading staged PGO binary for SHA-256")?;
            let hex = random::to_hex(&Sha256::digest(&bytes));
            let _ = writeln!(sums, "{hex}  {}", filename.to_string_lossy());
        }
    }
    fs::write(dist_dir.join("SHA256SUMS.txt"), sums.as_bytes())
        .or_fail("Failed writing SHA256SUMS.txt")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_cpu_and_out_options() {
        assert!(valid_target_cpu("x86-64-v3"));
        assert!(valid_target_cpu("native"));
        assert!(!valid_target_cpu(""));
        assert!(!valid_target_cpu("x86-64-v3;rm"));
        assert!(!valid_target_cpu("with space"));
        assert!(valid_out_dir("target/pgo-release"));
        assert!(!valid_out_dir("   "));
        assert!(!valid_out_dir("bad\0dir"));
    }

    #[test]
    fn encodes_rustflags_for_generate_use_and_plain() {
        let prof_dir = Path::new("/tmp/pgo dir");
        let gen_flags = encoded_rustflags(
            Some("-D warnings"),
            "x86-64-v3",
            FlagMode::Generate(prof_dir),
        )
        .unwrap();
        assert_eq!(
            gen_flags,
            "-D\x1fwarnings\x1f-C\x1ftarget-cpu=x86-64-v3\x1f-C\x1fprofile-generate=/tmp/pgo dir"
        );
        let prof_file = Path::new("/tmp/central.profdata");
        let use_flags = encoded_rustflags(None, "x86-64-v3", FlagMode::Use(prof_file)).unwrap();
        assert_eq!(
            use_flags,
            "-C\x1ftarget-cpu=x86-64-v3\x1f-C\x1fprofile-use=/tmp/central.profdata\x1f-C\x1fllvm-args=-pgo-warn-missing-function"
        );
        let plain = encoded_rustflags(None, "x86-64-v3", FlagMode::Plain).unwrap();
        assert_eq!(plain, "-C\x1ftarget-cpu=x86-64-v3");
    }

    #[test]
    fn the_profiled_and_the_plain_selection_never_share_a_binary() {
        for (_, bin) in PLAIN {
            assert!(PROFILED.iter().all(|(_, profiled)| *profiled != bin));
        }
        assert_eq!(
            selection(&PROFILED),
            [
                "-p",
                "central-web-server",
                "-p",
                "central-picture-codec",
                "--bin",
                "central-web",
                "--bin",
                "central-picture-worker",
            ]
        );
    }

    #[test]
    fn the_report_counts_unmatched_functions_and_shows_only_the_first() {
        let mut log = String::new();
        for at in 0..10 {
            let _ = writeln!(
                log,
                "warning: /src/x.rs: no profile data available for function f{at}"
            );
        }
        log.push_str("warning: x: function control flow change detected (hash mismatch) g\n");
        log.push_str("   Compiling central-web v0.1.0\n");
        let mut echo = Vec::new();
        let unmatched = report(log.as_bytes(), &mut echo);
        assert_eq!(unmatched, 11);
        let echo = String::from_utf8(echo).unwrap();
        assert!(echo.contains("function f0"));
        assert!(echo.contains("function f2"));
        assert!(!echo.contains("function f3"));
        assert!(echo.contains("Compiling central-web"));
        assert!(echo.contains("... and 8 more functions without counts"));
        let mut quiet = Vec::new();
        assert_eq!(report("   Finished\n".as_bytes(), &mut quiet), 0);
        assert_eq!(quiet, b"   Finished\n");
    }

    #[test]
    fn a_profile_takes_only_when_almost_every_function_has_counts() {
        assert!(took(0, 120_000).is_ok());
        assert!(took(6_000, 120_000).is_ok());
        assert!(took(6_001, 120_000).is_err());
        assert!(took(119_000, 120_000).is_err());
        assert!(took(0, 0).is_err());
        let failure = took(50, 100).unwrap_err();
        assert!(failure.message().contains("50 of its 100"));
    }

    #[test]
    fn reads_the_profile_summary_and_the_toolchain() {
        let summary = "Instrumentation level: IR  entry_first = 0\nTotal functions: 123456\nMaximum function count: 9\n";
        assert_eq!(total_functions(summary), Some(123_456));
        assert_eq!(total_functions("Maximum function count: 9\n"), None);
        let verbose = "rustc 1.100.0 (0ef1bc386 2026-11-09)\nbinary: rustc\nhost: x86_64-unknown-linux-gnu\nrelease: 1.100.0\nLLVM version: 21.1.8\n";
        assert_eq!(
            host_triple(verbose).as_deref(),
            Some("x86_64-unknown-linux-gnu")
        );
        assert_eq!(host_triple("rustc 1.100.0\n"), None);
        assert_eq!(
            stamp_path(Path::new("/w/central.profdata")),
            Path::new("/w/central.profdata.rustc")
        );
    }

    #[test]
    fn profraw_collector_skips_empty_and_non_profraw_files() {
        let dir = env::temp_dir().join(format!("xtask-pgo-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("empty.profraw"), b"").unwrap();
        fs::write(dir.join("other.txt"), b"hello").unwrap();
        fs::write(dir.join("b.profraw"), b"abc").unwrap();
        fs::write(dir.join("a.profraw"), b"xyz").unwrap();
        let found = collect_profraw(&dir).unwrap();
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].file_name().unwrap(), "a.profraw");
        assert_eq!(found[1].file_name().unwrap(), "b.profraw");
    }

    #[test]
    fn resolves_llvm_profdata_candidate_under_target_libdir_parent() {
        let libdir = Path::new("/rust/lib/rustlib/x86_64-unknown-linux-gnu/lib");
        let list = profdata_candidates(libdir, None);
        assert_eq!(list.len(), 1);
        assert!(list[0].ends_with(if cfg!(windows) {
            "x86_64-unknown-linux-gnu/bin/llvm-profdata.exe"
        } else {
            "x86_64-unknown-linux-gnu/bin/llvm-profdata"
        }));
    }
}
