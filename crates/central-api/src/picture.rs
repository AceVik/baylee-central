//! Profile pictures (`docs/profile-pictures.md`): what an account learns of
//! its own picture, the reports it filed, a picture report, and an
//! administrator's moderation. The pictures themselves never cross this
//! wire: they are uploaded and served on routes of their own, and every
//! answer names a picture only by its random key.
use serde::{Deserialize, Serialize};

/// A picture's key: 16 bytes of independent randomness, new at every
/// upload; it names the picture in its URLs and is never an account's id.
pub type PictureKey = [u8; 16];

/// The SHA-256 of a picture's PNG master: what reports and decisions name.
pub type PictureDigest = [u8; 32];

/// Most tags one [`crate::social::SocialRequest::Pictures`] may ask for.
pub const MAX_PICTURE_TAGS: usize = 300;
/// The longest note of a report, in characters.
pub const MAX_REPORT_NOTE: usize = 1_000;
/// The longest note an administrator writes to the person, in characters.
pub const MAX_NOTICE_NOTE: usize = 500;
/// The longest upload block in days; longer is "until lifted".
pub const MAX_BLOCK_DAYS: u16 = 365;
/// Reports per page of the administrators' queue.
pub const REPORT_PAGE: usize = 50;

/// Why a picture is reported or removed: the closed list the page has
/// words for.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PictureReason {
    /// Sexual content.
    Sexual,
    /// Violence or gore.
    Violence,
    /// Hate or extremist symbols.
    Hate,
    /// Harassment of someone.
    Harassment,
    /// Pretends to be someone else.
    Impersonation,
    /// Shows private information or another person without consent.
    Private,
    /// Anything else, which the note explains.
    Other,
}

impl PictureReason {
    /// Every reason, in the page's order.
    pub const ALL: [Self; 7] = [
        Self::Sexual,
        Self::Violence,
        Self::Hate,
        Self::Harassment,
        Self::Impersonation,
        Self::Private,
        Self::Other,
    ];

    /// The stored name (the schema's CHECK lists the same).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sexual => "sexual",
            Self::Violence => "violence",
            Self::Hate => "hate",
            Self::Harassment => "harassment",
            Self::Impersonation => "impersonation",
            Self::Private => "private",
            Self::Other => "other",
        }
    }

    /// The reason a stored name names.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|reason| reason.as_str() == name)
    }
}

/// Where a report stands.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PictureReportState {
    /// Waiting for an administrator.
    Open,
    /// An administrator kept the picture.
    Kept,
    /// An administrator removed the picture.
    Removed,
    /// The person replaced or removed the picture before a decision.
    Gone,
}

impl PictureReportState {
    /// The stored name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Kept => "kept",
            Self::Removed => "removed",
            Self::Gone => "gone",
        }
    }

    /// The state a stored name names.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        [Self::Open, Self::Kept, Self::Removed, Self::Gone]
            .into_iter()
            .find(|state| state.as_str() == name)
    }
}

/// A picture as its owner and administrators see its facts.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Picture {
    /// The key in its URLs.
    pub key: PictureKey,
    /// Its side in pixels (256).
    pub side: u16,
    /// The AVIF's size in bytes.
    pub avif_bytes: u32,
    /// The PNG master's size in bytes.
    pub png_bytes: u32,
    /// The PNG master's SHA-256.
    pub sha256: PictureDigest,
    /// When it was set, in Unix seconds.
    pub set_at: i64,
}

/// An upload block an administrator set.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum UploadBlock {
    /// Uploads are refused until then (Unix seconds).
    Until {
        /// The end of the block.
        until: i64,
    },
    /// Uploads are refused until an administrator lifts the block.
    UntilLifted,
}

/// What the person is told of an administrator's removal: when, why, and
/// the administrator's sentence; never who reported the picture.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PictureNotice {
    /// When the picture was removed, in Unix seconds.
    pub at: i64,
    /// The reason from the list.
    pub reason: PictureReason,
    /// The administrator's sentence, if any.
    pub note: Option<String>,
}

/// A report this account filed, for its own export.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FiledReport {
    /// The reason chosen.
    pub reason: PictureReason,
    /// The note written, if any.
    pub note: Option<String>,
    /// When it was filed, in Unix seconds.
    pub created_at: i64,
    /// Where it stands.
    pub state: PictureReportState,
}

/// A tag's current picture key, from [`crate::social::SocialRequest::Pictures`].
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaggedPicture {
    /// The account's public tag.
    pub tag: u32,
    /// Its picture's key.
    pub key: PictureKey,
}

/// An account in the moderation views, as it is called today (a deleted
/// one is absent).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Named {
    /// The account's id.
    pub id: [u8; 16],
    /// Its display name, if it has one.
    pub display_name: Option<String>,
    /// Its tag, if it has one.
    pub tag: Option<u32>,
}

/// A report as administrators see it: with its reporter, who the reported
/// person is never told of.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminPictureReport {
    /// The report's id.
    pub id: [u8; 16],
    /// The account whose picture was reported.
    pub account: Named,
    /// The reported picture's digest.
    pub sha256: PictureDigest,
    /// The current picture's key, when the reported picture is still the
    /// account's current one.
    pub current: Option<PictureKey>,
    /// Who reported it; none once that account was deleted.
    pub reporter: Option<Named>,
    /// The reason chosen.
    pub reason: PictureReason,
    /// The reporter's note.
    pub note: Option<String>,
    /// When it was filed, in Unix seconds.
    pub created_at: i64,
    /// Where it stands.
    pub state: PictureReportState,
    /// When it was decided, in Unix seconds.
    pub decided_at: Option<i64>,
}

/// What an administrator decides about a reported picture.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case", deny_unknown_fields)]
pub enum PictureDecision {
    /// The picture stays; its open reports become `kept`.
    Keep,
    /// The picture is deleted at once and the person told why; its open
    /// reports become `removed`.
    Remove {
        /// The reason the person is told.
        reason: PictureReason,
        /// A sentence the person is told, at most [`MAX_NOTICE_NOTE`].
        note: Option<String>,
        /// Whether, and how long, new uploads are refused.
        block: BlockChoice,
    },
}

/// An upload block chosen with a removal.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "block", rename_all = "snake_case", deny_unknown_fields)]
pub enum BlockChoice {
    /// No block.
    None,
    /// For 1 to [`MAX_BLOCK_DAYS`] days.
    Days {
        /// How many.
        days: u16,
    },
    /// Until an administrator lifts it.
    UntilLifted,
}

/// An account's request about its own picture
/// ([`crate::wire::Request::Picture`]).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum PictureRequest {
    /// Its facts, an administrator's notice of a removal, and an upload
    /// block. Answered with [`crate::wire::Response::MyPicture`].
    Mine,
    /// Remove it: both of its URLs answer 404 at once. `Done`, also
    /// without one.
    Remove,
    /// The picture reports this account filed, for its export. Answered
    /// with [`crate::wire::Response::MyPictureReports`].
    Reports,
}

/// An administrator's picture request ([`crate::admin::AdminRequest::Picture`]).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum PictureAdmin {
    /// Picture reports, newest first, [`REPORT_PAGE`] at a time: all, or
    /// those in one state. Answered with
    /// [`crate::admin::AdminAnswer::PictureReports`].
    Reports {
        /// Only reports in this state.
        state: Option<PictureReportState>,
        /// The last report of the page before, for the next.
        before: Option<[u8; 16]>,
    },
    /// One account's picture case: its current picture, the reports about
    /// it, its notice and its block. Answered with
    /// [`crate::admin::AdminAnswer::PictureCase`].
    Case {
        /// The account.
        account: [u8; 16],
    },
    /// Keep or remove the account's picture with this digest, as reported.
    /// Needs a password sign-in less than 30 minutes old; audited
    /// (`picture.kept`, `picture.removed`, `picture.blocked`). `Done`;
    /// `Conflict` when the account's picture is no longer that one.
    Decide {
        /// The account.
        account: [u8; 16],
        /// The PNG master's SHA-256 the decision is about.
        sha256: PictureDigest,
        /// The decision.
        decision: PictureDecision,
    },
    /// Lift an account's upload block. Needs a fresh password sign-in;
    /// audited (`picture.unblocked`). `Done`, also without a block.
    LiftBlock {
        /// The account.
        account: [u8; 16],
    },
}

impl PictureAdmin {
    /// Whether the request changes anything (and so needs a fresh
    /// sign-in and writes the audit).
    #[must_use]
    pub const fn changes(&self) -> bool {
        matches!(self, Self::Decide { .. } | Self::LiftBlock { .. })
    }
}

/// A note within `max` characters: not empty once trimmed, and without
/// control characters but line breaks.
#[must_use]
pub fn valid_note(note: &str, max: usize) -> bool {
    !note.trim().is_empty()
        && note.chars().count() <= max
        && note.chars().all(|c| c == '\n' || !c.is_control())
}

/// A key as it appears in a picture's URL: 32 lowercase hex digits.
#[must_use]
pub fn key_hex(key: &PictureKey) -> String {
    hex(key)
}

/// Bytes as lowercase hex digits, as a digest appears in an entity tag.
#[must_use]
pub fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|b| {
            [
                char::from(DIGITS[usize::from(b >> 4)]),
                char::from(DIGITS[usize::from(b & 15)]),
            ]
        })
        .collect()
}

/// The key a URL's 32 lowercase hex digits name.
#[must_use]
pub fn parse_key(text: &str) -> Option<PictureKey> {
    if text.len() != 32 || !text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
        return None;
    }
    let mut key = [0; 16];
    for (i, byte) in key.iter_mut().enumerate() {
        *byte = u8::from_str_radix(text.get(i * 2..i * 2 + 2)?, 16).ok()?;
    }
    Some(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reasons_and_states_round_trip_by_name() {
        for reason in PictureReason::ALL {
            assert_eq!(PictureReason::parse(reason.as_str()), Some(reason));
            let json = crate::encode(&reason).unwrap();
            assert_eq!(json, format!("\"{}\"", reason.as_str()).into_bytes());
        }
        assert_eq!(PictureReason::parse("spam"), None);
        assert_eq!(
            PictureReportState::parse("gone"),
            Some(PictureReportState::Gone)
        );
        assert_eq!(PictureReportState::parse("open "), None);
    }

    #[test]
    fn notes_are_bounded_text() {
        assert!(valid_note(
            "Shows my address.\nPlease remove.",
            MAX_REPORT_NOTE
        ));
        assert!(!valid_note("   ", MAX_REPORT_NOTE));
        assert!(!valid_note("a\u{0}b", MAX_REPORT_NOTE));
        assert!(!valid_note("a\u{1b}[31m", MAX_REPORT_NOTE));
        assert!(valid_note(&"ä".repeat(1_000), MAX_REPORT_NOTE));
        assert!(!valid_note(&"ä".repeat(1_001), MAX_REPORT_NOTE));
        assert!(!valid_note(&"x".repeat(501), MAX_NOTICE_NOTE));
    }

    #[test]
    fn keys_are_lowercase_hex_only() {
        let key = [0xab; 16];
        assert_eq!(parse_key(&key_hex(&key)), Some(key));
        assert_eq!(parse_key(&key_hex(&key).to_uppercase()), None);
        assert_eq!(parse_key("ab"), None);
        assert_eq!(parse_key(&"g".repeat(32)), None);
        assert_eq!(parse_key(&format!("{}.png", key_hex(&key))), None);
    }

    #[test]
    fn decisions_and_blocks_have_the_designed_shapes() {
        let remove = PictureDecision::Remove {
            reason: PictureReason::Hate,
            note: None,
            block: BlockChoice::Days { days: 7 },
        };
        let text = String::from_utf8(crate::encode(&remove).unwrap()).unwrap();
        assert_eq!(
            text,
            r#"{"decision":"remove","reason":"hate","note":null,"block":{"block":"days","days":7}}"#
        );
        assert_eq!(
            crate::decode::<PictureDecision>(text.as_bytes()).unwrap(),
            remove
        );
        // an unknown field in a variant with fields is refused (serde does
        // not refuse one beside a unit variant's tag, here as everywhere)
        assert!(
            crate::decode::<PictureDecision>(
                br#"{"decision":"remove","reason":"hate","note":null,"block":{"block":"none"},"x":1}"#
            )
            .is_err()
        );
        let block = UploadBlock::UntilLifted;
        assert_eq!(
            crate::encode(&block).unwrap(),
            br#"{"kind":"until_lifted"}"#
        );
    }
}
