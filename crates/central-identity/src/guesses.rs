//! Guesses at passwords, counted in memory only (`docs/native-login.md`
//! §"Guessing a password online").
//!
//! A sign-in's start hands out KE2, and with KE2 a guessed password is
//! checked without another word to Central. So every start is a guess until
//! the proof that follows shows it was none ([`Guesses::proved`]); the start
//! of a password change (two evaluations) and of a deletion count the same.
//!
//! Counted are each account over every place that is not its own, each
//! account at each place, each place over every account, and an IPv6 /48
//! over its /64s. A place an account proved its password from is its own
//! for [`OWN_DAYS`] days: it draws on its own count there and on nothing
//! else, so what others guess never keeps the account's holder out where
//! the holder signs in.
//!
//! Nothing here is written down or logged, and a restart forgets all of it.
//! Every table is bounded; a full one forgets its oldest entry and refuses
//! nobody. A refusal is `Busy` and spends nothing.
use central_api::Error;
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

const MINUTE: Duration = Duration::from_secs(60);
const HOUR: Duration = Duration::from_secs(3_600);
const DAY: Duration = Duration::from_hours(24);

/// OPAQUE evaluations a minute for one account from the places that are
/// not its own, the proven ones included: no more than that many guesses
/// leave in any minute.
pub(super) const ACCOUNT_PER_MINUTE: u32 = 8;
/// Guesses an hour at one account from the places that are not its own.
pub(super) const ACCOUNT_PER_HOUR: u32 = 30;
/// The same a day: what all the world together may try on one password.
pub(super) const ACCOUNT_PER_DAY: u32 = 100;
/// Guesses an hour at one account from one place: room for someone who
/// has to try a few passwords, and no more than a third of the account's
/// hour for any one address.
pub(super) const PAIR_PER_HOUR: u32 = 10;
/// The same a day.
pub(super) const PAIR_PER_DAY: u32 = 30;
/// Guesses an hour from one place at all accounts together, the ones a
/// place makes at accounts it is the own place of left out.
pub(super) const PLACE_PER_HOUR: u32 = 30;
/// The same a day.
pub(super) const PLACE_PER_DAY: u32 = 100;
/// Guesses an hour from one IPv6 /48 at all accounts together: four of
/// its /64s' budgets, however many of the 65,536 ask.
pub(super) const WIDE_PER_HOUR: u32 = 120;
/// The same a day.
pub(super) const WIDE_PER_DAY: u32 = 400;
/// Days a place stays an account's own after a proof from it.
pub(super) const OWN_DAYS: u64 = 30;
/// Own places one account has at once; a ninth takes the place of the one
/// that would end first, so no account's places push out another's.
pub(super) const OWN_PER_ACCOUNT: usize = 8;
/// Entries one table holds: accounts, accounts at places, places, and the
/// accounts with places of their own.
pub(super) const ENTRIES: usize = 16_384;

/// Where a request came from, as these budgets count it: the listener says
/// which address that is. Kept in memory only; it has no `Debug`, so that
/// nothing prints one by accident.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Place {
    /// An IPv4 address.
    V4([u8; 4]),
    /// The first 64 bits of an IPv6 address, the block one subscriber
    /// usually holds.
    V6([u8; 8]),
}

impl Place {
    /// The /48 an IPv6 place lies in.
    fn wide(self) -> Option<[u8; 6]> {
        match self {
            Self::V4(_) => None,
            Self::V6(prefix) => {
                let mut wide = [0; 6];
                wide.copy_from_slice(&prefix[..6]);
                Some(wide)
            }
        }
    }
}

/// A place, or the /48 around an IPv6 one.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Net {
    Place(Place),
    Wide([u8; 6]),
}

/// The most one tally may hold in each of its windows.
#[derive(Clone, Copy)]
struct Caps {
    minute: u32,
    hour: u32,
    day: u32,
}

const ACCOUNT: Caps = Caps {
    minute: ACCOUNT_PER_MINUTE,
    hour: ACCOUNT_PER_HOUR,
    day: ACCOUNT_PER_DAY,
};
const PAIR: Caps = Caps {
    minute: u32::MAX,
    hour: PAIR_PER_HOUR,
    day: PAIR_PER_DAY,
};
const PLACE: Caps = Caps {
    minute: u32::MAX,
    hour: PLACE_PER_HOUR,
    day: PLACE_PER_DAY,
};
const WIDE: Caps = Caps {
    minute: u32::MAX,
    hour: WIDE_PER_HOUR,
    day: WIDE_PER_DAY,
};
/// A reset's start: the minute's evaluations and nothing longer.
const EVALUATION: Caps = Caps {
    minute: ACCOUNT_PER_MINUTE,
    hour: u32::MAX,
    day: u32::MAX,
};

/// One window: when it began and what was spent in it.
#[derive(Clone, Copy)]
struct Span {
    start: Instant,
    used: u32,
}

impl Span {
    const fn new(now: Instant) -> Self {
        Self {
            start: now,
            used: 0,
        }
    }

    fn used(self, now: Instant, period: Duration) -> u32 {
        if now.saturating_duration_since(self.start) >= period {
            0
        } else {
            self.used
        }
    }

    fn spend(&mut self, now: Instant, period: Duration, units: u32) {
        if now.saturating_duration_since(self.start) >= period {
            *self = Self::new(now);
        }
        self.used = self.used.saturating_add(units);
    }

    /// What was spent at `at` comes back, unless the window began anew
    /// since: a later window never gets back what it did not count.
    fn give_back(&mut self, at: Instant, units: u32) {
        if self.start <= at {
            self.used = self.used.saturating_sub(units);
        }
    }
}

/// What one account, one account at one place, or one place has spent.
#[derive(Clone, Copy)]
struct Tally {
    minute: Span,
    hour: Span,
    day: Span,
}

impl Tally {
    const fn new(now: Instant) -> Self {
        Self {
            minute: Span::new(now),
            hour: Span::new(now),
            day: Span::new(now),
        }
    }

    fn fits(self, now: Instant, caps: Caps, units: u32) -> bool {
        self.minute.used(now, MINUTE).saturating_add(units) <= caps.minute
            && self.hour.used(now, HOUR).saturating_add(units) <= caps.hour
            && self.day.used(now, DAY).saturating_add(units) <= caps.day
    }

    fn spend(&mut self, now: Instant, units: u32) {
        self.minute.spend(now, MINUTE, units);
        self.hour.spend(now, HOUR, units);
        self.day.spend(now, DAY, units);
    }

    /// A proven start was no guess: its hour and day come back. The
    /// minute stays spent, since the evaluation happened.
    fn give_back(&mut self, at: Instant, units: u32) {
        self.hour.give_back(at, units);
        self.day.give_back(at, units);
    }

    /// Whether nothing is left to remember.
    fn passed(self, now: Instant) -> bool {
        self.minute.used(now, MINUTE) == 0
            && self.hour.used(now, HOUR) == 0
            && self.day.used(now, DAY) == 0
    }
}

/// Tallies by key, at most [`ENTRIES`] of them.
struct Table<K> {
    tallies: BTreeMap<K, Tally>,
    /// When the passed ones were last forgotten.
    swept: Option<Instant>,
}

impl<K> Default for Table<K> {
    fn default() -> Self {
        Self {
            tallies: BTreeMap::new(),
            swept: None,
        }
    }
}

impl<K: Ord + Copy> Table<K> {
    fn fits(&self, key: &K, now: Instant, caps: Caps, units: u32) -> bool {
        self.tallies
            .get(key)
            .copied()
            .unwrap_or_else(|| Tally::new(now))
            .fits(now, caps, units)
    }

    /// The key's tally, made if it is new. Passed tallies are forgotten
    /// once an hour, and when a newcomer finds the table full (then once
    /// a minute at most); if it is full of live ones, the one whose day
    /// began first makes room.
    fn entry(&mut self, key: K, now: Instant) -> &mut Tally {
        let known = self.tallies.contains_key(&key);
        let full = !known && self.tallies.len() >= ENTRIES;
        let since = self.swept.map(|swept| now.saturating_duration_since(swept));
        if since.is_none_or(|since| since >= HOUR || (full && since >= MINUTE)) {
            self.tallies.retain(|_, tally| !tally.passed(now));
            self.swept = Some(now);
        }
        if !known && self.tallies.len() >= ENTRIES {
            let oldest = self
                .tallies
                .iter()
                .min_by_key(|(_, tally)| tally.day.start)
                .map(|(key, _)| *key);
            if let Some(oldest) = oldest {
                self.tallies.remove(&oldest);
            }
        }
        self.tallies.entry(key).or_insert_with(|| Tally::new(now))
    }

    fn spend(&mut self, key: K, now: Instant, units: u32) {
        self.entry(key, now).spend(now, units);
    }

    fn give_back(&mut self, key: &K, at: Instant, units: u32) {
        if let Some(tally) = self.tallies.get_mut(key) {
            tally.give_back(at, units);
        }
    }
}

/// The places accounts proved their password from, each until its end.
#[derive(Default)]
struct Own(BTreeMap<[u8; 16], Vec<(Place, Instant)>>);

impl Own {
    fn is(&self, account: [u8; 16], place: Place, now: Instant) -> bool {
        self.0.get(&account).is_some_and(|places| {
            places
                .iter()
                .any(|(known, until)| *known == place && *until > now)
        })
    }

    /// `place` is `account`'s own from `now` on. An account keeps
    /// [`OWN_PER_ACCOUNT`] places and gives up its own soonest-ending one
    /// for another; only a table full of accounts makes one of them, the
    /// one whose last place ends first, give up all of its.
    fn mark(&mut self, account: [u8; 16], place: Place, now: Instant) {
        let until = now
            .checked_add(Duration::from_secs(OWN_DAYS * 86_400))
            .unwrap_or(now);
        if !self.0.contains_key(&account) && self.0.len() >= ENTRIES {
            self.0.retain(|_, places| {
                places.retain(|(_, until)| *until > now);
                !places.is_empty()
            });
        }
        if !self.0.contains_key(&account) && self.0.len() >= ENTRIES {
            let first = self
                .0
                .iter()
                .min_by_key(|(_, places)| places.iter().map(|(_, until)| *until).max())
                .map(|(account, _)| *account);
            if let Some(first) = first {
                self.0.remove(&first);
            }
        }
        let places = self.0.entry(account).or_default();
        places.retain(|(known, until)| *known != place && *until > now);
        if places.len() >= OWN_PER_ACCOUNT {
            let first = places
                .iter()
                .enumerate()
                .min_by_key(|(_, (_, until))| *until)
                .map(|(index, _)| index);
            if let Some(first) = first {
                places.swap_remove(first);
            }
        }
        places.push((place, until));
    }
}

/// Whose password a guess is at.
#[derive(Clone, Copy)]
enum Whose {
    /// An account with a credential.
    Account([u8; 16]),
    /// An id that is nobody's, or an account without a credential for the
    /// epoch asked: all of them together count as one account, so ids that
    /// are made up take no entries and the same refusals meet them.
    Nobody([u8; 16]),
    /// A name of the legacy store, which counts its own names
    /// (`limits.rs`, the store's lockout): here only the place counts.
    Legacy,
}

impl Whose {
    const fn id(self) -> Option<[u8; 16]> {
        match self {
            Self::Account(id) | Self::Nobody(id) => Some(id),
            Self::Legacy => None,
        }
    }
}

/// A guess as it was counted, kept with the pending attempt for
/// [`Guesses::proved`].
pub(super) struct Guess {
    whose: Whose,
    place: Option<Place>,
    /// Whether the place was the account's own when it was counted.
    own: bool,
    units: u32,
    at: Instant,
}

/// Every count of guesses.
#[derive(Default)]
pub(super) struct Guesses {
    accounts: Table<[u8; 16]>,
    nobody: Option<Tally>,
    pairs: Table<([u8; 16], Place)>,
    places: Table<Net>,
    own: Own,
}

impl Guesses {
    /// Whether `units` guesses at `account` from `place` fit, as far as
    /// that shows before the store is asked: checked before a worker or a
    /// statement is taken, so a flood of refused starts costs neither.
    pub(super) fn fits(
        &self,
        account: [u8; 16],
        place: Option<Place>,
        now: Instant,
        units: u32,
    ) -> Result<(), Error> {
        self.weigh(Whose::Account(account), place, now, units)
            .map(|_| ())
    }

    /// Count `units` guesses at `account` from `place`; `known` says
    /// whether the store has a credential for it. Nothing is counted when
    /// any budget is spent.
    pub(super) fn guess(
        &mut self,
        account: [u8; 16],
        known: bool,
        place: Option<Place>,
        now: Instant,
        units: u32,
    ) -> Result<Guess, Error> {
        let whose = if known {
            Whose::Account(account)
        } else {
            Whose::Nobody(account)
        };
        self.count(whose, place, now, units)
    }

    /// Whether one guess at a legacy name from `place` fits.
    pub(super) fn legacy_fits(&self, place: Option<Place>, now: Instant) -> Result<(), Error> {
        self.weigh(Whose::Legacy, place, now, 1).map(|_| ())
    }

    /// Count one guess at a legacy name from `place`.
    pub(super) fn legacy(&mut self, place: Option<Place>, now: Instant) -> Result<Guess, Error> {
        self.count(Whose::Legacy, place, now, 1)
    }

    /// Spend `units` of `account`'s evaluations a minute and nothing of
    /// its longer windows (`Limits::account`).
    pub(super) fn evaluate(
        &mut self,
        account: [u8; 16],
        now: Instant,
        units: u32,
    ) -> Result<(), Error> {
        if !self.accounts.fits(&account, now, EVALUATION, units) {
            return Err(Error::Busy);
        }
        self.accounts
            .entry(account, now)
            .minute
            .spend(now, MINUTE, units);
        Ok(())
    }

    /// The guess was none: the password was proven. Its hour and day come
    /// back wherever they were counted, and the place is the account's own
    /// from now on.
    pub(super) fn proved(&mut self, guess: &Guess, now: Instant) {
        let (at, units) = (guess.at, guess.units);
        if let (Some(account), Some(place)) = (guess.whose.id(), guess.place) {
            self.pairs.give_back(&(account, place), at, units);
        }
        if !guess.own {
            if let Some(place) = guess.place {
                self.places.give_back(&Net::Place(place), at, units);
                if let Some(wide) = place.wide() {
                    self.places.give_back(&Net::Wide(wide), at, units);
                }
            }
            if let Whose::Account(account) = guess.whose {
                self.accounts.give_back(&account, at, units);
            }
        }
        if let (Whose::Account(account), Some(place)) = (guess.whose, guess.place) {
            self.own.mark(account, place, now);
        }
    }

    fn count(
        &mut self,
        whose: Whose,
        place: Option<Place>,
        now: Instant,
        units: u32,
    ) -> Result<Guess, Error> {
        let own = self.weigh(whose, place, now, units)?;
        if let (Some(account), Some(place)) = (whose.id(), place) {
            self.pairs.spend((account, place), now, units);
        }
        if !own {
            if let Some(place) = place {
                self.places.spend(Net::Place(place), now, units);
                if let Some(wide) = place.wide() {
                    self.places.spend(Net::Wide(wide), now, units);
                }
            }
            match whose {
                Whose::Account(account) => self.accounts.spend(account, now, units),
                Whose::Nobody(_) => self
                    .nobody
                    .get_or_insert_with(|| Tally::new(now))
                    .spend(now, units),
                Whose::Legacy => {}
            }
        }
        Ok(Guess {
            whose,
            place,
            own,
            units,
            at: now,
        })
    }

    /// Whether the guess fits every budget it draws on, and whether the
    /// place is the account's own. An own place draws on its own count at
    /// the account and on nothing else.
    fn weigh(
        &self,
        whose: Whose,
        place: Option<Place>,
        now: Instant,
        units: u32,
    ) -> Result<bool, Error> {
        let fits = |fits: bool| if fits { Ok(()) } else { Err(Error::Busy) };
        if let (Some(account), Some(place)) = (whose.id(), place) {
            fits(self.pairs.fits(&(account, place), now, PAIR, units))?;
        }
        if let (Whose::Account(account), Some(place)) = (whose, place)
            && self.own.is(account, place, now)
        {
            return Ok(true);
        }
        if let Some(place) = place {
            fits(self.places.fits(&Net::Place(place), now, PLACE, units))?;
            if let Some(wide) = place.wide() {
                fits(self.places.fits(&Net::Wide(wide), now, WIDE, units))?;
            }
        }
        match whose {
            Whose::Account(account) => {
                fits(self.accounts.fits(&account, now, ACCOUNT, units))?;
            }
            Whose::Nobody(_) => fits(
                self.nobody
                    .is_none_or(|tally| tally.fits(now, ACCOUNT, units)),
            )?,
            Whose::Legacy => {}
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VICTIM: [u8; 16] = [7; 16];

    fn v4(n: u32) -> Place {
        Place::V4(n.to_be_bytes())
    }

    /// The `n`th /64 of one /48.
    fn v6(n: u16) -> Place {
        let [high, low] = n.to_be_bytes();
        Place::V6([0x20, 0x01, 0x0d, 0xb8, 0, 1, high, low])
    }

    fn account(n: u32) -> [u8; 16] {
        let mut id = [0xa0; 16];
        id[12..].copy_from_slice(&n.to_be_bytes());
        id
    }

    fn after(start: Instant, seconds: u64) -> Instant {
        start + Duration::from_secs(seconds)
    }

    /// Guesses at `VICTIM`, each from a place of its own, as fast as the
    /// minute allows from `start` on; how many were counted within `seconds`.
    fn from_everywhere(guesses: &mut Guesses, start: Instant, seconds: u64, first: u32) -> u32 {
        let mut counted = 0;
        for (place, second) in (first..).zip((0..seconds).step_by(5)) {
            let now = after(start, second);
            if guesses.guess(VICTIM, true, Some(v4(place)), now, 1).is_ok() {
                counted += 1;
            }
        }
        counted
    }

    #[test]
    fn all_the_world_guesses_one_password_thirty_times_an_hour_and_a_hundred_a_day() {
        let start = Instant::now();
        let mut guesses = Guesses::default();
        // Every guess from another address, twelve tried a minute.
        assert_eq!(
            from_everywhere(&mut guesses, start, 3_600, 0),
            ACCOUNT_PER_HOUR
        );
        let mut day = ACCOUNT_PER_HOUR;
        for hour in 1..24 {
            day += from_everywhere(&mut guesses, after(start, hour * 3_600), 3_600, 100_000);
        }
        assert_eq!(day, ACCOUNT_PER_DAY);
        // The next day has its hundred again.
        assert!(
            guesses
                .guess(VICTIM, true, Some(v4(9)), after(start, 86_400), 1)
                .is_ok()
        );
        // Another account was never touched by any of it.
        assert!(
            guesses
                .guess(account(1), true, Some(v4(1_000_000)), after(start, 600), 1)
                .is_ok()
        );
    }

    #[test]
    fn no_more_than_eight_evaluations_leave_in_a_minute() {
        let now = Instant::now();
        let mut guesses = Guesses::default();
        for place in 0..ACCOUNT_PER_MINUTE {
            guesses
                .guess(VICTIM, true, Some(v4(place)), now, 1)
                .unwrap();
        }
        assert!(guesses.guess(VICTIM, true, Some(v4(99)), now, 1).is_err());
        assert!(guesses.fits(VICTIM, Some(v4(99)), now, 1).is_err());
        // A password change's two do not fit the last unit, and spend nothing.
        let mut guesses = Guesses::default();
        for _ in 0..3 {
            guesses.guess(VICTIM, true, None, now, 2).unwrap();
        }
        guesses.guess(VICTIM, true, None, now, 1).unwrap();
        assert!(guesses.guess(VICTIM, true, None, now, 2).is_err());
        guesses.guess(VICTIM, true, None, now, 1).unwrap();
        assert!(guesses.guess(VICTIM, true, None, now, 1).is_err());
        guesses
            .guess(VICTIM, true, None, after(now, 60), 2)
            .unwrap();
    }

    #[test]
    fn one_place_spends_a_third_of_an_accounts_hour_and_no_more() {
        let start = Instant::now();
        let mut guesses = Guesses::default();
        let mut counted = 0;
        for minute in 0..60 {
            let now = after(start, minute * 60);
            if guesses.guess(VICTIM, true, Some(v4(1)), now, 1).is_ok() {
                counted += 1;
            }
        }
        assert_eq!(counted, PAIR_PER_HOUR);
        // Refused there, and nothing more was spent of the account's hour.
        let now = after(start, 3_599);
        assert!(guesses.guess(VICTIM, true, Some(v4(1)), now, 1).is_err());
        for place in 2..2 + (ACCOUNT_PER_HOUR - PAIR_PER_HOUR) {
            let now = after(start, 1_800 + u64::from(place) * 10);
            guesses
                .guess(VICTIM, true, Some(v4(place)), now, 1)
                .unwrap();
        }
        // Over the day one place gets thirty.
        let mut guesses = Guesses::default();
        let mut counted = 0;
        for minute in 0..1_440 {
            let now = after(start, minute * 60);
            if guesses.guess(VICTIM, true, Some(v4(1)), now, 1).is_ok() {
                counted += 1;
            }
        }
        assert_eq!(counted, PAIR_PER_DAY);
    }

    #[test]
    fn one_place_tries_thirty_accounts_an_hour_and_a_hundred_a_day() {
        let start = Instant::now();
        let mut guesses = Guesses::default();
        let mut counted = 0;
        for n in 0..1_000 {
            let now = after(start, u64::from(n) * 3);
            if guesses.guess(account(n), true, Some(v4(1)), now, 1).is_ok() {
                counted += 1;
            }
        }
        assert_eq!(counted, PLACE_PER_HOUR);
        let mut counted = 0;
        for n in 0..10_000 {
            let now = after(start, u64::from(n) * 8);
            if guesses
                .guess(account(n), n % 2 == 0, Some(v4(2)), now, 1)
                .is_ok()
            {
                counted += 1;
            }
        }
        assert_eq!(counted, PLACE_PER_DAY, "ids that are nobody's count too");
        // Another address is not kept out by it.
        assert!(
            guesses
                .guess(account(1), true, Some(v4(3)), start, 1)
                .is_ok()
        );
    }

    #[test]
    fn the_64s_of_one_48_share_four_places_budgets() {
        let start = Instant::now();
        let mut guesses = Guesses::default();
        let mut counted = 0;
        for n in 0..2_000_u16 {
            let now = after(start, u64::from(n));
            if guesses
                .guess(account(u32::from(n)), true, Some(v6(n)), now, 1)
                .is_ok()
            {
                counted += 1;
            }
        }
        assert_eq!(counted, WIDE_PER_HOUR);
        let mut counted = WIDE_PER_HOUR;
        for n in 2_000..20_000_u16 {
            let now = after(start, 3_600 + u64::from(n - 2_000) * 4);
            if guesses
                .guess(account(u32::from(n)), true, Some(v6(n)), now, 1)
                .is_ok()
            {
                counted += 1;
            }
        }
        assert_eq!(counted, WIDE_PER_DAY);
        // Another /48, and an IPv4 address, have their own.
        let elsewhere = Some(Place::V6([0x20, 0x01, 0x0d, 0xb8, 0, 2, 0, 0]));
        assert!(guesses.guess(account(1), true, elsewhere, start, 1).is_ok());
        assert!(
            guesses
                .guess(account(1), true, Some(v4(1)), start, 1)
                .is_ok()
        );
    }

    #[test]
    fn a_proven_start_was_no_guess() {
        let start = Instant::now();
        let mut guesses = Guesses::default();
        // Far more sign-ins than any budget, each proven: none is counted,
        // at the place before it is the account's own and after.
        for n in 0..500 {
            let now = after(start, n * 10);
            let guess = guesses.guess(VICTIM, true, Some(v4(1)), now, 1).unwrap();
            guesses.proved(&guess, now);
        }
        // Without a place the account's own windows give them back.
        for n in 0..500 {
            let now = after(start, n * 10);
            let guess = guesses.guess(account(1), true, None, now, 1).unwrap();
            guesses.proved(&guess, now);
        }
        // A wrong one in between stays counted: ten an hour at the place.
        let mut guesses = Guesses::default();
        let mut wrong = 0;
        for n in 0..60 {
            let now = after(start, n * 60);
            if guesses.guess(VICTIM, true, Some(v4(1)), now, 1).is_ok() {
                wrong += 1;
            }
            if let Ok(guess) = guesses.guess(account(1), true, Some(v4(1)), now, 1) {
                guesses.proved(&guess, now);
            }
        }
        assert_eq!(
            wrong, PAIR_PER_HOUR,
            "another account's proof gives none back"
        );
    }

    #[test]
    fn what_others_guess_does_not_keep_the_holder_out_at_an_own_place() {
        let start = Instant::now();
        let mut guesses = Guesses::default();
        let home = Some(v4(1));
        let guess = guesses.guess(VICTIM, true, home, start, 1).unwrap();
        guesses.proved(&guess, start);
        // The world spends the account's day.
        let mut spent = 0;
        for hour in 0..24 {
            spent += from_everywhere(&mut guesses, after(start, 60 + hour * 3_600), 3_540, 10);
        }
        assert_eq!(spent, ACCOUNT_PER_DAY);
        let now = after(start, 86_000);
        assert!(guesses.fits(VICTIM, Some(v4(5_000_000)), now, 1).is_err());
        assert!(
            guesses
                .guess(VICTIM, true, Some(v4(5_000_000)), now, 1)
                .is_err()
        );
        // The holder signs in at home all the same, and may mistype there.
        let guess = guesses.guess(VICTIM, true, home, now, 1).unwrap();
        guesses.proved(&guess, now);
        for _ in 0..PAIR_PER_HOUR {
            guesses.guess(VICTIM, true, home, now, 1).unwrap();
        }
        // An own place has its own bound: whoever sits there guesses no
        // faster than at any other.
        assert!(guesses.guess(VICTIM, true, home, now, 1).is_err());
        // It is no own place of another account.
        let mut guesses = Guesses::default();
        let guess = guesses.guess(VICTIM, true, home, start, 1).unwrap();
        guesses.proved(&guess, start);
        for n in 0..PLACE_PER_HOUR {
            guesses.guess(account(n), true, home, start, 1).unwrap();
        }
        assert!(guesses.guess(account(999), true, home, start, 1).is_err());
        assert!(guesses.guess(VICTIM, true, home, start, 1).is_ok());
    }

    #[test]
    fn an_own_place_ends_and_an_account_keeps_eight() {
        let start = Instant::now();
        let mut guesses = Guesses::default();
        // Six sign-ins a minute, each from another place and proven.
        for place in 0..9 {
            let now = after(start, u64::from(place) * 10);
            let guess = guesses
                .guess(VICTIM, true, Some(v4(place)), now, 1)
                .unwrap();
            guesses.proved(&guess, now);
        }
        // The ninth took the place of the first; the others stayed.
        let soon = after(start, 90);
        assert!(!guesses.own.is(VICTIM, v4(0), soon));
        for place in 1..9 {
            assert!(guesses.own.is(VICTIM, v4(place), soon));
        }
        // The last, proven after 80 seconds, ends thirty days after that.
        let last = after(start, OWN_DAYS * 86_400);
        assert!(guesses.own.is(VICTIM, v4(8), after(last, 79)));
        assert!(!guesses.own.is(VICTIM, v4(8), after(last, 80)));
        // A proof again begins the days anew.
        let now = after(start, 86_400);
        let guess = guesses.guess(VICTIM, true, Some(v4(8)), now, 1).unwrap();
        guesses.proved(&guess, now);
        assert!(guesses.own.is(VICTIM, v4(8), after(last, 80)));
        // One account's places never push out another's.
        let other = account(1);
        let guess = guesses.guess(other, true, Some(v4(77)), start, 1).unwrap();
        guesses.proved(&guess, start);
        for place in 100..400 {
            // Six a minute: a proof gives the hour and the day back, and
            // the minute's evaluations stay spent.
            let now = after(start, 100_000 + u64::from(place) * 10);
            let guess = guesses
                .guess(VICTIM, true, Some(v4(place)), now, 1)
                .unwrap();
            guesses.proved(&guess, now);
        }
        assert!(
            guesses
                .own
                .is(other, Place::V4([0, 0, 0, 77]), after(start, 110_000))
        );
        assert_eq!(
            guesses.own.0.get(&VICTIM).map(Vec::len),
            Some(OWN_PER_ACCOUNT)
        );
    }

    #[test]
    fn made_up_ids_take_no_entries_and_meet_one_accounts_budget() {
        let now = Instant::now();
        let mut guesses = Guesses::default();
        for n in 0..ACCOUNT_PER_MINUTE {
            // Before the store answered, an id shows nothing.
            guesses.fits(account(n), None, now, 1).unwrap();
            guesses.guess(account(n), false, None, now, 1).unwrap();
        }
        assert!(guesses.guess(account(99), false, None, now, 1).is_err());
        assert!(guesses.accounts.tallies.is_empty());
        // Real accounts are not kept out by it.
        guesses.guess(VICTIM, true, None, now, 1).unwrap();
        // A proof never comes for one, and marks no place.
        let guess = guesses.guess(account(1), false, Some(v4(1)), after(now, 60), 1);
        assert!(guess.is_ok());
        assert!(guesses.own.0.is_empty());
    }

    #[test]
    fn full_tables_forget_their_oldest_and_refuse_nobody() {
        let start = Instant::now();
        let mut places = Table::<u32>::default();
        let over = u32::try_from(ENTRIES).unwrap() + 100;
        for n in 0..over {
            assert!(places.fits(&n, start, PLACE, 1));
            places.spend(n, after(start, u64::from(n / 100)), 1);
            assert!(places.tallies.len() <= ENTRIES);
        }
        // The first hundred, the oldest, made room; the newest are counted.
        assert!(!places.tallies.contains_key(&0));
        assert!(places.tallies.contains_key(&(over - 1)));
        // Once their windows passed, all are forgotten at the next newcomer.
        places.spend(over, after(start, 2 * 86_400), 1);
        assert_eq!(places.tallies.len(), 1);
        // Own places: a table full of accounts gives up the one whose last
        // place ends first.
        let mut own = Own::default();
        for n in 0..=u32::try_from(ENTRIES).unwrap() {
            own.mark(
                account(n),
                Place::V4([1, 1, 1, 1]),
                after(start, u64::from(n)),
            );
        }
        assert_eq!(own.0.len(), ENTRIES);
        assert!(!own.0.contains_key(&account(0)));
        assert!(own.0.contains_key(&account(1)));
    }

    #[test]
    fn a_refusal_spends_nothing_anywhere() {
        let now = Instant::now();
        let mut guesses = Guesses::default();
        // The place's hour is spent on other accounts.
        for n in 0..PLACE_PER_HOUR {
            guesses
                .guess(account(n), true, Some(v4(1)), now, 1)
                .unwrap();
        }
        assert!(guesses.guess(VICTIM, true, Some(v4(1)), now, 1).is_err());
        assert!(!guesses.accounts.tallies.contains_key(&VICTIM));
        assert!(
            !guesses
                .pairs
                .tallies
                .contains_key(&(VICTIM, Place::V4([0, 0, 0, 1])))
        );
        // The account's minute is spent from elsewhere.
        for place in 10..10 + ACCOUNT_PER_MINUTE {
            guesses
                .guess(VICTIM, true, Some(v4(place)), now, 1)
                .unwrap();
        }
        assert!(guesses.guess(VICTIM, true, Some(v4(2)), now, 1).is_err());
        assert!(
            !guesses
                .places
                .tallies
                .contains_key(&Net::Place(Place::V4([0, 0, 0, 2])))
        );
    }

    #[test]
    fn a_later_window_gets_nothing_back() {
        let start = Instant::now();
        let mut guesses = Guesses::default();
        // A start just before the hour ends, proven just after it: the new
        // hour's own wrong guess stays counted.
        for n in 0..PAIR_PER_HOUR - 1 {
            let now = after(start, u64::from(n) * 10);
            guesses.guess(VICTIM, true, Some(v4(1)), now, 1).unwrap();
        }
        let late = guesses
            .guess(VICTIM, true, Some(v4(1)), after(start, 3_590), 1)
            .unwrap();
        let next = after(start, 3_601);
        guesses.guess(VICTIM, true, Some(v4(1)), next, 1).unwrap();
        guesses.proved(&late, next);
        let pair = guesses.pairs.tallies[&(VICTIM, Place::V4([0, 0, 0, 1]))];
        assert_eq!(pair.hour.used(next, HOUR), 1);
        assert_eq!(pair.day.used(next, DAY), PAIR_PER_HOUR);
    }

    #[test]
    fn a_resets_start_spends_the_minute_only() {
        let now = Instant::now();
        let mut guesses = Guesses::default();
        for _ in 0..ACCOUNT_PER_MINUTE {
            guesses.evaluate(VICTIM, now, 1).unwrap();
        }
        assert!(guesses.evaluate(VICTIM, now, 1).is_err());
        // It shares the minute with the sign-ins.
        assert!(guesses.guess(VICTIM, true, None, now, 1).is_err());
        // An account whose day is spent by guessers still takes a reset.
        let mut guesses = Guesses::default();
        let mut spent = 0;
        for hour in 0..24 {
            spent += from_everywhere(&mut guesses, after(now, hour * 3_600), 3_600, 0);
        }
        assert_eq!(spent, ACCOUNT_PER_DAY);
        let later = after(now, 86_000);
        assert!(guesses.guess(VICTIM, true, None, later, 1).is_err());
        guesses.evaluate(VICTIM, later, 1).unwrap();
        // And no reset spends the day.
        let mut guesses = Guesses::default();
        for minute in 0..200 {
            guesses
                .evaluate(VICTIM, after(now, minute * 60), 1)
                .unwrap();
        }
        guesses
            .guess(VICTIM, true, None, after(now, 200 * 60), 1)
            .unwrap();
    }

    #[test]
    fn a_legacy_guess_counts_at_its_place_only() {
        let now = Instant::now();
        let mut guesses = Guesses::default();
        for _ in 0..PLACE_PER_HOUR {
            guesses.legacy(Some(v4(1)), now).unwrap();
        }
        assert!(guesses.legacy(Some(v4(1)), now).is_err());
        // The same place's budget as the other sign-ins'.
        assert!(guesses.guess(VICTIM, true, Some(v4(1)), now, 1).is_err());
        assert!(guesses.legacy(Some(v4(2)), now).is_ok());
        // Proven, it comes back; without a place nothing is counted.
        let mut guesses = Guesses::default();
        for _ in 0..200 {
            let guess = guesses.legacy(Some(v4(1)), now).unwrap();
            guesses.proved(&guess, now);
            guesses.legacy(None, now).unwrap();
        }
        assert!(guesses.accounts.tallies.is_empty() && guesses.own.0.is_empty());
    }
}
