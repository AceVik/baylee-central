//! The account export as the released Baylee client decodes it: a frozen
//! copy of `central-api` at 4b14dcf0, which Baylee pins (`Cargo.toml`), field
//! for field and with its serde attributes. The client reads the export for
//! the lobby's name after every sign-in, so an answer it cannot decode shows
//! the private sign-in name instead. Never edit these types to follow a
//! change; a change that fails here needs a new read, not a new field.
use super::{AcceptedTerms as Terms, AccountExport, AdministeredGateway, ExportedGateway};
use crate::admin::{AccountState as State, AuditEvent as Event};
use crate::wire::AccountSource as Source;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case", deny_unknown_fields)]
enum Response {
    AccountExport { export: Box<Export> },
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Export {
    account: [u8; 16],
    username: String,
    display_name: Option<String>,
    display_name_at: Option<i64>,
    tag: Option<u32>,
    source: AccountSource,
    legacy_id: Option<String>,
    created_at: i64,
    last_sign_in_at: Option<i64>,
    vault_epoch: u64,
    document_revision: u64,
    state: AccountState,
    admin: bool,
    recovery: bool,
    reset_until: Option<i64>,
    objects: u64,
    object_bytes: u64,
    archives: u64,
    sessions: Vec<ExportedSession>,
    terms: Vec<AcceptedTerms>,
    gateways: Vec<Gateway>,
    administers: Vec<Administered>,
    audit: Vec<AuditEvent>,
    complete: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExportedSession {
    kind: SessionKind,
    created_at: i64,
    expires_at: i64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SessionKind {
    Native,
    Browser,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AcceptedTerms {
    subject: String,
    version: String,
    accepted_at: i64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Gateway {
    id: [u8; 16],
    origin: String,
    name: String,
    version: String,
    website: String,
    approved: bool,
    created_at: i64,
    last_seen: Option<i64>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Administered {
    id: [u8; 16],
    origin: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum AccountSource {
    Legacy,
    Registered,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum AccountState {
    Pending,
    Enrolled,
    Restored,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuditEvent {
    id: [u8; 16],
    at: i64,
    actor: Option<[u8; 16]>,
    action: String,
    account: Option<[u8; 16]>,
    gateway: Option<[u8; 16]>,
    beta_key: Option<[u8; 16]>,
}

/// Every list holds an entry and every option a value, so each nested type
/// is decoded too, and every enum's values are tried.
fn exports() -> Vec<AccountExport> {
    let full = |source, state, kind| AccountExport {
        account: [1; 16],
        username: "erika".into(),
        display_name: Some("Erika".into()),
        display_name_at: Some(1),
        tag: Some(0x1a2b),
        source,
        legacy_id: Some("legacy".into()),
        created_at: 1,
        last_sign_in_at: Some(2),
        vault_epoch: 3,
        document_revision: 4,
        state,
        admin: true,
        recovery: true,
        reset_until: Some(5),
        objects: 6,
        object_bytes: 7,
        archives: 8,
        sessions: vec![super::ExportedSession {
            kind,
            created_at: 9,
            expires_at: 10,
        }],
        terms: vec![Terms {
            subject: "central".into(),
            version: "2026-11-19".into(),
            accepted_at: 11,
        }],
        gateways: vec![ExportedGateway {
            id: [2; 16],
            origin: "https://gateway.test".into(),
            name: "Gateway".into(),
            version: "1".into(),
            website: "https://gateway.test".into(),
            approved: true,
            created_at: 12,
            last_seen: Some(13),
        }],
        administers: vec![AdministeredGateway {
            id: [3; 16],
            origin: "https://gateway.test".into(),
        }],
        audit: vec![Event {
            id: [4; 16],
            at: 14,
            actor: Some([5; 16]),
            action: "account.admin_granted".into(),
            account: Some([1; 16]),
            gateway: Some([2; 16]),
            beta_key: Some([6; 16]),
        }],
        complete: true,
    };
    vec![
        full(Source::Legacy, State::Pending, super::SessionKind::Native),
        full(
            Source::Registered,
            State::Enrolled,
            super::SessionKind::Browser,
        ),
        full(
            Source::Registered,
            State::Restored,
            super::SessionKind::Native,
        ),
    ]
}

#[test]
fn the_released_client_decodes_every_export() {
    for export in exports() {
        let bytes = crate::encode(&crate::wire::Response::AccountExport {
            export: Box::new(export),
        })
        .unwrap();
        let decoded: Response = serde_json::from_slice(&bytes)
            .unwrap_or_else(|error| panic!("the released client fails: {error}"));
        // and nothing of it is lost on the way
        assert_eq!(
            serde_json::to_value(&decoded).unwrap(),
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()
        );
    }
}
