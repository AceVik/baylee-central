//! Account-owned contacts shared by the website and native client.
//! Only public profiles cross this boundary; login names and account IDs do not.
use serde::{Deserialize, Serialize};

/// Maximum accepted contacts per account.
pub const MAX_CONTACTS: usize = 300;
/// Maximum outstanding requests sent by one account.
pub const MAX_OUTGOING: usize = 50;
/// Maximum outstanding requests received by one account.
pub const MAX_INCOMING: usize = 300;
/// Maximum explicit blocks owned by an account.
pub const MAX_BLOCKS: usize = 1000;

/// Public account identity, resolved by an exact tag, never a private login name.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Person {
    /// Public immutable tag.
    pub tag: u32,
    /// Chosen public name, rendered as text by clients.
    pub display_name: String,
}

/// The caller's relationship with this public profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Relation {
    /// Both accounts accepted the contact.
    Friend,
    /// A request that this account may accept or decline.
    Incoming,
    /// This account's request, which it may cancel.
    Outgoing,
    /// This account explicitly blocked the other account.
    Blocked,
}

/// One entry of an account's private contact list.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contact {
    /// The other account's public identity.
    pub person: Person,
    /// The caller's relationship, never whether the other account blocked it.
    pub relation: Relation,
}

/// An explicit contact transition. Every operation is safe to repeat.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactAction {
    /// Send a request; a crossed request accepts the contact.
    Request,
    /// Accept an incoming request. Does not accept an outgoing one.
    Accept,
    /// Remove a contact or cancel/decline an outstanding request.
    Remove,
    /// Block and atomically remove both requests and the friendship.
    Block,
    /// Remove only this account's block; never restore a former friendship.
    Unblock,
}

/// Social operations require a live session of the presenting listener's kind.
#[derive(Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum SocialRequest {
    /// Private group membership and invitations.
    Group {
        /// Closed group command.
        request: crate::groups::GroupRequest,
    },
    /// End-to-end encrypted chat on the same account-owned service.
    Chat {
        /// Closed encrypted-chat command.
        request: crate::chat::ChatRequest,
    },
    /// Read this account's complete, bounded contacts list.
    Contacts,
    /// Resolve one public tag. There is no account enumeration endpoint.
    Lookup {
        /// Exact public tag.
        tag: u32,
    },
    /// Change a relationship with the account identified by its public tag.
    Contact {
        /// The other account's public tag; self-actions are refused.
        tag: u32,
        /// The requested state transition.
        change: ContactAction,
    },
}

/// Closed social reply vocabulary; no request or reply contains a credential.
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SocialResponse {
    /// Private group result.
    Group {
        /// Closed group response.
        response: crate::groups::GroupResponse,
    },
    /// End-to-end encrypted chat reply.
    Chat {
        /// Closed chat response.
        response: crate::chat::ChatResponse,
    },
    /// Private contacts owned by the authenticated caller.
    Contacts {
        /// Ordered by numeric public tag, with at most one entry per account.
        contacts: Vec<Contact>,
    },
    /// The exact public profile, or none for unknown/unavailable profiles.
    Person {
        /// Never a private username or an unregistered reservation.
        person: Option<Person>,
    },
    /// The requested change completed, or was already complete.
    Done,
}
