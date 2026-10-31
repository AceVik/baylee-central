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

#[derive(Default)]
pub(super) struct Limits {
    global: Option<(Instant, u32)>,
    accounts: BTreeMap<[u8; 16], (Instant, u32)>,
    legacy: Option<(Instant, u32)>,
    legacy_names: BTreeMap<String, (Instant, u32)>,
    data: Window,
    control: Window,
    pairing: Window,
    peers: BTreeMap<([u8; 32], bool), Window>,
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
    pub(super) fn request(
        &mut self,
        request: &Request,
        bearer: Option<&Token>,
        now: Instant,
    ) -> Result<(), Error> {
        // A client's claim, polls and redeem carry no session; their own
        // window keeps a few pairings' polling from spending the sign-ins'.
        if matches!(
            request,
            Request::PairClaim { .. } | Request::PairPoll { .. } | Request::PairRedeem { .. }
        ) {
            return self.pairing.charge(now, 64);
        }
        if matches!(
            request,
            Request::Discover { .. }
                | Request::LoginStart { .. }
                | Request::LoginFinish { .. }
                | Request::Register { .. }
                | Request::EnrollmentStart { .. }
                | Request::EnrollmentFinish { .. }
                | Request::GatewayDirectory { .. }
                | Request::Terms
                | Request::GatewayTerms { .. }
                | Request::ResetStart { .. }
                | Request::ResetFinish { .. }
                | Request::LegacyLogin { .. }
        ) {
            return self.global(now);
        }
        let Some(bearer) = bearer else {
            return self.global(now);
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
    pub(super) fn global(&mut self, now: Instant) -> Result<(), Error> {
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

    /// Spend `units` of the account's eight OPAQUE evaluations a minute: a
    /// sign-in's start is one, a password change's start two (it answers a
    /// registration response besides KE2, and each is an evaluation of the
    /// account's OPRF key that can test one guessed password). Nothing is
    /// spent when the units do not fit.
    pub(super) fn account(
        &mut self,
        account: [u8; 16],
        now: Instant,
        units: u32,
    ) -> Result<(), Error> {
        self.accounts
            .retain(|_, (start, _)| now.duration_since(*start) < Duration::from_secs(60));
        if !self.accounts.contains_key(&account) && self.accounts.len() >= 1024 {
            return Err(Error::Busy);
        }
        let (_, used) = self.accounts.entry(account).or_insert((now, 0));
        if *used + units > 8 {
            return Err(Error::Busy);
        }
        *used += units;
        Ok(())
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
                .request(&Request::CreateObject, Some(&token), now)
                .unwrap();
        }
        assert_eq!(
            limits.request(&Request::CreateObject, Some(&token), now),
            Err(Error::Busy)
        );
        limits
            .request(
                &Request::Discover {
                    username: "synthetic".into(),
                },
                None,
                now,
            )
            .unwrap();
        limits
            .request(&Request::SessionStatus, Some(&token), now)
            .unwrap();
        limits.request(&Request::Logout, Some(&token), now).unwrap();
        limits
            .request(
                &Request::CreateObject,
                Some(&token),
                now + Duration::from_secs(1),
            )
            .unwrap();
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
            limits.request(&request, Some(&token), now).unwrap();
        }
        assert_eq!(limits.request(&request, None, now), Err(Error::Busy));
        assert_eq!(
            limits.request(&request, Some(&Token::generate().unwrap()), now),
            Err(Error::Busy)
        );
    }
    #[test]
    fn rate_limits_expire_and_have_bounded_cardinality() {
        let now = Instant::now();
        let mut limits = Limits::default();
        for _ in 0..32 {
            limits.global(now).unwrap();
        }
        assert_eq!(limits.global(now), Err(Error::Busy));
        limits.global(now + Duration::from_secs(1)).unwrap();
        for _ in 0..8 {
            limits.account([0; 16], now, 1).unwrap();
        }
        assert_eq!(limits.account([0; 16], now, 1), Err(Error::Busy));
        for i in 1_u128..1024 {
            limits.account(i.to_be_bytes(), now, 1).unwrap();
        }
        assert_eq!(limits.account([255; 16], now, 1), Err(Error::Busy));
        limits
            .account([255; 16], now + Duration::from_secs(60), 1)
            .unwrap();
        assert_eq!(limits.accounts.len(), 1);
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
