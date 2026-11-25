use super::*;
use crate::{
    Error,
    challenges::{AttemptState, Kind},
};

#[test]
fn every_route_reaches_a_certifiable_node_and_exactly_five_floors() {
    let nodes = coordinates();
    assert_eq!(nodes.len(), NODES);
    let mut reached = std::collections::BTreeSet::new();
    for route in 0_u32..81 {
        let mut digits = route;
        let mut progress = Progress::new(true);
        for floor in 1..=FLOORS {
            reached.insert(progress.node().unwrap());
            assert_eq!(
                nodes[usize::from(progress.node().unwrap())]
                    .iter()
                    .sum::<u8>(),
                floor - 1
            );
            progress.start().unwrap();
            progress
                .finish(AttemptState::Won, 10, Some(60_000), true)
                .unwrap();
            assert_eq!(progress.cleared, floor);
            if floor < FLOORS {
                assert_eq!(progress.phase, Phase::Choosing);
                assert_eq!(progress.start(), Err(Error::Conflict));
                progress
                    .choose(Track::ALL[usize::try_from(digits % 3).unwrap()])
                    .unwrap();
                digits /= 3;
            }
        }
        assert_eq!(progress.phase, Phase::Conquered);
        assert_eq!(progress.turns, 50);
        assert_eq!(progress.milliseconds, Some(300_000));
        assert!(progress.competitive);
        assert_eq!(progress.start(), Err(Error::Conflict));
    }
    assert_eq!(reached.len(), NODES);
}

#[test]
fn defeat_spends_exactly_one_life_and_void_preserves_the_encounter() {
    let mut progress = Progress::new(true);
    progress.start().unwrap();
    progress.finish(AttemptState::Void, 0, None, false).unwrap();
    assert_eq!(progress, Progress::new(true));
    for remaining in (0..3).rev() {
        let node = progress.node().unwrap();
        progress.start().unwrap();
        progress
            .finish(AttemptState::Lost, 9, Some(123), true)
            .unwrap();
        assert_eq!(progress.lives, remaining);
        assert_eq!(progress.node().unwrap(), node);
        let previous = progress.clone();
        assert_eq!(
            progress.finish(AttemptState::Lost, 9, Some(123), true),
            Err(Error::Conflict)
        );
        assert_eq!(progress, previous);
    }
    assert_eq!(progress.phase, Phase::Exhausted);
    assert_eq!(progress.start(), Err(Error::Conflict));
}

#[test]
fn interruption_and_rollover_keep_progress_but_cannot_mint_a_sprint() {
    let mut progress = Progress::new(true);
    progress.start().unwrap();
    progress.finish(AttemptState::Won, 15, None, false).unwrap();
    assert!(!progress.competitive);
    assert_eq!(progress.milliseconds, None);
    assert_eq!(progress.cleared, 1);
    progress.choose(Track::Precision).unwrap();
    progress.start().unwrap();
    progress
        .finish(AttemptState::Won, 10, Some(500), true)
        .unwrap();
    assert_eq!(progress.milliseconds, None);
    assert!(!progress.competitive);
}

#[test]
fn invalid_scores_and_choices_leave_the_run_unchanged() {
    let mut progress = Progress::new(true);
    assert_eq!(progress.choose(Track::Assault), Err(Error::Conflict));
    progress.start().unwrap();
    for (state, turn, ms) in [
        (AttemptState::Won, 0, Some(1)),
        (AttemptState::Won, 10_001, Some(1)),
        (AttemptState::Won, 1, Some(0)),
        (AttemptState::Playing, 1, Some(1)),
        (AttemptState::Won, 1, Some(604_800_001)),
    ] {
        let previous = progress.clone();
        assert_eq!(progress.finish(state, turn, ms, true), Err(Error::Rejected));
        assert_eq!(progress, previous);
    }
    progress.route = vec![Track::Assault; 5];
    assert_eq!(progress.node(), Err(Error::Rejected));
}

pub(super) fn theme() -> Theme {
    let words = |text: &str| Words {
        en: text.into(),
        de: text.into(),
    };
    Theme {
        period: Period::containing(Kind::Weekly, 1_794_182_400).unwrap(),
        title: words("The emerald ascent"),
        floors: std::array::from_fn(|i| words(&format!("Floor {}", i + 1))),
        upgrades: std::array::from_fn(|_| {
            std::array::from_fn(|_| Upgrade {
                title: words("Kindled resolve"),
                summary: words("Strengthen the supplied deck."),
                copies: 2,
                remove: "Shock".into(),
                add: "Lightning Bolt".into(),
            })
        }),
    }
}

#[test]
fn missing_duplicate_and_unsafe_manifest_content_fail_closed() {
    let mut manifest = Manifest {
        theme: theme(),
        nodes: (1..=35).map(|i| [i; 32]).collect(),
    };
    manifest.validate().unwrap();
    manifest.nodes[1] = manifest.nodes[0];
    assert_eq!(manifest.validate(), Err(Error::Rejected));
    manifest.nodes[1] = [2; 32];
    manifest.nodes.pop();
    assert_eq!(manifest.validate(), Err(Error::Rejected));
    manifest.nodes.push([35; 32]);
    manifest.theme.upgrades[0][0].add = "forged\ncard".into();
    assert_eq!(manifest.validate(), Err(Error::Rejected));
    manifest.theme = theme();
    manifest.theme.period = Period::containing(Kind::Daily, 1_794_182_400).unwrap();
    assert_eq!(manifest.validate(), Err(Error::Rejected));
}

#[test]
fn tower_rewards_are_distinct_permanent_tracks_with_five_tiers() {
    for kind in BadgeKind::ALL {
        assert_eq!(kind.progress(0).tier, None);
        for target in kind.thresholds() {
            assert_eq!(kind.progress(target - 1).next, Some(target));
            assert!(kind.progress(target).tier.is_some());
        }
        assert_eq!(kind.progress(u32::MAX).next, None);
    }
}

#[test]
fn every_upgrade_of_the_theme_is_bounded_and_readable() {
    type Change = fn(&mut Upgrade);
    // The first and the last upgrade, so that no track or step is skipped.
    let valid = |change: Change| {
        [false, true].map(|last| {
            let mut theme = theme();
            let upgrade = if last {
                theme.upgrades.last_mut().unwrap().last_mut().unwrap()
            } else {
                &mut theme.upgrades[0][0]
            };
            change(upgrade);
            theme.validate().is_ok()
        })
    };
    let cases: [(&str, Change, Change); 6] = [
        ("copies", |u| u.copies = 4, |u| u.copies = 5),
        ("no copies", |u| u.copies = 1, |u| u.copies = 0),
        (
            "added name",
            |u| u.add = "x".repeat(200),
            |u| u.add = "x".repeat(201),
        ),
        (
            "removed name",
            |u| u.remove = "x".repeat(200),
            |u| u.remove = "x".repeat(201),
        ),
        (
            "title",
            |u| u.title.en = "x".repeat(100),
            |u| u.title.en = "x".repeat(101),
        ),
        (
            "summary",
            |u| u.summary.de = "x".repeat(400),
            |u| u.summary.de = "x".repeat(401),
        ),
    ];
    for (what, inside, outside) in cases {
        assert_eq!(valid(inside), [true; 2], "{what}");
        assert_eq!(valid(outside), [false; 2], "{what}");
    }
    let refused: [(&str, Change); 5] = [
        ("a card for itself", |u| u.add.clone_from(&u.remove)),
        ("no card added", |u| u.add = "  ".into()),
        ("no card removed", |u| u.remove.clear()),
        ("an override", |u| u.remove = "Sho\u{202e}ck".into()),
        ("a line break", |u| u.remove = "Shock\n4 Bolt".into()),
    ];
    for (what, change) in refused {
        assert_eq!(valid(change), [false; 2], "{what}");
    }
    let mut floor = theme();
    floor.floors.last_mut().unwrap().de = "x".repeat(101);
    assert!(floor.validate().is_err());
    let mut title = theme();
    title.title.en.clear();
    assert!(title.validate().is_err());
}

#[test]
fn a_manifest_holds_each_node_once_and_none_of_zeros() {
    let nodes = |count: u8| (1..=count).map(|i| [i; 32]).collect::<Vec<_>>();
    let valid = |nodes: Vec<[u8; 32]>| {
        Manifest {
            theme: theme(),
            nodes,
        }
        .validate()
        .is_ok()
    };
    let all = u8::try_from(NODES).unwrap();
    assert!(valid(nodes(all)));
    assert!(!valid(nodes(all - 1)));
    assert!(!valid(nodes(all + 1)));
    assert!(!valid(vec![]));
    let mut zero = nodes(all);
    zero[usize::from(all) - 1] = [0; 32];
    assert!(!valid(zero));
    // The same node again, far from its first place.
    let mut twice = nodes(all);
    twice[usize::from(all) - 1] = [1; 32];
    assert!(!valid(twice));
}

/// A synthetic encounter of `period`, whose digest is its content's.
fn encounter(period: Period) -> crate::challenges::Publication {
    use crate::challenges::{Certificate, Description, Publication, SolutionFamily, Witness};
    let words = |text: &str| Words {
        en: text.into(),
        de: text.into(),
    };
    let mut value = Publication {
        description: Description {
            period,
            title: words("Synthetic encounter"),
            instructions: words("Win the synthetic encounter."),
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
                .map(|n| Witness {
                    record_digest: [n; 32],
                    strategy_digest: [n + 10; 32],
                    family: if n == 1 {
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
fn an_encounter_stands_only_at_its_own_node_and_in_its_own_week() {
    const NODE: u8 = 7;
    let week = 1_794_182_400;
    let manifest = |digest: [u8; 32]| {
        let mut nodes: Vec<[u8; 32]> = (1..=u8::try_from(NODES).unwrap())
            .map(|i| [i; 32])
            .collect();
        nodes[usize::from(NODE)] = digest;
        Manifest {
            theme: theme(),
            nodes,
        }
    };
    let good = encounter(theme().period);
    let placed = manifest(good.digest);
    assert_eq!(placed.validate_node(NODE, &good), Ok(()));
    for node in (0..=u8::MAX).filter(|&node| node != NODE) {
        assert!(placed.validate_node(node, &good).is_err(), "node {node}");
    }

    // Content changed under the digest the manifest holds, or the digest with it.
    let mut changed = good.clone();
    changed.package.push(0);
    assert!(placed.validate_node(NODE, &changed).is_err());
    changed.digest = changed.calculated_digest().unwrap();
    assert!(placed.validate_node(NODE, &changed).is_err());
    assert!(
        manifest(changed.digest)
            .validate_node(NODE, &changed)
            .is_ok()
    );

    // Another week's encounter, and a daily one, do not become this week's by their digest.
    for period in [
        Period::containing(Kind::Weekly, week + 7 * 86_400).unwrap(),
        Period::containing(Kind::Weekly, week - 7 * 86_400).unwrap(),
        Period::containing(Kind::Daily, week).unwrap(),
    ] {
        let other = encounter(period);
        assert!(other.validate().is_ok());
        let claimed = manifest(other.digest);
        assert!(claimed.validate().is_ok());
        assert!(claimed.validate_node(NODE, &other).is_err(), "{period:?}");
    }

    // A manifest that is none places nothing, at no node.
    let mut twice = placed.clone();
    twice.nodes[0] = twice.nodes[1];
    assert!(twice.validate_node(NODE, &good).is_err());
    let mut unthemed = placed;
    unthemed.theme.title.en.clear();
    assert!(unthemed.validate_node(NODE, &good).is_err());
    // Nor does an encounter without its evidence stand anywhere.
    let mut unproved = good;
    unproved.certificate.witnesses.pop();
    unproved.digest = unproved.calculated_digest().unwrap();
    assert!(
        manifest(unproved.digest)
            .validate_node(NODE, &unproved)
            .is_err()
    );
}
