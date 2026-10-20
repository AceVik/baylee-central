//! Development tasks for Baylee Central, one implementation for Linux, macOS
//! and Windows (`docs/xtask.md`): the gate, the code policy, the `PostgreSQL`
//! and development smoke fixtures, and the secret scan.
//!
//! Every task fails closed with a fixed sentence and never prints a secret it
//! handled. The PowerShell scripts under `scripts/` state the same checks and
//! remain for Windows development; their messages are kept identical.
#![forbid(unsafe_code)]

pub mod cli;
pub mod fail;
pub mod owned;
pub mod policy;
pub mod random;
pub mod secrets;

mod gate;
mod postgres;
mod process;
mod smoke;
mod socket;
mod workspace;

pub use cli::run;
