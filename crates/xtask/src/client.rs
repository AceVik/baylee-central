//! What `gate --client` checks: the workspace crates the Baylee desktop
//! client links, read from cargo's resolved graph, and these tasks.
//!
//! Baylee pins `central-api` and `central-client` (its root `Cargo.toml`);
//! the client links them and every workspace crate they reach through normal
//! or build dependencies. Development dependencies serve only their tests.
//! The server is built for Linux x86-64 alone (owner, 29.10.2026), so these
//! are what Windows and macOS must keep compiling (docs/xtask.md).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::Value;

use crate::fail::{Fail, OrFail, Result};
use crate::policy::metadata;

/// The crates Baylee depends on.
const PINNED: [&str; 2] = ["central-api", "central-client"];
/// The tasks that run there too: the policy's own regressions (links,
/// junctions, case-insensitive paths) depend on the platform.
const TASKS: &str = "xtask";

const FAILURE: &str = "Client crates could not be determined";

/// The packages the client gate checks, by name, sorted.
pub(crate) fn packages(root: &Path) -> Result<Vec<String>> {
    let graph = metadata::read(root, true)?;
    let mut roots = PINNED.to_vec();
    roots.push(TASKS);
    reach(&graph.json, &roots)
}

/// `roots` and every workspace member they reach through dependencies other
/// than development ones, by name, sorted.
fn reach(metadata: &Value, roots: &[&str]) -> Result<Vec<String>> {
    let members: BTreeSet<&str> = metadata["workspace_members"]
        .as_array()
        .or_fail(FAILURE)?
        .iter()
        .map(|id| id.as_str().or_fail(FAILURE))
        .collect::<Result<_>>()?;
    let mut names = BTreeMap::new();
    for package in metadata["packages"].as_array().or_fail(FAILURE)? {
        let id = package["id"].as_str().or_fail(FAILURE)?;
        if members.contains(id) {
            names.insert(id, package["name"].as_str().or_fail(FAILURE)?);
        }
    }
    let mut nodes = BTreeMap::new();
    for node in metadata["resolve"]["nodes"].as_array().or_fail(FAILURE)? {
        nodes.insert(node["id"].as_str().or_fail(FAILURE)?, node);
    }
    let mut pending = Vec::new();
    for root in roots {
        let mut matching = names.iter().filter(|&(_, name)| name == root);
        let (id, _) = matching.next().or_fail(FAILURE)?;
        if matching.next().is_some() {
            return Err(Fail::new(FAILURE));
        }
        pending.push(*id);
    }
    let mut reached = BTreeSet::new();
    while let Some(id) = pending.pop() {
        if !reached.insert(id) {
            continue;
        }
        let node = nodes.get(id).or_fail(FAILURE)?;
        for dep in node["deps"].as_array().or_fail(FAILURE)? {
            let pkg = dep["pkg"].as_str().or_fail(FAILURE)?;
            if members.contains(pkg) && linked(dep)? {
                pending.push(pkg);
            }
        }
    }
    let mut packages = reached
        .into_iter()
        .map(|id| {
            names
                .get(id)
                .map(|name| (*name).to_owned())
                .or_fail(FAILURE)
        })
        .collect::<Result<Vec<_>>>()?;
    packages.sort();
    Ok(packages)
}

/// Whether a dependency edge is built into its dependent outside of tests:
/// any normal (null) or build kind. An absent kind, or one cargo may add
/// later, fails.
fn linked(dep: &Value) -> Result<bool> {
    let kinds = dep["dep_kinds"].as_array().or_fail(FAILURE)?;
    if kinds.is_empty() {
        return Err(Fail::new(FAILURE));
    }
    let mut linked = false;
    for kind in kinds {
        match kind.get("kind").or_fail(FAILURE)? {
            Value::Null => linked = true,
            Value::String(kind) if kind == "build" => linked = true,
            Value::String(kind) if kind == "dev" => {}
            _ => return Err(Fail::new(FAILURE)),
        }
    }
    Ok(linked)
}

#[cfg(test)]
mod tests;
