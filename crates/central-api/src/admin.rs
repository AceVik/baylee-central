//! Central administration (`docs/administration.md`): what an administrator
//! asks and is answered, on the browser listener only, with a browser session
//! whose account holds the administrator role. No answer has a field that
//! could carry a credential, a wrapper, a document or any other ciphertext.
use crate::Error;
use crate::gateway::{GatewayMetadata, GatewayState};
use crate::owner::TokenInfo;
use crate::reset_code::ResetCode;
use crate::wire::AccountSource;
use serde::{Deserialize, Serialize};

/// Accounts and audit events per page.
pub const PAGE: usize = 50;
/// Audit events and owned gateways shown with one account.
pub const ACCOUNT_LIST: usize = 20;
/// The longest search, in bytes.
pub const MAX_QUERY: usize = 64;

/// An administrator's request, tagged by `action`.
#[derive(Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum AdminRequest {
    /// Accounts, newest first, [`PAGE`] at a time: all, or those whose
    /// username or display name contains the query (without case), or the one
    /// with a `#tag`. Answered with [`AdminAnswer::Accounts`].
    Accounts {
        /// Text or `#tag` ([`AccountQuery::parse`]); none lists all.
        query: Option<String>,
        /// The last account of the page before, for the next.
        before: Option<[u8; 16]>,
    },
    /// One account with its recovery state, the gateways it owns and its
    /// latest audit events. Answered with [`AdminAnswer::Account`].
    Account {
        /// The account's id.
        account: [u8; 16],
    },
    /// Grant or remove the administrator role: a change, so it needs a
    /// password sign-in less than 30 minutes old. Nobody removes their own
    /// role, so the last administrator stays; only an enrolled account is
    /// appointed. The same state again is `Done` and writes nothing.
    SetAdmin {
        /// The account's id.
        account: [u8; 16],
        /// Whether it holds the role afterwards.
        admin: bool,
    },
    /// Make a reset code for an enrolled account whose move is finished and
    /// which is neither an administrator's nor one's own
    /// (`docs/account-reset.md`): a change. A new code replaces the previous
    /// one; issuing ends nothing. Answered with [`AdminAnswer::ResetCode`].
    ResetLogin {
        /// The account's id.
        account: [u8; 16],
    },
    /// Audit events, newest first, [`PAGE`] at a time, of one account or all.
    /// Answered with [`AdminAnswer::Audit`].
    Audit {
        /// Only the events naming this account.
        account: Option<[u8; 16]>,
        /// The last event of the page before, for the next.
        before: Option<[u8; 16]>,
    },
    /// Registered gateways, newest first, [`PAGE`] at a time, all or those
    /// in one state. Answered with [`AdminAnswer::Gateways`].
    Gateways {
        /// Only gateways in this state.
        state: Option<GatewayState>,
        /// The last gateway of the page before, for the next.
        before: Option<[u8; 16]>,
    },
    /// One gateway with its tokens. Answered with [`AdminAnswer::Gateway`].
    Gateway {
        /// The gateway's id.
        gateway: [u8; 16],
    },
    /// Approve (or enable again) a gateway, or disable it: a change. Approval
    /// stays a person's decision after checking outside Central who runs the
    /// origin (`docs/gateway-registry.md`). Disabling hides it from the
    /// directory, stops its credentials and tokens, and ends its players'
    /// sign-ins there. The same state again is `Done` and writes nothing.
    SetGatewayApproval {
        /// The gateway's id.
        gateway: [u8; 16],
        /// Approved and enabled afterwards.
        enabled: bool,
    },
    /// Revoke one gateway token at once: a change. `Done` again for one
    /// revoked already.
    RevokeGatewayToken {
        /// The token's id.
        token: [u8; 16],
    },
    /// Give an enrolled account another tag: a change. The tag stays unique
    /// at Central (`Conflict` when another account holds it), so the handle
    /// `Name#tag` stays unique; gateways take it at the account's next
    /// sign-in there. The same tag again is `Done` and writes nothing.
    SetTag {
        /// The account's id.
        account: [u8; 16],
        /// Its tag afterwards, 1 to [`crate::profile::MAX_TAG`].
        tag: u32,
    },
    /// What is left of the legacy store (`docs/native-login.md` §"Legacy
    /// login"): its deadline and every account not moved yet, without hashes
    /// or snapshots. Answered with [`AdminAnswer::LegacyAccounts`]; refused
    /// when this Central has no legacy store.
    LegacyAccounts,
    /// The administration's first view: how many accounts and gateways
    /// there are, by kind and state. Counts only, no row. Answered with
    /// [`AdminAnswer::Overview`].
    Overview,
}

impl AdminRequest {
    /// Whether the request changes anything: a change needs a recent
    /// password sign-in and writes its audit event. Every action is named,
    /// so a new one does not compile until it is decided here.
    #[must_use]
    pub const fn changes(&self) -> bool {
        match self {
            Self::SetAdmin { .. }
            | Self::ResetLogin { .. }
            | Self::SetGatewayApproval { .. }
            | Self::RevokeGatewayToken { .. }
            | Self::SetTag { .. } => true,
            Self::Accounts { .. }
            | Self::Account { .. }
            | Self::Audit { .. }
            | Self::Gateways { .. }
            | Self::Gateway { .. }
            | Self::LegacyAccounts
            | Self::Overview => false,
        }
    }
}

/// A search over accounts.
#[derive(Debug, PartialEq, Eq)]
pub enum AccountQuery {
    /// Part of a username or display name, lowercase.
    Text(String),
    /// A tag, written `#3fa2`.
    Tag(u32),
}

impl AccountQuery {
    /// Read a search: `#` and one to eight hex digits is a tag, anything else
    /// of 1 to [`MAX_QUERY`] bytes of printable ASCII (spaces trimmed) is
    /// text, compared without case.
    ///
    /// # Errors
    /// `Rejected` for an empty, overlong, non-ASCII or control-bearing query
    /// and a malformed tag.
    pub fn parse(query: &str) -> Result<Self, Error> {
        let query = query.trim();
        if query.is_empty()
            || query.len() > MAX_QUERY
            || !query.bytes().all(|b| b.is_ascii_graphic() || b == b' ')
        {
            return Err(Error::Rejected);
        }
        if let Some(hex) = query.strip_prefix('#') {
            if hex.is_empty() || hex.len() > 8 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(Error::Rejected);
            }
            let tag = u32::from_str_radix(hex, 16).map_err(|_| Error::Rejected)?;
            return if tag == 0 || tag > crate::profile::MAX_TAG {
                Err(Error::Rejected)
            } else {
                Ok(Self::Tag(tag))
            };
        }
        Ok(Self::Text(query.to_ascii_lowercase()))
    }
}

/// How far an account got.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountState {
    /// Reserved, not enrolled yet.
    Pending,
    /// Enrolled; its first document not yet confirmed by a native client.
    Enrolled,
    /// A native client restored and confirmed its document.
    Restored,
}

/// One account as an administrator sees it: names and times, nothing encrypted.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminAccount {
    /// The account's id.
    pub id: [u8; 16],
    /// Its canonical username, the private sign-in name.
    pub username: String,
    /// Its display name, if it has one.
    pub display_name: Option<String>,
    /// Its tag, if it has one.
    pub tag: Option<u32>,
    /// Where it came from.
    pub source: AccountSource,
    /// How far it got.
    pub state: AccountState,
    /// When it was reserved, in Unix seconds.
    pub created_at: i64,
    /// Its last password sign-in, in Unix seconds, if any since Central kept it.
    pub last_sign_in_at: Option<i64>,
    /// It holds the administrator role.
    pub admin: bool,
    /// Until when an unused reset code is valid, in Unix seconds, if one is.
    pub reset_until: Option<i64>,
}

/// A gateway an account owns.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedGateway {
    /// The gateway's id.
    pub id: [u8; 16],
    /// Its origin.
    pub origin: String,
    /// An operator approved it.
    pub approved: bool,
}

/// A registered gateway as an administrator sees it.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminGateway {
    /// The gateway's id.
    pub id: [u8; 16],
    /// Its origin.
    pub origin: String,
    /// Its public metadata.
    pub metadata: GatewayMetadata,
    /// It is the official gateway, the bridge's.
    pub official: bool,
    /// The account that registered it; none for the official gateway.
    pub owner: Option<[u8; 16]>,
    /// That account's display name, if it has one.
    pub owner_name: Option<String>,
    /// That account's tag, if it has one.
    pub owner_tag: Option<u32>,
    /// Whether it is approved.
    pub state: GatewayState,
    /// A heartbeat arrived in the last 90 seconds.
    pub online: bool,
    /// Its last heartbeat, in Unix seconds, if any.
    pub last_seen: Option<i64>,
    /// When it was registered, in Unix seconds.
    pub created_at: i64,
    /// Its live tokens.
    pub live_tokens: u32,
    /// It still has the credential pair of `RegisterGateway`.
    pub credential_pair: bool,
}

/// How many accounts and gateways there are, for the administration's first
/// view. Accounts count once enrolled; gateways are the registered ones,
/// the official gateway not among them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminOverview {
    /// Enrolled accounts.
    pub accounts: u64,
    /// Of them, created in the last seven days.
    pub accounts_week: u64,
    /// Of them, moved from the official gateway.
    pub moved: u64,
    /// Accounts holding the administrator role.
    pub admins: u64,
    /// Registered gateways awaiting approval.
    pub pending: u64,
    /// Registered gateways approved and enabled.
    pub approved: u64,
    /// Registered gateways disabled.
    pub disabled: u64,
    /// Registered gateways heard from in the last 90 seconds.
    pub online: u64,
    /// Gateway tokens neither revoked nor expired.
    pub live_tokens: u64,
}

/// An account an audit page names, as it is called today: the record keeps
/// ids only, the read adds the names (a renamed account shows its new name,
/// a deleted one none).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamedAccount {
    /// The account's id.
    pub id: [u8; 16],
    /// Its display name, if it has one.
    pub display_name: Option<String>,
    /// Its tag, if it has one.
    pub tag: Option<u32>,
}

/// A gateway an audit page names, as it is called today.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamedGateway {
    /// The gateway's id.
    pub id: [u8; 16],
    /// Its name.
    pub name: String,
    /// Its origin.
    pub origin: String,
}

/// One audit event: who, what, and the ids it touched; never a word.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditEvent {
    /// The event's id, ordered by time.
    pub id: [u8; 16],
    /// When, in Unix seconds.
    pub at: i64,
    /// The administrator who acted, or none for the operator.
    pub actor: Option<[u8; 16]>,
    /// What was done, such as `account.admin_granted`.
    pub action: String,
    /// The account it touched, if any.
    pub account: Option<[u8; 16]>,
    /// The gateway it touched, if any.
    pub gateway: Option<[u8; 16]>,
    /// The beta key it touched, if any.
    pub beta_key: Option<[u8; 16]>,
}

/// The answer to an [`AdminRequest`] that reads; a change answers `Done`.
#[derive(Serialize, Deserialize)]
#[serde(tag = "answer", rename_all = "snake_case", deny_unknown_fields)]
pub enum AdminAnswer {
    /// A page of accounts.
    Accounts {
        /// At most [`PAGE`], newest first.
        accounts: Vec<AdminAccount>,
        /// Another page follows.
        more: bool,
    },
    /// One account.
    Account {
        /// Its row.
        account: AdminAccount,
        /// It has a recovery wrapper.
        recovery: bool,
        /// The gateways it owns, at most [`ACCOUNT_LIST`].
        gateways: Vec<OwnedGateway>,
        /// Its latest audit events, at most [`ACCOUNT_LIST`], newest first.
        audit: Vec<AuditEvent>,
    },
    /// A page of audit events.
    Audit {
        /// At most [`PAGE`], newest first.
        events: Vec<AuditEvent>,
        /// Another page follows.
        more: bool,
        /// The accounts the page names (actors and subjects) that still
        /// exist, each once.
        accounts: Vec<NamedAccount>,
        /// The gateways the page names that still exist, each once.
        gateways: Vec<NamedGateway>,
    },
    /// A page of gateways.
    Gateways {
        /// At most [`PAGE`], newest first.
        gateways: Vec<AdminGateway>,
        /// Another page follows.
        more: bool,
    },
    /// One gateway.
    Gateway {
        /// Its row.
        gateway: AdminGateway,
        /// Its tokens, newest first, at most [`ACCOUNT_LIST`].
        tokens: Vec<TokenInfo>,
    },
    /// A reset code, shown to the administrator once; Central keeps its digest.
    ResetCode {
        /// The code to hand over.
        code: ResetCode,
        /// Until when it is valid, in Unix seconds.
        expires_at: i64,
    },
    /// What is left of the legacy store.
    LegacyAccounts {
        /// Its deadline and its accounts.
        store: crate::legacy::LegacyStore,
    },
    /// The counts of the administration's first view.
    Overview {
        /// Accounts and gateways by kind and state.
        overview: AdminOverview,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_query_is_text_or_a_tag_and_nothing_unbounded() {
        assert_eq!(
            AccountQuery::parse("  Alice ").unwrap(),
            AccountQuery::Text("alice".into())
        );
        assert_eq!(
            AccountQuery::parse("#3fA2").unwrap(),
            AccountQuery::Tag(0x3fa2)
        );
        assert_eq!(AccountQuery::parse("#7").unwrap(), AccountQuery::Tag(7));
        assert_eq!(
            AccountQuery::parse("50%_x").unwrap(),
            AccountQuery::Text("50%_x".into())
        );
        let long = "a".repeat(MAX_QUERY + 1);
        for bad in [
            "",
            "   ",
            "#",
            "#0",
            "#123456789",
            "#zz",
            "#80000000",
            "tab\there",
            "nul\0",
            "ä",
            long.as_str(),
        ] {
            assert_eq!(AccountQuery::parse(bad), Err(Error::Rejected), "{bad:?}");
        }
        assert!(AccountQuery::parse(&"a".repeat(MAX_QUERY)).is_ok());
    }

    #[test]
    fn the_changes_are_named_and_the_reads_are_not() {
        assert!(
            AdminRequest::SetAdmin {
                account: [0; 16],
                admin: true
            }
            .changes()
        );
        assert!(AdminRequest::ResetLogin { account: [0; 16] }.changes());
        for change in [
            AdminRequest::SetGatewayApproval {
                gateway: [0; 16],
                enabled: false,
            },
            AdminRequest::RevokeGatewayToken { token: [0; 16] },
            AdminRequest::SetTag {
                account: [0; 16],
                tag: 7,
            },
        ] {
            assert!(change.changes());
        }
        for read in [
            AdminRequest::Gateways {
                state: Some(GatewayState::Pending),
                before: None,
            },
            AdminRequest::Gateway { gateway: [0; 16] },
            AdminRequest::Accounts {
                query: None,
                before: None,
            },
            AdminRequest::Account { account: [0; 16] },
            AdminRequest::Audit {
                account: None,
                before: None,
            },
        ] {
            assert!(!read.changes());
        }
    }

    #[test]
    fn the_requests_and_answers_have_the_designed_shapes() {
        let request: AdminRequest = crate::decode(
            br#"{"action":"set_admin","account":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],"admin":false}"#,
        )
        .unwrap();
        assert!(matches!(
            request,
            AdminRequest::SetAdmin { admin: false, .. }
        ));
        for bad in [
            br#"{"action":"set_admin","account":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]}"#.as_slice(),
            br#"{"action":"accounts","query":null,"before":null,"role":"admin"}"#,
            br#"{"action":"delete_everything"}"#,
        ] {
            assert!(crate::decode::<AdminRequest>(bad).is_err());
        }
        let answer = AdminAnswer::Audit {
            events: vec![AuditEvent {
                id: [1; 16],
                at: 1_800_000_000,
                actor: None,
                action: "account.admin_granted".into(),
                account: Some([2; 16]),
                gateway: None,
                beta_key: None,
            }],
            more: false,
            accounts: vec![NamedAccount {
                id: [2; 16],
                display_name: Some("Synthetic".into()),
                tag: Some(0x3fa2),
            }],
            gateways: Vec::new(),
        };
        let text = String::from_utf8(crate::encode(&answer).unwrap()).unwrap();
        assert!(text.starts_with(r#"{"answer":"audit","events":[{"id":[1,"#));
        assert!(text.contains(r#""accounts":[{"id":[2,"#));
        assert!(crate::decode::<AdminAnswer>(text.as_bytes()).is_ok());
        assert!(
            crate::decode::<AdminAnswer>(
                text.replace(r#""beta_key":null"#, r#""beta_key":null,"note":"x""#)
                    .as_bytes()
            )
            .is_err()
        );
        // the overview: counts only, every field named
        let overview = AdminAnswer::Overview {
            overview: AdminOverview {
                accounts: 12,
                accounts_week: 3,
                moved: 4,
                admins: 2,
                pending: 1,
                approved: 5,
                disabled: 0,
                online: 4,
                live_tokens: 7,
            },
        };
        let text = String::from_utf8(crate::encode(&overview).unwrap()).unwrap();
        assert!(text.starts_with(r#"{"answer":"overview","overview":{"accounts":12,"#));
        assert!(
            crate::decode::<AdminAnswer>(
                text.replace(r#""online":4"#, r#""online":4,"rows":[]"#)
                    .as_bytes()
            )
            .is_err()
        );
        let request = crate::decode::<AdminRequest>(br#"{"action":"overview"}"#).unwrap();
        assert!(matches!(request, AdminRequest::Overview) && !request.changes());
    }
}
