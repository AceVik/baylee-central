//! Central administration (`docs/administration.md`): what an administrator
//! asks and is answered, on the browser listener only, with a browser session
//! whose account holds the administrator role. No answer has a field that
//! could carry a credential, a wrapper, a document or any other ciphertext.
use crate::Error;
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
}

impl AdminRequest {
    /// Whether the request changes anything: a change needs a recent
    /// password sign-in and writes its audit event. Every action is named,
    /// so a new one does not compile until it is decided here.
    #[must_use]
    pub const fn changes(&self) -> bool {
        match self {
            Self::SetAdmin { .. } | Self::ResetLogin { .. } => true,
            Self::Accounts { .. } | Self::Account { .. } | Self::Audit { .. } => false,
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
    },
    /// A reset code, shown to the administrator once; Central keeps its digest.
    ResetCode {
        /// The code to hand over.
        code: ResetCode,
        /// Until when it is valid, in Unix seconds.
        expires_at: i64,
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
    fn only_set_admin_changes_anything() {
        assert!(
            AdminRequest::SetAdmin {
                account: [0; 16],
                admin: true
            }
            .changes()
        );
        assert!(AdminRequest::ResetLogin { account: [0; 16] }.changes());
        for read in [
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
        };
        let text = String::from_utf8(crate::encode(&answer).unwrap()).unwrap();
        assert!(text.starts_with(r#"{"answer":"audit","events":[{"id":[1,"#));
        assert!(crate::decode::<AdminAnswer>(text.as_bytes()).is_ok());
        assert!(
            crate::decode::<AdminAnswer>(
                text.replace(r#""beta_key":null"#, r#""beta_key":null,"note":"x""#)
                    .as_bytes()
            )
            .is_err()
        );
    }
}
