use super::*;

#[path = "tests/placement_study.rs"]
mod placement_study;

const EPSILON: f64 = 1.0e-8;

#[test]
fn public_elo_rounds_half_away_from_zero_and_saturates() {
    for (value, want) in [
        (0.49, 0),
        (-0.5, -1),
        (0.5, 1),
        (f64::INFINITY, i32::MAX),
        (f64::NEG_INFINITY, i32::MIN),
        (f64::NAN, 0),
        (f64::from(i32::MAX) + 1.0, i32::MAX),
        (f64::from(i32::MIN) - 1.0, i32::MIN),
    ] {
        assert_eq!(rounded(value), want);
    }
}

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
fn leagues_and_divisions_follow_the_elo_alone() {
    let at = |league, division| Rank { league, division };
    assert_eq!(rank(2500, 9, 0, 0).league, -1, "placement first");
    assert_eq!(rank(1100, 10, 0, 0), at(0, 0));
    // Wood's span 1200..1320 in three equal steps
    assert_eq!(rank(1200, 10, 0, 0), at(1, 3));
    assert_eq!(rank(1239, 10, 0, 0), at(1, 3));
    assert_eq!(rank(1240, 10, 0, 0), at(1, 2));
    assert_eq!(rank(1280, 10, 0, 0), at(1, 1));
    assert_eq!(rank(1320, 10, 0, 0), at(2, 3));
    // Gold's span 1680..1840 in four; no buffer for a division once held
    assert_eq!(rank(1719, 10, 0, 0), at(5, 4));
    assert_eq!(rank(1720, 10, 0, 0), at(5, 3));
    assert_eq!(rank(1759, 700, 0, 0), at(5, 3));
    assert_eq!(rank(1760, 76, 0, 0), at(5, 2));
    assert_eq!(rank(1744, 700, 0, 0), at(5, 3));
    // the number of games after placement changes nothing
    for games in [10, 76, 107, 239, 240, 5000] {
        assert_eq!(rank(1519, games, 0, 0), at(3, 2), "{games}");
        assert_eq!(rank(1520, games, 0, 0), at(3, 1), "{games}");
        assert_eq!(rank(2500, games, 0, 0), at(7, 1), "{games}");
    }
}

/// A higher Elo is never a lower rank, whatever the games, for a young and
/// an established record alike (owner, 28.11.2026).
#[test]
fn a_higher_elo_is_never_a_lower_rank() {
    let key = |r: Rank| (r.league, -r.division);
    for (age, idle) in [(0, 0), (183, 0), (184, 14), (400, 15)] {
        let mut before = key(rank(0, 10, age, idle));
        for elo in 1..=3200 {
            let now = key(rank(elo, 10, age, idle));
            assert!(now >= before, "{elo} at {age} days, idle {idle}");
            before = now;
        }
    }
}

#[test]
fn elite_requires_six_months_and_recent_activity() {
    for (elo, days, idle, tier) in [
        (2300, 183, 0, 7),
        (2300, 184, 0, 8),
        (2300, 184, 14, 8),
        (2300, 184, 15, 7),
        (2349, 300, 0, 8),
        (2350, 300, 0, 9),
        (2600, 300, 0, 9),
        (2600, 300, 15, 7),
    ] {
        assert_eq!(
            rank(elo, 10, days, idle).league,
            tier,
            "{elo} {days} {idle}"
        );
    }
    assert!(champion_eligible(
        CHAMPION_ELO,
        PLACED_AFTER,
        i64::from(ELITE_AGE_DAYS),
        i64::from(ELITE_IDLE_DAYS)
    ));
    for (elo, games, age, idle) in [
        (2489, 10, 184, 14),
        (2490, 9, 184, 14),
        (2490, 10, 183, 14),
        (2490, 10, 184, 15),
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
