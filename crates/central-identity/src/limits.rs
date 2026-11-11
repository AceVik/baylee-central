use super::Place;
use central_api::{Error, Token, wire::Request};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

/// Requests a second without a session, all clients together: discovery,
/// sign-in, registration and the other public operations.
pub(super) const GLOBAL_PER_SECOND: u32 = 32;
/// How long an unfinished OPAQUE sign-in, password change or deletion waits
/// for its second message.
pub(super) const PENDING_SECONDS: u64 = 60;
/// Unfinished OPAQUE exchanges held at once. More than the global budget can
/// start within their lifetime ([`GLOBAL_PER_SECOND`] × [`PENDING_SECONDS`] =
/// 1920), so no sequence of public requests fills the table before it spends
/// the budget everyone shares; a password change or deletion besides needs a
/// signed-in account and its eight units a minute. Each entry is an OPAQUE
/// server state and a credential record, about a kilobyte: a few megabytes
/// at most.
pub(super) const PENDING: usize = 4096;
const _: () = assert!(PENDING as u64 > GLOBAL_PER_SECOND as u64 * PENDING_SECONDS);

/// Legacy sign-ins a minute, all names together: the store holds a handful
/// of accounts, and each sign-in is one Argon2 derivation.
pub(super) const LEGACY_PER_MINUTE: u32 = 30;
/// Legacy sign-ins a minute for one name, as typed and lowercased; the
/// store's lockout counts the failures that pass this.
pub(super) const LEGACY_PER_NAME: u32 = 5;

/// Requests a minute from one place that draw on [`GLOBAL_PER_SECOND`]: a
/// little under a third of that minute's 1,920, so one address alone never
/// spends everyone's sign-ins, and far more than a house or an office
/// needs (a sign-in is three to five such requests).
pub(super) const PLACE_PER_MINUTE: u32 = 600;
/// A client's pairing requests a minute from one place: a pairing polls
/// once a second, so four at once, of the 64 a second all places share.
pub(super) const PLACE_PAIRING_PER_MINUTE: u32 = 240;
/// Places one such budget remembers. Never full of live minutes: fewer
/// requests pass the shared windows between two sweeps and the minute
/// before them (at most 64 a second) than it holds.
pub(super) const PLACES: usize = 8_192;
const _: () = assert!(PLACES > 2 * 60 * 64);
const MINUTE: Duration = Duration::from_secs(60);

/// What each place drew of one shared budget in its current minute, in
/// memory only; a place is forgotten at most a minute after its minute.
#[derive(Default)]
struct Places {
    windows: BTreeMap<Place, (Instant, u32)>,
    swept: Option<Instant>,
}

impl Places {
    /// Whether `place` may draw once more of `limit` a minute. A request
    /// the listener knows no place of meets the shared budget alone.
    fn fits(&self, place: Option<Place>, now: Instant, limit: u32) -> bool {
        place
            .and_then(|place| self.windows.get(&place))
            .is_none_or(|(start, used)| {
                now.saturating_duration_since(*start) >= MINUTE || *used < limit
            })
    }

    /// Count one request that passed the shared budget.
    fn spend(&mut self, place: Option<Place>, now: Instant) {
        let Some(place) = place else {
            return;
        };
        let due = self
            .swept
            .is_none_or(|swept| now.saturating_duration_since(swept) >= MINUTE);
        if due {
            self.windows
                .retain(|_, (start, _)| now.saturating_duration_since(*start) < MINUTE);
            self.swept = Some(now);
        }
        if self.windows.len() >= PLACES && !self.windows.contains_key(&place) {
            let oldest = self
                .windows
                .iter()
                .min_by_key(|(_, (start, _))| *start)
                .map(|(place, _)| *place);
            if let Some(oldest) = oldest {
                self.windows.remove(&oldest);
            }
        }
        let window = self.windows.entry(place).or_insert((now, 0));
        if now.saturating_duration_since(window.0) >= MINUTE {
            *window = (now, 0);
        }
        window.1 = window.1.saturating_add(1);
    }
}

#[derive(Default)]
pub(super) struct Limits {
    global: Option<(Instant, u32)>,
    /// Each place's share of [`GLOBAL_PER_SECOND`] ([`PLACE_PER_MINUTE`]).
    public: Places,
    /// Each place's share of the pairing window ([`PLACE_PAIRING_PER_MINUTE`]).
    paired: Places,
    /// Guesses at passwords: sign-ins, password changes and deletions
    /// (`guesses.rs`).
    pub(super) guesses: super::guesses::Guesses,
    legacy: Option<(Instant, u32)>,
    legacy_names: BTreeMap<String, (Instant, u32)>,
    data: Window,
    control: Window,
    pairing: Window,
    peers: BTreeMap<([u8; 32], bool), Window>,
    /// Reports each gateway filed in the current hour.
    reports: BTreeMap<[u8; 16], (Instant, u32)>,
}

#[derive(Default)]
struct Window(Option<(Instant, u32)>);
impl Window {
    fn charge(&mut self, now: Instant, maximum: u32) -> Result<(), Error> {
        let (start, used) = self.0.get_or_insert((now, 0));
        if now.duration_since(*start) >= Duration::from_secs(1) {
            *start = now;
            *used = 0;
        }
        if *used >= maximum {
            return Err(Error::Busy);
        }
        *used += 1;
        Ok(())
    }
}
impl Limits {
    /// Count a request against what all clients share, before anything
    /// else is looked at. `place` is where it came from: a request that
    /// needs no session draws on its place's share first
    /// ([`PLACE_PER_MINUTE`], [`PLACE_PAIRING_PER_MINUTE`]), so one address
    /// cannot spend what everyone signs in with. Nothing is spent when a
    /// budget is used up.
    pub(super) fn request(
        &mut self,
        request: &Request,
        bearer: Option<&Token>,
        place: Option<Place>,
        now: Instant,
    ) -> Result<(), Error> {
        // A client's claim, polls and redeem carry no session; their own
        // window keeps a few pairings' polling from spending the sign-ins'.
        if matches!(
            request,
            Request::PairClaim { .. } | Request::PairPoll { .. } | Request::PairRedeem { .. }
        ) {
            if !self.paired.fits(place, now, PLACE_PAIRING_PER_MINUTE) {
                return Err(Error::Busy);
            }
            self.pairing.charge(now, 64)?;
            self.paired.spend(place, now);
            return Ok(());
        }
        if matches!(
            request,
            Request::Discover { .. }
                | Request::LoginStart { .. }
                | Request::LoginFinish { .. }
                // Named, not left to the rule for requests without a
                // session: the table of challenges is sized by this budget
                // (`challenge::HELD`), whatever token comes with the request.
                | Request::RegisterChallenge
                | Request::RegisterChallengeFor { .. }
                | Request::Register { .. }
                | Request::EnrollmentStart { .. }
                | Request::EnrollmentFinish { .. }
                | Request::GatewayDirectory { .. }
                | Request::PublicFigures { .. }
                | Request::Terms
                | Request::GatewayTerms { .. }
                | Request::ResetStart { .. }
                | Request::ResetFinish { .. }
                | Request::LegacyLogin { .. }
        ) {
            return self.global(place, now);
        }
        let Some(bearer) = bearer else {
            return self.global(place, now);
        };
        // Separate bounded budgets keep a ciphertext transfer from exhausting
        // login or revocation checks. A bearer is still verified by dispatch;
        // merely supplying a token never confers authorization.
        let control = matches!(
            request,
            Request::SessionStatus
                | Request::RenewSession
                | Request::GatewaySession { .. }
                | Request::GatewayProfile { .. }
                | Request::GatewayAuthorize { .. }
                | Request::TermsStatus
                | Request::AccountSummary
                | Request::Logout
                | Request::SetGatewayAdmins { .. }
        );
        if control {
            self.control.charge(now, 512)?;
        } else {
            self.data.charge(now, 256)?;
        }
        self.peers.retain(|_, window| {
            window
                .0
                .is_some_and(|(start, _)| now.duration_since(start) < Duration::from_secs(1))
        });
        let key = (bearer.digest(), control);
        if !self.peers.contains_key(&key) && self.peers.len() >= 1024 {
            return Err(Error::Busy);
        }
        self.peers
            .entry(key)
            .or_default()
            .charge(now, if control { 128 } else { 64 })
    }
    /// Spend one of the [`GLOBAL_PER_SECOND`] requests everyone shares,
    /// and one of the [`PLACE_PER_MINUTE`] its place may draw of them.
    /// Nothing is spent when either is used up.
    pub(super) fn global(&mut self, place: Option<Place>, now: Instant) -> Result<(), Error> {
        if !self.public.fits(place, now, PLACE_PER_MINUTE) {
            return Err(Error::Busy);
        }
        self.everyone(now)?;
        self.public.spend(place, now);
        Ok(())
    }

    fn everyone(&mut self, now: Instant) -> Result<(), Error> {
        let (start, used) = self.global.get_or_insert((now, 0));
        if now.duration_since(*start) >= Duration::from_secs(1) {
            *start = now;
            *used = 0;
        }
        if *used >= GLOBAL_PER_SECOND {
            return Err(Error::Busy);
        }
        *used += 1;
        Ok(())
    }
    /// Spend one legacy sign-in: of [`LEGACY_PER_MINUTE`] for all names and
    /// [`LEGACY_PER_NAME`] for this one (as typed, lowercased, at most 64
    /// bytes of it). Nothing is spent when either is used up. A name that is
    /// nobody's is counted like one that is somebody's.
    pub(super) fn legacy(&mut self, typed: &str, now: Instant) -> Result<(), Error> {
        let minute = Duration::from_secs(60);
        let name: String = typed.chars().take(64).collect::<String>().to_lowercase();
        self.legacy_names
            .retain(|_, (start, _)| now.duration_since(*start) < minute);
        let (start, used) = self.legacy.get_or_insert((now, 0));
        if now.duration_since(*start) >= minute {
            *start = now;
            *used = 0;
        }
        let per_name = self.legacy_names.get(&name).map_or(0, |(_, used)| *used);
        if *used >= LEGACY_PER_MINUTE
            || per_name >= LEGACY_PER_NAME
            || (per_name == 0 && self.legacy_names.len() >= 1024)
        {
            return Err(Error::Busy);
        }
        *used += 1;
        self.legacy_names.entry(name).or_insert((now, 0)).1 += 1;
        Ok(())
    }

    /// A legacy sign-in from `place`: one of the names' budgets
    /// ([`Self::legacy`]) and one guess of the place's, which a proof
    /// gives back (`guesses.rs`). Nothing is spent when either is used up.
    pub(super) fn legacy_from(
        &mut self,
        typed: &str,
        place: Option<Place>,
        now: Instant,
    ) -> Result<super::guesses::Guess, Error> {
        self.guesses.legacy_fits(place, now)?;
        self.legacy(typed, now)?;
        self.guesses.legacy(place, now)
    }

    /// Spend one of the gateway's [`central_api::reports::PER_GATEWAY_HOUR`]
    /// reports an hour. Nothing is spent when it is used up.
    pub(super) fn report(&mut self, gateway: [u8; 16], now: Instant) -> Result<(), Error> {
        let hour = Duration::from_secs(3_600);
        self.reports
            .retain(|_, (start, _)| now.duration_since(*start) < hour);
        if !self.reports.contains_key(&gateway) && self.reports.len() >= 1024 {
            return Err(Error::Busy);
        }
        let (_, used) = self.reports.entry(gateway).or_insert((now, 0));
        if *used >= central_api::reports::PER_GATEWAY_HOUR {
            return Err(Error::Busy);
        }
        *used += 1;
        Ok(())
    }

    /// Spend `units` of the account's eight OPAQUE evaluations a minute
    /// and nothing of its longer windows: a reset's start, which only a
    /// valid reset code reaches, so that guessers who spent the account's
    /// hour or day do not keep its holder from a reset (`reset.rs`). The
    /// starts that test a guessed password (a sign-in's, a password
    /// change's, a deletion's) are counted by [`Self::guesses`]. Nothing
    /// is spent when the units do not fit.
    pub(super) fn account(
        &mut self,
        account: [u8; 16],
        now: Instant,
        units: u32,
    ) -> Result<(), Error> {
        self.guesses.evaluate(account, now, units)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn uploads_cannot_starve_authentication_or_revocation_checks() {
        let now = Instant::now();
        let mut limits = Limits::default();
        let token = Token::generate().unwrap();
        for _ in 0..64 {
            limits
                .request(&Request::CreateObject, Some(&token), None, now)
                .unwrap();
        }
        assert_eq!(
            limits.request(&Request::CreateObject, Some(&token), None, now),
            Err(Error::Busy)
        );
        limits
            .request(
                &Request::Discover {
                    username: "synthetic".into(),
                },
                None,
                None,
                now,
            )
            .unwrap();
        limits
            .request(&Request::SessionStatus, Some(&token), None, now)
            .unwrap();
        limits
            .request(&Request::Logout, Some(&token), None, now)
            .unwrap();
        limits
            .request(
                &Request::CreateObject,
                Some(&token),
                None,
                now + Duration::from_secs(1),
            )
            .unwrap();
    }

    fn place(n: u32) -> Place {
        Place::V4(n.to_be_bytes())
    }

    /// `count` requests of `place`, one every `step` milliseconds from
    /// `from` on; how many were served.
    fn served(
        limits: &mut Limits,
        request: &Request,
        from: Option<Place>,
        start: Instant,
        count: u32,
        step: u64,
    ) -> u32 {
        let mut served = 0;
        for index in 0..u64::from(count) {
            let now = start + Duration::from_millis(index * step);
            if limits.request(request, None, from, now).is_ok() {
                served += 1;
            }
        }
        served
    }

    #[test]
    fn one_place_draws_its_share_of_what_everyone_signs_in_with_and_no_more() {
        let start = Instant::now();
        let mut limits = Limits::default();
        let discover = Request::Discover {
            username: "synthetic".into(),
        };
        // As fast as the shared budget lets one address go (32 a second):
        // 600 in the minute, whatever it asks, and nothing after.
        assert_eq!(
            served(&mut limits, &discover, Some(place(1)), start, 700, 40),
            PLACE_PER_MINUTE
        );
        let later = start + Duration::from_secs(30);
        for request in [
            &discover,
            &Request::Terms,
            &Request::SessionStatus,
            &Request::LoginFinish {
                attempt: Token::generate().unwrap(),
                message: Vec::new(),
            },
        ] {
            assert_eq!(
                limits.request(request, None, Some(place(1)), later),
                Err(Error::Busy)
            );
            // Everyone else is served meanwhile, and so is a request the
            // listener knows no place of.
            limits
                .request(request, None, Some(place(2)), later)
                .unwrap();
            limits.request(request, None, None, later).unwrap();
        }
        // A request with a session draws on its session's budgets instead.
        let token = Token::generate().unwrap();
        limits
            .request(&Request::SessionStatus, Some(&token), Some(place(1)), later)
            .unwrap();
        // Its minute over, the place is served again.
        let next = start + Duration::from_secs(60);
        limits
            .request(&discover, None, Some(place(1)), next)
            .unwrap();
        // Refusals spent nothing of anyone's: the second is still whole.
        assert_eq!(
            served(&mut limits, &discover, Some(place(3)), next, 40, 0),
            31
        );
    }

    #[test]
    fn a_report_s_routes_draw_on_the_same_share() {
        let start = Instant::now();
        let mut limits = Limits::default();
        let mut passed = 0;
        for index in 0..700_u64 {
            let now = start + Duration::from_millis(index * 40);
            if limits.global(Some(place(1)), now).is_ok() {
                passed += 1;
            }
        }
        assert_eq!(passed, PLACE_PER_MINUTE);
        let later = start + Duration::from_secs(30);
        assert_eq!(limits.global(Some(place(1)), later), Err(Error::Busy));
        assert_eq!(
            limits.request(&Request::Terms, None, Some(place(1)), later),
            Err(Error::Busy)
        );
        limits.global(Some(place(2)), later).unwrap();
    }

    #[test]
    fn one_place_polls_four_pairings_and_spends_no_sign_in() {
        let start = Instant::now();
        let mut limits = Limits::default();
        let poll = Request::PairPoll {
            handle: Token::generate().unwrap(),
        };
        assert_eq!(
            served(&mut limits, &poll, Some(place(1)), start, 300, 20),
            PLACE_PAIRING_PER_MINUTE
        );
        let later = start + Duration::from_secs(30);
        assert_eq!(
            limits.request(&poll, None, Some(place(1)), later),
            Err(Error::Busy)
        );
        limits.request(&poll, None, Some(place(2)), later).unwrap();
        // The polls spent nothing of the place's other requests.
        limits
            .request(&Request::Terms, None, Some(place(1)), later)
            .unwrap();
    }

    #[test]
    fn places_are_forgotten_after_their_minute_and_none_is_refused_for_room() {
        let start = Instant::now();
        let mut places = Places::default();
        for n in 0..u32::try_from(PLACES).unwrap() {
            places.spend(Some(place(n)), start);
        }
        assert_eq!(places.windows.len(), PLACES);
        // Full of live minutes (more than the shared budget lets in): a
        // newcomer takes the oldest one's room.
        let soon = start + Duration::from_secs(1);
        places.spend(Some(place(u32::MAX)), soon);
        assert_eq!(places.windows.len(), PLACES);
        assert!(!places.fits(Some(place(u32::MAX)), soon, 1));
        // A minute on, the next request sweeps everything that passed.
        let later = start + Duration::from_secs(61);
        places.spend(Some(place(7)), later);
        assert_eq!(places.windows.len(), 1);
        // No place: nothing is kept, and nothing refuses.
        places.spend(None, later);
        assert!(places.fits(None, later, 0));
        assert_eq!(places.windows.len(), 1);
    }
    #[test]
    fn supplying_a_bearer_never_bypasses_the_authentication_budget() {
        let now = Instant::now();
        let mut limits = Limits::default();
        let token = Token::generate().unwrap();
        let request = Request::Discover {
            username: "synthetic".into(),
        };
        for _ in 0..32 {
            limits.request(&request, Some(&token), None, now).unwrap();
        }
        assert_eq!(limits.request(&request, None, None, now), Err(Error::Busy));
        assert_eq!(
            limits.request(&request, Some(&Token::generate().unwrap()), None, now),
            Err(Error::Busy)
        );
    }
    /// A registration challenge spends the budget all callers share,
    /// whichever way it is asked and whatever token comes with it: the
    /// table of challenges is sized by that budget.
    #[test]
    fn a_registration_challenge_spends_the_global_budget_with_or_without_a_token() {
        let now = Instant::now();
        for request in [
            Request::RegisterChallenge,
            Request::RegisterChallengeFor { signals: 0 },
        ] {
            let mut limits = Limits::default();
            for _ in 0..GLOBAL_PER_SECOND / 2 {
                limits.request(&request, None, None, now).unwrap();
                limits
                    .request(&request, Some(&Token::generate().unwrap()), None, now)
                    .unwrap();
            }
            assert_eq!(limits.request(&request, None, None, now), Err(Error::Busy));
            assert_eq!(
                limits.request(&request, Some(&Token::generate().unwrap()), None, now),
                Err(Error::Busy)
            );
            limits
                .request(&request, None, None, now + Duration::from_secs(1))
                .unwrap();
        }
    }

    #[test]
    fn the_figures_spend_the_public_budget_and_a_pulse_a_heartbeat_s() {
        let now = Instant::now();
        let mut limits = Limits::default();
        let token = Token::generate().unwrap();
        let figures = Request::PublicFigures { after: None };
        // Public: a bearer sent along buys no budget of its own.
        for _ in 0..GLOBAL_PER_SECOND {
            limits.request(&figures, Some(&token), None, now).unwrap();
        }
        for request in [&figures, &Request::GatewayDirectory { after: None }] {
            assert_eq!(limits.request(request, None, None, now), Err(Error::Busy));
            assert_eq!(
                limits.request(request, Some(&token), None, now),
                Err(Error::Busy)
            );
        }
        // A pulse and a heartbeat are one credential's same 64 a second.
        let metadata = || central_api::gateway::GatewayMetadata {
            name: "Synthetic".into(),
            version: "1".into(),
            website: "https://gateway.invalid/".into(),
        };
        let pulse = Request::GatewayPulse {
            metadata: metadata(),
            activity: central_api::gateway::GatewayActivity {
                players: 1,
                tables: 1,
                games: 1,
            },
        };
        let heartbeat = Request::GatewayHeartbeat {
            metadata: metadata(),
        };
        for sent in 0..64 {
            let request = if sent % 2 == 0 { &pulse } else { &heartbeat };
            limits.request(request, Some(&token), None, now).unwrap();
        }
        for request in [&pulse, &heartbeat] {
            assert_eq!(
                limits.request(request, Some(&token), None, now),
                Err(Error::Busy)
            );
        }
        // Without a credential a pulse is one more public request, refused
        // here with the budget and by dispatch otherwise.
        assert_eq!(limits.request(&pulse, None, None, now), Err(Error::Busy));
        // Neither took anything from a session's checks.
        limits
            .request(&Request::SessionStatus, Some(&token), None, now)
            .unwrap();
        limits
            .request(&figures, None, None, now + Duration::from_secs(1))
            .unwrap();
    }

    #[test]
    fn rate_limits_expire_and_have_bounded_cardinality() {
        let now = Instant::now();
        let mut limits = Limits::default();
        for _ in 0..32 {
            limits.global(None, now).unwrap();
        }
        assert_eq!(limits.global(None, now), Err(Error::Busy));
        limits.global(None, now + Duration::from_secs(1)).unwrap();
        for _ in 0..8 {
            limits.account([0; 16], now, 1).unwrap();
        }
        assert_eq!(limits.account([0; 16], now, 1), Err(Error::Busy));
        // However many accounts there are, none keeps another out
        // (`guesses.rs` bounds the table and what a full one forgets).
        for i in 1_u128..2048 {
            limits.account(i.to_be_bytes(), now, 1).unwrap();
        }
        assert_eq!(limits.account([0; 16], now, 1), Err(Error::Busy));
        limits
            .account([0; 16], now + Duration::from_secs(60), 1)
            .unwrap();
    }
    #[test]
    fn legacy_sign_ins_are_bounded_per_name_and_altogether() {
        let now = Instant::now();
        let mut limits = Limits::default();
        for _ in 0..LEGACY_PER_NAME {
            limits.legacy("Synthetic", now).unwrap();
        }
        // The same name in another case is the same name.
        assert_eq!(limits.legacy("SYNTHETIC", now), Err(Error::Busy));
        for i in 1..(LEGACY_PER_MINUTE / LEGACY_PER_NAME) {
            for _ in 0..LEGACY_PER_NAME {
                limits.legacy(&format!("name{i}"), now).unwrap();
            }
        }
        assert_eq!(
            limits.legacy("fresh", now),
            Err(Error::Busy),
            "all names together are bounded too"
        );
        let later = now + Duration::from_secs(60);
        limits.legacy("synthetic", later).unwrap();
        assert_eq!(limits.legacy_names.len(), 1, "a passed minute is forgotten");
    }

    #[test]
    fn a_password_change_costs_two_sign_ins_and_nothing_when_refused() {
        let now = Instant::now();
        let mut limits = Limits::default();
        for _ in 0..3 {
            limits.account([1; 16], now, 2).unwrap();
        }
        limits.account([1; 16], now, 1).unwrap();
        // Seven spent: a change's two do not fit, and spend nothing.
        assert_eq!(limits.account([1; 16], now, 2), Err(Error::Busy));
        limits.account([1; 16], now, 1).unwrap();
        assert_eq!(limits.account([1; 16], now, 1), Err(Error::Busy));
        limits
            .account([1; 16], now + Duration::from_secs(60), 2)
            .unwrap();
    }
}
