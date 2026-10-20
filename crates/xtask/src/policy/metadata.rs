//! What cargo says about the workspace (`cargo metadata`), and the rules on
//! its members and dependencies.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

use super::paths::inside;
use super::rules;
use crate::fail::{Fail, OrFail, Result};
use crate::process;
use crate::workspace;

const CRATES_IO: &str = "registry+https://github.com/rust-lang/crates.io-index";
/// Tool output under the root, which may hold no target (compared ignoring
/// case, as on the case-insensitive file systems of Windows and macOS).
const EXCLUDED: [&str; 4] = [".git", "target", "node_modules", "web/node_modules"];

/// `cargo metadata` for the workspace at `root`, with or without its graph.
pub(super) struct Metadata {
    pub(super) workspace_root: PathBuf,
    json: Value,
}

pub(super) fn read(root: &Path, graph: bool) -> Result<Metadata> {
    let failure = if graph {
        "Full Cargo metadata failed"
    } else {
        "Cargo metadata failed"
    };
    let mut command = Command::new(workspace::cargo());
    command
        .args([
            "metadata",
            "--all-features",
            "--locked",
            "--offline",
            "--format-version",
            "1",
        ])
        .current_dir(root);
    if !graph {
        command.arg("--no-deps");
    }
    let output = process::capture(&mut command, failure)?;
    if !output.status.success() {
        return Err(Fail::new(failure));
    }
    let json: Value = serde_json::from_slice(&output.stdout).or_fail(failure)?;
    let workspace_root = PathBuf::from(json["workspace_root"].as_str().or_fail(failure)?);
    Ok(Metadata {
        workspace_root,
        json,
    })
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key].as_str().or_fail("Cargo metadata failed")
}

fn list<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value[key].as_array().map_or(&[], Vec::as_slice)
}

/// Every member's manifest, publication, place, and target roots.
pub(super) fn check_members(meta: &Metadata) -> Result<()> {
    let root = &meta.workspace_root;
    let excluded: Vec<PathBuf> = EXCLUDED.iter().map(|dir| root.join(dir)).collect();
    for package in list(&meta.json, "packages") {
        let name = text(package, "name")?;
        let manifest_path = Path::new(text(package, "manifest_path")?);
        let manifest = fs::read_to_string(manifest_path).or_fail("Cargo metadata failed")?;
        if !rules::inherits_lints(&manifest) {
            return Err(Fail::new(format!("Missing inherited lints: {name}")));
        }
        // `publish = false` is an empty list; an absent `publish` (null) allows crates.io
        if package["publish"]
            .as_array()
            .is_none_or(|registries| !registries.is_empty())
        {
            return Err(Fail::new(format!(
                "Package publication must be disabled: {name}"
            )));
        }
        if !inside(manifest_path, root, false) {
            return Err(Fail::new(format!(
                "Workspace member outside reviewed root: {name}"
            )));
        }
        if excluded.iter().any(|dir| inside(manifest_path, dir, true)) {
            return Err(Fail::new(format!(
                "Workspace member inside excluded output directory: {name}"
            )));
        }
        for target in list(package, "targets") {
            check_target(target, root, &excluded)?;
        }
    }
    Ok(())
}

fn check_target(target: &Value, root: &Path, excluded: &[PathBuf]) -> Result<()> {
    let name = text(target, "name")?;
    let source = Path::new(text(target, "src_path")?);
    if excluded.iter().any(|dir| inside(source, dir, true)) {
        return Err(Fail::new(format!(
            "Source target inside excluded output directory: {name}"
        )));
    }
    if !inside(source, root, false) {
        return Err(Fail::new(format!(
            "Source target outside reviewed root: {name}"
        )));
    }
    let code = fs::read(source).or_fail("Cargo metadata failed")?;
    if !rules::forbids_unsafe_at_root(&String::from_utf8_lossy(&code)) {
        return Err(Fail::new(format!(
            "Missing crate-root unsafe prohibition: {name}"
        )));
    }
    Ok(())
}

/// Path dependencies stay in the workspace; registry ones are approved, with
/// approved features, and come from crates.io.
pub(super) fn check_dependencies(graph: &Metadata, approved_json: &str) -> Result<()> {
    let members: BTreeSet<&str> = list(&graph.json, "workspace_members")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    let features: BTreeMap<&str, Vec<&str>> = list(&graph.json["resolve"], "nodes")
        .iter()
        .filter_map(|node| {
            Some((
                node["id"].as_str()?,
                list(node, "features")
                    .iter()
                    .filter_map(Value::as_str)
                    .collect(),
            ))
        })
        .collect();
    let approved: Value = serde_json::from_str(approved_json)
        .or_fail("Approved package inventory could not be read")?;
    let approved_packages: BTreeSet<&str> = list(&approved, "packages")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    for package in list(&graph.json, "packages") {
        let name = text(package, "name")?;
        let id = text(package, "id")?;
        let Some(source) = package["source"].as_str() else {
            if !members.contains(id) {
                return Err(Fail::new(format!(
                    "Local dependency outside reviewed workspace: {name}"
                )));
            }
            continue;
        };
        let identity = format!("{name}@{}", text(package, "version")?);
        if !approved_packages.contains(identity.as_str()) {
            return Err(Fail::new(format!("Dependency not approved: {identity}")));
        }
        let allowed: BTreeSet<&str> = list(&approved["features"], &identity)
            .iter()
            .filter_map(Value::as_str)
            .collect();
        for feature in features.get(id).map_or(&[][..], Vec::as_slice) {
            if !allowed.contains(feature) {
                return Err(Fail::new(format!(
                    "Unapproved dependency feature: {identity}/{feature}"
                )));
            }
        }
        if source != CRATES_IO {
            return Err(Fail::new(format!(
                "Non-registry dependency forbidden: {identity}"
            )));
        }
    }
    Ok(())
}
