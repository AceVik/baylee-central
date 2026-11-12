//! Registration's proof of work (`docs/registration-abuse.md`): Central
//! issues a random nonce, keeps it in memory with the price it asked, and
//! takes a solution once, no sooner than three seconds after it was issued
//! and checked against that price alone.
//!
//! The work costs memory: Argon2id at [`MEMORY`] and [`PASSES`] for every
//! try (`central_crypto::proof`, the one function the page's Worker
//! searches with and [`prove`] checks with), so a program that is not a
//! page gains little over the page. Central pays for that too, one
//! evaluation for each part of an answer; [`prove`] says what bounds it.
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
//! counts, and of what was seen only the price stays, until the challenge
//! ends. This crate logs nothing.

use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use central_api::register::{self, SIGNAL_NO_LANGUAGES, SIGNAL_WEBDRIVER};
use central_api::{Error, Token};
use central_crypto::proof::{self, Work};
use tokio::sync::Semaphore;

use super::limits::PUBLIC_PER_SECOND;

/// How long a challenge of up to [`PATIENT`] bits over the base waits for
/// its solution; a harder one waits longer ([`lifetime`]).
pub(crate) const LIFETIME: Duration = Duration::from_secs(300);
/// The least time between a challenge and its solution: a form filled in
/// faster than this was not filled in by a person.
pub(crate) const MINIMUM: Duration = Duration::from_secs(3);
/// Challenges held at once: more than the public budget lets all callers
/// together ask for within the longest lifetime (asserted below; the
/// budget counts the spare requests of places that asked little, 40 a
/// second with them), so however many ask, no challenge makes room for
/// another before its own time is over.
pub(crate) const HELD: usize = 50_000;
/// Memory of one evaluation, in KiB: 16 MiB, a quarter of what the
/// password stretch takes in the page's Worker, and less than the legacy
/// store's check takes through the same workers here.
pub(crate) const MEMORY: u32 = 16_384;
/// Passes over that memory.
pub(crate) const PASSES: u32 = 1;
/// Parts of every challenge: eight keep the time close to its average, and
/// are what Central computes to check a right answer.
pub(crate) const PARTS: u8 = 8;
/// The difficulty when registrations are few: 128 evaluations on average
/// for all parts together, about a second of a laptop's browser and four
/// of a browser slowed four times, what the SHA-256 proof of sixteen bits
/// cost before it (measured, `docs/registration-abuse.md` §"A proof that
/// costs memory").
pub(crate) const BASE: u8 = 4;
/// Registrations an hour that cost no more than the base.
pub(crate) const BASELINE: usize = 30;
/// Bits a flood adds at most: from 1,920 registrations an hour on.
pub(crate) const RISE: u8 = 6;
/// Bits a request's own look adds at most.
pub(crate) const SUSPICION: u8 = 4;
/// Bits over the base at most, the flood's and the look's together: 64
/// times the base work, about a minute of that laptop and four to five of
/// the slowed browser.
pub(crate) const MOST: u8 = 6;
/// Bits over the base a challenge may have and still live [`LIFETIME`].
const PATIENT: u8 = 4;
const HOUR: Duration = Duration::from_secs(3_600);
/// How often the table is searched for challenges whose time is over.
const SWEEP: Duration = Duration::from_secs(1);
// What a page takes is what Central asks at its most, and not one unit
// more: the page's ceiling (`central-api`) is this crate's.
const _: () = assert!(
    register::work_cost(MEMORY, PASSES, PARTS, BASE + MOST) == register::WORK_MAX_COST
        && register::work_acceptable(MEMORY, PASSES, PARTS, BASE + MOST)
);
// The page reads its ceilings from `central-api`; the function that
// computes has its own (`central-crypto`). One set of numbers.
const _: () = assert!(
    register::WORK_MIN_MEMORY == proof::MIN_MEMORY
        && register::WORK_MAX_MEMORY == proof::MAX_MEMORY
        && register::WORK_MAX_PASSES == proof::MAX_PASSES
        && register::WORK_MAX_PARTS == proof::MAX_PARTS
        && register::WORK_MAX_DIFFICULTY == proof::MAX_DIFFICULTY
        && register::NONCE_LEN == proof::NONCE_LEN
);
const _: () = assert!(RISE <= MOST && SUSPICION <= MOST && PATIENT <= MOST);
// One more second for the window the public budget counts in.
const _: () = assert!(HELD as u64 > PUBLIC_PER_SECOND as u64 * (lifetime(MOST).as_secs() + 1));

/// How long a challenge of `extra` bits over the base lives: [`LIFETIME`],
/// doubled for each bit past [`PATIENT`], as the work doubles
/// (`docs/registration-abuse.md` §"A proof that costs memory" has what a
/// slowed browser needed of it).
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
    /// Its price: memory, passes, parts and the zero bits each part needs.
    pub work: Work,
    /// When it ends, Unix seconds.
    pub expires_at: i64,
}

/// A challenge waiting for its solution: when it was issued, when it ends,
/// and the price it was issued with, the only one its solution is checked
/// against.
struct Held {
    at: Instant,
    until: Instant,
    work: Work,
}

/// The challenges issued and the last hour's registrations.
pub(crate) struct Challenges {
    base: u8,
    memory: u32,
    minimum: Duration,
    held: BTreeMap<String, Held>,
    swept: Option<Instant>,
    registrations: VecDeque<Instant>,
}

impl Default for Challenges {
    fn default() -> Self {
        Challenges::new(BASE, MEMORY, MINIMUM)
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
    /// With `base` bits when registrations are few, `memory` KiB for each
    /// evaluation and `minimum` before a solution counts. Tests ask little;
    /// neither number can be more than the service's own.
    pub(crate) fn new(base: u8, memory: u32, minimum: Duration) -> Challenges {
        Challenges {
            base: base.min(BASE),
            memory: memory.clamp(proof::MIN_MEMORY, MEMORY),
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
        let work = Work {
            memory: self.memory,
            passes: PASSES,
            parts: PARTS,
            difficulty: self.base + extra,
        };
        let lifetime = lifetime(extra);
        if self.held.len() >= HELD {
            self.sweep(now);
        }
        // Not reached while the public budget holds (asserted above): the
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
                work,
            },
        );
        Ok(Issued {
            nonce,
            work,
            expires_at: unix(SystemTime::now() + lifetime),
        })
    }

    /// Takes the challenge `nonce` out of the table, once, whatever answer
    /// came with it, and says the price that answer is to be checked
    /// against: a challenge it does not know, one too young or one too old
    /// is `Rejected`, and gone all the same.
    ///
    /// # Errors
    /// `Rejected`.
    pub(crate) fn take(&mut self, nonce: &str, now: Instant) -> Result<Work, Error> {
        self.forget(now);
        let held = self.held.remove(nonce).ok_or(Error::Rejected)?;
        if now >= held.until || now.duration_since(held.at) < self.minimum {
            return Err(Error::Rejected);
        }
        Ok(held.work)
    }

    /// Counts a registration that went through.
    pub(crate) fn registered(&mut self, now: Instant) {
        self.forget(now);
        self.registrations.push_back(now);
    }
}

/// Takes the solution of `nonce`, once, whatever it is: a challenge the
/// table does not know, one too young or too old, or counters that do not
/// solve it at the price it was issued with are `Rejected`, and the
/// challenge is gone before the counters are judged.
///
/// The order, and what it bounds. A worker first: checking costs memory
/// and time, so it runs on one of the service's few workers, off the
/// executor, and without a free one the answer is `Busy` while the
/// challenge stays for the same solution to be sent again. Then the nonce
/// leaves the table. Then the counters are checked, part by part, and the
/// first that does not hold ends it: a wrong answer costs one evaluation,
/// a right one [`PARTS`]. So at any moment the checks hold at most the
/// workers' number times [`MEMORY`]; an address can make Central check as
/// often as the browser listener lets it send `Register`, and all callers
/// together as often as the shared budget lets them
/// ([`PUBLIC_PER_SECOND`]).
///
/// # Errors
/// `Busy` without a worker, `Rejected` as above, `Unavailable` when the
/// table or the worker failed.
pub(crate) async fn prove(
    challenges: &Mutex<Challenges>,
    workers: &Arc<Semaphore>,
    nonce: &str,
    counters: &[u64],
    now: Instant,
) -> Result<(), Error> {
    let permit = workers
        .clone()
        .try_acquire_owned()
        .map_err(|_| Error::Busy)?;
    let work = challenges
        .lock()
        .map_err(|_| Error::Unavailable)?
        .take(nonce, now)?;
    // Before anything is copied or allocated for it.
    if counters.len() != usize::from(work.parts) {
        return Err(Error::Rejected);
    }
    let (nonce, counters) = (nonce.to_owned(), counters.to_vec());
    let solved = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        proof::solved(&nonce, work, &counters)
    })
    .await
    .map_err(|_| Error::Unavailable)?;
    if solved { Ok(()) } else { Err(Error::Rejected) }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests ask for little memory, so that a search costs no time here.
    const LITTLE: u32 = 64;

    fn solve(issued: &Issued) -> Vec<u64> {
        proof::solve(&issued.nonce, issued.work).expect("solved")
    }

    /// What [`prove`] does with an answer, without its worker: the nonce
    /// leaves the table, then the counters are judged.
    fn spend(
        challenges: &mut Challenges,
        nonce: &str,
        counters: &[u64],
        now: Instant,
    ) -> Result<(), Error> {
        let work = challenges.take(nonce, now)?;
        if proof::solved(nonce, work, counters) {
            Ok(())
        } else {
            Err(Error::Rejected)
        }
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
    /// register, a challenge asks a price a page takes
    /// (`register::work_acceptable`), and lives as long as its difficulty
    /// needs.
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
                let Work {
                    memory,
                    passes,
                    parts,
                    difficulty,
                } = issued.work;
                assert_eq!((memory, passes, parts), (MEMORY, PASSES, PARTS));
                assert!(difficulty >= BASE);
                assert!(difficulty <= BASE + MOST, "{registered}");
                assert!(
                    register::work_acceptable(memory, passes, parts, difficulty),
                    "{registered}"
                );
                assert!(issued.work.acceptable());
                let held = &challenges.held[&issued.nonce];
                assert_eq!(held.until - held.at, lifetime(difficulty - BASE));
                assert_eq!(held.work, issued.work);
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
        let mut challenges = Challenges::new(1, LITTLE, MINIMUM);
        let start = Instant::now();
        for _ in 0..1_920 {
            challenges.registered(start);
        }
        let hard = challenges.issue(start, 0).expect("issued");
        assert_eq!(hard.work.difficulty, 1 + MOST);
        let counters = solve(&hard);
        let late = start + (lifetime(MOST).saturating_sub(Duration::from_secs(1)));
        assert_eq!(spend(&mut challenges, &hard.nonce, &counters, late), Ok(()));
        let hard = challenges.issue(start, 0).expect("issued");
        let counters = solve(&hard);
        assert_eq!(
            spend(
                &mut challenges,
                &hard.nonce,
                &counters,
                start + lifetime(MOST)
            ),
            Err(Error::Rejected),
            "its time is over"
        );
    }

    #[test]
    fn a_solution_counts_once_and_not_too_soon() {
        let mut challenges = Challenges::new(2, LITTLE, MINIMUM);
        let start = Instant::now();
        let issued = challenges.issue(start, 0).expect("issued");
        assert_eq!(issued.work.difficulty, 2);
        assert_eq!(issued.work.memory, LITTLE);
        assert!(register::valid_nonce(&issued.nonce));
        let counters = solve(&issued);
        assert_eq!(
            spend(
                &mut challenges,
                &issued.nonce,
                &counters,
                start + Duration::from_secs(1)
            ),
            Err(Error::Rejected),
            "too soon"
        );
        assert_eq!(
            spend(&mut challenges, &issued.nonce, &counters, start + MINIMUM),
            Err(Error::Rejected),
            "and gone with the try"
        );
        let again = challenges.issue(start, 0).expect("issued");
        let counters = solve(&again);
        let later = start + MINIMUM;
        assert_eq!(
            spend(&mut challenges, &again.nonce, &counters, later),
            Ok(())
        );
        assert_eq!(
            spend(&mut challenges, &again.nonce, &counters, later),
            Err(Error::Rejected),
            "once"
        );
        let old = challenges.issue(start, 0).expect("issued");
        let counters = solve(&old);
        assert_eq!(
            spend(&mut challenges, &old.nonce, &counters, start + LIFETIME),
            Err(Error::Rejected),
            "expired"
        );
        assert_eq!(
            spend(&mut challenges, "unknown", &[], later),
            Err(Error::Rejected)
        );
    }

    /// The difficulty a challenge was issued with is the one its solution
    /// is checked against: work for an easier one is refused, and the
    /// challenge is gone with it.
    #[test]
    fn a_solution_for_less_than_was_asked_is_refused() {
        let mut challenges = Challenges::new(2, LITTLE, Duration::ZERO);
        let start = Instant::now();
        let hard = challenges.issue(start, SUSPICION).expect("issued");
        assert_eq!(hard.work.difficulty, 2 + SUSPICION);
        // What the base would have asked. That it also solves six bits in
        // every part has a chance of 2^-32.
        let base = Work {
            difficulty: 2,
            ..hard.work
        };
        let easy = proof::solve(&hard.nonce, base).expect("solved");
        assert!(proof::solved(&hard.nonce, base, &easy));
        assert!(!proof::solved(&hard.nonce, hard.work, &easy));
        assert_eq!(
            spend(&mut challenges, &hard.nonce, &easy, start),
            Err(Error::Rejected)
        );
        assert_eq!(
            spend(&mut challenges, &hard.nonce, &solve(&hard), start),
            Err(Error::Rejected),
            "spent by the wrong answer"
        );
        // A plain request afterwards is easier, and its solution does not
        // open the hard one's nonce.
        let hard = challenges.issue(start, SUSPICION).expect("issued");
        let plain = challenges.issue(start, 0).expect("issued");
        assert_eq!(plain.work.difficulty, 2);
        assert_eq!(
            spend(&mut challenges, &hard.nonce, &solve(&plain), start),
            Err(Error::Rejected)
        );
        assert_eq!(
            spend(&mut challenges, &plain.nonce, &solve(&plain), start),
            Ok(())
        );
    }

    #[test]
    fn registrations_raise_the_difficulty_and_the_hour_lowers_it() {
        let mut challenges = Challenges::default();
        let start = Instant::now();
        assert_eq!(
            challenges.issue(start, 0).expect("issued").work.difficulty,
            BASE
        );
        for _ in 0..BASELINE * 2 {
            challenges.registered(start);
        }
        assert_eq!(
            challenges.issue(start, 0).expect("issued").work.difficulty,
            BASE + 1
        );
        // thousands an hour: six bits, and no more however many
        let flood = start + Duration::from_secs(60);
        for _ in 0..5_000 {
            challenges.registered(flood);
        }
        assert_eq!(
            challenges.issue(flood, 0).expect("issued").work.difficulty,
            BASE + RISE
        );
        // the first sixty leave the hour, the flood stays for its own
        assert_eq!(
            challenges
                .issue(start + HOUR, 0)
                .expect("issued")
                .work
                .difficulty,
            BASE + RISE
        );
        assert_eq!(
            challenges
                .issue(flood + HOUR, 0)
                .expect("issued")
                .work
                .difficulty,
            BASE
        );
    }

    #[test]
    fn a_look_and_a_flood_add_up_to_six_bits_and_no_more() {
        let mut challenges = Challenges::default();
        let start = Instant::now();
        let asked = |challenges: &mut Challenges, points| {
            challenges
                .issue(start, points)
                .expect("issued")
                .work
                .difficulty
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
        let mut challenges = Challenges::new(1, LITTLE, Duration::ZERO);
        let start = Instant::now();
        let early = challenges.issue(start, 0).expect("issued");
        let kept = challenges.issue(start, 0).expect("issued");
        for _ in 0..1_920 {
            challenges.registered(start);
        }
        assert_eq!(
            challenges.issue(start, 0).expect("issued").work.difficulty,
            1 + RISE
        );
        assert_eq!(early.work.difficulty, 1);
        assert_eq!(
            spend(&mut challenges, &early.nonce, &solve(&early), start),
            Ok(())
        );
        assert_eq!(
            spend(
                &mut challenges,
                &kept.nonce,
                &solve(&kept),
                start + LIFETIME
            ),
            Err(Error::Rejected)
        );
    }

    #[test]
    fn a_full_table_makes_room() {
        let mut challenges = Challenges::new(2, LITTLE, MINIMUM);
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

    /// As many challenges as the public budget lets all callers together
    /// ask for, for as long as the hardest challenge lives: the first one
    /// is still there for its solution a second before its end, and the
    /// table never reached its bound.
    #[test]
    fn a_crowd_at_the_global_budget_evicts_no_challenge_before_its_end() {
        let mut challenges = Challenges::new(1, LITTLE, MINIMUM);
        let start = Instant::now();
        for _ in 0..1_920 {
            challenges.registered(start);
        }
        let first = challenges.issue(start, 0).expect("issued");
        assert_eq!(first.work.difficulty, 1 + MOST);
        let counters = solve(&first);
        let seconds = lifetime(MOST).as_secs() - 1;
        let mut most = 0;
        for second in 0..seconds {
            let now = start + Duration::from_secs(second);
            for _ in 0..PUBLIC_PER_SECOND {
                challenges.issue(now, SUSPICION).expect("issued");
            }
            most = most.max(challenges.held.len());
        }
        assert!(most < HELD, "{most}");
        let late = start + Duration::from_secs(seconds);
        assert_eq!(
            spend(&mut challenges, &first.nonce, &counters, late),
            Ok(())
        );
    }

    #[test]
    fn challenges_whose_time_is_over_are_forgotten() {
        let mut challenges = Challenges::new(1, LITTLE, MINIMUM);
        let start = Instant::now();
        for _ in 0..100 {
            challenges.issue(start, 0).expect("issued");
        }
        challenges.issue(start + LIFETIME, 0).expect("issued");
        assert_eq!(challenges.held.len(), 1);
    }

    /// The older proof is no answer here: SHA-256 counters for the same
    /// nonce, at the base the page of before was asked, are refused like
    /// any wrong answer, and the challenge is gone with them.
    #[test]
    fn a_sha_256_solution_is_not_taken() {
        let mut challenges = Challenges::new(1, LITTLE, Duration::ZERO);
        let start = Instant::now();
        let issued = challenges.issue(start, 0).expect("issued");
        let older = register::solve(&issued.nonce, register::PARTS, 8).expect("solved");
        assert!(register::solved(&issued.nonce, 8, &older));
        // Its eight counters are of the right number, so they are judged.
        assert_eq!(older.len(), usize::from(PARTS));
        assert_eq!(
            spend(&mut challenges, &issued.nonce, &older, start),
            Err(Error::Rejected)
        );
        assert_eq!(
            spend(&mut challenges, &issued.nonce, &solve(&issued), start),
            Err(Error::Rejected),
            "spent"
        );
    }

    /// Tests may ask for less, never for more: the knob cannot raise what
    /// the service asks of a person.
    #[test]
    fn the_knob_only_lowers() {
        let start = Instant::now();
        let mut more = Challenges::new(u8::MAX, u32::MAX, Duration::ZERO);
        let issued = more.issue(start, 0).expect("issued");
        assert_eq!(issued.work.difficulty, BASE);
        assert_eq!(issued.work.memory, MEMORY);
        let mut less = Challenges::new(0, 0, Duration::ZERO);
        let issued = less.issue(start, 0).expect("issued");
        assert_eq!(issued.work.difficulty, 0);
        assert_eq!(issued.work.memory, proof::MIN_MEMORY);
        assert!(issued.work.acceptable());
    }

    fn wait<T>(work: impl Future<Output = T>) -> T {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("a runtime")
            .block_on(work)
    }

    /// [`prove`]'s order: without a worker the answer is `Busy` and the
    /// challenge stays, so the same solution counts when it is sent again;
    /// with one, a wrong answer spends the challenge.
    #[test]
    fn a_busy_service_keeps_the_challenge_for_its_solution() {
        let challenges = Mutex::new(Challenges::new(1, LITTLE, Duration::ZERO));
        let workers = Arc::new(Semaphore::new(2));
        let start = Instant::now();
        let issue = || {
            challenges
                .lock()
                .expect("a table")
                .issue(start, 0)
                .expect("issued")
        };
        let issued = issue();
        let counters = solve(&issued);
        let prove = |nonce: &str, counters: &[u64]| {
            wait(super::prove(&challenges, &workers, nonce, counters, start))
        };
        let held = [
            workers.clone().try_acquire_owned().expect("a worker"),
            workers.clone().try_acquire_owned().expect("a worker"),
        ];
        for _ in 0..3 {
            assert_eq!(prove(&issued.nonce, &counters), Err(Error::Busy));
        }
        // Nothing was looked at: an unknown nonce is `Busy` as well.
        assert_eq!(prove("unknown", &[]), Err(Error::Busy));
        assert!(
            challenges
                .lock()
                .expect("a table")
                .held
                .contains_key(&issued.nonce)
        );
        drop(held);
        assert_eq!(prove(&issued.nonce, &counters), Ok(()));
        assert_eq!(
            prove(&issued.nonce, &counters),
            Err(Error::Rejected),
            "once"
        );
        // The worker is given back after every check, right or wrong.
        assert_eq!(workers.available_permits(), 2);
        let wrong = issue();
        let mut answer = solve(&wrong);
        answer[7] += 1;
        while proof::solved(&wrong.nonce, wrong.work, &answer) {
            answer[7] += 1;
        }
        assert_eq!(prove(&wrong.nonce, &answer), Err(Error::Rejected));
        assert_eq!(
            prove(&wrong.nonce, &solve(&wrong)),
            Err(Error::Rejected),
            "spent by the wrong answer"
        );
        // Counters of another number are refused before a worker computes,
        // and spend the challenge all the same.
        let short = issue();
        let answer = solve(&short);
        assert_eq!(prove(&short.nonce, &answer[..7]), Err(Error::Rejected));
        assert_eq!(prove(&short.nonce, &answer), Err(Error::Rejected));
        assert_eq!(prove("unknown", &[]), Err(Error::Rejected));
        assert_eq!(workers.available_permits(), 2);
    }
}
