use super::*;

fn unit(power: u8, toughness: u8, keys: &[Keyword]) -> Unit {
    Unit {
        power,
        toughness,
        keywords: keys.to_vec(),
        tapped: false,
    }
}
fn duel(mine: Vec<Unit>, theirs: Vec<Unit>, phase: Phase) -> Puzzle {
    Puzzle {
        goal: Goal::Defeat,
        life: [15, 15],
        mana: 2,
        units: [mine, theirs],
        hand: Vec::new(),
        steps: vec![Step {
            phase,
            new_turn: None,
            draw: None,
        }],
    }
}
#[test]
fn ordinary_double_blocks_and_first_strike_deathtouch_are_simultaneous() {
    let p = duel(
        vec![unit(2, 2, &[]), unit(1, 3, &[])],
        vec![unit(4, 3, &[])],
        Phase::Block { attackers: 1 },
    );
    let line = Line(vec![Move::Block {
        assignments: [Some(0), Some(0), None],
    }]);
    let result = p.position(&line).unwrap();
    assert_eq!(result.board.life, [15, 15]);
    assert!(!result.board.units[0][0].alive);
    assert!(result.board.units[0][1].alive);
    assert_eq!(result.board.units[0][1].damage, 2);
    assert!(!result.board.units[1][0].alive);
    let mut p = p;
    p.units[0][0] = unit(1, 1, &[Keyword::Deathtouch, Keyword::FirstStrike]);
    assert!(
        p.position(&line).unwrap().board.units[0]
            .iter()
            .all(|u| u.alive)
    );
}

#[test]
fn flying_tapped_and_unassigned_ghost_slots_reject_illegal_blocks() {
    let mut p = duel(
        vec![unit(1, 4, &[]), unit(0, 3, &[Keyword::Reach])],
        vec![unit(2, 3, &[Keyword::Flying])],
        Phase::Block { attackers: 1 },
    );
    assert_eq!(p.choices(&Line::default()).unwrap().len(), 2);
    for slots in [
        [Some(0), None, None],
        [None, None, Some(0)],
        [None, Some(255), None],
    ] {
        assert!(
            p.position(&Line(vec![Move::Block { assignments: slots }]))
                .is_err()
        );
    }
    p.units[0][1].tapped = true;
    assert_eq!(p.choices(&Line::default()).unwrap().len(), 1);
}
#[test]
fn trample_assigns_lethal_in_slot_order_and_lifelink_uses_all_damage() {
    let p = duel(
        vec![unit(
            7,
            4,
            &[Keyword::Trample, Keyword::Lifelink, Keyword::Deathtouch],
        )],
        vec![unit(1, 5, &[]), unit(0, 5, &[])],
        Phase::Attack {
            guards: [Some(0), Some(0), None],
        },
    );
    let end = p
        .position(&Line(vec![Move::Attack { attackers: 1 }]))
        .unwrap();
    assert_eq!(end.board.life, [22, 10]);
    assert!(end.board.units[1].iter().all(|u| !u.alive));
}
#[test]
fn mana_targets_draws_cleanup_and_unknown_input_fail_closed() {
    let mut p = duel(vec![unit(1, 2, &[])], vec![], Phase::Cast);
    p.hand = vec![Spell::Bolt, Spell::Might];
    p.mana = 1;
    p.steps.push(Step {
        new_turn: Some(NewTurn {
            active: Side::You,
            mana: 2,
        }),
        draw: Some(Spell::Mend),
        phase: Phase::Cast,
    });
    let target = Some(Target::Unit {
        side: Side::You,
        slot: 0,
    });
    let bad = |spell, target| {
        p.position(&Line(vec![Move::Cast { spell, target }]))
            .is_err()
    };
    assert!(bad(Spell::Bolt, Some(Target::Player(Side::Opponent))));
    assert!(bad(
        Spell::Might,
        Some(Target::Unit {
            side: Side::Opponent,
            slot: 0
        })
    ));
    let line = Line(vec![Move::Cast {
        spell: Spell::Might,
        target,
    }]);
    let pos = p.position(&line).unwrap();
    assert_eq!((pos.board.units[0][0].power, pos.board.mana), (1, 2));
    assert_eq!(pos.board.hand, [Spell::Bolt, Spell::Mend]);
    assert!(crate::decode::<Line>(br#"["pass","pass","pass","pass"]"#).is_err());
}

#[test]
fn exhaustive_search_replays_all_wins_and_stops_at_a_fixed_work_budget() {
    let mut p = duel(
        vec![unit(1, 1, &[]), unit(1, 1, &[])],
        vec![],
        Phase::Attack { guards: [None; 3] },
    );
    p.life[1] = 1;
    let study = p.study(10).unwrap();
    assert_eq!(
        (study.nodes, study.wins, study.losses, study.shortest),
        (5, 3, 1, Some(1))
    );
    assert!(p.won(study.witness.as_ref().unwrap()).unwrap());
    assert!(p.study(4).is_err());
    assert!(!p.won(&Line::default()).unwrap());
    assert!(
        p.won(&Line(vec![Move::Attack { attackers: 1 }, Move::Pass]))
            .is_err()
    );
}

#[test]
fn a_script_cannot_grant_an_extra_combat_or_let_a_dead_defender_block() {
    let mut p = duel(
        vec![unit(1, 2, &[])],
        vec![unit(2, 1, &[])],
        Phase::Block { attackers: 1 },
    );
    p.steps.push(p.steps[0]);
    assert!(p.validate().is_err());
    p.steps[1] = Step {
        new_turn: Some(NewTurn {
            active: Side::You,
            mana: 0,
        }),
        draw: None,
        phase: Phase::Attack {
            guards: [Some(0), None, None],
        },
    };
    p.units[0][0].toughness = 3;
    let moves = Line(vec![
        Move::Block {
            assignments: [Some(0), None, None],
        },
        Move::Attack { attackers: 1 },
    ]);
    let end = p.position(&moves).unwrap();
    assert!(!end.board.units[1][0].alive);
    assert_eq!(end.board.life, [15, 14]);
}
