//! Setting a new password: with an administrator's reset code
//! (`docs/account-reset.md`), and by a signed-in account that proves its
//! current one (`docs/native-login.md` §"Password change"). A thin shell
//! over [`crate::flow`], as `exports.rs` is: states are opaque handles a
//! step consumes, the master key never leaves the module, and errors are
//! `rejected` or `unavailable`, nothing more.

use crate::exports::{Enrolled, LoginKey, refused};
use crate::flow::{self, Changed, Changing, Kept, Resetting};
use wasm_bindgen::prelude::{JsError, wasm_bindgen};

/// A reset under way. `message` is the OPAQUE registration request for
/// `ResetStart`.
#[wasm_bindgen]
pub struct Reset {
    state: Resetting,
    message: Vec<u8>,
}

#[wasm_bindgen]
impl Reset {
    /// The OPAQUE registration request.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn message(&self) -> Vec<u8> {
        self.message.clone()
    }
}

/// A reset that keeps the vault, ready: what `ResetFinish` sends with
/// `ResetVault::Keep`.
#[wasm_bindgen]
pub struct ResetKept(Kept);

#[wasm_bindgen]
impl ResetKept {
    /// The OPAQUE registration upload.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn record(&self) -> Vec<u8> {
        self.0.record.clone()
    }

    /// The same master key under the new password (73 bytes).
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn wrapped_master(&self) -> Vec<u8> {
        self.0.wrapped_master.as_bytes().to_vec()
    }
}

/// Start a reset: `Discover`'s identity, the new password, and the recovery
/// code when the vault is kept. Without a recovery code a new vault is made,
/// at the next epoch.
///
/// # Errors
///
/// `rejected` for a password shorter than eight characters or over-long, an
/// account that is not a `UUIDv7`, epoch zero and a recovery code that does
/// not read as one.
#[wasm_bindgen]
pub fn reset_start(
    password: String,
    account: &[u8],
    epoch: u64,
    recovery_code: Option<String>,
) -> Result<Reset, JsError> {
    let (state, message) =
        flow::reset_start(password, account, epoch, recovery_code).map_err(refused)?;
    Ok(Reset { state, message })
}

/// Finish a reset that keeps the vault with `ResetChallenge`'s registration
/// response and recovery wrapper. A recovery code that does not open the
/// wrapper is `rejected` here, before the reset code is spent.
///
/// # Errors
///
/// `rejected` for a reset started without a recovery code, a wrapper that
/// is malformed or does not open and a malformed response, `unavailable`
/// without randomness.
#[wasm_bindgen]
pub fn reset_keep(state: Reset, response: &[u8], recovery: &[u8]) -> Result<ResetKept, JsError> {
    flow::reset_keep(state.state, response, recovery)
        .map(ResetKept)
        .map_err(refused)
}

/// Finish a reset that makes a new vault with `ResetChallenge`'s
/// registration response: what a registration makes, at the next epoch,
/// with the new recovery code to show once. Free it once read.
///
/// # Errors
///
/// `rejected` for a reset started with a recovery code and a malformed
/// response, `unavailable` without randomness.
#[wasm_bindgen]
pub fn reset_new(state: Reset, response: &[u8]) -> Result<Enrolled, JsError> {
    flow::reset_new(state.state, response)
        .map(Enrolled)
        .map_err(refused)
}

/// A password change under way: `login` is KE1 under the current password
/// and `registration` the OPAQUE registration request for the new one, both
/// for `ChangePasswordStart`.
#[wasm_bindgen]
pub struct PasswordChange {
    state: Changing,
    login: Vec<u8>,
    registration: Vec<u8>,
}

#[wasm_bindgen]
impl PasswordChange {
    /// KE1 under the current password.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn login(&self) -> Vec<u8> {
        self.login.clone()
    }

    /// The OPAQUE registration request for the new password.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn registration(&self) -> Vec<u8> {
        self.registration.clone()
    }
}

/// A password change, ready: what `ChangePasswordFinish` sends.
#[wasm_bindgen]
pub struct PasswordChanged(Changed);

#[wasm_bindgen]
impl PasswordChanged {
    /// KE3 under the current password.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn proof(&self) -> Vec<u8> {
        self.0.proof.clone()
    }

    /// The OPAQUE registration upload for the new password.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn record(&self) -> Vec<u8> {
        self.0.record.clone()
    }

    /// The same master key under the new password (73 bytes).
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn wrapped_master(&self) -> Vec<u8> {
        self.0.wrapped_master.as_bytes().to_vec()
    }
}

/// Start a password change after a sign-in in this Worker: the sign-in's
/// state opens `BrowserSession`'s `wrapped_master`, and the change proves
/// `current` afresh and registers `new`.
///
/// # Errors
///
/// `rejected` for a new password shorter than eight characters or
/// over-long, and a wrapper that is malformed or does not open.
#[wasm_bindgen]
pub fn password_start(
    state: LoginKey,
    wrapped_master: &[u8],
    current: String,
    new: String,
) -> Result<PasswordChange, JsError> {
    let (state, login, registration) =
        flow::password_start(state.state, wrapped_master, current, new).map_err(refused)?;
    Ok(PasswordChange {
        state,
        login,
        registration,
    })
}

/// Finish a password change with `PasswordChallenge`'s KE2 and registration
/// response. A wrong current password is `rejected` here.
///
/// # Errors
///
/// `rejected` for a wrong current password and a malformed or foreign
/// challenge or response, `unavailable` without randomness.
#[wasm_bindgen]
pub fn password_finish(
    state: PasswordChange,
    challenge: &[u8],
    registration: &[u8],
) -> Result<PasswordChanged, JsError> {
    flow::password_finish(state.state, challenge, registration)
        .map(PasswordChanged)
        .map_err(refused)
}
