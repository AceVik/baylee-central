//! Whether the carousel may make a match for the AI asking, and if not, what
//! to tell its runner (`docs/ai-players.md` section 7). The caller counts;
//! this only decides, so every rule has its own test.
use central_api::ranked::ai::Unseated;
use sea_orm::prelude::Uuid;

/// What the carousel counts before it makes a match, and again after each
/// lock: the settings, the AI asking, and every AI match.
#[derive(Clone, Copy, Debug)]
pub(super) struct Facts {
    /// The open season, which the match is made in.
    pub(super) season: Option<Uuid>,
    /// AI-against-AI matches that may be live at once.
    pub(super) carousel_games: u8,
    /// AI-against-AI matches that may be made in a UTC day.
    pub(super) carousel_daily: u32,
    /// AI seats that may be live at once.
    pub(super) global_seats: u8,
    /// The AI asking is switched on.
    pub(super) enabled: bool,
    /// It offers a deck for the format under a session with time left.
    pub(super) offered: bool,
    /// Its matches reserved or playing now.
    pub(super) live: i64,
    /// Its `max_games`.
    pub(super) max_games: i16,
    /// Its matches made today (UTC), with a person or another AI.
    pub(super) today: i64,
    /// Its `daily_games`, none for no cap.
    pub(super) daily_games: Option<i32>,
    /// AI-against-AI matches reserved or playing now.
    pub(super) carousel_live: i64,
    /// AI-against-AI matches made today (UTC).
    pub(super) carousel_today: i64,
    /// AI seats in live matches, two for each AI-against-AI one.
    pub(super) seats: i64,
    /// A person's ticket waits in the format and season.
    pub(super) people: bool,
}

/// Why no match may be made now, none when every rule allows one. The
/// reasons go from the most lasting to the least: what needs an
/// administrator, what needs an offer, what needs the next day, what needs
/// a while.
pub(super) fn unseated(facts: &Facts) -> Option<Unseated> {
    if facts.season.is_none()
        || !facts.enabled
        || facts.daily_games == Some(0)
        || facts.carousel_games == 0
        || facts.carousel_daily == 0
        || facts.global_seats < 2
    {
        return Some(Unseated::Off);
    }
    if !facts.offered {
        return Some(Unseated::Offer);
    }
    if facts
        .daily_games
        .is_some_and(|cap| facts.today >= i64::from(cap))
        || facts.carousel_today >= i64::from(facts.carousel_daily)
    {
        return Some(Unseated::Tomorrow);
    }
    // People first, then every bound on what is live; a match takes two seats.
    if facts.people
        || facts.live >= i64::from(facts.max_games)
        || facts.carousel_live >= i64::from(facts.carousel_games)
        || facts.seats.saturating_add(2) > i64::from(facts.global_seats)
    {
        return Some(Unseated::Later);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Room for everything: a match may be made.
    fn open() -> Facts {
        Facts {
            season: Some(Uuid::from_u128(1)),
            carousel_games: 2,
            carousel_daily: 10,
            global_seats: 8,
            enabled: true,
            offered: true,
            live: 0,
            max_games: 2,
            today: 0,
            daily_games: None,
            carousel_live: 0,
            carousel_today: 0,
            seats: 0,
            people: false,
        }
    }

    fn with(change: impl FnOnce(&mut Facts)) -> Option<Unseated> {
        let mut facts = open();
        change(&mut facts);
        unseated(&facts)
    }

    #[test]
    fn with_room_everywhere_a_match_is_made_up_to_each_bound() {
        assert_eq!(unseated(&open()), None);
        // One below each bound still plays.
        assert_eq!(with(|f| f.live = 1), None);
        assert_eq!(with(|f| f.carousel_live = 1), None);
        assert_eq!(with(|f| f.carousel_today = 9), None);
        assert_eq!(with(|f| f.seats = 6), None);
        assert_eq!(
            with(|f| {
                f.daily_games = Some(3);
                f.today = 2;
            }),
            None
        );
    }

    #[test]
    fn what_an_administrator_must_change_is_off() {
        for change in [
            (|f: &mut Facts| f.season = None) as fn(&mut Facts),
            |f| f.enabled = false,
            |f| f.daily_games = Some(0),
            |f| f.carousel_games = 0,
            |f| f.carousel_daily = 0,
            |f| f.global_seats = 1,
            |f| f.global_seats = 0,
        ] {
            assert_eq!(with(change), Some(Unseated::Off));
        }
        // Off comes before every other reason.
        assert_eq!(
            with(|f| {
                f.enabled = false;
                f.offered = false;
                f.people = true;
                f.carousel_today = 99;
            }),
            Some(Unseated::Off)
        );
    }

    #[test]
    fn without_an_offer_the_runner_offers_first() {
        assert_eq!(with(|f| f.offered = false), Some(Unseated::Offer));
        assert_eq!(
            with(|f| {
                f.offered = false;
                f.people = true;
            }),
            Some(Unseated::Offer)
        );
    }

    #[test]
    fn a_daily_cap_waits_for_the_next_day() {
        assert_eq!(with(|f| f.carousel_today = 10), Some(Unseated::Tomorrow));
        assert_eq!(
            with(|f| {
                f.daily_games = Some(3);
                f.today = 3;
            }),
            Some(Unseated::Tomorrow)
        );
        // The day's cap outlasts a full table.
        assert_eq!(
            with(|f| {
                f.carousel_today = 11;
                f.live = 2;
            }),
            Some(Unseated::Tomorrow)
        );
    }

    #[test]
    fn people_first_and_every_live_bound_wait_a_while() {
        for change in [
            (|f: &mut Facts| f.people = true) as fn(&mut Facts),
            |f| f.live = 2,
            |f| f.carousel_live = 2,
            // One seat left is no room for a match of two AIs.
            |f| f.seats = 7,
            |f| f.seats = 8,
        ] {
            assert_eq!(with(change), Some(Unseated::Later));
        }
    }

    #[test]
    fn extreme_counts_never_overflow() {
        assert_eq!(with(|f| f.seats = i64::MAX), Some(Unseated::Later));
        assert_eq!(
            with(|f| {
                f.carousel_daily = u32::MAX;
                f.carousel_today = i64::MAX;
            }),
            Some(Unseated::Tomorrow)
        );
        assert_eq!(
            with(|f| {
                f.daily_games = Some(i32::MAX);
                f.today = i64::MIN;
            }),
            None
        );
    }
}
