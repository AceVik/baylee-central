use super::{
    BrowserReply, Discovery, Error, IdentityService, Place, Request, Response, Seen, Surface,
    Token, ai_gate::ForAi, browser_only, browser_operation, challenge,
};
use central_api::{
    chat::{ChatRequest, ChatResponse},
    groups::{Change as GroupChange, GroupRequest, GroupResponse},
    social::{ContactAction, SocialRequest, SocialResponse},
};
use std::{collections::BTreeSet, time::Instant};

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
        // An AI account's session makes its runner's operations alone
        // (`ai_gate.rs`). Where the terms are read below, the same statement
        // says whether the account is an AI; elsewhere it is asked here,
        // before anything is done or spent for the operation.
        let for_ai = ForAi::of(&request);
        let needs_terms = super::terms::needs_acceptance(&request);
        if for_ai == ForAi::Refused && !needs_terms {
            self.refuse_ai(bearer, surface).await?;
        }
        if let Request::LoginFinish { attempt, message } = request {
            return self.finish(&attempt, message, surface, bearer).await;
        }
        // A runner's exchange needs no terms: like a sign-in, it has no
        // session yet whose account could have accepted them.
        if let Request::AiRunnerSession { token, account } = request {
            return self
                .ai_runner_session(&token, account, bearer, surface)
                .await;
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
        if matches!(request, Request::RefreshSession) {
            if surface != Surface::Browser {
                return Err(Error::Rejected);
            }
            let refresh = bearer.ok_or(Error::Rejected)?;
            let next_access = super::Token::generate()?;
            let next_refresh = super::Token::generate()?;
            return match self
                .store
                .refresh_browser_session(refresh, &next_access, &next_refresh)
                .await?
            {
                central_postgres::RefreshOutcome::Refreshed {
                    expires_at,
                    previous_digest,
                } => {
                    self.issued(&next_access);
                    let next_digest = central_postgres::Session::Browser(&next_access).digest();
                    self.live.rotated(&previous_digest, next_digest);
                    Ok(BrowserReply {
                        response: Response::Renewed { expires_at },
                        session: Some(next_access),
                        refresh: Some(next_refresh),
                        clear_cookies: false,
                    })
                }
                central_postgres::RefreshOutcome::Rejected { revoked_digest } => {
                    if let Some(digest) = revoked_digest {
                        self.live.ended(&digest);
                    }
                    Err(Error::Rejected)
                }
                central_postgres::RefreshOutcome::Reused { revoked_digest } => {
                    self.live.ended(&revoked_digest);
                    Err(Error::Rejected)
                }
            };
        }
        // A choice's budget is spent before the terms are read for it.
        self.presence_budget(&request, bearer)?;
        if needs_terms {
            self.require_terms(bearer, surface, for_ai != ForAi::Allowed)
                .await?;
        }
        Ok(BrowserReply {
            response: self.respond(request, bearer, surface, seen, place).await?,
            session: None,
            refresh: None,
            clear_cookies: false,
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
        match request {
            Request::TowerToken {} => self.tower_token(bearer).await,
            Request::TowerIdentity {} => self.tower_identity(bearer).await,
            Request::TowerAward { delivery } => self.tower_award(&delivery, bearer).await,
            Request::TowerProgress {} => self.tower_progress(bearer, surface).await,
            Request::TowerAwardExport { after } => {
                self.tower_award_export(after, bearer, surface).await
            }
            Request::Challenges { request } => self.challenges(request, bearer, surface).await,
            Request::Advent { request } => self.advent(request, bearer, surface).await,
            Request::AdventHost { request } => self.advent_host(request, bearer).await,
            Request::Ranked { request } => self.ranked(request, bearer, surface).await,
            Request::RankedHost { request } => self.ranked_host(request, bearer).await,
            Request::ChallengeHost { request } => self.challenge_host(request, bearer).await,
            Request::Notifications { request } => self.notifications(request, bearer).await,
            Request::Social { request } => self.social(request, bearer, surface).await,
            request @ (Request::Presence
            | Request::SetPresence { .. }
            | Request::ReportPresence { .. }) => {
                self.presence_request(request, bearer, surface).await
            }
            Request::PublicFunding {} => self.public_funding(),
            other => {
                self.account_dispatch(other, bearer, surface, seen, place)
                    .await
            }
        }
    }

    async fn account_dispatch(
        &self,
        request: Request,
        bearer: Option<&Token>,
        surface: Surface,
        seen: Seen,
        place: Option<Place>,
    ) -> Result<Response, Error> {
        match request {
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
            Request::SendRecoveryBackupMail {
                email,
                attachment_base64,
                encrypted,
                lang,
            } => {
                self.send_recovery_backup_mail(bearer, &email, attachment_base64, encrypted, &lang)
                    .await
            }
            Request::PublicProfile { handle } => Ok(Response::PublicProfile {
                profile: Box::new(self.store.public_profile(&handle).await?),
            }),
            Request::FriendInviteInfo { code } => self.friend_invite_info(&code).await,
            request @ (Request::AccountSummary
            | Request::SetRecovery { .. }
            | Request::SetDisplayName { .. }
            | Request::ProfileDetails
            | Request::SetProfileDetails { .. }
            | Request::OwnBadges
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
            | Request::LegacySourcePage { .. }
            | Request::LegacyFinishArchive { .. }
            | Request::LegacyLink { .. }) => self.legacy(request, bearer, surface, place).await,
            request @ (Request::RegisterGateway { .. }
            | Request::GatewayDirectory { .. }
            | Request::GatewayHeartbeat { .. }
            | Request::GatewayPulse { .. }
            | Request::PublicFigures { .. }
            | Request::SetGatewayAdmins { .. }
            | Request::GatewayToken { .. }
            | Request::RankedToken { .. }
            | Request::RegisterInvalidate { .. }
            | Request::GatewayRevocations { .. }
            | Request::GatewayStatus) => self.gateway_request(request, bearer).await,
            request @ (Request::AccountExport
            | Request::ExportProfileDetails
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
            request @ (Request::LoginStart { .. }
            | Request::ChangePasswordStart { .. }
            | Request::ResetStart { .. }
            | Request::ResetFinish { .. }
            | Request::LoginFinish { .. }
            | Request::ChangePasswordFinish { .. }) => {
                self.credential_request(request, bearer, surface, place)
                    .await
            }
            request @ (Request::RegisterChallenge
            | Request::RegisterChallengeFor { .. }
            | Request::RegisterInviteChallenge { .. }
            | Request::Register { .. }
            | Request::EnrollmentStart { .. }
            | Request::EnrollmentFinish { .. }) => self.enrollment(request, surface, seen).await,
            request @ (Request::ReadDocument
            | Request::WriteDocument { .. }
            | Request::ConfirmRestore { .. }) => self.document_request(request, bearer).await,
            _ => Err(Error::Rejected),
        }
    }

    /// Registration challenges, proof of work and credential enrollment.
    async fn enrollment(
        &self,
        request: Request,
        surface: Surface,
        seen: Seen,
    ) -> Result<Response, Error> {
        match request {
            Request::RegisterChallenge => Self::older_challenge(surface),
            Request::RegisterChallengeFor { signals } => {
                self.register_challenge(surface, seen, signals, None).await
            }
            Request::RegisterInviteChallenge {
                signals,
                invite_code,
            } => {
                self.register_challenge(surface, seen, signals, Some(&invite_code))
                    .await
            }
            Request::Register {
                username,
                display_name,
                nonce,
                counters,
                invite_code,
            } => {
                let proof = (nonce.as_str(), counters.as_slice());
                self.register(
                    &username,
                    display_name.as_deref(),
                    proof,
                    invite_code.as_deref(),
                    surface,
                )
                .await
            }
            Request::EnrollmentStart { ticket, message } => {
                self.enrollment_challenge(&ticket, message).await
            }
            request @ Request::EnrollmentFinish { .. } => self.enrollment_finish(request).await,
            _ => Err(Error::Rejected),
        }
    }

    async fn social(
        &self,
        request: SocialRequest,
        bearer: Option<&Token>,
        surface: Surface,
    ) -> Result<Response, Error> {
        if surface == Surface::Native && native_refuses(&request) {
            return Err(Error::Rejected);
        }
        if matches!(
            &request,
            SocialRequest::FriendInvites
                | SocialRequest::CreateFriendInvite { .. }
                | SocialRequest::RevokeFriendInvite { .. }
                | SocialRequest::RedeemFriendInvite { .. }
        ) {
            return self.social_operation(request, bearer, surface).await;
        }
        let session = surface.session(bearer)?;
        let hint = social_hint(&request);
        let (response, concerns) = self.store.social_concerning(session, request).await?;
        // Committed: the push sockets of exactly the accounts it concerns are
        // told, whichever listener the change came from (`live.rs`).
        self.tell_social(session, hint, &response, &concerns).await;
        Ok(Response::Social { response })
    }

    async fn caller_for(
        &self,
        session: central_postgres::Session<'_>,
        targets: &BTreeSet<[u8; 16]>,
    ) -> Option<([u8; 16], u32)> {
        if !self.has_structured(targets) {
            return None;
        }
        if let Some(cached) = self.caller_tag_cached(&session.digest()) {
            return Some(cached);
        }
        self.store.session_account_tag(session).await.ok().flatten()
    }

    async fn tell_social(
        &self,
        session: central_postgres::Session<'_>,
        hint: SocialHint,
        response: &SocialResponse,
        concerns: &central_postgres::Concerns,
    ) {
        match (hint, response) {
            (
                SocialHint::ChatSend { conv },
                SocialResponse::Chat {
                    response: ChatResponse::Sent { id, at },
                },
            ) if !concerns.chat.is_empty() => {
                let caller = self.caller_for(session, &concerns.chat).await;
                self.tell_chat_message(
                    &concerns.chat,
                    &conv,
                    id,
                    *at,
                    caller.map(|(_, tag)| tag),
                    &concerns.social,
                );
            }
            (
                SocialHint::GroupCreate,
                SocialResponse::Group {
                    response: GroupResponse::Created { id },
                },
            ) if !concerns.social.is_empty() => {
                self.tell_thread(&concerns.social, "created", id);
            }
            (
                SocialHint::GroupChange { id, action },
                SocialResponse::Group {
                    response: GroupResponse::Done,
                },
            ) if !concerns.social.is_empty() => {
                self.tell_thread(&concerns.social, action, &id);
            }
            (SocialHint::Contact { tag, action }, _) => {
                let caller = self.caller_for(session, &concerns.social).await;
                self.tell_contact(&concerns.social, concerns.pair, tag, action, caller);
            }
            _ => self.tell(concerns),
        }
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
                // Both native and browser push sockets end with their session.
                self.live.ended(&session.digest());
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
            Request::ProfileDetails => Ok(Response::ProfileDetails {
                details: self.store.profile_details(cookie).await?,
            }),
            Request::SetProfileDetails { details } => {
                self.store.set_profile_details(cookie, &details).await?;
                Ok(Response::Done)
            }
            Request::OwnBadges => Ok(Response::OwnBadges {
                badges: Box::new(self.store.own_badges(cookie).await?),
            }),
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
        invite_code: Option<&str>,
    ) -> Result<Response, Error> {
        if surface != Surface::Browser {
            return Err(Error::Rejected);
        }
        let allowed = if self.store.registration_open().await? {
            true
        } else if let Some(code) = invite_code {
            self.store.invite_can_register(code).await?
        } else {
            false
        };
        if !allowed {
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
        invite_code: Option<&str>,
        surface: Surface,
    ) -> Result<Response, Error> {
        // Registration has the browser listener's per-address budget; the
        // native surface has none and does not register yet (M6).
        if surface != Surface::Browser {
            return Err(Error::Rejected);
        }
        // The proof of work is checked and spent first, whatever follows,
        // before the name is looked at; the store refuses while registration
        // is closed (unless a live invite code authorises enrollment).
        challenge::prove(
            &self.challenges,
            &self.workers,
            nonce,
            counters,
            Instant::now(),
        )
        .await?;
        let ticket = Token::generate()?;
        let identity = self
            .store
            .register_with_invite(username, display_name, &ticket, invite_code)
            .await?;
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
            request @ (Request::LegacyExport
            | Request::LegacyLink { .. }
            | Request::LegacySourcePage { .. }
            | Request::LegacyFinishArchive { .. }) => {
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
        let concerns = self
            .store
            .enroll_concerning(
                &ticket,
                &record,
                &wrapped_master,
                &document,
                recovery.as_deref(),
            )
            .await?;
        self.tell(&concerns);
        Ok(Response::Done)
    }
}

/// Social requests the native listener refuses. A picture's report is the
/// website's: the game client reads keys, not reports. The keys of tags
/// (`Pictures`) are served on both listeners by the same rules since M6
/// (19.11.2026, `docs/profile-pictures.md`), so the game client can show
/// the pictures of the people it lists.
enum SocialHint {
    ChatSend { conv: String },
    GroupCreate,
    GroupChange { id: String, action: &'static str },
    Contact { tag: u32, action: ContactAction },
    Other,
}

const fn group_action(change: &GroupChange) -> &'static str {
    match change {
        GroupChange::Rename { .. } => "renamed",
        GroupChange::SetDescription { .. } => "description",
        GroupChange::SetAdmin { .. } => "admin",
        GroupChange::RemovePicture => "picture",
        GroupChange::Invite { .. } => "invited",
        GroupChange::Join => "accepted",
        GroupChange::Withdraw { .. } => "withdrawn",
        GroupChange::Remove { .. } => "removed",
        GroupChange::Transfer { .. } => "transferred",
        GroupChange::Delete => "deleted",
    }
}

fn social_hint(request: &SocialRequest) -> SocialHint {
    match request {
        SocialRequest::Chat {
            request: ChatRequest::Send { conv, .. },
        } => SocialHint::ChatSend { conv: conv.clone() },
        SocialRequest::Group {
            request: GroupRequest::Create { .. },
        } => SocialHint::GroupCreate,
        SocialRequest::Group {
            request: GroupRequest::Change { id, change },
        } => SocialHint::GroupChange {
            id: id.clone(),
            action: group_action(change),
        },
        SocialRequest::Contact { tag, change } => SocialHint::Contact {
            tag: *tag,
            action: *change,
        },
        _ => SocialHint::Other,
    }
}

fn native_refuses(request: &SocialRequest) -> bool {
    matches!(
        request,
        central_api::social::SocialRequest::ReportPicture { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::native_refuses;
    use central_api::picture::{MAX_PICTURE_TAGS, PictureReason};
    use central_api::social::SocialRequest;

    #[test]
    fn the_native_listener_reads_picture_keys_and_files_no_report() {
        for tags in [vec![], vec![0x1a2b], vec![7; MAX_PICTURE_TAGS]] {
            assert!(!native_refuses(&SocialRequest::Pictures { tags }));
        }
        assert!(native_refuses(&SocialRequest::ReportPicture {
            tag: 0x1a2b,
            key: [1; 16],
            reason: PictureReason::Hate,
            note: None,
        }));
        assert!(!native_refuses(&SocialRequest::Contacts));
        assert!(!native_refuses(&SocialRequest::Lookup { tag: 0x1a2b }));
    }
}
