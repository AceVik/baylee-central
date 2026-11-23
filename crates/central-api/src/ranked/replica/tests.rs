use super::*;
use crate::{decode, encode};

fn id(n: u8) -> Id {
    let mut id = [n; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id
}

#[test]
fn replay_commitments_bind_the_exact_opening_seed_pools_actor_and_input() {
    let body = SetupBytes::new(vec![12, 34]).unwrap();
    let command = CommandBytes::new(vec![56, 78]).unwrap();
    let pool = vec![PoolEntry {
        card: [7; 32],
        copies: 2,
    }];
    let pools = [
        pool.clone(),
        vec![PoolEntry {
            card: [8; 32],
            copies: 1,
        }],
    ];
    let base = Head::opening(id(1), id(2), [9; 32], 15, &body, &pools);
    for other in [
        Head::opening(id(1), id(3), [9; 32], 15, &body, &pools),
        Head::opening(id(1), id(2), [8; 32], 15, &body, &pools),
        Head::opening(id(1), id(2), [9; 32], 16, &body, &pools),
        Head::opening(id(1), id(2), [9; 32], 15, &body, &[pool.clone(), pool]),
    ] {
        assert_ne!(other, base);
    }
    let first = base.next(id(1), id(4), Source::Seat0, &command).unwrap();
    assert_eq!(
        first,
        base.next(id(1), id(4), Source::Seat0, &command).unwrap()
    );
    assert_eq!(first.position, 1);
    for other in [
        base.next(id(1), id(4), Source::Seat1, &command),
        base.next(id(1), id(4), Source::Authority, &command),
        base.next(id(1), id(6), Source::Seat0, &command),
        base.next(id(2), id(4), Source::Seat0, &command),
        first.next(id(1), id(4), Source::Seat0, &command),
        base.next(
            id(1),
            id(4),
            Source::Seat0,
            &CommandBytes::new(vec![0]).unwrap(),
        ),
    ] {
        assert_ne!(other.unwrap().chain, first.chain);
    }
    assert!(
        Head {
            position: MAX_POSITION,
            ..base
        }
        .next(id(1), id(4), Source::Seat0, &command)
        .is_err()
    );
}

#[test]
fn private_hex_is_canonical_bounded_and_redacted_at_every_nesting_level() {
    let bytes = CommandBytes::new(vec![0xab, 0xcd]).unwrap();
    assert_eq!(encode(&bytes).unwrap(), br#""abcd""#);
    assert_eq!(decode::<CommandBytes>(br#""abcd""#).unwrap(), bytes);
    assert_eq!(
        serde_json::from_value::<CommandBytes>(serde_json::json!("abcd")).unwrap(),
        bytes
    );
    for bad in [
        br#""""#.as_slice(),
        br#""a""#,
        br#""AB""#,
        "\"ü\"".as_bytes(),
        br#"" 3""#,
    ] {
        assert!(decode::<CommandBytes>(bad).is_err());
    }
    assert!(CommandBytes::new(vec![0; 32 * 1024 + 1]).is_err());
    let large = SetupBytes::new(vec![0xff; 192 * 1024]).unwrap();
    assert_eq!(
        decode::<SetupBytes>(&encode(&large).unwrap()).unwrap(),
        large
    );
    let host = HostResponse::Page {
        opening: Head {
            position: 0,
            chain: [0; 32],
        },
        setup: Some(large),
        pools: None,
        initial: None,
        entries: vec![],
        certified: Head {
            position: 0,
            chain: [0; 32],
        },
    };
    assert!(format!("{host:?}").ends_with("([REDACTED])"));
    assert!(decode::<SeatResponse>(&encode(&host).unwrap()).is_err());
    let secret = relay::Secret::new("a".repeat(64)).unwrap();
    let body = SeatRequest::Relay {
        fence: Fence {
            match_id: id(1),
            generation: 1,
        },
        call: relay::Call::Reclaim {
            bearer: secret,
            game: id(3),
        },
    };
    assert!(!format!("{body:?}").contains(&"a".repeat(64)));
}

#[test]
fn consent_binds_operators_and_disallowed_seat_and_relay_shapes_are_rejected() {
    let host = HostApproval {
        epoch: 0,
        host_id: id(2),
        origin: "https://play.example".into(),
        terms: None,
    };
    let fence = Fence {
        match_id: id(1),
        generation: 1,
    };
    assert!(
        SeatRequest::ApproveHosts {
            fence,
            hosts: vec![host.clone()]
        }
        .validate(false)
        .is_ok()
    );
    assert!(
        SeatRequest::ApproveHosts {
            fence,
            hosts: vec![host.clone(), host]
        }
        .validate(false)
        .is_err()
    );
    assert!(
        SeatRequest::Submit {
            fence,
            expected: Head {
                position: 0,
                chain: [0; 32]
            },
            command_id: [0; 16],
            body: CommandBytes::new(vec![1]).unwrap()
        }
        .validate(false)
        .is_err()
    );
    assert!(decode::<relay::Call>(br#"{"admit":{"ticket":"unexpected"}}"#).is_err());
    assert!(decode::<relay::Call>(br#"{"admit":{"origin":"https://elsewhere.example"}}"#).is_err());
    assert!(decode::<SeatRequest>(br#"{"read":{"after":0}}"#).is_err());
    assert!(
        !crate::ranked::Request::Replicate {
            request: SeatRequest::Status { match_id: id(1) }
        }
        .is_public()
    );
}
