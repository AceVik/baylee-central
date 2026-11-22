//! Competitive play through the configured Central, independent of the player's home gateway.
use super::{Error, Request, Response, Session, Token, Transport};
use central_api::ranked as api;

/// Read a public board, season or profile.
///
/// # Errors
/// Rejects private operations, transport failures and mismatched response types.
pub fn read(transport: &impl Transport, request: api::Request) -> Result<api::Response, Error> {
    if !request.is_public() {
        return Err(Error::Rejected);
    }
    match transport.exchange(&Request::Ranked { request }, None)? {
        Response::Ranked { response } => Ok(response),
        _ => Err(Error::Rejected),
    }
}

impl Session {
    /// Manage this account's private queue entry or read the ladder.
    ///
    /// # Errors
    /// Requires this session's configured endpoint and a valid Central reply.
    pub fn ranked(
        &self,
        transport: &impl Transport,
        request: api::Request,
    ) -> Result<api::Response, Error> {
        self.same_endpoint(transport)?;
        match transport.exchange(&Request::Ranked { request }, Some((&self.token).into()))? {
            Response::Ranked { response } => Ok(response),
            _ => Err(Error::Rejected),
        }
    }
}

/// Send an availability update or match operation using a gateway credential.
///
/// # Errors
/// Central rejects unverified hosts, stale assignments and conflicting results.
pub fn host(
    transport: &impl Transport,
    credential: &Token,
    request: api::HostRequest,
) -> Result<api::HostResponse, Error> {
    match transport.exchange(&Request::RankedHost { request }, Some(credential.into()))? {
        Response::RankedHost { response } => Ok(response),
        _ => Err(Error::Rejected),
    }
}
