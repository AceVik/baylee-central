use super::*;

#[path = "tests/placement_study.rs"]
mod placement_study;

const EPSILON: f64 = 1.0e-8;

fn near(got: f64, expected: f64) {
    assert!((got - expected).abs() < EPSILON, "{got} vs {expected}");
}

fn established(elo: i32) -> Player {
    Player {
        score: f64::from(elo),
        placement: f64::from(elo),
        allowance: 0.0,
        games: 20,
    }
}

#[test]
fn documented_quotes_and_seat_order() {
    let even = [established(1500); 2];
    let advance = settle(even, Outcome::FirstWin, [0, 0], 0);
    near(advance[0].score, 1512.0);
    near(advance[1].score, 1488.0);
    let low = established(1300);
    let high = established(1900);
    for evidence in 0..=3_u8 {
        for q in 0..=6 {
            let weight = [1.0, 1.0, 0.5, 0.5, 0.25, 0.25, 0.25][usize::try_from(q).unwrap()];
            let next = settle([low, high], Outcome::FirstWin, [evidence, 3 - evidence], q);
            let reversed = settle([high, low], Outcome::SecondWin, [3 - evidence, evidence], q);
            near(
                next[0].score - low.score,
                weight * (20.0 + 4.0 * f64::from(evidence)),
            );
            near(
                high.score - next[1].score,
                weight * (26.0 - 4.0 * f64::from(evidence)),
            );
            near(reversed[0].score, next[1].score);
            near(reversed[1].score, next[0].score);
        }
    }
    let rookie = Player {
        games: PLACEMENT_GAMES - 1,
        ..low
    };
    near(
        settle([rookie, high], Outcome::FirstWin, [3, 3], 0)[1].score,
        1890.0,
    );
    assert_eq!(EVIDENCE_GAP, 250);
}

#[test]
fn asymmetric_corrections_keep_transfers_bounded() {
    // Exercise opposite allowances, repeated encounters, each personal streak,
    // a draw, and intervals spanning each scoring transition.
    for gap in [0, 149, 150, 151, 249, 250, 349, 350, 800] {
        for a in [-64.0, 0.0, 64.0] {
            for b in [-64.0, 0.0, 64.0] {
                for outcome in [Outcome::FirstWin, Outcome::SecondWin, Outcome::Draw] {
                    for upsets in [[0, 0], [3, 0], [0, 3], [3, 3]] {
                        let prior = [
                            Player {
                                allowance: a,
                                ..established(1500)
                            },
                            Player {
                                allowance: b,
                                ..established(1500 + gap)
                            },
                        ];
                        let next = settle(prior, outcome, upsets, 0);
                        let adjusted =
                            |ps: [Player; 2]| ps.iter().map(|p| p.score - p.allowance).sum();
                        near(adjusted(prior), adjusted(next));
                        for p in next {
                            assert!(p.allowance.abs() <= 64.0 + EPSILON);
                            assert_eq!(p.games, 21);
                        }
                        if gap >= 350 && outcome == Outcome::FirstWin {
                            assert!(next[0].score <= prior[0].score + 32.0 + EPSILON);
                            assert!(next[1].score >= prior[1].score - 26.0 - EPSILON);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn queue_calibrates_newcomers_and_draws_use_both_ratings() {
    let rookie = Player {
        games: 0,
        ..established(1500)
    };
    let peer = established(1500);
    let after = settle([rookie, peer], Outcome::FirstWin, [0; 2], 0);
    near(after[0].placement, 1528.0);
    assert_eq!(queue_elo(after[0]), 1527);
    assert_eq!(
        queue_elo(Player {
            games: 10,
            placement: 2500.0,
            ..rookie
        }),
        1620
    );
    assert_eq!(
        queue_elo(Player {
            placement: 2500.0,
            ..peer
        }),
        1500
    );
    near(
        settle([peer; 2], Outcome::Draw, [3; 2], 0)[0].score,
        peer.score,
    );
    let tied = settle([rookie, established(2000)], Outcome::Draw, [0; 2], 0);
    assert!(tied[0].score > 1500.0 && tied[1].score < 2000.0);
}

#[test]
fn divisions_apply_hysteresis_only_to_previously_held_steps() {
    let fresh = Rank::default();
    let entry = Rank {
        league: 5,
        division: 4,
    };
    let third = Rank {
        league: 5,
        division: 3,
    };
    let second = Rank {
        league: 5,
        division: 2,
    };
    assert_eq!(rank(2500, 9, 0, 0, fresh).league, -1);
    assert_eq!(rank(1400, 10, 0, 0, fresh), fresh);
    assert_eq!(rank(2500, 10, 0, 0, fresh).league, 3);
    assert_eq!(rank(2500, 239, 0, 0, fresh).league, 3);
    assert_eq!(rank(2500, 240, 0, 0, fresh).league, 4);
    assert_eq!(
        rank(1200, 20, 0, 0, fresh),
        Rank {
            league: 1,
            division: 3
        }
    );
    assert_eq!(rank(1759, 700, 0, 0, entry), third);
    assert_eq!(rank(1760, 619, 0, 0, third), third);
    assert_eq!(rank(1760, 620, 0, 0, third), second);
    assert_eq!(rank(1745, 700, 0, 0, second), second);
    assert_eq!(rank(1744, 700, 0, 0, second), third);
    assert_eq!(rank(1679, 700, 0, 0, entry).league, 4);
}

#[test]
fn elite_requires_six_months_volume_and_recent_activity() {
    let fresh = Rank::default();
    for (games, days, tier) in [
        (1999, 200, 7),
        (2000, 183, 7),
        (2000, 184, 8),
        (2500, 184, 9),
    ] {
        assert_eq!(rank(2600, games, days, 14, fresh).league, tier);
    }
    assert_eq!(rank(2600, 3000, 300, 15, fresh).league, 7);
    assert_eq!(rank(2600, 3000, 300, 0, fresh).league, 9);
    assert!(champion_eligible(
        CHAMPION_ELO,
        CHAMPION_GAMES,
        i64::from(ELITE_AGE_DAYS),
        i64::from(ELITE_IDLE_DAYS)
    ));
    for (elo, games, age, idle) in [
        (2489, 3000, 184, 14),
        (2490, 2999, 184, 14),
        (2490, 3000, 183, 14),
        (2490, 3000, 184, 15),
    ] {
        assert!(!champion_eligible(elo, games, age, idle));
    }
    assert_eq!(CHAMPION_LIMIT, 100);
}

#[test]
fn deck_penalty_accelerates_and_deck_result_accounts_for_pilots() {
    assert_eq!(deck_start(2150, 0.0), 2150);
    assert_eq!(deck_start(2150, 0.25), 1950);
    assert_eq!(deck_start(2150, 0.50), 1550);
    assert_eq!(deck_start(2150, 0.75), 950);
    assert_eq!(deck_start(2150, 1.0), DECK_FLOOR);
    assert_eq!(deck_after_match(1500, 1500, 0, Outcome::FirstWin, 10), 1512);
    assert_eq!(deck_after_match(1500, 1500, 0, Outcome::FirstWin, 9), 1520);
    assert_eq!(deck_after_match(1500, 1500, 0, Outcome::Draw, 10), 1500);
    assert!(deck_after_match(1500, 1500, 400, Outcome::FirstWin, 10) < 1512);
    assert_eq!(
        deck_after_match(DECK_FLOOR, DECK_FLOOR, 0, Outcome::SecondWin, 10),
        DECK_FLOOR
    );
}
