//! Registration, sign-in, a recovery code made after a sign-in, a reset
//! with an administrator's code and a password change, in plain Rust: what
//! the WebAssembly exports (`exports.rs`) call, and what the host's tests
//! drive against `central-crypto`'s server side. Every network call is the
//! page's.

use central_api::{Endpoint, register};
use central_client::MIN_PASSWORD_CHARS;
use central_crypto::{
    Context, CryptoError,
    chat::{DeviceSecrets, SECRETS_LEN},
    opaque::{Enrollment, Login},
    proof::{self, Proof, Work},
    vault::{MasterKey, ObjectContext, RecoveryCode, WrappedMasterKey, WrappingKey},
};
use zeroize::Zeroizing;

// The page reads a challenge's ceilings from `central-api`, this module
// and Central compute with `central-crypto`'s: one set of numbers, held
// equal here, where the two meet in the browser (and in the identity
// service's `challenge`, where they meet in Central).
const _: () = assert!(
    register::WORK_MIN_MEMORY == proof::MIN_MEMORY
        && register::WORK_MAX_MEMORY == proof::MAX_MEMORY
        && register::WORK_MAX_PASSES == proof::MAX_PASSES
        && register::WORK_MAX_PARTS == proof::MAX_PARTS
        && register::WORK_MAX_DIFFICULTY == proof::MAX_DIFFICULTY
        && register::NONCE_LEN == proof::NONCE_LEN
);

/// The plaintext sealed as a new account's document (object 1), byte for
/// byte as `docs/web-accounts.md` gives it: Baylee's `AccountData::read`
/// accepts it, so the account lists the official gateway.
pub const DOCUMENT: &[u8] = br#"{"version":1,"gateways":["https://baylee.acevik.de"],"preferences":{},"legacy":null,"chat":{"devices":[],"installations":[]}}"#;

/// The Central every step is bound to: the live one, compiled in.
#[must_use]
pub fn endpoint() -> Endpoint {
    Endpoint::live()
}

fn context(account: &[u8], epoch: u64) -> Result<([u8; 16], Context), CryptoError> {
    let account: [u8; 16] = account.try_into().map_err(|_| CryptoError::Invalid)?;
    Ok((account, Context::new(endpoint().issuer(), account, epoch)?))
}

/// A registration between its two messages: the OPAQUE state, which holds
/// the password, and the account it is for. Dropping it wipes both.
pub struct Registering {
    enrollment: Enrollment,
    account: [u8; 16],
    context: Context,
}

/// What a finished registration sends, and the recovery code to show once.
pub struct Registered {
    /// The OPAQUE registration upload.
    pub record: Vec<u8>,
    /// The master key under the password's wrapping key (73 bytes).
    pub wrapped_master: WrappedMasterKey,
    /// The recovery code's written form.
    pub recovery_code: Zeroizing<String>,
    /// The master key under the recovery code (73 bytes).
    pub recovery: WrappedMasterKey,
    /// [`DOCUMENT`] sealed as object 1, revision 1.
    pub document: Vec<u8>,
}

/// Start registering `account` at `epoch` (from `Register`'s `Enrollment`).
/// Returns the state and the OPAQUE registration request.
///
/// # Errors
///
/// Rejects an empty or over-long password, an account that is not a `UUIDv7`
/// and epoch zero.
pub fn enrollment_start(
    password: String,
    account: &[u8],
    epoch: u64,
) -> Result<(Registering, Vec<u8>), CryptoError> {
    let (account, context) = context(account, epoch)?;
    let (enrollment, message) = Enrollment::start(password, context)?;
    Ok((
        Registering {
            enrollment,
            account,
            context,
        },
        message,
    ))
}

/// Finish with Central's registration response: a new random master key,
/// wrapped under the password and under a new recovery code, and the initial
/// document sealed under it. The state is consumed, also on failure.
///
/// # Errors
///
/// Rejects a malformed response and fails without randomness.
pub fn enrollment_finish(state: Registering, response: &[u8]) -> Result<Registered, CryptoError> {
    let Registering {
        enrollment,
        account,
        context,
    } = state;
    let result = enrollment.finish(response)?;
    let master = MasterKey::generate()?;
    let wrapped_master = result.wrapping_key.wrap(&master)?;
    let code = RecoveryCode::generate()?;
    let recovery = code.wrap(context, &master)?;
    let document = master.seal_object(ObjectContext::new(context, account, 1)?, DOCUMENT)?;
    Ok(Registered {
        record: result.upload,
        wrapped_master,
        recovery_code: code.to_text(),
        recovery,
        document,
    })
}

/// A sign-in between KE1 and KE2: the OPAQUE state, which holds the
/// password, and the account it signs in to.
pub struct SigningIn {
    login: Login,
    context: Context,
}

/// A sign-in after KE3 was made: the password's wrapping key, kept only to
/// open the master key Central hands back, and the account it signed in to.
pub struct SignedIn {
    key: WrappingKey,
    context: Context,
}

/// A recovery code made after a sign-in: the code to show once, and the
/// master key wrapped under it for `SetRecovery`.
pub struct Recovery {
    /// The recovery code's written form.
    pub recovery_code: Zeroizing<String>,
    /// The master key under the recovery code (73 bytes).
    pub recovery: WrappedMasterKey,
}

/// Start signing in to `account` at `epoch` (from `Discover`). Returns the
/// state and KE1.
///
/// # Errors
///
/// As [`enrollment_start`].
pub fn login_start(
    password: String,
    account: &[u8],
    epoch: u64,
) -> Result<(SigningIn, Vec<u8>), CryptoError> {
    let (_, context) = context(account, epoch)?;
    let (login, message) = Login::start(password, context)?;
    Ok((SigningIn { login, context }, message))
}

/// Answer Central's KE2 with KE3. A wrong password fails here, before
/// anything is sent. The state is consumed, also on failure.
///
/// # Errors
///
/// Rejects a wrong password, another account, issuer or epoch, and a
/// malformed response.
pub fn login_finish(state: SigningIn, response: &[u8]) -> Result<(SignedIn, Vec<u8>), CryptoError> {
    let SigningIn { login, context } = state;
    let result = login.finish(response)?;
    Ok((
        SignedIn {
            key: result.wrapping_key,
            context,
        },
        result.finalization,
    ))
}

/// Check that the master key Central answered `LoginFinish` with opens under
/// this password and account, then wipe it: the page keeps no key.
///
/// # Errors
///
/// Rejects a wrapper that is malformed or does not open.
pub fn login_confirm(state: SignedIn, wrapped_master: &[u8]) -> Result<(), CryptoError> {
    let SignedIn { key, .. } = state;
    let wrapped = WrappedMasterKey::from_bytes(wrapped_master)?;
    key.unwrap(&wrapped).map(drop)
}

/// In place of [`login_confirm`], for an account that has no recovery code
/// (one moved from the official gateway): open the master key Central
/// answered `LoginFinish` with, failing as `login_confirm` does, then make a
/// new recovery code and wrap the same master key under it with the
/// account's context (`docs/crypto-protocol.md` §"Recovery code"). The
/// master key is wiped when the step ends; only the code and the wrapper
/// leave it. The state is consumed, also on failure.
///
/// # Errors
///
/// Rejects a wrapper that is malformed or does not open, and fails without
/// randomness.
pub fn recovery_create(state: SignedIn, wrapped_master: &[u8]) -> Result<Recovery, CryptoError> {
    let SignedIn { key, context } = state;
    let wrapped = WrappedMasterKey::from_bytes(wrapped_master)?;
    let master = key.unwrap(&wrapped)?;
    let code = RecoveryCode::generate()?;
    let recovery = code.wrap(context, &master)?;
    Ok(Recovery {
        recovery_code: code.to_text(),
        recovery,
    })
}

/// A reset between its two messages (`docs/account-reset.md` §"Using it"):
/// the registration of the new password, which holds it, under the epoch the
/// reset leaves the account at, and for a kept vault the recovery code and
/// the epoch its wrapper is bound to. Dropping it wipes them.
pub struct Resetting {
    registering: Registering,
    /// The recovery code and the account's context as discovered, for a
    /// reset that keeps the vault; nothing for one that makes a new vault.
    kept: Option<(RecoveryCode, Context)>,
}

/// What a reset that keeps the vault sends with `ResetVault::Keep`.
pub struct Kept {
    /// The OPAQUE registration upload for the new password.
    pub record: Vec<u8>,
    /// The same master key under the new password's wrapping key (73 bytes).
    pub wrapped_master: WrappedMasterKey,
}

/// Start a reset of `account`, discovered at `epoch`, with the new
/// `password`, as `central-client`'s `reset` does: with a `recovery_code`
/// the vault is kept and the password registered at `epoch`; without one a
/// new vault is made and the password registered at `epoch + 1`. Returns the
/// state and the OPAQUE registration request for `ResetStart`. The recovery
/// code is read here, before the reset code is sent anywhere.
///
/// # Errors
///
/// Rejects a password shorter than [`MIN_PASSWORD_CHARS`] or over-long, an
/// account that is not a `UUIDv7`, epoch zero, an epoch with no next one and
/// a recovery code that does not read as one.
pub fn reset_start(
    password: String,
    account: &[u8],
    epoch: u64,
    recovery_code: Option<String>,
) -> Result<(Resetting, Vec<u8>), CryptoError> {
    let mut password = Zeroizing::new(password);
    let recovery_code = recovery_code.map(Zeroizing::new);
    if password.chars().count() < MIN_PASSWORD_CHARS {
        return Err(CryptoError::Invalid);
    }
    let (account, discovered) = context(account, epoch)?;
    let (kept, target) = if let Some(code) = recovery_code {
        (Some((RecoveryCode::parse(&code)?, discovered)), discovered)
    } else {
        let next = epoch.checked_add(1).ok_or(CryptoError::Invalid)?;
        (None, Context::new(endpoint().issuer(), account, next)?)
    };
    let (enrollment, message) = Enrollment::start(std::mem::take(&mut *password), target)?;
    Ok((
        Resetting {
            registering: Registering {
                enrollment,
                account,
                context: target,
            },
            kept,
        },
        message,
    ))
}

/// Finish a reset that keeps the vault with `ResetChallenge`'s registration
/// response and recovery wrapper: the recovery code given at the start opens
/// the wrapper under the account's context, and the same master key is
/// wrapped under the new password. A code that does not open it fails here,
/// before `ResetFinish`, so the reset code is not spent. The master key is
/// wiped when the step ends. The state is consumed, also on failure.
///
/// # Errors
///
/// Rejects a state started without a recovery code, a wrapper that is
/// malformed or that the code does not open, and a malformed response;
/// fails without randomness.
pub fn reset_keep(state: Resetting, response: &[u8], recovery: &[u8]) -> Result<Kept, CryptoError> {
    let Resetting { registering, kept } = state;
    let (code, discovered) = kept.ok_or(CryptoError::Invalid)?;
    let wrapper = WrappedMasterKey::from_bytes(recovery)?;
    // the code first: a wrong one fails before the password is stretched
    let master = code.unwrap(discovered, &wrapper)?;
    let result = registering.enrollment.finish(response)?;
    let wrapped_master = result.wrapping_key.wrap(&master)?;
    Ok(Kept {
        record: result.upload,
        wrapped_master,
    })
}

/// Finish a reset that makes a new vault with `ResetChallenge`'s
/// registration response: what [`enrollment_finish`] makes for a new
/// account, at the next epoch. That is a new master key under the new
/// password and under a new recovery code, and [`DOCUMENT`] sealed as
/// object 1, for `ResetVault::New`. The state is consumed, also on failure.
///
/// # Errors
///
/// Rejects a state started with a recovery code and a malformed response;
/// fails without randomness.
pub fn reset_new(state: Resetting, response: &[u8]) -> Result<Registered, CryptoError> {
    let Resetting { registering, kept } = state;
    if kept.is_some() {
        return Err(CryptoError::Invalid);
    }
    enrollment_finish(registering, response)
}

/// A password change between its two messages (`docs/native-login.md`
/// §"Password change"): the OPAQUE login that proves the current password,
/// the registration of the new one, and the master key the sign-in in this
/// Worker opened. Dropping it wipes all three.
pub struct Changing {
    login: Login,
    enrollment: Enrollment,
    master: MasterKey,
}

/// What a password change sends with `ChangePasswordFinish`.
pub struct Changed {
    /// KE3 under the current password.
    pub proof: Vec<u8>,
    /// The OPAQUE registration upload for the new password.
    pub record: Vec<u8>,
    /// The same master key under the new password's wrapping key (73 bytes).
    pub wrapped_master: WrappedMasterKey,
}

/// Start a password change after a sign-in in this Worker, as
/// `central-client`'s `Session::change_password` does with the master key
/// its session holds: open the master key Central answered `LoginFinish`
/// with, then a fresh OPAQUE login with `current` and a registration of
/// `new`, both for the account the sign-in was for. Returns the state, KE1
/// and the registration request for `ChangePasswordStart`. The sign-in's
/// state is consumed, also on failure.
///
/// # Errors
///
/// Rejects a new password shorter than [`MIN_PASSWORD_CHARS`] or over-long,
/// an empty or over-long current one, and a wrapper that is malformed or
/// does not open.
pub fn password_start(
    state: SignedIn,
    wrapped_master: &[u8],
    current: String,
    new: String,
) -> Result<(Changing, Vec<u8>, Vec<u8>), CryptoError> {
    let (mut current, mut new) = (Zeroizing::new(current), Zeroizing::new(new));
    if new.chars().count() < MIN_PASSWORD_CHARS {
        return Err(CryptoError::Invalid);
    }
    let SignedIn { key, context } = state;
    let master = key.unwrap(&WrappedMasterKey::from_bytes(wrapped_master)?)?;
    let (login, login_message) = Login::start(std::mem::take(&mut *current), context)?;
    let (enrollment, registration) = Enrollment::start(std::mem::take(&mut *new), context)?;
    Ok((
        Changing {
            login,
            enrollment,
            master,
        },
        login_message,
        registration,
    ))
}

/// Finish a password change with `PasswordChallenge`'s KE2 and registration
/// response: the proof of the current password (a wrong one fails here,
/// before anything is sent), the new record, and the same master key wrapped
/// under the new password. The master key is wiped when the step ends. The
/// state is consumed, also on failure.
///
/// # Errors
///
/// Rejects a wrong current password, a malformed or foreign challenge and a
/// malformed registration response; fails without randomness.
pub fn password_finish(
    state: Changing,
    challenge: &[u8],
    registration: &[u8],
) -> Result<Changed, CryptoError> {
    let Changing {
        login,
        enrollment,
        master,
    } = state;
    let proof = login.finish(challenge)?;
    let enrolled = enrollment.finish(registration)?;
    let wrapped_master = enrolled.wrapping_key.wrap(&master)?;
    Ok(Changed {
        proof: proof.finalization,
        record: enrolled.upload,
        wrapped_master,
    })
}

/// A chat identity opened in the browser, and its secret bytes, which the
/// Worker seals under the browser's own non-extractable key so that the chat
/// opens again after a reload without the password (`docs/central-chat.md`).
pub struct OpenChat {
    /// The chat bound to its public tag.
    pub chat: central_client::chat::Chat,
    /// The identity's secrets (signing seed, agreement seed, vault key); wiped on drop.
    pub kept: Zeroizing<[u8; SECRETS_LEN]>,
}

/// Open the account's stable chat identity after OPAQUE, consuming the wrapping
/// key and wiping the master key before returning. The identity's bytes stay
/// in the Worker.
///
/// # Errors
/// Rejects a wrong master wrapper, account context or public tag.
pub fn chat_unlock(
    state: SignedIn,
    wrapped_master: &[u8],
    tag: u32,
) -> Result<OpenChat, CryptoError> {
    let SignedIn { key, context } = state;
    let master = key.unwrap(&WrappedMasterKey::from_bytes(wrapped_master)?)?;
    open_chat(master.chat_identity(context)?, tag)
}

/// The chat identity again from the bytes [`chat_unlock`] kept, after the
/// Worker opened them under the browser's key, which binds them to the
/// account, epoch and tag.
///
/// # Errors
/// Rejects bytes of another length and an invalid public tag.
pub fn chat_resume(kept: &[u8], tag: u32) -> Result<OpenChat, CryptoError> {
    let bytes: &[u8; SECRETS_LEN] = kept.try_into().map_err(|_| CryptoError::Invalid)?;
    open_chat(DeviceSecrets::from_bytes(bytes), tag)
}

fn open_chat(secrets: DeviceSecrets, tag: u32) -> Result<OpenChat, CryptoError> {
    let kept = secrets.to_bytes();
    let chat = central_client::chat::Chat::new(secrets, tag).map_err(|_| CryptoError::Invalid)?;
    Ok(OpenChat { chat, kept })
}

/// Registration's memory-hard proof of work for the challenge `nonce` at
/// the price `Response::RegisterWork` named
/// (`docs/registration-abuse.md` §"A proof that costs memory"): the memory
/// its evaluations work in, allocated once. The search is
/// [`Proof::search`], the function Central checks the answer with.
///
/// # Errors
/// Rejects a nonce of another shape and a price above the ceilings, before
/// any memory is taken.
pub fn work_start(
    nonce: &str,
    memory: u32,
    passes: u32,
    parts: u8,
    difficulty: u8,
) -> Result<Proof, CryptoError> {
    Proof::new(
        nonce,
        Work {
            memory,
            passes,
            parts,
            difficulty,
        },
    )
}
