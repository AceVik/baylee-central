//! `cargo xtask site`: the website's assets (`docs/proposals/frontend-rewrite.md`
//! §3), written to `target/site/` for `central-web` to serve:
//!
//! - the islands' WebAssembly (`central-web-islands`, `--profile islands`,
//!   paths remapped) and its wasm-bindgen glue (the pinned CLI of `wasm`);
//! - the stylesheet (`crates/central-web/style/`, in [`STYLES`]' order), the
//!   scripts (`crates/central-web/js/`), the fonts and pictures
//!   (`crates/central-web/assets/`);
//! - every file under a content-hashed name (`/assets/<name>.<hash>.<ext>`)
//!   with a Subresource Integrity value, in `manifest.json`; a stylesheet's
//!   `url("/assets/<name>")` is rewritten to the hashed name;
//! - Brotli and gzip copies of text files where `brotli` and `gzip` exist;
//! - `notices.txt`: the licences of everything the browser receives;
//! - with `BAYLEE_SRC` (a Baylee checkout, as the deploy hook has), the
//!   client's screenshots and a fallback card status, which carry Wizards'
//!   material and so never enter this repository.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};
use sha2::{Digest, Sha256, Sha384};

use crate::fail::{Fail, OrFail, Result};
use crate::{process, random, wasm, workspace};

/// The stylesheets, in the order they are joined.
pub const STYLES: [&str; 14] = [
    "tokens.css",
    "base.css",
    "buttons.css",
    "layout.css",
    "header.css",
    "components.css",
    "hero.css",
    "pages.css",
    "home.css",
    "gateways.css",
    "album.css",
    "footer.css",
    "app.css",
    "chat.css",
];

/// The scripts served under hashed names.
const SCRIPTS: [&str; 5] = [
    "boot.js",
    "pwa.js",
    "pow-worker.js",
    "home.js",
    "directory.js",
];

const TARGET: &str = "wasm32-unknown-unknown";
const PROFILE: &str = "islands";
const WEB: &str = "crates/central-web";

/// Builds the site into `target/site`.
///
/// # Errors
/// The first step that fails.
pub(crate) fn run(root: &Path) -> Result<()> {
    let target = wasm::target_dir(root);
    let out = target.join("site");
    if out.exists() {
        fs::remove_dir_all(&out).or_fail("target/site could not be cleared")?;
    }
    fs::create_dir_all(out.join("assets")).or_fail("target/site could not be made")?;
    let mut site = Site {
        out: out.clone(),
        files: BTreeMap::new(),
    };

    let web = root.join(WEB);
    copy_tree(&mut site, &web.join("assets/fonts"), "fonts")?;
    copy_tree(&mut site, &web.join("assets/img"), "img")?;
    if let Some(baylee) = std::env::var_os("BAYLEE_SRC") {
        content(&mut site, Path::new(&baylee))?;
    }
    for name in SCRIPTS {
        let bytes = fs::read(web.join("js").join(name)).or_fail("a script is missing")?;
        site.add(name, &bytes)?;
    }
    islands(root, &target, &mut site)?;
    crypto(root, &mut site)?;
    let mut css = String::new();
    for name in STYLES {
        css.push_str(&lean_css(
            &fs::read_to_string(web.join("style").join(name)).or_fail("a stylesheet is missing")?,
        )?);
    }
    let css = site.rewrite(&css)?;
    site.add("site.css", css.as_bytes())?;

    for name in ["favicon.ico", "apple-touch-icon.png"] {
        fs::copy(web.join("assets").join(name), out.join(name))
            .or_fail("a root file is missing")?;
    }
    let worker = service_worker(&web.join("js/sw.js"), &site.files)?;
    fs::write(out.join("sw.js"), worker).or_fail("sw.js could not be written")?;
    fs::write(out.join("notices.txt"), notices(root)?)
        .or_fail("notices.txt could not be written")?;
    compress(&out)?;
    let manifest = json!({ "files": site.files });
    fs::write(out.join("manifest.json"), format!("{manifest:#}\n"))
        .or_fail("manifest.json could not be written")?;
    println!(
        "Site built into {} ({} assets).",
        out.display(),
        site.files.len()
    );
    Ok(())
}

struct Site {
    out: PathBuf,
    files: BTreeMap<String, Value>,
}

impl Site {
    /// Writes `bytes` as `logical` under its hashed name and records it.
    fn add(&mut self, logical: &str, bytes: &[u8]) -> Result<String> {
        let hash = random::to_hex(&Sha256::digest(bytes));
        let (stem, extension) = logical
            .rsplit_once('.')
            .or_fail("an asset has no extension")?;
        let hashed = format!("{stem}.{}.{extension}", &hash[..12]);
        let path = self.out.join("assets").join(&hashed);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).or_fail("an asset folder could not be made")?;
        }
        fs::write(&path, bytes).or_fail("an asset could not be written")?;
        let url = format!("/assets/{hashed}");
        let integrity = format!("sha384-{}", base64(&Sha384::digest(bytes)));
        self.files.insert(
            logical.to_owned(),
            json!({ "url": url, "integrity": integrity }),
        );
        Ok(url)
    }

    /// `url("/assets/<logical>")` in a stylesheet, pointed at the hashed file.
    fn rewrite(&self, css: &str) -> Result<String> {
        let mut out = String::with_capacity(css.len());
        let mut rest = css;
        while let Some(at) = rest.find("url(\"/assets/") {
            out.push_str(&rest[..at]);
            let after = &rest[at + "url(\"/assets/".len()..];
            let end = after
                .find("\")")
                .or_fail("a stylesheet has an unclosed url()")?;
            let logical = &after[..end];
            let url = self
                .files
                .get(logical)
                .and_then(|entry| entry["url"].as_str())
                .ok_or_else(|| {
                    Fail::new(format!("the stylesheet names a missing asset: {logical}"))
                })?;
            let _ = write!(out, "url(\"{url}\")");
            rest = &after[end + 2..];
        }
        out.push_str(rest);
        Ok(out)
    }
}

/// The service worker with this build's stamp and the files the offline
/// page needs: the stylesheet, the scripts every page loads, the fonts and
/// the icon. The stamp is the digest of every asset's address, so a build
/// that changes any file installs as a new service worker.
fn service_worker(source: &Path, files: &BTreeMap<String, Value>) -> Result<String> {
    let text = fs::read_to_string(source).or_fail("the service worker is missing")?;
    let url = |name: &str| -> Result<String> {
        files
            .get(name)
            .and_then(|f| f.get("url"))
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_fail("an asset the offline page needs is missing")
    };
    let mut offline = vec![
        url("site.css")?,
        url("boot.js")?,
        url("pwa.js")?,
        url("img/icon.svg")?,
    ];
    let font = |name: &&String| {
        name.starts_with("fonts/")
            && Path::new(name.as_str())
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("woff2"))
    };
    for name in files.keys().filter(font) {
        offline.push(url(name)?);
    }
    let every: String = files
        .values()
        .filter_map(|f| f.get("url").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n");
    let build = &random::to_hex(&Sha256::digest(every.as_bytes()))[..16];
    let assets =
        serde_json::to_string(&offline).or_fail("the offline list could not be written")?;
    let stamped = text
        .replacen("\"__BUILD__\"", &format!("\"{build}\""), 1)
        .replacen(
            "/* the offline page's files */ []",
            &format!("/* the offline page's files */ {assets}"),
            1,
        );
    if stamped.contains("__BUILD__") || !stamped.contains(&assets) {
        return Err(Fail::new("the service worker's placeholders changed"));
    }
    Ok(stamped)
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0_u32, |n, (i, b)| n | (u32::from(*b) << (16 - 8 * i)));
        for i in 0..4 {
            if i <= chunk.len() {
                let index = usize::try_from((n >> (18 - 6 * i)) & 63).unwrap_or(0);
                out.push(char::from(ALPHABET[index]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Every file under `dir`, as `<prefix>/<relative path>`.
fn copy_tree(site: &mut Site, dir: &Path, prefix: &str) -> Result<()> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .or_fail("an asset folder is missing")?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect();
    entries.sort();
    for path in entries {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .or_fail("an asset name is not text")?;
        let logical = format!("{prefix}/{name}");
        if path.is_dir() {
            copy_tree(site, &path, &logical)?;
        } else {
            let bytes = fs::read(&path).or_fail("an asset could not be read")?;
            site.add(&logical, &bytes)?;
        }
    }
    Ok(())
}

/// The client's screenshots and a fallback card status from a Baylee
/// checkout (`web/site/img/shot-*`, `web/site/data/sets.json`).
fn content(site: &mut Site, baylee: &Path) -> Result<()> {
    let img = baylee.join("web/site/img");
    let mut shots = 0;
    for entry in fs::read_dir(&img)
        .or_fail("BAYLEE_SRC has no web/site/img")?
        .flatten()
    {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(base) = name.strip_prefix("shot-") else {
            continue;
        };
        let (stem, extension) = base.rsplit_once('.').unwrap_or((base, ""));
        if !matches!(extension, "avif" | "webp")
            || !stem
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b == b'-' || b.is_ascii_digit())
        {
            continue;
        }
        let logical = match stem.strip_suffix("-640") {
            Some(stem) => format!("media/shot-{stem}-640.{extension}"),
            None => format!("media/shot-{stem}-1200.{extension}"),
        };
        site.add(
            &logical,
            &fs::read(entry.path()).or_fail("a screenshot could not be read")?,
        )?;
        shots += 1;
    }
    let sets: Value = serde_json::from_str(
        &fs::read_to_string(baylee.join("web/site/data/sets.json"))
            .or_fail("BAYLEE_SRC has no card status")?,
    )
    .or_fail("Baylee's sets.json is malformed")?;
    let tag = process::capture(
        Command::new("git")
            .arg("-C")
            .arg(baylee)
            .args(["describe", "--tags", "--abbrev=0"]),
        "git describe failed in BAYLEE_SRC",
    )
    .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
    .unwrap_or_default();
    let number = |key: &str| sets["totals"][key].as_u64().unwrap_or(0);
    let cards = json!({
        "stats": {
            "implemented": number("implemented"),
            "generated": number("generated"),
            "partial": number("partial"),
            "total": sets["corpus"].as_u64().unwrap_or(0),
        },
        "tag": tag,
        "date": sets["generated_at"].as_str().and_then(|d| d.get(..10)).unwrap_or(""),
    });
    let fallback = site.out.join("fallback");
    fs::create_dir_all(&fallback).or_fail("target/site/fallback could not be made")?;
    fs::write(fallback.join("cards.json"), format!("{cards:#}\n"))
        .or_fail("cards.json could not be written")?;
    println!("Content from BAYLEE_SRC: {shots} screenshots and the card status.");
    Ok(())
}

/// The islands' module and glue, and the bootstrap that loads them.
/// A stylesheet without its comments, blank lines and indentation: a
/// fifth less to send before the first paint. Strings are kept as written
/// (a comment's opening inside one is text); a string broken over a line
/// is refused, since its indentation would be part of it.
fn lean_css(css: &str) -> Result<String> {
    let mut out = String::with_capacity(css.len());
    let mut chars = css.chars().peekable();
    let mut quote: Option<char> = None;
    while let Some(c) = chars.next() {
        match quote {
            Some(q) => {
                out.push(c);
                if c == '\\' {
                    match chars.next() {
                        Some('\n') | None => {
                            return Err(Fail::new("a stylesheet's string runs over a line"));
                        }
                        Some(next) => out.push(next),
                    }
                } else if c == '\n' {
                    return Err(Fail::new("a stylesheet's string is not closed"));
                } else if c == q {
                    quote = None;
                }
            }
            None if c == '/' && chars.peek() == Some(&'*') => {
                chars.next();
                let mut last = '\0';
                loop {
                    match chars.next() {
                        Some('/') if last == '*' => break,
                        Some(next) => last = next,
                        None => return Err(Fail::new("a stylesheet's comment is not closed")),
                    }
                }
            }
            None => {
                if c == '"' || c == '\'' {
                    quote = Some(c);
                }
                out.push(c);
            }
        }
    }
    let mut lean = String::with_capacity(out.len());
    for line in out.lines().map(str::trim).filter(|line| !line.is_empty()) {
        lean.push_str(line);
        lean.push('\n');
    }
    Ok(lean)
}

fn islands(root: &Path, target: &Path, site: &mut Site) -> Result<()> {
    let bindgen = wasm::cli(target)?;
    let home = wasm::cargo_home()?;
    process::succeed(
        Command::new(workspace::cargo())
            .args([
                "build",
                "--locked",
                "-p",
                "central-web-islands",
                "--lib",
                "--target",
                TARGET,
                "--profile",
                PROFILE,
            ])
            .env("CARGO_ENCODED_RUSTFLAGS", wasm::remap_flags(root, &home))
            .current_dir(root),
        "the islands' WebAssembly build failed",
    )?;
    let module = target
        .join(TARGET)
        .join(PROFILE)
        .join("central_web_islands.wasm");
    let glue = target.join("site-bindgen");
    if glue.exists() {
        fs::remove_dir_all(&glue).or_fail("the glue folder could not be cleared")?;
    }
    process::succeed(
        Command::new(&bindgen)
            .args([
                "--target",
                "web",
                "--remove-name-section",
                "--remove-producers-section",
                "--out-name",
                "islands",
                "--out-dir",
            ])
            .arg(&glue)
            .arg(&module),
        "wasm-bindgen failed for the islands",
    )?;
    let wasm_bytes =
        fs::read(glue.join("islands_bg.wasm")).or_fail("the islands' module is missing")?;
    for local in [root, home.as_path()] {
        if wasm::contains(&wasm_bytes, local.to_string_lossy().as_bytes()) {
            return Err(Fail::new("the islands' module names a local path"));
        }
    }
    let wasm_url = site.add("islands.wasm", &wasm_bytes)?;
    let glue_url = site.add(
        "islands-glue.js",
        &fs::read(glue.join("islands.js")).or_fail("the islands' glue is missing")?,
    )?;
    let template = fs::read_to_string(root.join(WEB).join("js/islands.js"))
        .or_fail("js/islands.js is missing")?;
    let boot = template
        .replace("./islands-glue.js", &glue_url)
        .replace("./islands.wasm", &wasm_url);
    if boot == template {
        return Err(Fail::new(
            "js/islands.js names neither ./islands-glue.js nor ./islands.wasm",
        ));
    }
    site.add("islands.js", boot.as_bytes())?;
    Ok(())
}

/// The committed browser crypto (`web/src/central-wasm`, which `wasm
/// --check` proves to be what the source builds), checked against its
/// `SHA256SUMS`, and the Worker that loads it.
fn crypto(root: &Path, site: &mut Site) -> Result<()> {
    let dir = root.join(wasm::OUTPUT);
    let sums =
        fs::read_to_string(dir.join(wasm::SUMS)).or_fail("the crypto's SHA256SUMS is missing")?;
    let read = |name: &str| -> Result<Vec<u8>> {
        let bytes = fs::read(dir.join(name)).or_fail("a crypto file is missing")?;
        let want = sums
            .lines()
            .find_map(|line| line.strip_suffix(name).map(str::trim))
            .or_fail("SHA256SUMS does not name a crypto file")?;
        if random::to_hex(&Sha256::digest(&bytes)) != want {
            return Err(Fail::new(format!("{name} does not match SHA256SUMS")));
        }
        Ok(bytes)
    };
    let glue = read("central_web_crypto.js")?;
    let module = read("central_web_crypto_bg.wasm")?;
    let glue_url = site.add("crypto-glue.js", &glue)?;
    let module_url = site.add("crypto.wasm", &module)?;
    let template = fs::read_to_string(root.join(WEB).join("js/crypto-worker.js"))
        .or_fail("js/crypto-worker.js is missing")?;
    let worker = template
        .replace("\"./central_web_crypto.js\"", &format!("\"{glue_url}\""))
        .replace(
            "\"./central_web_crypto_bg.wasm\"",
            &format!("\"{module_url}\""),
        );
    if worker == template {
        return Err(Fail::new("js/crypto-worker.js names neither crypto file"));
    }
    site.add("crypto-worker.js", worker.as_bytes())?;
    Ok(())
}

/// Brotli and gzip copies of the text files, where the tools exist; a
/// missing tool is no failure (Caddy compresses then).
fn compress(out: &Path) -> Result<()> {
    let have = |tool: &str| {
        Command::new(tool)
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
    };
    let (brotli, gzip) = (have("brotli"), have("gzip"));
    let mut stack = vec![out.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir)
            .or_fail("target/site could not be read")?
            .flatten()
        {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let text = path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
                matches!(e, "css" | "js" | "wasm" | "svg" | "txt" | "json" | "ico")
            });
            if !text || path.file_name().is_some_and(|n| n == "manifest.json") {
                continue;
            }
            if brotli {
                process::succeed(
                    Command::new("brotli")
                        .args(["-q", "11", "-k", "-f"])
                        .arg(&path),
                    "brotli failed",
                )?;
            }
            if gzip {
                process::succeed(
                    Command::new("gzip")
                        .args(["-9", "-k", "-f", "-n"])
                        .arg(&path),
                    "gzip failed",
                )?;
            }
        }
    }
    Ok(())
}

/// The licences of everything the browser receives: the fonts, the icons,
/// and every crate in the islands' module, each with its own licence files.
fn notices(root: &Path) -> Result<String> {
    let mut text = String::from(
        "baylee.app: third-party notices\n\
         ================================\n\n\
         Fonts: Alegreya Sans (Copyright 2013 The Alegreya Sans Project Authors) and Faustina\n\
         (Copyright 2016 The Faustina Project Authors), SIL Open Font License 1.1; the licence\n\
         texts are served beside the fonts.\n\n\
         Icons: Lucide (lucide-static 1.60.0), ISC License, Copyright (c) 2026 Lucide Icons and\n\
         Contributors; parts derived from Feather, MIT License, Copyright (c) 2013-2023 Cole Bemis.\n\n\
         The interactive parts are Rust compiled to WebAssembly from these crates:\n\n",
    );
    let output = process::capture(
        Command::new(workspace::cargo())
            .args([
                "metadata",
                "--format-version",
                "1",
                "--locked",
                "--offline",
                "--filter-platform",
                TARGET,
            ])
            .current_dir(root),
        "cargo metadata failed",
    )?;
    let metadata: Value =
        serde_json::from_slice(&output.stdout).or_fail("cargo metadata is malformed")?;
    let reachable = islands_graph(&metadata);
    let mut texts: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for package in metadata["packages"].as_array().into_iter().flatten() {
        let id = package["id"].as_str().unwrap_or("");
        if !reachable.contains(id) || package["source"].is_null() {
            continue;
        }
        let name = format!(
            "{} {}",
            package["name"].as_str().unwrap_or(""),
            package["version"].as_str().unwrap_or("")
        );
        let _ = writeln!(
            text,
            "- {name}: {}",
            package["license"]
                .as_str()
                .unwrap_or("see its licence files")
        );
        let dir = Path::new(package["manifest_path"].as_str().unwrap_or(""))
            .parent()
            .map(Path::to_path_buf);
        for entry in dir
            .and_then(|dir| fs::read_dir(dir).ok())
            .into_iter()
            .flatten()
            .flatten()
        {
            let file = entry.file_name().to_string_lossy().to_uppercase();
            let licence = ["LICENSE", "LICENCE", "COPYING", "NOTICE"]
                .iter()
                .any(|start| file.starts_with(start));
            if licence && let Ok(body) = fs::read_to_string(entry.path()) {
                texts.entry(body).or_default().push(name.clone());
            }
        }
    }
    let assets = root.join(WEB).join("assets");
    for (title, file) in [
        ("Alegreya Sans", "fonts/OFL-AlegreyaSans.txt"),
        ("Faustina", "fonts/OFL-Faustina.txt"),
        ("Lucide", "LICENSE-lucide.txt"),
    ] {
        let body = fs::read_to_string(assets.join(file)).or_fail("a licence text is missing")?;
        let _ = write!(
            text,
            "\n----------------------------------------------------------------\n{title}\n\n{body}\n"
        );
    }
    text.push_str(
        "\nThe crates' licence texts follow, each once, with the crates it belongs to.\n",
    );
    for (body, owners) in texts {
        let _ = write!(
            text,
            "\n----------------------------------------------------------------\n{}\n\n{body}\n",
            owners.join(", ")
        );
    }
    Ok(text)
}

/// The ids of every package `central-web-islands` reaches through normal
/// dependencies on the browser's target.
fn islands_graph(metadata: &Value) -> std::collections::BTreeSet<String> {
    let nodes = metadata["resolve"]["nodes"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let start = metadata["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|package| package["name"] == "central-web-islands")
        .and_then(|package| package["id"].as_str())
        .unwrap_or("")
        .to_owned();
    let mut seen = std::collections::BTreeSet::new();
    let mut stack = vec![start];
    while let Some(id) = stack.pop() {
        if !seen.insert(id.clone()) {
            continue;
        }
        let Some(node) = nodes.iter().find(|node| node["id"] == id.as_str()) else {
            continue;
        };
        for dependency in node["deps"].as_array().into_iter().flatten() {
            let normal = dependency["dep_kinds"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|kind| kind["kind"].is_null());
            if normal && let Some(pkg) = dependency["pkg"].as_str() {
                stack.push(pkg.to_owned());
            }
        }
    }
    seen
}

#[cfg(test)]
mod tests {
    use super::lean_css;

    #[test]
    fn a_lean_stylesheet_keeps_rules_and_strings() {
        let css = "/* a note\n   on two lines */\n.a {\n  content: \"/* not a comment */\";\n  color: red; /* why */\n}\n\n\n.b{quotes:'\\'' '\\''}\n";
        assert_eq!(
            lean_css(css).expect("lean"),
            ".a {\ncontent: \"/* not a comment */\";\ncolor: red;\n}\n.b{quotes:'\\'' '\\''}\n"
        );
        assert_eq!(lean_css("/*/ still */.c{}").expect("lean"), ".c{}\n");
        assert!(lean_css(".a{content:\"x\\\ny\"}").is_err());
        assert!(lean_css(".a{content:\"x\n\"}").is_err());
        assert!(lean_css(".a{} /* open").is_err());
    }
}
