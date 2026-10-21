use super::{
    BrowserReply, Discovery, Error, IdentityService, Request, Response, Surface, Token,
    browser_operation,
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
            self.require_terms(bearer).await?;
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
            Request::SessionStatus => {
                let status = self.store.session_status(surface.session(bearer)?).await?;
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
            request @ (Request::LegacyInvalidate { .. } | Request::LegacyClose { .. }) => {
                self.legacy_mutation(request, bearer).await
            }
            request @ (Request::RegisterGateway { .. }
            | Request::GatewayDirectory { .. }
            | Request::GatewayHeartbeat { .. }
            | Request::SetGatewayAdmins { .. }
            | Request::CheckGatewayAdmin { .. }
            | Request::GatewaySession { .. }
            | Request::GatewayTicket { .. }
            | Request::RedeemGateway { .. }) => self.gateway_request(request, bearer).await,
            Request::Discover { username } => self.discover(&username).await,
            Request::LoginStart { identity, message } => {
                self.start(identity, message, surface).await
            }
            // Answered by `dispatch`, which knows how to hand out the session.
            Request::LoginFinish { .. } => Err(Error::Rejected),
            Request::Register { username, beta_key } => {
                self.register(&username, &beta_key, surface).await
            }
            Request::LegacyReserve {
                legacy_id,
                username,
            } => {
                self.bridge(bearer)?;
                let ticket = Token::generate()?;
                let identity = self
                    .store
                    .reserve_legacy(&legacy_id, &username, &ticket)
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
            Request::Logout => {
                self.store.logout(surface.session(bearer)?).await?;
                Ok(Response::Done)
            }
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
            .register(username, &key.digest(), &ticket)
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
