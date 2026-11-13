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
