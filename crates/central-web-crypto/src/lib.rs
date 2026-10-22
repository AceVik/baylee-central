//! Registration and sign-in crypto for baylee.app, built to WebAssembly
//! (`docs/web-accounts.md` §"Browser crypto"), and the recovery code made
//! after a sign-in for an account that has none. OPAQUE, the master key, its
//! two wrappers and the initial account document come from `central-crypto`,
//! the live issuer from `central-api`'s `Endpoint`. It makes no network call;
//! the page carries every message. Built with `wasm-bindgen`, whose glue is
//! committed under `web/src/central-wasm/` (`web/README.md`).
#![forbid(unsafe_code)]

mod exports;
pub mod flow;

pub use exports::{
    Enrolled, Enrollment, Login, LoginKey, Recovered, central_origin, enrollment_finish,
    enrollment_start, login_confirm, login_finish, login_start, recovery_create,
};
