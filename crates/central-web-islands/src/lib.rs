//! baylee.app's islands as WebAssembly: `cargo xtask site` builds this
//! crate for `wasm32-unknown-unknown` twice, in two cargo runs so the
//! features never meet, and runs wasm-bindgen on each. Without features it
//! is the public module every page with islands loads; with `admin` it is
//! the administration's, which only `/{lang}/account/admin…` loads, and
//! which carries the public islands as well, since every island of
//! `central-web` is exported wherever that crate is linked. The islands
//! themselves are `central-web`'s; linking that crate here brings in their
//! exports. `islands.js` (crates/central-web/js) loads a module, calls
//! [`hydrate`], then hydrates each `<leptos-island>`.
#![forbid(unsafe_code)]

pub use central_web as web;

/// Prepares the reactive system for the islands. Called once by the
/// bootstrap before any island is hydrated.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    leptos::mount::hydrate_islands();
}
