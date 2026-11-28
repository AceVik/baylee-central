//! First passage to Master and Grandmaster at a steady ten games per calendar day.

use super::{ELITE_AGE_DAYS, Field, Outcome, SEEDS, draw, player, rank, rounded, settle};

fn report(first_days: &mut [i32]) {
    first_days.sort_unstable();
    if let Some(first) = first_days.first() {
        println!(
            "    {} reached, first playable day {first}, median first day {}",
            first_days.len(),
            first_days[first_days.len() / 2]
        );
    } else {
        println!("    none reached");
    }
}

#[test]
#[ignore = "3,686,400 additional games at ten per calendar day"]
fn elite_first_passage() {
    for skill in [2100, 2300, 2500] {
        let (mut master, mut grandmaster) = (Vec::new(), Vec::new());
        for seed in SEEDS {
            for stream in 0..64 {
                let mut rng = seed << 40 | stream;
                let mut pilot = player(1500, 0);
                let (mut reached_master, mut reached_gm) = (false, false);
                for game in 1..=3200 {
                    let rival_skill = Field::Dense.opponent(pilot);
                    let chance =
                        0.04 + 0.92 / (1.0 + 10_f64.powf(f64::from(rival_skill - skill) / 400.0));
                    let outcome = if draw(&mut rng) < chance {
                        Outcome::FirstWin
                    } else {
                        Outcome::SecondWin
                    };
                    pilot = settle([pilot, player(rival_skill, 500)], outcome, [0; 2], 0)[0];
                    let day = (game - 1) / 10 + 1;
                    let badge = rank(rounded(pilot.score), game, i64::from(day - 1), 0);
                    // Master and above ask for six months since the first game
                    if badge.league >= 8 && !reached_master {
                        assert!(day > ELITE_AGE_DAYS);
                        master.push(day);
                        reached_master = true;
                    }
                    if badge.league >= 9 && !reached_gm {
                        assert!(day > ELITE_AGE_DAYS);
                        grandmaster.push(day);
                        reached_gm = true;
                    }
                }
            }
        }
        println!("True Elo {skill}, 384 careers, ten games daily for 320 days. Master:");
        report(&mut master);
        println!("  Grandmaster:");
        report(&mut grandmaster);
        if skill == 2500 {
            assert!(!grandmaster.is_empty(), "elite must be attainable");
        }
    }
}
