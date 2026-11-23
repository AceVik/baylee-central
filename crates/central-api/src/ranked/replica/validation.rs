use super::{Fence, HostApproval, HostRequest, MAX_POSITION, SeatRequest};
use crate::{Error, gateway, ranked::valid_pool};

pub(super) const fn uuid(id: &[u8; 16]) -> bool {
    id[6] >> 4 == 7 && id[8] >> 6 == 2
}

impl Fence {
    fn valid(self, allow_single: bool) -> bool {
        uuid(&self.match_id)
            && (allow_single || self.generation > 0)
            && i64::try_from(self.generation).is_ok()
    }
}

impl HostApproval {
    /// Check the proposed verified operator and the published document binding.
    #[must_use]
    pub fn valid(&self, development: bool) -> bool {
        uuid(&self.host_id)
            && i64::try_from(self.epoch).is_ok()
            && gateway::validate_origin(&self.origin, development).is_ok()
            && self
                .terms
                .as_ref()
                .is_none_or(|v| crate::terms::valid_version(&v.version))
    }
}

impl SeatRequest {
    /// Validate bounded private operation parameters before account storage.
    ///
    /// # Errors
    /// Refuses invalid identifiers, origins, terms, bounds and generations.
    pub fn validate(&self, development: bool) -> Result<(), Error> {
        let valid = match self {
            Self::Relay { fence, call } => fence.valid(true) && call.valid(),
            Self::RelayRoute { fence } => fence.valid(true),
            Self::Join { key, .. } => uuid(key),
            Self::Status { match_id }
            | Self::Hosts { match_id }
            | Self::Participants { match_id } => uuid(match_id),
            Self::ApproveHosts { fence, hosts } => {
                fence.valid(true)
                    && matches!(hosts.len(), 1 | 2)
                    && hosts.iter().all(|v| v.valid(development))
                    && hosts
                        .windows(2)
                        .all(|v| v[0].host_id != v[1].host_id && v[0].origin != v[1].origin)
            }
            Self::DeclineHost { fence, host } => fence.valid(true) && host.valid(development),
            Self::Presence { fence, .. } => fence.valid(false),
            Self::Submit {
                fence,
                expected,
                command_id,
                ..
            } => fence.valid(false) && expected.position < MAX_POSITION && uuid(command_id),
        };
        valid.then_some(()).ok_or(Error::Rejected)
    }
}

impl HostRequest {
    /// Validate host input shape separately from DB authorization and replay.
    ///
    /// # Errors
    /// Refuses invalid identifiers, positions, seat masks, pools and generations.
    pub fn validate(&self) -> Result<(), Error> {
        let valid = match self {
            Self::Work | Self::Ready { .. } => true,
            Self::Inspect { match_id } => uuid(match_id),
            Self::Pulse { fence } | Self::Promote { fence } => fence.valid(false),
            Self::Presence { fence, attached } => fence.valid(false) && *attached < 4,
            Self::Setup {
                fence, game, pools, ..
            } => fence.valid(false) && uuid(game) && pools.iter().all(|p| valid_pool(p)),
            Self::Read { fence, after } | Self::Tail { fence, after } => {
                fence.valid(false) && *after <= MAX_POSITION
            }
            Self::Attest {
                fence,
                head,
                awaited,
                decisions,
                terminal,
                ..
            } => {
                fence.valid(false)
                    && head.position <= MAX_POSITION
                    && *awaited < 4
                    && (*awaited & 1 != 0) == decisions[0].is_some()
                    && (*awaited & 2 != 0) == decisions[1].is_some()
                    && (terminal.is_none() || *awaited == 0)
            }
        };
        valid.then_some(()).ok_or(Error::Rejected)
    }
}
