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
            SeatRequest::Presence { fence, .. }
            | SeatRequest::Submit { fence, .. }
            | SeatRequest::DeclineHost { fence, .. },
            SeatResponse::Status { status },
        ) => status.fence.match_id == fence.match_id && status.fence.generation >= fence.generation,
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
        (
            HostRequest::Pulse { fence } | HostRequest::Promote { fence },
            HostResponse::AwaitingConsent { fence: actual, .. },
        ) => fence.match_id == actual.match_id && actual.generation >= fence.generation,
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

#[cfg(test)]
mod tests {
    use super::*;
    use api::replica::{CommandBytes, ConsentState, Fence, Head, SeatStatus};

    #[test]
    fn pending_submit_can_return_authority_status_but_never_acknowledges_another_command() {
        let fence = Fence {
            match_id: [1; 16],
            generation: 3,
        };
        let expected = Head {
            position: 7,
            chain: [8; 32],
        };
        let r = SeatRequest::Submit {
            fence,
            expected,
            command_id: [3; 16],
            body: CommandBytes::new(vec![5]).unwrap(),
        };
        let reply = |fence| SeatResponse::Status {
            status: SeatStatus {
                fence,
                phase: Phase::Paused,
                host_id: [2; 16],
                origin: "https://gate.example".into(),
                game: Some([4; 16]),
                consent: ConsentState::Ready,
                head: expected,
                clock: None,
            },
        };
        for term in [3, 4] {
            assert!(responds(
                &r,
                &reply(Fence {
                    generation: term,
                    ..fence
                }),
                Some([5; 16])
            ));
        }
        assert!(!responds(
            &r,
            &reply(Fence {
                generation: 2,
                ..fence
            }),
            Some([5; 16])
        ));
        assert!(!responds(
            &r,
            &reply(Fence {
                match_id: [9; 16],
                ..fence
            }),
            Some([5; 16])
        ));
        assert!(responds(
            &r,
            &SeatResponse::Receipt {
                command_id: [3; 16],
                position: 8
            },
            Some([5; 16])
        ));
        assert!(!responds(
            &r,
            &SeatResponse::Receipt {
                command_id: [9; 16],
                position: 8
            },
            Some([5; 16])
        ));
        assert!(!responds(
            &r,
            &SeatResponse::Receipt {
                command_id: [3; 16],
                position: 9
            },
            Some([5; 16])
        ));
    }

    const FENCE: Fence = Fence {
        match_id: [1; 16],
        generation: 3,
    };
    const OWNER: api::Id = [5; 16];

    fn answered(request: &SeatRequest, response: &SeatResponse) -> bool {
        responds(request, response, Some(OWNER))
    }

    #[test]
    fn an_assignment_is_this_account_s_own_for_the_deck_it_queued() {
        let join = SeatRequest::Join {
            key: [2; 16],
            format: api::Format::Standard,
            deck: [7; 32],
        };
        let base = || api::Assignment {
            match_id: [1; 16],
            host_id: [3; 16],
            origin: "https://play.example".into(),
            format: api::Format::Standard,
            account: OWNER,
            seat: 1,
            deck: [7; 32],
            game: None,
        };
        let reply = |key, assignment| SeatResponse::Queue {
            state: api::QueueState::Assigned { key, assignment },
        };
        assert!(answered(&join, &reply([2; 16], base())));
        assert!(!responds(&join, &reply([2; 16], base()), None));
        assert!(!responds(&join, &reply([2; 16], base()), Some([6; 16])));
        assert!(!answered(&join, &reply([9; 16], base())));
        let changes: [fn(&mut api::Assignment); 4] = [
            |a| a.format = api::Format::Commander,
            |a| a.account = [6; 16],
            |a| a.seat = 2,
            |a| a.deck[31] ^= 1,
        ];
        for change in changes {
            let mut assignment = base();
            change(&mut assignment);
            assert!(
                !answered(&join, &reply([2; 16], assignment.clone())),
                "{assignment:?}"
            );
        }
        let waiting = |key, format| SeatResponse::Queue {
            state: api::QueueState::Waiting {
                key,
                format,
                queued_at: 1,
            },
        };
        assert!(answered(&join, &waiting([2; 16], api::Format::Standard)));
        assert!(!answered(&join, &waiting([9; 16], api::Format::Standard)));
        assert!(!answered(&join, &waiting([2; 16], api::Format::Commander)));
        // A join that leads to no queue was not answered.
        assert!(!answered(
            &join,
            &SeatResponse::Queue {
                state: api::QueueState::Idle
            }
        ));
    }

    #[test]
    fn a_consent_counts_only_for_the_hosts_and_generation_that_were_approved() {
        use api::replica::{HostApproval, HostConsentOffer, Role};
        let host = |n: u8| HostApproval {
            epoch: 1,
            host_id: [n; 16],
            origin: format!("https://host{n}.example"),
            terms: None,
        };
        let offer = |host, approved, needs_gateway_terms| HostConsentOffer {
            host,
            role: Role::Primary,
            approved,
            needs_gateway_terms,
        };
        let request = SeatRequest::ApproveHosts {
            fence: FENCE,
            hosts: vec![host(1), host(2)],
        };
        let reply = |fence, hosts| SeatResponse::Hosts { fence, hosts };
        assert!(answered(
            &request,
            &reply(
                FENCE,
                vec![offer(host(2), true, false), offer(host(1), true, false)]
            )
        ));
        for hosts in [
            vec![],
            vec![offer(host(1), true, false)],
            vec![offer(host(1), true, false), offer(host(2), false, false)],
            vec![offer(host(1), true, false), offer(host(2), true, true)],
            vec![offer(host(1), true, false), offer(host(3), true, false)],
            // The same operator at another epoch is another disclosure.
            vec![
                offer(host(1), true, false),
                offer(
                    HostApproval {
                        epoch: 2,
                        ..host(2)
                    },
                    true,
                    false,
                ),
            ],
        ] {
            assert!(
                !answered(&request, &reply(FENCE, hosts.clone())),
                "{hosts:?}"
            );
        }
        let both = vec![offer(host(1), true, false), offer(host(2), true, false)];
        for fence in [
            Fence {
                generation: 4,
                ..FENCE
            },
            Fence {
                match_id: [9; 16],
                ..FENCE
            },
        ] {
            assert!(!answered(&request, &reply(fence, both.clone())));
        }
        let hosts = SeatRequest::Hosts { match_id: [1; 16] };
        assert!(answered(&hosts, &reply(FENCE, vec![])));
        let other = Fence {
            match_id: [9; 16],
            ..FENCE
        };
        assert!(!answered(&hosts, &reply(other, vec![])));
    }

    #[test]
    fn a_relayed_answer_fits_its_call_and_a_refusal_carries_no_body() {
        use api::replica::relay::{Call, HttpBytes, Reply, Secret};
        let secret = |text: &str| Secret::new(text.into()).unwrap();
        let good = "a".repeat(64);
        let relay = |call| SeatRequest::Relay { fence: FENCE, call };
        let admit = relay(Call::Admit {});
        let reclaim = relay(Call::Reclaim {
            bearer: secret(&good),
            game: [4; 16],
        });
        let ticket = relay(Call::SocketTicket {
            game: [4; 16],
            seat_token: secret(&good),
        });
        let http = |status, body: Option<&[u8]>| SeatResponse::Relayed {
            reply: Reply::Http {
                status,
                body: body.map(|bytes| HttpBytes::new(bytes.to_vec()).unwrap()),
            },
        };
        let socket = |text: &str| SeatResponse::Relayed {
            reply: Reply::Socket {
                ticket: secret(text),
            },
        };
        for request in [&admit, &reclaim] {
            assert!(answered(request, &http(200, Some(b"{}"))));
            for status in [202, 400, 599] {
                assert!(answered(request, &http(status, None)), "{status}");
                assert!(!answered(request, &http(status, Some(b"{}"))), "{status}");
            }
            for status in [0, 101, 199, 600] {
                assert!(!answered(request, &http(status, None)), "{status}");
            }
            assert!(!answered(request, &socket(&good)));
        }
        assert!(answered(&ticket, &socket(&good)));
        assert!(!answered(&ticket, &socket(&good[1..])));
        assert!(!answered(&ticket, &socket(&good.to_uppercase())));
        assert!(!answered(&ticket, &http(200, Some(b"{}"))));
    }

    #[test]
    fn a_route_and_the_names_at_a_table_are_bounded_before_they_are_shown() {
        let route = SeatRequest::RelayRoute { fence: FENCE };
        for (seat, accepted) in [(0, true), (1, true), (2, false), (255, false)] {
            let reply = SeatResponse::RelayRoute {
                origin: "https://play.example".into(),
                game: None,
                seat,
            };
            assert_eq!(answered(&route, &reply), accepted, "{seat}");
        }
        let names = SeatRequest::Participants { match_id: [1; 16] };
        let reply = |match_id, name: &str| SeatResponse::Participants {
            match_id,
            seats: [None, Some(name.into())],
        };
        assert!(answered(&names, &reply([1; 16], &"x".repeat(32))));
        assert!(!answered(&names, &reply([9; 16], "x")));
        for name in [
            "x".repeat(33).as_str(),
            "two\nlines",
            "over\u{202e}ride",
            "\u{feff}",
            "bell\u{7}",
        ] {
            assert!(!answered(&names, &reply([1; 16], name)), "{name:?}");
        }
    }

    #[test]
    fn no_answer_serves_a_request_of_another_kind() {
        let requests = [
            SeatRequest::RelayRoute { fence: FENCE },
            SeatRequest::Hosts { match_id: [1; 16] },
            SeatRequest::Participants { match_id: [1; 16] },
        ];
        let replies = [
            SeatResponse::RelayRoute {
                origin: "https://play.example".into(),
                game: None,
                seat: 0,
            },
            SeatResponse::Hosts {
                fence: FENCE,
                hosts: vec![],
            },
            SeatResponse::Participants {
                match_id: [1; 16],
                seats: [None, None],
            },
            SeatResponse::Receipt {
                command_id: [1; 16],
                position: 1,
            },
        ];
        for (i, request) in requests.iter().enumerate() {
            for (j, reply) in replies.iter().enumerate() {
                assert_eq!(answered(request, reply), i == j, "{i} {j}");
            }
        }
    }
}
