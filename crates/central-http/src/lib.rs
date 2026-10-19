//! Local HTTP foundation, reusable as a library by integration tests and the CLI.
//!
//! Supports the optional native account protocol with encrypted-document storage.
//! Human-data intake and readiness stay disabled. The listener and database
//! provisioning remain development-only; this is not a production launch path.
#![forbid(unsafe_code)]

mod boundary;
mod native;
mod server;

pub use server::DevelopmentServer;
