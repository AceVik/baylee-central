//! Admission controls use synthetic packages; they do not certify an engine solution.

use super::*;

fn words(text: &str) -> Words {
    Words {
        en: text.into(),
        de: text.into(),
    }
}

fn publication() -> Publication {
    let mut value = Publication {
        description: Description {
            period: Period::containing(Kind::Daily, 1_794_182_400).unwrap(),
            title: words("Synthetic task"),
            instructions: words("Win with the supplied deck."),
            deck: "60 Synthetic card".into(),
            executor: "synthetic-build".into(),
            target_turn: 5,
            expected_minutes: 20,
        },
        digest: [0; 32],
        package: b"synthetic-private-package".to_vec(),
        certificate: Certificate {
            validator: "synthetic-v1".into(),
            witnesses: (1..=3)
                .map(|value| Witness {
                    record_digest: [value; 32],
                    strategy_digest: [value + 10; 32],
                    family: if value == 1 {
                        SolutionFamily::Combat
                    } else {
                        SolutionFamily::DirectDamage
                    },
                })
                .collect(),
        },
        hints: [words("First hint"), words("Second hint")],
    };
    value.digest = value.calculated_digest().unwrap();
    value
}

#[test]
fn an_integrity_digest_binds_every_part_of_the_publication() {
    let original = publication();
    original.validate().unwrap();
    let encoded = crate::encode(&original).unwrap();
    let decoded: Publication = crate::decode(&encoded).unwrap();
    decoded.validate().unwrap();
    let mutations: [fn(&mut Publication); 5] = [
        |p| p.description.target_turn += 1,
        |p| p.package.push(1),
        |p| p.hints[0].de.push('!'),
        |p| p.certificate.validator.push('2'),
        |p| p.certificate.witnesses[0].strategy_digest[0] ^= 1,
    ];
    for mutate in mutations {
        let mut changed = original.clone();
        mutate(&mut changed);
        assert!(
            changed.validate().is_err(),
            "changed content retained its old digest"
        );
    }
}

#[test]
fn duplicate_actions_or_a_single_family_are_not_multiple_solutions() {
    let mut duplicate = publication();
    duplicate.certificate.witnesses[1].strategy_digest =
        duplicate.certificate.witnesses[0].strategy_digest;
    duplicate.digest = duplicate.calculated_digest().unwrap();
    assert!(duplicate.validate().is_err());
    let mut family = publication();
    for witness in &mut family.certificate.witnesses {
        witness.family = SolutionFamily::Combat;
    }
    family.digest = family.calculated_digest().unwrap();
    assert!(family.validate().is_err());
    let mut two = publication();
    two.certificate.witnesses.pop();
    two.digest = two.calculated_digest().unwrap();
    assert!(two.validate().is_err());
}

#[test]
fn bounds_and_unsafe_text_are_rejected_even_with_a_matching_digest() {
    let mutations: [fn(&mut Publication); 5] = [
        |p| p.description.title.en = "invisible\u{202E}text".into(),
        |p| p.description.expected_minutes = 0,
        |p| p.description.period.ends_at += 1,
        |p| p.package.resize(MAX_PACKAGE_BYTES + 1, 0),
        |p| p.certificate.witnesses[0].record_digest = [0; 32],
    ];
    for mutate in mutations {
        let mut changed = publication();
        mutate(&mut changed);
        changed.digest = changed.calculated_digest().unwrap();
        assert!(changed.validate().is_err());
    }
}

#[test]
fn public_responses_cannot_include_execution_material() {
    let private = publication();
    let public = Challenge {
        id: id(1),
        description: private.description,
        digest: private.digest,
        withdrawn: false,
    };
    let json = serde_json::to_value(Response::Detail {
        now: 1_794_182_400,
        challenge: public,
        standings: vec![],
    })
    .unwrap();
    let challenge = json.get("challenge").unwrap().as_object().unwrap();
    for forbidden in ["package", "seed", "witnesses", "hints", "account_id"] {
        assert!(!challenge.contains_key(forbidden));
    }
    let mut forged = json;
    forged
        .as_object_mut()
        .unwrap()
        .insert("package".into(), "secret".into());
    assert!(serde_json::from_value::<Response>(forged).is_err());
}

fn id(tail: u8) -> [u8; 16] {
    let mut id = [0; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id[15] = tail;
    id
}

#[test]
fn a_partial_game_is_not_a_completion_and_time_is_bounded() {
    let base = Completion {
        attempt: id(1),
        game: id(2),
        state: AttemptState::Won,
        turn: 5,
        finished_at: 1_794_182_500,
        milliseconds: Some(40_000),
        record_digest: [1; 32],
    };
    base.validate().unwrap();
    for state in [AttemptState::Reserved, AttemptState::Playing] {
        assert!(
            Completion {
                state,
                ..base.clone()
            }
            .validate()
            .is_err()
        );
    }
    for milliseconds in [Some(0), Some(u64::MAX)] {
        assert!(
            Completion {
                milliseconds,
                ..base.clone()
            }
            .validate()
            .is_err()
        );
    }
    assert!(
        Completion {
            turn: 0,
            ..base.clone()
        }
        .validate()
        .is_err()
    );
    Completion {
        state: AttemptState::Void,
        turn: 0,
        milliseconds: None,
        ..base
    }
    .validate()
    .unwrap();
}

#[test]
fn a_description_is_bounded_in_every_field() {
    type Change = fn(&mut Description);
    let valid = |change: Change| {
        let mut description = publication().description;
        change(&mut description);
        description.validate().is_ok()
    };
    let cases: [(&str, Change, Change); 9] = [
        (
            "title",
            |d| d.title.en = "x".repeat(100),
            |d| d.title.en = "x".repeat(101),
        ),
        (
            "title in octets",
            |d| d.title.de = "ä".repeat(50),
            |d| d.title.de = "ä".repeat(51),
        ),
        (
            "instructions",
            |d| d.instructions.de = "x".repeat(2_000),
            |d| d.instructions.de = "x".repeat(2_001),
        ),
        (
            "deck",
            |d| d.deck = "x".repeat(16_000),
            |d| d.deck = "x".repeat(16_001),
        ),
        (
            "deck lines",
            |d| d.deck = "60 A\n1 B".into(),
            |d| d.deck = " \n\n ".into(),
        ),
        (
            "executor",
            |d| d.executor = "x".repeat(160),
            |d| d.executor = "x".repeat(161),
        ),
        ("first turn", |d| d.target_turn = 1, |d| d.target_turn = 0),
        (
            "last turn",
            |d| d.target_turn = 500,
            |d| d.target_turn = 501,
        ),
        (
            "minutes",
            |d| d.expected_minutes = 180,
            |d| d.expected_minutes = 181,
        ),
    ];
    for (what, inside, outside) in cases {
        assert!(valid(inside), "{what}");
        assert!(!valid(outside), "{what}");
    }
    let refused: [(&str, Change); 8] = [
        ("one language is missing", |d| d.title.de = String::new()),
        ("blank", |d| d.title.en = "\u{a0} ".into()),
        ("executor of two lines", |d| d.executor = "a\nb".into()),
        ("return", |d| d.deck = "60 A\r\n1 B".into()),
        ("tab", |d| d.instructions.en = "a\tb".into()),
        ("escape", |d| d.deck = "60 \u{1b}[2J".into()),
        ("mark", |d| d.title.en = "a\u{200f}b".into()),
        ("isolate", |d| d.executor = "a\u{2066}b".into()),
    ];
    for (what, change) in refused {
        assert!(!valid(change), "{what}");
    }
}

#[test]
fn a_certificate_needs_three_to_twelve_different_witnesses() {
    type Change = fn(&mut Publication);
    let with = |count: u8| {
        let mut value = publication();
        value.certificate.witnesses = (1..=count)
            .map(|n| Witness {
                record_digest: [n; 32],
                strategy_digest: [n + 100; 32],
                family: if n == 1 {
                    SolutionFamily::Combat
                } else {
                    SolutionFamily::DirectDamage
                },
            })
            .collect();
        value
    };
    let sealed = |mut value: Publication| {
        value.digest = value.calculated_digest().unwrap();
        value.validate().is_ok()
    };
    for (count, accepted) in [(0, false), (2, false), (3, true), (12, true), (13, false)] {
        assert_eq!(sealed(with(count)), accepted, "{count}");
    }
    let refused: [(&str, Change); 7] = [
        ("the same record", |p| {
            p.certificate.witnesses[2].record_digest = p.certificate.witnesses[0].record_digest;
        }),
        ("no strategy", |p| {
            p.certificate.witnesses[2].strategy_digest = [0; 32];
        }),
        ("no package", |p| p.package.clear()),
        ("no validator", |p| p.certificate.validator.clear()),
        ("a long validator", |p| {
            p.certificate.validator = "x".repeat(101);
        }),
        ("an empty hint", |p| p.hints[1].en.clear()),
        ("a long hint", |p| p.hints[0].de = "x".repeat(2_001)),
    ];
    for (what, change) in refused {
        let mut value = with(3);
        change(&mut value);
        assert!(!sealed(value), "{what}");
    }
    let mut most = with(3);
    most.package.resize(MAX_PACKAGE_BYTES, 7);
    assert!(sealed(most));
}

#[test]
fn a_completion_is_bounded_at_each_edge() {
    type Change = fn(&mut Completion);
    let valid = |change: Change| {
        let mut completion = Completion {
            attempt: id(1),
            game: id(2),
            state: AttemptState::Won,
            turn: 5,
            finished_at: 1_794_182_500,
            milliseconds: None,
            record_digest: [1; 32],
        };
        change(&mut completion);
        completion.validate().is_ok()
    };
    let cases: [(&str, Change, Change); 5] = [
        ("turn", |c| c.turn = 10_000, |c| c.turn = 10_001),
        ("first second", |c| c.finished_at = 1, |c| c.finished_at = 0),
        (
            "last second",
            |c| c.finished_at = 253_402_300_799,
            |c| c.finished_at = 253_402_300_800,
        ),
        (
            "a week of play",
            |c| c.milliseconds = Some(604_800_000),
            |c| c.milliseconds = Some(604_800_001),
        ),
        // A loss needs no record; a win is nothing without one.
        (
            "record",
            |c| {
                c.state = AttemptState::Lost;
                c.record_digest = [0; 32];
            },
            |c| c.record_digest = [0; 32],
        ),
    ];
    for (what, inside, outside) in cases {
        assert!(valid(inside), "{what}");
        assert!(!valid(outside), "{what}");
    }
    let refused: [(&str, Change); 7] = [
        ("before the epoch", |c| c.finished_at = -1),
        ("another version", |c| c.attempt[6] = 0x40),
        ("another variant", |c| c.attempt[8] = 0xc0),
        ("no game", |c| c.game = [0; 16]),
        ("no turn lost", |c| {
            c.state = AttemptState::Lost;
            c.turn = 0;
        }),
        ("no turn abandoned", |c| {
            c.state = AttemptState::Abandoned;
            c.turn = 0;
        }),
        ("an overflowing turn", |c| c.turn = u32::MAX),
    ];
    for (what, change) in refused {
        assert!(!valid(change), "{what}");
    }
}
