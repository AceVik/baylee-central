//! Deleting the account (`docs/account-deletion.md`, GDPR Art. 17). The
//! deletion proves the password afresh, so a stolen session token alone
//! deletes nothing, and it comes in two steps, so the application can show
//! what goes with the account before anything is sent that cannot be undone.
use super::{Error, Request, Response, Session, Token, Transport, context};
use central_api::admin::OwnedGateway;
use central_crypto::opaque::Login;

/// The most gateways one account owns (`docs/gateway-registry.md`).
const OWNED: usize = 8;

/// A deletion whose password is proven and which waits for the person's
/// confirmation. Central keeps it for 60 seconds after its start.
pub struct Deletion {
    attempt: Token,
    proof: Vec<u8>,
    gateways: Vec<OwnedGateway>,
}

impl Deletion {
    /// The gateways the account owns, which are deleted with it: their
    /// sign-in through Central ends for every player. Show them before
    /// [`Session::delete_account`].
    #[must_use]
    pub fn gateways(&self) -> &[OwnedGateway] {
        &self.gateways
    }
}

impl Session {
    /// Prove `password` for deleting the account, without deleting anything
    /// yet. Central's terms need not be accepted. The account must have
    /// finished moving and hold no administrator role.
    ///
    /// # Errors
    /// `Rejected` for a wrong password, an ended session, an unfinished move
    /// or an administrator; `Busy` once the account's login budget is spent
    /// (a start counts as a sign-in).
    pub fn start_deletion(
        &self,
        transport: &impl Transport,
        password: String,
    ) -> Result<Deletion, Error> {
        self.same_endpoint(transport)?;
        let context = context(&self.endpoint, self.identity)?;
        let (login, message) = Login::start(password, context).map_err(|_| Error::Rejected)?;
        let Response::DeleteChallenge {
            attempt,
            login: challenge,
            gateways,
        } = transport.exchange(
            &Request::DeleteAccountStart { login: message },
            Some(&self.token),
        )?
        else {
            return Err(Error::Rejected);
        };
        if gateways.len() > OWNED {
            return Err(Error::Rejected);
        }
        // A wrong password fails here, and the attempt simply expires.
        let proof = login.finish(&challenge).map_err(|_| Error::Rejected)?;
        Ok(Deletion {
            attempt,
            proof: proof.finalization,
            gateways,
        })
    }

    /// Delete the account, at once and for good: its documents, objects,
    /// sessions on every device and the website, and the gateways it owns.
    /// Drop this session afterwards.
    ///
    /// # Errors
    /// `Rejected` once the deletion's 60 seconds have passed or when it was
    /// started with another session, and for an ended session; nothing was
    /// deleted then while [`Session::check_active`] still succeeds. A lost
    /// answer is `Unavailable` while this session lives (nothing was
    /// deleted; start again) and `Rejected` once it has ended: the account
    /// was deleted, or another sign-in replaced the session; signing in tells
    /// which.
    pub fn delete_account(
        &self,
        transport: &impl Transport,
        deletion: Deletion,
    ) -> Result<(), Error> {
        self.same_endpoint(transport)?;
        match transport.exchange(
            &Request::DeleteAccountFinish {
                attempt: deletion.attempt,
                proof: deletion.proof,
            },
            Some(&self.token),
        ) {
            Ok(Response::Done) => Ok(()),
            Ok(_) => Err(Error::Rejected),
            // The deletion ends this session: while it lives, nothing was deleted.
            Err(Error::Unavailable) => Err(match self.check_active(transport) {
                Ok(()) | Err(Error::Unavailable) => Error::Unavailable,
                Err(_) => Error::Rejected,
            }),
            Err(error) => Err(error),
        }
    }
}
