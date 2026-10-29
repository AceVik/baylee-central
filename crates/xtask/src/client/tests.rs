use super::*;
use serde_json::json;

fn id(name: &str) -> String {
    format!("path+file:///fixture/crates/{name}#0.1.0")
}

fn dep(name: &str, kinds: &[Option<&str>]) -> Value {
    json!({
        "pkg": id(name),
        "dep_kinds": kinds.iter().map(|kind| json!({ "kind": kind, "target": null })).collect::<Vec<_>>()
    })
}

/// A workspace shaped like this one: the client reaches the server crates
/// only as development dependencies of its tests.
fn workspace() -> Value {
    let rows = [
        ("central-api", vec![dep("serde", &[None])]),
        (
            "central-client",
            vec![
                dep("central-api", &[None]),
                dep("central-crypto", &[None]),
                dep("central-postgres", &[Some("dev")]),
            ],
        ),
        ("central-crypto", vec![dep("codegen", &[Some("build")])]),
        ("codegen", vec![]),
        (
            "central-postgres",
            vec![
                dep("central-migration", &[None]),
                dep("central-crypto", &[None]),
            ],
        ),
        ("central-migration", vec![]),
        (
            "central-web-crypto",
            vec![dep("central-crypto", &[None, Some("dev")])],
        ),
        ("xtask", vec![dep("serde", &[None])]),
        ("serde", vec![]),
    ];
    json!({
        "workspace_members": rows.iter()
            .filter(|(name, _)| *name != "serde")
            .map(|(name, _)| id(name))
            .collect::<Vec<_>>(),
        "packages": rows.iter()
            .map(|(name, _)| json!({ "id": id(name), "name": name }))
            .collect::<Vec<_>>(),
        "resolve": { "nodes": rows.iter()
            .map(|(name, deps)| json!({ "id": id(name), "deps": deps }))
            .collect::<Vec<_>>() }
    })
}

fn node<'a>(graph: &'a mut Value, name: &str) -> &'a mut Value {
    graph["resolve"]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["id"] == id(name))
        .unwrap()
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|name| (*name).to_owned()).collect()
}

#[test]
fn normal_and_build_edges_are_followed_development_ones_not() {
    assert_eq!(
        reach(&workspace(), &["central-api", "central-client"]),
        Ok(names(&[
            "central-api",
            "central-client",
            "central-crypto",
            "codegen"
        ]))
    );
}

#[test]
fn an_edge_that_is_also_normal_is_followed() {
    assert_eq!(
        reach(&workspace(), &["central-web-crypto"]),
        Ok(names(&["central-crypto", "central-web-crypto", "codegen"]))
    );
}

#[test]
fn a_root_outside_the_workspace_fails() {
    for roots in [&["central-client", "baylee"][..], &["serde"]] {
        assert_eq!(
            reach(&workspace(), roots),
            Err(Fail::new(FAILURE)),
            "{roots:?}"
        );
    }
}

#[test]
fn an_unknown_or_absent_kind_fails() {
    for kinds in [
        json!([{ "kind": "bench", "target": null }]),
        json!([{ "target": null }, { "kind": "dev", "target": null }]),
        json!([]),
        Value::Null,
    ] {
        let mut graph = workspace();
        node(&mut graph, "central-client")["deps"][0]["dep_kinds"] = kinds.clone();
        assert_eq!(
            reach(&graph, &["central-client"]),
            Err(Fail::new(FAILURE)),
            "{kinds}"
        );
    }
}

#[test]
fn a_reached_member_without_its_node_fails() {
    let mut graph = workspace();
    graph["resolve"]["nodes"]
        .as_array_mut()
        .unwrap()
        .retain(|node| node["id"] != id("central-crypto"));
    assert_eq!(reach(&graph, &["central-client"]), Err(Fail::new(FAILURE)));
}

/// The set docs/xtask.md names. A new workspace dependency of the pinned
/// crates changes it: then the docs, and Windows and macOS CI, change too.
#[test]
fn this_workspace() {
    let root = crate::workspace::root().unwrap();
    assert_eq!(
        packages(&root),
        Ok(names(&[
            "central-api",
            "central-client",
            "central-crypto",
            "xtask"
        ]))
    );
}
