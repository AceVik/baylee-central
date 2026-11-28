//! The training of `cargo xtask pgo` (step 2): what the profiled binaries do
//! in production, on loopback and with synthetic data only.
//!
//! - `central-web` in its deployed mode (`--config-stdin`) with the site
//!   `cargo xtask site` built, the synthetic data of
//!   `crates/central-web-server/tests/data` and a count of page views: every
//!   page under every language prefix ([`LANGS`], [`SLUGS`]), the addresses
//!   without one ([`ROOT_PATHS`]) in several browsers' languages, the card
//!   suggestions, and every hashed asset with each encoding and again with
//!   its `ETag`.
//! - `central-picture-worker`, one process per picture as systemd starts
//!   it: pictures ffmpeg draws from its `testsrc2` pattern in each format a
//!   browser uploads ([`DRAWN`]), the codec tests' fixtures, an ICO and an
//!   SVG made here, its own AVIF answer read back, and files it must
//!   refuse. ffmpeg is required: without it the decoders of the formats it
//!   draws would go untrained, and untrained code under a profile is cold.
//!
//! Nothing here is a photo or a person, and nothing leaves the machine:
//! `central-web` runs without GitHub and without Central.

use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use serde_json::Value;

use super::{bin_path, free_loopback_port, graceful_stop, http_agent, wait_ready_line};
use crate::fail::{Fail, OrFail, Result, after_cleanup};
use crate::owned::OwnedDir;
use crate::{process, site};

/// The languages' path prefixes (`central_web::words::Lang::ALL`).
pub(super) const LANGS: [&str; 11] = [
    "en", "de", "fr", "it", "es", "pt-br", "ru", "ja", "ko", "zh-hans", "zh-hant",
];

/// The pages under a language prefix (`central_web::site::Page`): the
/// listed ones, sign-in, registration, reset and offline, the account
/// area's sections as a visitor without a session gets them, a player's
/// profile, an invitation, a page removed for good (410) and one the site
/// never had (404).
pub(super) const SLUGS: [&str; 28] = [
    "",
    "download",
    "gateways",
    "donate",
    "faq",
    "security",
    "contact",
    "impressum",
    "privacy",
    "terms",
    "fan-content",
    "signin",
    "register",
    "reset",
    "offline",
    "account",
    "account/chat",
    "account/contacts",
    "account/profile",
    "account/security",
    "account/privacy",
    "account/gateways",
    "account/notifications",
    "account/admin",
    "player/bayleefan-1a2b",
    "invite/AAAAAAAAAAAAAAAAAAAAAA",
    "challenges",
    "nope",
];

/// Addresses without a language: the redirects, which the browser's
/// languages choose, the generated files and one the site does not have.
const ROOT_PATHS: [&str; 13] = [
    "/",
    "/download",
    "/cards",
    "/agb",
    "/datenschutz",
    "/admin",
    "/robots.txt",
    "/sitemap.xml",
    "/funding.json",
    "/.well-known/tdmrep.json",
    "/sw.js",
    "/favicon.ico",
    "/nope",
];

/// `Accept-Language` as browsers send it.
const ACCEPT_LANGUAGES: [&str; 4] = [
    "de-DE,de;q=0.9,en;q=0.8",
    "ja,en-US;q=0.7",
    "pt-BR,pt;q=0.9",
    "*",
];

/// What a visitor types into the card search (`/cards/suggest?q=`).
const CARD_QUERIES: [&str; 10] = [
    "a", "so", "sol", "sol%20r", "light", "dra", "bolt", "the%20", "%C3%A6", "zzzz",
];

/// `Accept-Encoding`: Brotli first, gzip only, neither.
const ENCODINGS: [&str; 3] = ["br, gzip", "gzip", "identity"];

/// Passes over every address in the training.
const WEB_ROUNDS: usize = 6;

/// The largest answer read: the administration's islands uncompressed.
const BODY_LIMIT: u64 = 32 * 1024 * 1024;

/// What `central-web` prints once it listens.
const WEB_READY: &str = "central-web: listening on ";

/// Where the codec tests' pictures are (`tests/fixtures/README.md` there).
const FIXTURES: &str = "crates/central-picture-codec/tests/fixtures";

/// The worker protocol's bytes for the formats (`central_picture::Format::code`).
const PNG: u8 = 1;
const JPEG: u8 = 2;
const GIF: u8 = 3;
const WEBP: u8 = 4;
const AVIF: u8 = 5;
const SVG: u8 = 6;
const BMP: u8 = 7;
const ICO: u8 = 8;

/// A picture ffmpeg draws: its file name, size, frames, format byte and
/// the encoder's arguments.
struct Drawn {
    name: &'static str,
    size: &'static str,
    frames: u32,
    format: u8,
    encoder: &'static [&'static str],
}

/// What a browser uploads, at the sizes phones and screens make, drawn from
/// ffmpeg's `testsrc2` pattern (colour bars, gradients, moving shapes).
const DRAWN: [Drawn; 9] = [
    Drawn {
        name: "photo.jpg",
        size: "1600x1200",
        frames: 1,
        format: JPEG,
        encoder: &["-c:v", "mjpeg", "-q:v", "3", "-pix_fmt", "yuvj420p"],
    },
    Drawn {
        name: "portrait.jpg",
        size: "720x1280",
        frames: 1,
        format: JPEG,
        encoder: &["-c:v", "mjpeg", "-q:v", "6", "-pix_fmt", "yuvj444p"],
    },
    Drawn {
        name: "screen.png",
        size: "1024x768",
        frames: 1,
        format: PNG,
        encoder: &["-c:v", "png", "-pix_fmt", "rgba"],
    },
    Drawn {
        name: "square.png",
        size: "256x256",
        frames: 1,
        format: PNG,
        encoder: &["-c:v", "png", "-pix_fmt", "rgb24"],
    },
    Drawn {
        name: "lossy.webp",
        size: "1024x768",
        frames: 1,
        format: WEBP,
        encoder: &["-c:v", "libwebp", "-lossless", "0", "-quality", "80"],
    },
    Drawn {
        name: "lossless.webp",
        size: "512x512",
        frames: 1,
        format: WEBP,
        encoder: &["-c:v", "libwebp", "-lossless", "1"],
    },
    Drawn {
        name: "still.gif",
        size: "512x512",
        frames: 1,
        format: GIF,
        encoder: &["-c:v", "gif"],
    },
    Drawn {
        name: "moving.gif",
        size: "320x320",
        frames: 6,
        format: GIF,
        encoder: &["-c:v", "gif", "-loop", "0"],
    },
    Drawn {
        name: "bitmap.bmp",
        size: "800x600",
        frames: 1,
        format: BMP,
        encoder: &["-c:v", "bmp"],
    },
];

/// The names of the pictures ffmpeg draws.
#[cfg(test)]
pub(super) fn drawn_names() -> impl Iterator<Item = &'static str> {
    DRAWN.iter().map(|drawn| drawn.name)
}

/// A vector picture with a gradient, an opacity and a curve; no text, so no
/// font is needed.
const SVG_PICTURE: &str = concat!(
    r##"<svg xmlns="http://www.w3.org/2000/svg" width="512" height="512" viewBox="0 0 512 512">"##,
    r##"<defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1">"##,
    r##"<stop offset="0" stop-color="#336699"/><stop offset="1" stop-color="#f0c040"/>"##,
    r##"</linearGradient></defs><rect width="512" height="512" fill="url(#g)"/>"##,
    r##"<circle cx="256" cy="220" r="120" fill="#ffffff" fill-opacity="0.7"/>"##,
    r##"<path d="M96 448 Q256 300 416 448 Z" fill="#203040"/></svg>"##,
);

/// Conversions of each training picture.
const PICTURE_RUNS: usize = 2;

/// A training picture: what it is called, its format byte, its bytes, and
/// whether the worker must convert it (a fixture may be one it refuses).
pub(super) struct Picture {
    pub(super) name: String,
    pub(super) format: u8,
    pub(super) bytes: Vec<u8>,
    must_convert: bool,
}

/// Runs the training with the instrumented binaries in `bin_dir`, their
/// counts going to `profile_file` (`LLVM_PROFILE_FILE`).
pub(super) fn run(root: &Path, bin_dir: &Path, profile_file: &str) -> Result<()> {
    if !root.join("target/site/manifest.json").is_file() {
        site::run(root)?;
    }
    web(root, bin_dir, profile_file)?;
    pictures(root, bin_dir, profile_file)
}

/// A running `central-web` and its port.
pub(super) struct Web {
    child: Child,
    pub(super) port: u16,
    _views: OwnedDir,
}

impl Web {
    /// Starts `binary` on a free loopback port, its counts going to
    /// `profile_file` where it is instrumented.
    pub(super) fn start(root: &Path, binary: &Path, profile_file: Option<&str>) -> Result<Self> {
        if !binary.is_file() {
            return Err(Fail::new(
                "The central-web binary to train or measure is missing",
            ));
        }
        let views = OwnedDir::new("central-pgo-views-")?;
        let port = free_loopback_port()?;
        let config = serde_json::json!({
            "listen": format!("127.0.0.1:{port}"),
            "origin": format!("http://127.0.0.1:{port}"),
            "site": root.join("target/site"),
            "github": false,
            "fallback": root.join("crates/central-web-server/tests/data"),
            "views": views.path().join("views.json"),
        })
        .to_string();
        let mut cmd = Command::new(binary);
        cmd.arg("--config-stdin")
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(file) = profile_file {
            cmd.env("LLVM_PROFILE_FILE", file);
        }
        #[cfg(unix)]
        std::os::unix::process::CommandExt::process_group(&mut cmd, 0);
        let mut child = cmd.spawn().or_fail("Cannot start central-web")?;
        let wrote = child
            .stdin
            .take()
            .map(|mut stdin| stdin.write_all(config.as_bytes()));
        if !matches!(wrote, Some(Ok(()))) {
            let _ = process::stop_tree(&mut child);
            return Err(Fail::new("Failed writing central-web's configuration"));
        }
        if let Err(failure) = wait_ready_line(&mut child, WEB_READY, "central-web") {
            let _ = process::stop_tree(&mut child);
            return Err(failure);
        }
        Ok(Self {
            child,
            port,
            _views: views,
        })
    }

    /// The address its pages are under.
    pub(super) fn base(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Stops it with `SIGINT`, so an instrumented one writes its counts.
    pub(super) fn stop(mut self) -> Result<()> {
        graceful_stop(&mut self.child, "central-web")
    }
}

fn web(root: &Path, bin_dir: &Path, profile_file: &str) -> Result<()> {
    let server = Web::start(root, &bin_path(bin_dir, "central-web"), Some(profile_file))?;
    let outcome = (|| -> Result<()> {
        let agent = http_agent();
        let base = server.base();
        expect_status(&agent, &format!("{base}/en/"), 200)?;
        expect_status(&agent, &format!("{base}/en/nope"), 404)?;
        expect_status(&agent, &format!("{base}/en/challenges"), 410)?;
        for round in 0..WEB_ROUNDS {
            let (asked, answered) = pages(&agent, &base, round);
            if answered != asked {
                return Err(Fail::new(format!(
                    "central-web answered {answered} of {asked} training requests"
                )));
            }
        }
        assets(&agent, &base, &root.join("target/site"));
        Ok(())
    })();
    after_cleanup(outcome, server.stop())
}

/// One pass over every address of the website at `base`: how many requests
/// it made and how many were answered (whatever their status).
pub(super) fn pages(agent: &ureq::Agent, base: &str, round: usize) -> (usize, usize) {
    let encoding = ENCODINGS[round % ENCODINGS.len()];
    let mut asked = 0;
    let mut answered = 0;
    let mut ask = |url: String, language: Option<&str>| {
        asked += 1;
        let mut headers = vec![("Accept-Encoding", encoding)];
        if let Some(language) = language {
            headers.push(("Accept-Language", language));
        }
        if fetch(agent, &url, &headers).is_some() {
            answered += 1;
        }
    };
    for lang in LANGS {
        for slug in SLUGS {
            ask(format!("{base}/{lang}/{slug}"), None);
        }
    }
    for (at, path) in ROOT_PATHS.iter().enumerate() {
        let language = ACCEPT_LANGUAGES[(at + round) % ACCEPT_LANGUAGES.len()];
        ask(format!("{base}{path}"), Some(language));
    }
    for query in CARD_QUERIES {
        ask(format!("{base}/cards/suggest?q={query}"), None);
    }
    (asked, answered)
}

/// Every hashed asset with each encoding, then once more with its `ETag`.
fn assets(agent: &ureq::Agent, base: &str, site_root: &Path) {
    for asset in asset_paths(site_root) {
        let url = format!("{base}{asset}");
        for encoding in ENCODINGS {
            let etag =
                fetch(agent, &url, &[("Accept-Encoding", encoding)]).and_then(|(_, tag)| tag);
            if let Some(etag) = etag {
                let _ = fetch(
                    agent,
                    &url,
                    &[
                        ("Accept-Encoding", encoding),
                        ("If-None-Match", etag.as_str()),
                    ],
                );
            }
        }
    }
}

/// `GET url` with `headers`, its body read to the end: the status and the
/// `ETag`, or `None` when nothing answered.
fn fetch(
    agent: &ureq::Agent,
    url: &str,
    headers: &[(&str, &str)],
) -> Option<(u16, Option<String>)> {
    let mut request = agent.get(url);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let mut response = request.call().ok()?;
    let status = response.status().as_u16();
    let etag = response
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    response
        .body_mut()
        .with_config()
        .limit(BODY_LIMIT)
        .read_to_vec()
        .ok()?;
    Some((status, etag))
}

fn expect_status(agent: &ureq::Agent, url: &str, expected: u16) -> Result<()> {
    match fetch(agent, url, &[("Accept-Encoding", "br, gzip")]) {
        Some((status, _)) if status == expected => Ok(()),
        _ => Err(Fail::new(format!(
            "central-web did not answer a training page with {expected}"
        ))),
    }
}

/// The hashed assets `manifest.json` names.
fn asset_paths(site_root: &Path) -> Vec<String> {
    let Ok(bytes) = fs::read(site_root.join("manifest.json")) else {
        return Vec::new();
    };
    let Ok(val) = serde_json::from_slice::<Value>(&bytes) else {
        return Vec::new();
    };
    let mut paths = BTreeSet::new();
    collect_asset_strings(&val, &mut paths);
    paths.into_iter().collect()
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

fn pictures(root: &Path, bin_dir: &Path, profile_file: &str) -> Result<()> {
    let worker = bin_path(bin_dir, "central-picture-worker");
    if !worker.is_file() {
        return Err(Fail::new(
            "Instrumented central-picture-worker binary missing",
        ));
    }
    process::capture(
        Command::new(&worker)
            .arg("--probe")
            .current_dir(root)
            .env("LLVM_PROFILE_FILE", profile_file),
        "Failed invoking central-picture-worker --probe",
    )?;
    let corpus = corpus(root)?;
    let mut drawn_avif = None;
    for picture in &corpus {
        for _ in 0..PICTURE_RUNS {
            let answer = convert(&worker, picture.format, &picture.bytes, Some(profile_file))?;
            if picture.must_convert && !converted(&answer) {
                return Err(Fail::new(format!(
                    "central-picture-worker refused the training picture {}",
                    picture.name
                )));
            }
            if picture.name == "photo.jpg" {
                drawn_avif = first_part(&answer).map(<[u8]>::to_vec);
            }
        }
    }
    // the worker's own AVIF, as a browser that has one uploads it
    let avif = drawn_avif.or_fail("central-picture-worker gave no AVIF for the photo")?;
    for _ in 0..PICTURE_RUNS {
        let answer = convert(&worker, AVIF, &avif, Some(profile_file))?;
        if !converted(&answer) {
            return Err(Fail::new("central-picture-worker refused its own AVIF"));
        }
    }
    refusals(&worker, &corpus, profile_file)
}

/// Files the worker meets from browsers that break off or lie: a photo cut
/// short (which a lenient decoder may still read), and, refused by their
/// first bytes, a PNG said to be a JPEG and bytes that are no picture.
fn refusals(worker: &Path, corpus: &[Picture], profile_file: &str) -> Result<()> {
    let photo = corpus
        .iter()
        .find(|p| p.name == "photo.jpg")
        .or_fail("The training has no photo")?;
    let screen = corpus
        .iter()
        .find(|p| p.name == "screen.png")
        .or_fail("The training has no screenshot")?;
    let cut = &photo.bytes[..photo.bytes.len() * 2 / 3];
    convert(worker, JPEG, cut, Some(profile_file))?;
    for (format, bytes) in [(JPEG, &screen.bytes[..]), (PNG, &b"not a picture"[..])] {
        let answer = convert(worker, format, bytes, Some(profile_file))?;
        if converted(&answer) {
            return Err(Fail::new(
                "central-picture-worker converted a file it must refuse",
            ));
        }
    }
    Ok(())
}

/// The training pictures: [`DRAWN`] by ffmpeg, an ICO around the drawn
/// square, the SVG, and the codec tests' fixtures.
pub(super) fn corpus(root: &Path) -> Result<Vec<Picture>> {
    let ffmpeg = PathBuf::from("ffmpeg");
    let found = Command::new(&ffmpeg)
        .arg("-version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    if !found {
        return Err(Fail::new(
            "PGO training needs ffmpeg on the PATH to draw its pictures",
        ));
    }
    let dir = OwnedDir::new("central-pgo-pictures-")?;
    let mut corpus = Vec::new();
    for drawn in &DRAWN {
        let file = dir.path().join(drawn.name);
        let frames = drawn.frames.to_string();
        let source = format!("testsrc2=size={}:rate=10", drawn.size);
        let mut cmd = Command::new(&ffmpeg);
        cmd.args(["-nostdin", "-hide_banner", "-loglevel", "error", "-y"])
            .args(["-f", "lavfi", "-i", &source, "-frames:v", &frames])
            .args(drawn.encoder)
            .arg(&file);
        process::succeed(&mut cmd, "ffmpeg could not draw a PGO training picture")?;
        let bytes = fs::read(&file).or_fail("A drawn PGO training picture is unreadable")?;
        corpus.push(Picture {
            name: drawn.name.to_owned(),
            format: drawn.format,
            bytes,
            must_convert: true,
        });
    }
    dir.remove()?;
    let icon = {
        let square = corpus
            .iter()
            .find(|p| p.name == "square.png")
            .or_fail("The training has no square to make an icon of")?;
        ico(&square.bytes)?
    };
    corpus.push(Picture {
        name: "icon.ico".to_owned(),
        format: ICO,
        bytes: icon,
        must_convert: true,
    });
    corpus.push(Picture {
        name: "vector.svg".to_owned(),
        format: SVG,
        bytes: SVG_PICTURE.as_bytes().to_vec(),
        must_convert: true,
    });
    corpus.extend(fixtures(&root.join(FIXTURES))?);
    Ok(corpus)
}

/// The codec tests' pictures, by their extension.
fn fixtures(dir: &Path) -> Result<Vec<Picture>> {
    let mut found = Vec::new();
    for entry in fs::read_dir(dir).or_fail("The picture fixtures are missing")? {
        let path = entry.or_fail("A picture fixture is unreadable")?.path();
        let format = match path.extension().and_then(|e| e.to_str()) {
            Some("jpg") => JPEG,
            Some("webp") => WEBP,
            Some("avif") => AVIF,
            Some("png") => PNG,
            Some("gif") => GIF,
            _ => continue,
        };
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .or_fail("A picture fixture has no name")?
            .to_owned();
        let bytes = fs::read(&path).or_fail("A picture fixture is unreadable")?;
        found.push(Picture {
            name,
            format,
            bytes,
            must_convert: false,
        });
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    if found.is_empty() {
        return Err(Fail::new("The picture fixtures are missing"));
    }
    Ok(found)
}

/// An ICO file holding `png` (256 × 256) as its one entry.
fn ico(png: &[u8]) -> Result<Vec<u8>> {
    let size = u32::try_from(png.len()).or_fail("The icon's picture is too large")?;
    let mut out = Vec::with_capacity(22 + png.len());
    out.extend_from_slice(&[0, 0, 1, 0, 1, 0]); // reserved, type icon, one entry
    out.extend_from_slice(&[0, 0, 0, 0]); // 256 × 256, no palette, reserved
    out.extend_from_slice(&1_u16.to_le_bytes()); // planes
    out.extend_from_slice(&32_u16.to_le_bytes()); // bits per pixel
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(&22_u32.to_le_bytes()); // the picture follows the entry
    out.extend_from_slice(png);
    Ok(out)
}

/// Frames a request matching `central_picture::protocol` (`BCPQ`, version 2,
/// format, u32 BE length).
fn frame(format: u8, payload: &[u8]) -> Result<Vec<u8>> {
    let len = u32::try_from(payload.len()).or_fail("A training picture is too large")?;
    let mut out = Vec::with_capacity(10 + payload.len());
    out.extend_from_slice(b"BCPQ");
    out.push(2);
    out.push(format);
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(payload);
    Ok(out)
}

/// One worker process converting `bytes` (`--no-ffmpeg`): its answer frame.
pub(super) fn convert(
    worker: &Path,
    format: u8,
    bytes: &[u8],
    profile_file: Option<&str>,
) -> Result<Vec<u8>> {
    let request = frame(format, bytes)?;
    let mut cmd = Command::new(worker);
    cmd.arg("--no-ffmpeg").current_dir("/");
    if let Some(file) = profile_file {
        cmd.env("LLVM_PROFILE_FILE", file);
    }
    let output = process::feed(&mut cmd, &request, "central-picture-worker run failed")?;
    if !output.status.success() || !output.stdout.starts_with(b"BCPA\x02") {
        return Err(Fail::new(
            "central-picture-worker gave no answer frame in PGO training",
        ));
    }
    Ok(output.stdout)
}

/// Whether an answer frame carries pictures (byte 5: 0), not a refusal.
fn converted(answer: &[u8]) -> bool {
    answer.get(5) == Some(&0)
}

/// The answer's first part, the AVIF, behind its u32 BE length.
fn first_part(answer: &[u8]) -> Option<&[u8]> {
    let rest = answer.get(6..)?;
    let length = usize::try_from(u32::from_be_bytes(rest.get(..4)?.try_into().ok()?)).ok()?;
    rest.get(4..length.checked_add(4)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_language_and_page_is_asked_once_a_pass() {
        assert_eq!(LANGS.len(), 11);
        let mut unique = BTreeSet::new();
        assert!(SLUGS.iter().all(|slug| unique.insert(*slug)));
        assert!(SLUGS.iter().all(|slug| !slug.starts_with('/')));
        assert!(ROOT_PATHS.iter().all(|path| path.starts_with('/')));
        // nothing answers on this port: every request is asked and none answered
        let agent = http_agent();
        let port = free_loopback_port().unwrap();
        let (asked, answered) = pages(&agent, &format!("http://127.0.0.1:{port}"), 0);
        assert_eq!(
            asked,
            LANGS.len() * SLUGS.len() + ROOT_PATHS.len() + CARD_QUERIES.len()
        );
        assert_eq!(answered, 0);
    }

    #[test]
    fn frames_a_request_and_reads_an_answer() {
        let framed = frame(JPEG, b"abc").unwrap();
        assert_eq!(&framed[..6], b"BCPQ\x02\x02");
        assert_eq!(&framed[6..10], &3_u32.to_be_bytes());
        assert_eq!(&framed[10..], b"abc");
        let mut answer = b"BCPA\x02\x00".to_vec();
        answer.extend_from_slice(&4_u32.to_be_bytes());
        answer.extend_from_slice(b"avif");
        answer.extend_from_slice(&3_u32.to_be_bytes());
        answer.extend_from_slice(b"png");
        assert!(converted(&answer));
        assert_eq!(first_part(&answer), Some(&b"avif"[..]));
        assert!(!converted(b"BCPA\x02\x01\x06"));
        assert_eq!(first_part(b"BCPA\x02\x00\x00\x00"), None);
        let mut short = b"BCPA\x02\x00".to_vec();
        short.extend_from_slice(&9_u32.to_be_bytes());
        short.extend_from_slice(b"avif");
        assert_eq!(first_part(&short), None);
    }

    #[test]
    fn an_icon_holds_its_png_behind_one_entry() {
        let png = b"\x89PNG\r\n\x1a\nrest";
        let icon = ico(png).unwrap();
        assert_eq!(&icon[..6], &[0, 0, 1, 0, 1, 0]);
        assert_eq!(
            &icon[14..18],
            &u32::try_from(png.len()).unwrap().to_le_bytes()
        );
        assert_eq!(&icon[18..22], &22_u32.to_le_bytes());
        assert_eq!(&icon[22..], png);
    }

    #[test]
    fn the_fixtures_are_found_by_their_format() {
        let root = crate::workspace::root().unwrap();
        let found = fixtures(&root.join(FIXTURES)).unwrap();
        assert!(found.iter().any(|p| p.format == JPEG));
        assert!(found.iter().any(|p| p.format == WEBP));
        assert!(found.iter().any(|p| p.format == AVIF));
        assert!(found.iter().all(|p| p.name != "README.md"));
    }

    #[test]
    fn the_drawn_pictures_cover_what_a_browser_uploads() {
        for format in [JPEG, PNG, WEBP, GIF, BMP] {
            assert!(DRAWN.iter().any(|d| d.format == format), "{format}");
        }
        assert!(DRAWN.iter().any(|d| d.frames > 1));
        assert!(SVG_PICTURE.starts_with("<svg") && SVG_PICTURE.ends_with("</svg>"));
    }
}
