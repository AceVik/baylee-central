//! Auditable policy primitives; no network, database or cryptographic implementation.
//!
//! Callers must obtain facts from trusted, authenticated sources. These types do
//! not authenticate supplied values or authorize an HTTP request by themselves.
#![forbid(unsafe_code)]

pub mod config;
pub mod training;
