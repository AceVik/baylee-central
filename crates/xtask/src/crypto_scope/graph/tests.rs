use super::*;
use serde_json::json;

fn graph() -> Value {
    let rows = [
        ("central-crypto", "0.1.0", vec!["opaque-ke"], vec![]),
        (
            "opaque-ke",
            "4.0.1",
            vec!["voprf", "elliptic-curve", "curve25519-dalek"],
            vec!["argon2", "ristretto255"],
        ),
        (
            "voprf",
            "0.5.0",
            vec!["elliptic-curve", "curve25519-dalek"],
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
        (
            "curve25519-dalek",
            "4.1.3",
            vec![],
            vec!["rand_core", "zeroize"],
        ),
    ];
    json!({
        "workspace_members": ["central-crypto"],
        "packages": rows.iter().map(|(name, version, _, _)| json!({
            "id": name, "name": name, "version": version,
            "source": if *name == "central-crypto" { None } else { Some(REGISTRY) },
            "manifest_path": "/fixture/crates/central-crypto/Cargo.toml"
        })).collect::<Vec<_>>(),
        "resolve": { "nodes": rows.iter().map(|(name, _, deps, features)| json!({
            "id": name, "dependencies": deps, "features": features
        })).collect::<Vec<_>>() }
    })
}

#[test]
fn accepts_reviewed_graph() {
    assert!(verify(&graph()).is_ok());
}

#[test]
fn rejects_new_consumers_at_every_affected_boundary() {
    for target in ["opaque-ke", "voprf", "elliptic-curve", "crypto-bigint"] {
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
    for index in 0..6 {
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
    for index in 1..6 {
        let mut changed = graph();
        changed["resolve"]["nodes"][index]["features"]
            .as_array_mut()
            .unwrap()
            .push(json!("unreviewed"));
        assert!(verify(&changed).is_err());
    }
    for index in 0..4 {
        let mut changed = graph();
        changed["resolve"]["nodes"][index]["dependencies"] = json!([]);
        assert!(verify(&changed).is_err());
    }
    let mut changed = graph();
    changed["workspace_members"] = json!([]);
    assert!(verify(&changed).is_err());
    assert!(verify(&json!({})).is_err());
}
