//! Central's own terms of use, compiled in from `legal/` (`docs/terms.md`),
//! and the operations on terms.
use super::{Error, IdentityService, Request, Response, Session, Surface, Token};
use central_api::{
    gateway::validate_origin,
    terms::{self, DRAFT, TermsDocument, TermsSubject, TermsText, TermsVersion},
};

/// The binding German text and its English translation, in tag order.
const SOURCES: [(&str, &str); 2] = [
    ("de", include_str!("../../../legal/terms.de.md")),
    ("en", include_str!("../../../legal/terms.en.md")),
];

/// A text's leading `<!-- key: value -->` lines, then its Markdown.
fn split(source: &str) -> (Option<&str>, Option<&str>, &str) {
    let (mut version, mut updated) = (None, None);
    let mut rest = source;
    while let Some(line) = rest.lines().next() {
        let Some(comment) = line
            .trim()
            .strip_prefix("<!--")
            .and_then(|l| l.strip_suffix("-->"))
        else {
            break;
        };
        match comment.trim().split_once(':') {
            Some(("version", value)) => version = Some(value.trim()),
            Some(("updated", value)) => updated = Some(value.trim()),
            _ => {}
        }
        rest = rest[line.len()..].strip_prefix('\n').unwrap_or("");
    }
    (version, updated, rest.trim_start_matches('\n'))
}

/// Central's terms in force: none while the texts are drafts.
///
/// # Errors
/// Rejects texts without a version, with versions that differ between the
/// languages, or outside the bounds every document keeps; the service then
/// does not start.
pub(super) fn central() -> Result<Option<TermsDocument>, Error> {
    let mut version = None;
    let mut texts = Vec::with_capacity(SOURCES.len());
    for (lang, source) in SOURCES {
        let (named, updated, markdown) = split(source);
        let named = named.ok_or(Error::Rejected)?;
        if version.is_some_and(|v| v != named) {
            return Err(Error::Rejected);
        }
        version = Some(named);
        texts.push(TermsText {
            lang: lang.into(),
            updated: updated.map(Into::into),
            markdown: markdown.into(),
        });
    }
    let version = version.ok_or(Error::Rejected)?;
    if version == DRAFT {
        return Ok(None);
    }
    if !terms::valid_version(version) {
        return Err(Error::Rejected);
    }
    terms::validate(&texts)?;
    Ok(Some(TermsDocument {
        version: TermsVersion {
            version: version.into(),
            digest: terms::digest(&texts),
        },
        texts,
    }))
}

/// Operations a session whose account has not accepted the terms in force
/// may still use: read its status, its summary and the terms, accept, read
/// and export its own data, confirm a restore, sign out.
pub(super) fn needs_acceptance(request: &Request) -> bool {
    matches!(
        request,
        Request::CreateObject
            | Request::WriteObject { .. }
            | Request::DeleteObject { .. }
            | Request::WriteDocument { .. }
            | Request::RegisterGateway { .. }
            | Request::SetGatewayAdmins { .. }
            | Request::GatewayTicket { .. }
            | Request::SetRecovery { .. }
            | Request::SetDisplayName { .. }
            | Request::ChangePasswordStart { .. }
            | Request::Admin { .. }
    )
}

impl IdentityService {
    fn current(&self) -> Option<&TermsVersion> {
        self.terms.as_ref().map(|document| &document.version)
    }

    /// Refuse a session operation while Central's terms in force are not
    /// accepted by the session's account. The token is a session of the
    /// listener's kind: on the browser listener the cookie's.
    pub(super) async fn require_terms(
        &self,
        bearer: Option<&Token>,
        surface: Surface,
    ) -> Result<(), Error> {
        let Some(current) = self.current() else {
            return Ok(());
        };
        if self
            .store
            .terms_status(surface.session(bearer)?, Some(current))
            .await?
        {
            Ok(())
        } else {
            Err(Error::TermsRequired)
        }
    }

    fn origin(&self, origin: &str) -> Result<(), Error> {
        validate_origin(origin, !self.legacy_discovery)
    }

    pub(super) async fn terms_request(
        &self,
        request: Request,
        bearer: Option<&Token>,
        surface: Surface,
    ) -> Result<Response, Error> {
        match request {
            Request::Terms => Ok(Response::Terms {
                terms: self.terms.clone(),
            }),
            Request::GatewayTerms { origin } => {
                self.origin(&origin)?;
                Ok(Response::Terms {
                    terms: self.store.gateway_terms(&origin).await?,
                })
            }
            Request::TermsStatus => Ok(Response::TermsStatus {
                accepted: self
                    .store
                    .terms_status(surface.session(bearer)?, self.current())
                    .await?,
                current: self.current().cloned(),
            }),
            Request::AcceptTerms {
                subject,
                version,
                digest,
            } => {
                let session = surface.session(bearer)?;
                let accepted = TermsVersion { version, digest };
                match subject {
                    TermsSubject::Central => {
                        if self.current() != Some(&accepted) {
                            return Err(Error::Conflict);
                        }
                        self.store.accept_central_terms(session, &accepted).await?;
                    }
                    TermsSubject::Gateway { origin } => {
                        // A gateway's terms are accepted from the client only.
                        let Session::Native(session) = session else {
                            return Err(Error::Rejected);
                        };
                        self.origin(&origin)?;
                        if !terms::valid_version(&accepted.version) {
                            return Err(Error::Rejected);
                        }
                        self.store
                            .accept_gateway_terms(session, &origin, &accepted)
                            .await?;
                    }
                }
                Ok(Response::Done)
            }
            Request::PublishGatewayTerms { version, texts } => {
                let (_, origin) = self.gateway_authority(bearer).await?;
                if !terms::valid_version(&version) {
                    return Err(Error::Rejected);
                }
                terms::validate(&texts)?;
                let published = TermsVersion {
                    version,
                    digest: terms::digest(&texts),
                };
                self.store
                    .publish_gateway_terms(&origin, &published, &texts)
                    .await?;
                Ok(Response::TermsPublished {
                    digest: published.digest,
                })
            }
            Request::GatewayAuthorize {
                lease,
                purpose,
                terms,
            } => {
                let (gateway, origin) = self.gateway_authority(bearer).await?;
                let (identity, expires_at, central_terms, gateway_terms) = self
                    .store
                    .gateway_authorize(
                        &lease,
                        &origin,
                        gateway,
                        purpose,
                        self.current(),
                        terms.as_ref(),
                    )
                    .await?;
                Ok(Response::GatewayAuthorization {
                    identity,
                    expires_at,
                    central_terms,
                    gateway_terms,
                })
            }
            _ => Err(Error::Rejected),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_committed_texts_parse_and_agree_on_one_version() {
        // Drafts are not in force; anything else must be a valid document.
        let parsed = central();
        assert!(parsed.is_ok(), "legal/terms.*.md must parse");
        for (_, source) in SOURCES {
            let (version, updated, markdown) = split(source);
            assert!(version.is_some(), "a version line");
            assert!(updated.is_some(), "an updated line");
            assert!(markdown.starts_with('#'), "the text begins with its title");
            assert!(!markdown.contains("<!--"), "no comment reaches the text");
        }
    }

    #[test]
    fn leading_comments_are_read_and_removed() {
        let (version, updated, markdown) =
            split("<!-- version: 2026-11-1 -->\n<!-- updated: 2026-11-02 -->\n\n# Title\n\nBody\n");
        assert_eq!(version, Some("2026-11-1"));
        assert_eq!(updated, Some("2026-11-02"));
        assert_eq!(markdown, "# Title\n\nBody\n");
        let (version, _, markdown) = split("# Title\n<!-- version: late -->\n");
        assert_eq!(version, None, "only leading lines name the version");
        assert_eq!(markdown, "# Title\n<!-- version: late -->\n");
    }

    #[test]
    fn a_draft_is_never_in_force() {
        let (version, _, _) = split(SOURCES[0].1);
        if version == Some(DRAFT) {
            assert!(central().unwrap().is_none());
        } else {
            let document = central().unwrap().expect("a published version is in force");
            assert_ne!(document.version.version, DRAFT);
            assert_eq!(document.texts.len(), SOURCES.len());
        }
    }

    #[test]
    fn only_using_operations_need_acceptance() {
        assert!(needs_acceptance(&Request::CreateObject));
        assert!(needs_acceptance(&Request::GatewayTicket {
            gateway: "https://gateway.invalid".into(),
            purpose: central_api::wire::TicketPurpose::Admit,
        }));
        assert!(needs_acceptance(&Request::SetRecovery {
            recovery: vec![1; 73]
        }));
        for free in [
            Request::SessionStatus,
            Request::AccountSummary,
            Request::RenewSession,
            Request::Logout,
            Request::Terms,
            Request::TermsStatus,
            Request::ReadDocument,
            Request::ConfirmRestore { revision: 1 },
            Request::ListObjects { after: None },
            // A person who will not accept new terms may still see their data and leave.
            Request::AccountExport,
            Request::DeleteAccountStart { login: vec![] },
            Request::DeleteAccountFinish {
                attempt: Token::generate().unwrap(),
                proof: vec![],
            },
        ] {
            assert!(!needs_acceptance(&free));
        }
    }
}
