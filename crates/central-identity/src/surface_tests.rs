//! Which listener takes the operations of events, ladders, tasks and the Tower, whether
//! they act on a cookie, and which of them wait for the terms.

use super::*;
use central_api::{
    advent, challenges,
    ranked::{self, replica},
    tower,
};

const ID: [u8; 16] = [1; 16];

const FENCE: replica::Fence = replica::Fence {
    match_id: ID,
    generation: 1,
};

fn ranked(request: ranked::Request) -> Request {
    Request::Ranked { request }
}

fn seat(request: replica::SeatRequest) -> Request {
    ranked(ranked::Request::Replicate { request })
}

fn task(request: challenges::Request) -> Request {
    Request::Challenges { request }
}

fn climb(request: tower::Request) -> Request {
    task(challenges::Request::Tower { request })
}

fn approval() -> replica::HostApproval {
    replica::HostApproval {
        epoch: 0,
        host_id: ID,
        origin: "https://play.example".into(),
        terms: None,
    }
}

/// Ladder reads that name no account of the caller's.
fn public_ladder() -> Vec<Request> {
    vec![
        ranked(ranked::Request::Board {
            season: None,
            format: ranked::Format::Standard,
            track: ranked::Track::Player,
            league: ranked::League::Gold,
            division: None,
            after: None,
            players: ranked::ai::BoardPlayers::Everyone,
        }),
        ranked(ranked::Request::Profile {
            target: ranked::ProfileTarget::Account(ID),
            season: None,
        }),
        ranked(ranked::Request::Seasons { before: None }),
    ]
}

/// Ladder operations of one player, which the game client alone makes.
fn own_ladder() -> Vec<Request> {
    vec![
        ranked(ranked::Request::OwnProfile { season: None }),
        ranked(ranked::Request::PreviewDeck {
            format: ranked::Format::Standard,
            pool: vec![],
        }),
        ranked(ranked::Request::Cancel { key: ID }),
        ranked(ranked::Request::Status),
        seat(replica::SeatRequest::Status { match_id: ID }),
        seat(replica::SeatRequest::Hosts { match_id: ID }),
        seat(replica::SeatRequest::Participants { match_id: ID }),
        seat(replica::SeatRequest::RelayRoute { fence: FENCE }),
        seat(replica::SeatRequest::Presence {
            fence: FENCE,
            connected: true,
        }),
        seat(replica::SeatRequest::Relay {
            fence: FENCE,
            call: replica::relay::Call::Admit {},
        }),
        // Saying no to an operator never waits for a document.
        seat(replica::SeatRequest::DeclineHost {
            fence: FENCE,
            host: approval(),
        }),
    ]
}

/// The account's own ranked rows, for its download: every dataset and a
/// continuation of each.
fn own_export() -> Vec<Request> {
    ranked::export::Table::ALL
        .into_iter()
        .flat_map(|table| {
            let key = match table {
                ranked::export::Table::Decks => ranked::export::Key::Deck([1; 32]),
                ranked::export::Table::Cosmetics => ranked::export::Key::Step {
                    league: 1,
                    division: 3,
                },
                _ => ranked::export::Key::Id(ID),
            };
            [None, Some(key)].map(|after| ranked(ranked::Request::Export { table, after }))
        })
        .collect()
}

/// Ladder operations that begin or continue playing.
fn playing_ladder() -> Vec<Request> {
    vec![
        ranked(ranked::Request::Join {
            key: ID,
            format: ranked::Format::Standard,
            deck: [1; 32],
        }),
        seat(replica::SeatRequest::Join {
            key: ID,
            format: ranked::Format::Standard,
            deck: [1; 32],
            ai: ranked::ai::AiOpponents::Local,
        }),
        seat(replica::SeatRequest::ApproveHosts {
            fence: FENCE,
            hosts: vec![approval()],
        }),
        seat(replica::SeatRequest::Submit {
            fence: FENCE,
            expected: replica::Head {
                position: 0,
                chain: [0; 32],
            },
            command_id: ID,
            body: replica::CommandBytes::new(vec![1]).unwrap(),
        }),
    ]
}

/// Operations under a credential that is no account's: a host's or the Tower's.
fn services() -> Vec<Request> {
    vec![
        Request::RankedHost {
            request: ranked::HostRequest::Ready { capacity: 1 },
        },
        Request::ChallengeHost {
            request: challenges::HostRequest::Status { attempt: ID },
        },
        Request::ChallengeHost {
            request: challenges::HostRequest::Inventory { from: 0, until: 1 },
        },
        Request::AdventHost {
            request: advent::table::HostRequest::Closed { game: ID },
        },
        Request::TowerIdentity {},
    ]
}

/// What the game client alone asks for, of events and for credentials.
fn client_only() -> Vec<Request> {
    let year = 2026;
    let mut all = vec![
        Request::TowerToken {},
        Request::RankedToken {
            gateway: "https://play.example".into(),
            fence: FENCE,
        },
    ];
    all.extend(
        [
            advent::Request::CurrentCalendar {},
            advent::Request::Calendar { year },
            advent::Request::Puzzle { year, door: 1 },
            advent::Request::Claim { year, door: 1 },
            advent::Request::Inventory { after: None },
            advent::Request::Collection { after: None },
        ]
        .map(|request| Request::Advent { request }),
    );
    all
}

/// Task operations that only read, end or hide something.
fn task_reads() -> Vec<Request> {
    vec![
        task(challenges::Request::Catalogue { before: None }),
        task(challenges::Request::Detail {
            challenge: ID,
            board: challenges::Board::FirstWin,
            offset: 0,
        }),
        task(challenges::Request::Progress),
        task(challenges::Request::Personal { challenge: ID }),
        task(challenges::Request::Export {
            table: challenges::ExportTable::Attempts,
            after: None,
        }),
        task(challenges::Request::Attempt { attempt: ID }),
        task(challenges::Request::Abandon { attempt: ID }),
        task(challenges::Request::Visibility {
            challenge: ID,
            listed: false,
        }),
        climb(tower::Request::Read {
            season: None,
            board: tower::Board::Ascent,
            offset: 0,
        }),
        climb(tower::Request::Run { run: ID }),
        climb(tower::Request::Locate { game: ID }),
        climb(tower::Request::Abandon {
            run: ID,
            revision: 1,
        }),
        climb(tower::Request::Visibility {
            season: ID,
            listed: false,
        }),
        climb(tower::Request::Export {
            table: tower::ExportTable::Runs,
            after: None,
        }),
        Request::TowerProgress {},
        Request::TowerAwardExport { after: None },
    ]
}

/// Task operations that begin, continue or publish something.
fn task_uses() -> Vec<Request> {
    vec![
        task(challenges::Request::Begin {
            challenge: ID,
            request_id: ID,
            listed: false,
        }),
        task(challenges::Request::Hint {
            challenge: ID,
            level: 1,
        }),
        task(challenges::Request::Visibility {
            challenge: ID,
            listed: true,
        }),
        climb(tower::Request::Begin {
            season: ID,
            request_id: ID,
            listed: false,
        }),
        climb(tower::Request::Start {
            run: ID,
            revision: 1,
            request_id: ID,
        }),
        climb(tower::Request::Choose {
            run: ID,
            revision: 1,
            track: tower::Track::Assault,
            request_id: ID,
        }),
        climb(tower::Request::Visibility {
            season: ID,
            listed: true,
        }),
    ]
}

fn name(request: &Request) -> String {
    String::from_utf8(central_api::encode(request).unwrap()).unwrap()
}

#[test]
fn the_website_reads_the_public_ladder_and_takes_no_seat() {
    for request in public_ladder() {
        assert!(browser_operation(&request), "{}", name(&request));
        // Nothing of the visitor's account goes into a public answer.
        assert!(!uses_session(&request), "{}", name(&request));
        assert!(!browser_only(&request), "{}", name(&request));
        assert!(!terms::needs_acceptance(&request), "{}", name(&request));
    }
    for request in own_ladder().into_iter().chain(playing_ladder()) {
        assert!(!browser_operation(&request), "{}", name(&request));
        assert!(!browser_only(&request), "{}", name(&request));
    }
}

#[test]
fn the_ranked_export_is_the_signed_in_account_s_on_both_listeners_without_the_terms() {
    for request in own_export() {
        assert!(browser_operation(&request), "{}", name(&request));
        // The account comes from the session, never from the request.
        assert!(uses_session(&request), "{}", name(&request));
        assert!(!browser_only(&request), "{}", name(&request));
        assert!(!terms::needs_acceptance(&request), "{}", name(&request));
        let Request::Ranked { request } = &request else {
            unreachable!()
        };
        assert!(!request.is_public());
    }
}

#[test]
fn a_service_credential_s_operation_never_comes_from_a_cookie() {
    for request in services().into_iter().chain(client_only()) {
        assert!(!browser_operation(&request), "{}", name(&request));
        assert!(!uses_session(&request), "{}", name(&request));
        // The native listener takes them; their credential decides there.
        assert!(!browser_only(&request), "{}", name(&request));
    }
}

#[test]
fn tasks_belong_to_the_signed_in_account_on_both_listeners() {
    for request in task_reads().into_iter().chain(task_uses()) {
        assert!(browser_operation(&request), "{}", name(&request));
        assert!(uses_session(&request), "{}", name(&request));
        assert!(!browser_only(&request), "{}", name(&request));
    }
}

#[test]
fn playing_waits_for_the_terms_and_seeing_or_leaving_does_not() {
    let waits = playing_ladder()
        .into_iter()
        .chain(task_uses())
        .chain(client_only());
    for request in waits {
        assert!(terms::needs_acceptance(&request), "{}", name(&request));
    }
    let open = own_ladder()
        .into_iter()
        .chain(task_reads())
        .chain(services());
    for request in open {
        assert!(!terms::needs_acceptance(&request), "{}", name(&request));
    }
}
