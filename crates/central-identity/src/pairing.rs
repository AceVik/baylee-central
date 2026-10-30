//! Pairing a signed-in website's master key to Baylee's client, and the
//! account's key check (`docs/web-accounts.md` §"Pairing"; Baylee
//! `docs/proposals/deep-links.md` §"Protocol specification").
//!
//! The page starts a pairing with its cookie and puts the handle and a
//! one-time secret into a link; the client claims it on the native listener
//! with its public key, a verifier's challenge and a MAC under that secret;
//! the page checks the MAC itself and approves with the master key sealed to
//! the client and a proof under the key check, which Central verifies here;
//! the client redeems once with the verifier and receives a native session
//! together with the sealed key. Central sees neither the secret nor the key.
use super::{Error, IdentityService, Request, Response, Surface, Token};
use central_crypto::pairing::{self, SEALED_LEN};
use central_crypto::vault::KeyCheck;

/// Exactly 32 bytes, or refused.
fn exact(bytes: &[u8]) -> Result<[u8; 32], Error> {
    bytes.try_into().map_err(|_| Error::Rejected)
}

impl IdentityService {
    /// The key check's and the pairing's requests.
    pub(super) async fn pairing_request(
        &self,
        request: Request,
        bearer: Option<&Token>,
        surface: Surface,
    ) -> Result<Response, Error> {
        match request {
            Request::KeyCheckStatus => Ok(Response::KeyCheck {
                set: self
                    .store
                    .key_check(surface.session(bearer)?)
                    .await?
                    .is_some(),
            }),
            Request::SetKeyCheck { check } => {
                let check = exact(&check)?;
                self.store
                    .set_key_check(surface.session(bearer)?, &check)
                    .await?;
                Ok(Response::Done)
            }
            Request::PairStart => {
                let browser = bearer.ok_or(Error::Rejected)?;
                let handle = Token::generate()?;
                let expires_at = self.store.start_pairing(browser, &handle.digest()).await?;
                Ok(Response::PairStarted { handle, expires_at })
            }
            Request::PairStatus { handle } => {
                let browser = bearer.ok_or(Error::Rejected)?;
                Ok(
                    match self.store.pairing_claim(browser, &handle.digest()).await? {
                        Some(claim) => Response::PairClaimed {
                            public_key: claim.public_key.to_vec(),
                            challenge: claim.challenge.to_vec(),
                            mac: claim.mac.to_vec(),
                        },
                        None => Response::Pending,
                    },
                )
            }
            Request::PairApprove {
                handle,
                sealed,
                proof,
            } => {
                self.approve(bearer.ok_or(Error::Rejected)?, &handle, &sealed, &proof)
                    .await?;
                Ok(Response::Done)
            }
            Request::PairCancel { handle } => {
                let browser = bearer.ok_or(Error::Rejected)?;
                self.store.cancel_pairing(browser, &handle.digest()).await?;
                Ok(Response::Done)
            }
            Request::PairClaim {
                handle,
                public_key,
                challenge,
                mac,
            } => {
                let expires_at = self
                    .store
                    .claim_pairing(
                        &handle.digest(),
                        &exact(&public_key)?,
                        &exact(&challenge)?,
                        &exact(&mac)?,
                    )
                    .await?;
                Ok(Response::PairAccepted { expires_at })
            }
            Request::PairPoll { handle } => {
                Ok(if self.store.poll_pairing(&handle.digest()).await? {
                    Response::Approved
                } else {
                    Response::Pending
                })
            }
            Request::PairRedeem { handle, verifier } => {
                let challenge = pairing::challenge_of(&exact(&verifier)?);
                let token = Token::generate()?;
                let redeemed = self
                    .store
                    .redeem_pairing(&handle.digest(), &challenge, &token)
                    .await?;
                Ok(Response::PairSession {
                    token,
                    identity: redeemed.identity,
                    username: redeemed.username,
                    sealed: redeemed.sealed,
                    expires_at: redeemed.expires_at,
                })
            }
            _ => Err(Error::Rejected),
        }
    }

    /// The page's approval, with the proof under the stored key check: a
    /// cookie alone cannot make it, and it covers this handle, key, challenge
    /// and seal.
    async fn approve(
        &self,
        browser: &Token,
        handle: &Token,
        sealed: &[u8],
        proof: &[u8],
    ) -> Result<(), Error> {
        let sealed: [u8; SEALED_LEN] = sealed.try_into().map_err(|_| Error::Rejected)?;
        let approval = self
            .store
            .pairing_for_approval(browser, &handle.digest())
            .await?;
        let check = KeyCheck::from_bytes(&approval.key_check).map_err(|_| Error::Unavailable)?;
        if !pairing::approval_ok(
            &check,
            &handle.to_secret_bytes(),
            &approval.claim.public_key,
            &approval.claim.challenge,
            &sealed,
            proof,
        ) {
            return Err(Error::Rejected);
        }
        self.store
            .approve_pairing(browser, &handle.digest(), &sealed)
            .await
    }
}
