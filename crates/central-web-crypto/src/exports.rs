//! The module's exports: a thin shell over [`crate::flow`]. A state is an
//! opaque handle the page cannot read; a step that takes one consumes it
//! (its JavaScript object is emptied) and the state is wiped when dropped,
//! also when the page frees it unused. Errors are `rejected` or
//! `unavailable`, nothing more.
//!
//! Only the module calls these: the conversions they use exist in
//! WebAssembly alone, so the host's tests drive `flow` instead.

use crate::flow::{self, Registered, Registering, SignedIn, SigningIn};
use central_crypto::CryptoError;
use wasm_bindgen::prelude::{JsError, JsValue, wasm_bindgen};

fn refused(error: CryptoError) -> JsError {
    JsError::new(match error {
        CryptoError::Invalid => "rejected",
        CryptoError::RandomnessUnavailable => "unavailable",
    })
}

/// A registration under way. `message` is the OPAQUE registration request
/// for `EnrollmentStart`.
#[wasm_bindgen]
pub struct Enrollment {
    state: Registering,
    message: Vec<u8>,
}

#[wasm_bindgen]
impl Enrollment {
    /// The OPAQUE registration request.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn message(&self) -> Vec<u8> {
        self.message.clone()
    }
}

/// A finished registration: what `EnrollmentFinish` sends, and the
/// recovery code to show once. Free it once read; that wipes the code.
#[wasm_bindgen]
pub struct Enrolled(Registered);

#[wasm_bindgen]
impl Enrolled {
    /// The OPAQUE registration upload.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn record(&self) -> Vec<u8> {
        self.0.record.clone()
    }

    /// The master key under the password (73 bytes).
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn wrapped_master(&self) -> Vec<u8> {
        self.0.wrapped_master.as_bytes().to_vec()
    }

    /// The recovery code's written form. Read from the wiped buffer rather
    /// than handed over, so no copy stays behind in the module's memory.
    #[wasm_bindgen(getter, unchecked_return_type = "string")]
    #[must_use]
    pub fn recovery_code(&self) -> JsValue {
        JsValue::from_str(&self.0.recovery_code)
    }

    /// The master key under the recovery code (73 bytes).
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn recovery(&self) -> Vec<u8> {
        self.0.recovery.as_bytes().to_vec()
    }

    /// The initial account document, sealed.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn document(&self) -> Vec<u8> {
        self.0.document.clone()
    }
}

/// A sign-in under way. `message` is KE1 for `LoginStart`.
#[wasm_bindgen]
pub struct Login {
    state: SigningIn,
    message: Vec<u8>,
}

#[wasm_bindgen]
impl Login {
    /// KE1.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn message(&self) -> Vec<u8> {
        self.message.clone()
    }
}

/// A sign-in whose KE3 is ready: `message` goes to `LoginFinish`, and the
/// handle checks the master key that answer brings.
#[wasm_bindgen]
pub struct LoginKey {
    state: SignedIn,
    message: Vec<u8>,
}

#[wasm_bindgen]
impl LoginKey {
    /// KE3.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn message(&self) -> Vec<u8> {
        self.message.clone()
    }
}

/// The origin of the Central this module is bound to: the live one.
#[wasm_bindgen]
#[must_use]
pub fn central_origin() -> String {
    flow::endpoint().origin().to_owned()
}

/// Start registering: the `identity` of `Register`'s answer and the password.
///
/// # Errors
///
/// `rejected` for an empty or over-long password, an account that is not a
/// `UUIDv7` or epoch zero.
#[wasm_bindgen]
pub fn enrollment_start(
    password: String,
    account: &[u8],
    epoch: u64,
) -> Result<Enrollment, JsError> {
    let (state, message) = flow::enrollment_start(password, account, epoch).map_err(refused)?;
    Ok(Enrollment { state, message })
}

/// Finish registering with `EnrollmentChallenge`'s message.
///
/// # Errors
///
/// `rejected` for a malformed response, `unavailable` without randomness.
#[wasm_bindgen]
pub fn enrollment_finish(state: Enrollment, response: &[u8]) -> Result<Enrolled, JsError> {
    flow::enrollment_finish(state.state, response)
        .map(Enrolled)
        .map_err(refused)
}

/// Start signing in: `Discover`'s identity and the password.
///
/// # Errors
///
/// As [`enrollment_start`].
#[wasm_bindgen]
pub fn login_start(password: String, account: &[u8], epoch: u64) -> Result<Login, JsError> {
    let (state, message) = flow::login_start(password, account, epoch).map_err(refused)?;
    Ok(Login { state, message })
}

/// Answer `LoginChallenge`'s KE2. A wrong password is `rejected` here.
///
/// # Errors
///
/// `rejected` for a wrong password or a malformed or foreign response.
#[wasm_bindgen]
pub fn login_finish(state: Login, response: &[u8]) -> Result<LoginKey, JsError> {
    let (state, message) = flow::login_finish(state.state, response).map_err(refused)?;
    Ok(LoginKey { state, message })
}

/// Check that `BrowserSession`'s `wrapped_master` opens, and keep nothing.
///
/// # Errors
///
/// `rejected` when it is malformed or does not open.
#[wasm_bindgen]
pub fn login_confirm(state: LoginKey, wrapped_master: &[u8]) -> Result<(), JsError> {
    flow::login_confirm(state.state, wrapped_master).map_err(refused)
}
