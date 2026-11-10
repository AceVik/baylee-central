//! baylee.app's pages and islands (`docs/proposals/frontend-rewrite.md`).
//!
//! The same crate is compiled twice: for the server, where Leptos renders
//! every page to HTML (`site`), and for the browser, where only the islands
//! (`islands`) are hydrated. Which half Leptos builds follows the target, not
//! a feature, so building the workspace with every feature never mixes them.
//! Public pages carry no island and load no WebAssembly.
#![forbid(unsafe_code)]
// Leptos types a view by its whole element tree; the messenger's
// conversation is deeper than the compiler's default of 128 allows.
#![recursion_limit = "256"]

pub mod data;
mod format;
pub mod words;

mod islands;
mod markdown;
mod rich;

#[cfg(not(target_arch = "wasm32"))]
pub mod site;
