//! Reproducible offline study of the production policy, with no database or network.

use super::super::{
    ELITE_AGE_DAYS, Outcome, Player, Rank, deck_after_match, deck_start, queue_elo, rank, rounded,
    settle,
};

const SEEDS: [u64; 6] = [581, 1901, 3299, 7429, 8191, 13_007];
const HISTORIES: u32 = 512;

/// A deterministic draw from `SplitMix64` for the simulation's match outcomes.
fn draw(seed: &mut u64) -> f64 {
    *seed = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let a = (*seed ^ (*seed >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    let b = (a ^ (a >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    let upper = u32::try_from((b ^ (b >> 31)) >> 32).unwrap();
    f64::from(upper) / 4_294_967_296.0
}

fn player(elo: i32, games: i32) -> Player {
    Player {
        score: f64::from(elo),
        placement: f64::from(elo),
        allowance: 0.0,
        games,
    }
}

#[derive(Clone, Copy, Debug)]
enum Field {
    /// Established opponents of every rating are present and change every game.
    Dense,
    /// An established 1000-Elo account is the only available opponent.
    LoneWeak,
    /// An established 2000-Elo account is the only available opponent.
    LoneStrong,
}

impl Field {
    fn opponent(self, seeker: Player) -> i32 {
        match self {
            Self::Dense => ((queue_elo(seeker) + 25) / 50 * 50).clamp(800, 2400),
            Self::LoneWeak => 1000,
            Self::LoneStrong => 2000,
        }
    }
}

#[derive(Default)]
struct Sample {
    wins: u32,
    published_elo: f64,
    rank_counts: [u32; 11],
    provisional_elo: f64,
    deck_elo: f64,
}

impl Sample {
    fn add(&mut self, wins: u32, player: Player, badge: Rank, deck: i32) {
        self.wins += wins;
        self.published_elo += player.score;
        self.rank_counts[usize::try_from(badge.league).unwrap()] += 1;
        self.provisional_elo += f64::from(queue_elo(player));
        self.deck_elo += f64::from(deck);
    }

    fn print(&self, games: i32, histories: u32) {
        let n = f64::from(histories);
        println!(
            "  {games:3}: mean wins {:.2}, Elo {:.1}, queue Elo {:.1}, deck Elo {:.1}, league counts {:?}",
            f64::from(self.wins) / n,
            self.published_elo / n,
            self.provisional_elo / n,
            self.deck_elo / n,
            self.rank_counts
        );
    }
}

struct Study {
    at_ten: Sample,
    at_hundred: Sample,
    sparse_placement_upsets: u32,
    largest_favourite_loss: f64,
}

impl Study {
    fn history(&mut self, field: Field, skill: i32, mut seed: u64) {
        let mut pilot = player(1500, 0);
        let mut opponent = player(field.opponent(pilot), 500);
        let mut deck = 1500;
        let mut rival_deck = 1500;
        let mut wins = 0;
        for game in 1..=100 {
            let rival_skill = field.opponent(pilot);
            let prior_meetings = if matches!(field, Field::Dense) {
                opponent = player(rival_skill, 500);
                rival_deck = 1500;
                0
            } else {
                // Ten games per day, finished inside six hours, followed by an overnight break.
                (game - 1) % 10
            };
            let win_p = 0.04 + 0.92 / (1.0 + 10_f64.powf(f64::from(rival_skill - skill) / 400.0));
            let win = draw(&mut seed) < win_p;
            wins += u32::from(win);
            let (outcome, reverse) = if win {
                (Outcome::FirstWin, Outcome::SecondWin)
            } else {
                (Outcome::SecondWin, Outcome::FirstWin)
            };
            let gap = rounded(pilot.score) - rounded(opponent.score);
            let old_opponent_score = opponent.score;
            // Fresh evenly matched opponents have no wide-upset streak. A pool of two
            // cannot add distinct established opponents to its evidence count.
            [pilot, opponent] = settle([pilot, opponent], outcome, [0; 2], prior_meetings);
            if game <= 10 && win && gap <= -350 {
                self.sparse_placement_upsets += 1;
                self.largest_favourite_loss = self
                    .largest_favourite_loss
                    .max(old_opponent_score - opponent.score);
            }
            (deck, rival_deck) = (
                deck_after_match(deck, rival_deck, gap, outcome, game - 1),
                deck_after_match(rival_deck, deck, -gap, reverse, 500 + game - 1),
            );
            let badge = rank(rounded(pilot.score), game, i64::from((game - 1) / 10), 0);
            if game < 10 {
                assert_eq!(badge.league, -1, "published before placement completed");
            } else if game == 10 {
                self.at_ten.add(wins, pilot, badge, deck);
            } else if game == 100 {
                self.at_hundred.add(wins, pilot, badge, deck);
            }
        }
    }
}

#[test]
#[ignore = "2,764,800 synthetic games; run explicitly with --ignored --nocapture"]
fn placement_population() {
    let histories = HISTORIES * u32::try_from(SEEDS.len()).unwrap();
    for field in [Field::Dense, Field::LoneWeak, Field::LoneStrong] {
        let mut previous_elo = 0.0;
        for skill in [1100, 1500, 1900] {
            let mut study = Study {
                at_ten: Sample::default(),
                at_hundred: Sample::default(),
                sparse_placement_upsets: 0,
                largest_favourite_loss: 0.0,
            };
            for seed in SEEDS {
                for stream in 0..HISTORIES {
                    study.history(field, skill, seed << 40 | u64::from(stream));
                }
            }
            println!("{field:?}, true pilot Elo {skill}, {histories} histories:");
            study.at_ten.print(10, histories);
            study.at_hundred.print(100, histories);
            assert_eq!(study.at_ten.rank_counts.iter().sum::<u32>(), histories);
            assert!(study.at_ten.published_elo > previous_elo);
            previous_elo = study.at_ten.published_elo;
            if matches!(field, Field::LoneStrong) {
                assert!(study.sparse_placement_upsets > 0);
                assert!(study.largest_favourite_loss <= 10.000_001);
                println!(
                    "  {} provisional wide upsets, maximum favourite loss {:.2}",
                    study.sparse_placement_upsets, study.largest_favourite_loss
                );
            }
        }
    }
}

#[path = "placement_decks.rs"]
mod deck_edits;

#[path = "placement_progression.rs"]
mod progression;
