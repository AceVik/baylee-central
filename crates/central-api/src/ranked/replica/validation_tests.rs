//! Refusals of both replica request shapes, at the exact bounds.

use super::relay::{Call, Secret};
use super::*;
use crate::ranked::{PoolEntry, valid_pool};
use crate::terms::TermsVersion;
use crate::{decode, encode};

fn id(n: u8) -> Id {
    let mut id = [n; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id
}

fn fence(generation: u64) -> Fence {
    Fence {
        match_id: id(1),
        generation,
    }
}

const HEAD: Head = Head {
    position: 0,
    chain: [0; 32],
};

fn pool(entries: &[(u8, u16)]) -> Vec<PoolEntry> {
    entries
        .iter()
        .map(|&(card, copies)| PoolEntry {
            card: [card; 32],
            copies,
        })
        .collect()
}

fn attest(awaited: u8, decisions: [Option<u64>; 2], terminal: Option<Outcome>) -> HostRequest {
    HostRequest::Attest {
        fence: fence(1),
        head: HEAD,
        state: [1; 32],
        accepted: true,
        awaited,
        decisions,
        terminal,
    }
}

fn approval(host: u8, origin: &str) -> HostApproval {
    HostApproval {
        epoch: 0,
        host_id: id(host),
        origin: origin.into(),
        terms: None,
    }
}

/// Identifiers that are not `UUIDv7`: wrong version nibble, wrong variant bits.
fn foreign_ids() -> [Id; 5] {
    let with = |index: usize, value: u8| {
        let mut id = id(1);
        id[index] = value;
        id
    };
    [
        [0; 16],
        with(6, 0x40),
        with(6, 0x80),
        with(8, 0x40),
        with(8, 0xc0),
    ]
}

#[test]
fn a_host_request_needs_a_real_match_and_a_started_generation() {
    let shapes: [fn(Fence) -> HostRequest; 6] = [
        |fence| HostRequest::Pulse { fence },
        |fence| HostRequest::Promote { fence },
        |fence| HostRequest::Presence { fence, attached: 3 },
        |fence| HostRequest::Read { fence, after: 0 },
        |fence| HostRequest::Tail { fence, after: 0 },
        |fence| HostRequest::Setup {
            fence,
            game: id(2),
            body: SetupBytes::new(vec![1]).unwrap(),
            pools: [pool(&[(1, 1)]), pool(&[(2, 1)])],
        },
    ];
    for shape in shapes {
        assert!(shape(fence(1)).validate().is_ok());
        // The store keeps a generation as a signed 64-bit integer.
        assert!(shape(fence(i64::MAX.unsigned_abs())).validate().is_ok());
        assert!(
            shape(fence(i64::MAX.unsigned_abs() + 1))
                .validate()
                .is_err()
        );
        assert!(shape(fence(u64::MAX)).validate().is_err());
        assert!(shape(fence(0)).validate().is_err());
        for match_id in foreign_ids() {
            let fence = Fence {
                match_id,
                generation: 1,
            };
            assert!(shape(fence).validate().is_err(), "{match_id:x?}");
        }
    }
    assert!(HostRequest::Inspect { match_id: id(1) }.validate().is_ok());
    for match_id in foreign_ids() {
        assert!(HostRequest::Inspect { match_id }.validate().is_err());
    }
}

#[test]
fn a_host_request_stops_at_the_last_seat_bit_and_journal_position() {
    for attached in 0..=u8::MAX {
        let request = HostRequest::Presence {
            fence: fence(1),
            attached,
        };
        assert_eq!(request.validate().is_ok(), attached < 4, "{attached}");
    }
    for (after, accepted) in [
        (MAX_POSITION, true),
        (MAX_POSITION + 1, false),
        (u64::MAX, false),
    ] {
        let fence = fence(1);
        for request in [
            HostRequest::Read { fence, after },
            HostRequest::Tail { fence, after },
        ] {
            assert_eq!(request.validate().is_ok(), accepted, "{after}");
        }
    }
}

#[test]
fn a_setup_names_its_game_and_two_canonical_pools() {
    let setup = |game, pools| HostRequest::Setup {
        fence: fence(1),
        game,
        body: SetupBytes::new(vec![1]).unwrap(),
        pools,
    };
    let good = || pool(&[(1, 1)]);
    assert!(setup(id(2), [good(), good()]).validate().is_ok());
    for game in foreign_ids() {
        assert!(setup(game, [good(), good()]).validate().is_err());
    }
    // One bad pool is enough, in either seat.
    assert!(setup(id(2), [vec![], good()]).validate().is_err());
    assert!(setup(id(2), [good(), vec![]]).validate().is_err());
}

#[test]
fn a_pool_is_one_strictly_ordered_multiset_of_at_most_250_cards() {
    assert!(valid_pool(&pool(&[(1, 249), (2, 1)])));
    assert!(valid_pool(&pool(&[(7, 250)])));
    for (entries, why) in [
        (pool(&[]), "empty"),
        (pool(&[(1, 0)]), "no copies"),
        (pool(&[(1, 4), (2, 0)]), "no copies behind a real entry"),
        (pool(&[(1, 2), (1, 2)]), "the same card twice"),
        (pool(&[(2, 1), (1, 1)]), "descending"),
        (pool(&[(1, 251)]), "one card too many"),
        (
            pool(&[(1, 250), (2, 1)]),
            "one card too many over two entries",
        ),
        // A 16-bit total would wrap to a small number here.
        (pool(&[(1, u16::MAX), (2, 2)]), "wrapping total"),
        (
            pool(&[(1, u16::MAX), (2, u16::MAX)]),
            "two wrapping entries",
        ),
    ] {
        assert!(!valid_pool(&entries), "{why}");
    }
    let distinct: Vec<PoolEntry> = (0..251_u16)
        .map(|n| {
            let mut card = [0; 32];
            card[..2].copy_from_slice(&n.to_be_bytes());
            PoolEntry { card, copies: 1 }
        })
        .collect();
    assert!(valid_pool(&distinct[..250]));
    assert!(!valid_pool(&distinct));
}

#[test]
fn an_attested_decision_exists_for_exactly_the_awaited_seats() {
    let decision = [None, Some(7)];
    for awaited in 0..=u8::MAX {
        for first in decision {
            for second in decision {
                let consistent = awaited < 4
                    && first.is_some() == (awaited % 2 == 1)
                    && second.is_some() == (awaited / 2 % 2 == 1);
                assert_eq!(
                    attest(awaited, [first, second], None).validate().is_ok(),
                    consistent,
                    "{awaited} {first:?} {second:?}"
                );
            }
        }
    }
}

#[test]
fn a_finished_game_awaits_nobody() {
    for terminal in [Outcome::FirstWins, Outcome::SecondWins, Outcome::Draw] {
        assert!(attest(0, [None, None], Some(terminal)).validate().is_ok());
        for (awaited, decisions) in [
            (1, [Some(1), None]),
            (2, [None, Some(1)]),
            (3, [Some(1), Some(1)]),
        ] {
            assert!(attest(awaited, decisions, None).validate().is_ok());
            assert!(
                attest(awaited, decisions, Some(terminal))
                    .validate()
                    .is_err()
            );
        }
    }
}

#[test]
fn an_attested_head_lies_inside_the_journal() {
    for (position, accepted) in [(MAX_POSITION, true), (MAX_POSITION + 1, false)] {
        let request = HostRequest::Attest {
            fence: fence(1),
            head: Head { position, ..HEAD },
            state: [1; 32],
            accepted: true,
            awaited: 0,
            decisions: [None, None],
            terminal: None,
        };
        assert_eq!(request.validate().is_ok(), accepted, "{position}");
    }
}

#[test]
fn a_seat_submits_behind_the_last_position_and_never_before_a_generation() {
    let submit = |fence, position, command_id| SeatRequest::Submit {
        fence,
        expected: Head { position, ..HEAD },
        command_id,
        body: CommandBytes::new(vec![1]).unwrap(),
    };
    assert!(
        submit(fence(1), MAX_POSITION - 1, id(3))
            .validate(false)
            .is_ok()
    );
    // At the last position no successor head exists.
    assert!(
        submit(fence(1), MAX_POSITION, id(3))
            .validate(false)
            .is_err()
    );
    assert!(submit(fence(0), 0, id(3)).validate(false).is_err());
    for command in foreign_ids() {
        assert!(submit(fence(1), 0, command).validate(false).is_err());
    }
    for (generation, accepted) in [(0, false), (1, true), (u64::MAX, false)] {
        let presence = SeatRequest::Presence {
            fence: fence(generation),
            connected: true,
        };
        assert_eq!(presence.validate(false).is_ok(), accepted, "{generation}");
    }
}

#[test]
fn consent_and_relay_shapes_accept_the_single_generation_and_no_foreign_match() {
    let shapes: [fn(Fence) -> SeatRequest; 4] = [
        |fence| SeatRequest::RelayRoute { fence },
        |fence| SeatRequest::Relay {
            fence,
            call: Call::Admit {},
        },
        |fence| SeatRequest::ApproveHosts {
            fence,
            hosts: vec![approval(2, "https://play.example")],
        },
        |fence| SeatRequest::DeclineHost {
            fence,
            host: approval(2, "https://play.example"),
        },
    ];
    for shape in shapes {
        assert!(shape(fence(0)).validate(false).is_ok());
        assert!(shape(fence(u64::MAX)).validate(false).is_err());
        for match_id in foreign_ids() {
            let fence = Fence {
                match_id,
                generation: 1,
            };
            assert!(shape(fence).validate(false).is_err(), "{match_id:x?}");
        }
    }
    let by_match: [fn(Id) -> SeatRequest; 3] = [
        |match_id| SeatRequest::Status { match_id },
        |match_id| SeatRequest::Hosts { match_id },
        |match_id| SeatRequest::Participants { match_id },
    ];
    for shape in by_match {
        assert!(shape(id(1)).validate(false).is_ok());
        for foreign in foreign_ids() {
            assert!(shape(foreign).validate(false).is_err());
        }
    }
    for key in foreign_ids() {
        let join = SeatRequest::Join {
            key,
            format: Format::Standard,
            deck: [1; 32],
            ai: crate::ranked::ai::AiOpponents::Local,
        };
        assert!(join.validate(false).is_err());
    }
}

#[test]
fn approval_names_one_or_two_different_operators() {
    let approve = |hosts| {
        SeatRequest::ApproveHosts {
            fence: fence(1),
            hosts,
        }
        .validate(false)
        .is_ok()
    };
    let (a, b) = ("https://a.example", "https://b.example");
    assert!(approve(vec![approval(2, a), approval(3, b)]));
    assert!(!approve(vec![]));
    assert!(!approve(vec![approval(2, a), approval(2, b)]), "same host");
    assert!(
        !approve(vec![approval(2, a), approval(3, a)]),
        "same origin"
    );
    assert!(!approve(vec![
        approval(2, a),
        approval(3, b),
        approval(4, "https://c.example"),
    ]));
}

#[test]
fn an_approved_operator_has_a_public_origin_a_storable_epoch_and_published_terms() {
    assert!(approval(2, "https://play.example:8443").valid(false));
    for origin in [
        "",
        "http://play.example",
        "https://play.example/",
        "https://play.example/path",
        "https://play.example.",
        "https://PLAY.example",
        "https://user@play.example",
        "https://user:pass@play.example",
        "https://play.example:443",
        "https://play.example?next=https://evil.example",
        "https://play.example#x",
        "https://play.example\n",
        " https://play.example",
        "javascript:alert(1)",
        "http://127.0.0.1:8080",
    ] {
        assert!(!approval(2, origin).valid(false), "{origin:?}");
    }
    // Loopback is an operator only where the caller says this is development.
    assert!(approval(2, "http://127.0.0.1:8080").valid(true));
    assert!(!approval(2, "http://192.0.2.1:8080").valid(true));

    let changed = |change: fn(&mut HostApproval)| {
        let mut approval = approval(2, "https://play.example");
        change(&mut approval);
        approval.valid(false)
    };
    assert!(changed(|a| a.epoch = i64::MAX.unsigned_abs()));
    assert!(!changed(|a| a.epoch = i64::MAX.unsigned_abs() + 1));
    assert!(!changed(|a| a.host_id = [0; 16]));
    let terms = |version: &str| {
        let mut approval = approval(2, "https://play.example");
        approval.terms = Some(TermsVersion {
            version: version.into(),
            digest: [1; 32],
        });
        approval.valid(false)
    };
    assert!(terms("2026-11-01"));
    for version in ["", "two words", "line\nbreak", crate::terms::DRAFT, "ä"] {
        assert!(!terms(version), "{version:?}");
    }
}

#[test]
fn a_relayed_capability_is_exactly_64_lowercase_hex_digits() {
    let secret = |text: &str| Secret::new(text.into()).unwrap();
    let good = "0123456789abcdef".repeat(4);
    let calls = |capability: &str, game: Id| {
        [
            Call::Reclaim {
                bearer: secret(capability),
                game,
            },
            Call::SocketTicket {
                game,
                seat_token: secret(capability),
            },
        ]
    };
    let relay = |call| SeatRequest::Relay {
        fence: fence(1),
        call,
    };
    for call in calls(&good, id(2)) {
        assert!(relay(call).validate(false).is_ok());
    }
    for game in foreign_ids() {
        for call in calls(&good, game) {
            assert!(relay(call).validate(false).is_err());
        }
    }
    let play = |capability: &str| {
        relay(Call::Play {
            bearer: secret(capability),
            deck: SetupBytes::new(vec![1]).unwrap(),
        })
    };
    assert!(play(&good).validate(false).is_ok());
    for bad in [
        &good[1..],
        &format!("{good}0"),
        &good.to_uppercase(),
        &good.replace('f', "g"),
        &good.replace("01", "0x"),
    ] {
        assert!(play(bad).validate(false).is_err(), "{bad}");
        for call in calls(bad, id(2)) {
            assert!(relay(call).validate(false).is_err(), "{bad}");
        }
    }
}

#[test]
fn a_relay_secret_cannot_carry_a_header_break_or_outgrow_a_token() {
    let most = "a".repeat(crate::SignedToken::MAX_BYTES);
    assert_eq!(Secret::new(most.clone()).unwrap().expose(), most);
    for bad in [
        String::new(),
        format!("{most}a"),
        "token\r\nX-Injected: 1".into(),
        "two words".into(),
        "tab\t".into(),
        "nul\0".into(),
        "del\u{7f}".into(),
        "tökén".into(),
    ] {
        assert!(Secret::new(bad.clone()).is_err());
        // The wire applies the same check as the constructor.
        assert!(decode::<Secret>(&encode(&bad).unwrap()).is_err());
    }
}

#[test]
fn the_opening_binds_which_seat_plays_which_cards() {
    let body = SetupBytes::new(vec![1]).unwrap();
    let opening = |first: &[(u8, u16)], second: &[(u8, u16)]| {
        Head::opening(
            id(1),
            id(2),
            [3; 32],
            4,
            &body,
            &[pool(first), pool(second)],
        )
        .chain
    };
    // The same cards in the same order, divided between the seats differently.
    let base = opening(&[(1, 1), (2, 1)], &[(3, 1)]);
    for other in [
        opening(&[(1, 1)], &[(2, 1), (3, 1)]),
        opening(&[(1, 1), (2, 1), (3, 1)], &[]),
        opening(&[(3, 1)], &[(1, 1), (2, 1)]),
        opening(&[(1, 2), (2, 1)], &[(3, 1)]),
        opening(&[(1, 1), (2, 1)], &[(3, 256)]),
    ] {
        assert_ne!(other, base);
    }
    let longer = SetupBytes::new(vec![1, 0]).unwrap();
    let pools = [pool(&[(1, 1), (2, 1)]), pool(&[(3, 1)])];
    assert_ne!(
        Head::opening(id(1), id(2), [3; 32], 4, &longer, &pools).chain,
        base
    );
    assert_ne!(
        Head::opening(id(5), id(2), [3; 32], 4, &body, &pools).chain,
        base
    );
}
