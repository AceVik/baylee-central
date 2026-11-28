//! Public leagues and divisions follow the Elo alone, as in chess or League of
//! Legends (owner, 28.11.2026): a higher Elo is never a lower rank. Confirmed
//! games count only for the ten placement games; Master and above also ask
//! for six months since the first game and recent activity.

/// Maximum number of simultaneously published player Champion titles worldwide.
pub(crate) const CHAMPION_LIMIT: i32 = 100;
/// Minimum public score for eligibility for a Champion title.
pub(crate) const CHAMPION_ELO: i32 = 2490;
/// Confirmed results before a record is placed and has a public rank.
pub(crate) const PLACED_AFTER: i32 = 10;
/// Minimum elapsed whole days since a first ranked game for Master and above.
pub(crate) const ELITE_AGE_DAYS: i32 = 184;
/// Maximum inactivity in whole days for the elite ranks.
pub(crate) const ELITE_IDLE_DAYS: i32 = 14;

// Index is the public league identifier. Each entry sets the league's minimum
// integer Elo and number of numbered divisions, which split the league's Elo
// span evenly.
const GATES: [(i32, i32); 11] = [
    (0, 0),
    (1200, 3),
    (1320, 3),
    (1440, 3),
    (1560, 3),
    (1680, 4),
    (1840, 4),
    (2000, 4),
    (2240, 0),
    (2350, 0),
    (CHAMPION_ELO, 0),
];
const MASTER: usize = 8;
const CHAMPION: usize = 10;

/// Public league and Roman division numeral, independent of localization.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Rank {
    /// Unranked (-1) until placement completes, then Paper (0) through Champion (10).
    pub(crate) league: i32,
    /// One is the best division; zero denotes an undivided league.
    pub(crate) division: i32,
}

fn elite_eligible(age_days: i64, idle_days: i64) -> bool {
    age_days >= i64::from(ELITE_AGE_DAYS) && (0..=i64::from(ELITE_IDLE_DAYS)).contains(&idle_days)
}

/// Determine eligibility before global top-100 selection and tiebreaking.
#[cfg(test)]
pub(crate) fn champion_eligible(elo: i32, games: i32, age_days: i64, idle_days: i64) -> bool {
    elo >= CHAMPION_ELO && games >= PLACED_AFTER && elite_eligible(age_days, idle_days)
}

/// Compute the local public rank up to Grandmaster from the Elo alone, once
/// placed. The standings publisher assigns the scarce Champion titles.
/// Durations are elapsed whole days since the first and most recent confirmed
/// match for this ranked record; they gate only Master and above.
pub(crate) fn rank(elo: i32, games: i32, age_days: i64, idle_days: i64) -> Rank {
    if games < PLACED_AFTER {
        return Rank {
            league: -1,
            division: 0,
        };
    }
    let until = if elite_eligible(age_days, idle_days) {
        CHAMPION
    } else {
        MASTER
    };
    let tier = GATES[..until]
        .iter()
        .rposition(|&(points, _)| elo >= points)
        .unwrap_or(0);
    let (points, divisions) = GATES[tier];
    let league = i32::try_from(tier).expect("eleven leagues");
    if divisions == 0 {
        return Rank {
            league,
            division: 0,
        };
    }
    let (next_points, _) = GATES[tier + 1];
    let span = next_points - points;
    let mut division = divisions;
    for step in 1..divisions {
        // The step's share of the league's span, rounded up.
        if elo < points + (span * step + divisions - 1) / divisions {
            break;
        }
        division = divisions - step;
    }
    Rank { league, division }
}
