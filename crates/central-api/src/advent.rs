//! Private, account-bound Advent doors and durable cosmetic inventory.
use serde::{Deserialize, Serialize};
mod chunk;
pub mod puzzle;
pub mod table;

/// Server-provisioned visual finish.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Finish {
    /// Ordinary surface treatment.
    Standard,
    /// More elaborate surface effect.
    Special,
    /// The highest finish.
    Finale,
}

/// The two optional cosmetics selected by an admitted player.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Equipment {
    /// Equipped owned sleeve.
    pub sleeve: Option<Id>,
    /// Equipped owned playmat.
    pub playmat: Option<Id>,
}

/// Database-issued opaque item identity.
pub type Id = [u8; 16];
/// SHA-256 of the exact immutable asset.
pub type Digest = [u8; 32];
/// Fixed number of scheduled rewards.
pub const DOORS: u8 = 24;
/// Upper bound for one independently authorized transfer page.
pub const PAGE_BYTES: u32 = 192 * 1024;
/// Largest privately provisioned image.
pub const MAX_ART_BYTES: u32 = 16 * 1024 * 1024;

/// Operator-supplied localized caption or hint.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Text {
    /// German copy.
    pub de: String,
    /// English copy.
    pub en: String,
}
/// Cosmetic slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// The card back displayed by this player.
    Sleeve,
    /// This player's play surface.
    Playmat,
}
impl Kind {
    /// Stable storage label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sleeve => "sleeve",
            Self::Playmat => "playmat",
        }
    }
}
/// Image encodings admitted by private provisioning.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum Mime {
    /// Static `WebP`.
    #[serde(rename = "image/webp")]
    Webp,
}
/// Description of verified immutable content. Only an authorized viewer receives it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AssetMeta {
    /// Released surface finish.
    pub tier: Finish,
    /// Digest verified after assembling all chunks.
    pub digest: Digest,
    /// Exact total byte length.
    pub size: u32,
    /// Encoded image format.
    pub mime: Mime,
    /// Pixel width.
    pub width: u16,
    /// Pixel height.
    pub height: u16,
}
/// An earned item in this account's inventory.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Reward {
    /// Opaque selector. Possession is always checked independently.
    pub id: Id,
    /// Cosmetic slot.
    pub kind: Kind,
    /// Revealed item title.
    pub title: Text,
    /// Image metadata and transfer commitment.
    pub asset: AssetMeta,
    /// Originating campaign.
    pub year: u16,
    /// Originating numbered door.
    pub door: u8,
    /// First grant on Central's clock, Unix seconds.
    pub earned_at: i64,
}
/// The only calendar facts a player can see before opening a door.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DoorState {
    /// Future release.
    Locked,
    /// Available for unlimited puzzle attempts.
    Open,
    /// Solved and ready for personal reward collection.
    Solved,
    /// Reward already belongs to the account.
    Claimed,
    /// The campaign ended before a successful solution.
    Expired,
}
/// Individual calendar entry without any future item or puzzle metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Door {
    /// One through twenty-four.
    pub door: u8,
    /// This account's state.
    pub state: DoorState,
    /// Central release time, Unix seconds.
    pub enables_at: i64,
}
/// Authorized content selector. Neither a path nor an origin can be supplied.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum AssetScope {
    /// Private event mosaic during the release window.
    Calendar {
        #[doc = "Campaign year."]
        year: u16,
    },
    /// An item owned by the authenticated account.
    Owned {
        #[doc = "Owned item identifier."]
        item: Id,
    },
    /// Central checks both occupied chairs and resolves precisely the currently equipped slot.
    Equipped {
        #[doc = "Active match."]
        match_id: Id,
        #[doc = "Visible chair, zero or one."]
        seat: u8,
        #[doc = "Visible slot."]
        kind: Kind,
    },
}
/// Individual private asset range.
#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AssetPage {
    /// Verified metadata.
    pub asset: AssetMeta,
    /// Byte position in the immutable file.
    pub offset: u32,
    /// At most one bounded chunk.
    #[serde(with = "chunk")]
    pub bytes: Vec<u8>,
}
impl std::fmt::Debug for AssetPage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssetPage")
            .field("offset", &self.offset)
            .field("length", &self.bytes.len())
            .finish_non_exhaustive()
    }
}
/// Session-bound Advent operations. The caller never supplies its account identity.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    /// Discover the active or closest upcoming campaign using Central time.
    CurrentCalendar {},
    /// Resolve only cosmetics visible to this ranked participant.
    TableLooks {
        #[doc = "Active ranked match."]
        match_id: Id,
    },
    /// Select a reusable table look through the verified ordinary gateway protocol.
    Table {
        #[doc = "Session-owned table request."]
        request: table::Request,
    },
    /// Freeze this participant's owned deck cosmetics for the current ranked game.
    Present {
        #[doc = "Current ranked duel."]
        match_id: Id,
        #[doc = "Deck selections."]
        look: Equipment,
    },
    /// Read this account's twenty-four door states.
    Calendar {
        #[doc = "Campaign year."]
        year: u16,
    },
    /// Play the public position of a released puzzle.
    Puzzle {
        #[doc = "Campaign year."]
        year: u16,
        #[doc = "Door number."]
        door: u8,
    },
    /// Reveal one of two free hints.
    Hint {
        #[doc = "Campaign year."]
        year: u16,
        #[doc = "Door number."]
        door: u8,
        #[doc = "One or two."]
        number: u8,
    },
    /// Let Central evaluate one legal action sequence.
    Submit {
        #[doc = "Campaign year."]
        year: u16,
        #[doc = "Door number."]
        door: u8,
        #[doc = "Player's moves."]
        line: puzzle::Line,
    },
    /// Collect a solved door's fixed reward. Retries return the original grant.
    Claim {
        #[doc = "Campaign year."]
        year: u16,
        #[doc = "Door number."]
        door: u8,
    },
    /// Page this account's permanent inventory by immutable item id.
    Inventory {
        #[doc = "Optional exclusive previous item."]
        after: Option<Id>,
    },
    /// Fetch a private bounded chunk, or validate an already cached file.
    Asset {
        #[doc = "Entitlement or confirmed live participant."]
        scope: AssetScope,
        #[doc = "Aligned byte offset."]
        offset: u32,
        #[doc = "Previously verified cache digest on the first page."]
        cached: Option<Digest>,
    },
}
/// Replies are specific to the authenticated request and have no pre-release reward catalogue.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Response {
    /// No active or upcoming campaign has been provisioned.
    NoCalendar {},
    /// Private projection of the two displayed looks in seat order.
    TableLooks {
        #[doc = "Per-seat visible slots."]
        seats: [SeatLooks; 2],
    },
    /// Authenticated ordinary table appearance.
    Table {
        #[doc = "Private table response."]
        response: table::Response,
    },
    /// The player's match selection was recorded.
    Presented,
    /// Calendar and authoritative time.
    Calendar {
        #[doc = "Campaign year."]
        year: u16,
        #[doc = "Private mosaic metadata only in the active window."]
        hero: Option<AssetMeta>,
        #[doc = "First release, Unix seconds."]
        starts_at: i64,
        #[doc = "Exclusive close, Unix seconds."]
        closes_at: i64,
        #[doc = "Central time, Unix seconds."]
        now: i64,
        #[doc = "Ordered twenty-four entries."]
        doors: Vec<Door>,
    },
    /// Released puzzle with genuine board mechanics.
    Puzzle {
        #[doc = "Campaign year."]
        year: u16,
        #[doc = "Door number."]
        door: u8,
        #[doc = "Public starting position."]
        puzzle: puzzle::Puzzle,
    },
    /// One explicitly requested hint.
    Hint {
        #[doc = "Hint number."]
        number: u8,
        #[doc = "Revealed text."]
        text: Text,
    },
    /// Outcome found by Central's deterministic evaluation.
    Submission {
        #[doc = "This line won."]
        won: bool,
    },
    /// The original permanent claim.
    Reward {
        #[doc = "Revealed reward."]
        reward: Reward,
    },
    /// Current inventory page.
    Inventory {
        #[doc = "Owned items."]
        items: Vec<Reward>,
        #[doc = "Exclusive cursor when more remain."]
        next: Option<Id>,
    },
    /// A bounded, authorized portion of private content.
    Asset {
        #[doc = "Private transfer."]
        page: AssetPage,
    },
    /// The claimant already holds the exact bytes.
    Cached {
        #[doc = "Metadata of the still-authorized image."]
        asset: AssetMeta,
    },
}

/// Only the two assets displayed on an admitted chair.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SeatLooks {
    /// Visible sleeve metadata.
    pub sleeve: Option<AssetMeta>,
    /// Visible playmat metadata.
    pub playmat: Option<AssetMeta>,
}

impl AssetMeta {
    /// Check the size limits before reserving any transfer or image buffer.
    ///
    /// # Errors
    /// Rejects an empty, oversized or out-of-range declaration.
    pub fn validate(&self) -> Result<(), crate::Error> {
        if (1..=MAX_ART_BYTES).contains(&self.size)
            && (1..=4096).contains(&self.width)
            && (1..=4096).contains(&self.height)
        {
            Ok(())
        } else {
            Err(crate::Error::Rejected)
        }
    }
}

impl Equipment {
    /// Whether both deck cosmetic slots are unset.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.sleeve.is_none() && self.playmat.is_none()
    }
}

impl AssetMeta {
    /// Verify the complete immutable file before caching or decoding it.
    ///
    /// # Errors
    /// Rejects an invalid declaration, incomplete content or a different digest.
    pub fn verify(&self, bytes: &[u8]) -> Result<(), crate::Error> {
        use sha2::{Digest as _, Sha256};
        self.validate()?;
        if bytes.len() != self.size as usize
            || <[u8; 32]>::from(Sha256::digest(bytes)) != self.digest
        {
            return Err(crate::Error::Rejected);
        }
        Ok(())
    }
}
