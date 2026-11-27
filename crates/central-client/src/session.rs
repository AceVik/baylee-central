use super::{
    Document, Endpoint, Error, Identity, MasterKey, ObjectContext, Request, Response, SignedToken,
    Transport, context,
};
use zeroize::Zeroizing;

/// An authenticated native account. Tokens and keys stay private and cannot be cloned or printed.
pub struct Session {
    pub(crate) endpoint: Endpoint,
    pub(crate) identity: Identity,
    pub(crate) token: SignedToken,
    /// None for an AI runner's session, which no password opened
    /// ([`crate::ai_runner_session`]).
    master: Option<MasterKey>,
    revision: u64,
    document: Document,
    confirmed: bool,
}

impl Session {
    pub(super) fn new(
        endpoint: Endpoint,
        identity: Identity,
        token: SignedToken,
        master: MasterKey,
    ) -> Self {
        Self {
            endpoint,
            identity,
            token,
            master: Some(master),
            revision: 0,
            document: Zeroizing::new(Vec::new()),
            confirmed: false,
        }
    }

    /// An AI runner's session: no master key, no document, and no restore
    /// confirmed, so every operation that needs one of them refuses.
    pub(crate) fn runner(endpoint: Endpoint, identity: Identity, token: SignedToken) -> Self {
        Self {
            endpoint,
            identity,
            token,
            master: None,
            revision: 0,
            document: Zeroizing::new(Vec::new()),
            confirmed: false,
        }
    }

    /// The master key, which a runner's session does not have. Every
    /// operation that needs it asks before it sends anything.
    pub(crate) fn master(&self) -> Result<&MasterKey, Error> {
        self.master.as_ref().ok_or(Error::Rejected)
    }

    /// Whether this is an AI runner's session, opened with a runner token
    /// ([`crate::ai_runner_session`]) and holding no master key.
    #[must_use]
    pub const fn is_runner(&self) -> bool {
        self.master.is_none()
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

    /// The stable account chat identity shared by browser and native clients.
    ///
    /// # Errors
    /// Refuses a runner's session, an invalid account context or failed key
    /// derivation.
    pub fn chat_identity(&self) -> Result<central_crypto::chat::DeviceSecrets, Error> {
        self.master()?
            .chat_identity(context(&self.endpoint, self.identity)?)
            .map_err(|_| Error::Rejected)
    }

    /// The key of this device's local chat vault whose 32-byte scope is
    /// `scope` (Baylee's `password.vault`), derived one way from the master
    /// key (`docs/crypto-protocol.md` §"Derived from the master key"), so the
    /// vault opens without the password. The master key stays inside.
    ///
    /// # Errors
    /// A runner's session, which has no master key, and an identity this
    /// endpoint cannot bind (never for a live session).
    pub fn local_vault_key(&self, scope: &[u8; 32]) -> Result<Zeroizing<[u8; 32]>, Error> {
        let master = self.master()?;
        let context = context(&self.endpoint, self.identity)?;
        master
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
        match transport.exchange(&Request::SessionStatus, Some((&self.token).into()))? {
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
        match transport.exchange(&Request::RenewSession, Some((&self.token).into()))? {
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
    /// Rejects a runner's session, transport changes, unavailable/corrupt content or a
    /// revision older than this device saw.
    pub fn refresh(&mut self, transport: &impl Transport) -> Result<(), Error> {
        self.same_endpoint(transport)?;
        self.master()?;
        let Response::Document { revision, document } =
            transport.exchange(&Request::ReadDocument, Some((&self.token).into()))?
        else {
            return Err(Error::Rejected);
        };
        if revision < self.revision {
            return Err(Error::Conflict);
        }
        let plaintext = self
            .master()?
            .open_object(self.object(revision)?, &document)
            .map_err(|_| Error::Rejected)?;
        self.document = plaintext;
        self.revision = revision;
        Ok(())
    }

    /// Confirm only after application validation/restoration succeeds. This enables bridge cutover.
    ///
    /// # Errors
    /// Rejects a runner's session, which restores no document, changed revisions, wrong
    /// endpoint, expired sessions or unavailable storage.
    pub fn confirm_restored(&mut self, transport: &impl Transport) -> Result<(), Error> {
        self.same_endpoint(transport)?;
        self.master()?;
        match transport.exchange(
            &Request::ConfirmRestore {
                revision: self.revision,
            },
            Some((&self.token).into()),
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
    /// Rejects a runner's session, oversized input, exhausted revisions, concurrent edits
    /// or transport/storage failure.
    pub fn save(&mut self, transport: &impl Transport, plaintext: &[u8]) -> Result<(), Error> {
        self.same_endpoint(transport)?;
        let revision = self.revision.checked_add(1).ok_or(Error::Rejected)?;
        let document = self
            .master()?
            .seal_object(self.object(revision)?, plaintext)
            .map_err(|_| Error::Rejected)?;
        match transport.exchange(
            &Request::WriteDocument {
                expected_revision: self.revision,
                document,
            },
            Some((&self.token).into()),
        )? {
            Response::Done => {
                self.revision = revision;
                self.document = Zeroizing::new(plaintext.to_vec());
                Ok(())
            }
            _ => Err(Error::Rejected),
        }
    }

    /// Ask for a token for one gateway after restoration, without exposing the Central token.
    /// The same call refreshes it. The answer is the token and its absolute expiry in Unix
    /// seconds; hand the token to that gateway and to no one else.
    ///
    /// # Errors
    /// Rejects unsupported gateways, unconfirmed restoration, expiry or transport failure.
    pub fn gateway_token(
        &self,
        transport: &impl Transport,
        gateway: String,
        purpose: central_api::wire::GatewayPurpose,
    ) -> Result<(SignedToken, i64), Error> {
        self.same_endpoint(transport)?;
        if !self.confirmed {
            return Err(Error::Rejected);
        }
        match transport.exchange(
            &Request::GatewayToken { gateway, purpose },
            Some((&self.token).into()),
        )? {
            Response::GatewayToken { token, expires_at } => Ok((token, expires_at)),
            _ => Err(Error::Rejected),
        }
    }

    /// Obtain scoped identity access after restoration. Only the returned trusted
    /// Tower origin may receive this credential; it is not a Central session.
    /// # Errors
    /// Unconfirmed restoration, disabled Tower, dead session or invalid destination.
    pub fn tower_token(
        &self,
        transport: &impl Transport,
    ) -> Result<central_api::tower_access::Grant, Error> {
        self.same_endpoint(transport)?;
        if !self.confirmed {
            return Err(Error::Rejected);
        }
        match transport.exchange(&Request::TowerToken {}, Some((&self.token).into()))? {
            Response::TowerToken { grant } => {
                central_api::gateway::validate_origin(
                    &grant.origin,
                    self.endpoint.legacy_origin().is_none(),
                )?;
                Ok(grant)
            }
            _ => Err(Error::Rejected),
        }
    }

    /// Revoke the session remotely. Callers drop this SDK object afterward to wipe local secrets.
    ///
    /// # Errors
    /// Reports failure to confirm revocation; local disposal still remains available.
    pub fn logout(&self, transport: &impl Transport) -> Result<(), Error> {
        self.same_endpoint(transport)?;
        match transport.exchange(&Request::Logout, Some((&self.token).into()))? {
            Response::Done => Ok(()),
            _ => Err(Error::Rejected),
        }
    }
}
