//! The interactive parts of the pages, hydrated in the browser. Each island
//! renders on the server like any component, so its page is complete
//! without script; in the browser the WebAssembly takes it over. Islands
//! live in this private module: `#[island]` generates an undocumented props
//! struct that must not become public.
//!
//! The administration (`admin`, `feedback` and their island,
//! `Administration`) is in the browser's build only with the `admin`
//! feature: the administration's own module (`cargo xtask site`), which
//! only its pages load. No other island reaches into it, so the public
//! module carries none of it.

mod account;
#[cfg(any(not(target_arch = "wasm32"), feature = "admin"))]
mod admin;
mod auth;
mod b64;
mod central;
mod challenges;
mod chat;
mod clip;
mod contact;
mod crypto;
mod early;
mod email;
#[cfg(any(not(target_arch = "wasm32"), feature = "admin"))]
mod feedback;
mod field;
mod focus;
mod fragment;
mod impressum;
mod keep;
#[cfg(any(not(target_arch = "wasm32"), feature = "admin"))]
mod moderation;
pub(crate) mod net;
mod password;
mod picture;
mod pow;
mod push;
mod reset;
mod session;
#[cfg(target_arch = "wasm32")]
mod trusted;

// The pages use the islands on the server; in the browser they are reached
// through their WebAssembly exports only.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use account::Account;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use admin::Administration;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use auth::{Register, SignIn};
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use challenges::Challenges;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use chat::Messenger;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use contact::ContactForm;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use impressum::Impressum;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use reset::Reset;
