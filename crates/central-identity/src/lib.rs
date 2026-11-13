//! Identity orchestration for the native API and the website's browser listener;
//! no transport, logging or plaintext password handling.
#![forbid(unsafe_code)]

use central_api::terms::TermsSubject;
use central_api::{
    Endpoint, Error, Token,
    wire::{Discovery, Identity, Request, Response},
};
use central_crypto::{
    Context,
    opaque::server::{Attempt, ServerKeys},
};
use central_postgres::{AccountStore, Credential, LegacyStore, Session};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use subtle::ConstantTimeEq;
use tokio::sync::Semaphore;

mod admin;
mod challenge;
pub use challenge::{Agent, Seen};
mod deletion;
mod dispatch;
mod email;
pub use email::Delivery;
mod gateways;
mod guesses;
pub use guesses::Place;
mod legacy;
mod limits;
mod objects;
mod owner;
mod pairing;
mod password;
mod pictures;
pub use central_postgres::pictures::Encoding as PictureEncoding;
pub use pictures::{Admitted, SLOTS as PICTURE_SLOTS, UploadRefusal};
mod push;
pub use push::{GatewayDelivery, GatewayOutcome};
mod reports;
pub use reports::PartUpload;
mod reset;
mod terms;
mod tokens;
pub use tokens::{Presented, TokenKeys, credential_operation};

/// A mailer as the service knows one ([`IdentityService::with_mailer`]):
/// the deployed one is `central_mail::Client`, and a test puts its own in
/// its place.
pub mod mail {
    pub use central_mail::{Mail, Mailer, Outcome, What};
}

struct Pending {
    expires: Instant,
    attempt: Attempt,
    credential: Option<Credential>,
    surface: Surface,
    purpose: Purpose,
    /// The start as it was counted: a guess, until the proof shows it was
    /// none (`guesses.rs`).
    guess: guesses::Guess,
}

/// What a pending OPAQUE login proves the password for.
#[derive(Clone, Copy)]
enum Purpose {
    /// A sign-in, finished by `LoginFinish`.
    Login,
    /// A password change, finished by `ChangePasswordFinish` with the session
    /// whose token has this digest, and with no other.
    ChangePassword([u8; 32]),
    /// An account's deletion, finished by `DeleteAccountFinish` with the
    /// session whose token has this digest, and with no other.
    DeleteAccount([u8; 32]),
}

/// The listener a request arrived on.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Surface {
    /// `/v1/native`: bearer sessions, every operation.
    Native,
    /// The website's `/api/v1`: cookie sessions, [`browser_operation`] only.
    Browser,
}

impl Surface {
    /// The presented token as a session of this surface's kind.
    fn session(self, bearer: Option<&Token>) -> Result<Session<'_>, Error> {
        let token = bearer.ok_or(Error::Rejected)?;
        Ok(match self {
            Self::Native => Session::Native(token),
            Self::Browser => Session::Browser(token),
        })
    }
}

/// What a request on the browser listener produced.
pub struct BrowserReply {
    /// The answer. It never carries a session token.
    pub response: Response,
    /// A browser session just issued, to be set as the `__Host-central` cookie
    /// and nowhere else.
    pub session: Option<Token>,
}

/// The operations the browser listener serves (`docs/web-accounts.md`); every
/// other one is refused before anything else happens. Terms are accepted there
/// for Central only; the public gateway directory and the public figures are
/// read there without a session. A gateway's pulse, like its heartbeat, is
/// the native route's. The match names every operation, so a new one does
/// not compile until it is decided here.
#[must_use]
pub fn browser_operation(request: &Request) -> bool {
    match request {
        Request::Notifications { .. }
        | Request::Social { .. }
        | Request::Discover { .. }
        | Request::LoginStart { .. }
        | Request::LoginFinish { .. }
        | Request::RegisterChallenge
        | Request::RegisterChallengeFor { .. }
        | Request::Register { .. }
        | Request::EnrollmentStart { .. }
        | Request::EnrollmentFinish { .. }
        | Request::SessionStatus
        | Request::RenewSession
        | Request::Logout
        | Request::Terms
        | Request::TermsStatus
        | Request::GatewayDirectory { .. }
        | Request::PublicFigures { .. }
        | Request::AccountSummary
        | Request::SetRecovery { .. }
        | Request::SetDisplayName { .. }
        | Request::ChangePasswordStart { .. }
        | Request::ChangePasswordFinish { .. }
        | Request::Admin { .. }
        | Request::Owner { .. }
        | Request::Email { .. }
        | Request::ResetMail { .. }
        | Request::ResetStart { .. }
        | Request::ResetFinish { .. }
        | Request::AccountExport
        | Request::DeleteAccountStart { .. }
        | Request::DeleteAccountFinish { .. }
        | Request::KeyCheckStatus
        | Request::SetKeyCheck { .. }
        | Request::PairStart
        | Request::PairStatus { .. }
        | Request::PairApprove { .. }
        | Request::PairCancel { .. }
        | Request::Picture { .. } => true,
        Request::AcceptTerms { subject, .. } => matches!(subject, TermsSubject::Central),
        Request::CreateObject
        | Request::ReadObject { .. }
        | Request::ListObjects { .. }
        | Request::WriteObject { .. }
        | Request::DeleteObject { .. }
        | Request::LegacyInvalidate { .. }
        | Request::LegacyClose { .. }
        | Request::LegacyProfile { .. }
        | Request::RegisterGateway { .. }
        | Request::GatewayHeartbeat { .. }
        | Request::GatewayPulse { .. }
        | Request::SetGatewayAdmins { .. }
        | Request::LegacyReserve { .. }
        | Request::ReadDocument
        | Request::WriteDocument { .. }
        | Request::ConfirmRestore { .. }
        | Request::LegacyStatus { .. }
        | Request::GatewayToken { .. }
        | Request::RegisterInvalidate { .. }
        | Request::GatewayRevocations { .. }
        | Request::GatewayTerms { .. }
        | Request::PublishGatewayTerms { .. }
        | Request::GatewayStatus
        | Request::LegacyLogin { .. }
        | Request::LegacyExport
        | Request::LegacyLink { .. }
        | Request::PairClaim { .. }
        | Request::PairPoll { .. }
        | Request::PairRedeem { .. } => false,
    }
}

/// Operations the browser listener alone serves: registration has its
/// per-address budget there, the account page's requests are the website's
/// (the native client makes no recovery code and chooses no display name
/// yet, M6), administration is the website's (`docs/administration.md`),
/// and so is an account's area with its sessions, gateways and tokens.
/// The native listener refuses them before anything else is checked.
fn browser_only(request: &Request) -> bool {
    matches!(
        request,
        Request::Notifications { .. }
            | Request::RegisterChallenge
            | Request::RegisterChallengeFor { .. }
            | Request::Register { .. }
            | Request::AccountSummary
            | Request::SetRecovery { .. }
            | Request::SetDisplayName { .. }
            | Request::Admin { .. }
            | Request::Owner { .. }
            | Request::PairStart
            | Request::PairStatus { .. }
            | Request::PairApprove { .. }
            | Request::PairCancel { .. }
            | Request::Picture { .. }
    )
}

/// Browser operations that act on the cookie's session; the others ignore it.
fn uses_session(request: &Request) -> bool {
    matches!(
        request,
        Request::Notifications { .. }
            | Request::Social { .. }
            | Request::SessionStatus
            | Request::RenewSession
            | Request::Logout
            | Request::TermsStatus
            | Request::AcceptTerms { .. }
            | Request::AccountSummary
            | Request::SetRecovery { .. }
            | Request::SetDisplayName { .. }
            | Request::ChangePasswordStart { .. }
            | Request::ChangePasswordFinish { .. }
            | Request::Admin { .. }
            | Request::Owner { .. }
            | Request::AccountExport
            | Request::DeleteAccountStart { .. }
            | Request::DeleteAccountFinish { .. }
            | Request::KeyCheckStatus
            | Request::SetKeyCheck { .. }
            | Request::PairStart
            | Request::PairStatus { .. }
            | Request::PairApprove { .. }
            | Request::PairCancel { .. }
            | Request::Email { .. }
            | Request::Picture { .. }
    )
}

/// The answer to a password proof that issued `token`: the signed native bearer in
/// the body, or the browser session apart from it, for the cookie. Either way the
/// session is vouched for before its first request ([`IdentityService::issued`]).
fn signed_in(
    service: &IdentityService,
    surface: Surface,
    identity: Identity,
    issued: central_postgres::Issued,
    token: Token,
) -> Result<BrowserReply, Error> {
    service.issued(&token);
    Ok(match surface {
        Surface::Native => BrowserReply {
            response: Response::Session {
                token: service.central_token(&token, identity.account)?,
                identity,
                wrapped_master: issued.wrapped_master,
            },
            session: None,
        },
        Surface::Browser => BrowserReply {
            response: Response::BrowserSession {
                identity,
                username: issued.username,
                wrapped_master: issued.wrapped_master,
                expires_at: issued.expires_at,
            },
            session: Some(token),
        },
    })
}

/// Shared service state. Pending challenges are bounded, expiring and single-use.
pub struct IdentityService {
    store: AccountStore,
    issuer: [u8; 32],
    /// Central's own origin: every signed token's `iss`, and a Central token's `aud`.
    origin: String,
    /// The signing key and the keys the JWKS lists.
    tokens: TokenKeys,
    keys: Arc<ServerKeys>,
    bridge_digest: [u8; 32],
    /// The official gateway's origins, canonical first (`Endpoint::official_origins`).
    official: Vec<String>,
    /// Whether discovery answers `Legacy` for a name it does not find: on
    /// the live service, and wherever a legacy store is configured.
    legacy_discovery: bool,
    /// Whether gateway origins may be numeric-loopback development origins:
    /// everywhere but the live service. Its own flag, not the inverse of
    /// [`Self::legacy_discovery`], which a development legacy store turns on.
    development: bool,
    pending: Mutex<BTreeMap<[u8; 32], Pending>>,
    resets: Mutex<BTreeMap<[u8; 32], reset::PendingReset>>,
    /// Registration's proofs of work and the last hour's registrations.
    challenges: Mutex<challenge::Challenges>,
    limits: Mutex<limits::Limits>,
    /// The public figures' number of accounts and when it was counted
    /// ([`gateways::FIGURES_SECONDS`]); one value, whoever asks.
    accounts: Mutex<gateways::Counted>,
    workers: Arc<Semaphore>,
    terms: Option<central_api::terms::TermsDocument>,
    /// The legacy store, from the cutover until its deadline
    /// (`docs/native-login.md` §"Legacy login"); with it the bridge's own
    /// legacy operations are refused.
    legacy: Option<LegacyStore>,
    push_key: Option<String>,
    /// The mailer, what was asked of it and what waits for it (`email.rs`).
    mails: email::Mails,
    /// The picture worker and the upload slots (`pictures.rs`).
    pictures: pictures::Pictures,
}

impl IdentityService {
    /// Compose a service from an already validated store and restored deployment secrets.
    /// The bridge credential is separately configured; no user session can substitute for it.
    /// Development deliberately does not advertise the live migration fallback.
    ///
    /// # Errors
    /// Rejects an invalid development gateway or live gateway other than the official bridge,
    /// and compiled-in terms of use that do not parse (`docs/terms.md`). A live
    /// service's official gateway is [`central_api::OFFICIAL_ORIGIN`] with its
    /// compiled aliases.
    pub fn new(
        store: AccountStore,
        endpoint: &Endpoint,
        keys: ServerKeys,
        bridge: &Token,
        gateway: &str,
        tokens: TokenKeys,
    ) -> Result<Self, Error> {
        let official = if endpoint.legacy_origin().is_some() {
            if gateway != central_api::OFFICIAL_ORIGIN {
                return Err(Error::Rejected);
            }
            endpoint.official_origins()
        } else {
            Endpoint::development(gateway)?;
            vec![gateway.to_owned()]
        };
        Ok(Self {
            store,
            issuer: endpoint.issuer(),
            origin: endpoint.origin().to_owned(),
            tokens,
            keys: Arc::new(keys),
            bridge_digest: bridge.digest(),
            official,
            legacy_discovery: endpoint.legacy_origin().is_some(),
            development: endpoint.legacy_origin().is_none(),
            pending: Mutex::default(),
            resets: Mutex::default(),
            challenges: Mutex::default(),
            limits: Mutex::new(limits::Limits::keeping(bridge.digest())),
            accounts: Mutex::default(),
            workers: Arc::new(Semaphore::new(4)),
            terms: terms::central()?,
            legacy: None,
            push_key: None,
            mails: email::Mails::default(),
            pictures: pictures::Pictures::default(),
        })
    }

    /// Registration's proof of work at `base` bits and `memory` KiB for
    /// each evaluation, taken `minimum` after it was issued, for tests that
    /// register many accounts. It only lowers: more than the service's own
    /// numbers is cut to them. The service's own launch never calls this
    /// and keeps `docs/registration-abuse.md`'s price and its three
    /// seconds; no configuration and no request reaches it.
    #[must_use]
    pub fn with_registration_challenge(self, base: u8, memory: u32, minimum: Duration) -> Self {
        if let Ok(mut challenges) = self.challenges.lock() {
            *challenges = challenge::Challenges::new(base, memory, minimum);
        }
        self
    }

    /// Advertise the configured public VAPID key; never private signing material.
    #[must_use]
    pub fn with_push_key(mut self, key: String) -> Self {
        self.push_key = Some(key);
        self
    }

    /// Send mail through `mailer` (`docs/email.md`): the codes that confirm
    /// an account's address, reset codes and notices. Without one no
    /// address is named or confirmed and no reset code is mailed; an
    /// address already held can still be read, withdrawn and removed.
    /// Whoever calls this also drives [`Self::deliver_mail`].
    #[must_use]
    pub fn with_mailer(mut self, mailer: Arc<dyn mail::Mailer>) -> Self {
        self.mails.mailer = Some(mailer);
        self
    }

    /// Serve the legacy sign-in from `store` (`docs/native-login.md`
    /// §"Legacy login"): discovery answers `Legacy` for every name it does
    /// not find enrolled, in development too, and the bridge's own legacy
    /// operations are refused, since Central is the source now.
    #[must_use]
    pub fn with_legacy(mut self, store: LegacyStore) -> Self {
        self.legacy = Some(store);
        self.legacy_discovery = true;
        self
    }

    /// Further numeric-loopback origins of the official gateway in development,
    /// as a live service has [`central_api::OFFICIAL_ALIASES`].
    ///
    /// # Errors
    /// Rejects any alias on a live service, whose aliases are compiled, and one
    /// that is not a numeric-loopback development origin.
    pub fn with_aliases(mut self, aliases: Vec<String>) -> Result<Self, Error> {
        if !self.development {
            return Err(Error::Rejected);
        }
        for alias in aliases {
            if Endpoint::development(&alias)?.origin() != alias {
                return Err(Error::Rejected);
            }
            if !self.official.contains(&alias) {
                self.official.push(alias);
            }
        }
        Ok(self)
    }

    /// The official gateway's canonical origin.
    fn canonical(&self) -> &str {
        &self.official[0]
    }

    /// Whether `origin` is one of the official gateway's.
    fn is_official(&self, origin: &str) -> bool {
        self.official.iter().any(|o| o == origin)
    }

    /// Put `terms` in force instead of the compiled-in `legal/` texts, for an
    /// integration test or a staging run. A live service keeps the committed
    /// texts: only a reviewed commit changes what is in force (`docs/terms.md`).
    #[must_use]
    pub fn with_terms(mut self, terms: Option<central_api::terms::TermsDocument>) -> Self {
        self.terms = terms;
        self
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

    async fn start(
        &self,
        identity: Identity,
        message: Vec<u8>,
        surface: Surface,
        place: Option<Place>,
    ) -> Result<Response, Error> {
        let context = self.context(identity)?;
        // What the budgets show without the store is refused before a
        // worker or a statement is taken: refused starts cost neither.
        self.limits
            .lock()
            .map_err(|_| Error::Unavailable)?
            .guesses
            .fits(identity.account, place, Instant::now(), 1)?;
        let permit = self
            .workers
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::Busy)?;
        // Capacity is checked before a query/curve operation and again before insertion.
        self.prune_pending()?;
        let credential = self.store.credential(identity).await?;
        // Counted once a worker is held, so a start that found none spent
        // nothing: one guess, at the account or at nobody's.
        let guess = self
            .limits
            .lock()
            .map_err(|_| Error::Unavailable)?
            .guesses
            .guess(
                identity.account,
                credential.is_some(),
                place,
                Instant::now(),
                1,
            )?;
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
        if pending.len() >= limits::PENDING {
            return Err(Error::Busy);
        }
        pending.insert(
            ticket.digest(),
            Pending {
                expires: Instant::now() + Duration::from_secs(limits::PENDING_SECONDS),
                attempt,
                credential,
                surface,
                purpose: Purpose::Login,
                guess,
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
        if pending.len() >= limits::PENDING {
            return Err(Error::Busy);
        }
        Ok(())
    }

    async fn finish(
        &self,
        ticket: &Token,
        message: Vec<u8>,
        surface: Surface,
        previous: Option<&Token>,
    ) -> Result<BrowserReply, Error> {
        // The worker first: without one the attempt stays and the answer is
        // `Busy`, which a client asks again. Taken after the attempt, a
        // right password would be lost with it and count as a wrong one.
        let permit = self
            .workers
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::Busy)?;
        // Consumption precedes validation, including invalid message size/proofs.
        let pending = self
            .pending
            .lock()
            .map_err(|_| Error::Unavailable)?
            .remove(&ticket.digest())
            .ok_or(Error::Rejected)?;
        // An attempt finishes on the listener that started it, as a sign-in.
        if pending.expires <= Instant::now()
            || pending.surface != surface
            || !matches!(pending.purpose, Purpose::Login)
        {
            return Err(Error::Rejected);
        }
        let (credential, guess) = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            pending
                .attempt
                .finish(&message)
                .map_err(|_| Error::Rejected)?;
            Ok::<_, Error>((pending.credential.ok_or(Error::Rejected)?, pending.guess))
        })
        .await
        .map_err(|_| Error::Unavailable)??;
        // The password is proven: the start was no guess, whatever the
        // store says to the session.
        self.proved(&guess)?;
        let token = Token::generate()?;
        let issued = match surface {
            Surface::Browser => {
                self.store
                    .issue_browser_session(&credential, &token, previous)
                    .await?
            }
            Surface::Native => self.store.issue_session(&credential, &token).await?,
        };
        // Only now is the place the account's own: an attempt begun before
        // a password change proves the old one, and the store refused it.
        self.accepted(&guess);
        signed_in(self, surface, credential.identity, issued, token)
    }

    /// Process a typed native command. Authentication is checked for each operation.
    ///
    /// # Errors
    /// Returns generic authorization, capacity, conflict or availability errors only.
    pub async fn handle(
        &self,
        request: Request,
        presented: Option<&Presented>,
    ) -> Result<Response, Error> {
        self.handle_from(request, presented, None).await
    }

    /// [`Self::handle`] for a request from `place`, the client's address as
    /// the listener counts it. A place confers nothing: it only shares out
    /// the guesses at passwords (`docs/native-login.md` §"Guessing a
    /// password online"). Without one, each account's own budget alone
    /// bounds them.
    ///
    /// # Errors
    /// Returns generic authorization, capacity, conflict or availability errors only.
    pub async fn handle_from(
        &self,
        request: Request,
        presented: Option<&Presented>,
        place: Option<Place>,
    ) -> Result<Response, Error> {
        // A bare credential is a gateway's and a signed session a client's, whatever the
        // store would make of the capability: see [`credential_operation`].
        let allowed = match presented {
            Some(Presented::Credential(_)) => credential_operation(&request),
            Some(Presented::Session(_)) => !credential_operation(&request),
            None => true,
        };
        if !allowed {
            return Err(Error::Rejected);
        }
        let bearer = presented.map(Presented::token);
        let told = limits::told(&request, bearer);
        self.limits
            .lock()
            .map_err(|_| Error::Unavailable)?
            .request(&request, bearer, place, Instant::now())?;
        let reply = self
            .dispatch(request, bearer, Surface::Native, Seen::default(), place)
            .await;
        self.answered(told.as_ref(), bearer, &reply).await;
        Ok(reply?.response)
    }

    /// What an answer says about the bearer its request was counted by, for
    /// the budgets alone (`limits.rs`): whether it is a session's or a
    /// credential's. It grants nothing and is never a refusal: the answer
    /// stands. A session the answer issues was vouched for when it was
    /// issued ([`Self::issued`]).
    ///
    /// An answer that vouches for a bearer not vouched for before asks
    /// the store once whose gateways the bearer belongs to, so that one
    /// account's gateway credentials and tokens are counted together.
    /// When the store cannot say, the bearer is not vouched for by this
    /// answer and waits with the strangers until one can be asked about.
    async fn answered(
        &self,
        told: Option<&limits::Told>,
        bearer: Option<&Token>,
        reply: &Result<BrowserReply, Error>,
    ) {
        let (Some(told), Some(bearer)) = (told, bearer) else {
            return;
        };
        if !self.heard(told, reply) {
            return;
        }
        // The lock is not held across the question.
        let Ok(owner) = self.store.bearer_owner(bearer).await else {
            return;
        };
        if let Ok(mut limits) = self.limits.lock() {
            limits.vouch(told, owner, Instant::now());
        }
    }

    /// The part of [`Self::answered`] that asks nobody; `true` when the
    /// store is to say whose the bearer is.
    fn heard(&self, told: &limits::Told, reply: &Result<BrowserReply, Error>) -> bool {
        let Ok(mut limits) = self.limits.lock() else {
            return false;
        };
        limits.answered(told, reply.as_ref().err(), Instant::now())
    }

    /// A sign-in, a password change or a pairing just issued the session
    /// whose capability is `session`: it is vouched for before its first
    /// request (`limits.rs`). Called where the capability is at hand,
    /// since a native answer carries it only inside the signed Central
    /// token. It grants nothing and is never a refusal.
    fn issued(&self, session: &Token) {
        if let Ok(mut limits) = self.limits.lock() {
            limits.issued(session, Instant::now());
        }
    }

    /// A start's password was proven: its guess comes back (`guesses.rs`).
    fn proved(&self, guess: &guesses::Guess) -> Result<(), Error> {
        self.limits
            .lock()
            .map_err(|_| Error::Unavailable)?
            .guesses
            .proved(guess);
        Ok(())
    }

    /// The store issued the session a proof was for, so the credential
    /// proven is the account's as it stands: the proof's place is the
    /// account's own from now on (`guesses.rs`). Never a refusal: what the
    /// store committed is answered as done.
    fn accepted(&self, guess: &guesses::Guess) {
        if let Ok(mut limits) = self.limits.lock() {
            limits.guesses.accepted(guess, Instant::now());
        }
    }

    /// The store committed another password for `account`, or its
    /// deletion: the places it proved the old one from are forgotten, and
    /// `at`, where the new one was set, is its own (`guesses.rs`). Never
    /// a refusal: what the store committed is answered as done.
    fn changed(&self, account: [u8; 16], at: Option<Place>) {
        if let Ok(mut limits) = self.limits.lock() {
            limits.guesses.changed(account, at, Instant::now());
        }
    }

    /// Process a command from the website's browser listener. Only
    /// [`browser_operation`]s are served; the cookie's token is a browser
    /// session, used only by the operations that act on one, and a sign-in
    /// returns the new session apart from the response.
    ///
    /// # Errors
    /// Returns generic authorization, capacity, conflict or availability errors only.
    pub async fn handle_browser(
        &self,
        request: Request,
        cookie: Option<&Token>,
    ) -> Result<BrowserReply, Error> {
        self.handle_browser_from(request, cookie, None, Seen::default())
            .await
    }

    /// [`Self::handle_browser`] for a request from `place`, as
    /// [`Self::handle_from`] takes one, with what the listener saw of the
    /// request ([`Seen`]). Only a registration challenge reads `seen`, for
    /// its difficulty (`docs/registration-abuse.md` §"Hardened"): it never
    /// refuses a request and grants nothing.
    ///
    /// # Errors
    /// Returns generic authorization, capacity, conflict or availability errors only.
    pub async fn handle_browser_from(
        &self,
        request: Request,
        cookie: Option<&Token>,
        place: Option<Place>,
        seen: Seen,
    ) -> Result<BrowserReply, Error> {
        if !browser_operation(&request) {
            return Err(Error::Rejected);
        }
        let session = if uses_session(&request) { cookie } else { None };
        let told = limits::told(&request, session);
        self.limits
            .lock()
            .map_err(|_| Error::Unavailable)?
            .request(&request, session, place, Instant::now())?;
        // Login keeps its anonymous budget and proof requirement. The old cookie
        // identifies only existing push consent after that proof is verified.
        let context = if matches!(&request, Request::LoginFinish { .. }) {
            cookie
        } else {
            session
        };
        let reply = self
            .dispatch(request, context, Surface::Browser, seen, place)
            .await;
        self.answered(told.as_ref(), session, &reply).await;
        reply
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use central_api::{
        gateway::GatewayMetadata,
        wire::{GatewayPurpose, Identity},
    };

    fn token() -> Token {
        Token::generate().unwrap()
    }

    const IDENTITY: Identity = Identity {
        account: [0; 16],
        epoch: 1,
    };

    fn served() -> Vec<Request> {
        let mut served = entering();
        served.extend(within());
        served
    }

    /// The served operations up to the terms: finding an account, signing
    /// in, registering, the session.
    fn entering() -> Vec<Request> {
        let identity = IDENTITY;
        vec![
            Request::Social {
                request: central_api::social::SocialRequest::Contacts,
            },
            Request::Discover {
                username: "synthetic".into(),
            },
            Request::LoginStart {
                identity,
                message: vec![],
            },
            Request::LoginFinish {
                attempt: token(),
                message: vec![],
            },
            Request::RegisterChallenge,
            Request::RegisterChallengeFor { signals: 0 },
            Request::Register {
                username: "synthetic".into(),
                display_name: Some("Synthetic".into()),
                nonce: String::new(),
                counters: Vec::new(),
            },
            Request::EnrollmentStart {
                ticket: token(),
                message: vec![],
            },
            Request::EnrollmentFinish {
                ticket: token(),
                record: vec![],
                wrapped_master: vec![],
                document: vec![],
                recovery: None,
            },
            Request::SessionStatus,
            Request::RenewSession,
            Request::Logout,
            Request::Terms,
            Request::TermsStatus,
            Request::AcceptTerms {
                subject: TermsSubject::Central,
                version: "1".into(),
                digest: [0; 32],
            },
        ]
    }

    /// The rest of the served operations: the directory and what an
    /// account does with itself.
    fn within() -> Vec<Request> {
        vec![
            Request::GatewayDirectory { after: None },
            Request::PublicFigures { after: None },
            Request::AccountSummary,
            Request::Picture {
                request: central_api::picture::PictureRequest::Mine,
            },
            Request::SetRecovery {
                recovery: vec![1; 73],
            },
            Request::SetDisplayName {
                display_name: "Synthetic".into(),
            },
            Request::ChangePasswordStart {
                login: vec![],
                registration: vec![],
            },
            Request::ChangePasswordFinish {
                attempt: token(),
                proof: vec![],
                record: vec![],
                wrapped_master: vec![],
            },
            Request::Admin {
                request: central_api::admin::AdminRequest::Accounts {
                    query: None,
                    before: None,
                },
            },
            Request::ResetStart {
                username: "synthetic".into(),
                code: central_api::reset_code::ResetCode::generate().unwrap(),
                registration: vec![],
            },
            Request::ResetFinish {
                attempt: token(),
                record: vec![],
                wrapped_master: vec![],
                vault: central_api::wire::ResetVault::Keep,
            },
            Request::AccountExport,
            Request::DeleteAccountStart { login: vec![] },
            Request::DeleteAccountFinish {
                attempt: token(),
                proof: vec![],
            },
            Request::Owner {
                request: central_api::owner::OwnerRequest::Sessions,
            },
            Request::KeyCheckStatus,
            Request::SetKeyCheck { check: vec![] },
            Request::PairStart,
            Request::PairStatus { handle: token() },
            Request::PairApprove {
                handle: token(),
                sealed: vec![],
                proof: vec![],
            },
            Request::PairCancel { handle: token() },
            Request::Email {
                request: central_api::email::EmailRequest::Status,
            },
            Request::ResetMail {
                username: "synthetic".into(),
                lang: "en".into(),
            },
        ]
    }

    fn refused() -> Vec<Request> {
        let identity = IDENTITY;
        let metadata = || GatewayMetadata {
            name: "Synthetic".into(),
            version: "1".into(),
            website: "https://gateway.invalid".into(),
        };
        vec![
            Request::LegacyInvalidate {
                legacy_id: "x".into(),
            },
            Request::LegacyClose {
                legacy_id: "x".into(),
                identity,
            },
            Request::LegacyProfile {
                legacy_id: "x".into(),
                display_name: Some("Synthetic".into()),
                tag: 13,
            },
            Request::RegisterGateway {
                origin: "https://gateway.invalid".into(),
                metadata: metadata(),
            },
            Request::GatewayHeartbeat {
                metadata: metadata(),
            },
            // a gateway's counts come with its heartbeat's credential, never a page's
            pulse(metadata()),
            Request::SetGatewayAdmins {
                gateway: [0; 16],
                expected_revision: 1,
                accounts: vec![],
            },
            Request::LegacyReserve {
                legacy_id: "x".into(),
                username: "synthetic".into(),
                display_name: Some("Synthetic".into()),
                tag: Some(13),
            },
            Request::ReadDocument,
            Request::WriteDocument {
                expected_revision: 1,
                document: vec![],
            },
            Request::ConfirmRestore { revision: 1 },
            Request::LegacyStatus {
                legacy_id: "x".into(),
            },
            Request::GatewayToken {
                gateway: "https://gateway.invalid".into(),
                purpose: GatewayPurpose::Admit,
            },
            Request::RegisterInvalidate {
                path: "/auth/central/invalidate".into(),
            },
            Request::GatewayRevocations { after: 0 },
            Request::GatewayTerms {
                origin: "https://gateway.invalid".into(),
            },
            Request::AcceptTerms {
                subject: TermsSubject::Gateway {
                    origin: "https://gateway.invalid".into(),
                },
                version: "1".into(),
                digest: [0; 32],
            },
            Request::PublishGatewayTerms {
                version: "1".into(),
                texts: vec![],
            },
            // a gateway's token is no website's business
            Request::GatewayStatus,
        ]
        .into_iter()
        .chain(objects())
        .chain(claimed_by_the_client())
        .chain(moved_by_the_client())
        .collect()
    }

    /// A pulse with counts Central would take: what refuses it is where it
    /// was sent.
    fn pulse(metadata: GatewayMetadata) -> Request {
        Request::GatewayPulse {
            metadata,
            activity: central_api::gateway::GatewayActivity {
                players: 1,
                playing: 1,
                tables: 1,
                games: 1,
            },
        }
    }

    /// A native session's own objects.
    fn objects() -> [Request; 5] {
        [
            Request::CreateObject,
            Request::ReadObject { id: [0; 16] },
            Request::ListObjects { after: None },
            Request::WriteObject {
                id: [0; 16],
                expected_revision: 0,
                document: vec![],
            },
            Request::DeleteObject {
                id: [0; 16],
                expected_revision: 1,
            },
        ]
    }

    /// The client's half of a pairing, which only the native listener serves.
    fn claimed_by_the_client() -> [Request; 3] {
        [
            Request::PairClaim {
                handle: token(),
                public_key: vec![],
                challenge: vec![],
                mac: vec![],
            },
            Request::PairPoll { handle: token() },
            Request::PairRedeem {
                handle: token(),
                verifier: vec![],
            },
        ]
    }

    /// The legacy store's sign-in and move, which only the native listener
    /// serves.
    fn moved_by_the_client() -> [Request; 3] {
        [
            Request::LegacyLogin {
                username: "synthetic".into(),
                password: central_api::legacy::LegacyPassword::new("synthetic".into()),
            },
            Request::LegacyExport,
            Request::LegacyLink { digest: [0; 32] },
        ]
    }

    #[test]
    fn the_browser_listener_serves_exactly_its_operations() {
        let (served, refused) = (served(), refused());
        assert!(served.iter().all(browser_operation));
        assert!(!refused.iter().any(browser_operation));
        let named: std::collections::BTreeSet<String> = served
            .iter()
            .chain(&refused)
            .map(|request| {
                let text = String::from_utf8(central_api::encode(request).unwrap()).unwrap();
                let start = text.find(r#""operation":""#).unwrap() + 13;
                text[start..].split('"').next().unwrap().to_owned()
            })
            .collect();
        assert_eq!(named.len(), 67, "every operation is in one of the lists");
    }

    #[test]
    fn the_native_listener_refuses_the_websites_own_operations() {
        let (served, _) = (served(), refused());
        let only: Vec<&Request> = served.iter().filter(|r| browser_only(r)).collect();
        assert_eq!(only.len(), 13);
        // Both ways to ask for a registration challenge: neither is served
        // where no address is counted.
        assert!(browser_only(&Request::RegisterChallenge));
        assert!(browser_only(&Request::RegisterChallengeFor { signals: 0 }));
        // An account's own area is the website's alone, as administration is.
        assert!(browser_only(&Request::Owner {
            request: central_api::owner::OwnerRequest::Gateways,
        }));
        assert!(browser_only(&Request::Admin {
            request: central_api::admin::AdminRequest::Accounts {
                query: None,
                before: None,
            },
        }));
        assert!(only.iter().all(|r| browser_operation(r)));
        assert!(!browser_only(&Request::SessionStatus));
        assert!(!browser_only(&Request::Logout));
    }

    #[test]
    fn only_session_operations_take_the_cookie() {
        assert!(uses_session(&Request::SessionStatus));
        assert!(uses_session(&Request::RenewSession));
        assert!(uses_session(&Request::Logout));
        assert!(uses_session(&Request::TermsStatus));
        assert!(uses_session(&Request::AccountSummary));
        assert!(uses_session(&Request::SetRecovery {
            recovery: vec![1; 73]
        }));
        assert!(uses_session(&Request::SetDisplayName {
            display_name: "Synthetic".into()
        }));
        assert!(uses_session(&Request::ChangePasswordStart {
            login: vec![],
            registration: vec![],
        }));
        assert!(uses_session(&Request::ChangePasswordFinish {
            attempt: token(),
            proof: vec![],
            record: vec![],
            wrapped_master: vec![],
        }));
        // Deletion and export act on the cookie's account, on both listeners.
        for request in [
            Request::AccountExport,
            Request::DeleteAccountStart { login: vec![] },
            Request::DeleteAccountFinish {
                attempt: token(),
                proof: vec![],
            },
        ] {
            assert!(uses_session(&request) && browser_operation(&request));
            assert!(!browser_only(&request));
        }
        assert!(uses_session(&Request::Owner {
            request: central_api::owner::OwnerRequest::Sessions,
        }));
        // An account's address is its session's on both listeners; a reset
        // code by mail is asked for without one, whatever cookie comes along.
        let address = Request::Email {
            request: central_api::email::EmailRequest::Status,
        };
        assert!(uses_session(&address) && browser_operation(&address));
        // Reading it needs no accepted terms; naming one does (`terms.rs`).
        assert!(!browser_only(&address) && !terms::needs_acceptance(&address));
        let mailed = Request::ResetMail {
            username: "synthetic".into(),
            lang: "en".into(),
        };
        assert!(!uses_session(&mailed) && browser_operation(&mailed));
        assert!(!browser_only(&mailed) && !terms::needs_acceptance(&mailed));
        assert!(!uses_session(&Request::Terms));
        assert!(!uses_session(&Request::GatewayDirectory { after: None }));
        // The figures are for everyone: a cookie sent along is not looked at.
        assert!(!uses_session(&Request::PublicFigures { after: None }));
        assert!(!browser_only(&Request::PublicFigures { after: None }));
        assert!(!terms::needs_acceptance(&Request::PublicFigures {
            after: None
        }));
        assert!(!uses_session(&Request::Discover {
            username: "synthetic".into()
        }));
        assert!(!uses_session(&Request::Register {
            username: "synthetic".into(),
            display_name: None,
            nonce: String::new(),
            counters: Vec::new(),
        }));
        assert!(!uses_session(&Request::RegisterChallenge));
        assert!(!uses_session(&Request::RegisterChallengeFor { signals: 3 }));
    }
}
