//! Signing in from a signed-in website: the client's half of a pairing
//! (`docs/web-accounts.md` §"Pairing"), and the key check a password sign-in
//! stores for an account that has none.
//!
//! The link gives the client a handle and the page's one-time secret. The
//! client claims the pairing with a key pair and a verifier made for it,
//! waits until the page approves, and redeems once with the verifier: the
//! answer is a native session and the master key sealed to its key under the
//! link's secret, which only this client can open, and which Central cannot
//! have made, since it never sees the secret. The password plays no part.
use super::{Endpoint, Error, Request, Response, Session, Token, Transport, context};
use central_crypto::pairing::{self, ClientPairing, PairingSecret, SealBinding};
use zeroize::Zeroizing;

/// A claimed pairing, waiting for the page's approval. Holds the client's
/// private key, its verifier and the link's secret for its five minutes;
/// all are wiped when it is dropped, so a client that gives up (cancelled,
/// expired, refused) drops it.
pub struct Pairing {
    endpoint: Endpoint,
    handle: Zeroizing<[u8; 32]>,
    secret: PairingSecret,
    client: ClientPairing,
    expires_at: i64,
}

impl std::fmt::Debug for Pairing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Pairing([REDACTED])")
    }
}

/// Whom a redeemed pairing signs in as, for the client's question: the
/// display name and tag other players see beside the private username.
#[derive(Clone, PartialEq, Eq)]
pub struct PairedAccount {
    /// The account's canonical username, its private sign-in name.
    pub username: String,
    /// The display name other players see; none until the account chose one.
    pub display_name: Option<String>,
    /// The tag beside the display name; none until it was drawn or carried.
    pub tag: Option<u32>,
}

/// Claim the pairing a link names, with its handle and the page's secret,
/// which the pairing keeps until the seal is opened. A claim whose answer
/// was lost may be sent again: the same claim is answered the same.
///
/// # Errors
/// `Conflict` when another client claimed it first, `Rejected` for an
/// unknown, ended or late pairing, and transport failures; the secret is
/// wiped with the refusal.
pub fn claim(
    transport: &impl Transport,
    handle: [u8; 32],
    secret: PairingSecret,
) -> Result<Pairing, Error> {
    let client = ClientPairing::generate().map_err(|_| Error::Unavailable)?;
    let endpoint = transport.endpoint().clone();
    let mac = client.claim_mac(&secret, &endpoint.issuer(), &handle);
    let handle = Zeroizing::new(handle);
    let Response::PairAccepted { expires_at } = transport.exchange(
        &Request::PairClaim {
            handle: Token::from_secret_bytes(handle.clone()),
            public_key: client.public_key().to_vec(),
            challenge: client.challenge().to_vec(),
            mac: mac.to_vec(),
        },
        None,
    )?
    else {
        return Err(Error::Rejected);
    };
    Ok(Pairing {
        endpoint,
        handle,
        secret,
        client,
        expires_at,
    })
}

impl Pairing {
    /// The six characters the page shows beside Approve, for the client to
    /// show while it waits.
    #[must_use]
    pub fn comparison_code(&self) -> String {
        pairing::comparison_code(&self.handle, self.client.public_key())
    }

    /// The pairing's end, Unix seconds.
    #[must_use]
    pub const fn expires_at(&self) -> i64 {
        self.expires_at
    }

    /// Whether the page approved yet.
    ///
    /// # Errors
    /// `Rejected` once the pairing ended or its polls ran out; transport.
    pub fn approved(&self, transport: &impl Transport) -> Result<bool, Error> {
        self.same_endpoint(transport)?;
        match transport.exchange(
            &Request::PairPoll {
                handle: Token::from_secret_bytes(self.handle.clone()),
            },
            None,
        )? {
            Response::Approved => Ok(true),
            Response::Pending => Ok(false),
            _ => Err(Error::Rejected),
        }
    }

    /// Redeem the approved pairing, once: the session, with the master key
    /// opened from the seal, and the account for the client's question. A
    /// seal that does not open ends the session it came with. The link's
    /// secret is wiped as soon as the seal was tried, opened or not.
    ///
    /// # Errors
    /// `Rejected` for a pairing not approved or ended, a seal that does not
    /// open under this pairing's secret and binding, or a session that cannot
    /// read its document; transport failures.
    pub fn redeem(self, transport: &impl Transport) -> Result<(Session, PairedAccount), Error> {
        self.same_endpoint(transport)?;
        let Response::PairSession {
            token,
            identity,
            username,
            display_name,
            tag,
            sealed,
            ..
        } = transport.exchange(
            &Request::PairRedeem {
                handle: Token::from_secret_bytes(self.handle.clone()),
                verifier: self.client.verifier().to_vec(),
            },
            None,
        )?
        else {
            return Err(Error::Rejected);
        };
        let Self {
            endpoint,
            handle,
            secret,
            client,
            expires_at,
        } = self;
        let master = context(&endpoint, identity).and_then(|context| {
            let binding = SealBinding {
                context,
                handle: *handle,
                expires_at,
            };
            client
                .open(&secret, &sealed, &binding)
                .map_err(|_| Error::Rejected)
        });
        drop(secret);
        let master = match master {
            Ok(master) => master,
            Err(error) => {
                let _ = transport.exchange(&Request::Logout, Some(&token));
                return Err(error);
            }
        };
        let mut session = Session::new(endpoint, identity, token, master);
        if let Err(error) = session.refresh(transport) {
            let _ = session.logout(transport);
            return Err(error);
        }
        Ok((
            session,
            PairedAccount {
                username,
                display_name,
                tag,
            },
        ))
    }

    fn same_endpoint(&self, transport: &impl Transport) -> Result<(), Error> {
        if transport.endpoint() != &self.endpoint {
            return Err(Error::Rejected);
        }
        Ok(())
    }
}

impl Session {
    /// After a password sign-in: store the account's key check if it has
    /// none, which Central takes only within two minutes of the sign-in. A
    /// check already set is left as it is.
    ///
    /// # Errors
    /// Transport failures and a refusal other than "already set".
    pub fn ensure_key_check(&self, transport: &impl Transport) -> Result<(), Error> {
        self.same_endpoint(transport)?;
        if matches!(
            transport.exchange(&Request::KeyCheckStatus, Some(&self.token))?,
            Response::KeyCheck { set: true }
        ) {
            return Ok(());
        }
        let check = self
            .master
            .key_check(context(&self.endpoint, self.identity)?)
            .map_err(|_| Error::Rejected)?;
        match transport.exchange(
            &Request::SetKeyCheck {
                check: check.as_bytes().to_vec(),
            },
            Some(&self.token),
        ) {
            Ok(Response::Done) | Err(Error::Conflict) => Ok(()),
            Ok(_) => Err(Error::Rejected),
            Err(error) => Err(error),
        }
    }
}
