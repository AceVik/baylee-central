use super::{Outcome, rounded};

/// Confirmed matches before the faster private placement estimate expires.
pub(crate) const PLACEMENT_GAMES: i32 = 20;
/// Minimum public Elo difference for a result to contribute streak evidence.
pub(crate) const EVIDENCE_GAP: i32 = 250;
const ALLOWANCE_CAP: f64 = 64.0;

/// Persistent state for one account in a format. Both seats must come from the
/// same pre-match snapshot; a result advances both records in one transaction.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Player {
    /// Public Elo before rounding.
    pub(crate) score: f64,
    /// Fast private estimate for provisional matchmaking.
    pub(crate) placement: f64,
    /// Outstanding personal share of any points introduced or removed by asymmetric caps.
    pub(crate) allowance: f64,
    /// Number of confirmed rated matches in this record.
    pub(crate) games: i32,
}

/// Elo used to search for opponents, blending out placement after 20 matches.
pub(crate) fn queue_elo(player: Player) -> i32 {
    let remaining = PLACEMENT_GAMES - player.games.clamp(0, PLACEMENT_GAMES);
    let difference = (player.placement - player.score).clamp(-240.0, 240.0);
    rounded(player.score + difference * f64::from(remaining) / f64::from(PLACEMENT_GAMES))
}

/// Expected score for the first player, including the four-percent variance
/// floor at extreme rating differences. A positive gap favours the first seat.
pub(crate) fn expectation(gap: i32) -> f64 {
    let logistic = 1.0 / (1.0 + 10_f64.powf(-f64::from(gap.clamp(-4000, 4000)) / 400.0));
    0.04 + 0.92 * logistic
}

fn balance(players: &mut [Player; 2], points: &mut [f64; 2], ordinary: bool) {
    let imbalance = points[0] + points[1];
    let sign = imbalance.signum();
    let room = players
        .each_ref()
        .map(|p| (ALLOWANCE_CAP - sign * p.allowance).max(0.0));
    let total = imbalance.abs().min(room[0] + room[1]);
    let first = (total / 2.0).clamp((total - room[1]).max(0.0).min(room[0]), room[0]);
    players[0].allowance += sign * first;
    players[1].allowance += sign * (total - first);
    // Once the available credit runs out, shrink the larger quote. Expanding
    // the smaller one could defeat the favourite's protection on isolated losses.
    let larger = usize::from(points[1].abs() > points[0].abs());
    points[larger] -= imbalance - sign * total;
    if ordinary {
        for (p, amount) in players.iter_mut().zip(points) {
            let release =
                p.allowance.signum() * p.allowance.abs().min(1.0).min(0.25 * amount.abs());
            p.allowance -= release;
            *amount -= release;
        }
    }
}

/// Apply a rated result to both pre-match states simultaneously.
///
/// Each seat's evidence count contains prior distinct established opponents
/// in its current role, excluding the current opponent. The history reader
/// enforces the 12-match and 14-day windows. A normal qualifying outcome
/// clears the streak for that role. Draws neither increment nor clear it.
/// Prior meetings count this account pair within the preceding six hours.
pub(crate) fn settle(
    mut players: [Player; 2],
    outcome: Outcome,
    prior_upsets: [u8; 2],
    previous_meetings: i32,
) -> [Player; 2] {
    let gap = rounded(players[0].score).saturating_sub(rounded(players[1].score));
    let size = gap.saturating_abs();
    let low = usize::from(gap > 0);
    let high = 1 - low;
    let p_low = expectation(-size);
    let first_score = outcome.first_score();
    let low_score = if low == 0 {
        first_score
    } else {
        1.0 - first_score
    };
    let blend = (f64::from(size - 150) / 200.0).clamp(0.0, 1.0);
    let ordinary_gain = 24.0 * (1.0 - p_low);
    let upset_gain = 20.0 + 4.0 * f64::from(prior_upsets[low].min(3));
    let upset_loss = if players[low].games < PLACEMENT_GAMES {
        10.0
    } else {
        14.0 + 4.0 * f64::from(prior_upsets[high].min(3))
    };
    let repeat_weight = match previous_meetings {
        ..=1 => 1.0,
        2 | 3 => 0.5,
        _ => 0.25,
    };
    // Interpolate between the upset and expected outcome quotes. Conditional
    // expected points are zero under the same win probability; a draw scores half.
    let factor = repeat_weight * (low_score - p_low) / (1.0 - p_low);
    let mut delta = [0.0; 2];
    delta[low] = (ordinary_gain + blend * (upset_gain - ordinary_gain)) * factor;
    delta[high] = -(ordinary_gain + blend * (upset_loss - ordinary_gain)) * factor;

    let fast_gap = queue_elo(players[0]).saturating_sub(queue_elo(players[1]));
    let fast_change = 56.0 * (first_score - expectation(fast_gap));
    balance(
        &mut players,
        &mut delta,
        size < EVIDENCE_GAP || low_score < 1.0,
    );
    for (seat, player) in players.iter_mut().enumerate() {
        if player.games < PLACEMENT_GAMES {
            player.placement += if seat == 0 { fast_change } else { -fast_change };
        }
        player.score += delta[seat];
        player.games = player.games.saturating_add(1);
    }
    players
}
