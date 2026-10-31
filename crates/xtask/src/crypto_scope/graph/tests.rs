use super::*;
use serde_json::json;

// A package's id is its name, except for the two curve25519-dalek generations.
const DALEK: &str = "curve25519-dalek";
const DALEK_X25519: &str = "curve25519-dalek@5";

fn graph() -> Value {
    let rows = [
        ("central-crypto", "0.1.0", vec!["opaque-ke", "hpke"], vec![]),
        (
            "opaque-ke",
            "4.0.1",
            vec!["voprf", "elliptic-curve", DALEK],
            vec!["argon2", "ristretto255"],
        ),
        (
            "voprf",
            "0.5.0",
            vec!["elliptic-curve", DALEK],
            vec!["danger", "ristretto255", "ristretto255-ciphersuite"],
        ),
        (
            "elliptic-curve",
            "0.13.8",
            vec!["crypto-bigint"],
            vec![
                "arithmetic",
                "default",
                "digest",
                "ff",
                "group",
                "hash2curve",
                "sec1",
                "voprf",
            ],
        ),
        (
            "crypto-bigint",
            "0.5.5",
            vec![],
            vec!["generic-array", "rand_core", "zeroize"],
        ),
        (DALEK, "4.1.3", vec![], vec!["rand_core", "zeroize"]),
        (DALEK_X25519, "5.0.0", vec![], vec!["zeroize"]),
        (
            "hpke",
            "0.15.1",
            vec!["x25519-dalek", DALEK_X25519],
            vec!["alloc", "chacha", "hkdfsha2", "x25519"],
        ),
        (
            "x25519-dalek",
            "3.0.0",
            vec![DALEK_X25519],
            vec!["static_secrets", "zeroize"],
        ),
    ];
    json!({
        "workspace_members": ["central-crypto"],
        "packages": rows.iter().map(|(id, version, _, _)| json!({
            "id": id, "name": id.split('@').next().unwrap(), "version": version,
            "source": if *id == "central-crypto" { None } else { Some(REGISTRY) },
            "manifest_path": "/fixture/crates/central-crypto/Cargo.toml"
        })).collect::<Vec<_>>(),
        "resolve": { "nodes": rows.iter().map(|(id, _, deps, features)| json!({
            "id": id, "dependencies": deps, "features": features
        })).collect::<Vec<_>>() }
    })
}

#[test]
fn accepts_reviewed_graph() {
    assert!(verify(&graph()).is_ok());
}

#[test]
fn rejects_new_consumers_at_every_affected_boundary() {
    for target in [
        "opaque-ke",
        "voprf",
        "elliptic-curve",
        "crypto-bigint",
        DALEK,
        DALEK_X25519,
        "hpke",
        "x25519-dalek",
    ] {
        let mut changed = graph();
        changed["resolve"]["nodes"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "id": "new-consumer", "dependencies": [target], "features": []
            }));
        assert!(verify(&changed).is_err(), "{target}");
    }
}

#[test]
fn rejects_changed_missing_and_duplicate_packages_and_nodes() {
    for index in 0..9 {
        for key in ["source", "version"] {
            let mut changed = graph();
            changed["packages"][index][key] = json!("unreviewed");
            assert!(verify(&changed).is_err(), "{index}/{key}");
        }
        for duplicate in [false, true] {
            for location in ["packages", "nodes"] {
                let mut changed = graph();
                let list = if location == "packages" {
                    &mut changed["packages"]
                } else {
                    &mut changed["resolve"]["nodes"]
                }
                .as_array_mut()
                .unwrap();
                if duplicate {
                    list.push(list[index].clone());
                } else {
                    list.remove(index);
                }
                assert!(verify(&changed).is_err(), "{location}/{index}/{duplicate}");
            }
        }
    }
}

#[test]
fn rejects_feature_expansion_removed_edges_and_nonworkspace_crypto() {
    for index in 1..9 {
        let mut changed = graph();
        changed["resolve"]["nodes"][index]["features"]
            .as_array_mut()
            .unwrap()
            .push(json!("unreviewed"));
        assert!(verify(&changed).is_err());
    }
    // Every row with dependencies: central-crypto, opaque-ke, voprf,
    // elliptic-curve, hpke and x25519-dalek.
    for index in [0, 1, 2, 3, 7, 8] {
        let mut changed = graph();
        changed["resolve"]["nodes"][index]["dependencies"] = json!([]);
        assert!(verify(&changed).is_err());
    }
    let mut changed = graph();
    changed["workspace_members"] = json!([]);
    assert!(verify(&changed).is_err());
    assert!(verify(&json!({})).is_err());
}

#[test]
fn rejects_a_third_curve25519_dalek_generation() {
    let mut changed = graph();
    changed["packages"].as_array_mut().unwrap().push(json!({
        "id": "curve25519-dalek@4.1.4", "name": DALEK, "version": "4.1.4",
        "source": REGISTRY, "manifest_path": "/fixture/crates/central-crypto/Cargo.toml"
    }));
    changed["resolve"]["nodes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id": "curve25519-dalek@4.1.4", "dependencies": [], "features": []}));
    assert!(verify(&changed).is_err());
}
