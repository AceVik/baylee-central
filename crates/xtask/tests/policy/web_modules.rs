//! npm's install of the web app is skipped, links and all, but only there.

use crate::fixture::Fixture;

#[test]
fn links_inside_web_node_modules_pass() {
    let fixture = Fixture::new();
    fixture.link("web/node_modules/.bin/tool");
    fixture.accepts();
}

#[test]
fn a_link_elsewhere_under_web_is_refused() {
    let fixture = Fixture::new();
    fixture.link("web/node_modules/.bin/tool");
    fixture.link("web/linked");
    fixture.rejects("Linked source outside the review boundary: linked");
}

#[test]
fn a_deeper_node_modules_is_walked() {
    let fixture = Fixture::new();
    fixture.link("web/tools/node_modules/linked");
    fixture.rejects("Linked source outside the review boundary: linked");
}

#[test]
fn only_the_exact_name_is_skipped() {
    let fixture = Fixture::new();
    fixture.link("web/Node_Modules/linked");
    fixture.rejects("Linked source outside the review boundary: linked");
}

#[test]
fn a_link_named_web_node_modules_is_refused() {
    let fixture = Fixture::new();
    fixture.link("web/node_modules");
    fixture.rejects("Linked source outside the review boundary: node_modules");
}

#[test]
fn a_target_cannot_hide_in_web_node_modules() {
    let fixture = Fixture::new();
    fixture.write("web/node_modules/fixture.rs", "#![forbid(unsafe_code)]");
    fixture.manifest("[lib]\npath = \"../../web/node_modules/fixture.rs\"\n");
    fixture.rejects("Source target inside excluded output directory: policy_fixture");
}
