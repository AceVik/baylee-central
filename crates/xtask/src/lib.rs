//! Development tasks for Baylee Central, one implementation for Linux, macOS
//! and Windows (`docs/xtask.md`): the gate, the code policy, the `PostgreSQL`
//! and development smoke fixtures, the secret scan, and the website's
//! WebAssembly build.
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

mod crypto_scope;
mod gate;
mod postgres;
mod process;
mod smoke;
mod socket;
mod wasm;
mod workspace;

pub use cli::run;
