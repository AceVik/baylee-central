//! Links and junctions must not import unchecked sources into the tree.

use crate::fixture::Fixture;

#[test]
fn a_link_anywhere_is_refused() {
    let fixture = Fixture::new();
    fixture.link("linked");
    fixture.rejects("Linked source outside the review boundary: linked");
}

#[test]
fn the_root_output_directories_are_skipped_links_and_all() {
    for dir in ["target", "node_modules", "Node_Modules"] {
        let fixture = Fixture::new();
        fixture.link(&format!("{dir}/linked"));
        fixture.accepts();
    }
}
