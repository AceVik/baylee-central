//! Narrow, temporary disclosure of a wearer's earned cosmetics at a casual table.
use super::{AssetMeta, AssetPage, Digest, Equipment, Id, Kind};
use crate::{Error, Token, ranked::SeatProof};
use serde::{Deserialize, Serialize};

/// Random presentation capability. It is useless without the registered host's
/// separate credential and that wearer's actual gateway admission.
#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Proof(Token);
impl Proof {
    /// Generate an independent 256-bit capability.
    ///
    /// # Errors
    /// Secure operating-system randomness is unavailable.
    pub fn generate() -> Result<Self, Error> {
        Ok(Self(Token::generate()?))
    }
    /// The only persisted representation.
    #[must_use]
    pub fn digest(&self) -> [u8; 32] {
        self.0.digest()
    }
}
impl Clone for Proof {
    fn clone(&self) -> Self {
        Self(Token::from_secret_bytes(self.0.to_secret_bytes()))
    }
}

/// A private table selection by an authenticated native account.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    /// Create a five-minute invitation for the specific seat this player joined.
    Begin {
        /// Exact gateway origin associated with this account's live ordinary admission.
        origin: String,
        /// The actual game identified on the player's seat ticket.
        game: Id,
        /// Actual seat index, at most seven.
        seat: u8,
        /// Owned items chosen for this deck.
        look: Equipment,
    },
}

/// Native-only response. No peer or account identifier is exposed to the table.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Response {
    /// Present to this game's seat-authorized admission endpoint.
    Grant {
        #[doc = "Transient bearer."]
        proof: Proof,
    },
}

/// Public-to-participants descriptions of this seat's selected rewards.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Worn {
    /// Sleeve metadata if selected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_sleeve: Option<AssetMeta>,
    /// Playmat metadata if selected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_playmat: Option<AssetMeta>,
}

/// Restricted operations for a live, registered gateway's own table.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum HostRequest {
    /// Bind the native wearer's table grant to the same authenticated gateway chair.
    Confirm {
        #[doc = "Logical table UUID."]
        game: Id,
        #[doc = "Actual wearer's seat."]
        seat: u8,
        #[doc = "Opaque owner consent."]
        proof: Proof,
        #[doc = "SID from the verified wearer's ordinary gateway login."]
        admission: SeatProof,
    },
    /// Read only the selected slot on the approved game and chair.
    Art {
        #[doc = "Logical table UUID."]
        game: Id,
        #[doc = "Wearer's seat."]
        seat: u8,
        #[doc = "Time-limited proof for this approved table."]
        proof: Proof,
        #[doc = "Selected surface."]
        kind: Kind,
        #[doc = "Aligned byte offset."]
        offset: u32,
        #[doc = "Optional validated cache digest for offset zero."]
        cached: Option<Digest>,
    },
}

/// Host cannot request unreleased items, the player's collection or other tables.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum HostResponse {
    /// Release exactly this seat's selected metadata.
    Confirmed {
        #[doc = "Approved surface metadata."]
        worn: Worn,
        #[doc = "Remaining seconds on the absolute presentation lease, rounded down."]
        valid_for: u16,
    },
    /// One bounded image page.
    Asset {
        #[doc = "Authorized image slice."]
        page: AssetPage,
    },
    /// Authorization for the caller's exact cache entry was checked again.
    Cached {
        #[doc = "Authorized metadata."]
        asset: AssetMeta,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn table_proofs_redact_secret_material() {
        let proof = Proof::generate().unwrap();
        assert_eq!(format!("{proof:?}"), "Proof(Token([REDACTED]))");
        assert_eq!(proof.digest(), proof.clone().digest());
    }
}
