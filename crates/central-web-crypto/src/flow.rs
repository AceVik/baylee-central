//! Registration, sign-in and a recovery code made after a sign-in, in plain
//! Rust: what the WebAssembly exports (`exports.rs`) call, and what the
//! host's tests drive against `central-crypto`'s server side. Every network
//! call is the page's.

use central_api::{Endpoint, register};
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
