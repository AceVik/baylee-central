//! Opaque encrypted objects. Names, file paths and plaintext metadata belong in encrypted manifests.
use serde::{Deserialize, Serialize};

/// Server-generated object identity and compare-and-swap state.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectState {
    /// Immutable database-generated `UUIDv7`, scoped to the authenticated account.
    pub id: [u8; 16],
    /// Zero means reserved but unwritten; positive revisions never move backwards.
    pub revision: u64,
    /// Deletion keeps a tombstone so old ciphertext cannot recreate the same object.
    pub deleted: bool,
}

/// Maximum number of owned objects, including tombstones. Larger files use bounded chunks.
pub const MAX_ACCOUNT_OBJECTS: usize = 16_384;
/// Maximum encrypted bytes per account. Reservations and metadata are additionally count-bounded.
pub const MAX_ACCOUNT_BYTES: u64 = 536_870_912;
