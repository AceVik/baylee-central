use super::{Outcome, rounded};

/// Starting Elo for a brand-new composition with no established reference,
/// also the minimum deck score and the residue after a complete replacement.
pub(crate) const DECK_FLOOR: i32 = 150;

/// Inherited Elo for a new canonical pool. The caller selects the nearest
/// established season anchor, using the lesser of frozen and current ratings.
/// `edited_share` is one minus the multiset overlap divided by the larger
/// pool size. Main deck and sideboard copies share one pool.
pub(crate) fn deck_start(rating: i32, edited_share: f64) -> i32 {
    let fraction = edited_share.clamp(0.0, 1.0);
    let loss = fraction * (0.2 + 0.8 * fraction);
    let anchor = rating.max(DECK_FLOOR);
    anchor - rounded(f64::from(anchor - DECK_FLOOR) * loss)
}

/// New Elo for the first seat's deck vs its opponent's pre-match deck Elo.
/// The pilots' pre-match Elo difference adjusts the deck's expected performance.
pub(crate) fn deck_after_match(
    rating: i32,
    against: i32,
    pilot_gap: i32,
    outcome: Outcome,
    prior_games: i32,
) -> i32 {
    let exponent = (f64::from(against) - f64::from(rating) - f64::from(pilot_gap)) / 400.0;
    let expected = 1.0 / (1.0 + 10_f64.powf(exponent));
    let k = if prior_games < 10 { 40.0 } else { 24.0 };
    let change = rounded(k * (outcome.first_score() - expected));
    rating.saturating_add(change).max(DECK_FLOOR)
}
