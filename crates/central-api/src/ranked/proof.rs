//! A host echoes the SID it actually received in a validated game-only JWT.
use crate::Error;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Opaque proof of presentation for a single ranked admission. The authority
/// independently checks the live native session, match, seat, host and generation.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(transparent)]
pub struct SeatProof([u8; 16]);

impl std::fmt::Debug for SeatProof {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SeatProof([REDACTED])")
    }
}

impl SeatProof {
    /// Decode the canonical signed SID.
    ///
    /// # Errors
    /// Rejects any other length, case or character.
    pub fn from_sid(text: &str) -> Result<Self, Error> {
        Ok(Self(*crate::token::lower_hex(text)?))
    }
    /// Wrap the SID of a previously issued scoped grant.
    #[must_use]
    pub const fn from_bytes(sid: [u8; 16]) -> Self {
        Self(sid)
    }
    /// Borrow the SID for the authority's exact lookup.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::SeatProof;
    #[test]
    fn proof_has_exact_width_and_debug_redacts_it() {
        let text = "ab".repeat(16);
        let parsed = SeatProof::from_sid(&text).unwrap();
        assert_eq!(parsed, SeatProof::from_bytes([0xab; 16]));
        assert_eq!(format!("{parsed:?}"), "SeatProof([REDACTED])");
        assert!(SeatProof::from_sid(&text.to_uppercase()).is_err());
        assert!(SeatProof::from_sid(&text[..30]).is_err());
        assert!(SeatProof::from_sid(&"ü".repeat(16)).is_err());
    }
}
