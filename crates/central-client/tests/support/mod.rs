use central_api::{
    Bearer, Endpoint, Error, PROTOCOL_HEADER, PROTOCOL_VERSION, SignedToken, Token,
    wire::{Request, Response},
};
use central_client::Transport;
use central_crypto::jwt::{self, Expected, GatewayClaims, KeySet, RevocationClaims, SigningKey};
use central_identity::{IdentityService, TokenKeys};
use central_postgres::DevConfig;
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
pub(super) mod admin;
pub(super) mod browser;
pub(super) mod deletion;
pub(super) mod email;
pub(super) mod gateways;
pub(super) mod guessing;
pub(super) mod legacy;
pub(super) mod notifications;
pub(super) mod pairing;
pub(super) mod password;
pub(super) mod registration;
pub(super) mod reports;
pub(super) mod reset;
pub(super) mod strangers;
pub(super) mod terms;
pub(super) mod tokens;

pub(super) fn config(user: &str, password: &str) -> DevConfig {
    assert_eq!(
        std::env::var("CENTRAL_TEST_FIXTURE").unwrap(),
        "isolated-v1"
    );
    DevConfig::new(
        std::env::var("CENTRAL_TEST_PORT").unwrap().parse().unwrap(),
        user,
        std::env::var(password).unwrap(),
        "central_native_test",
    )
    .unwrap()
}

pub(super) struct Http {
    pub(super) endpoint: Endpoint,
    pub(super) address: SocketAddr,
    /// The service behind the listener, for what no request reaches: the revocation sender.
    pub(super) identity: Arc<IdentityService>,
    pub(super) lose_write_reply: AtomicBool,
    /// Lose the answer to the next committed `ChangePasswordFinish`.
    pub(super) lose_password_reply: AtomicBool,
    /// Lose the answer to the next committed `DeleteAccountFinish`.
    pub(super) lose_delete_reply: AtomicBool,
}

impl Transport for Http {
    fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }
    fn exchange(&self, request: &Request, bearer: Option<Bearer<'_>>) -> Result<Response, Error> {
        self.sent(request, bearer, None)
    }
}

impl Http {
    /// A request without a session as the proxy would hand it on from
    /// `hop`: the address the account listener's budgets count it by.
    pub(super) fn from(&self, hop: &str, request: &Request) -> Result<Response, Error> {
        self.sent(request, None, Some(hop))
    }

    /// A body as written, not as this build's types encode it: what a
    /// program built against another shape of the wire would send.
    pub(super) fn written(
        &self,
        body: &str,
        bearer: Option<Bearer<'_>>,
    ) -> Result<Response, Error> {
        central_api::decode(&self.posted(body.as_bytes(), bearer, None)?)
    }

    fn sent(
        &self,
        request: &Request,
        bearer: Option<Bearer<'_>>,
        hop: Option<&str>,
    ) -> Result<Response, Error> {
        let answer = self.posted(&central_api::encode(request)?, bearer, hop)?;
        if matches!(request, Request::WriteDocument { .. })
            && self.lose_write_reply.swap(false, Ordering::AcqRel)
        {
            return Err(Error::Unavailable);
        }
        if matches!(request, Request::ChangePasswordFinish { .. })
            && self.lose_password_reply.swap(false, Ordering::AcqRel)
        {
            return Err(Error::Unavailable);
        }
        if matches!(request, Request::DeleteAccountFinish { .. })
            && self.lose_delete_reply.swap(false, Ordering::AcqRel)
        {
            return Err(Error::Unavailable);
        }
        central_api::decode(&answer)
    }

    /// One request's bytes to the native route, and its answer's body once
    /// the status says there is one.
    fn posted(
        &self,
        bytes: &[u8],
        bearer: Option<Bearer<'_>>,
        hop: Option<&str>,
    ) -> Result<Vec<u8>, Error> {
        pace();
        let mut stream = TcpStream::connect_timeout(&self.address, Duration::from_secs(3))
            .map_err(|_| Error::Unavailable)?;
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        write!(stream, "POST /v1/native HTTP/1.1\r\nHost: {}\r\n{PROTOCOL_HEADER}: {PROTOCOL_VERSION}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n", self.address, bytes.len()).unwrap();
        if let Some(token) = bearer {
            write!(stream, "Authorization: {}\r\n", token.header().as_str()).unwrap();
        }
        if let Some(hop) = hop {
            write!(stream, "X-Forwarded-For: {hop}\r\n").unwrap();
        }
        stream.write_all(b"\r\n").unwrap();
        stream.write_all(bytes).unwrap();
        let mut response = Vec::new();
        stream
            .take((central_api::MAX_WIRE_BYTES + 4096) as u64)
            .read_to_end(&mut response)
            .map_err(|_| Error::Unavailable)?;
        let split = response
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .ok_or(Error::Rejected)?;
        let head = std::str::from_utf8(&response[..split]).map_err(|_| Error::Rejected)?;
        assert!(head.to_lowercase().contains("cache-control: no-store"));
        let status = head.split_whitespace().nth(1).ok_or(Error::Rejected)?;
        match status {
            "200" => {}
            "403" => return Err(Error::TermsRequired),
            "409" => return Err(Error::Conflict),
            "426" => return Err(Error::UpdateRequired),
            "429" => return Err(Error::Busy),
            "503" => return Err(Error::Unavailable),
            _ => return Err(Error::Rejected),
        }
        Ok(response.split_off(split + 4))
    }
}

/// Keep this long, mixed HTTP scenario below the global 32 requests/second
/// public budget, regardless of CPU speed or where a one-second window starts.
/// Exact budget exhaustion is asserted in dedicated boundary/limiter tests.
pub(super) fn pace() {
    static LAST: std::sync::Mutex<Option<std::time::Instant>> = std::sync::Mutex::new(None);
    let mut last = LAST.lock().unwrap();
    if let Some(previous) = *last {
        std::thread::sleep(Duration::from_millis(35).saturating_sub(previous.elapsed()));
    }
    *last = Some(std::time::Instant::now());
}

/// The fixture's key ring, as a restart restores it from its configuration: every service
/// of one run signs with one key, made once, so that a second service on the same store (a
/// restart, another mailer) verifies what the first one signed.
pub(super) fn token_keys() -> TokenKeys {
    static SEED: std::sync::OnceLock<zeroize::Zeroizing<[u8; 32]>> = std::sync::OnceLock::new();
    let seed = SEED.get_or_init(|| SigningKey::generate_seed().unwrap());
    TokenKeys::new(SigningKey::from_seed(seed), Vec::new()).unwrap()
}

/// What a gateway reads the clock as.
pub(super) fn now() -> i64 {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    )
    .unwrap()
}

impl Http {
    /// One request exactly as given, with no protocol header but what `headers` says: what
    /// an old client, or an attacker, sends. Answers the status, the head and the body.
    pub(super) fn raw(
        &self,
        start: &str,
        headers: &[(&str, &str)],
        body: &[u8],
    ) -> (u16, String, Vec<u8>) {
        pace();
        let mut stream = TcpStream::connect_timeout(&self.address, Duration::from_secs(3)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        write!(stream, "{start} HTTP/1.1\r\nHost: {}\r\n", self.address).unwrap();
        for (name, value) in headers {
            write!(stream, "{name}: {value}\r\n").unwrap();
        }
        write!(stream, "Content-Length: {}\r\n\r\n", body.len()).unwrap();
        stream.write_all(body).unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).unwrap();
        let split = response.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
        let head = std::str::from_utf8(&response[..split]).unwrap().to_owned();
        let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
        (status, head, response[split + 4..].to_vec())
    }

    /// The keys, fetched as a gateway fetches them.
    pub(super) fn keys(&self) -> KeySet {
        let (status, _, body) = self.raw("GET /.well-known/jwks.json", &[], b"");
        assert_eq!(status, 200);
        KeySet::from_jwks(&body).unwrap()
    }

    fn expected<'a>(&'a self, audience: &'a str) -> Expected<'a> {
        Expected {
            issuer: self.endpoint.origin(),
            audience,
            now: now(),
        }
    }

    /// A gateway token, verified as the gateway at `audience` verifies it.
    pub(super) fn gateway_claims(&self, token: &SignedToken, audience: &str) -> GatewayClaims {
        jwt::verify(token.expose(), &self.keys(), self.expected(audience))
            .expect("a token this gateway accepts")
    }

    /// The `sid`s after `after` that `bearer`'s gateway at `audience` is told of, and its last
    /// number and floor.
    pub(super) fn revoked(
        &self,
        bearer: &Token,
        audience: &str,
        after: u64,
    ) -> (Vec<String>, u64, u64) {
        let Response::GatewayRevocations { token, last, floor } = self
            .exchange(&Request::GatewayRevocations { after }, Some(bearer.into()))
            .unwrap()
        else {
            panic!("a page of revocations");
        };
        let sids = token.map_or_else(Vec::new, |token| {
            let claims: RevocationClaims =
                jwt::verify(token.expose(), &self.keys(), self.expected(audience))
                    .expect("a message this gateway accepts");
            claims.sids
        });
        (sids, last, floor)
    }
}
