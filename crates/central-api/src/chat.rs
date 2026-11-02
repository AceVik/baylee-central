//! Encrypted chat transport. Ciphertext matches Baylee's existing signed envelope.
//! Central validates membership and signatures but never receives a private key.
use serde::{Deserialize, Serialize};

/// Maximum ciphertext body, including its key commitment and authentication tag.
pub const MAX_CIPHERTEXT: usize = 4096 + 32 + 16;
/// Maximum devices registered for an account.
pub const MAX_DEVICES: usize = 5;
/// Deliveries read in one bounded page.
pub const PAGE_SIZE: usize = 50;
/// Pending deliveries retained per device, independent of ciphertext byte limits.
pub const MAX_DELIVERIES: usize = 5000;

/// A signed device statement in the existing native format, base64url without padding.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceRegistration {
    /// Ed25519 public identity key, 32 bytes.
    pub ik: String,
    /// X25519 public encryption key, 32 bytes.
    pub dk: String,
    /// Identity signature binding both keys to the public account tag, 64 bytes.
    pub dsig: String,
    /// Plain, bounded OS-family label, never a hostname or device identifier.
    pub label: String,
}

/// A device authenticated to its Central account and self-signed by its identity key.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceKey {
    /// The native protocol's 16-character signing-key fingerprint.
    pub id: String,
    /// Public Ed25519 key in base64url.
    pub ik: String,
    /// Public X25519 key in base64url.
    pub dk: String,
    /// Signature of the account-bound device statement in base64url.
    pub dsig: String,
    /// OS family, never a machine name.
    pub label: String,
    /// Registration time in Unix milliseconds.
    pub added_at: u64,
}

/// A permitted recipient's current devices.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccountKeys {
    /// Canonical lowercase hexadecimal public tag.
    pub tag: String,
    /// Public display name and tag.
    pub handle: String,
    /// At most five authenticated devices.
    pub devices: Vec<DeviceKey>,
}

/// The exact JSON bytes of this header are signed, never a reserialization.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Header {
    /// Version one.
    pub v: u32,
    /// Native sender-generated `UUIDv7` correlation ID, not the database identity.
    pub id: String,
    /// Canonical conversation ID.
    pub conv: String,
    /// Public sender tag.
    pub from: String,
    /// Signing device fingerprint.
    pub dev: String,
    /// Sender Unix milliseconds, bounded at first acceptance.
    pub ts: u64,
    /// Version one carries `text` bodies only.
    pub kind: String,
}

/// A recipient's encrypted per-message key, matching the native wire shape.
#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wrap {
    /// Recipient device fingerprint.
    #[serde(rename = "d")]
    pub device: String,
    /// Exactly 48 bytes in canonical base64url.
    pub k: String,
}

/// An opaque signed message and one key wrap for each authorized current device.
#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sealed {
    /// At most 512 signed JSON header bytes in base64url.
    pub hdr: String,
    /// 24-byte `XChaCha20` nonce in base64url.
    pub n: String,
    /// Bounded ciphertext, including its 32-byte key commitment.
    pub ct: String,
    /// 32-byte ephemeral X25519 public key in base64url.
    pub epk: String,
    /// 64-byte Ed25519 signature in base64url.
    pub sig: String,
    /// Exact current recipient set; no unauthorized or duplicate wrap is accepted.
    pub wraps: Vec<Wrap>,
}

/// One account-owned delivery. The sender keys allow authenticating retained history.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Delivery {
    /// Commit-ordered device cursor; safe as a JavaScript integer.
    pub seq: u64,
    /// Public sender profile.
    pub sender: super::social::Person,
    /// Sending device, stored with the authenticated message.
    pub device: DeviceKey,
    /// Original signed envelope, with only this recipient's key wrap.
    pub sealed: Sealed,
    /// Server acceptance time in Unix milliseconds.
    pub at: u64,
}

/// Account-owned chat operations; the containing Social request supplies authentication.
#[derive(Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum ChatRequest {
    /// Register a device after verifying its statement. Repeating an identical key is free.
    Register {
        /// Public keys and their account-bound signature, never private material.
        device: DeviceRegistration,
    },
    /// Read authorized current devices, including this account's own devices.
    Keys {
        /// At most 60 distinct public numeric tags.
        tags: Vec<u32>,
    },
    /// Submit an idempotent signed message to an authorized conversation.
    Send {
        /// Canonical conversation, also bound inside the signed header.
        conv: String,
        /// Authenticated ciphertext; Central cannot decrypt it.
        sealed: Sealed,
    },
    /// Read this account/device's durable inbox, without destructive acknowledgement.
    Inbox {
        /// An active device owned by the authenticated account.
        device: String,
        /// Exclusive cursor, zero to replay retained history.
        after: u64,
    },
}

/// Chat responses, never credentials or plaintext private messages.
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ChatResponse {
    /// The account-owned registered device.
    Registered {
        /// Fingerprint derived from the verified identity key.
        id: String,
    },
    /// Authenticated and authorized peer keys.
    Keys {
        /// Unknown or unauthorized peers are omitted.
        keys: Vec<AccountKeys>,
    },
    /// Durable, idempotent server acceptance.
    Sent {
        /// The unchanged signed correlation ID.
        id: String,
        /// Original acceptance time, including on retry.
        at: u64,
    },
    /// One bounded page, in committed delivery order.
    Inbox {
        /// At most `PAGE_SIZE` deliveries belonging to this account/device.
        messages: Vec<Delivery>,
        /// Another page may follow; request it without waiting for the next poll.
        more: bool,
    },
}
