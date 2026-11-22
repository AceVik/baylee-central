//! Direct, staged and reverted edits use identical outcomes for the final version.

use super::{Outcome, SEEDS, deck_after_match, deck_start, draw};
use std::{cmp::Ordering, collections::BTreeMap};

// First 60 slots are main, last 15 are sideboard. Repeated identities exercise copy-counting.
type List = [u16; 75];
type Record = (i32, i32);

fn base() -> List {
    std::array::from_fn(|i| u16::try_from(i / 3).unwrap())
}

fn edited(count: usize) -> List {
    let mut cards = base();
    for card in &mut cards[..count] {
        *card += 100;
    }
    cards
}

fn key(mut list: List) -> List {
    list.sort_unstable();
    list
}

fn distance(first: List, second: List) -> f64 {
    let (first, second) = (key(first), key(second));
    let (mut i, mut j, mut common) = (0, 0, 0);
    while i < first.len() && j < second.len() {
        match first[i].cmp(&second[j]) {
            Ordering::Less => i += 1,
            Ordering::Greater => j += 1,
            Ordering::Equal => {
                common += 1;
                i += 1;
                j += 1;
            }
        }
    }
    f64::from(75 - common) / 75.0
}

struct Book {
    anchor: i32,
    versions: BTreeMap<List, Record>,
}

impl Book {
    fn new(anchor: i32) -> Self {
        Self {
            anchor,
            versions: BTreeMap::from([(key(base()), (anchor, 40))]),
        }
    }

    fn open(&mut self, list: List) -> Record {
        let reference = self.anchor.min(self.versions[&key(base())].0);
        *self
            .versions
            .entry(key(list))
            .or_insert_with(|| (deck_start(reference, distance(base(), list)), 0))
    }

    fn play(&mut self, list: List, against: i32, outcome: Outcome) -> Record {
        let (elo, games) = self.open(list);
        let updated = (deck_after_match(elo, against, 0, outcome, games), games + 1);
        self.versions.insert(key(list), updated);
        updated
    }
}

fn result(seed: &mut u64) -> Outcome {
    if draw(seed) < 0.5 {
        Outcome::FirstWin
    } else {
        Outcome::SecondWin
    }
}

#[test]
#[ignore = "run alongside the production placement population study"]
fn staged_edits_and_returns() {
    let (mut resumes, mut matches, mut controls) = (0, 0, Vec::new());
    for mut seed in SEEDS {
        for anchor in [1000, 1500, 1900] {
            for changes in 1..=75 {
                let (mut direct, mut staged) = (Book::new(anchor), Book::new(anchor));
                for _ in 0..10 {
                    let outcome = result(&mut seed);
                    direct.play(base(), 1500, outcome);
                    staged.play(base(), 1500, outcome);
                    matches += 2;
                }
                let original = direct.open(base());
                let (mut previous_list, mut previous_elo) = (base(), anchor.min(original.0));
                for changed in (1..changes).step_by(3) {
                    let list = edited(changed);
                    let outcome = result(&mut seed);
                    staged.play(list, 1500, outcome);
                    let naive_start = deck_start(previous_elo, distance(previous_list, list));
                    previous_elo = deck_after_match(naive_start, 1500, 0, outcome, 0);
                    previous_list = list;
                    matches += 1;
                }
                let final_list = edited(changes);
                let inherited = direct.open(final_list);
                assert_eq!(staged.open(final_list), inherited);
                if changes == 75 {
                    assert_eq!(inherited, (150, 0));
                    controls.push(
                        deck_start(previous_elo, distance(previous_list, final_list)) - inherited.0,
                    );
                }
                for _ in 0..10 {
                    let outcome = result(&mut seed);
                    assert_eq!(
                        staged.play(final_list, 1500, outcome),
                        direct.play(final_list, 1500, outcome)
                    );
                    matches += 2;
                }
                let saved = direct.open(final_list);
                let mut sideboard_swap = final_list;
                sideboard_swap.rotate_right(15);
                assert_eq!(staged.open(base()), original);
                assert_eq!(staged.open(sideboard_swap), saved);
                assert_eq!(saved.1, 10);
                resumes += 2;
            }
        }
    }
    controls.sort_unstable();
    println!(
        "{matches} match outcomes, {resumes} verified returns / sideboard swaps. Median full-rebuild advantage of naive predecessor scoring: {} Elo",
        controls[controls.len() / 2]
    );
}
