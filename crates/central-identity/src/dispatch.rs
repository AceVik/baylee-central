use super::{
    BrowserReply, Discovery, Error, IdentityService, Request, Response, Surface, Token,
    browser_only, browser_operation,
};
use central_api::beta_key::BetaKey;

impl IdentityService {
    pub(super) async fn dispatch(
        &self,
        request: Request,
        bearer: Option<&Token>,
        surface: Surface,
    ) -> Result<BrowserReply, Error> {
        if surface == Surface::Browser && !browser_operation(&request) {
            return Err(Error::Rejected);
        }
        // Before the terms are looked at: the native listener has no such operation.
        if surface == Surface::Native && browser_only(&request) {
            return Err(Error::Rejected);
        }
        if let Request::LoginFinish { attempt, message } = request {
            return self.finish(&attempt, message, surface).await;
        }
        Ok(BrowserReply {
            response: self.respond(request, bearer, surface).await?,
            session: None,
        })
    }

    async fn respond(
        &self,
        request: Request,
        bearer: Option<&Token>,
        surface: Surface,
    ) -> Result<Response, Error> {
        if super::terms::needs_acceptance(&request) {
            self.require_terms(bearer, surface).await?;
        }
        match request {
            request @ (Request::Terms
            | Request::GatewayTerms { .. }
            | Request::TermsStatus
            | Request::AcceptTerms { .. }
            | Request::PublishGatewayTerms { .. }
            | Request::GatewayAuthorize { .. }) => {
                self.terms_request(request, bearer, surface).await
            }
            request @ (Request::CreateObject
            | Request::ReadObject { .. }
            | Request::ListObjects { .. }
            | Request::WriteObject { .. }
            | Request::DeleteObject { .. }) => self.object_request(request, bearer).await,
            request @ (Request::SessionStatus | Request::RenewSession | Request::Logout) => {
                self.session_request(&request, bearer, surface).await
            }
            request @ (Request::AccountSummary
            | Request::SetRecovery { .. }
            | Request::SetDisplayName { .. }) => {
                self.account_request(request, bearer, surface).await
            }
            request @ (Request::LegacyInvalidate { .. }
            | Request::LegacyClose { .. }
            | Request::LegacyProfile { .. }) => self.legacy_mutation(request, bearer).await,
            request @ (Request::RegisterGateway { .. }
            | Request::GatewayDirectory { .. }
            | Request::GatewayHeartbeat { .. }
            | Request::SetGatewayAdmins { .. }
            | Request::CheckGatewayAdmin { .. }
            | Request::GatewaySession { .. }
            | Request::GatewayProfile { .. }
            | Request::GatewayTicket { .. }
            | Request::RedeemGateway { .. }) => self.gateway_request(request, bearer).await,
            Request::Discover { username } => self.discover(&username).await,
            Request::LoginStart { identity, message } => {
                self.start(identity, message, surface).await
            }
            // Answered by `dispatch`, which knows how to hand out the session.
            Request::LoginFinish { .. } => Err(Error::Rejected),
            Request::Register {
                username,
                beta_key,
                display_name,
            } => {
                self.register(&username, &beta_key, display_name.as_deref(), surface)
                    .await
            }
            Request::LegacyReserve {
                legacy_id,
                username,
                display_name,
                tag,
            } => {
                self.bridge(bearer)?;
                let ticket = Token::generate()?;
                let identity = self
                    .store
                    .reserve_legacy(&legacy_id, &username, display_name.as_deref(), tag, &ticket)
                    .await?;
                Ok(Response::Enrollment { identity, ticket })
            }
            Request::EnrollmentStart { ticket, message } => {
                self.enrollment_challenge(&ticket, message).await
            }
            Request::EnrollmentFinish {
                ticket,
                record,
                wrapped_master,
                document,
                recovery,
            } => self
                .store
                .enroll(
                    &ticket,
                    &record,
                    &wrapped_master,
                    &document,
                    recovery.as_deref(),
                )
                .await
                .map(|()| Response::Done),
            request @ (Request::ReadDocument
            | Request::WriteDocument { .. }
            | Request::ConfirmRestore { .. }) => self.document_request(request, bearer).await,
            Request::LegacyStatus { legacy_id } => {
                self.bridge(bearer)?;
                let (identity, state) = self.store.legacy_status(&legacy_id).await?;
                Ok(Response::LegacyStatus { identity, state })
            }
        }
    }

    /// The session's own requests, on either listener: its status, its
    /// renewal (up to 12 hours after the sign-in) and its end.
    async fn session_request(
        &self,
        request: &Request,
        bearer: Option<&Token>,
        surface: Surface,
    ) -> Result<Response, Error> {
        let session = surface.session(bearer)?;
        match request {
            Request::RenewSession => Ok(Response::Renewed {
                expires_at: self.store.renew_session(session).await?,
            }),
            Request::Logout => {
                self.store.logout(session).await?;
                Ok(Response::Done)
            }
            Request::SessionStatus => {
                let status = self.store.session_status(session).await?;
                Ok(match surface {
                    Surface::Native => Response::SessionStatus {
                        identity: status.identity,
                        expires_at: status.expires_at,
                        migration_closed: status.migration_closed,
                    },
                    // The page shows the name; the native answer stays as old
                    // clients decode it.
                    Surface::Browser => Response::BrowserStatus {
                        identity: status.identity,
                        username: status.username,
                        expires_at: status.expires_at,
                    },
                })
            }
            _ => Err(Error::Rejected),
        }
    }

    /// The account page's requests, on the browser listener only
    /// ([`browser_only`]): the session's account, a recovery wrapper for an
    /// account without one, and a display name for an account without one
    /// (`docs/web-accounts.md`).
    async fn account_request(
        &self,
        request: Request,
        bearer: Option<&Token>,
        surface: Surface,
    ) -> Result<Response, Error> {
        let (Surface::Browser, Some(cookie)) = (surface, bearer) else {
            return Err(Error::Rejected);
        };
        match request {
            Request::AccountSummary => {
                let summary = self.store.account_summary(cookie).await?;
                Ok(Response::AccountSummary {
                    identity: summary.identity,
                    username: summary.username,
                    display_name: summary.display_name,
                    tag: summary.tag,
                    source: summary.source,
                    recovery: summary.recovery,
                    // Roles come with a later step (the accounts and
                    // administration design, step 5).
                    admin: false,
                    signed_in_at: summary.signed_in_at,
                    expires_at: summary.expires_at,
                })
            }
            Request::SetRecovery { recovery } => {
                self.store.set_recovery(cookie, &recovery).await?;
                Ok(Response::Done)
            }
            Request::SetDisplayName { display_name } => {
                self.store.set_display_name(cookie, &display_name).await?;
                Ok(Response::Done)
            }
            _ => Err(Error::Rejected),
        }
    }

    async fn discover(&self, username: &str) -> Result<Response, Error> {
        let discovery = if let Some(identity) = self.store.discover(username).await? {
            Discovery::Central { identity }
        } else if self.legacy_discovery {
            Discovery::Legacy
        } else {
            return Err(Error::Rejected);
        };
        Ok(Response::Discovery { discovery })
    }

    async fn register(
        &self,
        username: &str,
        beta_key: &str,
        display_name: Option<&str>,
        surface: Surface,
    ) -> Result<Response, Error> {
        // Registration has the browser listener's per-address budget; the
        // native surface has none and does not register yet (M6).
        if surface != Surface::Browser {
            return Err(Error::Rejected);
        }
        // A malformed key is refused before the name is looked at.
        let key = BetaKey::parse(beta_key)?;
        let ticket = Token::generate()?;
        let identity = self
            .store
            .register(username, display_name, &key.digest(), &ticket)
            .await?;
        Ok(Response::Enrollment { identity, ticket })
    }

    async fn document_request(
        &self,
        request: Request,
        bearer: Option<&Token>,
    ) -> Result<Response, Error> {
        let session = bearer.ok_or(Error::Rejected)?;
        match request {
            Request::ReadDocument => {
                let (revision, document) = self.store.document(session).await?;
                Ok(Response::Document { revision, document })
            }
            Request::WriteDocument {
                expected_revision,
                document,
            } => {
                self.store
                    .write_document(session, expected_revision, &document)
                    .await?;
                Ok(Response::Done)
            }
            Request::ConfirmRestore { revision } => {
                self.store.confirm_restore(session, revision).await?;
                Ok(Response::Done)
            }
            _ => Err(Error::Rejected),
        }
    }

    async fn legacy_mutation(
        &self,
        request: Request,
        bearer: Option<&Token>,
    ) -> Result<Response, Error> {
        self.bridge(bearer)?;
        match request {
            Request::LegacyInvalidate { legacy_id } => {
                self.store.invalidate_legacy(&legacy_id).await?;
            }
            Request::LegacyClose {
                legacy_id,
                identity,
            } => self.store.close_legacy(&legacy_id, identity).await?,
            Request::LegacyProfile {
                legacy_id,
                display_name,
                tag,
            } => {
                self.store
                    .legacy_profile(&legacy_id, display_name.as_deref(), tag)
                    .await?;
            }
            _ => return Err(Error::Rejected),
        }
        Ok(Response::Done)
    }

    async fn enrollment_challenge(
        &self,
        ticket: &Token,
        message: Vec<u8>,
    ) -> Result<Response, Error> {
        let identity = self.store.enrollment(ticket).await?;
        let context = self.context(identity)?;
        let permit = self
            .workers
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::Busy)?;
        let keys = self.keys.clone();
        let message = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            keys.enrollment_response(context, &message)
                .map_err(|_| Error::Rejected)
        })
        .await
        .map_err(|_| Error::Unavailable)??;
        Ok(Response::EnrollmentChallenge { message })
    }
}
