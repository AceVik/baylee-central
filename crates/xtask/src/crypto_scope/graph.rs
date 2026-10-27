//! Exact sources, features and reverse edges, including inactive consumers.

use std::collections::BTreeSet;

use serde_json::Value;

use crate::fail::{Fail, OrFail, Result};

const REGISTRY: &str = "registry+https://github.com/rust-lang/crates.io-index";
const FAILURE: &str = "Crypto dependency scope changed; review RUSTSEC-2026-0387 again";

pub(super) fn verify(metadata: &Value) -> Result<&str> {
    let packages = metadata["packages"].as_array().or_fail(FAILURE)?;
    let nodes = metadata["resolve"]["nodes"].as_array().or_fail(FAILURE)?;
    let central = package(packages, "central-crypto", "0.1.0", None)?;
    node(nodes, central)?;
    let members = metadata["workspace_members"].as_array().or_fail(FAILURE)?;
    if !members
        .iter()
        .any(|member| member.as_str() == Some(central))
    {
        return Err(Fail::new(FAILURE));
    }
    let opaque = package(packages, "opaque-ke", "4.0.1", Some(REGISTRY))?;
    let voprf = package(packages, "voprf", "0.5.0", Some(REGISTRY))?;
    let elliptic = package(packages, "elliptic-curve", "0.13.8", Some(REGISTRY))?;
    let bigint = package(packages, "crypto-bigint", "0.5.5", Some(REGISTRY))?;
    let dalek = package(packages, "curve25519-dalek", "4.1.3", Some(REGISTRY))?;
    callers(nodes, opaque, &[central])?;
    callers(nodes, voprf, &[opaque])?;
    callers(nodes, elliptic, &[opaque, voprf])?;
    callers(nodes, bigint, &[elliptic])?;
    features(nodes, opaque, &["argon2", "ristretto255"])?;
    features(
        nodes,
        voprf,
        &["danger", "ristretto255", "ristretto255-ciphersuite"],
    )?;
    features(
        nodes,
        elliptic,
        &[
            "arithmetic",
            "default",
            "digest",
            "ff",
            "group",
            "hash2curve",
            "sec1",
            "voprf",
        ],
    )?;
    features(nodes, bigint, &["generic-array", "rand_core", "zeroize"])?;
    features(nodes, dalek, &["rand_core", "zeroize"])?;
    packages
        .iter()
        .find(|p| p["id"].as_str() == Some(central))
        .or_fail(FAILURE)?["manifest_path"]
        .as_str()
        .or_fail(FAILURE)
}

fn package<'a>(
    packages: &'a [Value],
    name: &str,
    version: &str,
    source: Option<&str>,
) -> Result<&'a str> {
    let mut matching = packages.iter().filter(|p| p["name"].as_str() == Some(name));
    let package = matching.next().or_fail(FAILURE)?;
    if matching.next().is_some()
        || package["version"].as_str() != Some(version)
        || package["source"].as_str() != source
    {
        return Err(Fail::new(FAILURE));
    }
    package["id"].as_str().or_fail(FAILURE)
}

fn node<'a>(nodes: &'a [Value], id: &str) -> Result<&'a Value> {
    let mut matching = nodes.iter().filter(|node| node["id"].as_str() == Some(id));
    let node = matching.next().or_fail(FAILURE)?;
    if matching.next().is_some() {
        return Err(Fail::new(FAILURE));
    }
    Ok(node)
}

fn strings(values: &Value) -> Result<BTreeSet<&str>> {
    values
        .as_array()
        .or_fail(FAILURE)?
        .iter()
        .map(|value| value.as_str().or_fail(FAILURE))
        .collect()
}

fn callers(nodes: &[Value], target: &str, expected: &[&str]) -> Result<()> {
    node(nodes, target)?;
    let mut actual = BTreeSet::new();
    for node in nodes {
        let id = node["id"].as_str().or_fail(FAILURE)?;
        if strings(&node["dependencies"])?.contains(target) {
            actual.insert(id);
        }
    }
    if actual != expected.iter().copied().collect() {
        return Err(Fail::new(FAILURE));
    }
    Ok(())
}

fn features(nodes: &[Value], id: &str, expected: &[&str]) -> Result<()> {
    if strings(&node(nodes, id)?["features"])? != expected.iter().copied().collect() {
        return Err(Fail::new(FAILURE));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
