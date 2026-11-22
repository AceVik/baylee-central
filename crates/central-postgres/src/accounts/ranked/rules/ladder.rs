/// Maximum number of simultaneously published player Champion titles worldwide.
pub(crate) const CHAMPION_LIMIT: i32 = 100;
/// Minimum public score for eligibility for a Champion title.
pub(crate) const CHAMPION_ELO: i32 = 2490;
/// Minimum confirmed results for eligibility for a Champion title.
pub(crate) const CHAMPION_GAMES: i32 = 3000;
/// Minimum elapsed whole days since a first ranked game for Master and above.
pub(crate) const ELITE_AGE_DAYS: i32 = 184;
/// Maximum inactivity in whole days for the elite ranks.
pub(crate) const ELITE_IDLE_DAYS: i32 = 14;

// Index is the public league identifier. Each entry sets the league's minimum
// integer Elo, confirmed matches and number of numbered divisions.
const GATES: [(i32, i32, i32); 11] = [
    (0, 0, 0),
    (1200, 20, 3),
    (1320, 60, 3),
    (1440, 120, 3),
    (1560, 240, 3),
    (1680, 440, 4),
    (1840, 800, 4),
    (2000, 1240, 4),
    (2240, 2000, 0),
    (2350, 2500, 0),
    (CHAMPION_ELO, CHAMPION_GAMES, 0),
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
    let (points, matches, _) = GATES[CHAMPION];
    elo >= points && games >= matches && elite_eligible(age_days, idle_days)
}

/// Compute the local public rank up to Grandmaster. The standings publisher
/// assigns the scarce Champion titles. Durations are elapsed whole days since
/// the first and most recent confirmed match for this ranked record.
pub(crate) fn rank(elo: i32, games: i32, age_days: i64, idle_days: i64, previous: Rank) -> Rank {
    if games < 10 {
        return Rank {
            league: -1,
            division: 0,
        };
    }
    // A ten-game placement can earn at most Bronze. Silver and higher use
    // confirmed volume exclusively, independently of the opponent field.
    let placement_credit = elo.saturating_sub(1400).clamp(0, 200) * 110 / 200;
    let volume = |tier: usize| {
        if tier <= 3 {
            games.saturating_add(placement_credit)
        } else {
            games
        }
    };
    let until = if elite_eligible(age_days, idle_days) {
        CHAMPION
    } else {
        MASTER
    };
    let tier = GATES[..until]
        .iter()
        .enumerate()
        .rposition(|(tier, &(points, matches, _))| elo >= points && volume(tier) >= matches)
        .unwrap_or(0);
    let (points, matches, divisions) = GATES[tier];
    let league = i32::try_from(tier).expect("eleven leagues");
    if divisions == 0 {
        return Rank {
            league,
            division: 0,
        };
    }

    let (next_points, next_matches, _) = GATES[tier + 1];
    let mut division = divisions;
    for step in 1..divisions {
        let numeral = divisions - step;
        // Once held, a division has a 15-point demotion buffer. League
        // boundaries and match counts are always strict.
        let held = league == previous.league && (1..=numeral).contains(&previous.division);
        let threshold = |span| (span * step + divisions - 1) / divisions;
        let required = points + threshold(next_points - points) - if held { 15 } else { 0 };
        if elo < required || volume(tier) < matches + threshold(next_matches - matches) {
            break;
        }
        division = numeral;
    }
    Rank { league, division }
}
