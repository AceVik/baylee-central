use super::{Error, Session, Token, Transport, api};
use api::replica::{HostRequest, HostResponse, Phase, SeatRequest, SeatResponse};

pub(super) fn responds(
    request: &SeatRequest,
    response: &SeatResponse,
    owner: Option<api::Id>,
) -> bool {
    match (request, response) {
        (SeatRequest::Relay { call, .. }, SeatResponse::Relayed { reply }) => match (call, reply) {
            (
                api::replica::relay::Call::SocketTicket { .. },
                api::replica::relay::Reply::Socket { ticket },
            ) => ticket.is_capability(),
            (
                api::replica::relay::Call::Admit { .. }
                | api::replica::relay::Call::Play { .. }
                | api::replica::relay::Call::Reclaim { .. },
                api::replica::relay::Reply::Http { status, body },
            ) => (200..=599).contains(status) && (*status == 200 || body.is_none()),
            _ => false,
        },
        (SeatRequest::RelayRoute { .. }, SeatResponse::RelayRoute { seat, .. }) => *seat < 2,
        (SeatRequest::Join { key, format, deck }, SeatResponse::Queue { state }) => match state {
            api::QueueState::Assigned {
                key: k,
                assignment: a,
            } => {
                k == key
                    && a.format == *format
                    && a.deck == *deck
                    && Some(a.account) == owner
                    && a.seat < 2
            }
            api::QueueState::Waiting {
                key: k, format: f, ..
            }
            | api::QueueState::Finished {
                key: k, format: f, ..
            } => k == key && f == format,
            api::QueueState::Idle => false,
        },
        (
            SeatRequest::Submit {
                command_id,
                expected,
                ..
            },
            SeatResponse::Receipt {
                command_id: actual,
                position,
            },
        ) => actual == command_id && expected.position.checked_add(1) == Some(*position),
        (SeatRequest::Status { match_id }, SeatResponse::Status { status }) => {
            status.fence.match_id == *match_id
        }
        (
            SeatRequest::Presence { fence, .. } | SeatRequest::DeclineHost { fence, .. },
            SeatResponse::Status { status },
        ) => status.fence == *fence,
        (SeatRequest::Hosts { match_id }, SeatResponse::Hosts { fence, .. }) => {
            fence.match_id == *match_id
        }
        (
            SeatRequest::ApproveHosts {
                fence,
                hosts: requested,
            },
            SeatResponse::Hosts {
                fence: actual,
                hosts,
            },
        ) => {
            *fence == *actual
                && requested.iter().all(|h| {
                    hosts
                        .iter()
                        .any(|o| o.host == *h && o.approved && !o.needs_gateway_terms)
                })
        }
        (
            SeatRequest::Participants { match_id },
            SeatResponse::Participants {
                match_id: actual,
                seats,
            },
        ) => {
            match_id == actual
                && seats
                    .iter()
                    .flatten()
                    .all(|s| s.len() <= 32 && s.chars().all(central_api::gateway::shown_char))
        }
        _ => false,
    }
}

impl Session {
    /// Use this player's own Central session for a ranked command or a fenced routing read.
    ///
    /// # Errors
    /// Refuses a foreign service, invalid scope or a reply for a different operation.
    pub fn replica(
        &self,
        transport: &impl Transport,
        request: SeatRequest,
    ) -> Result<SeatResponse, Error> {
        let development = transport.endpoint().origin().starts_with("http://");
        request.validate(development)?;
        let api::Response::Replica { response } =
            self.ranked(transport, api::Request::Replicate { request })?
        else {
            return Err(Error::Rejected);
        };
        let valid = match &response {
            SeatResponse::Status { status } => {
                central_api::gateway::validate_origin(&status.origin, development).is_ok()
                    && status.head.position <= api::replica::MAX_POSITION
                    && (status.phase != Phase::Single || status.fence.generation == 0)
            }
            SeatResponse::Hosts { hosts, .. } => {
                matches!(hosts.len(), 1 | 2)
                    && hosts.iter().all(|o| o.host.valid(development))
                    && hosts.windows(2).all(|p| {
                        p[0].host.host_id != p[1].host.host_id
                            && p[0].host.origin != p[1].host.origin
                            && p[0].role != p[1].role
                    })
            }
            _ => true,
        };
        if valid {
            Ok(response)
        } else {
            Err(Error::Rejected)
        }
    }
}

/// Exchange a verified gateway's narrowly scoped replication operation with Central.
///
/// # Errors
/// Refuses invalid input, authentication failures and a mismatched or oversized reply.
pub fn replica_host(
    transport: &impl Transport,
    credential: &Token,
    request: HostRequest,
) -> Result<HostResponse, Error> {
    request.validate()?;
    let wire = crate::Request::RankedHost {
        request: api::HostRequest::Replicate { request },
    };
    let response = transport.exchange(&wire, Some(credential.into()))?;
    let (
        crate::Request::RankedHost {
            request: api::HostRequest::Replicate { request },
        },
        crate::Response::RankedHost {
            response: api::HostResponse::Replica { response },
        },
    ) = (wire, response)
    else {
        return Err(Error::Rejected);
    };
    let valid = match (request, &response) {
        (HostRequest::Work, HostResponse::Work { items, pending }) => {
            u8::try_from(items.len() + pending.len()).is_ok()
        }
        (
            HostRequest::Read { after, .. } | HostRequest::Tail { after, .. },
            HostResponse::Page {
                opening,
                setup,
                pools,
                entries,
                certified,
                ..
            },
        ) => {
            opening.position == 0
                && certified.position <= api::replica::MAX_POSITION
                && entries.len() <= api::replica::PAGE
                && setup.is_some() == pools.is_some()
                && (after == 0 || setup.is_none())
                && entries
                    .iter()
                    .zip(after + 1..)
                    .all(|(e, pos)| e.head.position == pos && pos <= api::replica::MAX_POSITION)
        }
        (HostRequest::Inspect { match_id }, HostResponse::Assignment { work }) => {
            work.fence.match_id == match_id
        }
        (HostRequest::Inspect { match_id }, HostResponse::AwaitingConsent { fence, .. }) => {
            fence.match_id == match_id
        }
        (
            HostRequest::Pulse { fence } | HostRequest::Promote { fence },
            HostResponse::Assignment { work },
        ) => fence.match_id == work.fence.match_id && work.fence.generation >= fence.generation,
        (HostRequest::Pulse { fence }, HostResponse::AwaitingConsent { fence: actual, .. }) => {
            fence == *actual
        }
        (
            HostRequest::Presence { .. }
            | HostRequest::Ready { .. }
            | HostRequest::Setup { .. }
            | HostRequest::Attest { .. }
            | HostRequest::Pulse { .. }
            | HostRequest::Promote { .. },
            HostResponse::Done,
        )
        | (HostRequest::Inspect { .. }, HostResponse::Single) => true,
        _ => false,
    };
    if valid {
        Ok(response)
    } else {
        Err(Error::Rejected)
    }
}
