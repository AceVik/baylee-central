//! What Central knows about an account, for the person's own export (GDPR
//! Art. 15 and 20; `docs/account-deletion.md`). Metadata only: no field
//! carries a credential, a wrapper, a digest, a token or a ciphertext. The
//! encrypted half (the account document and the private objects) is read and
//! decrypted by the client.
use crate::admin::{AccountState, AuditEvent};
use crate::wire::AccountSource;
use serde::{Deserialize, Serialize};

/// Most entries of each list in an [`AccountExport`]; a list that holds more
/// is cut there and the export says so with `complete: false`.
pub const EXPORT_LIST: usize = 256;

/// Everything Central stores about one account, apart from its secrets and
/// ciphertext. Times are Unix seconds.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccountExport {
    /// The account's id.
    pub account: [u8; 16],
    /// Its canonical username, the private sign-in name.
    pub username: String,
    /// Its display name, if it chose one, and when.
    pub display_name: Option<String>,
    /// When the display name was chosen.
    pub display_name_at: Option<i64>,
    /// The tag beside the display name.
    pub tag: Option<u32>,
    /// Where the account came from.
    pub source: AccountSource,
    /// The official gateway's id of a moved account.
    pub legacy_id: Option<String>,
    /// When the account was created.
    pub created_at: i64,
    /// Its last password sign-in.
    pub last_sign_in_at: Option<i64>,
    /// The vault epoch its data is encrypted under.
    pub vault_epoch: u64,
    /// The account document's revision.
    pub document_revision: u64,
    /// `enrolled`, or `restored` once a native client confirmed the document.
    pub state: AccountState,
    /// It holds Central's administrator role.
    pub admin: bool,
    /// It has a recovery wrapper.
    pub recovery: bool,
    /// The expiry of an outstanding reset code an administrator made.
    pub reset_until: Option<i64>,
    /// How many private objects it stores (deleted ones not counted).
    pub objects: u64,
    /// The bytes they count against the quota.
    pub object_bytes: u64,
    /// Encrypted copies of earlier vaults a move kept.
    pub archives: u64,
    /// Its sessions, at most one of each kind; no token.
    pub sessions: Vec<ExportedSession>,
    /// The terms it accepted, oldest first.
    pub terms: Vec<AcceptedTerms>,
    /// The gateways it registered.
    pub gateways: Vec<ExportedGateway>,
    /// The gateways whose owners named it an administrator.
    pub administers: Vec<AdministeredGateway>,
    /// The audit events naming it, as the account acted on or as the
    /// administrator who acted, newest first.
    pub audit: Vec<AuditEvent>,
    /// No list was cut at [`EXPORT_LIST`].
    pub complete: bool,
}

/// One session, without its token.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportedSession {
    /// `native` or `browser`.
    pub kind: SessionKind,
    /// Its password sign-in.
    pub created_at: i64,
    /// When it ends unless renewed.
    pub expires_at: i64,
}

/// The listener a session belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionKind {
    /// The native client's bearer session.
    Native,
    /// The website's cookie session.
    Browser,
}

/// One acceptance of terms.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptedTerms {
    /// `central`, or the gateway's origin.
    pub subject: String,
    /// The version accepted.
    pub version: String,
    /// When.
    pub accepted_at: i64,
}

/// A gateway the account registered, with its public metadata.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportedGateway {
    /// The gateway's id.
    pub id: [u8; 16],
    /// Its origin.
    pub origin: String,
    /// Its self-reported name.
    pub name: String,
    /// Its self-reported software version.
    pub version: String,
    /// Its website.
    pub website: String,
    /// An operator approved it.
    pub approved: bool,
    /// When it was registered.
    pub created_at: i64,
    /// Its last heartbeat.
    pub last_seen: Option<i64>,
}

/// A gateway the account administers.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdministeredGateway {
    /// The gateway's id.
    pub id: [u8; 16],
    /// Its origin.
    pub origin: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_largest_export_fits_the_wire() {
        let id = Some([255; 16]);
        let origin = format!("https://{}", "x".repeat(248));
        let export = AccountExport {
            account: [255; 16],
            username: "x".repeat(24),
            display_name: Some("x".repeat(16)),
            display_name_at: Some(i64::MIN),
            tag: Some(u32::MAX),
            source: AccountSource::Legacy,
            legacy_id: Some("\u{1}".repeat(64)),
            created_at: i64::MIN,
            last_sign_in_at: Some(i64::MIN),
            vault_epoch: u64::MAX,
            document_revision: u64::MAX,
            state: AccountState::Restored,
            admin: false,
            recovery: false,
            reset_until: Some(i64::MIN),
            objects: u64::MAX,
            object_bytes: u64::MAX,
            archives: u64::MAX,
            sessions: [SessionKind::Native, SessionKind::Browser]
                .into_iter()
                .map(|kind| ExportedSession {
                    kind,
                    created_at: i64::MIN,
                    expires_at: i64::MIN,
                })
                .collect(),
            terms: (0..EXPORT_LIST)
                .map(|_| AcceptedTerms {
                    subject: "\u{1}".repeat(256),
                    version: "\"".repeat(64),
                    accepted_at: i64::MIN,
                })
                .collect(),
            gateways: (0..8)
                .map(|_| ExportedGateway {
                    id: [255; 16],
                    origin: origin.clone(),
                    name: "\u{1}".repeat(80),
                    version: "\u{1}".repeat(64),
                    website: "\u{1}".repeat(512),
                    approved: false,
                    created_at: i64::MIN,
                    last_seen: Some(i64::MIN),
                })
                .collect(),
            administers: (0..EXPORT_LIST)
                .map(|_| AdministeredGateway {
                    id: [255; 16],
                    origin: origin.clone(),
                })
                .collect(),
            audit: (0..EXPORT_LIST)
                .map(|_| AuditEvent {
                    id: [255; 16],
                    at: i64::MIN,
                    actor: id,
                    action: "x".repeat(65),
                    account: id,
                    gateway: id,
                    beta_key: id,
                })
                .collect(),
            complete: false,
        };
        let bytes = crate::encode(&crate::wire::Response::AccountExport {
            export: Box::new(export),
        })
        .unwrap();
        assert!(bytes.len() <= crate::MAX_WIRE_BYTES, "{}", bytes.len());
        assert!(crate::decode::<crate::wire::Response>(&bytes).is_ok());
    }
}
