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
        api::Request::Catalogue { .. }
            | api::Request::Detail { .. }
            | api::Request::Tower {
                request: central_api::tower::Request::Read { .. }
            }
    ) {
        return Err(Error::Rejected);
    }
    match transport.exchange(&Request::Challenges { request }, None)? {
        Response::Challenges { response } => Ok(response),
        _ => Err(Error::Rejected),
    }
}

impl Session {
    /// Stream one account-owned challenge dataset into an export without retaining it in memory.
    /// Every page's ownership is enforced by Central; the SDK checks shape and cursor progress.
    ///
    /// # Errors
    /// Session or transport failure, invalid paging, or a caller error stops the export.
    pub fn export_challenges(
        &self,
        transport: &impl Transport,
        table: api::ExportTable,
        mut each: impl FnMut(api::ExportEntry) -> Result<(), Error>,
    ) -> Result<u64, Error> {
        let mut after = None;
        let mut count = 0_u64;
        loop {
            let api::Response::Export {
                entries,
                after: next,
            } = self.challenges(transport, api::Request::Export { table, after })?
            else {
                return Err(Error::Rejected);
            };
            api::validate_export_page(table, after, &entries, next)?;
            for entry in entries {
                each(entry)?;
                count = count.checked_add(1).ok_or(Error::Rejected)?;
            }
            if next.is_none() {
                return Ok(count);
            }
            after = next;
        }
    }

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
