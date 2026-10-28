//! Deleting the session's account, and exporting what Central stores about
//! it (`docs/account-deletion.md`), on either listener and without Central's
//! terms: a person who will not accept new terms may still leave, and may
//! still see their data. A stolen session token alone deletes nothing: the
//! deletion carries a fresh OPAQUE proof of the password, bound to the
//! session that started it and charged to the account's login budget.
use super::{Error, IdentityService, Pending, Purpose, Request, Response, Surface, Token};
use central_crypto::opaque::server::ServerKeys;
use std::time::{Duration, Instant};
use subtle::ConstantTimeEq;

/// What a deletion's start spends of the account's login budget: KE2's one
/// OPRF evaluation, as a sign-in.
const DELETE_UNITS: u32 = 1;

impl IdentityService {
    /// The export and the two steps of a deletion.
    pub(super) async fn lifecycle_request(
        &self,
        request: Request,
        bearer: Option<&Token>,
        surface: Surface,
    ) -> Result<Response, Error> {
        match request {
            Request::AccountExport => {
                let export = self.store.account_export(surface.session(bearer)?).await?;
                Ok(Response::AccountExport {
                    export: Box::new(export),
                })
            }
            Request::DeleteAccountStart { login } => {
                self.delete_start(login, bearer, surface).await
            }
            Request::DeleteAccountFinish { attempt, proof } => {
                self.delete_finish(&attempt, proof, bearer, surface).await
            }
            _ => Err(Error::Rejected),
        }
    }

    /// KE2 for the password of the session's own account and epoch, with the
    /// gateways the deletion would take.
    async fn delete_start(
        &self,
        login: Vec<u8>,
        bearer: Option<&Token>,
        surface: Surface,
    ) -> Result<Response, Error> {
        let session = surface.session(bearer)?;
        let (credential, gateways) = self.store.deletion_start(session).await?;
        let identity = credential.identity;
        let context = self.context(identity)?;
        // The login budget bounds guesses at the password, also with a
        // stolen session: KE2 is one evaluation of the account's OPRF key.
        self.limits
            .lock()
            .map_err(|_| Error::Unavailable)?
            .account(identity.account, Instant::now(), DELETE_UNITS)?;
        let permit = self
            .workers
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::Busy)?;
        self.prune_pending()?;
        let keys = self.keys.clone();
        let (attempt, login, credential) = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let record = ServerKeys::enrollment_record(&credential.record)
                .map_err(|_| Error::Unavailable)?;
            let (attempt, login) = keys
                .start_login(context, Some(&record), &login)
                .map_err(|_| Error::Rejected)?;
            Ok::<_, Error>((attempt, login, credential))
        })
        .await
        .map_err(|_| Error::Unavailable)??;
        let ticket = Token::generate()?;
        let started_by = bearer.ok_or(Error::Rejected)?.digest();
        let mut pending = self.pending.lock().map_err(|_| Error::Unavailable)?;
        if pending.len() >= 128 {
            return Err(Error::Busy);
        }
        pending.insert(
            ticket.digest(),
            Pending {
                expires: Instant::now() + Duration::from_secs(60),
                attempt,
                credential: Some(credential),
                surface,
                purpose: Purpose::DeleteAccount(started_by),
            },
        );
        Ok(Response::DeleteChallenge {
            attempt: ticket,
            login,
            gateways,
        })
    }

    /// Verify the proof of the password, then delete the account.
    async fn delete_finish(
        &self,
        attempt: &Token,
        proof: Vec<u8>,
        bearer: Option<&Token>,
        surface: Surface,
    ) -> Result<Response, Error> {
        // Consumption precedes every check, as for a sign-in.
        let pending = self
            .pending
            .lock()
            .map_err(|_| Error::Unavailable)?
            .remove(&attempt.digest())
            .ok_or(Error::Rejected)?;
        let presented = bearer.ok_or(Error::Rejected)?;
        let Purpose::DeleteAccount(started_by) = pending.purpose else {
            return Err(Error::Rejected);
        };
        // Finished on the listener and with the session that started it.
        if pending.expires <= Instant::now()
            || pending.surface != surface
            || !bool::from(presented.digest().ct_eq(&started_by))
        {
            return Err(Error::Rejected);
        }
        let permit = self
            .workers
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::Busy)?;
        let credential = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            pending
                .attempt
                .finish(&proof)
                .map_err(|_| Error::Rejected)?;
            pending.credential.ok_or(Error::Rejected)
        })
        .await
        .map_err(|_| Error::Unavailable)??;
        self.store
            .delete_account(surface.session(Some(presented))?, &credential)
            .await?;
        Ok(Response::Done)
    }
}
