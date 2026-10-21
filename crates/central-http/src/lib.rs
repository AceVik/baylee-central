//! Local HTTP foundation, reusable as a library by integration tests and the CLI.
//!
//! Supports the optional native account protocol with encrypted-document storage,
//! and the website's browser listener for registration and sign-in.
//! Human-data intake and readiness stay disabled. The listener and database
//! provisioning remain development-only; this is not a production launch path.
#![forbid(unsafe_code)]

mod boundary;
mod browser;
mod native;
mod server;

pub use browser::REQUEST_BYTES as BROWSER_REQUEST_BYTES;
pub use server::DevelopmentServer;
