//! Using an administrator's reset code (`docs/account-reset.md`), on either
//! listener without a session. The code authorizes the registration of a new
//! password; its start spends one unit of the account's login budget, after
//! the code is found, so nobody without it spends anything of the account's.
use super::{Credential, Error, IdentityService, Request, Response, Surface, Token};
use central_api::reset_code::ResetCode;
use central_api::wire::ResetVault;
use std::time::{Duration, Instant};

/// Pending resets at once; a start needs a valid code, so few are expected.
const PENDING_RESETS: usize = 32;

/// A reset whose registration response was answered.
pub(super) struct PendingReset {
    expires: Instant,
    credential: Credential,
    digest: [u8; 32],
    surface: Surface,
}

impl IdentityService {
    /// The requests that set a new password, apart from a change's finish,
    /// which `dispatch` answers with the caller's next session.
    pub(super) async fn credential_request(
        &self,
        request: Request,
        bearer: Option<&Token>,
        surface: Surface,
    ) -> Result<Response, Error> {
        match request {
            request @ Request::ChangePasswordStart { .. } => {
                self.change_password_start(request, bearer, surface).await
            }
            Request::ResetStart {
                username,
                code,
                registration,
            } => {
                self.reset_start(&username, &code, registration, surface)
                    .await
            }
            Request::ResetFinish {
                attempt,
                record,
                wrapped_master,
                vault,
            } => {
                self.reset_finish(&attempt, &record, &wrapped_master, &vault, surface)
                    .await
            }
            // Answered by `dispatch`, which knows how to hand out the session.
            _ => Err(Error::Rejected),
        }
    }

    async fn reset_start(
        &self,
        username: &str,
        code: &ResetCode,
        registration: Vec<u8>,
        surface: Surface,
    ) -> Result<Response, Error> {
        let digest = code.digest();
        let (credential, recovery) = self.store.reset_credential(username, &digest).await?;
        let identity = credential.identity;
        let context = self.context(identity)?;
        // The registration response evaluates the account's OPRF key once.
        self.limits
            .lock()
            .map_err(|_| Error::Unavailable)?
            .account(identity.account, Instant::now(), 1)?;
        let permit = self
            .workers
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::Busy)?;
        let keys = self.keys.clone();
        let registration = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            keys.enrollment_response(context, &registration)
                .map_err(|_| Error::Rejected)
        })
        .await
        .map_err(|_| Error::Unavailable)??;
        let ticket = Token::generate()?;
        let mut pending = self.resets.lock().map_err(|_| Error::Unavailable)?;
        let now = Instant::now();
        pending.retain(|_, reset| reset.expires > now);
        if pending.len() >= PENDING_RESETS {
            return Err(Error::Busy);
        }
        pending.insert(
            ticket.digest(),
            PendingReset {
                expires: now + Duration::from_secs(60),
                credential,
                digest,
                surface,
            },
        );
        Ok(Response::ResetChallenge {
            attempt: ticket,
            identity,
            registration,
            recovery,
        })
    }

    async fn reset_finish(
        &self,
        attempt: &Token,
        record: &[u8],
        wrapped: &[u8],
        vault: &ResetVault,
        surface: Surface,
    ) -> Result<Response, Error> {
        // Consumption precedes every check.
        let pending = self
            .resets
            .lock()
            .map_err(|_| Error::Unavailable)?
            .remove(&attempt.digest())
            .ok_or(Error::Rejected)?;
        if pending.expires <= Instant::now() || pending.surface != surface {
            return Err(Error::Rejected);
        }
        self.store
            .finish_reset(&pending.credential, &pending.digest, record, wrapped, vault)
            .await?;
        Ok(Response::Done)
    }
}
