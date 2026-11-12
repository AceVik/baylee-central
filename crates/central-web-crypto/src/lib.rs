//! Registration and sign-in crypto for baylee.app, built to WebAssembly
//! (`docs/web-accounts.md` §"Browser crypto"), the recovery code made
//! after a sign-in for an account that has none, a new password set with an
//! administrator's reset code (`docs/account-reset.md`) or by a signed-in
//! account that proves its current one (`docs/native-login.md` §"Password
//! change"), and registration's memory-hard proof of work. OPAQUE, the
//! master key, its two wrappers and the initial account document come from
//! `central-crypto`, the live issuer from `central-api`'s `Endpoint`. It
//! makes no network call; the page carries every message. Built with
//! `wasm-bindgen`, whose glue is committed under `crates/central-web/crypto/`
//! (`crates/central-web/crypto/README.md`).
#![forbid(unsafe_code)]

mod chat_exports;
mod exports;
pub mod flow;
mod password_exports;

pub use exports::{
    Enrolled, Enrollment, Login, LoginKey, Recovered, Work, central_origin, enrollment_finish,
    enrollment_start, login_confirm, login_finish, login_start, recovery_create, work_start,
};
pub use password_exports::{
    PasswordChange, PasswordChanged, Reset, ResetKept, password_finish, password_start, reset_keep,
    reset_new, reset_start,
};
