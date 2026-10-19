//! Closed version-one request/response vocabulary. No secret-bearing type implements Debug.
use crate::Token;
use serde::{Deserialize, Serialize};

/// A gateway capability authorizes exactly one action, never a general account session.
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TicketPurpose {
    /// Admit to gameplay using an already-linked account.
    Admit,
    /// Read a source snapshot to resume interrupted migration.
    Export,
    /// Finalize the immutable account link after successful restore.
    Link,
}

impl TicketPurpose {
    /// Stable database representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Admit => "admit",
            Self::Export => "export",
            Self::Link => "link",
        }
    }
}

/// Immutable identity metadata authenticated by the configured central HTTPS endpoint.
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    /// Database-generated `UUIDv7`.
    pub account: [u8; 16],
    /// Vault epoch; authentication resets never silently initialize another vault.
    pub epoch: u64,
}

/// Native client request, serialized as a tagged object. Bearer credentials belong in headers.
#[derive(Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    /// Resolve a private login name. Unknown names receive the same legacy eligibility result.
    Discover {
        /// Username as entered, canonicalized by the service's versioned name policy.
        username: String,
    },
    /// First OPAQUE login message.
    LoginStart {
        /// Account mapping from discovery, checked again by the server.
        identity: Identity,
        /// KE1; exact suite length is enforced before expensive processing.
        message: Vec<u8>,
    },
    /// Final OPAQUE proof. A pending attempt is consumed on success and failure.
    LoginFinish {
        /// Random short-lived attempt capability.
        attempt: Token,
        /// KE3, not a password.
        message: Vec<u8>,
    },
    /// Authenticated legacy gateway reserves an identity after fresh password verification.
    LegacyReserve {
        /// Immutable ID at the sole configured legacy gateway; never an email match.
        legacy_id: String,
        /// Canonical username vouched for by the authenticated migration bridge.
        username: String,
    },
    /// An enrollment capability authorizes a response, not replacement of an existing record.
    EnrollmentStart {
        /// Short-lived capability returned through the authenticated legacy login.
        ticket: Token,
        /// OPAQUE registration request.
        message: Vec<u8>,
    },
    /// Atomic first enrollment: credential, key wrapper and initial encrypted manifest.
    EnrollmentFinish {
        /// Same operation capability; consumed only by the atomic successful commit.
        ticket: Token,
        /// OPAQUE registration upload.
        record: Vec<u8>,
        /// Client-encrypted random master key, exactly 73 bytes.
        wrapped_master: Vec<u8>,
        /// Initial encrypted account document. Central never receives its plaintext.
        document: Vec<u8>,
    },
    /// Restore the account document after final OPAQUE authentication.
    ReadDocument,
    /// Compare-and-swap an encrypted account document, never unconditional overwrite.
    WriteDocument {
        /// Last authenticated revision read by this client.
        expected_revision: u64,
        /// Ciphertext authenticated under `expected_revision` + 1.
        document: Vec<u8>,
    },
    /// Client has restored and authenticated its first central copy.
    ConfirmRestore {
        /// Revision whose ciphertext the client successfully authenticated and restored.
        revision: u64,
    },
    /// Migration bridge queries durable cutover state; no client assertion is accepted.
    LegacyStatus {
        /// Original gateway account identifier.
        legacy_id: String,
    },
    /// Issue a one-use ticket scoped to a configured gateway; never forward a central session.
    GatewayTicket {
        /// Exact canonical gateway origin.
        gateway: String,
        /// Single action authorized by this ticket.
        purpose: TicketPurpose,
    },
    /// Configured gateway redeems its own audience-bound ticket.
    RedeemGateway {
        /// One-use, short-lived ticket delivered by the native client.
        ticket: Token,
        /// Action enforced by the receiving gateway route.
        purpose: TicketPurpose,
    },
    /// Revoke the presented central session immediately.
    Logout,
}

/// Account credential destination decision; transport errors are never legacy eligibility.
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Discovery {
    /// Existing enrolled account.
    Central {
        /// Authenticated account mapping.
        identity: Identity,
    },
    /// Explicit legacy enrollment path, only valid with the compiled live endpoint policy.
    Legacy,
}

/// Durable migration state visible only to its authenticated gateway bridge.
#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MigrationState {
    /// Enrollment has not committed; old data and login remain available.
    Pending,
    /// Credential and encrypted data committed, but restoration is not confirmed.
    Enrolled,
    /// An authenticated client confirmed successful decryption; cutover may proceed.
    Restored,
}

/// Native response. Cryptographic material and bearer tokens are deliberately not printable.
#[derive(Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case", deny_unknown_fields)]
pub enum Response {
    /// Account lookup decision.
    Discovery {
        /// Closed decision type; never an arbitrary credential URL.
        discovery: Discovery,
    },
    /// OPAQUE KE2 plus a one-use attempt capability.
    LoginChallenge {
        /// Attempt identifier/capability, unrelated to account UUIDs.
        attempt: Token,
        /// OPAQUE KE2.
        message: Vec<u8>,
    },
    /// Issued only after successful KE3 verification and current credential checks.
    Session {
        /// Short-lived revocable central bearer.
        token: Token,
        /// Account and epoch bound into the completed OPAQUE exchange.
        identity: Identity,
        /// Stored client-encrypted master key.
        wrapped_master: Vec<u8>,
    },
    /// Newly reserved or resumable legacy enrollment operation.
    Enrollment {
        /// Central identity reserved by PostgreSQL before client key derivation.
        identity: Identity,
        /// Fresh short-lived enrollment capability, bound to this account.
        ticket: Token,
    },
    /// OPAQUE registration response.
    EnrollmentChallenge {
        /// Suite-specific registration response.
        message: Vec<u8>,
    },
    /// Authenticated encrypted account document.
    Document {
        /// Monotonically increasing compare-and-swap revision.
        revision: u64,
        /// Ciphertext; successful HTTP does not mean the client can decrypt it.
        document: Vec<u8>,
    },
    /// Durable migration status; bridge-authenticated only.
    LegacyStatus {
        /// Current monotone migration stage.
        state: MigrationState,
        /// Immutable central account mapping.
        identity: Identity,
    },
    /// Client-delivered, audience-bound gateway ticket.
    GatewayTicket {
        /// One-use capability with a 30-second lifetime.
        ticket: Token,
    },
    /// Gateway learns only the immutable identity it is permitted to admit.
    GatewayIdentity {
        /// Central account mapping.
        identity: Identity,
        /// Existing legacy account, when this is the migration gateway.
        legacy_id: String,
    },
    /// Mutation committed. Lost replies require an authenticated read before retry.
    Done,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wire_rejects_passwords_unknown_operations_and_unbounded_messages() {
        for bytes in [
            br#"{"operation":"discover","username":"synthetic","password":"never"}"#.as_slice(),
            br#"{"operation":"reset_everything"}"#,
            br#"{"operation":"logout"} trailing"#,
            br#"{"operation":"logout","operation":"read_document"}"#,
        ] {
            assert!(crate::decode::<Request>(bytes).is_err());
        }
        assert!(crate::decode::<Request>(&vec![b' '; crate::MAX_WIRE_BYTES + 1]).is_err());
    }
}
