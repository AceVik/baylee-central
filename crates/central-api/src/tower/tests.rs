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
