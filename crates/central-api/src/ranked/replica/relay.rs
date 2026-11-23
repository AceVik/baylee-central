//! Fixed authenticated seat operations forwarded through Central. The
//! selected host never receives a player's network address or Central session.
use super::{Id, SetupBytes};
use crate::Error;
use crate::private_bytes::PrivateBytes;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use zeroize::Zeroizing;

/// Maximum response body necessary for one seat admission, grant or acknowledgement.
pub type HttpBytes = PrivateBytes<4096>;

/// Opaque scoped credential. Its contents are never formatted.
#[derive(Clone, Eq, PartialEq)]
pub struct Secret(Zeroizing<String>);

impl Secret {
    /// Import a bounded printable credential.
    ///
    /// # Errors
    /// Rejects empty, oversized, non-ASCII or whitespace-bearing material.
    pub fn new(value: String) -> Result<Self, Error> {
        let text = Zeroizing::new(value);
        if text.is_empty()
            || text.len() > crate::SignedToken::MAX_BYTES
            || !text.bytes().all(|b| b.is_ascii_graphic())
        {
            return Err(Error::Rejected);
        }
        Ok(Self(text))
    }
    /// Present only to the intended fixed relay operation.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// Validate the canonical spelling of gateway, seat and one-use socket capabilities.
    #[must_use]
    pub fn is_capability(&self) -> bool {
        self.0.len() == 64 && self.0.bytes().all(|b| matches!(b,b'0'..=b'9'|b'a'..=b'f'))
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret([REDACTED])")
    }
}
impl Serialize for Secret {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.expose())
    }
}
impl<'de> Deserialize<'de> for Secret {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

/// Only these operations exist at the game relay.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Call {
    /// Ask Central to mint and present a pseudonymous proof to its assigned host.
    Admit {},
    /// Submit one player's projected, frozen playable deck.
    Play {
        #[doc = "Scoped host session."]
        bearer: Secret,
        #[doc = "Encoded normalized deck, bounded on ingestion."]
        deck: SetupBytes,
    },
    /// Reclaim this participant's already-bound game seat.
    Reclaim {
        #[doc = "Scoped host session."]
        bearer: Secret,
        #[doc = "Current engine game."]
        game: Id,
    },
    /// Exchange a host seat credential for a one-use socket admission at Central.
    SocketTicket {
        #[doc = "Current engine game."]
        game: Id,
        #[doc = "Host's seat capability."]
        seat_token: Secret,
    },
}

impl Call {
    pub(super) fn valid(&self) -> bool {
        match self {
            Self::Admit {} => true,
            Self::Play { bearer, .. } => bearer.is_capability(),
            Self::Reclaim { bearer, game } => {
                bearer.is_capability() && super::validation::uuid(game)
            }
            Self::SocketTicket { seat_token, game } => {
                seat_token.is_capability() && super::validation::uuid(game)
            }
        }
    }
}

/// Successful response or generic fixed-path HTTP refusal from the scoped relay.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Reply {
    /// Bounded host-local seat exchange. Only successful grant bodies may pass.
    Http {
        #[doc = "HTTP status."]
        status: u16,
        #[doc = "Private bounded response."]
        body: Option<HttpBytes>,
    },
    /// One-use socket ticket issued by Central, never the host capability.
    Socket {
        #[doc = "Short-lived Central-only ticket."]
        ticket: Secret,
    },
}
