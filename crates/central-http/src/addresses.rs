//! The native listener's budgets per client address (`docs/native-login.md`
//! §"Guessing a password online"), in memory only: how often one address
//! may ask what a guesser or a collector of names repeats. The browser
//! listener has its own (`browser.rs`), with the same numbers where both
//! serve an operation. The address is the boundary's
//! ([`super::boundary::Origin`]); a request it knows none of has no budget
//! here and meets the account service's alone. A budget refuses with the
//! route's `Busy` and never grants anything.
use super::{
    browser::{Book, HOUR, MINUTE},
    in_flight::Client,
};
use central_api::wire::Request;
use std::time::Instant;

/// Discoveries a minute, so that names are collected no faster here than
/// through the website.
pub(crate) const DISCOVER: u32 = 30;
/// Sign-in starts a minute, apart from the discoveries. The account
/// service counts the guesses among them far more closely; this keeps one
/// address's refused ones from spending what everyone shares.
pub(crate) const LOGIN: u32 = 30;
/// Legacy sign-ins a minute: a third of what all names share (30), so no
/// one address spends that alone.
pub(crate) const LEGACY: u32 = 10;
/// Reset starts an hour: a reset needs a code, and a few tries leave room
/// for typing it.
pub(crate) const RESET: u32 = 10;

/// Each budget's addresses; one is forgotten once its window has passed.
#[derive(Default)]
pub(crate) struct Budgets {
    discover: Book,
    login: Book,
    legacy: Book,
    reset: Book,
}

impl Budgets {
    /// Spend one of `client`'s units for `request`; `false` when none is
    /// left. An operation without a budget here always passes: the account
    /// service counts each address's share of the requests that need no
    /// session, and a session's requests by the session.
    pub(crate) fn charge(&mut self, request: &Request, client: Client, now: Instant) -> bool {
        match request {
            Request::Discover { .. } => self.discover.charge(client, now, DISCOVER, MINUTE),
            Request::LoginStart { .. } => self.login.charge(client, now, LOGIN, MINUTE),
            Request::LegacyLogin { .. } => self.legacy.charge(client, now, LEGACY, MINUTE),
            Request::ResetStart { .. } => self.reset.charge(client, now, RESET, HOUR),
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use central_api::{Token, legacy::LegacyPassword, wire::Identity};
    use std::{net::IpAddr, time::Duration};

    fn client(last: u8) -> Client {
        Client::from(IpAddr::from([198, 51, 100, last]))
    }

    /// How many of `count` requests at `now` the address is served.
    fn served(
        budgets: &mut Budgets,
        request: &Request,
        client: Client,
        now: Instant,
        count: u32,
    ) -> u32 {
        let mut served = 0;
        for _ in 0..count {
            served += u32::from(budgets.charge(request, client, now));
        }
        served
    }

    fn requests() -> [(Request, u32, Duration); 4] {
        [
            (
                Request::Discover {
                    username: "synthetic".into(),
                },
                DISCOVER,
                MINUTE,
            ),
            (
                Request::LoginStart {
                    identity: Identity {
                        account: [7; 16],
                        epoch: 1,
                    },
                    message: vec![0; 32],
                },
                LOGIN,
                MINUTE,
            ),
            (
                Request::LegacyLogin {
                    username: "synthetic".into(),
                    password: LegacyPassword::new("synthetic words".into()),
                },
                LEGACY,
                MINUTE,
            ),
            (
                Request::ResetStart {
                    username: "synthetic".into(),
                    code: central_api::reset_code::ResetCode::generate().unwrap(),
                    registration: Vec::new(),
                },
                RESET,
                HOUR,
            ),
        ]
    }

    #[test]
    fn one_address_has_its_budget_for_each_operation_and_no_other_s() {
        let start = Instant::now();
        let mut budgets = Budgets::default();
        for (request, limit, period) in requests() {
            // Its budget and no more, whatever it asks for or about.
            assert_eq!(
                served(&mut budgets, &request, client(1), start, limit + 5),
                limit
            );
            // Another address is served meanwhile.
            assert!(budgets.charge(&request, client(2), start));
            // Until the window has passed; then it starts again.
            let late = start + period.saturating_sub(Duration::from_secs(1));
            assert!(!budgets.charge(&request, client(1), late));
            assert!(budgets.charge(&request, client(1), start + period));
        }
        // One operation's budget is not another's: all four were served
        // for the same address above. A request that needs a session, or
        // any other, has none here.
        for _ in 0..1_000 {
            assert!(budgets.charge(&Request::Terms, client(1), start));
            assert!(budgets.charge(&Request::SessionStatus, client(1), start));
            assert!(budgets.charge(
                &Request::LoginFinish {
                    attempt: Token::generate().unwrap(),
                    message: Vec::new(),
                },
                client(1),
                start
            ));
        }
    }

    #[test]
    fn an_ipv6_subscriber_is_one_address() {
        let start = Instant::now();
        let mut budgets = Budgets::default();
        let (request, limit, _) = requests().into_iter().next().unwrap();
        let of = |text: &str| Client::from(text.parse::<IpAddr>().unwrap());
        let (one, same, other) = (
            of("2001:db8:1:2::1"),
            of("2001:db8:1:2:ffff::9"),
            of("2001:db8:1:3::1"),
        );
        assert_eq!(served(&mut budgets, &request, one, start, limit), limit);
        assert!(!budgets.charge(&request, same, start));
        assert!(budgets.charge(&request, other, start));
    }
}
