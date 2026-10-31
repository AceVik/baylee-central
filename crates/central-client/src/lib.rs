//! Native account SDK with injected transport. No database, server keys or UI dependency.
#![forbid(unsafe_code)]

use central_api::{
    Endpoint, Error, Token,
    wire::{Discovery, Identity, Request, Response},
};
use central_crypto::{
    Context,
    opaque::{Enrollment, Login},
    vault::{MasterKey, ObjectContext, WrappedMasterKey},
};
use zeroize::Zeroizing;

mod deletion;
mod export;
mod gateways;
mod legacy;
mod objects;
mod password;
mod reset;
mod session;
mod terms;
pub use deletion::Deletion;
pub use gateways::{GatewayRegistration, gateway_directory};
pub use legacy::{LegacyEnrollment, legacy_login};
pub use objects::ObjectDocument;
pub use password::MIN_PASSWORD_CHARS;
pub use reset::{ResetData, reset};
pub use session::Session;
pub use terms::{TermsStatus, central_terms, gateway_terms};

/// Transport contract: verified HTTPS in live mode, no redirects, bounded bodies/timeouts,
/// no ambient proxy credentials and no logging of headers, bodies or remote diagnostics.
/// A transport is constructed for exactly one [`Endpoint`], never supplied by a gateway.
pub trait Transport {
    /// Endpoint selected by the client/operator configuration.
    fn endpoint(&self) -> &Endpoint;
    /// Exchange exactly one protocol request. Errors must not authorize legacy fallback.
    ///
    /// # Errors
    /// Returns redacted transport, authentication, conflict or capacity errors.
    fn exchange(&self, request: &Request, bearer: Option<&Token>) -> Result<Response, Error>;
}

fn context(endpoint: &Endpoint, identity: Identity) -> Result<Context, Error> {
    Context::new(endpoint.issuer(), identity.account, identity.epoch).map_err(|_| Error::Rejected)
}

/// Resolve the login mapping. `Legacy` permits [`legacy_login`] at this same
/// Central, live or the explicit development endpoint; never another
/// destination for the password.
///
/// # Errors
/// Rejects malformed mappings and unavailable transports.
pub fn discover(transport: &impl Transport, username: String) -> Result<Discovery, Error> {
    let Response::Discovery { discovery } =
        transport.exchange(&Request::Discover { username }, None)?
    else {
        return Err(Error::Rejected);
    };
    if let Discovery::Central { identity } = &discovery {
        context(transport.endpoint(), *identity)?;
    }
    Ok(discovery)
}

/// Complete OPAQUE before issuing an SDK session, unwrap its key, then restore its document.
/// Application schema validation must finish before [`Session::confirm_restored`] is called.
///
/// # Errors
/// Wrong password, substituted identity/wrapper, corrupt document and transport failure all reject.
pub fn login(
    transport: &impl Transport,
    identity: Identity,
    password: String,
) -> Result<Session, Error> {
    let context = context(transport.endpoint(), identity)?;
    let (login, message) = Login::start(password, context).map_err(|_| Error::Rejected)?;
    let Response::LoginChallenge { attempt, message } =
        transport.exchange(&Request::LoginStart { identity, message }, None)?
    else {
        return Err(Error::Rejected);
    };
    let result = login.finish(&message).map_err(|_| Error::Rejected)?;
    let Response::Session {
        token,
        identity: answered,
        wrapped_master,
    } = transport.exchange(
        &Request::LoginFinish {
            attempt,
            message: result.finalization,
        },
        None,
    )?
    else {
        return Err(Error::Rejected);
    };
    let master = (|| {
        if answered.account != identity.account || answered.epoch != identity.epoch {
            return Err(Error::Rejected);
        }
        let wrapped = WrappedMasterKey::from_bytes(&wrapped_master).map_err(|_| Error::Rejected)?;
        result
            .wrapping_key
            .unwrap(&wrapped)
            .map_err(|_| Error::Rejected)
    })();
    let master = match master {
        Ok(master) => master,
        Err(error) => {
            let _ = transport.exchange(&Request::Logout, Some(&token));
            return Err(error);
        }
    };
    let mut session = Session::new(transport.endpoint().clone(), identity, token, master);
    if let Err(error) = session.refresh(transport) {
        let _ = session.logout(transport);
        return Err(error);
    }
    Ok(session)
}

/// Enroll only with a bridge-authorized capability and a fully prepared initial document.
/// The SDK never grants ownership or deletes the legacy source. Login/restore follows separately.
///
/// # Errors
/// Rejects malformed enrollment, oversized data, encryption errors and failed/ambiguous commits.
pub fn enroll(
    transport: &impl Transport,
    identity: Identity,
    ticket: Token,
    password: String,
    document: &[u8],
) -> Result<(), Error> {
    let context = context(transport.endpoint(), identity)?;
    let (enrollment, message) =
        Enrollment::start(password, context).map_err(|_| Error::Rejected)?;
    let start = Request::EnrollmentStart { ticket, message };
    let Response::EnrollmentChallenge { message } = transport.exchange(&start, None)? else {
        return Err(Error::Rejected);
    };
    let result = enrollment.finish(&message).map_err(|_| Error::Rejected)?;
    let master = MasterKey::generate().map_err(|_| Error::Unavailable)?;
    let wrapped = result
        .wrapping_key
        .wrap(&master)
        .map_err(|_| Error::Unavailable)?;
    let object = ObjectContext::new(context, identity.account, 1).map_err(|_| Error::Rejected)?;
    let document = master
        .seal_object(object, document)
        .map_err(|_| Error::Rejected)?;
    let Request::EnrollmentStart { ticket, .. } = start else {
        return Err(Error::Rejected);
    };
    let request = Request::EnrollmentFinish {
        ticket,
        record: result.upload,
        wrapped_master: wrapped.as_bytes().to_vec(),
        document,
        recovery: None,
    };
    match transport.exchange(&request, None)? {
        Response::Done => Ok(()),
        _ => Err(Error::Rejected),
    }
}

/// Zeroizing plaintext account document returned only after authentication/decryption.
pub type Document = Zeroizing<Vec<u8>>;
