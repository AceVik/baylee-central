//! Account-owned contacts shared by the website and native client.
//! Only public profiles cross this boundary; login names and account IDs do not.
use crate::Error;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use zeroize::Zeroizing;

/// Maximum accepted contacts per account.
pub const MAX_CONTACTS: usize = 300;
/// Maximum outstanding requests sent by one account.
pub const MAX_OUTGOING: usize = 50;
/// Maximum outstanding requests received by one account.
pub const MAX_INCOMING: usize = 300;
/// Maximum explicit blocks owned by an account.
pub const MAX_BLOCKS: usize = 1000;
/// Maximum active (unredeemed, unrevoked, unexpired) friend invites per account.
pub const MAX_ACTIVE_FRIEND_INVITES: usize = 10;
/// Default validity period (in days) when creating a friend invite.
pub const DEFAULT_FRIEND_INVITE_DAYS: u8 = 7;
/// Maximum validity period (in days) allowed for a friend invite.
pub const MAX_FRIEND_INVITE_DAYS: u8 = 14;
/// Maximum creator-only label length in Unicode scalar values.
pub const MAX_INVITE_LABEL: usize = 64;
/// Maximum optional invite greeting/note length in Unicode scalar values.
pub const MAX_INVITE_NOTE: usize = 280;
/// Cumulative friend-invite milestones (`Bronze` .. `Obsidian`, Ingress-inspired tiers).
pub const RECRUITER_THRESHOLDS: [u32; 5] = [1, 3, 10, 25, 50];
/// Entropy of each generated friend invite code in bytes (128 bits).
pub const FRIEND_INVITE_CODE_BYTES: usize = 16;
/// Length of a canonical unpadded base64url friend invite code (`16` bytes -> `22` chars).
pub const FRIEND_INVITE_CODE_LEN: usize = 22;

const FRIEND_INVITE_DOMAIN: &[u8] = b"baylee-central/friend-invite/v1\0";
const B64_URL: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
/// Maximum search query length in Unicode scalar values.
pub const MAX_SEARCH_QUERY: usize = 32;
/// Maximum public profiles returned by one search query.
pub const MAX_SEARCH_RESULTS: usize = 8;
/// Non-foil finish bit in [`CardPrinting::finishes`].
pub const FINISH_NONFOIL: u8 = 1 << 0;
/// Traditional foil finish bit in [`CardPrinting::finishes`].
pub const FINISH_FOIL: u8 = 1 << 1;
/// Etched foil finish bit in [`CardPrinting::finishes`].
pub const FINISH_ETCHED: u8 = 1 << 2;
/// Maximum card query length in Unicode scalar values.
pub const MAX_CARD_QUERY: usize = 80;
/// Maximum set/collector/finish filter length in Unicode scalar values.
pub const MAX_CARD_FILTER: usize = 40;
/// Maximum card suggestions returned per query.
pub const MAX_CARD_RESULTS: usize = 8;
/// Maximum card printings returned for one card.
pub const MAX_CARD_PRINTINGS: usize = 40;

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
    /// Resolve one public tag.
    Lookup {
        /// Exact public tag.
        tag: u32,
    },
    /// Search public profiles by display name or tag, excluding blocked pairs.
    Search {
        /// Query text, at most [`MAX_SEARCH_QUERY`] characters.
        query: String,
    },
    /// Change a relationship with the account identified by its public tag.
    Contact {
        /// The other account's public tag; self-actions are refused.
        tag: u32,
        /// The requested state transition.
        change: ContactAction,
    },
    /// The current picture keys of up to
    /// [`crate::picture::MAX_PICTURE_TAGS`] public tags
    /// (`docs/profile-pictures.md`). Every tag outside the caller's
    /// contacts and groups spends one unit of the lookup budget. Unknown
    /// tags, accounts without a picture and pairs with a block between them
    /// are absent. Answered with [`SocialResponse::Pictures`].
    Pictures {
        /// Exact public tags.
        tags: Vec<u32>,
    },
    /// Report the account's current picture to the administrators, with a
    /// reason from the list and an optional note. `Done`; one's own
    /// picture, one the caller cannot see and a key that is not the
    /// account's current one are `Rejected` alike.
    ReportPicture {
        /// The account's public tag.
        tag: u32,
        /// The key of the picture the caller saw.
        key: crate::picture::PictureKey,
        /// The reason.
        reason: crate::picture::PictureReason,
        /// At most [`crate::picture::MAX_REPORT_NOTE`] characters.
        note: Option<String>,
    },
    /// Current-season highest player rank per public tag, used to frame peer
    /// avatars in contacts and chat. Respects blocks identically to `Pictures`.
    Ranks {
        /// Exact public tags (bounded by [`crate::picture::MAX_PICTURE_TAGS`]).
        tags: Vec<u32>,
    },
    /// Authenticated card autocomplete limited strictly to `lang` and English (`en`).
    CardSuggest {
        /// Partial or exact card name query.
        query: String,
        /// Client UI language tag (`de`, `en`, `pt-br`, `zh-hans`, etc.).
        lang: String,
        /// Optional Scryfall set code or set name filter.
        #[serde(default)]
        set_filter: Option<String>,
        /// Optional collector number filter.
        #[serde(default)]
        collector_filter: Option<String>,
        /// Optional finish filter (`foil`, `etched`, `nonfoil`).
        #[serde(default)]
        finish_filter: Option<String>,
    },
    /// Authenticated list of printings for one card name in `lang` and English.
    CardPrintings {
        /// Card name (localized in `lang` or English).
        name: String,
        /// Client UI language tag.
        lang: String,
        /// Optional set code or set name filter.
        #[serde(default)]
        set_filter: Option<String>,
        /// Optional collector number filter.
        #[serde(default)]
        collector_filter: Option<String>,
        /// Optional finish filter (`foil`, `etched`, `nonfoil`).
        #[serde(default)]
        finish_filter: Option<String>,
    },
    /// List all friend invites created by the signed-in account.
    /// Answered with [`SocialResponse::FriendInvites`].
    FriendInvites,
    /// Create a single-use 128-bit friend invite and optionally mail it to `email`.
    /// Answered with [`SocialResponse::FriendInviteCreated`].
    CreateFriendInvite {
        /// Optional creator-only private label (`1..=64` chars).
        #[serde(default)]
        label: Option<String>,
        /// Optional recipient-facing note (`1..=280` chars).
        #[serde(default)]
        note: Option<String>,
        /// Optional recipient email address to address/mail this invite to.
        #[serde(default)]
        email: Option<String>,
        /// Preferred mail language tag (`en`, `de`, ...). Defaults to `en`.
        #[serde(default)]
        lang: Option<String>,
        /// Optional expiry in days (`1..=14`, defaults to `7`).
        #[serde(default)]
        expires_days: Option<u8>,
    },
    /// Revoke an active friend invite owned by the signed-in account.
    /// Answered with [`SocialResponse::Done`].
    RevokeFriendInvite {
        /// Invite `UUIDv7` bytes (`FriendInviteSummary::id`).
        invite: [u8; 16],
    },
    /// Redeem an active friend invite for the signed-in account, creating a
    /// mutual accepted friendship with the inviter.
    /// Answered with [`SocialResponse::FriendInviteRedeemed`].
    RedeemFriendInvite {
        /// Single-use invite code from the invite link.
        code: String,
        /// Opt-in flag to initiate pending email confirmation using the invite's
        /// `invited_email` when this account has no confirmed email yet.
        #[serde(default)]
        link_email: bool,
        /// Preferred confirmation mail language tag (`en`, `de`, ...).
        #[serde(default)]
        lang: Option<String>,
    },
}

/// One Scryfall-identified card printing summary for authenticated chat previews.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardPrinting {
    /// Printing UUID (`cards.scryfall.io` key), always omitted for Universes Beyond
    /// or printings without an image so browsers never fetch prohibited art.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scryfall_id: Option<String>,
    /// Lowercase Scryfall set code (e.g. `2x2`, `lea`).
    pub set_code: String,
    /// English set name (e.g. `Double Masters 2022`).
    pub set_name: String,
    /// Collector number string (e.g. `117`, `162a`).
    pub collector_number: String,
    /// Scryfall printing language (`en`, `de`, `fr`, ..., `zhs`, `zht`).
    pub lang: String,
    /// Bitmask of [`FINISH_NONFOIL`], [`FINISH_FOIL`], [`FINISH_ETCHED`].
    pub finishes: u8,
    /// Whether this printing belongs to a Universes Beyond set or carries a triangle stamp.
    pub ub: bool,
}

/// One localized card autocomplete suggestion returned by authenticated catalog search.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardSuggestion {
    /// Name in the matched language (client UI language or English).
    pub name: String,
    /// Canonical English card name.
    pub en_name: String,
    /// Scryfall language code of `name`.
    pub matched_lang: String,
    /// Best matching printing for inline preview, if catalogued.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<CardPrinting>,
}

/// One public tag's highest current-season player rank.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaggedRank {
    /// The account's public tag.
    pub tag: u32,
    /// Its highest current-season player rank.
    pub rank: crate::ranked::Rank,
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
    /// Ranked public profiles matching a search query (at most [`MAX_SEARCH_RESULTS`]).
    People {
        /// Public profiles, never private usernames or blocked accounts.
        people: Vec<Person>,
    },
    /// Current picture keys, in the order of the tags asked for.
    Pictures {
        /// At most one per tag.
        pictures: Vec<crate::picture::TaggedPicture>,
    },
    /// Current-season highest player ranks for requested tags that hold one.
    Ranks {
        /// At most one per tag.
        ranks: Vec<TaggedRank>,
    },
    /// Up to [`MAX_CARD_RESULTS`] localized card suggestions.
    CardSuggestions {
        /// Ranked matches in the client language and English.
        items: Vec<CardSuggestion>,
    },
    /// Matching card summary and up to [`MAX_CARD_PRINTINGS`] available printings.
    CardPrintings {
        /// Best matching card summary, if found.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        card: Option<CardSuggestion>,
        /// Available printings matching the request filters.
        printings: Vec<CardPrinting>,
    },
    /// List of friend invites owned by the signed-in account.
    FriendInvites {
        /// Invites newest-first (`created_at DESC`).
        invites: Vec<FriendInviteSummary>,
        /// Active invite cap per account ([`MAX_ACTIVE_FRIEND_INVITES`]).
        active_limit: u32,
        /// Whether Central currently has outbound mail delivery configured.
        mail_available: bool,
        /// Current Ingress-inspired tiered recruiter medal progress for the caller.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        recruiter_badge: Option<SocialBadge>,
    },
    /// Newly created single-use friend invite plus the raw code (returned once).
    FriendInviteCreated {
        /// Public/creator summary for the new invite.
        invite: FriendInviteSummary,
        /// Raw single-use 128-bit invite code (shown once; Central keeps only its digest).
        code: String,
        /// `true` when an outbound invite email was handed to the mailer.
        mail_sent: bool,
    },
    /// Successful invite redemption for an existing signed-in account.
    FriendInviteRedeemed {
        /// Public profile info of the inviter (now a mutual contact).
        inviter: Person,
        /// `true` when `link_email` initiated a pending confirmation email for
        /// this account because it had no confirmed email yet.
        email_pending: bool,
    },
    /// The requested change completed, or was already complete.
    Done,
}

/// Permanent social achievement tracks shared by website and native client.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SocialBadgeKind {
    /// Friends successfully recruited through personal friend invites.
    Recruiter,
}

/// Account progress on a permanent social achievement track.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SocialBadge {
    /// Track.
    pub kind: SocialBadgeKind,
    /// Verified non-regressing cumulative count for this track.
    pub count: u32,
    /// Highest earned tier, or `None` before the first milestone.
    pub tier: Option<crate::challenges::Tier>,
    /// Next cumulative milestone, or `None` once Obsidian is reached.
    pub next: Option<u32>,
}

impl SocialBadgeKind {
    /// All social tracks in display order.
    pub const ALL: [Self; 1] = [Self::Recruiter];

    /// Cumulative milestones in tier order (`Bronze` .. `Obsidian`).
    #[must_use]
    pub const fn thresholds(self) -> [u32; 5] {
        match self {
            Self::Recruiter => RECRUITER_THRESHOLDS,
        }
    }

    /// Evaluates a verified count into a tiered [`SocialBadge`].
    #[must_use]
    pub fn progress(self, count: u32) -> SocialBadge {
        let tiers = [
            crate::challenges::Tier::Bronze,
            crate::challenges::Tier::Silver,
            crate::challenges::Tier::Gold,
            crate::challenges::Tier::Platinum,
            crate::challenges::Tier::Obsidian,
        ];
        let thresholds = self.thresholds();
        let reached = thresholds
            .iter()
            .take_while(|&&target| count >= target)
            .count();
        SocialBadge {
            kind: self,
            count,
            tier: reached.checked_sub(1).map(|idx| tiers[idx]),
            next: thresholds.get(reached).copied(),
        }
    }
}

/// Lifecycle state of a single-use friend invite.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FriendInviteStatus {
    /// Unused, not revoked and before `expires_at`.
    Active,
    /// Consumed by a registration or existing-account redemption.
    Redeemed,
    /// Revoked by the creator before use.
    Revoked,
    /// Passed `expires_at` without redemption or revocation.
    Expired,
}

/// Creator-facing friend invite record (never includes the reusable secret code).
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FriendInviteSummary {
    /// Invite `UUIDv7` bytes (`[u8; 16]`).
    pub id: [u8; 16],
    /// Creator-only private label (`1..=64` chars), omitted from public previews.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Optional recipient-facing note (`1..=280` chars).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Optional addressed recipient email address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invited_email: Option<String>,
    /// Current invite lifecycle status.
    pub status: FriendInviteStatus,
    /// Creation timestamp (Unix seconds).
    pub created_at: i64,
    /// Expiration timestamp (Unix seconds).
    pub expires_at: i64,
    /// Redemption timestamp (Unix seconds), present when [`FriendInviteStatus::Redeemed`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redeemed_at: Option<i64>,
    /// Redeemer's public player handle, present when [`FriendInviteStatus::Redeemed`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redeemed_by: Option<Person>,
}

impl fmt::Debug for FriendInviteSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FriendInviteSummary")
            .field("id", &self.id)
            .field("label", &self.label)
            .field("note", &self.note)
            .field(
                "invited_email",
                &self.invited_email.as_ref().map(|_| "<redacted>"),
            )
            .field("status", &self.status)
            .field("created_at", &self.created_at)
            .field("expires_at", &self.expires_at)
            .field("redeemed_at", &self.redeemed_at)
            .field("redeemed_by", &self.redeemed_by)
            .finish()
    }
}

/// Unauthenticated capability-scoped preview for a friend invite code
/// ([`crate::wire::Response::FriendInviteInfo`]).
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FriendInvitePreview {
    /// Whether open public registration is currently enabled.
    pub registration_open: bool,
    /// `true` when public registration is closed and new accounts require a valid invite.
    pub invite_only: bool,
    /// Inviter public profile (`tag` and `display_name`).
    pub inviter: Person,
    /// Optional recipient-facing note; exposed only while [`FriendInviteStatus::Active`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Optional addressed recipient email; always omitted (`None`) from
    /// unauthenticated `FriendInviteInfo` responses to avoid leaking PII.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invited_email: Option<String>,
    /// Current invite lifecycle status.
    pub status: FriendInviteStatus,
    /// Creation timestamp (Unix seconds).
    pub created_at: i64,
    /// Expiration timestamp (Unix seconds).
    pub expires_at: i64,
    /// Redeemer's public handle when authenticated; always omitted (`None`) from
    /// unauthenticated `FriendInviteInfo` responses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redeemed_by: Option<Person>,
}

impl fmt::Debug for FriendInvitePreview {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FriendInvitePreview")
            .field("registration_open", &self.registration_open)
            .field("invite_only", &self.invite_only)
            .field("inviter", &self.inviter)
            .field("note", &self.note)
            .field(
                "invited_email",
                &self.invited_email.as_ref().map(|_| "<redacted>"),
            )
            .field("status", &self.status)
            .field("created_at", &self.created_at)
            .field("expires_at", &self.expires_at)
            .field("redeemed_by", &self.redeemed_by)
            .finish()
    }
}

const fn b64url_val(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'-' => Some(62),
        b'_' => Some(63),
        _ => None,
    }
}

fn encode_invite_code(raw: &[u8; FRIEND_INVITE_CODE_BYTES]) -> Zeroizing<String> {
    let mut out = Zeroizing::new(String::with_capacity(FRIEND_INVITE_CODE_LEN));
    let (chunks, rem) = raw.as_chunks::<3>();
    for chunk in chunks {
        let b0 = usize::from(chunk[0]);
        let b1 = usize::from(chunk[1]);
        let b2 = usize::from(chunk[2]);
        out.push(char::from(B64_URL[b0 >> 2]));
        out.push(char::from(B64_URL[((b0 & 0x03) << 4) | (b1 >> 4)]));
        out.push(char::from(B64_URL[((b1 & 0x0f) << 2) | (b2 >> 6)]));
        out.push(char::from(B64_URL[b2 & 0x3f]));
    }
    if rem.len() == 1 {
        let b0 = usize::from(rem[0]);
        out.push(char::from(B64_URL[b0 >> 2]));
        out.push(char::from(B64_URL[(b0 & 0x03) << 4]));
    }
    out
}

fn decode_invite_code(code: &str) -> Option<Zeroizing<[u8; FRIEND_INVITE_CODE_BYTES]>> {
    let bytes = code.as_bytes();
    if bytes.len() != FRIEND_INVITE_CODE_LEN {
        return None;
    }
    let mut vals = Zeroizing::new([0u8; FRIEND_INVITE_CODE_LEN]);
    for (idx, &b) in bytes.iter().enumerate() {
        vals[idx] = b64url_val(b)?;
    }
    if (vals[21] & 0x0f) != 0 {
        return None;
    }
    let mut raw = Zeroizing::new([0u8; FRIEND_INVITE_CODE_BYTES]);
    for (i, group) in vals[..20].as_chunks::<4>().0.iter().enumerate() {
        let base = i * 3;
        raw[base] = (group[0] << 2) | (group[1] >> 4);
        raw[base + 1] = (group[1] << 4) | (group[2] >> 2);
        raw[base + 2] = (group[2] << 6) | group[3];
    }
    raw[15] = (vals[20] << 2) | (vals[21] >> 4);
    Some(raw)
}

const fn is_bidi_format(ch: char) -> bool {
    matches!(
        ch,
        '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}'
    )
}

/// Generates a fresh 128-bit CSPRNG invite code and its domain-separated SHA-256 digest.
///
/// # Errors
/// Returns [`Error::Unavailable`] if the operating system random source fails.
pub fn generate_friend_invite() -> Result<(Zeroizing<String>, [u8; 32]), Error> {
    let mut raw = Zeroizing::new([0u8; FRIEND_INVITE_CODE_BYTES]);
    getrandom::fill(&mut *raw).map_err(|_| Error::Unavailable)?;
    let digest: [u8; 32] = Sha256::new()
        .chain_update(FRIEND_INVITE_DOMAIN)
        .chain_update(*raw)
        .finalize()
        .into();
    Ok((encode_invite_code(&raw), digest))
}

/// Returns `true` when `code` is a syntactically canonical 128-bit friend invite code.
#[must_use]
pub fn valid_friend_invite_code(code: &str) -> bool {
    decode_invite_code(code).is_some()
}

/// Validates a canonical friend invite code and returns its domain-separated SHA-256 digest.
///
/// # Errors
/// Returns [`Error::Rejected`] when `code` is malformed or non-canonical.
pub fn invite_code_digest(code: &str) -> Result<[u8; 32], Error> {
    let raw = decode_invite_code(code).ok_or(Error::Rejected)?;
    Ok(Sha256::new()
        .chain_update(FRIEND_INVITE_DOMAIN)
        .chain_update(*raw)
        .finalize()
        .into())
}

/// Trims and validates optional invite text (`label` or `note`), rejecting control
/// and directional formatting characters or lengths beyond `max_chars` Unicode scalars.
///
/// # Errors
/// Returns [`Error::Rejected`] when trimmed text exceeds `max_chars` or contains control/bidi chars.
pub fn clean_invite_text(raw: Option<&str>, max_chars: usize) -> Result<Option<String>, Error> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > max_chars
        || trimmed
            .chars()
            .any(|ch| ch.is_control() || is_bidi_format(ch))
    {
        return Err(Error::Rejected);
    }
    Ok(Some(trimmed.to_owned()))
}

/// Validates invite expiry days (`1..=14`, defaulting to [`DEFAULT_FRIEND_INVITE_DAYS`]).
///
/// # Errors
/// Returns [`Error::Rejected`] for `0` or values above [`MAX_FRIEND_INVITE_DAYS`].
pub fn invite_expiry_days(days: Option<u8>) -> Result<u8, Error> {
    match days {
        None => Ok(DEFAULT_FRIEND_INVITE_DAYS),
        Some(value @ 1..=MAX_FRIEND_INVITE_DAYS) => Ok(value),
        Some(_) => Err(Error::Rejected),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn friend_invite_codes_round_trip_and_digest_is_domain_separated() {
        let (code, digest) = generate_friend_invite().unwrap();
        assert_eq!(code.len(), FRIEND_INVITE_CODE_LEN);
        assert!(valid_friend_invite_code(&code));
        assert_eq!(invite_code_digest(&code).unwrap(), digest);

        // Second generation yields an independent secret and digest.
        let (other, other_digest) = generate_friend_invite().unwrap();
        assert_ne!(code.as_str(), other.as_str());
        assert_ne!(digest, other_digest);

        // Non-canonical trailing bits in the 22nd character are rejected.
        let mut altered = code.as_str().as_bytes().to_vec();
        let last_val = b64url_val(altered[21]).unwrap();
        altered[21] = B64_URL[usize::from(last_val ^ 1)];
        let bad = std::str::from_utf8(&altered).unwrap();
        assert!(!valid_friend_invite_code(bad));
        assert_eq!(invite_code_digest(bad), Err(Error::Rejected));

        for invalid in [
            "",
            "abc",
            "AAAAAAAAAAAAAAAAAAAAA=",
            "AAAAAAAAAAAAAAAAAAAAAAB",
        ] {
            assert!(!valid_friend_invite_code(invalid), "{invalid}");
            assert_eq!(invite_code_digest(invalid), Err(Error::Rejected));
        }
    }

    #[test]
    fn clean_invite_text_and_expiry_enforce_bounds_and_safe_chars() {
        assert_eq!(clean_invite_text(None, MAX_INVITE_LABEL).unwrap(), None);
        assert_eq!(
            clean_invite_text(Some("   \t "), MAX_INVITE_LABEL).unwrap(),
            None
        );
        assert_eq!(
            clean_invite_text(Some("  LAN Group  "), MAX_INVITE_LABEL).unwrap(),
            Some("LAN Group".to_owned())
        );
        assert_eq!(
            clean_invite_text(Some(&"a".repeat(MAX_INVITE_LABEL + 1)), MAX_INVITE_LABEL),
            Err(Error::Rejected)
        );
        assert_eq!(
            clean_invite_text(Some("hi\nthere"), MAX_INVITE_NOTE),
            Err(Error::Rejected)
        );
        assert_eq!(
            clean_invite_text(Some("bad\u{202E}bidi"), MAX_INVITE_NOTE),
            Err(Error::Rejected)
        );
        assert_eq!(invite_expiry_days(None), Ok(DEFAULT_FRIEND_INVITE_DAYS));
        assert_eq!(invite_expiry_days(Some(1)), Ok(1));
        assert_eq!(invite_expiry_days(Some(MAX_FRIEND_INVITE_DAYS)), Ok(14));
        assert_eq!(invite_expiry_days(Some(0)), Err(Error::Rejected));
        assert_eq!(invite_expiry_days(Some(15)), Err(Error::Rejected));
    }

    #[test]
    fn friend_invite_types_round_trip_and_redact_email_in_debug() {
        let summary = FriendInviteSummary {
            id: [7; 16],
            label: Some("Squad".into()),
            note: Some("Join me!".into()),
            invited_email: Some("friend@example.org".into()),
            status: FriendInviteStatus::Active,
            created_at: 1_800_000_000,
            expires_at: 1_800_604_800,
            redeemed_at: None,
            redeemed_by: None,
        };
        assert!(!format!("{summary:?}").contains("friend@example.org"));
        let preview = FriendInvitePreview {
            registration_open: false,
            invite_only: true,
            inviter: Person {
                tag: 0x1a2b,
                display_name: "Alice".into(),
            },
            note: Some("Join me!".into()),
            invited_email: Some("friend@example.org".into()),
            status: FriendInviteStatus::Active,
            created_at: 1_800_000_000,
            expires_at: 1_800_604_800,
            redeemed_by: None,
        };
        assert!(!format!("{preview:?}").contains("friend@example.org"));

        let req: SocialRequest = serde_json::from_str(
            r#"{"action":"create_friend_invite","label":"L","note":"Hi","email":"a@b.example","lang":"de","expires_days":3}"#,
        )
        .unwrap();
        assert!(matches!(
            req,
            SocialRequest::CreateFriendInvite {
                expires_days: Some(3),
                ..
            }
        ));
        let redeem: SocialRequest =
            serde_json::from_str(r#"{"action":"redeem_friend_invite","code":"abc"}"#).unwrap();
        assert!(matches!(
            redeem,
            SocialRequest::RedeemFriendInvite {
                link_email: false,
                lang: None,
                ..
            }
        ));
        let resp: SocialResponse = serde_json::from_slice(
            &serde_json::to_vec(&SocialResponse::FriendInvites {
                invites: vec![summary],
                active_limit: 10,
                mail_available: true,
                recruiter_badge: Some(SocialBadgeKind::Recruiter.progress(3)),
            })
            .unwrap(),
        )
        .unwrap();
        assert!(matches!(
            resp,
            SocialResponse::FriendInvites {
                active_limit: 10,
                mail_available: true,
                recruiter_badge: Some(SocialBadge {
                    kind: SocialBadgeKind::Recruiter,
                    count: 3,
                    tier: Some(crate::challenges::Tier::Silver),
                    next: Some(10),
                }),
                ..
            }
        ));
    }

    #[test]
    fn recruiter_badge_tiers_follow_cumulative_milestones() {
        use crate::challenges::Tier;
        assert_eq!(
            SocialBadgeKind::Recruiter.progress(0),
            SocialBadge {
                kind: SocialBadgeKind::Recruiter,
                count: 0,
                tier: None,
                next: Some(1),
            }
        );
        for (count, expected_tier, expected_next) in [
            (1, Some(Tier::Bronze), Some(3)),
            (2, Some(Tier::Bronze), Some(3)),
            (3, Some(Tier::Silver), Some(10)),
            (10, Some(Tier::Gold), Some(25)),
            (25, Some(Tier::Platinum), Some(50)),
            (50, Some(Tier::Obsidian), None),
            (250, Some(Tier::Obsidian), None),
        ] {
            let badge = SocialBadgeKind::Recruiter.progress(count);
            assert_eq!(badge.tier, expected_tier);
            assert_eq!(badge.next, expected_next);
        }
    }
}
