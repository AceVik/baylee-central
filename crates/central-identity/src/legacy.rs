//! The legacy store's sign-in and move (`docs/native-login.md` §"Legacy
//! login"), served when the service has a legacy store
//! ([`IdentityService::with_legacy`]): the one path on which Central verifies
//! a password itself. It holds the password only for one Argon2 derivation on
//! a worker, compares in constant time and forgets it; nothing here logs.
//!
//! Every refusal before the password is proven is the same `Rejected` after
//! the same work: an unknown, closed, locked-out or registered name derives
//! against a fixed hash under the same parameters, and counts a failure
//! against a row that is not there.
use super::{Error, IdentityService, Response, Surface, Token};
use central_api::{
    admin::AdminAnswer,
    legacy::{LegacyPassword, LegacyStore as Listing},
    wire::Request,
};
use central_crypto::legacy::{HASH_BYTES, SALT_BYTES, derive};
use std::time::Instant;
use subtle::ConstantTimeEq;

/// Derived against for a name with no usable row: no password has this
/// output under this salt that anyone knows, and it costs what a real one does.
const DUMMY_SALT: [u8; SALT_BYTES] = *b"central/legacy/\0";
const DUMMY_HASH: [u8; HASH_BYTES] = [0; HASH_BYTES];

impl IdentityService {
    /// `LegacyLogin`: prove the old password, then reserve the account's
    /// identity and hand it its source.
    pub(super) async fn legacy_login(
        &self,
        username: &str,
        password: LegacyPassword,
        surface: Surface,
    ) -> Result<Response, Error> {
        let legacy = self.legacy.as_ref().ok_or(Error::Rejected)?;
        if surface != Surface::Native || !password.bounded() {
            return Err(Error::Rejected);
        }
        self.limits
            .lock()
            .map_err(|_| Error::Unavailable)?
            .legacy(username, Instant::now())?;
        let mut candidate = legacy.candidate(username).await?;
        let salt = candidate
            .as_ref()
            .map_or(DUMMY_SALT, |candidate| candidate.salt);
        // Moved out, still zeroizing; the row needs no verifier after this.
        let expected = candidate
            .as_mut()
            .map(|candidate| std::mem::take(&mut candidate.hash));
        let permit = self
            .workers
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::Busy)?;
        let proven = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let derived = derive(password.expose(), &salt).map_err(|_| Error::Rejected)?;
            drop(password);
            let expected = expected.as_deref().unwrap_or(&DUMMY_HASH);
            Ok::<_, Error>(bool::from(derived.ct_eq(expected)))
        })
        .await
        .map_err(|_| Error::Unavailable)??;
        let Some(candidate) = candidate.filter(|_| proven) else {
            legacy.failed(username).await?;
            return Err(Error::Rejected);
        };
        legacy.succeeded(&candidate.legacy_id).await?;
        // The gateway's own refusals, after the proof: a forced change and a
        // ban in force move nothing (the operator sees them in the listing).
        if candidate.must_change_password || candidate.banned {
            return Err(Error::Rejected);
        }
        let ticket = Token::generate()?;
        // `Conflict` once the account enrolled: its own OPAQUE sign-in resumes it.
        let identity = self
            .store
            .reserve_legacy(
                &candidate.legacy_id,
                &candidate.username,
                candidate.display_name.as_deref(),
                candidate.tag,
                &ticket,
            )
            .await?;
        let snapshot = legacy
            .snapshot(&candidate.legacy_id)
            .await?
            .ok_or(Error::Conflict)?;
        Ok(Response::LegacyEnrollment {
            identity,
            ticket,
            gateway: self.gateway.clone(),
            snapshot,
        })
    }

    /// `LegacyExport` and `LegacyLink`, with the session of a moving
    /// account on the native listener; the dispatch checked the terms.
    pub(super) async fn legacy_request(
        &self,
        request: Request,
        bearer: Option<&Token>,
        surface: Surface,
    ) -> Result<Response, Error> {
        let legacy = self.legacy.as_ref().ok_or(Error::Rejected)?;
        if surface != Surface::Native {
            return Err(Error::Rejected);
        }
        let moving = self.store.legacy_move(surface.session(bearer)?).await?;
        let legacy_id = moving.legacy_id.ok_or(Error::Rejected)?;
        match request {
            Request::LegacyExport if !moving.closed => Ok(Response::LegacySnapshot {
                snapshot: legacy.snapshot(&legacy_id).await?.ok_or(Error::Rejected)?,
            }),
            Request::LegacyLink { digest } => {
                if !moving.closed {
                    if !moving.restored {
                        return Err(Error::Rejected);
                    }
                    // A row is there until the close or the deadline; after
                    // the deadline a restored account still closes.
                    if let Some(snapshot) = legacy.snapshot(&legacy_id).await?
                        && snapshot.digest != digest
                    {
                        return Err(Error::Conflict);
                    }
                    self.store.close_legacy(&legacy_id, moving.identity).await?;
                }
                // Again after the close, this deletes a row whose deletion
                // was lost; the sweep would too.
                legacy.remove(&legacy_id).await?;
                Ok(Response::Done)
            }
            _ => Err(Error::Rejected),
        }
    }

    /// `AdminRequest::LegacyAccounts`: the deadline and what is left, with
    /// each move's progress, for an administrator only.
    pub(super) async fn admin_legacy(&self, cookie: &Token) -> Result<AdminAnswer, Error> {
        let legacy = self.legacy.as_ref().ok_or(Error::Rejected)?;
        self.store.require_administrator(cookie).await?;
        let (deadline, mut accounts) = legacy.listing().await?;
        let ids: Vec<String> = accounts
            .iter()
            .map(|account| account.legacy_id.clone())
            .collect();
        let progress = self.store.legacy_progress(&ids).await?;
        for (account, progress) in accounts.iter_mut().zip(progress) {
            account.progress = progress;
        }
        Ok(AdminAnswer::LegacyAccounts {
            store: Listing { deadline, accounts },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dummy_costs_what_a_real_hash_costs_and_proves_nothing() {
        // Same parameters, so the same work; and no password of the test's
        // makes the all-zero output.
        let derived = derive(b"synthetic", &DUMMY_SALT).unwrap();
        assert!(!bool::from(derived.ct_eq(&DUMMY_HASH)));
        assert_eq!(derived.len(), HASH_BYTES);
    }
}
