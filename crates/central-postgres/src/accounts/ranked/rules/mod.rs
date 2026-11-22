//! Ranked policy used by Central for confirmed 1v1 results.

mod deck;
mod ladder;
mod player;

pub(crate) use deck::{deck_after_match, deck_start};
pub(crate) use ladder::{
    CHAMPION_ELO, CHAMPION_GAMES, CHAMPION_LIMIT, ELITE_AGE_DAYS, ELITE_IDLE_DAYS, Rank, rank,
};
pub(crate) use player::{EVIDENCE_GAP, PLACEMENT_GAMES, Player, queue_elo, settle};
#[cfg(test)]
use {deck::DECK_FLOOR, ladder::champion_eligible};

/// The authoritative result, relative to the first seat of the supplied pair.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Outcome {
    /// The first seat won.
    FirstWin,
    /// The second seat won.
    SecondWin,
    /// The game ended in a draw.
    Draw,
}

impl Outcome {
    fn first_score(self) -> f64 {
        match self {
            Self::FirstWin => 1.0,
            Self::SecondWin => 0.0,
            Self::Draw => 0.5,
        }
    }
}

/// Round only at a public Elo boundary. Internal player balances retain fractions.
pub(crate) fn rounded(value: f64) -> i32 {
    // Integral Elo is I/O-bound (settlement and queue admission). This checked
    // conversion avoids narrowing the fractional score before the final rounding.
    value
        .round()
        .clamp(f64::from(i32::MIN), f64::from(i32::MAX))
        .to_string()
        .parse()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;
