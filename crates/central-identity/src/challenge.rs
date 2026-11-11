//! Registration's proof of work (`docs/registration-abuse.md`): Central
//! issues a random nonce, keeps it in memory with the difficulty it asked
//! for, and takes a solution once, no sooner than three seconds after it
//! was issued and checked against that difficulty alone.
//!
//! The difficulty grows two ways, and never past six bits over the base
//! (64 times the work), so no person is locked out. A flood adds one bit
//! (twice the work) for each doubling of the last hour's registrations
//! above a baseline, six at most, and the hour takes it back. A request
//! that looks automated adds up to four ([`points`]): what the browser
//! listener saw of its headers and of its address's haste ([`Seen`]), and
//! what the page says of its own browser. A challenge lives five minutes,
//! and twice as long for each bit past four, so the device that needs
//! longest still hands its solution in.
//!
//! Nothing about the person is kept: the nonce is random, the counts are
//! counts, and of what was seen only the difficulty stays, until the
//! challenge ends. This crate logs nothing.

use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use central_api::register::{self, MAX_DIFFICULTY, SIGNAL_NO_LANGUAGES, SIGNAL_WEBDRIVER};
use central_api::{Error, Token};

use super::limits::GLOBAL_PER_SECOND;

/// How long a challenge of up to [`PATIENT`] bits over the base waits for
/// its solution; a harder one waits longer ([`lifetime`]).
pub(crate) const LIFETIME: Duration = Duration::from_secs(300);
/// The least time between a challenge and its solution: a form filled in
/// faster than this was not filled in by a person.
pub(crate) const MINIMUM: Duration = Duration::from_secs(3);
/// Challenges held at once: more than the global budget lets all callers
/// together ask for within the longest lifetime (asserted below), so
/// however many ask, no challenge makes room for another before its own
/// time is over.
pub(crate) const HELD: usize = 40_000;
/// The difficulty when registrations are few: about two seconds of a
/// mid-range phone's JavaScript for all parts together.
pub(crate) const BASE: u8 = 16;
/// Registrations an hour that cost no more than the base.
pub(crate) const BASELINE: usize = 30;
/// Bits a flood adds at most: from 1,920 registrations an hour on.
pub(crate) const RISE: u8 = 6;
/// Bits a request's own look adds at most.
pub(crate) const SUSPICION: u8 = 4;
/// Bits over the base at most, the flood's and the look's together: about
/// two minutes of that phone, one of a laptop (measured,
/// `docs/registration-abuse.md` §"Hardened").
pub(crate) const MOST: u8 = 6;
/// Bits over the base a challenge may have and still live [`LIFETIME`].
const PATIENT: u8 = 4;
const HOUR: Duration = Duration::from_secs(3_600);
/// How often the table is searched for challenges whose time is over.
const SWEEP: Duration = Duration::from_secs(1);
const _: () = assert!(BASE + MOST == MAX_DIFFICULTY);
const _: () = assert!(RISE <= MOST && SUSPICION <= MOST && PATIENT <= MOST);
// One more second for the window the global budget counts in.
const _: () = assert!(HELD as u64 > GLOBAL_PER_SECOND as u64 * (lifetime(MOST).as_secs() + 1));

/// How long a challenge of `extra` bits over the base lives: [`LIFETIME`],
/// doubled for each bit past [`PATIENT`], as the work doubles. That is
/// about nine times what the phone of [`BASE`] needs on average.
const fn lifetime(extra: u8) -> Duration {
    Duration::from_secs(LIFETIME.as_secs() << extra.saturating_sub(PATIENT))
}

/// What a request's `User-Agent` says of its sender.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Agent {
    /// Nothing that tells it from a person's browser.
    #[default]
    Browser,
    /// None was sent, or an empty one.
    Missing,
    /// It names a tool, a crawler or a headless browser.
    Tool,
}

/// What the browser listener saw of a request for a registration challenge:
/// plain facts, never a header's text and never the address. The default is
/// a request as a person's browser sends it. Of this only the difficulty of
/// the one challenge is kept, and nothing is logged.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Seen {
    /// No `Accept-Language`, which every browser sends.
    pub no_language: bool,
    /// No `Sec-Fetch-Site`, which every current browser sends.
    pub no_fetch_site: bool,
    /// What its `User-Agent` says.
    pub agent: Agent,
    /// How much the address, or the block of addresses around it, asked
    /// this hour: 0 for a few challenges, up to 3.
    pub frequent: u8,
}

/// The bits a request's look adds, [`SUSPICION`] at most, counted as the
/// Impressum's service counts them (`docs/registration-abuse.md`
/// §"Hardened"). A person's browser scores 0. `signals` is what the page
/// reported of its browser ([`central_api::register::SIGNALS`]); other bits
/// say nothing. Every sign only ever adds work: none refuses.
pub(crate) fn points(seen: Seen, signals: u8) -> u8 {
    let mut points = seen.frequent.min(3);
    points += u8::from(seen.no_language);
    points += u8::from(seen.no_fetch_site);
    if seen.agent != Agent::Browser {
        points += 2;
    }
    if signals & SIGNAL_WEBDRIVER != 0 {
        points += 2;
    }
    if signals & SIGNAL_NO_LANGUAGES != 0 {
        points += 1;
    }
    points.min(SUSPICION)
}

/// An issued challenge.
pub(crate) struct Issued {
    /// The nonce, 32 random bytes in hex.
    pub nonce: String,
    /// Leading zero bits each part needs.
    pub difficulty: u8,
    /// When it ends, Unix seconds.
    pub expires_at: i64,
}

/// A challenge waiting for its solution: when it was issued, when it ends,
/// and the difficulty it was issued with, the only one its solution is
/// checked against.
struct Held {
    at: Instant,
    until: Instant,
    difficulty: u8,
}

/// The challenges issued and the last hour's registrations.
pub(crate) struct Challenges {
    base: u8,
    minimum: Duration,
    held: BTreeMap<String, Held>,
    swept: Option<Instant>,
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
/// doubling above the baseline (60, 120, 240, 480, 960, 1,920), [`RISE`]
/// at most.
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
            base: base.min(MAX_DIFFICULTY - MOST),
            minimum,
            held: BTreeMap::new(),
            swept: None,
            registrations: VecDeque::new(),
        }
    }

    /// Forgets the registrations older than an hour and, once a
    /// [`SWEEP`], the challenges whose time is over (a solution is checked
    /// against its own challenge's end, whenever the table was last swept).
    fn forget(&mut self, now: Instant) {
        while self
            .registrations
            .front()
            .is_some_and(|at| now.duration_since(*at) >= HOUR)
        {
            self.registrations.pop_front();
        }
        if self
            .swept
            .is_none_or(|swept| now.duration_since(swept) >= SWEEP)
        {
            self.sweep(now);
        }
    }

    fn sweep(&mut self, now: Instant) {
        self.held.retain(|_, held| now < held.until);
        self.swept = Some(now);
    }

    /// Issues a challenge for a request that scored `points` ([`points`]):
    /// the base, the flood's rise and the points, [`MOST`] bits over the
    /// base at most.
    ///
    /// # Errors
    /// `Unavailable` without randomness.
    pub(crate) fn issue(&mut self, now: Instant, points: u8) -> Result<Issued, Error> {
        self.forget(now);
        let extra = (rise(self.registrations.len()) + points.min(SUSPICION)).min(MOST);
        let difficulty = self.base + extra;
        let lifetime = lifetime(extra);
        if self.held.len() >= HELD {
            self.sweep(now);
        }
        // Not reached while the global budget holds (asserted above): the
        // oldest makes room, so a crowd cannot lock others out.
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
                until: now + lifetime,
                difficulty,
            },
        );
        Ok(Issued {
            nonce,
            difficulty,
            expires_at: unix(SystemTime::now() + lifetime),
        })
    }

    /// Takes the solution of `nonce`, once, whatever it is: a challenge it
    /// does not know, one too young or too old, or counters that do not
    /// solve it at the difficulty it was issued with are `Rejected`, and
    /// the challenge is gone before any of that is judged.
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
        if now >= held.until
            || now.duration_since(held.at) < self.minimum
            || !register::solved(nonce, held.difficulty, counters)
        {
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

    fn solve(issued: &Issued) -> Vec<u64> {
        register::solve(&issued.nonce, register::PARTS, issued.difficulty).expect("solved")
    }

    #[test]
    fn rises_with_each_doubling() {
        assert_eq!(rise(0), 0);
        assert_eq!(rise(59), 0);
        for (count, bits) in [(60, 1), (120, 2), (240, 3), (480, 4), (960, 5), (1_920, 6)] {
            assert_eq!(rise(count - 1), bits - 1, "{count}");
            assert_eq!(rise(count), bits, "{count}");
        }
        assert_eq!(rise(100_000), RISE);
    }

    #[test]
    fn a_person_s_browser_scores_nothing_and_each_sign_adds() {
        let seen = Seen::default();
        assert_eq!(points(seen, 0), 0);
        let one = |change: fn(&mut Seen)| {
            let mut seen = Seen::default();
            change(&mut seen);
            points(seen, 0)
        };
        assert_eq!(one(|s| s.no_language = true), 1);
        assert_eq!(one(|s| s.no_fetch_site = true), 1);
        assert_eq!(one(|s| s.agent = Agent::Missing), 2);
        assert_eq!(one(|s| s.agent = Agent::Tool), 2);
        assert_eq!(one(|s| s.frequent = 1), 1);
        assert_eq!(one(|s| s.frequent = 3), 3);
        // more haste than the listener counts is no more than the most
        assert_eq!(one(|s| s.frequent = 200), 3);
        assert_eq!(points(seen, SIGNAL_WEBDRIVER), 2);
        assert_eq!(points(seen, SIGNAL_NO_LANGUAGES), 1);
        assert_eq!(points(seen, register::SIGNALS), 3);
        // bits Central does not know say nothing
        assert_eq!(points(seen, !register::SIGNALS), 0);
        assert_eq!(points(seen, u8::MAX), 3);
    }

    #[test]
    fn the_signs_together_stop_at_four_bits() {
        let every = Seen {
            no_language: true,
            no_fetch_site: true,
            agent: Agent::Tool,
            frequent: 3,
        };
        assert_eq!(points(every, u8::MAX), SUSPICION);
        let script = Seen {
            no_language: true,
            no_fetch_site: true,
            agent: Agent::Missing,
            ..Seen::default()
        };
        assert_eq!(points(script, 0), SUSPICION);
        let driven = Seen {
            agent: Agent::Tool,
            ..Seen::default()
        };
        assert_eq!(points(driven, SIGNAL_WEBDRIVER), SUSPICION);
    }

    /// No person is ever locked out: whatever is seen and however many
    /// register, a challenge asks at most [`MAX_DIFFICULTY`], which a page
    /// takes, and lives as long as its difficulty needs.
    #[test]
    fn every_combination_gets_a_challenge_a_page_takes() {
        let start = Instant::now();
        for registered in [0, 60, 240, 1_920, 30_000] {
            let mut challenges = Challenges::default();
            for _ in 0..registered {
                challenges.registered(start);
            }
            for seen_points in 0..=u8::MAX {
                let issued = challenges.issue(start, seen_points).expect("issued");
                assert!(issued.difficulty >= BASE);
                assert!(issued.difficulty <= MAX_DIFFICULTY, "{registered}");
                let held = &challenges.held[&issued.nonce];
                assert_eq!(held.until - held.at, lifetime(issued.difficulty - BASE));
                assert_eq!(held.difficulty, issued.difficulty);
            }
        }
    }

    #[test]
    fn a_harder_challenge_lives_as_much_longer_as_it_is_harder() {
        for extra in 0..=PATIENT {
            assert_eq!(lifetime(extra), LIFETIME);
        }
        assert_eq!(lifetime(5), Duration::from_mins(10));
        assert_eq!(lifetime(6), Duration::from_mins(20));
        let mut challenges = Challenges::new(1, MINIMUM);
        let start = Instant::now();
        for _ in 0..1_920 {
            challenges.registered(start);
        }
        let hard = challenges.issue(start, 0).expect("issued");
        assert_eq!(hard.difficulty, 1 + MOST);
        let counters = solve(&hard);
        let late = start + (lifetime(MOST).saturating_sub(Duration::from_secs(1)));
        assert_eq!(challenges.spend(&hard.nonce, &counters, late), Ok(()));
        let hard = challenges.issue(start, 0).expect("issued");
        let counters = solve(&hard);
        assert_eq!(
            challenges.spend(&hard.nonce, &counters, start + lifetime(MOST)),
            Err(Error::Rejected),
            "its time is over"
        );
    }

    #[test]
    fn a_solution_counts_once_and_not_too_soon() {
        let mut challenges = Challenges::new(4, MINIMUM);
        let start = Instant::now();
        let issued = challenges.issue(start, 0).expect("issued");
        assert_eq!(issued.difficulty, 4);
        assert!(register::valid_nonce(&issued.nonce));
        let counters = solve(&issued);
        assert_eq!(
            challenges.spend(&issued.nonce, &counters, start + Duration::from_secs(1)),
            Err(Error::Rejected),
            "too soon"
        );
        assert_eq!(
            challenges.spend(&issued.nonce, &counters, start + MINIMUM),
            Err(Error::Rejected),
            "and gone with the try"
        );
        let again = challenges.issue(start, 0).expect("issued");
        let counters = solve(&again);
        let later = start + MINIMUM;
        assert_eq!(challenges.spend(&again.nonce, &counters, later), Ok(()));
        assert_eq!(
            challenges.spend(&again.nonce, &counters, later),
            Err(Error::Rejected),
            "once"
        );
        let old = challenges.issue(start, 0).expect("issued");
        let counters = solve(&old);
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

    /// The difficulty a challenge was issued with is the one its solution
    /// is checked against: work for an easier one is refused, and the
    /// challenge is gone with it.
    #[test]
    fn a_solution_for_less_than_was_asked_is_refused() {
        let mut challenges = Challenges::new(2, Duration::ZERO);
        let start = Instant::now();
        let hard = challenges.issue(start, SUSPICION).expect("issued");
        assert_eq!(hard.difficulty, 2 + SUSPICION);
        // What the base would have asked. That it also solves six bits in
        // every part has a chance of 2^-32.
        let easy = register::solve(&hard.nonce, register::PARTS, 2).expect("solved");
        assert!(register::solved(&hard.nonce, 2, &easy));
        assert!(!register::solved(&hard.nonce, hard.difficulty, &easy));
        assert_eq!(
            challenges.spend(&hard.nonce, &easy, start),
            Err(Error::Rejected)
        );
        assert_eq!(
            challenges.spend(&hard.nonce, &solve(&hard), start),
            Err(Error::Rejected),
            "spent by the wrong answer"
        );
        // A plain request afterwards is easier, and its solution does not
        // open the hard one's nonce.
        let hard = challenges.issue(start, SUSPICION).expect("issued");
        let plain = challenges.issue(start, 0).expect("issued");
        assert_eq!(plain.difficulty, 2);
        assert_eq!(
            challenges.spend(&hard.nonce, &solve(&plain), start),
            Err(Error::Rejected)
        );
        assert_eq!(
            challenges.spend(&plain.nonce, &solve(&plain), start),
            Ok(())
        );
    }

    #[test]
    fn registrations_raise_the_difficulty_and_the_hour_lowers_it() {
        let mut challenges = Challenges::default();
        let start = Instant::now();
        assert_eq!(challenges.issue(start, 0).expect("issued").difficulty, BASE);
        for _ in 0..BASELINE * 2 {
            challenges.registered(start);
        }
        assert_eq!(
            challenges.issue(start, 0).expect("issued").difficulty,
            BASE + 1
        );
        // thousands an hour: six bits, and no more however many
        let flood = start + Duration::from_secs(60);
        for _ in 0..5_000 {
            challenges.registered(flood);
        }
        assert_eq!(
            challenges.issue(flood, 0).expect("issued").difficulty,
            BASE + RISE
        );
        // the first sixty leave the hour, the flood stays for its own
        assert_eq!(
            challenges
                .issue(start + HOUR, 0)
                .expect("issued")
                .difficulty,
            BASE + RISE
        );
        assert_eq!(
            challenges
                .issue(flood + HOUR, 0)
                .expect("issued")
                .difficulty,
            BASE
        );
    }

    #[test]
    fn a_look_and_a_flood_add_up_to_six_bits_and_no_more() {
        let mut challenges = Challenges::default();
        let start = Instant::now();
        let asked = |challenges: &mut Challenges, points| {
            challenges.issue(start, points).expect("issued").difficulty
        };
        assert_eq!(asked(&mut challenges, 1), BASE + 1);
        assert_eq!(asked(&mut challenges, SUSPICION), BASE + SUSPICION);
        assert_eq!(asked(&mut challenges, 200), BASE + SUSPICION);
        for _ in 0..BASELINE * 2 {
            challenges.registered(start);
        }
        assert_eq!(asked(&mut challenges, 0), BASE + 1);
        assert_eq!(asked(&mut challenges, SUSPICION), BASE + 1 + SUSPICION);
        for _ in 0..BASELINE * 6 {
            challenges.registered(start);
        }
        assert_eq!(asked(&mut challenges, 0), BASE + 3);
        assert_eq!(asked(&mut challenges, 2), BASE + 5);
        assert_eq!(asked(&mut challenges, SUSPICION), BASE + MOST);
    }

    /// The difficulty is fixed when a challenge is issued: one asked for
    /// before a flood keeps its price for its five minutes, and no longer.
    #[test]
    fn a_challenge_from_before_the_rise_keeps_its_price_for_its_lifetime_only() {
        let mut challenges = Challenges::new(1, Duration::ZERO);
        let start = Instant::now();
        let early = challenges.issue(start, 0).expect("issued");
        let kept = challenges.issue(start, 0).expect("issued");
        for _ in 0..1_920 {
            challenges.registered(start);
        }
        assert_eq!(
            challenges.issue(start, 0).expect("issued").difficulty,
            1 + RISE
        );
        assert_eq!(early.difficulty, 1);
        assert_eq!(
            challenges.spend(&early.nonce, &solve(&early), start),
            Ok(())
        );
        assert_eq!(
            challenges.spend(&kept.nonce, &solve(&kept), start + LIFETIME),
            Err(Error::Rejected)
        );
    }

    #[test]
    fn a_full_table_makes_room() {
        let mut challenges = Challenges::new(4, MINIMUM);
        let start = Instant::now();
        let first = challenges.issue(start, 0).expect("issued");
        for _ in 1..HELD {
            challenges
                .issue(start + Duration::from_millis(1), 0)
                .expect("issued");
        }
        challenges
            .issue(start + Duration::from_millis(2), 0)
            .expect("issued");
        assert_eq!(challenges.held.len(), HELD);
        assert!(
            !challenges.held.contains_key(&first.nonce),
            "the oldest went"
        );
    }

    /// As many challenges as the global budget lets all callers together
    /// ask for, for as long as the hardest challenge lives: the first one
    /// is still there for its solution a second before its end, and the
    /// table never reached its bound.
    #[test]
    fn a_crowd_at_the_global_budget_evicts_no_challenge_before_its_end() {
        let mut challenges = Challenges::new(1, MINIMUM);
        let start = Instant::now();
        for _ in 0..1_920 {
            challenges.registered(start);
        }
        let first = challenges.issue(start, 0).expect("issued");
        assert_eq!(first.difficulty, 1 + MOST);
        let counters = solve(&first);
        let seconds = lifetime(MOST).as_secs() - 1;
        let mut most = 0;
        for second in 0..seconds {
            let now = start + Duration::from_secs(second);
            for _ in 0..GLOBAL_PER_SECOND {
                challenges.issue(now, SUSPICION).expect("issued");
            }
            most = most.max(challenges.held.len());
        }
        assert!(most < HELD, "{most}");
        let late = start + Duration::from_secs(seconds);
        assert_eq!(challenges.spend(&first.nonce, &counters, late), Ok(()));
    }

    #[test]
    fn challenges_whose_time_is_over_are_forgotten() {
        let mut challenges = Challenges::new(1, MINIMUM);
        let start = Instant::now();
        for _ in 0..100 {
            challenges.issue(start, 0).expect("issued");
        }
        challenges.issue(start + LIFETIME, 0).expect("issued");
        assert_eq!(challenges.held.len(), 1);
    }
}
