use super::{
    BrowserReply, Discovery, Error, IdentityService, Place, Request, Response, Seen, Surface,
    Token, browser_only, browser_operation, challenge,
};
use std::time::Instant;

impl IdentityService {
    /// `place` is where the request came from, for the starts that test a
    /// guessed password (`guesses.rs`); nothing else reads it.
    pub(super) async fn dispatch(
        &self,
        request: Request,
        bearer: Option<&Token>,
        surface: Surface,
        seen: Seen,
        place: Option<Place>,
    ) -> Result<BrowserReply, Error> {
        if surface == Surface::Browser && !browser_operation(&request) {
            return Err(Error::Rejected);
        }
        // Before the terms are looked at: the native listener has no such operation.
        if surface == Surface::Native && browser_only(&request) {
            return Err(Error::Rejected);
        }
        if let Request::LoginFinish { attempt, message } = request {
            return self.finish(&attempt, message, surface, bearer).await;
        }
        if let Request::ChangePasswordFinish {
            attempt,
            proof,
            record,
            wrapped_master,
        } = request
        {
            // The start needed the terms accepted, and the attempt is bound to
            // its session; acceptances are never withdrawn, and a new version
            // comes with a restart, which forgets every pending attempt.
            return self
                .change_password_finish(bearer, surface, &attempt, proof, &record, &wrapped_master)
                .await;
        }
        Ok(BrowserReply {
            response: self.respond(request, bearer, surface, seen, place).await?,
            session: None,
        })
    }

    async fn respond(
        &self,
        request: Request,
        bearer: Option<&Token>,
        surface: Surface,
        seen: Seen,
        place: Option<Place>,
    ) -> Result<Response, Error> {
        if super::terms::needs_acceptance(&request) {
            self.require_terms(bearer, surface).await?;
        }
        match request {
            Request::Notifications { request } => self.notifications(request, bearer).await,
            Request::Social { request } => self.social(request, bearer, surface).await,
            request @ (Request::Terms
            | Request::GatewayTerms { .. }
            | Request::TermsStatus
            | Request::AcceptTerms { .. }
            | Request::PublishGatewayTerms { .. }) => {
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
            Request::Email { request } => self.email_request(request, bearer, surface).await,
            Request::ResetMail { username, lang } => self.reset_mail(username, lang),
            request @ (Request::AccountSummary
            | Request::SetRecovery { .. }
            | Request::SetDisplayName { .. }
            | Request::Admin { .. }
            | Request::Owner { .. }
            | Request::Picture { .. }) => self.account_request(request, bearer, surface).await,
            request @ (Request::LegacyReserve { .. }
            | Request::LegacyInvalidate { .. }
            | Request::LegacyClose { .. }
            | Request::LegacyProfile { .. }
            | Request::LegacyStatus { .. }
            | Request::LegacyLogin { .. }
            | Request::LegacyExport
            | Request::LegacyLink { .. }) => self.legacy(request, bearer, surface, place).await,
            request @ (Request::RegisterGateway { .. }
            | Request::GatewayDirectory { .. }
            | Request::GatewayHeartbeat { .. }
            | Request::GatewayPulse { .. }
            | Request::PublicFigures { .. }
            | Request::SetGatewayAdmins { .. }
            | Request::GatewayToken { .. }
            | Request::RegisterInvalidate { .. }
            | Request::GatewayRevocations { .. }
            | Request::GatewayStatus) => self.gateway_request(request, bearer).await,
            request @ (Request::AccountExport
            | Request::DeleteAccountStart { .. }
            | Request::DeleteAccountFinish { .. }) => {
                self.lifecycle_request(request, bearer, surface, place)
                    .await
            }
            request @ (Request::KeyCheckStatus
            | Request::SetKeyCheck { .. }
            | Request::PairStart
            | Request::PairStatus { .. }
            | Request::PairApprove { .. }
            | Request::PairCancel { .. }
            | Request::PairClaim { .. }
            | Request::PairPoll { .. }
            | Request::PairRedeem { .. }) => self.pairing_request(request, bearer, surface).await,
            Request::Discover { username } => self.discover(&username).await,
            Request::LoginStart { identity, message } => {
                self.start(identity, message, surface, place).await
            }
            request @ (Request::ChangePasswordStart { .. }
            | Request::ResetStart { .. }
            | Request::ResetFinish { .. }
            | Request::LoginFinish { .. }
            | Request::ChangePasswordFinish { .. }) => {
                self.credential_request(request, bearer, surface, place)
                    .await
            }
            Request::RegisterChallenge => Self::older_challenge(surface),
            Request::RegisterChallengeFor { signals } => {
                self.register_challenge(surface, seen, signals).await
            }
            request @ Request::Register { .. } => self.register_request(request, surface).await,
            Request::EnrollmentStart { ticket, message } => {
                self.enrollment_challenge(&ticket, message).await
            }
            request @ Request::EnrollmentFinish { .. } => self.enrollment_finish(request).await,
            request @ (Request::ReadDocument
            | Request::WriteDocument { .. }
            | Request::ConfirmRestore { .. }) => self.document_request(request, bearer).await,
        }
    }

    /// A registration, with its proof of work (`docs/registration-abuse.md`).
    async fn register_request(
        &self,
        request: Request,
        surface: Surface,
    ) -> Result<Response, Error> {
        let Request::Register {
            username,
            display_name,
            nonce,
            counters,
        } = request
        else {
            return Err(Error::Rejected);
        };
        let proof = (nonce.as_str(), counters.as_slice());
        self.register(&username, display_name.as_deref(), proof, surface)
            .await
    }

    async fn social(
        &self,
        request: central_api::social::SocialRequest,
        bearer: Option<&Token>,
        surface: Surface,
    ) -> Result<Response, Error> {
        // Pictures are the website's until the native client has a route
        // of its own (M6, `docs/profile-pictures.md`).
        if surface == Surface::Native
            && matches!(
                request,
                central_api::social::SocialRequest::Pictures { .. }
                    | central_api::social::SocialRequest::ReportPicture { .. }
            )
        {
            return Err(Error::Rejected);
        }
        Ok(Response::Social {
            response: self.store.social(surface.session(bearer)?, request).await?,
        })
    }

    async fn notifications(
        &self,
        request: central_api::notifications::NotificationRequest,
        bearer: Option<&Token>,
    ) -> Result<Response, Error> {
        if matches!(
            &request,
            central_api::notifications::NotificationRequest::Subscribe { .. }
        ) && self.push_key.is_none()
        {
            return Err(Error::Unavailable);
        }
        let mut state = self
            .store
            .notifications(bearer.ok_or(Error::Rejected)?, request)
            .await?;
        state.public_key.clone_from(&self.push_key);
        Ok(Response::Notifications { state })
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
    /// (`docs/web-accounts.md`); and administration (`docs/administration.md`).
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
            Request::Admin { request } => self.admin_request(request, bearer, surface).await,
            Request::Owner { request } => self.owner_request(request, cookie).await,
            // An account's own picture (`docs/profile-pictures.md`).
            Request::Picture { request } => self.picture_request(request, cookie).await,
            Request::AccountSummary => {
                let summary = self.store.account_summary(cookie).await?;
                Ok(Response::AccountSummary {
                    identity: summary.identity,
                    username: summary.username,
                    display_name: summary.display_name,
                    tag: summary.tag,
                    source: summary.source,
                    recovery: summary.recovery,
                    admin: summary.admin,
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

    /// A proof-of-work challenge for one registration
    /// (`docs/registration-abuse.md`), or word that an administrator closed
    /// registration. What the listener saw of the request and what the page
    /// reported of its browser (`signals`) only make it harder: no request
    /// is ever refused for its look.
    async fn register_challenge(
        &self,
        surface: Surface,
        seen: Seen,
        signals: u8,
    ) -> Result<Response, Error> {
        if surface != Surface::Browser {
            return Err(Error::Rejected);
        }
        if !self.store.registration_open().await? {
            return Ok(Response::RegistrationClosed);
        }
        let issued = self
            .challenges
            .lock()
            .map_err(|_| Error::Unavailable)?
            .issue(Instant::now(), challenge::points(seen, signals))?;
        Ok(Response::RegisterWork {
            nonce: issued.nonce,
            parts: issued.work.parts,
            difficulty: issued.work.difficulty,
            memory: issued.work.memory,
            passes: issued.work.passes,
            expires_at: issued.expires_at,
        })
    }

    /// The request for the older, SHA-256 proof: this service issues none,
    /// so no nonce comes of it and nothing is kept. The answer is the one
    /// a page from before has words for, registration is paused; a page of
    /// this build never asks this of a Central that answers its own
    /// request (`docs/registration-abuse.md` §"A proof that costs memory").
    fn older_challenge(surface: Surface) -> Result<Response, Error> {
        if surface != Surface::Browser {
            return Err(Error::Rejected);
        }
        Ok(Response::RegistrationClosed)
    }

    async fn register(
        &self,
        username: &str,
        display_name: Option<&str>,
        (nonce, counters): (&str, &[u64]),
        surface: Surface,
    ) -> Result<Response, Error> {
        // Registration has the browser listener's per-address budget; the
        // native surface has none and does not register yet (M6).
        if surface != Surface::Browser {
            return Err(Error::Rejected);
        }
        // The proof of work is checked and spent first, whatever follows,
        // before the name is looked at; the store refuses while registration
        // is closed. `Busy` without a worker spends nothing.
        challenge::prove(
            &self.challenges,
            &self.workers,
            nonce,
            counters,
            Instant::now(),
        )
        .await?;
        let ticket = Token::generate()?;
        let identity = self.store.register(username, display_name, &ticket).await?;
        self.challenges
            .lock()
            .map_err(|_| Error::Unavailable)?
            .registered(Instant::now());
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

    /// Every legacy operation: the legacy store's sign-in and move
    /// (`legacy.rs`), and the bridge's own, which only a Central without the
    /// store serves.
    async fn legacy(
        &self,
        request: Request,
        bearer: Option<&Token>,
        surface: Surface,
        place: Option<Place>,
    ) -> Result<Response, Error> {
        match request {
            Request::LegacyLogin { username, password } => {
                self.legacy_login(&username, password, surface, place).await
            }
            request @ (Request::LegacyExport | Request::LegacyLink { .. }) => {
                self.legacy_request(request, bearer, surface).await
            }
            Request::LegacyStatus { legacy_id } => {
                self.bridge(bearer)?;
                if self.legacy.is_some() {
                    return Err(Error::Rejected);
                }
                let (identity, state) = self.store.legacy_status(&legacy_id).await?;
                Ok(Response::LegacyStatus { identity, state })
            }
            request => self.legacy_mutation(request, bearer).await,
        }
    }

    async fn legacy_mutation(
        &self,
        request: Request,
        bearer: Option<&Token>,
    ) -> Result<Response, Error> {
        self.bridge(bearer)?;
        // Central is the source once it holds the legacy store: the bridge
        // reserves, invalidates, closes and carries nothing any more.
        if self.legacy.is_some() {
            return Err(Error::Rejected);
        }
        match request {
            Request::LegacyReserve {
                legacy_id,
                username,
                display_name,
                tag,
            } => {
                let ticket = Token::generate()?;
                let identity = self
                    .store
                    .reserve_legacy(&legacy_id, &username, display_name.as_deref(), tag, &ticket)
                    .await?;
                return Ok(Response::Enrollment { identity, ticket });
            }
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

    async fn enrollment_finish(&self, request: Request) -> Result<Response, Error> {
        let Request::EnrollmentFinish {
            ticket,
            record,
            wrapped_master,
            document,
            recovery,
        } = request
        else {
            return Err(Error::Rejected);
        };
        self.store
            .enroll(
                &ticket,
                &record,
                &wrapped_master,
                &document,
                recovery.as_deref(),
            )
            .await
            .map(|()| Response::Done)
    }
}
