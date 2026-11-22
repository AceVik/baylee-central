//! The legacy store's sign-in and move (`docs/native-login.md` §"Legacy
//! login"): the accounts of the official gateway that had not moved when
//! Central took their sign-in over. [`LegacyPassword`] is the one wire type
//! that carries a password to Central, on the native listener only, and only
//! until the store's deadline; everything else here is what a move is handed
//! and what an administrator sees of what is left.
mod import;
pub use import::valid_import_snapshot;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// The longest password the legacy sign-in takes, in bytes. The gateway took
/// up to 128 characters.
pub const MAX_LEGACY_PASSWORD: usize = 1024;

/// The largest source snapshot: the first snapshot format's bound, which the
/// gateway's export enforced already.
pub const MAX_SNAPSHOT_BYTES: usize = 196_608;

/// Maximum complete source accepted by the offline import.
pub const MAX_ARCHIVE_BYTES: usize = 8 * 1024 * 1024;
/// Maximum original plaintext bytes in one authenticated source response.
pub const ARCHIVE_PAGE_BYTES: usize = 64 * 1024;

/// The bounded enrollment document for a source too large for the account vault.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveSource {
    /// Its native source manifest. Old document readers reject this field.
    pub paged_deck_source: ArchiveManifest,
}

/// Immutable source descriptor. The original source digest is withheld until the
/// client has read and checked the entire operator-staged export.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveManifest {
    /// Source and paging schema, currently one.
    pub version: u8,
    /// Byte length of the exact source including all deck history.
    pub bytes: u32,
    /// Domain-separated commitment to the origin account, source digest and length.
    pub generation: [u8; 32],
}

impl ArchiveManifest {
    /// Whether this manifest is a supported oversized archive.
    #[must_use]
    pub fn valid(&self) -> bool {
        self.version == 1
            && usize::try_from(self.bytes)
                .is_ok_and(|b| (MAX_SNAPSHOT_BYTES + 1..=MAX_ARCHIVE_BYTES).contains(&b))
    }

    /// Number of pages, or zero for an invalid manifest.
    #[must_use]
    pub fn pages(&self) -> u16 {
        if !self.valid() {
            return 0;
        }
        u16::try_from(self.bytes.div_ceil(65_536)).unwrap_or(0)
    }
}

/// Bind an immutable legacy account and source size to its original digest.
#[must_use]
pub fn source_generation(legacy_id: &str, digest: [u8; 32], bytes: u32) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    hash.update(b"baylee:legacy-deck-source:v1\0");
    hash.update(legacy_id.as_bytes());
    hash.update([0]);
    hash.update(bytes.to_be_bytes());
    hash.update(digest);
    hash.finalize().into()
}

/// Digest of the exact source bytes, including all authored history.
#[must_use]
pub fn source_digest(bytes: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes).into()
}

/// Hash the ordered live private-object inventory at the source-to-library cut.
#[must_use]
pub fn object_inventory(objects: &[crate::object::ObjectState]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    hash.update(b"baylee:legacy-deck-objects:v1\0");
    for object in objects.iter().filter(|o| !o.deleted && o.revision > 0) {
        hash.update(object.id);
        hash.update(object.revision.to_be_bytes());
    }
    hash.finalize().into()
}

/// A legacy account's password, sent once to Central's native listener over
/// verified TLS and never stored or logged there. Neither `Debug`, `Display`
/// nor `Clone`; wiped when dropped.
#[derive(Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(transparent)]
pub struct LegacyPassword(String);

impl LegacyPassword {
    /// Take the password as typed; the sign-in refuses an empty or overlong one.
    #[must_use]
    pub const fn new(password: String) -> Self {
        Self(password)
    }

    /// The password's bytes, for the verification and nothing else.
    #[must_use]
    pub fn expose(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Whether the sign-in takes it at all: 1 to [`MAX_LEGACY_PASSWORD`] bytes.
    #[must_use]
    pub fn bounded(&self) -> bool {
        (1..=MAX_LEGACY_PASSWORD).contains(&self.0.len())
    }
}

/// A legacy account's source as the official gateway exported it at the
/// cutover. Central never reads `document`; the client parses it.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacySnapshot {
    /// The account's immutable id at the official gateway.
    pub legacy_id: String,
    /// Its canonical username.
    pub username: String,
    /// SHA-256 of `document`; `LegacyLink` names it back.
    pub digest: [u8; 32],
    /// The gateway's export, UTF-8 JSON
    /// `{"preferences": {...}, "decks": [...]}`, at most [`MAX_SNAPSHOT_BYTES`].
    pub document: Vec<u8>,
}

impl LegacySnapshot {
    /// Whether `digest` is the SHA-256 of `document` and the document is
    /// within bounds: the client checks what it is handed before it
    /// encrypts it.
    #[must_use]
    pub fn verified(&self) -> bool {
        use sha2::{Digest, Sha256};
        self.document.len() <= MAX_SNAPSHOT_BYTES
            && <[u8; 32]>::from(Sha256::digest(&self.document)) == self.digest
    }
}

/// How far a legacy account's move got, as an administrator sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LegacyProgress {
    /// No sign-in reserved its identity yet.
    Unstarted,
    /// Reserved; the client has not enrolled.
    Reserved,
    /// Enrolled; the client has not confirmed its restored document.
    Enrolled,
    /// Restored; the client has not closed the move.
    Restored,
}

/// One account left in the legacy store: names, times and counters, no hash
/// and no snapshot.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyAccount {
    /// Its id at the official gateway.
    pub legacy_id: String,
    /// Its canonical username.
    pub username: String,
    /// Its display name, when it kept the rule.
    pub display_name: Option<String>,
    /// Its tag.
    pub tag: Option<u32>,
    /// How far its move got.
    pub progress: LegacyProgress,
    /// When it was imported, Unix seconds.
    pub imported_at: i64,
    /// Failed sign-ins since the last good one or lockout.
    pub failures: u32,
    /// Until when it is locked out, Unix seconds, while it is.
    pub locked_until: Option<i64>,
    /// The gateway required a new password: its move is refused.
    pub must_change_password: bool,
    /// A ban is in force: its move is refused.
    pub banned: bool,
    /// Its snapshot's size, in bytes.
    pub snapshot_bytes: u32,
}

/// What is left of the legacy store.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyStore {
    /// After this, Unix seconds, no legacy sign-in is taken and the sweep
    /// deletes every row; none until the operator sets one.
    pub deadline: Option<i64>,
    /// Every account left, by username.
    pub accounts: Vec<LegacyAccount>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archive_manifest_bounds_and_generation_bind_the_original_account() {
        let digest = source_digest(b"synthetic original");
        let make = |bytes| ArchiveManifest {
            version: 1,
            bytes,
            generation: source_generation("a", digest, bytes),
        };
        assert!(!make(196_608).valid());
        assert_eq!(make(196_609).pages(), 4);
        assert_eq!(make(8 * 1024 * 1024).pages(), 128);
        assert!(!make(8 * 1024 * 1024 + 1).valid());
        assert_ne!(make(196_610).generation, make(196_609).generation);
        assert_ne!(
            source_generation("b", digest, 196_609),
            make(196_609).generation
        );
        assert_ne!(digest, make(196_609).generation);
    }

    #[test]
    fn the_password_has_no_printable_form_and_crosses_as_a_string() {
        let password = LegacyPassword::new("synthetic".into());
        assert_eq!(crate::encode(&password).unwrap(), b"\"synthetic\"");
        assert!(password.bounded());
        assert!(!LegacyPassword::new(String::new()).bounded());
        assert!(!LegacyPassword::new("x".repeat(MAX_LEGACY_PASSWORD + 1)).bounded());
        assert!(LegacyPassword::new("x".repeat(MAX_LEGACY_PASSWORD)).bounded());
    }

    #[test]
    fn a_snapshot_is_verified_against_its_digest() {
        use sha2::{Digest, Sha256};
        let document = br#"{"decks":[],"preferences":{}}"#.to_vec();
        let mut snapshot = LegacySnapshot {
            legacy_id: "synthetic".into(),
            username: "synthetic".into(),
            digest: Sha256::digest(&document).into(),
            document,
        };
        assert!(snapshot.verified());
        snapshot.document.push(b' ');
        assert!(!snapshot.verified());
    }

    #[test]
    fn a_full_snapshot_fits_the_wire() {
        let snapshot = LegacySnapshot {
            legacy_id: "0".repeat(64),
            username: "a".repeat(24),
            digest: [255; 32],
            document: vec![255; MAX_SNAPSHOT_BYTES],
        };
        let bytes = crate::encode(&crate::wire::Response::LegacySnapshot { snapshot }).unwrap();
        assert!(bytes.len() <= crate::MAX_WIRE_BYTES, "{}", bytes.len());
    }
}
