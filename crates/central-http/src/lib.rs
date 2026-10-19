//! Local HTTP foundation, reusable as a library by integration tests and the CLI.
//!
//! No authentication/storage endpoints exist yet. Human-data intake and readiness
//! stay disabled. This is not a production listener or an SSO implementation.
#![forbid(unsafe_code)]

mod boundary;
mod native;
mod server;

pub use server::DevelopmentServer;
