//! Social requests keep the session credential inside the account SDK.
use super::{Error, Request, Response, Session, Transport};
use central_api::social::{Contact, ContactAction, Person, SocialRequest, SocialResponse};

impl Session {
    /// Read the account's contacts, requests and blocks from Central.
    ///
    /// # Errors
    /// Rejects a substituted endpoint, expired session or unexpected response.
    pub fn contacts(&self, transport: &impl Transport) -> Result<Vec<Contact>, Error> {
        match self.social(transport, SocialRequest::Contacts)? {
            SocialResponse::Contacts { contacts } => Ok(contacts),
            _ => Err(Error::Rejected),
        }
    }

    /// Find an exact public tag; never searches private account names.
    ///
    /// # Errors
    /// Rejects invalid sessions, tags and malformed responses; rate limits return `Busy`.
    pub fn contact_lookup(
        &self,
        transport: &impl Transport,
        tag: u32,
    ) -> Result<Option<Person>, Error> {
        match self.social(transport, SocialRequest::Lookup { tag })? {
            SocialResponse::Person { person } => Ok(person),
            _ => Err(Error::Rejected),
        }
    }

    /// Request, accept, remove, block or unblock one public account.
    ///
    /// # Errors
    /// Refuses unauthorized transitions, expired sessions and exhausted quotas.
    pub fn contact_change(
        &self,
        transport: &impl Transport,
        tag: u32,
        change: ContactAction,
    ) -> Result<(), Error> {
        match self.social(transport, SocialRequest::Contact { tag, change })? {
            SocialResponse::Done => Ok(()),
            _ => Err(Error::Rejected),
        }
    }

    /// Execute an account-owned social operation with the native session.
    ///
    /// # Errors
    /// Refuses another endpoint, expired sessions and unexpected responses.
    pub fn social(
        &self,
        transport: &impl Transport,
        request: SocialRequest,
    ) -> Result<SocialResponse, Error> {
        self.same_endpoint(transport)?;
        match transport.exchange(&Request::Social { request }, Some(&self.token))? {
            Response::Social { response } => Ok(response),
            _ => Err(Error::Rejected),
        }
    }
}
