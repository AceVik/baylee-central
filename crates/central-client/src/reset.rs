//! Setting a new password with an administrator's reset code
//! (`docs/account-reset.md`). A lost answer to the finish cannot be retried,
//! since the code is spent: sign in with the new password to learn whether
//! it took.
use super::{
    Endpoint, Error, Identity, MasterKey, ObjectContext, Request, Response, Session, Transport,
    WrappedMasterKey, context, discover, password::MIN_PASSWORD_CHARS,
};
use central_api::{reset_code::ResetCode, wire::Discovery, wire::ResetVault};
use central_crypto::{opaque::Enrollment, vault::RecoveryCode};

/// What a reset does with the account's encrypted data.
#[derive(Clone, Copy)]
pub enum ResetData<'a> {
    /// Keep it: the recovery code opens the account's recovery wrapper, and
    /// the same master key is wrapped under the new password.
    Recovery(&'a RecoveryCode),
    /// Start a new vault at the next epoch with this first account document
    /// (plaintext, as for enrollment). The old documents and objects, which
    /// nobody can open any more, are deleted, and a new recovery code is made.
    New(&'a [u8]),
}

/// The registration and the master key's wrapper under the new password.
struct Registered {
    attempt: central_api::Token,
    record: Vec<u8>,
    recovery: Option<Vec<u8>>,
    wrapping_key: central_crypto::vault::WrappingKey,
}

/// Start a reset for `identity` (as discovered), registering `new` at `epoch`.
fn start(
    transport: &impl Transport,
    endpoint: &Endpoint,
    username: String,
    code: ResetCode,
    identity: Identity,
    epoch: u64,
    new: String,
) -> Result<Registered, Error> {
    if new.chars().count() < MIN_PASSWORD_CHARS {
        return Err(Error::Rejected);
    }
    let target = context(
        endpoint,
        Identity {
            account: identity.account,
            epoch,
        },
    )?;
    let (enrollment, registration) = Enrollment::start(new, target).map_err(|_| Error::Rejected)?;
    let Response::ResetChallenge {
        attempt,
        identity: answered,
        registration,
        recovery,
    } = transport.exchange(
        &Request::ResetStart {
            username,
            code,
            registration,
        },
        None,
    )?
    else {
        return Err(Error::Rejected);
    };
    if answered.account != identity.account || answered.epoch != identity.epoch {
        return Err(Error::Rejected);
    }
    let enrolled = enrollment
        .finish(&registration)
        .map_err(|_| Error::Rejected)?;
    Ok(Registered {
        attempt,
        record: enrolled.upload,
        recovery,
        wrapping_key: enrolled.wrapping_key,
    })
}

fn finish(
    transport: &impl Transport,
    registered: Registered,
    master: &MasterKey,
    vault: ResetVault,
) -> Result<(), Error> {
    let wrapped = registered
        .wrapping_key
        .wrap(master)
        .map_err(|_| Error::Unavailable)?;
    match transport.exchange(
        &Request::ResetFinish {
            attempt: registered.attempt,
            record: registered.record,
            wrapped_master: wrapped.as_bytes().to_vec(),
            vault,
        },
        None,
    )? {
        Response::Done => Ok(()),
        _ => Err(Error::Rejected),
    }
}

/// Set a new password for `username` with an administrator's reset `code`,
/// keeping the data with the recovery code or starting a new vault. Every
/// session of the account ends; sign in with the new password afterwards.
/// Returns the new recovery code of a new vault, to show the person once.
///
/// # Errors
/// `Rejected` for a new password shorter than [`MIN_PASSWORD_CHARS`], an
/// unknown account, a wrong, spent or expired code, a recovery code that does
/// not open the account's wrapper (or an account without one) and a
/// malformed answer; `Busy` once the account's login budget is spent;
/// transport failure.
pub fn reset(
    transport: &impl Transport,
    username: String,
    code: ResetCode,
    new: String,
    data: ResetData<'_>,
) -> Result<Option<RecoveryCode>, Error> {
    let Discovery::Central { identity } = discover(transport, username.clone())? else {
        return Err(Error::Rejected);
    };
    let endpoint = transport.endpoint().clone();
    match data {
        ResetData::Recovery(recovery_code) => {
            let registered = start(
                transport,
                &endpoint,
                username,
                code,
                identity,
                identity.epoch,
                new,
            )?;
            let wrapper = registered.recovery.as_deref().ok_or(Error::Rejected)?;
            let wrapper = WrappedMasterKey::from_bytes(wrapper).map_err(|_| Error::Rejected)?;
            let master = recovery_code
                .unwrap(context(&endpoint, identity)?, &wrapper)
                .map_err(|_| Error::Rejected)?;
            finish(transport, registered, &master, ResetVault::Keep)?;
            Ok(None)
        }
        ResetData::New(document) => {
            let epoch = identity.epoch.checked_add(1).ok_or(Error::Rejected)?;
            let registered = start(transport, &endpoint, username, code, identity, epoch, new)?;
            let next = context(
                &endpoint,
                Identity {
                    account: identity.account,
                    epoch,
                },
            )?;
            let master = MasterKey::generate().map_err(|_| Error::Unavailable)?;
            let object =
                ObjectContext::new(next, identity.account, 1).map_err(|_| Error::Rejected)?;
            let document = master
                .seal_object(object, document)
                .map_err(|_| Error::Rejected)?;
            let recovery_code = RecoveryCode::generate().map_err(|_| Error::Unavailable)?;
            let recovery = recovery_code
                .wrap(next, &master)
                .map_err(|_| Error::Unavailable)?;
            finish(
                transport,
                registered,
                &master,
                ResetVault::New {
                    document,
                    recovery: Some(recovery.as_bytes().to_vec()),
                },
            )?;
            Ok(Some(recovery_code))
        }
    }
}

impl Session {
    /// Set a new password with an administrator's reset `code`, keeping the
    /// data with the master key this signed-in device holds. Every session of
    /// the account ends, this one too: sign in with the new password
    /// afterwards. `username` is the account's, as for [`reset`].
    ///
    /// # Errors
    /// As [`reset`], and `Rejected` when `username` is another account's.
    pub fn redeem_reset(
        &self,
        transport: &impl Transport,
        username: String,
        code: ResetCode,
        new: String,
    ) -> Result<(), Error> {
        self.same_endpoint(transport)?;
        let registered = start(
            transport,
            &self.endpoint,
            username,
            code,
            self.identity,
            self.identity.epoch,
            new,
        )?;
        finish(transport, registered, &self.master, ResetVault::Keep)
    }
}
