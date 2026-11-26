//! The native listener's budgets per client address (`docs/native-login.md`
//! §"Guessing a password online"), in memory only: how often one address
//! may ask what a guesser or a collector of names repeats, and how often
//! it may read the public directory and figures. The browser listener has
//! its own (`browser.rs`), with the same numbers where both serve an
//! operation. The address is the boundary's
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
/// Requests for a reset code by mail an hour (`docs/email.md`), on each
/// listener: a person asks once or twice, and each request may send a mail
/// to somebody else's address.
pub(crate) const RESET_MAIL: u32 = 5;

/// Readings a minute of the gateway directory and the public figures
/// together, as on the browser listener: both are read in pages, side by
/// side, and a minute's budget leaves room for paging.
pub(crate) const DIRECTORY: u32 = 60;

/// Deliveries to Ko-fi's webhook a minute (`docs/operations.md` §"Donations"):
/// Ko-fi sends one a payment from a few addresses of its own, and a minute's
/// thirty leave room for a burst of retries. Spent before the token is read, so
/// that guessing it costs a guesser this budget too.
pub(crate) const KOFI: u32 = 30;

/// Refused report tokens an hour (`docs/reports.md` §"Report tokens"): past
/// them the store is not asked about another from the address until the hour
/// is over. A token is 256 random bits, so this bounds the asking, not the
/// guessing; a script whose token was revoked meets it as well. A gateway's
/// credential is neither counted nor refused by it.
pub(crate) const REPORT_REFUSALS: u32 = 30;

/// AI runner exchanges an hour (`docs/ai-players.md` §2): a runner trades its
/// token for a session per AI account and rolls over every few hours, far
/// fewer than this.
pub(crate) const RUNNER_EXCHANGES: u32 = 60;
/// Refused runner exchanges an hour: past them the store is not asked about
/// another from the address until the hour is over, as for report tokens
/// ([`REPORT_REFUSALS`]). A runner whose token was revoked meets it too.
pub(crate) const RUNNER_REFUSALS: u32 = 30;

/// Each budget's addresses; one is forgotten once its window has passed.
#[derive(Default)]
pub(crate) struct Budgets {
    discover: Book,
    login: Book,
    legacy: Book,
    reset: Book,
    directory: Book,
    reset_mail: Book,
    kofi: Book,
    report_refusals: Book,
    runner: Book,
    runner_refusals: Book,
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
            Request::ResetMail { .. } => self.reset_mail.charge(client, now, RESET_MAIL, HOUR),
            Request::AiRunnerSession { .. } => {
                self.runner.charge(client, now, RUNNER_EXCHANGES, HOUR)
            }
            Request::Ranked { request } if request.is_public() => {
                self.directory.charge(client, now, DIRECTORY, MINUTE)
            }
            Request::GatewayDirectory { .. }
            | Request::PublicFigures { .. }
            | Request::PublicFunding {} => self.directory.charge(client, now, DIRECTORY, MINUTE),
            _ => true,
        }
    }

    /// Whether the store may be asked about a report token from `client`:
    /// its refusals in the hour are fewer than [`REPORT_REFUSALS`]. Requests
    /// at once are asked before either is refused, so they may pass it by as
    /// many as the listener serves at once.
    pub(crate) fn may_present(&self, client: Client, now: Instant) -> bool {
        !self
            .report_refusals
            .spent(client, now, REPORT_REFUSALS, HOUR)
    }

    /// Count a report token of `client` that was refused.
    pub(crate) fn report_refused(&mut self, client: Client, now: Instant) {
        self.report_refusals
            .charge(client, now, REPORT_REFUSALS, HOUR);
    }

    /// Whether the store may be asked about a runner token from `client`:
    /// its refused exchanges in the hour are fewer than [`RUNNER_REFUSALS`].
    /// Exchanges at once are asked before either is refused, so they may
    /// pass it by as many as the listener serves at once.
    pub(crate) fn may_exchange(&self, client: Client, now: Instant) -> bool {
        !self
            .runner_refusals
            .spent(client, now, RUNNER_REFUSALS, HOUR)
    }

    /// Count a runner exchange of `client` that was refused.
    pub(crate) fn exchange_refused(&mut self, client: Client, now: Instant) {
        self.runner_refusals
            .charge(client, now, RUNNER_REFUSALS, HOUR);
    }

    /// Spend one of `client`'s deliveries to Ko-fi's webhook; `false` when none is
    /// left.
    pub(crate) fn charge_kofi(&mut self, client: Client, now: Instant) -> bool {
        self.kofi.charge(client, now, KOFI, MINUTE)
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

    fn requests() -> [(Request, u32, Duration); 6] {
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
            (
                Request::ResetMail {
                    username: "synthetic".into(),
                    lang: "en".into(),
                },
                RESET_MAIL,
                HOUR,
            ),
            (
                Request::AiRunnerSession {
                    token: Token::generate().unwrap(),
                    account: [7; 16],
                },
                RUNNER_EXCHANGES,
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
        // One operation's budget is not another's: all six were served
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

    /// The directory and the public figures are one budget, as on the
    /// browser listener: they are read side by side, page after page.
    #[test]
    fn the_directory_and_the_figures_share_one_budget_an_address() {
        let start = Instant::now();
        let mut budgets = Budgets::default();
        let directory = Request::GatewayDirectory { after: None };
        let figures = Request::PublicFigures {
            after: Some([7; 16]),
        };
        let mut passed = 0;
        for turn in 0..2 * DIRECTORY {
            let request = if turn % 2 == 0 { &directory } else { &figures };
            passed += u32::from(budgets.charge(request, client(1), start));
        }
        assert_eq!(passed, DIRECTORY);
        let late = start + MINUTE.saturating_sub(Duration::from_secs(1));
        for request in [&directory, &figures] {
            assert!(!budgets.charge(request, client(1), late));
            // Another address is served meanwhile.
            assert!(budgets.charge(request, client(2), late));
        }
        // The address's other budgets are whole.
        for (request, limit, _) in requests() {
            assert_eq!(
                served(&mut budgets, &request, client(1), late, limit + 1),
                limit
            );
        }
        assert!(budgets.charge(&directory, client(1), start + MINUTE));
        // The donation figures are read beside them and spend the same budget.
        let mut budgets = Budgets::default();
        let funding = Request::PublicFunding {};
        assert_eq!(
            served(&mut budgets, &funding, client(1), start, DIRECTORY + 1),
            DIRECTORY
        );
        assert!(!budgets.charge(&directory, client(1), start));
        assert!(!budgets.charge(&figures, client(1), start));
        assert!(budgets.charge(&funding, client(2), start));
    }

    /// Ko-fi's deliveries have their own budget, which no other spends.
    #[test]
    fn ko_fi_s_deliveries_have_their_own_budget_an_address() {
        let start = Instant::now();
        let mut budgets = Budgets::default();
        let mut served = 0;
        for _ in 0..KOFI + 10 {
            served += u32::from(budgets.charge_kofi(client(1), start));
        }
        assert_eq!(served, KOFI);
        assert!(budgets.charge_kofi(client(2), start));
        // the address's directory and other budgets are whole
        assert!(budgets.charge(&Request::PublicFunding {}, client(1), start));
        for (request, limit, _) in requests() {
            assert_eq!(
                served_of(&mut budgets, &request, client(1), start, limit),
                limit
            );
        }
        assert!(budgets.charge_kofi(client(1), start + MINUTE));
    }

    /// An address's refused report tokens are the last the store is asked
    /// about for the hour, and nothing else is spent by them.
    #[test]
    fn refused_report_tokens_are_counted_by_their_address() {
        let start = Instant::now();
        let mut budgets = Budgets::default();
        for _ in 0..REPORT_REFUSALS {
            assert!(budgets.may_present(client(1), start));
            budgets.report_refused(client(1), start);
        }
        assert!(!budgets.may_present(client(1), start));
        assert!(budgets.may_present(client(2), start));
        let late = start + HOUR.saturating_sub(Duration::from_secs(1));
        // refusals past the bound do not prolong it
        budgets.report_refused(client(1), late);
        assert!(!budgets.may_present(client(1), late));
        assert!(budgets.may_present(client(1), start + HOUR));
        for (request, limit, _) in requests() {
            assert_eq!(
                served(&mut budgets, &request, client(1), start, limit),
                limit
            );
        }
        // asking remembers nobody
        assert!(budgets.may_present(client(3), start));
        assert!(!budgets.report_refusals.spent(client(3), start, 1, HOUR));
    }

    /// An address's refused runner exchanges are the last the store is asked
    /// about for the hour; they are counted apart from refused report tokens
    /// and spend nothing else.
    #[test]
    fn refused_runner_exchanges_are_counted_by_their_address() {
        let start = Instant::now();
        let mut budgets = Budgets::default();
        for _ in 0..RUNNER_REFUSALS {
            assert!(budgets.may_exchange(client(1), start));
            budgets.exchange_refused(client(1), start);
        }
        assert!(!budgets.may_exchange(client(1), start));
        assert!(budgets.may_exchange(client(2), start));
        // a report token from the address is still asked about, and the
        // other way round
        assert!(budgets.may_present(client(1), start));
        for _ in 0..REPORT_REFUSALS {
            budgets.report_refused(client(2), start);
        }
        assert!(budgets.may_exchange(client(2), start));
        let late = start + HOUR.saturating_sub(Duration::from_secs(1));
        budgets.exchange_refused(client(1), late);
        assert!(!budgets.may_exchange(client(1), late));
        assert!(budgets.may_exchange(client(1), start + HOUR));
        for (request, limit, _) in requests() {
            assert_eq!(
                served(&mut budgets, &request, client(1), start, limit),
                limit
            );
        }
        assert!(!budgets.runner_refusals.spent(client(3), start, 1, HOUR));
    }

    fn served_of(
        budgets: &mut Budgets,
        request: &Request,
        client: Client,
        now: Instant,
        count: u32,
    ) -> u32 {
        served(budgets, request, client, now, count)
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
