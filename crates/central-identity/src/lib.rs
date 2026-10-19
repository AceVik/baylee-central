//! Native identity orchestration; no transport, logging or plaintext password handling.
#![forbid(unsafe_code)]

use central_api::{
    Endpoint, Error, Token,
    wire::{Discovery, Identity, Request, Response},
};
use central_crypto::{
    Context,
    opaque::server::{Attempt, ServerKeys},
};
use central_postgres::{AccountStore, Credential};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use subtle::ConstantTimeEq;
use tokio::sync::Semaphore;

mod dispatch;
mod gateways;
mod limits;
mod objects;

struct Pending {
    expires: Instant,
    attempt: Attempt,
    credential: Option<Credential>,
}

/// Shared service state. Pending challenges are bounded, expiring and single-use.
pub struct IdentityService {
    store: AccountStore,
    issuer: [u8; 32],
    keys: Arc<ServerKeys>,
    bridge_digest: [u8; 32],
    gateway: String,
    legacy_discovery: bool,
    pending: Mutex<BTreeMap<[u8; 32], Pending>>,
    limits: Mutex<limits::Limits>,
    workers: Arc<Semaphore>,
}

impl IdentityService {
    /// Compose a service from an already validated store and restored deployment secrets.
    /// The bridge credential is separately configured; no user session can substitute for it.
    /// Development deliberately does not advertise the live migration fallback.
    ///
    /// # Errors
    /// Rejects an invalid development gateway or live gateway other than the official bridge.
    pub fn new(
        store: AccountStore,
        endpoint: &Endpoint,
        keys: ServerKeys,
        bridge: &Token,
        gateway: &str,
    ) -> Result<Self, Error> {
        if endpoint.legacy_origin().is_some() {
            if gateway != central_api::LEGACY_ORIGIN {
                return Err(Error::Rejected);
            }
        } else {
            Endpoint::development(gateway)?;
        }
        Ok(Self {
            store,
            issuer: endpoint.issuer(),
            keys: Arc::new(keys),
            bridge_digest: bridge.digest(),
            gateway: gateway.into(),
            legacy_discovery: endpoint.legacy_origin().is_some(),
            pending: Mutex::default(),
            limits: Mutex::default(),
            workers: Arc::new(Semaphore::new(4)),
        })
    }

    fn context(&self, identity: Identity) -> Result<Context, Error> {
        Context::new(self.issuer, identity.account, identity.epoch).map_err(|_| Error::Rejected)
    }

    fn bridge(&self, bearer: Option<&Token>) -> Result<(), Error> {
        let digest = bearer.ok_or(Error::Rejected)?.digest();
        if !bool::from(digest.ct_eq(&self.bridge_digest)) {
            return Err(Error::Rejected);
        }
        Ok(())
    }

    async fn start(&self, identity: Identity, message: Vec<u8>) -> Result<Response, Error> {
        let context = self.context(identity)?;
        self.limits
            .lock()
            .map_err(|_| Error::Unavailable)?
            .account(identity.account, Instant::now())?;
        let permit = self
            .workers
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::Busy)?;
        // Capacity is checked before a query/curve operation and again before insertion.
        self.prune_pending()?;
        let credential = self.store.credential(identity).await?;
        let keys = self.keys.clone();
        let (attempt, message, credential) = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let record = credential
                .as_ref()
                .map(|c| ServerKeys::enrollment_record(&c.record))
                .transpose()
                .map_err(|_| Error::Unavailable)?;
            let (attempt, message) = keys
                .start_login(context, record.as_ref(), &message)
                .map_err(|_| Error::Rejected)?;
            Ok::<_, Error>((attempt, message, credential))
        })
        .await
        .map_err(|_| Error::Unavailable)??;
        let ticket = Token::generate()?;
        let mut pending = self.pending.lock().map_err(|_| Error::Unavailable)?;
        if pending.len() >= 128 {
            return Err(Error::Busy);
        }
        pending.insert(
            ticket.digest(),
            Pending {
                expires: Instant::now() + Duration::from_secs(60),
                attempt,
                credential,
            },
        );
        Ok(Response::LoginChallenge {
            attempt: ticket,
            message,
        })
    }

    fn prune_pending(&self) -> Result<(), Error> {
        let mut pending = self.pending.lock().map_err(|_| Error::Unavailable)?;
        let now = Instant::now();
        pending.retain(|_, value| value.expires > now);
        if pending.len() >= 128 {
            return Err(Error::Busy);
        }
        Ok(())
    }

    async fn finish(&self, ticket: &Token, message: Vec<u8>) -> Result<Response, Error> {
        // Consumption precedes validation, including invalid message size/proofs.
        let pending = self
            .pending
            .lock()
            .map_err(|_| Error::Unavailable)?
            .remove(&ticket.digest())
            .ok_or(Error::Rejected)?;
        if pending.expires <= Instant::now() {
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
                .finish(&message)
                .map_err(|_| Error::Rejected)?;
            pending.credential.ok_or(Error::Rejected)
        })
        .await
        .map_err(|_| Error::Unavailable)??;
        let token = Token::generate()?;
        let wrapped_master = self.store.issue_session(&credential, &token).await?;
        Ok(Response::Session {
            token,
            identity: credential.identity,
            wrapped_master,
        })
    }

    /// Process a typed native command. Authentication is checked for each operation.
    ///
    /// # Errors
    /// Returns generic authorization, capacity, conflict or availability errors only.
    pub async fn handle(
        &self,
        request: Request,
        bearer: Option<&Token>,
    ) -> Result<Response, Error> {
        self.limits
            .lock()
            .map_err(|_| Error::Unavailable)?
            .request(&request, bearer, Instant::now())?;
        self.dispatch(request, bearer).await
    }
}
