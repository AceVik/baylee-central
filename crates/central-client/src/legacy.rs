//! The legacy sign-in and move (`docs/native-login.md` §"Legacy login"): the
//! old password goes once, to this Central only, which verifies it against the
//! official gateway's hash, reserves the account and hands its source over.
//! Enrollment and the OPAQUE sign-in that follow use the password locally.
use super::{Error, Identity, Request, Response, Session, Token, Transport, context};
use central_api::{
    Endpoint,
    legacy::{LegacyPassword, LegacySnapshot},
};

/// A legacy sign-in's reservation.
pub struct LegacyEnrollment {
    /// The account's reserved Central identity.
    pub identity: Identity,
    /// The enrollment capability for [`crate::enroll`].
    pub ticket: Token,
    /// The official gateway the account moves from.
    pub gateway: String,
    /// Its source, checked against its digest.
    pub snapshot: LegacySnapshot,
}

/// The gateway a move names: the official gateway's compiled origin from
/// before it moved with the live endpoint, an explicit loopback one in
/// development.
fn official(endpoint: &Endpoint, gateway: &str) -> Result<(), Error> {
    match endpoint.legacy_origin() {
        Some(official) if official == gateway => Ok(()),
        Some(_) => Err(Error::Rejected),
        None => Endpoint::development(gateway).map(|_| ()),
    }
}

/// Sign in a legacy account at this Central with its old password, after
/// [`crate::discover`] answered `Legacy`. The password reaches the
/// transport's endpoint alone.
///
/// # Errors
/// `Rejected` for every refusal before the proof and for a forced change or a
/// ban; `Conflict` once the account enrolled (sign in through OPAQUE);
/// `Busy` past the sign-in budget; and a malformed answer or a source that is
/// not its digest's.
pub fn legacy_login(
    transport: &impl Transport,
    username: String,
    password: &str,
) -> Result<LegacyEnrollment, Error> {
    let request = Request::LegacyLogin {
        username,
        password: LegacyPassword::new(password.to_owned()),
    };
    let response = transport.exchange(&request, None);
    drop(request);
    let Response::LegacyEnrollment {
        identity,
        ticket,
        gateway,
        snapshot,
    } = response?
    else {
        return Err(Error::Rejected);
    };
    context(transport.endpoint(), identity)?;
    official(transport.endpoint(), &gateway)?;
    if !snapshot.verified() {
        return Err(Error::Rejected);
    }
    Ok(LegacyEnrollment {
        identity,
        ticket,
        gateway,
        snapshot,
    })
}

impl Session {
    /// The account's source again, while its move is open.
    ///
    /// # Errors
    /// Rejects a closed or registered account, unaccepted terms
    /// (`TermsRequired`), a source that is not its digest's and failed
    /// network calls.
    pub fn legacy_export(&self, transport: &impl Transport) -> Result<LegacySnapshot, Error> {
        self.same_endpoint(transport)?;
        match transport.exchange(&Request::LegacyExport, Some(&self.token))? {
            Response::LegacySnapshot { snapshot } if snapshot.verified() => Ok(snapshot),
            _ => Err(Error::Rejected),
        }
    }

    /// Close the account's move once its document is restored and
    /// confirmed: `digest` is the source digest the document holds. Again
    /// after the close is no error.
    ///
    /// # Errors
    /// `Conflict` for a digest that is not the stored source's (export it
    /// again); rejects an unrestored or registered account, unaccepted terms
    /// (`TermsRequired`) and failed network calls.
    pub fn legacy_link(&self, transport: &impl Transport, digest: [u8; 32]) -> Result<(), Error> {
        self.same_endpoint(transport)?;
        match transport.exchange(&Request::LegacyLink { digest }, Some(&self.token))? {
            Response::Done => Ok(()),
            _ => Err(Error::Rejected),
        }
    }
}
