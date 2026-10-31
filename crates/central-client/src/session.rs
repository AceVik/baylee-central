use super::{
    Document, Endpoint, Error, Identity, MasterKey, ObjectContext, Request, Response, Token,
    Transport, context,
};
use zeroize::Zeroizing;

/// An authenticated native account. Tokens and keys stay private and cannot be cloned or printed.
pub struct Session {
    pub(crate) endpoint: Endpoint,
    pub(crate) identity: Identity,
    pub(crate) token: Token,
    pub(crate) master: MasterKey,
    revision: u64,
    document: Document,
    confirmed: bool,
}

impl Session {
    pub(super) fn new(
        endpoint: Endpoint,
        identity: Identity,
        token: Token,
        master: MasterKey,
    ) -> Self {
        Self {
            endpoint,
            identity,
            token,
            master,
            revision: 0,
            document: Zeroizing::new(Vec::new()),
            confirmed: false,
        }
    }
    pub(crate) fn same_endpoint(&self, transport: &impl Transport) -> Result<(), Error> {
        if transport.endpoint() != &self.endpoint {
            return Err(Error::Rejected);
        }
        Ok(())
    }
    fn object(&self, revision: u64) -> Result<ObjectContext, Error> {
        ObjectContext::new(
            context(&self.endpoint, self.identity)?,
            self.identity.account,
            revision,
        )
        .map_err(|_| Error::Rejected)
    }

    /// Immutable account metadata; not an authorization credential.
    #[must_use]
    pub const fn identity(&self) -> Identity {
        self.identity
    }

    /// The key of this device's local chat vault whose 32-byte scope is
    /// `scope` (Baylee's `password.vault`), derived one way from the master
    /// key (`docs/crypto-protocol.md` §"Derived from the master key"), so the
    /// vault opens without the password. The master key stays inside.
    ///
    /// # Errors
    /// An identity this endpoint cannot bind (never for a live session).
    pub fn local_vault_key(&self, scope: &[u8; 32]) -> Result<Zeroizing<[u8; 32]>, Error> {
        let context = context(&self.endpoint, self.identity)?;
        self.master
            .local_vault_key(context, scope)
            .map(central_crypto::vault::LocalVaultKey::into_bytes)
            .map_err(|_| Error::Rejected)
    }

    /// Check whether this is still the account's single active session. Never
    /// renews expiry ([`Session::renew`] does).
    ///
    /// # Errors
    /// Rejects superseded/revoked sessions, identity substitution and transport failure.
    pub fn check_active(&self, transport: &impl Transport) -> Result<(), Error> {
        self.migration_closed(transport).map(|_| ())
    }

    /// Check live session status and whether legacy authorization has been permanently retired.
    /// Completed accounts no longer need the legacy gateway online during ordinary login.
    ///
    /// # Errors
    /// Rejects revoked sessions, wrong identity and unavailable transport; never authorizes fallback.
    pub fn migration_closed(&self, transport: &impl Transport) -> Result<bool, Error> {
        self.same_endpoint(transport)?;
        match transport.exchange(&Request::SessionStatus, Some(&self.token))? {
            Response::SessionStatus {
                identity,
                expires_at,
                migration_closed,
            } if identity.account == self.identity.account
                && identity.epoch == self.identity.epoch
                && expires_at > 0 =>
            {
                Ok(migration_closed)
            }
            _ => Err(Error::Rejected),
        }
    }

    /// Renew this session: it expires 30 minutes from now, never later than 12
    /// hours after the password sign-in, and the gateway leases delegated from
    /// it follow. Returns the new absolute Unix-seconds expiry. Renew well
    /// before it; [`Session::check_active`] still says whether it lives.
    ///
    /// # Errors
    /// Rejects an expired, superseded or revoked session (and a Central that
    /// does not renew yet) and reports transport failure; a failed renewal
    /// ends nothing by itself.
    pub fn renew(&self, transport: &impl Transport) -> Result<i64, Error> {
        self.same_endpoint(transport)?;
        match transport.exchange(&Request::RenewSession, Some(&self.token))? {
            Response::Renewed { expires_at } if expires_at > 0 => Ok(expires_at),
            _ => Err(Error::Rejected),
        }
    }

    /// Successfully decrypted document. The application must validate its own version/schema.
    #[must_use]
    pub fn document(&self) -> &[u8] {
        &self.document
    }

    /// Last successfully authenticated document revision.
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    /// Refresh with account/revision authentication; retain the prior copy on every failure.
    ///
    /// # Errors
    /// Rejects transport changes, unavailable/corrupt content or a revision older than this device saw.
    pub fn refresh(&mut self, transport: &impl Transport) -> Result<(), Error> {
        self.same_endpoint(transport)?;
        let Response::Document { revision, document } =
            transport.exchange(&Request::ReadDocument, Some(&self.token))?
        else {
            return Err(Error::Rejected);
        };
        if revision < self.revision {
            return Err(Error::Conflict);
        }
        let plaintext = self
            .master
            .open_object(self.object(revision)?, &document)
            .map_err(|_| Error::Rejected)?;
        self.document = plaintext;
        self.revision = revision;
        Ok(())
    }

    /// Confirm only after application validation/restoration succeeds. This enables bridge cutover.
    ///
    /// # Errors
    /// Rejects changed revisions, wrong endpoint, expired sessions or unavailable storage.
    pub fn confirm_restored(&mut self, transport: &impl Transport) -> Result<(), Error> {
        self.same_endpoint(transport)?;
        match transport.exchange(
            &Request::ConfirmRestore {
                revision: self.revision,
            },
            Some(&self.token),
        )? {
            Response::Done => {
                self.confirmed = true;
                Ok(())
            }
            _ => Err(Error::Rejected),
        }
    }

    /// Encrypt and atomically save a new application-validated document.
    /// On a lost response, refresh before retrying; never blindly overwrite another device's data.
    ///
    /// # Errors
    /// Rejects oversized input, exhausted revisions, concurrent edits or transport/storage failure.
    pub fn save(&mut self, transport: &impl Transport, plaintext: &[u8]) -> Result<(), Error> {
        self.same_endpoint(transport)?;
        let revision = self.revision.checked_add(1).ok_or(Error::Rejected)?;
        let document = self
            .master
            .seal_object(self.object(revision)?, plaintext)
            .map_err(|_| Error::Rejected)?;
        match transport.exchange(
            &Request::WriteDocument {
                expected_revision: self.revision,
                document,
            },
            Some(&self.token),
        )? {
            Response::Done => {
                self.revision = revision;
                self.document = Zeroizing::new(plaintext.to_vec());
                Ok(())
            }
            _ => Err(Error::Rejected),
        }
    }

    /// Issue a one-use gateway capability after restoration, without exposing the central session.
    ///
    /// # Errors
    /// Rejects unsupported gateways, unconfirmed restoration, expired sessions or transport failure.
    pub fn gateway_ticket(
        &self,
        transport: &impl Transport,
        gateway: String,
    ) -> Result<Token, Error> {
        self.gateway_capability(transport, gateway, central_api::wire::TicketPurpose::Admit)
    }

    /// Issue a one-use, audience- and action-bound gateway capability after restoration.
    ///
    /// # Errors
    /// Rejects unsupported gateways, unconfirmed restoration, expiry or transport failure.
    pub fn gateway_capability(
        &self,
        transport: &impl Transport,
        gateway: String,
        purpose: central_api::wire::TicketPurpose,
    ) -> Result<Token, Error> {
        self.same_endpoint(transport)?;
        if !self.confirmed {
            return Err(Error::Rejected);
        }
        match transport.exchange(
            &Request::GatewayTicket { gateway, purpose },
            Some(&self.token),
        )? {
            Response::GatewayTicket { ticket } => Ok(ticket),
            _ => Err(Error::Rejected),
        }
    }

    /// Revoke the session remotely. Callers drop this SDK object afterward to wipe local secrets.
    ///
    /// # Errors
    /// Reports failure to confirm revocation; local disposal still remains available.
    pub fn logout(&self, transport: &impl Transport) -> Result<(), Error> {
        self.same_endpoint(transport)?;
        match transport.exchange(&Request::Logout, Some(&self.token))? {
            Response::Done => Ok(()),
            _ => Err(Error::Rejected),
        }
    }
}
