//! Private, account-owned chat groups. Invitations never grant message access.
use crate::social::Person;
use serde::{Deserialize, Serialize};

/// Maximum memberships and outstanding invitations combined per account.
pub const MAX_GROUPS: usize = 20;
/// Maximum members and outstanding invitations combined per group.
pub const MAX_MEMBERS: usize = 50;
/// Maximum group description length in Unicode scalar values.
pub const MAX_DESCRIPTION: usize = 280;

/// A current group member.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Member {
    /// Public identity.
    pub person: Person,
    /// Whether this member may transfer or delete the group.
    pub owner: bool,
    /// Whether this member holds administrator privileges in the group.
    #[serde(default)]
    pub admin: bool,
}

/// Group metadata visible only to accepted members.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    /// Server-generated `UUIDv7`; the conversation is `grp:<id>`.
    pub id: String,
    /// Bounded display text, never markup.
    pub name: String,
    /// Optional group description, at most [`MAX_DESCRIPTION`] characters.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// Current group picture key, if set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub picture_key: Option<crate::picture::PictureKey>,
    /// Whether the group picture moves (has a `.sequence.avif`).
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub picture_moving: bool,
    /// Accepted members in join order.
    pub members: Vec<Member>,
    /// Pending invitees, not authorized recipients.
    pub invited: Vec<Person>,
}

/// An invitation reveals its inviter and group metadata, not the other members.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Invite {
    /// Server-generated group ID.
    pub group: String,
    /// Group display name.
    pub name: String,
    /// Optional group description, at most [`MAX_DESCRIPTION`] characters.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// Current group picture key, if set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub picture_key: Option<crate::picture::PictureKey>,
    /// Whether the group picture moves (has a `.sequence.avif`).
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub picture_moving: bool,
    /// Public identity of the inviting member.
    pub inviter: Person,
    /// Server invitation time in Unix milliseconds.
    pub at: u64,
}

/// Bounded private group list belonging to the authenticated caller.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Groups {
    /// Groups this account has explicitly joined.
    pub groups: Vec<Group>,
    /// Invitations this account may accept or decline.
    pub invites: Vec<Invite>,
}

/// One authorized group transition.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "change", rename_all = "snake_case", deny_unknown_fields)]
pub enum Change {
    /// Admin or owner changes the group's public-to-members label.
    Rename {
        /// New display text, 1–48 characters.
        name: String,
    },
    /// Admin or owner changes the group's description.
    SetDescription {
        /// New description text, 0–280 characters.
        description: String,
    },
    /// Admin or owner grants or revokes administrator status for a joined member.
    SetAdmin {
        /// Member's public tag; the group owner stays an administrator permanently.
        tag: u32,
        /// Whether the member should hold administrator privileges.
        admin: bool,
    },
    /// Admin or owner removes the group's picture.
    RemovePicture,
    /// A joined member invites a non-blocked enrolled player.
    Invite {
        /// Invitee's public tag.
        tag: u32,
    },
    /// The invitee accepts its own invitation.
    Join,
    /// Invitee declines; inviter, admin or owner withdraws an invitation.
    Withdraw {
        /// Invitee's public tag.
        tag: u32,
    },
    /// A member leaves, or an owner/admin removes another eligible member.
    Remove {
        /// Member's public tag. An owner must transfer or delete before leaving.
        tag: u32,
    },
    /// Owner transfers ownership to another accepted member.
    Transfer {
        /// New owner's public tag.
        tag: u32,
    },
    /// Owner deletes the group. Previously delivered ciphertext is retained.
    Delete,
}

/// Group commands share the containing social request's authentication.
#[derive(Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum GroupRequest {
    /// List the caller's groups and invitations.
    List,
    /// Create a group with the caller as its sole member and owner.
    Create {
        /// Display text, 1–48 characters without hidden formatting.
        name: String,
    },
    /// Change an existing group under its membership lock.
    Change {
        /// Canonical `UUIDv7` group ID.
        id: String,
        /// Requested transition.
        change: Change,
    },
}

/// Closed group response vocabulary.
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GroupResponse {
    /// Consistent membership snapshot.
    Groups {
        /// The caller's private group metadata.
        snapshot: Groups,
    },
    /// Newly created group.
    Created {
        /// `UUIDv7` from PostgreSQL `INSERT RETURNING`.
        id: String,
    },
    /// Authorized transition committed.
    Done,
}

fn visible_char(c: char) -> bool {
    !c.is_control()
        && !matches!(
            c,
            '\u{00ad}'
                | '\u{061c}'
                | '\u{180e}'
                | '\u{200b}'..='\u{200f}'
                | '\u{2028}'..='\u{202e}'
                | '\u{2060}'..='\u{206f}'
                | '\u{feff}'
                | '\u{fff9}'..='\u{fffb}'
        )
}

/// A nonblank group name with at most 48 visible Unicode scalar values.
/// Text is preserved; native clients may normalize it before submission.
#[must_use]
pub fn valid_name(name: &str) -> bool {
    !name.trim().is_empty() && name.chars().count() <= 48 && name.chars().all(visible_char)
}

/// A group description of 0 to [`MAX_DESCRIPTION`] Unicode scalar values,
/// allowing line breaks while refusing control and hidden formatting characters.
#[must_use]
pub fn valid_description(text: &str) -> bool {
    text.chars().count() <= MAX_DESCRIPTION && text.chars().all(|c| c == '\n' || visible_char(c))
}

#[cfg(test)]
mod tests {
    use super::{MAX_DESCRIPTION, valid_description, valid_name};

    #[test]
    fn group_labels_bound_unicode_and_refuse_hidden_formatting() {
        for name in ["Freunde", "日本語", " Café ", &"é".repeat(48)] {
            assert!(valid_name(name));
        }
        for name in ["", "  ", "a\n", "a\u{202e}b", "a\u{200b}b", &"é".repeat(49)] {
            assert!(!valid_name(name));
        }
    }

    #[test]
    fn group_descriptions_allow_empty_and_newlines_within_bounds() {
        for text in [
            "",
            "Commander night\nFridays at 20:00",
            &"é".repeat(MAX_DESCRIPTION),
        ] {
            assert!(valid_description(text));
        }
        for text in [
            "a\rb",
            "a\tb",
            "a\u{0}b",
            "a\u{202e}b",
            "a\u{200b}b",
            &"é".repeat(MAX_DESCRIPTION + 1),
        ] {
            assert!(!valid_description(text));
        }
    }
}
