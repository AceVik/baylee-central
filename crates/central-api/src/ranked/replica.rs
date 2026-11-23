//! Version-one ranked replication. Seat requests carry account authentication;
//! host journal pages are a different response type and never reach seats.
use super::{Format, Id, Outcome, PoolEntry, QueueState};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

pub mod clock;
pub mod relay;
mod validation;

// Service-only wire shapes carry deterministic seeds, private deck
// commitments and replay evidence, including inside generic responses.
macro_rules! private_debug {
    ($($kind:ty),+ $(,)?) => {$(
        impl std::fmt::Debug for $kind {
            fn fmt(&self, f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {
                f.write_str(concat!(stringify!($kind),"([REDACTED])"))
            }
        }
    )+};
}
private_debug!(Evidence, Work, Entry, HostRequest, HostResponse);

#[cfg(test)]
mod tests;
/// Private opening, encoded within Central's global wire limit.
pub type SetupBytes = crate::private_bytes::PrivateBytes<{ 192 * 1024 }>;
/// One private ordered input including any format framing.
pub type CommandBytes = crate::private_bytes::PrivateBytes<{ 32 * 1024 }>;

/// SHA-256 commitment.
pub type Digest = [u8; 32];
/// Maximum entries in a journal page.
pub const PAGE: usize = 8;
/// Maximum ordered inputs in a protected match.
pub const MAX_POSITION: u64 = 4096;
/// Maximum combined private command bytes for a single protected game.
pub const MAX_JOURNAL_BYTES: usize = 16 * 1024 * 1024;

/// A match and the only currently authorized leadership generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fence {
    /// Stable match ID.
    pub match_id: Id,
    /// Monotonic term, starting at one for protected games.
    pub generation: u64,
}

/// A position and its immutable history commitment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Head {
    /// Zero is the frozen private opening.
    pub position: u64,
    /// Hash of the ordered prefix at this position.
    pub chain: Digest,
}

/// The identity authorizing a committed input, derived by Central.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    /// Authenticated first player.
    Seat0,
    /// Authenticated second player.
    Seat1,
    /// Central's own durable clock transition.
    Authority,
}

impl Head {
    /// Bind the private setup, engine compatibility and both sorted pools.
    #[must_use]
    pub fn opening(
        match_id: Id,
        game: Id,
        compatibility: Digest,
        seed: u64,
        body: &SetupBytes,
        pools: &[Vec<PoolEntry>; 2],
    ) -> Self {
        let mut hash = Sha256::new();
        hash.update(b"baylee/ranked/opening/v1\0");
        hash.update(match_id);
        hash.update(game);
        hash.update(compatibility);
        hash.update(seed.to_be_bytes());
        hash.update((body.as_slice().len() as u64).to_be_bytes());
        hash.update(body.as_slice());
        for pool in pools {
            hash.update((pool.len() as u64).to_be_bytes());
            for entry in pool {
                hash.update(entry.card);
                hash.update(entry.copies.to_be_bytes());
            }
        }
        Self {
            position: 0,
            chain: hash.finalize().into(),
        }
    }

    /// Commit the actor, command ID and exact private bytes in order.
    ///
    /// # Errors
    /// Rejects a position beyond the maximum durable game length.
    pub fn next(
        self,
        match_id: Id,
        command_id: Id,
        source: Source,
        body: &CommandBytes,
    ) -> Result<Self, crate::Error> {
        if self.position >= MAX_POSITION {
            return Err(crate::Error::Busy);
        }
        let position = self.position + 1;
        let mut hash = Sha256::new();
        hash.update(b"baylee/ranked/input/v1\0");
        hash.update(match_id);
        hash.update(self.chain);
        hash.update(position.to_be_bytes());
        hash.update(command_id);
        hash.update([match source {
            Source::Seat0 => 0,
            Source::Seat1 => 1,
            Source::Authority => 2,
        }]);
        hash.update(body.as_slice());
        Ok(Self {
            position,
            chain: hash.finalize().into(),
        })
    }
}

/// Replication role granted to a specific verified host.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Current player-facing leader.
    Primary,
    /// Independent verifier.
    Standby,
}

/// Recovery state visible to an assigned participant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// Legacy/unprotected at match creation.
    Single,
    /// Waiting for independent opening verification.
    Preparing,
    /// Accepting independently authenticated decisions.
    Playing,
    /// Awaiting a healthy verifier or a new leader.
    Paused,
    /// Hosts disagree, play and settlement are suspended.
    Disputed,
    /// Final result committed.
    Finished,
    /// Recovery closed without a rated outcome.
    Void,
}

/// Private executor evidence. Never published to players or spectators.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    /// SHA-256 digest of deterministic private execution.
    pub state: Digest,
    /// Whether the engine accepted this attempted input.
    pub accepted: bool,
    /// Bit mask of players with outstanding decisions.
    pub awaited: u8,
    /// Engine decision identity of each outstanding question.
    pub decisions: [Option<u64>; 2],
    /// Independently reconstructed terminal outcome, if any.
    pub terminal: Option<Outcome>,
}

/// Active match routing and certified position for its own player.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeatStatus {
    /// Personal disclosure status across both assigned operators.
    pub consent: ConsentState,
    /// Current generation.
    pub fence: Fence,
    /// Current primary's verified gateway ID.
    pub host_id: Id,
    /// Current primary's canonical origin.
    pub origin: String,
    /// Stable engine game once the opening is registered.
    pub game: Option<Id>,
    /// Certified durable prefix. No private engine state.
    pub head: Head,
    /// Protection and recovery state.
    pub phase: Phase,
    /// Public decision and reconnect clocks, once the protected game starts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clock: Option<clock::ClockView>,
}

/// What this participant can do with the current operator assignments.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsentState {
    /// This participant must review at least one named operator.
    Needed,
    /// This participant approved all operators; another approval is outstanding.
    Waiting,
    /// Both participants approved every active operator.
    Ready,
}

/// A specific operator disclosure in this match, bound to the displayed terms.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostApproval {
    /// Operator verification epoch in force when this consent was displayed.
    pub epoch: u64,
    /// Central's stable identity for this verified operator.
    pub host_id: Id,
    /// Exact canonical destination.
    pub origin: String,
    /// Exact terms document shown, if published.
    pub terms: Option<crate::terms::TermsVersion>,
}

/// The participant's standing for a requested operator.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostConsentOffer {
    /// Named operator and terms binding for this match.
    pub host: HostApproval,
    /// Currently assigned function. Approval permits verification or hosting.
    pub role: Role,
    /// This participant has expressly approved disclosure for this match.
    pub approved: bool,
    /// The operator's terms still require this participant's acceptance.
    pub needs_gateway_terms: bool,
}

/// Assigned private work for one verified host. The peer and player
/// responses never receive the host's service credential.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Work {
    /// Immutable Central-selected RNG seed for the independent executors.
    pub seed: u64,
    /// Fences every mutating host operation.
    pub fence: Fence,
    /// Bound engine game.
    pub game: Option<Id>,
    /// Role of this authenticated caller.
    pub role: Role,
    /// Build, protocol and rules compatibility commitment.
    pub compatibility: Digest,
    /// Match format.
    pub format: Format,
    /// Aliases for this host and match in stable seat order.
    pub accounts: [Id; 2],
    /// Registered private decks' commitments.
    pub decks: [Digest; 2],
    /// Current activity.
    pub phase: Phase,
    /// Latest quorum-certified prefix.
    pub head: Head,
}

/// A single ordered host-only journal input.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// Position including this input.
    pub head: Head,
    /// Idempotency ID, unchanged across retries/generations.
    pub command_id: Id,
    /// Actor authenticated independently of the host.
    pub source: Source,
    /// Private encoded command.
    pub body: CommandBytes,
    /// Prior independent agreement, for replay of a certified prefix.
    pub evidence: Option<Evidence>,
}

/// Upgraded operations available only to an authenticated player.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum SeatRequest {
    /// Forward a narrowly typed seat operation through Central's network boundary.
    Relay {
        /// Exact current authority.
        fence: Fence,
        /// Fixed host operation without any caller-selected destination.
        call: relay::Call,
    },
    /// Reauthorize the current player's assigned host, game and seat for the relay.
    RelayRoute {
        /// Must match the current assignment and consent.
        fence: Fence,
    },
    /// Decline a newly assigned operator before any private disclosure to it.
    DeclineHost {
        #[doc = "Observed generation."]
        fence: Fence,
        #[doc = "Exact operator and document declined."]
        host: HostApproval,
    },
    /// Read assigned operator names and this participant's disclosure choices.
    Hosts {
        #[doc = "Assigned match."]
        match_id: Id,
    },
    /// Approve the exact list of named operators after a deliberate user choice.
    ApproveHosts {
        #[doc = "Observed leadership and assignments."]
        fence: Fence,
        #[doc = "Display-bound named disclosures."]
        hosts: Vec<HostApproval>,
    },
    /// Publicly chosen handles of this match's two participants.
    Participants {
        #[doc = "Assigned match."]
        match_id: Id,
    },
    /// This account reports whether its own game socket is attached.
    Presence {
        #[doc = "Current generation."]
        fence: Fence,
        #[doc = "Whether this player is attached."]
        connected: bool,
    },
    /// Select replica-compatible matching.
    Join {
        #[doc = "Stable queue operation ID."]
        key: Id,
        #[doc = "Ranked format."]
        format: Format,
        #[doc = "Frozen playable deck commitment."]
        deck: Digest,
    },
    /// Read current authority and certified position for this player's match.
    Status {
        #[doc = "Assigned match."]
        match_id: Id,
    },
    /// Record an input intent using this account's assigned seat.
    Submit {
        #[doc = "Observed current generation."]
        fence: Fence,
        #[doc = "Certified parent position."]
        expected: Head,
        #[doc = "Stable `UUIDv7` reused only for this intent."]
        command_id: Id,
        #[doc = "Private encoded seat command."]
        body: CommandBytes,
    },
}

/// Account response. Cannot represent another seat's body or opening.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum SeatResponse {
    /// Bounded opaque result of this player's own relay operation.
    Relayed {
        /// Seat-scoped admission or one-time Central socket ticket.
        reply: relay::Reply,
    },
    /// Current authenticated seat routing for the application relay.
    RelayRoute {
        /// Verified primary's canonical origin.
        origin: String,
        /// Bound engine game once the opening is registered.
        game: Option<Id>,
        /// Zero-based seat index.
        seat: u8,
    },
    /// Current independently verified operators and the caller's choices.
    Hosts {
        #[doc = "Current generation and match."]
        fence: Fence,
        #[doc = "One or two named operators."]
        hosts: Vec<HostConsentOffer>,
    },
    /// Participants' published handles in stable seat order.
    Participants {
        #[doc = "Requested match."]
        match_id: Id,
        #[doc = "Only explicitly published handles."]
        seats: [Option<String>; 2],
    },
    /// The caller's existing ranked queue shape.
    Queue {
        #[doc = "This player's queue state."]
        state: QueueState,
    },
    /// Latest authority and prefix.
    Status {
        #[doc = "Public-to-owner routing data."]
        status: SeatStatus,
    },
    /// Durable acknowledgement before independent verification.
    Receipt {
        #[doc = "Stable command identity."]
        command_id: Id,
        #[doc = "Assigned journal position."]
        position: u64,
    },
}

/// Independently verified host operations.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum HostRequest {
    /// Poll the ordered suffix and certifications without retransmitting the private opening.
    Tail {
        /// Current assigned authority.
        fence: Fence,
        /// Exclusive journal position, including zero for initial certification.
        after: u64,
    },
    /// Report which player sockets the current leader observes attached.
    Presence {
        #[doc = "Current generation."]
        fence: Fence,
        #[doc = "Two-bit attached-seat mask."]
        attached: u8,
    },
    /// Advertise compatible, verified engine slots.
    Ready {
        #[doc = "Capacity including active games."]
        capacity: u8,
        #[doc = "Exact build and rules compatibility."]
        compatibility: Digest,
    },
    /// List only this gateway's own assigned replicas.
    Work,
    /// Authoritative mode and current role for one assigned game.
    Inspect {
        #[doc = "Ranked match."]
        match_id: Id,
    },
    /// Renew this generation's per-role liveness.
    Pulse {
        #[doc = "Assigned generation."]
        fence: Fence,
    },
    /// Freeze the primary's full engine opening and both exact rating pools.
    Setup {
        #[doc = "Assigned generation."]
        fence: Fence,
        #[doc = "Stable engine game."]
        game: Id,
        #[doc = "Private complete encoded setup."]
        body: SetupBytes,
        #[doc = "Ordered pools corresponding to the registered decks."]
        pools: [Vec<PoolEntry>; 2],
    },
    /// Read a bounded ordered suffix on the assigned service credential.
    Read {
        #[doc = "Assigned generation."]
        fence: Fence,
        #[doc = "Exclusive position, zero requests the opening too."]
        after: u64,
    },
    /// Submit this executor's own evidence at a specific input.
    Attest {
        #[doc = "Current generation."]
        fence: Fence,
        #[doc = "Exact log commitment."]
        head: Head,
        #[doc = "Private engine digest."]
        state: Digest,
        #[doc = "Engine accepted the attempted move."]
        accepted: bool,
        #[doc = "Seats awaiting a decision."]
        awaited: u8,
        #[doc = "Decision identities in seat order."]
        decisions: [Option<u64>; 2],
        #[doc = "Reconstructed terminal outcome."]
        terminal: Option<Outcome>,
    },
    /// Attempt a fenced takeover after the current leader's lease lapses.
    Promote {
        #[doc = "Last observed generation."]
        fence: Fence,
    },
}

/// A role whose lease can be renewed while the players review its disclosure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PendingWork {
    /// Current assignment generation without either player's identity.
    pub fence: Fence,
    /// Intended hosting function.
    pub role: Role,
}

/// Private host-only result. Journal bytes and opening cannot be represented in a seat response.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum HostResponse {
    /// Authorized single-host primary, outside the replicated protocol.
    Single,
    /// Host is assigned, but the private opening cannot yet be disclosed.
    AwaitingConsent {
        #[doc = "Current authority."]
        fence: Fence,
        #[doc = "Assigned role."]
        role: Role,
    },
    /// Durable operation acknowledged.
    Done,
    /// This gateway's active work.
    Work {
        #[doc = "Assigned bounded work list."]
        items: Vec<Work>,
        /// Assigned roles awaiting explicit consent before private work is released.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pending: Vec<PendingWork>,
    },
    /// Certified state, optional opening and a bounded ordered suffix.
    Page {
        #[doc = "Immutable opening commitment."]
        opening: Head,
        #[doc = "Private opening bytes on the first page."]
        setup: Option<SetupBytes>,
        #[doc = "Immutable validated rating pools on the first page."]
        pools: Option<[Vec<PoolEntry>; 2]>,
        #[doc = "Evidence at opening, if already independently certified."]
        initial: Option<Evidence>,
        #[doc = "Ordered suffix including any pending input."]
        entries: Vec<Entry>,
        #[doc = "Last quorum-certified head."]
        certified: Head,
    },
    /// Single authoritative assignment.
    Assignment {
        #[doc = "Current routing and role."]
        work: Work,
    },
}
