//! The interactive parts of the pages, hydrated in the browser. Each island
//! renders on the server like any component, so its page is complete
//! without script; in the browser the WebAssembly takes it over. Islands
//! live in this private module: `#[island]` generates an undocumented props
//! struct that must not become public.

mod contact;
mod impressum;
pub(crate) mod net;
#[cfg(target_arch = "wasm32")]
mod trusted;

// The pages use the islands on the server; in the browser they are reached
// through their WebAssembly exports only.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use contact::ContactForm;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use impressum::Impressum;
