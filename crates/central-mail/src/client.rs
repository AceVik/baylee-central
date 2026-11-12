//! The mailer's client (`docs/email.md`, the mailer's `API.md`): one
//! `POST /v1/send` a mail, over plain HTTP to a numeric loopback address
//! and nowhere else. No proxy, no redirect, no name is looked up, no
//! connection is kept. The token is wiped when dropped and never printed.
use crate::{Mail, Mailer, Outcome, render::render};
use central_api::Error;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, time::Duration};
use ureq::{
    Agent, Body,
    config::Config,
    http::{Response, Uri},
    unversioned::{
        resolver::{ResolvedSocketAddrs, Resolver},
        transport::{DefaultConnector, NextTimeout},
    },
};
use zeroize::Zeroizing;

/// Time to connect: the mailer is on this host, and an absent one should
/// cost a request little.
const CONNECT: Duration = Duration::from_secs(1);
/// Time for a whole request. The mailer queues a mail and answers; it does
/// not deliver first. Shorter than the five seconds the listeners give a
/// request, so that a mailer that does not answer is answered as busy and
/// the request is not cut off between the mail and what the store keeps.
const OVERALL: Duration = Duration::from_secs(3);
/// Most bytes of the answer's head: the mailer's is a few lines.
const HEAD_BYTES: usize = 8192;
/// Most bytes read of a refusal: the mailer's are a few dozen.
const ANSWER_BYTES: u64 = 4096;
/// Shortest token taken: 32 characters hold 128 bits even as hexadecimal
/// digits. The mailer's own are 64.
const TOKEN_MIN: usize = 32;
/// Longest token taken, so that the header stays a line.
const TOKEN_MAX: usize = 256;

/// Central's bearer token for the mailer. Neither Clone nor Display; Debug
/// redacts it and it is wiped when dropped.
pub struct Token(Zeroizing<String>);

impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Token([REDACTED])")
    }
}

impl Token {
    /// Take a token as the configuration holds it: 32 to 256 characters of
    /// the ones a bearer token is made of (RFC 6750: letters, digits and
    /// `-._~+/=`), so that it is a header's value and nothing else.
    ///
    /// # Errors
    /// `Rejected` for any other text. The text is wiped either way.
    pub fn new(text: String) -> Result<Self, Error> {
        let text = Zeroizing::new(text);
        let fits = (TOKEN_MIN..=TOKEN_MAX).contains(&text.len())
            && text
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-._~+/=".contains(&b));
        fits.then_some(Self(text)).ok_or(Error::Rejected)
    }
}

/// Answers the one address the client was made for and no other, without
/// looking any name up.
#[derive(Debug)]
struct Loopback(SocketAddr);

impl Resolver for Loopback {
    fn resolve(
        &self,
        uri: &Uri,
        _: &Config,
        _: NextTimeout,
    ) -> Result<ResolvedSocketAddrs, ureq::Error> {
        let named = uri.scheme_str() == Some("http")
            && uri
                .authority()
                .is_some_and(|authority| authority.as_str() == self.0.to_string());
        if !named || !self.0.ip().is_loopback() {
            return Err(ureq::Error::HostNotFound);
        }
        let mut only = self.empty();
        only.push(self.0);
        Ok(only)
    }
}

/// The mailer's address from the configuration: `http://`, a numeric
/// loopback address and a port, and nothing after it.
fn loopback(endpoint: &str) -> Result<SocketAddr, Error> {
    let address: SocketAddr = endpoint
        .strip_prefix("http://")
        .ok_or(Error::Rejected)?
        .parse()
        .map_err(|_| Error::Rejected)?;
    let scoped = matches!(address, SocketAddr::V6(v6) if v6.scope_id() != 0);
    if !address.ip().is_loopback() || address.port() == 0 || scoped {
        return Err(Error::Rejected);
    }
    Ok(address)
}

/// The request's body, as the mailer's `API.md` names its fields. The
/// idempotency key travels in its header only.
#[derive(Serialize)]
struct Send<'a> {
    to: &'a str,
    subject: &'a str,
    text: &'a str,
    html: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    ttl_secs: Option<u32>,
    limit_key: &'a str,
}

/// A refusal's body; only its code is read.
#[derive(Deserialize)]
struct Refusal {
    error: String,
}

/// Lower-case hexadecimal digits of `bytes`.
fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|byte| {
            [
                DIGITS[usize::from(byte >> 4)],
                DIGITS[usize::from(byte & 15)],
            ]
        })
        .map(char::from)
        .collect()
}

/// The deployed [`Mailer`]: the client of the mailer on this host. Debug
/// prints nothing of it.
pub struct Client {
    agent: Agent,
    url: String,
    authorization: Zeroizing<String>,
    site: String,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Client([REDACTED])")
    }
}

impl Client {
    /// A client for the mailer at `endpoint`, whose mails link to the site
    /// at the origin `site`. `endpoint` is `http://`, a numeric loopback
    /// address and a port (`http://127.0.0.1:28795`), with nothing after
    /// it. `site` is a canonical HTTPS origin; with `development` it is a
    /// numeric loopback HTTP origin instead. Nothing is sent here.
    ///
    /// # Errors
    /// `Rejected` for an endpoint that is not loopback HTTP and for a site
    /// that is no such origin.
    pub fn new(endpoint: &str, site: &str, token: Token, development: bool) -> Result<Self, Error> {
        let address = loopback(endpoint)?;
        central_api::gateway::validate_origin(site, development)?;
        let config = Agent::config_builder()
            .https_only(false)
            .proxy(None)
            .max_redirects(0)
            .max_idle_connections(0)
            .max_idle_connections_per_host(0)
            .timeout_connect(Some(CONNECT))
            .timeout_global(Some(OVERALL))
            .max_response_header_size(HEAD_BYTES)
            .http_status_as_error(false)
            .build();
        // The token ends here: only the header line made of it is kept.
        let Token(token) = token;
        let mut authorization = Zeroizing::new(String::with_capacity(7 + token.len()));
        authorization.push_str("Bearer ");
        authorization.push_str(&token);
        Ok(Self {
            agent: Agent::with_parts(config, DefaultConnector::default(), Loopback(address)),
            url: format!("http://{address}/v1/send"),
            authorization,
            site: site.to_owned(),
        })
    }
}

impl Mailer for Client {
    fn send(&self, mail: &Mail) -> Outcome {
        // A key of its own for every mail: Central asks once and never
        // again, so the key only keeps one request from becoming two mails
        // on the way.
        let mut key = [0; 16];
        if getrandom::fill(&mut key).is_err() {
            return Outcome::Unavailable;
        }
        let rendered = render(mail, &self.site);
        let body = serde_json::to_vec(&Send {
            to: mail.to(),
            subject: &rendered.subject,
            text: &rendered.text,
            html: &rendered.html,
            ttl_secs: mail.ttl_secs(),
            limit_key: &format!("account:{}", hex(&mail.account())),
        })
        .map(Zeroizing::new);
        let Ok(body) = body else {
            return Outcome::Unavailable;
        };
        let answer = self
            .agent
            .post(&self.url)
            .header("Authorization", self.authorization.as_str())
            .header("Content-Type", "application/json")
            .header("Idempotency-Key", hex(&key))
            .send(body.as_slice());
        answer.map_or(Outcome::Unavailable, outcome)
    }
}

/// What the mailer's answer means. Only a refusal's body is read, and of
/// it at most [`ANSWER_BYTES`].
fn outcome(mut answer: Response<Body>) -> Outcome {
    match answer.status().as_u16() {
        // Queued; or queued before under this key, which no mail of
        // Central's is sent under twice.
        200 | 202 => Outcome::Sent,
        // A limit, a full queue, a mailer that stops: not sent.
        429 | 503 => Outcome::Busy,
        400 => {
            let refusal = answer
                .body_mut()
                .with_config()
                .limit(ANSWER_BYTES)
                .read_to_vec()
                .ok()
                .and_then(|bytes| serde_json::from_slice::<Refusal>(&bytes).ok());
            if refusal.is_some_and(|refusal| refusal.error == "invalid_address") {
                Outcome::Rejected
            } else {
                Outcome::Unavailable
            }
        }
        _ => Outcome::Unavailable,
    }
}

#[cfg(test)]
mod tests;
