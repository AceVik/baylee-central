//! Challenge operations use the configured Central endpoint, independent of the current gate.
use super::{Error, Request, Response, Session, Transport};
use central_api::challenges as api;

/// Read public tasks or rankings without a session.
///
/// # Errors
/// Refuses account operations, malformed replies or transport failure.
pub fn read(transport: &impl Transport, request: api::Request) -> Result<api::Response, Error> {
    if !matches!(
        request,
        api::Request::Catalogue { .. } | api::Request::Detail { .. }
    ) {
        return Err(Error::Rejected);
    }
    match transport.exchange(&Request::Challenges { request }, None)? {
        Response::Challenges { response } => Ok(response),
        _ => Err(Error::Rejected),
    }
}

impl Session {
    /// Read or update this account's challenge participation. A retry of `Begin` must retain
    /// its original request ID until its outcome is known; the server owns the attempt ID.
    ///
    /// # Errors
    /// Refuses another endpoint, expired sessions, conflicting attempts or unexpected replies.
    pub fn challenges(
        &self,
        transport: &impl Transport,
        request: api::Request,
    ) -> Result<api::Response, Error> {
        self.same_endpoint(transport)?;
        match transport.exchange(&Request::Challenges { request }, Some((&self.token).into()))? {
            Response::Challenges { response } => Ok(response),
            _ => Err(Error::Rejected),
        }
    }
}
