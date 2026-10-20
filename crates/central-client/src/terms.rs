//! Terms of use (`docs/terms.md`): read them, ask whether they are accepted,
//! accept exactly the version shown. Every document read is checked here
//! against its own digest, so a client never shows a text other than the one
//! it then names in [`Session::accept_terms`].
use super::{Error, Request, Response, Session, Transport};
use central_api::{
    gateway::validate_origin,
    terms::{self, TermsDocument, TermsSubject, TermsVersion},
};

/// Central's terms for one session.
pub struct TermsStatus {
    /// Central's version in force, or none.
    pub current: Option<TermsVersion>,
    /// True when the account accepted `current`, or when none is in force.
    pub accepted: bool,
}

fn checked(document: Option<TermsDocument>) -> Result<Option<TermsDocument>, Error> {
    if let Some(document) = &document {
        terms::validate(&document.texts)?;
        if !terms::valid_version(&document.version.version)
            || terms::digest(&document.texts) != document.version.digest
        {
            return Err(Error::Rejected);
        }
    }
    Ok(document)
}

fn development(transport: &impl Transport) -> bool {
    transport.endpoint().origin().starts_with("http://")
}

/// Central's terms in force, or none.
///
/// # Errors
/// Rejects a document whose texts do not match its digest, and transport failure.
pub fn central_terms(transport: &impl Transport) -> Result<Option<TermsDocument>, Error> {
    let Response::Terms { terms } = transport.exchange(&Request::Terms, None)? else {
        return Err(Error::Rejected);
    };
    checked(terms)
}

/// A gateway's current terms as Central holds them, or none.
///
/// # Errors
/// Rejects a non-canonical origin, a document whose texts do not match its
/// digest, and transport failure.
pub fn gateway_terms(
    transport: &impl Transport,
    origin: &str,
) -> Result<Option<TermsDocument>, Error> {
    validate_origin(origin, development(transport))?;
    let Response::Terms { terms } = transport.exchange(
        &Request::GatewayTerms {
            origin: origin.into(),
        },
        None,
    )?
    else {
        return Err(Error::Rejected);
    };
    checked(terms)
}

impl Session {
    /// Central's current version and whether this account accepted it.
    ///
    /// # Errors
    /// Rejects a revoked session and transport failure.
    pub fn terms_status(&self, transport: &impl Transport) -> Result<TermsStatus, Error> {
        self.same_endpoint(transport)?;
        let Response::TermsStatus { current, accepted } =
            transport.exchange(&Request::TermsStatus, Some(&self.token))?
        else {
            return Err(Error::Rejected);
        };
        if current
            .as_ref()
            .is_some_and(|v| !terms::valid_version(&v.version))
        {
            return Err(Error::Rejected);
        }
        Ok(TermsStatus { current, accepted })
    }

    /// Accept exactly `version` of `subject`'s terms: the version and digest of
    /// the document this client showed. Accepting it again is no error.
    ///
    /// # Errors
    /// `Conflict` when the document changed since it was read (read and show it
    /// again); rejects a revoked session and transport failure.
    pub fn accept_terms(
        &self,
        transport: &impl Transport,
        subject: TermsSubject,
        version: &TermsVersion,
    ) -> Result<(), Error> {
        self.same_endpoint(transport)?;
        if let TermsSubject::Gateway { origin } = &subject {
            validate_origin(origin, development(transport))?;
        }
        if !terms::valid_version(&version.version) {
            return Err(Error::Rejected);
        }
        match transport.exchange(
            &Request::AcceptTerms {
                subject,
                version: version.version.clone(),
                digest: version.digest,
            },
            Some(&self.token),
        )? {
            Response::Done => Ok(()),
            _ => Err(Error::Rejected),
        }
    }
}
