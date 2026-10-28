use super::{Error, Request, Response, Session, Transport, context};
use central_crypto::opaque::{Enrollment, Login};

/// The fewest characters a new password may have, as the website asks
/// (OWASP ASVS 6.2.1). The OPAQUE primitive bounds it to 1024 bytes.
pub const MIN_PASSWORD_CHARS: usize = 8;

impl Session {
    /// Change the account's password: prove `current` with a fresh OPAQUE
    /// login, register `new`, and wrap this session's master key under it,
    /// so the documents, objects and recovery code stay as they are. Every
    /// session of the account ends, on every device and the website; this
    /// one continues under the new session Central answers. The account must
    /// have finished moving and accepted Central's terms in force.
    ///
    /// # Errors
    /// `Rejected` for a new password shorter than [`MIN_PASSWORD_CHARS`] or
    /// longer than 1024 bytes, for a wrong `current` (found before anything
    /// is committed) and for an ended session; `Busy` once the account's
    /// login budget is spent (a change counts as two sign-ins); `TermsRequired`. After `Rejected`,
    /// [`Session::check_active`] tells which: a live session means nothing
    /// changed; an ended one may have committed, so sign in again with
    /// `new` first. A lost answer is `Rejected` when this session has ended
    /// meanwhile, and `Unavailable` otherwise: a retry is then safe, since
    /// a change that did commit ended this session and its retry is refused.
    pub fn change_password(
        &mut self,
        transport: &impl Transport,
        current: String,
        new: String,
    ) -> Result<(), Error> {
        self.same_endpoint(transport)?;
        if new.chars().count() < MIN_PASSWORD_CHARS {
            return Err(Error::Rejected);
        }
        let context = context(&self.endpoint, self.identity)?;
        let (login, login_message) = Login::start(current, context).map_err(|_| Error::Rejected)?;
        let (enrollment, registration) =
            Enrollment::start(new, context).map_err(|_| Error::Rejected)?;
        let Response::PasswordChallenge {
            attempt,
            login: challenge,
            registration,
        } = transport.exchange(
            &Request::ChangePasswordStart {
                login: login_message,
                registration,
            },
            Some(&self.token),
        )?
        else {
            return Err(Error::Rejected);
        };
        // A wrong current password fails here, and the attempt simply expires.
        let proof = login.finish(&challenge).map_err(|_| Error::Rejected)?;
        let enrolled = enrollment
            .finish(&registration)
            .map_err(|_| Error::Rejected)?;
        let wrapped = enrolled
            .wrapping_key
            .wrap(&self.master)
            .map_err(|_| Error::Unavailable)?;
        let sent = wrapped.as_bytes().to_vec();
        let answer = transport.exchange(
            &Request::ChangePasswordFinish {
                attempt,
                proof: proof.finalization,
                record: enrolled.upload,
                wrapped_master: sent.clone(),
            },
            Some(&self.token),
        );
        let (token, identity, wrapped_master) = match answer {
            Ok(Response::Session {
                token,
                identity,
                wrapped_master,
            }) => (token, identity, wrapped_master),
            Ok(_) => return Err(Error::Rejected),
            // The commit ends this session: while it lives, nothing changed,
            // and once it has ended a retry is refused at its start.
            Err(Error::Unavailable) => {
                return Err(match self.check_active(transport) {
                    Ok(()) | Err(Error::Unavailable) => Error::Unavailable,
                    Err(_) => Error::Rejected,
                });
            }
            Err(error) => return Err(error),
        };
        if identity.account != self.identity.account
            || identity.epoch != self.identity.epoch
            || wrapped_master != sent
        {
            let _ = transport.exchange(&Request::Logout, Some(&token));
            return Err(Error::Rejected);
        }
        self.token = token;
        Ok(())
    }
}
