//! Registration's proof of work (`docs/registration-abuse.md`): Central
//! issues a random nonce, keeps it in memory for five minutes, and takes a
//! solution once, no sooner than three seconds after it was issued. The
//! difficulty grows by one bit (twice the work) for each doubling of the
//! last hour's registrations above a baseline, by three bits at most, so a
//! flood gets expensive while registration stays open. Nothing about the
//! person is kept: the nonce is random and the counts are counts. This
//! crate logs nothing; the difficulty a challenge carries shows the rise.

use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use central_api::register::{self, MAX_DIFFICULTY};
use central_api::{Error, Token};

/// How long a challenge waits for its solution.
pub(crate) const LIFETIME: Duration = Duration::from_secs(300);
/// The least time between a challenge and its solution: a form filled in
/// faster than this was not filled in by a person.
pub(crate) const MINIMUM: Duration = Duration::from_secs(3);
/// Challenges held at once. Its own per-address budget limits who fills it;
/// when full the oldest makes room, so a crowd cannot lock others out.
pub(crate) const HELD: usize = 10_000;
/// The difficulty when registrations are few: about two seconds of a
/// mid-range phone's JavaScript for all parts together.
pub(crate) const BASE: u8 = 16;
/// Registrations an hour that cost no more than the base.
pub(crate) const BASELINE: usize = 30;
/// Bits the difficulty may rise at most.
pub(crate) const RISE: u8 = 3;
const HOUR: Duration = Duration::from_secs(3_600);
const _: () = assert!(BASE + RISE <= MAX_DIFFICULTY);

/// An issued challenge.
pub(crate) struct Issued {
    /// The nonce, 32 random bytes in hex.
    pub nonce: String,
    /// Leading zero bits each part needs.
    pub difficulty: u8,
    /// When it ends, Unix seconds.
    pub expires_at: i64,
}

struct Held {
    at: Instant,
    difficulty: u8,
}

/// The challenges issued and the last hour's registrations.
pub(crate) struct Challenges {
    base: u8,
    minimum: Duration,
    held: BTreeMap<String, Held>,
    registrations: VecDeque<Instant>,
}

impl Default for Challenges {
    fn default() -> Self {
        Challenges::new(BASE, MINIMUM)
    }
}

fn unix(at: SystemTime) -> i64 {
    at.duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_secs()).ok())
        .unwrap_or(0)
}

/// The rise for `count` registrations in the last hour: one bit for each
/// doubling above the baseline, three at most.
pub(crate) fn rise(count: usize) -> u8 {
    let mut bits = 0;
    let mut threshold = BASELINE * 2;
    while count >= threshold && bits < RISE {
        bits += 1;
        threshold *= 2;
    }
    bits
}

impl Challenges {
    /// With `base` bits when registrations are few and `minimum` before a
    /// solution counts (tests ask little).
    pub(crate) fn new(base: u8, minimum: Duration) -> Challenges {
        Challenges {
            base: base.min(MAX_DIFFICULTY - RISE),
            minimum,
            held: BTreeMap::new(),
            registrations: VecDeque::new(),
        }
    }

    fn forget(&mut self, now: Instant) {
        self.held.retain(|_, h| now.duration_since(h.at) < LIFETIME);
        while self
            .registrations
            .front()
            .is_some_and(|at| now.duration_since(*at) >= HOUR)
        {
            self.registrations.pop_front();
        }
    }

    /// The difficulty now.
    fn difficulty(&mut self, now: Instant) -> u8 {
        self.forget(now);
        self.base + rise(self.registrations.len())
    }

    /// Issues a challenge.
    ///
    /// # Errors
    /// `Unavailable` without randomness.
    pub(crate) fn issue(&mut self, now: Instant) -> Result<Issued, Error> {
        let difficulty = self.difficulty(now);
        if self.held.len() >= HELD {
            let oldest = self
                .held
                .iter()
                .min_by_key(|(_, h)| h.at)
                .map(|(nonce, _)| nonce.clone());
            if let Some(oldest) = oldest {
                self.held.remove(&oldest);
            }
        }
        let nonce = Token::generate()?.hex().to_string();
        self.held.insert(
            nonce.clone(),
            Held {
                at: now,
                difficulty,
            },
        );
        Ok(Issued {
            nonce,
            difficulty,
            expires_at: unix(SystemTime::now() + LIFETIME),
        })
    }

    /// Takes the solution of `nonce`, once, whatever it is: a challenge it
    /// does not know, one too young or too old, or counters that do not
    /// solve it are `Rejected`.
    ///
    /// # Errors
    /// `Rejected`.
    pub(crate) fn spend(
        &mut self,
        nonce: &str,
        counters: &[u64],
        now: Instant,
    ) -> Result<(), Error> {
        self.forget(now);
        let held = self.held.remove(nonce).ok_or(Error::Rejected)?;
        let age = now.duration_since(held.at);
        if age < self.minimum || !register::solved(nonce, held.difficulty, counters) {
            return Err(Error::Rejected);
        }
        Ok(())
    }

    /// Counts a registration that went through.
    pub(crate) fn registered(&mut self, now: Instant) {
        self.forget(now);
        self.registrations.push_back(now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rises_with_each_doubling() {
        assert_eq!(rise(0), 0);
        assert_eq!(rise(59), 0);
        assert_eq!(rise(60), 1);
        assert_eq!(rise(120), 2);
        assert_eq!(rise(240), 3);
        assert_eq!(rise(100_000), RISE);
    }

    #[test]
    fn a_solution_counts_once_and_not_too_soon() {
        let mut challenges = Challenges::new(4, MINIMUM);
        let start = Instant::now();
        let issued = challenges.issue(start).expect("issued");
        assert_eq!(issued.difficulty, 4);
        assert!(register::valid_nonce(&issued.nonce));
        let counters =
            register::solve(&issued.nonce, register::PARTS, issued.difficulty).expect("solved");
        assert_eq!(
            challenges.spend(&issued.nonce, &counters, start + Duration::from_secs(1)),
            Err(Error::Rejected),
            "too soon"
        );
        let again = challenges.issue(start).expect("issued");
        let counters =
            register::solve(&again.nonce, register::PARTS, again.difficulty).expect("solved");
        let later = start + MINIMUM;
        assert_eq!(challenges.spend(&again.nonce, &counters, later), Ok(()));
        assert_eq!(
            challenges.spend(&again.nonce, &counters, later),
            Err(Error::Rejected),
            "once"
        );
        let old = challenges.issue(start).expect("issued");
        let counters =
            register::solve(&old.nonce, register::PARTS, old.difficulty).expect("solved");
        assert_eq!(
            challenges.spend(&old.nonce, &counters, start + LIFETIME),
            Err(Error::Rejected),
            "expired"
        );
        assert_eq!(
            challenges.spend("unknown", &[], later),
            Err(Error::Rejected)
        );
    }

    #[test]
    fn registrations_raise_the_difficulty_and_the_hour_lowers_it() {
        let mut challenges = Challenges::default();
        let start = Instant::now();
        assert_eq!(challenges.issue(start).expect("issued").difficulty, BASE);
        for _ in 0..BASELINE * 2 {
            challenges.registered(start);
        }
        assert_eq!(
            challenges.issue(start).expect("issued").difficulty,
            BASE + 1
        );
        assert_eq!(
            challenges.issue(start + HOUR).expect("issued").difficulty,
            BASE
        );
    }

    #[test]
    fn a_full_table_makes_room() {
        let mut challenges = Challenges::new(4, MINIMUM);
        let start = Instant::now();
        let first = challenges.issue(start).expect("issued");
        for _ in 1..HELD {
            challenges
                .issue(start + Duration::from_millis(1))
                .expect("issued");
        }
        challenges
            .issue(start + Duration::from_millis(2))
            .expect("issued");
        assert_eq!(challenges.held.len(), HELD);
        assert!(
            !challenges.held.contains_key(&first.nonce),
            "the oldest went"
        );
    }
}
