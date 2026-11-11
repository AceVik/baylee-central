//! The interactive parts of the pages, hydrated in the browser. Each island
//! renders on the server like any component, so its page is complete
//! without script; in the browser the WebAssembly takes it over. Islands
//! live in this private module: `#[island]` generates an undocumented props
//! struct that must not become public.

mod account;
mod auth;
mod b64;
mod central;
mod chat;
mod contact;
mod crypto;
mod early;
mod field;
mod focus;
mod impressum;
pub(crate) mod net;
mod pow;
mod push;
mod session;
#[cfg(target_arch = "wasm32")]
mod trusted;

// The pages use the islands on the server; in the browser they are reached
// through their WebAssembly exports only.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use account::Account;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use auth::{Register, SignIn};
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use chat::Messenger;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use contact::ContactForm;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use impressum::Impressum;
