//! baylee.app's islands as one WebAssembly module: `cargo xtask site`
//! builds this crate for `wasm32-unknown-unknown` and runs wasm-bindgen on
//! it. The islands themselves are `central-web`'s; linking that crate here
//! brings in their exports. `islands.js` (crates/central-web/js) loads the
//! module, calls [`hydrate`], then hydrates each `<leptos-island>`.
#![forbid(unsafe_code)]

pub use central_web as web;

/// Prepares the reactive system for the islands. Called once by the
/// bootstrap before any island is hydrated.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    leptos::mount::hydrate_islands();
}
