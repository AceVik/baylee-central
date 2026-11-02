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
    // Two generations of curve25519-dalek, each behind its own callers:
    // 4.1.3 for the Ristretto suite, 5.0.0 for pairing and chat
    // (docs/dependency-proposal/{hpke,chat}.md). Neither new chat curve
    // path depends on elliptic-curve or crypto-bigint.
    let [dalek, dalek_x25519] = generations(packages, "curve25519-dalek", ["4.1.3", "5.0.0"])?;
    let hpke = package(packages, "hpke", "0.15.1", Some(REGISTRY))?;
    let x25519 = package(packages, "x25519-dalek", "3.0.0", Some(REGISTRY))?;
    let ed_dalek = package(packages, "ed25519-dalek", "3.0.0", Some(REGISTRY))?;
    let ed = package(packages, "ed25519", "3.0.0", Some(REGISTRY))?;
    // JWT uses only the signature 2 trait; its AWS-LC provider has no edge
    // to RustCrypto elliptic-curve/bigint. Preserve every affected caller check.
    let [jwt_signature, signature] = generations(packages, "signature", ["2.2.0", "3.0.0"])?;
    let jwt = package(packages, "jsonwebtoken", "11.1.0", Some(REGISTRY))?;
    callers(nodes, jwt_signature, &[jwt])?;
    features(nodes, jwt_signature, &["alloc", "std"])?;
    features(nodes, jwt, &[])?;
    callers(nodes, ed_dalek, &[central])?;
    callers(nodes, ed, &[ed_dalek])?;
    callers(nodes, signature, &[ed])?;
    features(nodes, ed_dalek, &["zeroize"])?;
    features(nodes, ed, &[])?;
    features(nodes, signature, &[])?;
    callers(nodes, opaque, &[central])?;
    callers(nodes, voprf, &[opaque])?;
    callers(nodes, elliptic, &[opaque, voprf])?;
    callers(nodes, bigint, &[elliptic])?;
    callers(nodes, dalek, &[opaque, voprf])?;
    callers(nodes, hpke, &[central])?;
    callers(nodes, x25519, &[central, hpke])?;
    callers(nodes, dalek_x25519, &[ed_dalek, hpke, x25519])?;
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
    features(nodes, hpke, &["alloc", "chacha", "hkdfsha2", "x25519"])?;
    features(nodes, x25519, &["static_secrets", "zeroize"])?;
    features(nodes, dalek_x25519, &["digest", "zeroize"])?;
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

/// The ids of exactly these versions of `name`, each once, all from the
/// registry; any other version of it, or a missing one, fails.
fn generations<'a, const N: usize>(
    packages: &'a [Value],
    name: &str,
    versions: [&str; N],
) -> Result<[&'a str; N]> {
    let matching: Vec<&Value> = packages
        .iter()
        .filter(|p| p["name"].as_str() == Some(name))
        .collect();
    if matching.len() != N {
        return Err(Fail::new(FAILURE));
    }
    let mut ids = [""; N];
    for (id, version) in ids.iter_mut().zip(versions) {
        let mut found = matching
            .iter()
            .filter(|p| p["version"].as_str() == Some(version));
        let package = found.next().or_fail(FAILURE)?;
        if found.next().is_some() || package["source"].as_str() != Some(REGISTRY) {
            return Err(Fail::new(FAILURE));
        }
        *id = package["id"].as_str().or_fail(FAILURE)?;
    }
    Ok(ids)
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
