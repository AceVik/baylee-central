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

/// Every move the wire can carry, in slots that exist and slots that do not.
fn every_move() -> Vec<Move> {
    const SPELLS: [Spell; 8] = [
        Spell::Might,
        Spell::Spark,
        Spell::Bolt,
        Spell::Frost,
        Spell::Wings,
        Spell::Edge,
        Spell::Rally,
        Spell::Mend,
    ];
    const SLOTS: [u8; 5] = [0, 1, 2, 3, u8::MAX];
    let mut targets = vec![None];
    for side in [Side::You, Side::Opponent] {
        targets.push(Some(Target::Player(side)));
        targets.extend(SLOTS.map(|slot| Some(Target::Unit { side, slot })));
    }
    let mut moves = vec![Move::Pass];
    for spell in SPELLS {
        moves.extend(targets.iter().map(|&target| Move::Cast { spell, target }));
    }
    moves.extend((0..=u8::MAX).map(|attackers| Move::Attack { attackers }));
    let slots = || [None].into_iter().chain(SLOTS.map(Some));
    for a in slots() {
        for b in slots() {
            for c in slots() {
                moves.push(Move::Block {
                    assignments: [a, b, c],
                });
            }
        }
    }
    moves
}

/// Walks the whole tree and returns how many unfinished positions it has.
fn offered_is_exactly_what_is_accepted(p: &Puzzle, line: &Line, moves: &[Move]) -> usize {
    let offered = p.choices(line).unwrap();
    let finished = p.position(line).unwrap().result.is_some();
    assert_eq!(offered.is_empty(), finished);
    for &choice in moves {
        let mut next = line.clone();
        next.0.push(choice);
        assert_eq!(
            p.position(&next).is_ok(),
            offered.contains(&choice),
            "{choice:?} after {line:?}"
        );
    }
    for (i, choice) in offered.iter().enumerate() {
        assert!(!offered[..i].contains(choice), "{choice:?} offered twice");
    }
    let mut nodes = usize::from(!finished);
    for choice in offered {
        let mut next = line.clone();
        next.0.push(choice);
        nodes += offered_is_exactly_what_is_accepted(p, &next, moves);
    }
    nodes
}

#[test]
fn a_submitted_move_is_accepted_exactly_when_it_is_offered() {
    let moves = every_move();
    let mut spells = duel(
        vec![unit(2, 2, &[]), unit(1, 1, &[Keyword::Flying])],
        vec![unit(3, 1, &[Keyword::Reach])],
        Phase::Cast,
    );
    spells.hand = vec![Spell::Spark, Spell::Spark, Spell::Rally, Spell::Frost];
    spells.mana = 2;
    spells.steps.push(Step {
        phase: Phase::Attack {
            guards: [Some(1), None, None],
        },
        new_turn: None,
        draw: Some(Spell::Mend),
    });
    // Two casts of the one spell that is twice in hand, both attackers, and a kill.
    assert!(offered_is_exactly_what_is_accepted(&spells, &Line::default(), &moves) > 8);

    let mut tapped = unit(5, 5, &[]);
    tapped.tapped = true;
    let mut blocks = duel(
        vec![unit(1, 1, &[]), tapped, unit(0, 4, &[Keyword::Reach])],
        vec![unit(2, 2, &[Keyword::Flying]), unit(2, 2, &[])],
        Phase::Block { attackers: 0b11 },
    );
    blocks.steps.push(Step {
        new_turn: Some(NewTurn {
            active: Side::You,
            mana: 1,
        }),
        draw: Some(Spell::Wings),
        phase: Phase::Cast,
    });
    blocks.steps.push(Step {
        new_turn: None,
        draw: None,
        phase: Phase::Attack {
            guards: [Some(0), Some(2), None],
        },
    });
    assert!(offered_is_exactly_what_is_accepted(&blocks, &Line::default(), &moves) > 20);
}

#[test]
fn a_line_stops_where_the_duel_is_decided_and_at_three_moves() {
    let mut p = duel(vec![unit(1, 1, &[])], vec![], Phase::Cast);
    p.hand = vec![Spell::Bolt];
    p.life[1] = 3;
    p.steps = vec![p.steps[0]; 3];
    let bolt = Move::Cast {
        spell: Spell::Bolt,
        target: Some(Target::Player(Side::Opponent)),
    };
    assert!(p.won(&Line(vec![bolt])).unwrap());
    assert_eq!(p.choices(&Line(vec![bolt])).unwrap(), []);
    // A won duel is over: nothing follows it, not even a pass.
    assert!(p.won(&Line(vec![bolt, Move::Pass])).is_err());
    assert!(p.won(&Line(vec![bolt, bolt])).is_err());
    assert!(!p.won(&Line(vec![Move::Pass; 3])).unwrap());
    assert!(p.won(&Line(vec![Move::Pass; 4])).is_err());
    // Striking oneself down is a loss, whatever the opponent's life.
    p.life = [3, 3];
    let own = Move::Cast {
        spell: Spell::Bolt,
        target: Some(Target::Player(Side::You)),
    };
    let end = p.position(&Line(vec![own])).unwrap();
    assert_eq!(end.result, Some(false));
    for wire in [
        r#"["pass","pass","pass"]"#,
        r#"[{"attack":{"attackers":255}}]"#,
        "[]",
    ] {
        assert!(crate::decode::<Line>(wire.as_bytes()).is_ok(), "{wire}");
    }
    for wire in [
        r#"["pass","pass","pass","pass"]"#,
        r#"[{"attack":{"attackers":256}}]"#,
        r#"[{"attack":{"attackers":-1}}]"#,
        r#"[{"block":{"assignments":[0,1]}}]"#,
        r#"[{"block":{"assignments":[0,1,2,3]}}]"#,
        r#"[{"cast":{"spell":"lightning","target":null}}]"#,
        r#"[{"cast":{"spell":"bolt","target":{"player":"judge"}}}]"#,
        r#"[{"attack":{"attackers":1,"extra":1}}]"#,
        r#"["concede"]"#,
        r#"{"0":"pass"}"#,
        "null",
    ] {
        assert!(crate::decode::<Line>(wire.as_bytes()).is_err(), "{wire}");
    }
}

#[test]
fn a_puzzle_is_admitted_to_each_bound_and_not_one_past_it() {
    type Change = fn(&mut Puzzle);
    let base = || {
        duel(
            vec![unit(1, 1, &[]), unit(1, 1, &[])],
            vec![unit(1, 1, &[]), unit(1, 1, &[])],
            Phase::Cast,
        )
    };
    let valid = |change: Change| {
        let mut puzzle = base();
        change(&mut puzzle);
        puzzle.validate().is_ok()
    };
    let turn = |mana| {
        Some(NewTurn {
            active: Side::You,
            mana,
        })
    };
    assert!(base().validate().is_ok());
    let bounds: [(&str, Change, Change); 11] = [
        (
            "steps",
            |p| p.steps = vec![p.steps[0]; 3],
            |p| p.steps = vec![p.steps[0]; 4],
        ),
        ("no steps", |_| {}, |p| p.steps.clear()),
        ("life", |p| p.life = [40, 40], |p| p.life[1] = 41),
        ("no life", |p| p.life = [1, 1], |p| p.life[0] = 0),
        ("mana", |p| p.mana = 6, |p| p.mana = 7),
        (
            "hand",
            |p| p.hand = vec![Spell::Bolt; 5],
            |p| p.hand = vec![Spell::Bolt; 6],
        ),
        (
            "power",
            |p| p.units[0][1].power = 12,
            |p| p.units[0][1].power = 13,
        ),
        (
            "toughness",
            |p| p.units[1][1].toughness = 12,
            |p| p.units[1][1].toughness = 13,
        ),
        (
            "no toughness",
            |p| p.units[1][1].toughness = 1,
            |p| p.units[1][1].toughness = 0,
        ),
        (
            "units",
            |p| p.units[1].push(p.units[1][0].clone()),
            |p| p.units[1] = vec![p.units[1][0].clone(); 4],
        ),
        (
            "keywords",
            |p| p.units[0][0].keywords = vec![Keyword::Flying, Keyword::Reach],
            |p| p.units[0][0].keywords = vec![Keyword::Flying, Keyword::Reach, Keyword::Flying],
        ),
    ];
    for (what, inside, outside) in bounds {
        assert!(valid(inside), "{what}");
        assert!(!valid(outside), "{what}");
    }
    // A card drawn later counts as one in hand.
    let mut drawn = base();
    drawn.hand = vec![Spell::Bolt; 4];
    drawn.steps[0].draw = Some(Spell::Mend);
    assert!(drawn.validate().is_ok());
    drawn.hand.push(Spell::Bolt);
    assert!(drawn.validate().is_err());
    // Mana of a later turn has the same limit as the opening's.
    for (mana, accepted) in [(6, true), (7, false)] {
        let mut later = base();
        later.steps[0].new_turn = turn(mana);
        assert_eq!(later.validate().is_ok(), accepted, "{mana}");
    }
    // The same side has no two turns in a row.
    let mut twice = base();
    twice.steps = vec![
        Step {
            new_turn: turn(1),
            ..twice.steps[0]
        };
        2
    ];
    assert!(twice.validate().is_err());
}

#[test]
fn a_script_names_only_creatures_that_exist_in_the_turn_of_their_side() {
    let one = || {
        duel(
            vec![unit(1, 1, &[]), unit(1, 1, &[])],
            vec![unit(1, 1, &[])],
            Phase::Cast,
        )
    };
    let phase = |phase| {
        let mut puzzle = one();
        puzzle.steps[0].phase = phase;
        puzzle.validate().is_ok()
    };
    for (attackers, accepted) in [(0, false), (1, true), (2, false), (3, false), (255, false)] {
        assert_eq!(phase(Phase::Block { attackers }), accepted, "{attackers}");
    }
    for (guards, accepted) in [
        ([None; 3], true),
        ([Some(0), None, None], true),
        ([Some(1), None, None], true),
        // A target slot without a creature, and a guard that does not exist.
        ([Some(2), None, None], false),
        ([Some(255), None, None], false),
        ([None, Some(0), None], false),
        ([None, None, Some(0)], false),
    ] {
        assert_eq!(phase(Phase::Attack { guards }), accepted, "{guards:?}");
    }
    // Nobody attacks in the other's turn, and an empty board attacks with nobody.
    for (active, block, attack) in [(Side::You, false, true), (Side::Opponent, true, false)] {
        for (phase, accepted) in [
            (Phase::Block { attackers: 1 }, block),
            (Phase::Attack { guards: [None; 3] }, attack),
        ] {
            let mut puzzle = one();
            puzzle.steps[0] = Step {
                new_turn: Some(NewTurn { active, mana: 0 }),
                draw: None,
                phase,
            };
            assert_eq!(puzzle.validate().is_ok(), accepted, "{active:?} {phase:?}");
        }
    }
    let mut empty = one();
    empty.units[1].clear();
    empty.steps[0].phase = Phase::Block { attackers: 1 };
    assert!(empty.validate().is_err());
}
