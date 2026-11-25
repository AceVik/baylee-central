//! Automated pure-Rust Profile-Guided Optimization (`cargo xtask pgo`) for the
//! production Linux servers (`central-service`, `central-web`, `central-picture-worker`).
//!
//! Runs a four-stage pipeline targeting `--target-cpu x86-64-v3` (overridable):
//! 1. Builds instrumented binaries (`target/pgo-gen`) using `[profile.release-pgo]`.
//! 2. Exercises realistic local loopback HTTP & codec workloads so LLVM
//!    records `.profraw` execution counters (`SIGINT` flushes profiles cleanly).
//! 3. Merges raw counters via `llvm-profdata merge` into `<out>/central.profdata`.
//! 4. Rebuilds optimized binaries (`target/pgo-use`) with `FatLTO` + `profile-use`,
//!    stages them under `<out>/dist/`, and writes `<out>/dist/SHA256SUMS.txt`.

use std::collections::BTreeSet;
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

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::fail::{Fail, OrFail, Result, after_cleanup};
use crate::{process, random, site, workspace};

const DEFAULT_OUT: &str = "target/pgo-release";
const DEFAULT_CPU: &str = "x86-64-v3";
const WEB_READY: &str = "central-web: listening on ";
const SERVICE_READY: &str = "central: development listener at http://127.0.0.1:";

/// Command options for `cargo xtask pgo`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// Output directory for `central.profdata` and `dist/` binaries.
    pub out: String,
    /// Target CPU microarchitecture passed to `-C target-cpu=...`.
    pub target_cpu: String,
    /// Only collect and merge `.profraw` into `<out>/central.profdata`; skip stage-4 build.
    pub profile_only: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            out: DEFAULT_OUT.to_owned(),
            target_cpu: DEFAULT_CPU.to_owned(),
            profile_only: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FlagMode<'a> {
    Generate(&'a Path),
    Use(&'a Path),
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
    let out_dir = resolve_out_dir(root, &opts.out);
    let profraw_dir = out_dir.join("profraw");
    let profdata_path = out_dir.join("central.profdata");
    let gen_target = root.join("target/pgo-gen");
    let use_target = root.join("target/pgo-use");

    reset_dir(&profraw_dir, "PGO profile directory could not be prepared")?;
    let profraw_abs = profraw_dir
        .canonicalize()
        .or_fail("PGO profile directory could not be resolved")?;

    println!(
        "PGO stage 1/4: building instrumented binaries (target-cpu={}, profile=release-pgo)...",
        opts.target_cpu
    );
    build_binaries(
        root,
        &gen_target,
        &opts.target_cpu,
        FlagMode::Generate(&profraw_abs),
    )?;

    println!("PGO stage 2/4: collecting realistic training profiles...");
    exercise_workload(root, &gen_target.join("release-pgo"), &profraw_abs)?;

    let raw_files = collect_profraw(&profraw_abs)?;
    if raw_files.is_empty() {
        return Err(Fail::new(
            "PGO workload produced no non-empty .profraw files",
        ));
    }

    println!(
        "PGO stage 3/4: merging {} .profraw files via {}...",
        raw_files.len(),
        llvm_profdata.display()
    );
    merge_profdata(&llvm_profdata, &raw_files, &profdata_path)?;
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

    if opts.profile_only {
        println!("PGO profile collection finished (--profile-only).");
        return Ok(());
    }

    println!(
        "PGO stage 4/4: building optimized FatLTO + {} release binaries...",
        opts.target_cpu
    );
    build_binaries(
        root,
        &use_target,
        &opts.target_cpu,
        FlagMode::Use(&profdata_abs),
    )?;

    let dist_dir = out_dir.join("dist");
    stage_dist(&use_target.join("release-pgo"), &dist_dir)?;
    println!("PGO release binaries staged under {}", dist_dir.display());
    Ok(())
}

fn resolve_out_dir(root: &Path, raw: &str) -> PathBuf {
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
    let rustc = env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let libdir_out = process::capture(
        Command::new(&rustc).args(["--print", "target-libdir"]),
        "rustc unavailable while locating llvm-profdata",
    )?;
    let target_libdir = PathBuf::from(String::from_utf8_lossy(&libdir_out.stdout).trim());
    let sysroot_out = process::capture(
        Command::new(&rustc).args(["--print", "sysroot"]),
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
    flags.push("-C".to_owned());
    match mode {
        FlagMode::Generate(dir) => {
            let s = dir.to_str().or_fail("Non-UTF-8 PGO profile directory")?;
            flags.push(format!("profile-generate={s}"));
        }
        FlagMode::Use(file) => {
            let s = file.to_str().or_fail("Non-UTF-8 PGO profile file path")?;
            flags.push(format!("profile-use={s}"));
            flags.push("-C".to_owned());
            flags.push("llvm-args=-pgo-warn-missing-function".to_owned());
        }
    }
    Ok(flags.join("\x1f"))
}

fn build_binaries(
    root: &Path,
    target_dir: &Path,
    target_cpu: &str,
    mode: FlagMode<'_>,
) -> Result<()> {
    let base_flags = env::var("RUSTFLAGS").ok();
    let encoded = encoded_rustflags(base_flags.as_deref(), target_cpu, mode)?;
    let mut cmd = Command::new(workspace::cargo());
    cmd.current_dir(root)
        .args([
            "build",
            "--locked",
            "--profile",
            "release-pgo",
            "--target-dir",
        ])
        .arg(target_dir)
        .args([
            "-p",
            "central-service",
            "-p",
            "central-web-server",
            "-p",
            "central-picture-codec",
            "--bin",
            "central-service",
            "--bin",
            "central-web",
            "--bin",
            "central-picture-worker",
        ])
        .env_remove("RUSTFLAGS")
        .env("CARGO_ENCODED_RUSTFLAGS", encoded);
    process::succeed(&mut cmd, "PGO cargo build failed")
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

fn exercise_workload(root: &Path, bin_dir: &Path, profraw_dir: &Path) -> Result<()> {
    let profile_file = llvm_profile_pattern(profraw_dir)?;
    if !root.join("target/site/manifest.json").is_file() {
        site::run(root)?;
    }
    exercise_web(root, bin_dir, &profile_file)?;
    exercise_service(root, bin_dir, &profile_file)?;
    exercise_picture_worker(root, bin_dir, &profile_file)?;
    Ok(())
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
        .timeout_global(Some(Duration::from_secs(5)))
        .build();
    ureq::Agent::new_with_config(cfg)
}

fn load_asset_paths(site_root: &Path) -> Vec<String> {
    let Ok(bytes) = fs::read(site_root.join("manifest.json")) else {
        return Vec::new();
    };
    let Ok(val) = serde_json::from_slice::<Value>(&bytes) else {
        return Vec::new();
    };
    let mut paths = BTreeSet::new();
    collect_asset_strings(&val, &mut paths);
    paths.into_iter().take(24).collect()
}

fn collect_asset_strings(val: &Value, out: &mut BTreeSet<String>) {
    match val {
        Value::String(s) if s.starts_with("/assets/") => {
            out.insert(s.clone());
        }
        Value::Array(items) => {
            for item in items {
                collect_asset_strings(item, out);
            }
        }
        Value::Object(map) => {
            for value in map.values() {
                collect_asset_strings(value, out);
            }
        }
        _ => {}
    }
}

fn exercise_web(root: &Path, bin_dir: &Path, profile_file: &str) -> Result<()> {
    let binary = bin_path(bin_dir, "central-web");
    if !binary.is_file() {
        return Err(Fail::new("Instrumented central-web binary missing"));
    }
    let port = free_loopback_port()?;
    let origin = format!("http://127.0.0.1:{port}");
    let config_json = serde_json::json!({
        "listen": format!("127.0.0.1:{port}"),
        "origin": origin,
        "site": root.join("target/site"),
        "github": false,
        "fallback": root.join("crates/central-web-server/tests/data"),
    })
    .to_string();

    let mut cmd = Command::new(&binary);
    cmd.arg("--config-stdin")
        .current_dir(root)
        .env("LLVM_PROFILE_FILE", profile_file)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut cmd, 0);
    let mut child = cmd
        .spawn()
        .or_fail("Cannot start instrumented central-web")?;
    let write_res = child
        .stdin
        .take()
        .map(|mut stdin| stdin.write_all(config_json.as_bytes()));
    if !matches!(write_res, Some(Ok(()))) {
        let _ = process::stop_tree(&mut child);
        return Err(Fail::new("Failed writing central-web PGO stdin config"));
    }

    let outcome = (|| -> Result<()> {
        let _ready = wait_ready_line(&mut child, WEB_READY, "central-web")?;
        let agent = http_agent();
        let base = format!("http://127.0.0.1:{port}");
        let routes = [
            "/robots.txt",
            "/en/",
            "/de/",
            "/ja/",
            "/en/security",
            "/de/security",
            "/en/gateways",
            "/en/faq",
            "/en/download",
            "/en/signin",
            "/en/register",
            "/en/invite/AAAAAAAAAAAAAAAAAAAAAA",
            "/en/account",
            "/en/account/chat",
            "/en/nope",
            "/static/audio/security/en/cues.json",
        ];
        for _ in 0..8 {
            for route in routes {
                let _ = agent
                    .get(&format!("{base}{route}"))
                    .header("Accept-Encoding", "br, gzip")
                    .call();
            }
            let _ = agent
                .get(&format!("{base}/download"))
                .header("Accept-Language", "de-DE,de;q=0.9,en;q=0.8")
                .call();
        }
        for asset in load_asset_paths(&root.join("target/site")) {
            let url = format!("{base}{asset}");
            if let Ok(resp) = agent.get(&url).header("Accept-Encoding", "br, gzip").call()
                && let Some(etag) = resp.headers().get("etag").and_then(|v| v.to_str().ok())
            {
                let _ = agent.get(&url).header("If-None-Match", etag).call();
            }
        }
        Ok(())
    })();
    let cleanup = graceful_stop(&mut child, "central-web");
    after_cleanup(outcome, cleanup)
}

fn exercise_service(root: &Path, bin_dir: &Path, profile_file: &str) -> Result<()> {
    let binary = bin_path(bin_dir, "central-service");
    if !binary.is_file() {
        return Err(Fail::new("Instrumented central-service binary missing"));
    }
    let mut cmd = Command::new(&binary);
    cmd.arg("--dev")
        .current_dir(root)
        .env("LLVM_PROFILE_FILE", profile_file)
        .env("BAYLEE_CENTRAL_LISTEN", "127.0.0.1:0")
        .env("BAYLEE_CENTRAL_REQUEST_BYTES", "2048")
        .env("BAYLEE_CENTRAL_CONCURRENT_REQUESTS", "4");
    let mut child = process::spawn_grouped(&mut cmd, "Cannot start instrumented central-service")?;

    let outcome = (|| -> Result<()> {
        let line = wait_ready_line(&mut child, SERVICE_READY, "central-service")?;
        let port: u16 = line
            .strip_prefix(SERVICE_READY)
            .and_then(|s| s.trim().parse().ok())
            .or_fail("Unexpected central-service listen line")?;
        let agent = http_agent();
        let base = format!("http://127.0.0.1:{port}");
        for _ in 0..25 {
            for path in [
                "/health/live",
                "/health/ready",
                "/auth/login",
                "/api/v1/unknown",
            ] {
                let _ = agent.get(&format!("{base}{path}")).call();
            }
            let _ = agent
                .get(&format!("{base}/health/live"))
                .header("Origin", "https://attacker.invalid")
                .call();
            let _ = agent
                .post(&format!("{base}/api/v1/bootstrap"))
                .header("Content-Type", "application/json")
                .send(r#"{"kind":"friend_invite_info","code":"AAAAAAAAAAAAAAAAAAAAAA"}"#);
        }
        Ok(())
    })();
    let cleanup = graceful_stop(&mut child, "central-service");
    after_cleanup(outcome, cleanup)
}

fn exercise_picture_worker(root: &Path, bin_dir: &Path, profile_file: &str) -> Result<()> {
    let binary = bin_path(bin_dir, "central-picture-worker");
    if !binary.is_file() {
        return Err(Fail::new(
            "Instrumented central-picture-worker binary missing",
        ));
    }
    let _ = process::capture(
        Command::new(&binary)
            .arg("--probe")
            .current_dir(root)
            .env("LLVM_PROFILE_FILE", profile_file),
        "Failed invoking central-picture-worker --probe",
    )?;

    let png_payload = minimal_rgba_png(32, 32);
    let valid_frame = frame_picture_request(1, &png_payload);
    for _ in 0..4 {
        let mut cmd = Command::new(&binary);
        cmd.arg("--no-ffmpeg")
            .current_dir(root)
            .env("LLVM_PROFILE_FILE", profile_file);
        let output = process::feed(&mut cmd, &valid_frame, "PGO picture worker run failed")?;
        if !output.status.success() {
            return Err(Fail::new(
                "central-picture-worker failed on valid PNG frame",
            ));
        }
    }
    let bad_frame = frame_picture_request(1, b"not-a-valid-png");
    let mut bad_cmd = Command::new(&binary);
    bad_cmd
        .arg("--no-ffmpeg")
        .current_dir(root)
        .env("LLVM_PROFILE_FILE", profile_file);
    let bad_out = process::feed(
        &mut bad_cmd,
        &bad_frame,
        "PGO picture worker error path failed",
    )?;
    if !bad_out.status.success() {
        return Err(Fail::new(
            "central-picture-worker failed to serialize clean refusal",
        ));
    }
    Ok(())
}

/// Frames a request matching `central_picture::protocol` (`BCPQ`, version 2, format, u32 BE len).
pub fn frame_picture_request(format_code: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(10 + payload.len());
    out.extend_from_slice(b"BCPQ");
    out.push(2);
    out.push(format_code);
    let len = u32::try_from(payload.len()).unwrap_or(u32::MAX);
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(payload);
    out
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFF_u32;
    for &b in bytes {
        crc ^= u32::from(b);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

fn adler32(bytes: &[u8]) -> u32 {
    const MOD: u32 = 65_521;
    let mut s1 = 1_u32;
    let mut s2 = 0_u32;
    for &b in bytes {
        s1 = (s1 + u32::from(b)) % MOD;
        s2 = (s2 + s1) % MOD;
    }
    (s2 << 16) | s1
}

fn push_png_chunk(out: &mut Vec<u8>, tag: [u8; 4], data: &[u8]) {
    let len = u32::try_from(data.len()).unwrap_or(0);
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(&tag);
    out.extend_from_slice(data);
    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(&tag);
    crc_input.extend_from_slice(data);
    out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
}

/// Generates a minimal deterministic RGBA PNG image using uncompressed deflate stored blocks.
fn minimal_rgba_png(width: u16, height: u16) -> Vec<u8> {
    let row_bytes = 1 + usize::from(width) * 4;
    let mut raw = Vec::with_capacity(usize::from(height) * row_bytes);
    for y in 0..height {
        raw.push(0); // filter type 0
        for x in 0..width {
            let xv = u32::from(x);
            let yv = u32::from(y);
            raw.push(u8::try_from((xv * 7 + yv * 3) & 0xff).unwrap_or(0));
            raw.push(u8::try_from((yv * 9 + 40) & 0xff).unwrap_or(0));
            raw.push(u8::try_from(((xv ^ yv) * 11) & 0xff).unwrap_or(0));
            raw.push(255);
        }
    }
    // zlib wrapper around single uncompressed stored block (raw.len() <= 65535).
    let raw_len = u16::try_from(raw.len()).unwrap_or(u16::MAX);
    let mut zlib = Vec::with_capacity(raw.len() + 11);
    zlib.extend_from_slice(&[0x78, 0x01, 0x01]);
    zlib.extend_from_slice(&raw_len.to_le_bytes());
    zlib.extend_from_slice(&(!raw_len).to_le_bytes());
    zlib.extend_from_slice(&raw[..usize::from(raw_len)]);
    zlib.extend_from_slice(&adler32(&raw[..usize::from(raw_len)]).to_be_bytes());

    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&u32::from(width).to_be_bytes());
    ihdr.extend_from_slice(&u32::from(height).to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]); // 8-bit RGBA

    let mut png = Vec::new();
    png.extend_from_slice(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    push_png_chunk(&mut png, *b"IHDR", &ihdr);
    push_png_chunk(&mut png, *b"IDAT", &zlib);
    push_png_chunk(&mut png, *b"IEND", &[]);
    png
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

fn stage_dist(use_bin_dir: &Path, dist_dir: &Path) -> Result<()> {
    reset_dir(dist_dir, "PGO dist directory could not be prepared")?;
    let mut sums = String::new();
    for name in ["central-service", "central-web", "central-picture-worker"] {
        let src = bin_path(use_bin_dir, name);
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
    fn encodes_rustflags_for_generate_and_use() {
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
    }

    #[test]
    fn builds_valid_bcpq_frame_and_png_header() {
        let png = minimal_rgba_png(8, 8);
        assert!(png.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]));
        let framed = frame_picture_request(1, &png);
        assert_eq!(&framed[..6], b"BCPQ\x02\x01");
        assert_eq!(
            u32::from_be_bytes(framed[6..10].try_into().unwrap()) as usize,
            png.len()
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
