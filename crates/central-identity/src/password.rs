//! Password change under a live session, on either listener
//! (`docs/native-login.md` §"Password change"). A stolen session token alone
//! changes nothing: the change carries a fresh OPAQUE proof of the current
//! password, bound to the session that started it and charged to the
//! account's login budget.
use super::{
    BrowserReply, Error, IdentityService, Pending, Place, Purpose, Request, Response, Surface,
    Token, limits, signed_in,
};
use central_crypto::opaque::server::ServerKeys;
use central_mail::Mail;
use std::time::{Duration, Instant};
use subtle::ConstantTimeEq;

/// What a change's start spends of the account's login budget: two OPRF
/// evaluations, KE2's and the registration response's.
const CHANGE_UNITS: u32 = 2;

impl IdentityService {
    /// KE2 for the current password and the registration response for the
    /// new one, both for the session's own account and epoch.
    pub(super) async fn change_password_start(
        &self,
        request: Request,
        bearer: Option<&Token>,
        surface: Surface,
        place: Option<Place>,
    ) -> Result<Response, Error> {
        let Request::ChangePasswordStart {
            login,
            registration,
        } = request
        else {
            return Err(Error::Rejected);
        };
        let session = surface.session(bearer)?;
        let credential = self.store.password_credential(session).await?;
        let identity = credential.identity;
        let context = self.context(identity)?;
        let permit = self
            .workers
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::Busy)?;
        self.prune_pending()?;
        // The login budget bounds guesses at the current password, also with
        // a stolen session. The registration response is a second evaluation
        // of the account's OPRF key: with it a guess unmasks KE2's envelope as
        // KE2's own evaluation does, so a start costs two sign-ins and a
        // session guesses no faster than anyone can without one. Counted
        // once a worker is held, and given back by the proof.
        let guess = self
            .limits
            .lock()
            .map_err(|_| Error::Unavailable)?
            .guesses
            .guess(identity.account, true, place, Instant::now(), CHANGE_UNITS)?;
        let keys = self.keys.clone();
        let (attempt, login, registration, credential) = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let record = ServerKeys::enrollment_record(&credential.record)
                .map_err(|_| Error::Unavailable)?;
            let (attempt, login) = keys
                .start_login(context, Some(&record), &login)
                .map_err(|_| Error::Rejected)?;
            let registration = keys
                .enrollment_response(context, &registration)
                .map_err(|_| Error::Rejected)?;
            Ok::<_, Error>((attempt, login, registration, credential))
        })
        .await
        .map_err(|_| Error::Unavailable)??;
        let ticket = Token::generate()?;
        let started_by = bearer.ok_or(Error::Rejected)?.digest();
        let mut pending = self.pending.lock().map_err(|_| Error::Unavailable)?;
        if pending.len() >= limits::PENDING {
            return Err(Error::Busy);
        }
        pending.insert(
            ticket.digest(),
            Pending {
                expires: Instant::now() + Duration::from_secs(limits::PENDING_SECONDS),
                attempt,
                credential: Some(credential),
                surface,
                purpose: Purpose::ChangePassword(started_by),
                guess,
            },
        );
        Ok(Response::PasswordChallenge {
            attempt: ticket,
            login,
            registration,
        })
    }

    /// Verify the proof of the current password, then commit the new record
    /// and wrapper and answer the caller's next session, as a sign-in does.
    pub(super) async fn change_password_finish(
        &self,
        bearer: Option<&Token>,
        surface: Surface,
        attempt: &Token,
        proof: Vec<u8>,
        record: &[u8],
        wrapped: &[u8],
    ) -> Result<BrowserReply, Error> {
        // The worker first, as for a sign-in: without one the attempt stays.
        let permit = self
            .workers
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::Busy)?;
        // Consumption precedes every check, as for a sign-in.
        let pending = self
            .pending
            .lock()
            .map_err(|_| Error::Unavailable)?
            .remove(&attempt.digest())
            .ok_or(Error::Rejected)?;
        let presented = bearer.ok_or(Error::Rejected)?;
        let Purpose::ChangePassword(started_by) = pending.purpose else {
            return Err(Error::Rejected);
        };
        // Finished on the listener and with the session that started it.
        if pending.expires <= Instant::now()
            || pending.surface != surface
            || !bool::from(presented.digest().ct_eq(&started_by))
        {
            return Err(Error::Rejected);
        }
        let (credential, guess) = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            pending
                .attempt
                .finish(&proof)
                .map_err(|_| Error::Rejected)?;
            Ok::<_, Error>((pending.credential.ok_or(Error::Rejected)?, pending.guess))
        })
        .await
        .map_err(|_| Error::Unavailable)??;
        // The current password is proven, whatever the store says to the
        // new one.
        self.proved(&guess)?;
        let token = Token::generate()?;
        let done = self
            .store
            .change_password(
                surface.session(Some(presented))?,
                &credential,
                record,
                wrapped,
                surface.session(Some(&token))?,
            )
            .await;
        // The account's confirmed address is told, later.
        let issued =
            self.mails
                .committed(done, credential.identity.account, Mail::password_changed)?;
        // Whoever knew the old password has no place of the account's any
        // more; the one this change came from stays its own.
        self.changed(credential.identity.account, guess.place());
        Ok(signed_in(surface, credential.identity, issued, token))
    }
}
