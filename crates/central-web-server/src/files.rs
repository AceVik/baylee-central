//! The built assets (`cargo xtask site`), read once at start into memory
//! and served by their hashed addresses only: no request path ever becomes
//! a file system path. A hashed file is cached for a year; the few files
//! with fixed names (`/sw.js`, `/favicon.ico`, …) are revalidated.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use axum::body::Bytes;
use central_web::site::Assets;

/// One file to serve.
#[derive(Debug)]
pub struct File {
    /// Its media type.
    pub kind: &'static str,
    /// Its bytes.
    pub bytes: Bytes,
    /// The same, Brotli-compressed, when the build made it.
    pub brotli: Option<Bytes>,
    /// The same, gzip-compressed, when the build made it.
    pub gzip: Option<Bytes>,
    /// A hashed name: cached for a year.
    pub immutable: bool,
}

/// Every file by its address.
#[derive(Debug, Default)]
pub struct Files {
    by_path: HashMap<String, Arc<File>>,
}

/// The fixed-name files at the root, served as they are.
pub const ROOT: [&str; 4] = [
    "favicon.ico",
    "apple-touch-icon.png",
    "sw.js",
    "notices.txt",
];

fn kind(name: &str) -> Option<&'static str> {
    let extension = name.rsplit_once('.').map(|(_, e)| e)?;
    Some(match extension {
        "css" => "text/css; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "wasm" => "application/wasm",
        "woff2" => "font/woff2",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "jpg" => "image/jpeg",
        "png" => "image/png",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "txt" => "text/plain; charset=utf-8",
        "json" => "application/json",
        _ => return None,
    })
}

/// A problem with the build's output.
#[derive(Debug)]
pub struct Broken(pub String);

impl Files {
    /// Reads the manifest and every file it names, plus the root files.
    ///
    /// # Errors
    /// A missing manifest, a missing file or a name of an unknown type.
    pub fn load(site: &Path) -> Result<(Assets, Files), Broken> {
        let text = std::fs::read_to_string(site.join("manifest.json")).map_err(|_| {
            Broken(format!(
                "{}: no manifest.json; run cargo xtask site",
                site.display()
            ))
        })?;
        let assets: Assets =
            serde_json::from_str(&text).map_err(|_| Broken("manifest.json is malformed".into()))?;
        let mut files = Files::default();
        let mut add = |path: String, on_disk: &Path, immutable: bool| -> Result<(), Broken> {
            let name = on_disk.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let kind = kind(name).ok_or_else(|| Broken(format!("{name}: no media type")))?;
            let bytes = std::fs::read(on_disk)
                .map_err(|_| Broken(format!("{}: unreadable", on_disk.display())))?;
            let beside = |suffix: &str| {
                let mut compressed = on_disk.as_os_str().to_owned();
                compressed.push(suffix);
                std::fs::read(compressed).ok().map(Bytes::from)
            };
            files.by_path.insert(
                path,
                Arc::new(File {
                    kind,
                    brotli: beside(".br"),
                    gzip: beside(".gz"),
                    bytes: Bytes::from(bytes),
                    immutable,
                }),
            );
            Ok(())
        };
        for asset in assets.files.values() {
            let relative = asset
                .url
                .strip_prefix('/')
                .filter(|r| r.starts_with("assets/") && !r.contains(".."))
                .ok_or_else(|| Broken(format!("{}: not under /assets/", asset.url)))?;
            add(asset.url.clone(), &site.join(relative), true)?;
        }
        for name in ROOT {
            add(format!("/{name}"), &site.join(name), false)?;
        }
        Ok((assets, files))
    }

    /// The file at `path`.
    #[must_use]
    pub fn get(&self, path: &str) -> Option<Arc<File>> {
        self.by_path.get(path).cloned()
    }
}
