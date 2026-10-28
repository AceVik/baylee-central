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
use central_postgres::{AccountStore, Credential, Session};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use subtle::ConstantTimeEq;
use tokio::sync::Semaphore;

mod admin;
mod deletion;
mod dispatch;
mod gateways;
mod limits;
mod objects;
mod password;
mod reset;
mod terms;

struct Pending {
    expires: Instant,
    attempt: Attempt,
    credential: Option<Credential>,
    surface: Surface,
    purpose: Purpose,
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
/// for Central only; the public gateway directory is read there without a
/// session. The match names every operation, so a new one does not compile
/// until it is decided here.
#[must_use]
pub fn browser_operation(request: &Request) -> bool {
    match request {
        Request::Discover { .. }
        | Request::LoginStart { .. }
        | Request::LoginFinish { .. }
        | Request::Register { .. }
        | Request::EnrollmentStart { .. }
        | Request::EnrollmentFinish { .. }
        | Request::SessionStatus
        | Request::RenewSession
        | Request::Logout
        | Request::Terms
        | Request::TermsStatus
        | Request::GatewayDirectory { .. }
        | Request::AccountSummary
        | Request::SetRecovery { .. }
        | Request::SetDisplayName { .. }
        | Request::ChangePasswordStart { .. }
        | Request::ChangePasswordFinish { .. }
        | Request::Admin { .. }
        | Request::ResetStart { .. }
        | Request::ResetFinish { .. }
        | Request::AccountExport
        | Request::DeleteAccountStart { .. }
        | Request::DeleteAccountFinish { .. } => true,
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
        | Request::SetGatewayAdmins { .. }
        | Request::CheckGatewayAdmin { .. }
        | Request::GatewaySession { .. }
        | Request::GatewayProfile { .. }
        | Request::LegacyReserve { .. }
        | Request::ReadDocument
        | Request::WriteDocument { .. }
        | Request::ConfirmRestore { .. }
        | Request::LegacyStatus { .. }
        | Request::GatewayTicket { .. }
        | Request::RedeemGateway { .. }
        | Request::GatewayTerms { .. }
        | Request::PublishGatewayTerms { .. }
        | Request::GatewayAuthorize { .. } => false,
    }
}

/// Operations the browser listener alone serves: registration has its
/// per-address budget there, the account page's requests are the website's
/// (the native client makes no recovery code and chooses no display name
/// yet, M6), and administration is the website's (`docs/administration.md`).
/// The native listener refuses them before anything else is checked.
fn browser_only(request: &Request) -> bool {
    matches!(
        request,
        Request::Register { .. }
            | Request::AccountSummary
            | Request::SetRecovery { .. }
            | Request::SetDisplayName { .. }
            | Request::Admin { .. }
    )
}

/// Browser operations that act on the cookie's session; the others ignore it.
fn uses_session(request: &Request) -> bool {
    matches!(
        request,
        Request::SessionStatus
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
            | Request::AccountExport
            | Request::DeleteAccountStart { .. }
            | Request::DeleteAccountFinish { .. }
    )
}

/// The answer to a password proof that issued `token`: the native bearer in
/// the body, or the browser session apart from it, for the cookie.
fn signed_in(
    surface: Surface,
    identity: Identity,
    issued: central_postgres::Issued,
    token: Token,
) -> BrowserReply {
    match surface {
        Surface::Native => BrowserReply {
            response: Response::Session {
                token,
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
    }
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
    resets: Mutex<BTreeMap<[u8; 32], reset::PendingReset>>,
    limits: Mutex<limits::Limits>,
    workers: Arc<Semaphore>,
    terms: Option<central_api::terms::TermsDocument>,
}

impl IdentityService {
    /// Compose a service from an already validated store and restored deployment secrets.
    /// The bridge credential is separately configured; no user session can substitute for it.
    /// Development deliberately does not advertise the live migration fallback.
    ///
    /// # Errors
    /// Rejects an invalid development gateway or live gateway other than the official bridge,
    /// and compiled-in terms of use that do not parse (`docs/terms.md`).
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
            resets: Mutex::default(),
            limits: Mutex::default(),
            workers: Arc::new(Semaphore::new(4)),
            terms: terms::central()?,
        })
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
    ) -> Result<Response, Error> {
        let context = self.context(identity)?;
        self.limits
            .lock()
            .map_err(|_| Error::Unavailable)?
            .account(identity.account, Instant::now(), 1)?;
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
                surface,
                purpose: Purpose::Login,
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

    async fn finish(
        &self,
        ticket: &Token,
        message: Vec<u8>,
        surface: Surface,
    ) -> Result<BrowserReply, Error> {
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
        let issued = self
            .store
            .issue_session(&credential, surface.session(Some(&token))?)
            .await?;
        Ok(signed_in(surface, credential.identity, issued, token))
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
        Ok(self
            .dispatch(request, bearer, Surface::Native)
            .await?
            .response)
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
        if !browser_operation(&request) {
            return Err(Error::Rejected);
        }
        let session = if uses_session(&request) { cookie } else { None };
        self.limits
            .lock()
            .map_err(|_| Error::Unavailable)?
            .request(&request, session, Instant::now())?;
        self.dispatch(request, session, Surface::Browser).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use central_api::{
        gateway::GatewayMetadata,
        terms::TermsVersion,
        wire::{Identity, TicketPurpose},
    };

    fn token() -> Token {
        Token::generate().unwrap()
    }

    const IDENTITY: Identity = Identity {
        account: [0; 16],
        epoch: 1,
    };

    fn served() -> Vec<Request> {
        let identity = IDENTITY;
        vec![
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
            Request::Register {
                username: "synthetic".into(),
                beta_key: String::new(),
                display_name: Some("Synthetic".into()),
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
            Request::GatewayDirectory { after: None },
            Request::AccountSummary,
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
            Request::SetGatewayAdmins {
                gateway: [0; 16],
                expected_revision: 1,
                accounts: vec![],
            },
            Request::CheckGatewayAdmin { account: [0; 16] },
            Request::GatewaySession {
                lease: token(),
                purpose: TicketPurpose::Admit,
            },
            Request::GatewayProfile {
                lease: token(),
                purpose: TicketPurpose::Admit,
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
            Request::GatewayTicket {
                gateway: "https://gateway.invalid".into(),
                purpose: TicketPurpose::Admit,
            },
            Request::RedeemGateway {
                ticket: token(),
                purpose: TicketPurpose::Admit,
            },
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
            Request::GatewayAuthorize {
                lease: token(),
                purpose: TicketPurpose::Admit,
                terms: Some(TermsVersion {
                    version: "1".into(),
                    digest: [0; 32],
                }),
            },
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
        assert_eq!(named.len(), 48, "every operation is in one of the lists");
    }

    #[test]
    fn the_native_listener_refuses_the_websites_own_operations() {
        let (served, _) = (served(), refused());
        let only: Vec<&Request> = served.iter().filter(|r| browser_only(r)).collect();
        assert_eq!(only.len(), 5);
        // Administration is the website's alone.
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
        assert!(!uses_session(&Request::Terms));
        assert!(!uses_session(&Request::GatewayDirectory { after: None }));
        assert!(!uses_session(&Request::Discover {
            username: "synthetic".into()
        }));
        assert!(!uses_session(&Request::Register {
            username: "synthetic".into(),
            beta_key: String::new(),
            display_name: None,
        }));
    }
}
