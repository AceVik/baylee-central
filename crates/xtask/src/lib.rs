//! Development tasks for Baylee Central, one implementation for Linux, macOS
//! and Windows (`docs/xtask.md`): the gate, the code policy, the `PostgreSQL`
//! and development smoke fixtures, the secret scan, the website's
//! WebAssembly crypto build and the website's assets.
//!
//! Every task fails closed with a fixed sentence and never prints a secret it
//! handled. They replaced the PowerShell scripts under `scripts/` (removed
//! 2026-10-20) with the same checks and the same messages.
#![forbid(unsafe_code)]

pub mod cli;
pub mod fail;
pub mod owned;
pub mod policy;
pub mod random;
pub mod secrets;

mod cards_ingest;
mod chrono_scope;
mod client;
mod crypto_scope;
mod dev_site;
mod gate;
mod pgo;
mod postgres;
mod process;
mod security_audio;
mod site;
mod smoke;
mod socket;
mod wasm;
mod workspace;

pub use cli::run;
