//! Which eligible AI fills a lonely search: one swappable decision
//! (`docs/ai-players.md` section 3, open question 2). The hard rules (enabled,
//! a live offer, free slots, caps) are the caller's; this only ranks.
use central_api::ranked::ai::AiKind;
use sea_orm::prelude::Uuid;

/// One AI that passed every hard rule.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Candidate {
    pub(crate) id: Uuid,
    pub(crate) kind: AiKind,
    /// Its queue Elo in the search's format.
    pub(crate) elo: i32,
    /// Finished matches with this person in the last six hours.
    pub(crate) meetings: i32,
    /// Its matches reserved or playing now.
    pub(crate) live: i64,
}

/// The queue's own repeat cost for each recent meeting (`matchmaking::preference`).
pub(crate) const REPEAT: i64 = 140;
/// Candidates within this of the cheapest are a near tie, where the free
/// kinds go first.
pub(crate) const TIE: i64 = 100;

/// The queue's distance: the rating gap and the repeat cost, at most three meetings.
pub(crate) fn cost(person: i32, candidate: &Candidate) -> i64 {
    (i64::from(person) - i64::from(candidate.elo)).abs()
        + REPEAT * i64::from(candidate.meetings.clamp(0, 3))
}

/// The order in which to try the candidates, best first: within [`TIE`] of
/// the cheapest the free kinds before a language model, then cost, fewer live
/// matches, the account id.
pub(crate) fn order(person: i32, candidates: &[Candidate]) -> Vec<usize> {
    let cheapest = candidates.iter().map(|c| cost(person, c)).min();
    let mut order: Vec<usize> = (0..candidates.len()).collect();
    order.sort_by_key(|&i| {
        let candidate = &candidates[i];
        let price = cost(person, candidate);
        let near = cheapest.is_some_and(|least| price - least <= TIE);
        let tier = match (near, candidate.kind.free()) {
            (true, true) => 0,
            (true, false) => 1,
            (false, _) => 2,
        };
        (tier, price, candidate.live, candidate.id)
    });
    order
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ai(id: u128, kind: AiKind, elo: i32, meetings: i32, live: i64) -> Candidate {
        Candidate {
            id: Uuid::from_u128(id),
            kind,
            elo,
            meetings,
            live,
        }
    }

    #[test]
    fn the_nearest_wins_and_a_recent_meeting_costs_as_in_the_queue() {
        let near = ai(1, AiKind::House, 1500, 0, 0);
        let far = ai(2, AiKind::House, 1700, 0, 0);
        assert_eq!(order(1500, &[far, near])[0], 1);
        // Met twice in six hours: 280 more, so the farther one comes first.
        let met = ai(1, AiKind::House, 1500, 2, 0);
        assert_eq!(order(1500, &[met, far])[0], 1);
        // More than three meetings cost no more than three.
        assert_eq!(cost(1500, &ai(1, AiKind::House, 1500, 9, 0)), 3 * REPEAT);
        assert_eq!(cost(1500, &ai(1, AiKind::House, 1500, -4, 0)), 0);
    }

    #[test]
    fn a_free_kind_goes_before_a_model_only_on_a_near_tie() {
        let model = ai(1, AiKind::Model, 1500, 0, 0);
        let house = ai(2, AiKind::House, 1590, 0, 0);
        assert_eq!(order(1500, &[model, house])[0], 1);
        let distant = ai(2, AiKind::Net, 1650, 0, 0);
        assert_eq!(order(1500, &[model, distant])[0], 0);
    }

    #[test]
    fn equal_costs_go_to_the_less_busy_then_the_lower_id() {
        let busy = ai(1, AiKind::House, 1500, 0, 3);
        let idle = ai(2, AiKind::House, 1500, 0, 0);
        assert_eq!(order(1500, &[busy, idle]), vec![1, 0]);
        let other = ai(0, AiKind::House, 1500, 0, 0);
        assert_eq!(order(1500, &[idle, other]), vec![1, 0]);
        assert!(order(1500, &[]).is_empty());
        // Extreme ratings never overflow.
        let low = ai(3, AiKind::House, i32::MIN, 3, i64::MAX);
        assert_eq!(order(i32::MAX, &[low]).len(), 1);
    }
}
