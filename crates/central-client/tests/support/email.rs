//! An account's e-mail address and a reset code by mail over the real
//! store (`docs/email.md`), with a mailer of the test's own in the
//! mailer's place: no mail leaves the process, and every address is in a
//! reserved domain.
//!
//! Most of it asks a second service on the fixture's store, as Central is
//! after a restart, directly: its budgets are its own, so nothing here
//! spends what the listeners' service counts. The last part asks through
//! the real listeners, where each address has its hour's requests.
use super::{
    Http,
    browser::{Browser, accept_terms},
    pace,
    password::{DOCUMENT, NEW, OLD, Registered, browser_sign_in, context, register, sign_in},
};
use central_api::{
    Error, Token,
    admin::{AdminAnswer, AdminRequest},
    email::{EmailRequest, EmailState, MailCode},
    reset_code::ResetCode,
    wire::{Identity, Request, Response},
};
use central_client::{ResetData, Transport};
use central_crypto::opaque::server::ServerKeys;
use central_identity::{
    Delivery, IdentityService, Seen,
    mail::{Mail, Mailer, Outcome, What},
};
use central_postgres::AccountStore;
use sea_orm::{ConnectionTrait, DatabaseConnection, prelude::Uuid};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::runtime::Runtime;

/// One mail as the service handed it to the mailer.
#[derive(Clone)]
struct Handed {
    to: String,
    account: [u8; 16],
    lang: &'static str,
    what: What,
    code: Option<String>,
    ttl: Option<u32>,
}

/// A mailer of the test's own: it keeps every mail handed to it and says
/// what the test scripted, `Sent` when nothing is scripted.
#[derive(Default)]
pub(crate) struct Recorder {
    handed: Mutex<Vec<Handed>>,
    answers: Mutex<VecDeque<Outcome>>,
}

impl Mailer for Recorder {
    fn send(&self, mail: &Mail) -> Outcome {
        self.handed.lock().unwrap().push(Handed {
            to: mail.to().to_owned(),
            account: mail.account(),
            lang: mail.lang().code(),
            what: mail.what(),
            code: mail.code().map(str::to_owned),
            ttl: mail.ttl_secs(),
        });
        self.answers
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(Outcome::Sent)
    }
}

impl Recorder {
    fn count(&self) -> usize {
        self.handed.lock().unwrap().len()
    }

    fn last(&self) -> Handed {
        self.handed.lock().unwrap().last().unwrap().clone()
    }

    /// The code of the last mail handed over.
    fn code(&self) -> String {
        self.last().code.expect("a mail with a code")
    }

    /// What the mailer says to the next mail.
    fn next(&self, outcome: Outcome) {
        self.answers.lock().unwrap().push_back(outcome);
    }
}

/// What the fixture lends this module.
pub(crate) struct Fixture<'a> {
    pub(crate) http: &'a Http,
    pub(crate) browser: &'a Browser,
    /// The listeners' own service, which sends through `mailer`.
    pub(crate) service: &'a IdentityService,
    pub(crate) mailer: &'a Arc<Recorder>,
    pub(crate) store: &'a AccountStore,
    pub(crate) keys: &'a ServerKeys,
    pub(crate) bridge: &'a Token,
    pub(crate) runtime: &'a Runtime,
    pub(crate) operator: &'a DatabaseConnection,
}

impl Fixture<'_> {
    /// A second service on the same store with the same keys, which has
    /// counted nothing yet; it sends through the fixture's mailer or, with
    /// `mail` false, has none.
    fn central(&self, mail: bool) -> IdentityService {
        let service = IdentityService::new(
            self.store.clone(),
            &self.http.endpoint,
            ServerKeys::from_secret_bytes(&self.keys.to_secret_bytes()).unwrap(),
            self.bridge,
            "http://127.0.0.1:28766",
        )
        .unwrap()
        .with_terms(Some(super::terms::central_document()));
        if mail {
            service.with_mailer(self.mailer.clone())
        } else {
            service
        }
    }

    fn sql(&self, statement: &str) {
        self.runtime
            .block_on(self.operator.execute_unprepared(statement))
            .unwrap();
    }
}

/// A service asked directly, as its listeners ask it.
struct Asked<'a> {
    service: &'a IdentityService,
    runtime: &'a Runtime,
}

impl Asked<'_> {
    /// The website's request, with the cookie's session.
    fn web(&self, cookie: &Token, request: EmailRequest) -> Result<EmailState, Error> {
        pace();
        let reply = self.runtime.block_on(self.service.handle_browser_from(
            Request::Email { request },
            Some(cookie),
            None,
            Seen::default(),
        ))?;
        assert!(reply.session.is_none());
        let Response::Email { state } = reply.response else {
            panic!("an address's state");
        };
        Ok(state)
    }

    /// The client's request, with its bearer.
    fn native(&self, bearer: &Token, request: EmailRequest) -> Result<EmailState, Error> {
        pace();
        let response = self.runtime.block_on(self.service.handle_from(
            Request::Email { request },
            Some(bearer),
            None,
        ))?;
        let Response::Email { state } = response else {
            panic!("an address's state");
        };
        Ok(state)
    }

    /// A request for a reset code by mail on the account listener: the
    /// answer's bytes.
    fn reset(&self, name: &str) -> Result<Vec<u8>, Error> {
        pace();
        let response = self.runtime.block_on(self.service.handle_from(
            Request::ResetMail {
                username: name.into(),
                lang: "en".into(),
            },
            None,
            None,
        ))?;
        Ok(central_api::encode(&response).unwrap())
    }

    /// The same on the browser listener, with a cookie that came along.
    fn reset_on_the_site(&self, name: &str, cookie: Option<&Token>) -> Result<Vec<u8>, Error> {
        pace();
        let reply = self.runtime.block_on(self.service.handle_browser_from(
            Request::ResetMail {
                username: name.into(),
                lang: "en".into(),
            },
            cookie,
            None,
            Seen::default(),
        ))?;
        assert!(reply.session.is_none());
        Ok(central_api::encode(&reply.response).unwrap())
    }

    fn deliver(&self) -> Option<Delivery> {
        self.runtime.block_on(self.service.deliver_mail())
    }
}

/// A registered account with the terms accepted and a fresh sign-in on
/// each listener.
struct Account {
    name: &'static str,
    registered: Registered,
    cookie: Token,
    bearer: Token,
}

impl Account {
    fn identity(&self) -> Identity {
        self.registered.identity
    }
}

fn account(fixture: &Fixture<'_>, home: &Browser, name: &'static str) -> Account {
    let registered = register(name, fixture.http, home);
    let identity = registered.identity;
    let cookie = browser_sign_in(home, identity, context(fixture.http, identity), OLD);
    accept_terms(home, &cookie);
    let (bearer, _) = sign_in(fixture.http, identity, OLD);
    Account {
        name,
        registered,
        cookie,
        bearer,
    }
}

fn set(address: &str, lang: &str) -> EmailRequest {
    EmailRequest::Set {
        address: address.into(),
        lang: lang.into(),
    }
}

fn resend(lang: &str) -> EmailRequest {
    EmailRequest::Resend { lang: lang.into() }
}

fn confirm(code: &str) -> EmailRequest {
    EmailRequest::Confirm {
        code: MailCode::parse(code).unwrap(),
    }
}

fn wrong() -> EmailRequest {
    EmailRequest::Confirm {
        code: MailCode::generate().unwrap(),
    }
}

/// The state names exactly this address and this one that waits, each
/// with its time, on a Central that sends mail.
fn holds(state: &EmailState, address: Option<&str>, pending: Option<&str>) {
    assert!(state.mail);
    assert_eq!(state.address.as_deref(), address);
    assert_eq!(state.pending.as_deref(), pending);
    assert_eq!(state.confirmed_at.is_some(), address.is_some());
    assert_eq!(state.pending_until.is_some(), pending.is_some());
}

pub(crate) fn run(fixture: &Fixture<'_>) {
    let central = fixture.central(true);
    let asked = Asked {
        service: &central,
        runtime: fixture.runtime,
    };
    let home = Browser {
        address: fixture.browser.address,
        client: "198.51.100.90",
    };
    let other = Browser {
        address: fixture.browser.address,
        client: "198.51.100.91",
    };
    let owner = owner(fixture, &asked, &home);
    wrong_codes(fixture, &asked, &owner);
    let flaky = refusals(fixture, &asked, &home);
    let plain = account(fixture, &other, "mail-d");
    stale(fixture, &plain);
    resets(fixture, &asked, &other, &plain);
    unsent(fixture, &asked, &flaky);
    without_a_mailer(fixture, &asked, &flaky);
    whole(fixture);
    listeners(fixture, &home, &owner);
}

const FIRST: &str = "mail-a@example.org";
const FOURTH: &str = "mail-a-fourth@example.org";
const SECOND: &str = "mail-a-second@example.org";

/// An account names an address, confirms it with the mailed code, replaces
/// it and removes it; the address that lost its place is told each time.
fn owner(fixture: &Fixture<'_>, asked: &Asked<'_>, home: &Browser) -> Account {
    let mailer = fixture.mailer;
    let registered = register("mail-a", fixture.http, home);
    let identity = registered.identity;
    let cookie = browser_sign_in(home, identity, context(fixture.http, identity), OLD);
    // Before Central's terms are accepted the address can be read,
    // withdrawn and removed, and not named.
    holds(
        &asked.web(&cookie, EmailRequest::Status).unwrap(),
        None,
        None,
    );
    holds(
        &asked.web(&cookie, EmailRequest::Withdraw).unwrap(),
        None,
        None,
    );
    holds(
        &asked.web(&cookie, EmailRequest::Remove).unwrap(),
        None,
        None,
    );
    assert_eq!(
        asked
            .web(
                &cookie,
                EmailRequest::Set {
                    address: FIRST.into(),
                    lang: "en".into(),
                },
            )
            .err(),
        Some(Error::TermsRequired)
    );
    assert_eq!(mailer.count(), 0);
    accept_terms(home, &cookie);
    let (bearer, _) = sign_in(fixture.http, identity, OLD);
    holds(
        &asked.web(&cookie, EmailRequest::Status).unwrap(),
        None,
        None,
    );
    // Each listener takes its own kind of session and no other.
    for refused in [
        asked.web(&bearer, EmailRequest::Status),
        asked.native(&cookie, EmailRequest::Status),
        asked.native(&Token::generate().unwrap(), EmailRequest::Status),
    ] {
        assert_eq!(refused.err(), Some(Error::Rejected));
    }
    // What is no address or no language tag is refused before any mail.
    for (address, lang) in [
        ("not an address", "en"),
        ("mail-a@example.org\r\nBcc: other@example.org", "en"),
        (FIRST, "EN"),
        (FIRST, ""),
    ] {
        assert_eq!(
            asked.web(&cookie, set(address, lang)).err(),
            Some(Error::Rejected)
        );
    }
    assert_eq!(mailer.count(), 0);
    // The address waits for its code, a quarter of an hour.
    let state = asked.web(&cookie, set(FIRST, "de")).unwrap();
    holds(&state, None, Some(FIRST));
    let now = i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    )
    .unwrap();
    assert!((state.pending_until.unwrap() - now - 900).abs() <= 30);
    let mail = mailer.last();
    assert_eq!((mail.to.as_str(), mail.lang), (FIRST, "de"));
    assert_eq!((mail.what, mail.ttl), (What::Confirm, Some(900)));
    assert_eq!(mail.account, identity.account);
    let code = mailer.code();
    assert_eq!(code.len(), 9, "XXXX-XXXX");
    // A wrong code confirms nothing; the right one does, from the account's
    // other session too. Nobody is told of a first address.
    assert_eq!(asked.web(&cookie, wrong()).err(), Some(Error::Rejected));
    holds(
        &asked.native(&bearer, confirm(&code)).unwrap(),
        Some(FIRST),
        None,
    );
    assert_eq!(asked.deliver(), None);
    // A code confirms once.
    assert_eq!(
        asked.web(&cookie, confirm(&code)).err(),
        Some(Error::Rejected)
    );
    let account = Account {
        name: "mail-a",
        registered,
        cookie,
        bearer,
    };
    replaced_and_removed(fixture, asked, &account);
    account
}

/// The second half of [`owner`]: another address replaces the first, and
/// the address is removed.
fn replaced_and_removed(fixture: &Fixture<'_>, asked: &Asked<'_>, account: &Account) {
    let mailer = fixture.mailer;
    let cookie = &account.cookie;
    // The address the account has already is no new one: no mail.
    let before = mailer.count();
    assert_eq!(
        asked.web(cookie, set(FIRST, "en")).err(),
        Some(Error::Conflict)
    );
    assert_eq!(mailer.count(), before);
    // Another address waits while the first stays in use.
    let state = asked.web(cookie, set(SECOND, "en")).unwrap();
    holds(&state, Some(FIRST), Some(SECOND));
    let earlier = mailer.code();
    // A new code for the address that waits: the earlier one ends.
    holds(
        &asked.web(cookie, resend("fr")).unwrap(),
        Some(FIRST),
        Some(SECOND),
    );
    let mail = mailer.last();
    assert_eq!((mail.to.as_str(), mail.lang), (SECOND, "fr"));
    assert_eq!((mail.what, mail.ttl), (What::Confirm, Some(900)));
    let code = mailer.code();
    assert_ne!(code, earlier);
    assert_eq!(
        asked.web(cookie, confirm(&earlier)).err(),
        Some(Error::Rejected)
    );
    holds(
        &asked.web(cookie, confirm(&code)).unwrap(),
        Some(SECOND),
        None,
    );
    // The replaced address is told, in the language it was confirmed in,
    // once, without a code and without a time to live.
    assert_eq!(asked.deliver(), Some(Delivery::Sent));
    assert_eq!(asked.deliver(), None);
    let notice = mailer.last();
    assert_eq!((notice.to.as_str(), notice.lang), (FIRST, "de"));
    assert_eq!((notice.what, notice.ttl), (What::Replaced, None));
    assert!(notice.code.is_none());
    assert_eq!(notice.account, account.identity().account);
    // Giving up what waits: nothing to give up is no error; a withdrawn
    // address's code confirms nothing, and no new code is mailed for it.
    holds(
        &asked.web(cookie, EmailRequest::Withdraw).unwrap(),
        Some(SECOND),
        None,
    );
    asked
        .web(cookie, set("mail-a-third@example.org", "en"))
        .unwrap();
    let withdrawn = mailer.code();
    holds(
        &asked
            .native(&account.bearer, EmailRequest::Withdraw)
            .unwrap(),
        Some(SECOND),
        None,
    );
    let before = mailer.count();
    for request in [confirm(&withdrawn), resend("en")] {
        assert_eq!(asked.web(cookie, request).err(), Some(Error::Rejected));
    }
    assert_eq!(mailer.count(), before);
    // Removed, and the removed address is told.
    holds(
        &asked.web(cookie, EmailRequest::Remove).unwrap(),
        None,
        None,
    );
    assert_eq!(asked.deliver(), Some(Delivery::Sent));
    let notice = mailer.last();
    assert_eq!((notice.to.as_str(), notice.lang), (SECOND, "fr"));
    assert_eq!((notice.what, notice.ttl), (What::Removed, None));
    assert!(notice.code.is_none());
    // Nothing to remove is no error and tells nobody.
    holds(
        &asked.web(cookie, EmailRequest::Remove).unwrap(),
        None,
        None,
    );
    assert_eq!(asked.deliver(), None);
}

/// Five wrong codes give the address that waits up; the account's sixth
/// mail of the hour is refused, and no mail goes out for it.
fn wrong_codes(fixture: &Fixture<'_>, asked: &Asked<'_>, account: &Account) {
    let mailer = fixture.mailer;
    let cookie = &account.cookie;
    // The account's fifth mail with a code this hour.
    asked.web(cookie, set(FOURTH, "en")).unwrap();
    let code = mailer.code();
    for tried in 1..=5 {
        assert_eq!(asked.web(cookie, wrong()).err(), Some(Error::Rejected));
        let state = asked.web(cookie, EmailRequest::Status).unwrap();
        let waits = (tried < 5).then_some(FOURTH);
        holds(&state, None, waits);
    }
    // The right code comes too late.
    assert_eq!(
        asked.web(cookie, confirm(&code)).err(),
        Some(Error::Rejected)
    );
    let before = mailer.count();
    assert_eq!(
        asked.web(cookie, set(FOURTH, "en")).err(),
        Some(Error::Busy)
    );
    assert_eq!(
        asked
            .native(&account.bearer, set("mail-a-fifth@example.org", "en"))
            .err(),
        Some(Error::Busy)
    );
    assert_eq!(mailer.count(), before);
    holds(
        &asked.web(cookie, EmailRequest::Status).unwrap(),
        None,
        None,
    );
}

const KEPT: &str = "mail-f@example.org";
const WAITING: &str = "mail-f-second@example.org";

/// A mail the mailer does not take leaves nothing new behind and is
/// answered `Busy`; an address the mailer refuses is `Rejected`. The
/// attempts count: the sixth of the hour is not handed over at all.
fn refusals(fixture: &Fixture<'_>, asked: &Asked<'_>, home: &Browser) -> Account {
    let mailer = fixture.mailer;
    let account = account(fixture, home, "mail-f");
    let cookie = &account.cookie;
    asked.web(cookie, set(KEPT, "en")).unwrap();
    let code = mailer.code();
    for (said, answered, request) in [
        (Outcome::Busy, Error::Busy, set(WAITING, "en")),
        (Outcome::Unavailable, Error::Busy, resend("en")),
        (Outcome::Rejected, Error::Rejected, set(WAITING, "en")),
    ] {
        let before = mailer.count();
        mailer.next(said);
        assert_eq!(asked.web(cookie, request).err(), Some(answered));
        assert_eq!(mailer.count(), before + 1, "handed over and not taken");
        // What waited before still waits, with the code it had.
        holds(
            &asked.web(cookie, EmailRequest::Status).unwrap(),
            None,
            Some(KEPT),
        );
    }
    holds(
        &asked.web(cookie, confirm(&code)).unwrap(),
        Some(KEPT),
        None,
    );
    // The fifth mail of the hour; a sixth is not handed over.
    holds(
        &asked.web(cookie, set(WAITING, "en")).unwrap(),
        Some(KEPT),
        Some(WAITING),
    );
    let before = mailer.count();
    assert_eq!(asked.web(cookie, resend("en")).err(), Some(Error::Busy));
    assert_eq!(mailer.count(), before);
    account
}

/// A sign-in older than 30 minutes names no address; it still reads, and
/// gives up what waits. Asked of the listeners' own service.
fn stale(fixture: &Fixture<'_>, account: &Account) {
    let asked = Asked {
        service: fixture.service,
        runtime: fixture.runtime,
    };
    let mailer = fixture.mailer;
    let aged = format!(
        "UPDATE central_identity.sessions SET created_at = created_at - interval '6 minutes'
         WHERE account_id = '{}'",
        Uuid::from_bytes(account.identity().account)
    );
    fixture.sql(&aged);
    let before = mailer.count();
    for refused in [
        asked.web(&account.cookie, set("mail-d@example.org", "en")),
        asked.native(&account.bearer, set("mail-d@example.org", "en")),
    ] {
        assert_eq!(refused.err(), Some(Error::Rejected));
    }
    assert_eq!(mailer.count(), before);
    holds(
        &asked.web(&account.cookie, EmailRequest::Status).unwrap(),
        None,
        None,
    );
    fixture.sql(&aged.replace("- interval", "+ interval"));
    // Fresh again, the address waits; it is no address of the account's
    // until it is confirmed, which a reset by mail below relies on.
    holds(
        &asked
            .web(&account.cookie, set("mail-d@example.org", "en"))
            .unwrap(),
        None,
        Some("mail-d@example.org"),
    );
    fixture.sql(&aged);
    holds(
        &asked.web(&account.cookie, resend("de")).unwrap(),
        None,
        Some("mail-d@example.org"),
    );
    assert_eq!(mailer.count(), before + 2);
}

/// Give `account` the confirmed address `address`.
fn addressed(fixture: &Fixture<'_>, asked: &Asked<'_>, account: &Account, address: &str) {
    asked.web(&account.cookie, set(address, "en")).unwrap();
    let code = fixture.mailer.code();
    holds(
        &asked.web(&account.cookie, confirm(&code)).unwrap(),
        Some(address),
        None,
    );
}

/// Move every time a reset code was mailed for `account` into the past.
fn age_mails(fixture: &Fixture<'_>, account: &Account) {
    fixture.sql(&format!(
        "UPDATE central_identity.account_emails SET reset_mailed = ARRAY(
             SELECT t - interval '11 minutes' FROM unnest(reset_mailed) t ORDER BY t)
         WHERE account_id = '{}'",
        Uuid::from_bytes(account.identity().account)
    ));
}

/// A reset code by mail: the answer is the same for every name, a code
/// goes to an ordinary account's confirmed address and to nobody else,
/// it is redeemed as an administrator's is, and it never replaces one.
fn resets(fixture: &Fixture<'_>, asked: &Asked<'_>, home: &Browser, plain: &Account) {
    let mailer = fixture.mailer;
    let admin = account(fixture, home, "mail-b");
    addressed(fixture, asked, &admin, "mail-b@example.org");
    fixture
        .runtime
        .block_on(central_postgres::appoint_admin(fixture.operator, "mail-b"))
        .unwrap();
    let ordinary = account(fixture, home, "mail-c");
    addressed(fixture, asked, &ordinary, "mail-c@example.org");
    // A name that is nobody's, an account whose address only waits, an
    // administrator's and an ordinary one: one answer.
    let names = ["mail-nobody", plain.name, admin.name, ordinary.name];
    let answers: Vec<Vec<u8>> = names
        .iter()
        .map(|name| asked.reset(name).unwrap())
        .collect();
    assert!(answers.iter().all(|answer| answer == &answers[0]));
    assert_eq!(answers[0], central_api::encode(&Response::Done).unwrap());
    // Afterwards, off the requests: one mail, to the ordinary account.
    let before = mailer.count();
    let done: Vec<Option<Delivery>> = names.iter().map(|_| asked.deliver()).collect();
    assert_eq!(
        done,
        [
            Some(Delivery::Nothing),
            Some(Delivery::Nothing),
            Some(Delivery::Nothing),
            Some(Delivery::Sent)
        ]
    );
    assert_eq!(asked.deliver(), None);
    assert_eq!(mailer.count(), before + 1);
    let mail = mailer.last();
    assert_eq!((mail.to.as_str(), mail.lang), ("mail-c@example.org", "en"));
    assert_eq!((mail.what, mail.ttl), (What::Reset, Some(3_600)));
    assert_eq!(mail.account, ordinary.identity().account);
    let reset_code = mailer.code();
    // The website's listener answers the same, whatever cookie comes
    // along; within ten minutes no second code is made.
    for cookie in [None, Some(&ordinary.cookie)] {
        assert_eq!(
            asked.reset_on_the_site(ordinary.name, cookie).unwrap(),
            answers[0]
        );
        assert_eq!(asked.deliver(), Some(Delivery::Nothing));
    }
    // Texts that are no name are answered alike and mail nothing.
    let long = "a".repeat(300);
    for name in ["", long.as_str(), "mail-c\r\n", "mail-c@example.org"] {
        assert_eq!(asked.reset(name).unwrap(), answers[0]);
    }
    while let Some(done) = asked.deliver() {
        assert_eq!(done, Delivery::Nothing);
    }
    assert_eq!(mailer.count(), before + 1);
    redeemed(fixture, asked, &admin, &ordinary, &reset_code);
}

/// The second half of [`resets`]: the mailed code sets a new password as
/// an administrator's does, once; a request by mail leaves an
/// administrator's code as it is.
fn redeemed(
    fixture: &Fixture<'_>,
    asked: &Asked<'_>,
    admin: &Account,
    ordinary: &Account,
    reset_code: &str,
) {
    let (http, mailer) = (fixture.http, fixture.mailer);
    let code = || ResetCode::parse(reset_code).unwrap();
    let kept = ResetData::Recovery(&ordinary.registered.code);
    assert!(
        central_client::reset(http, ordinary.name.into(), code(), NEW.into(), kept)
            .unwrap()
            .is_none()
    );
    let session = central_client::login(http, ordinary.identity(), NEW.into()).unwrap();
    assert_eq!(session.document(), DOCUMENT);
    session.logout(http).unwrap();
    let kept = ResetData::Recovery(&ordinary.registered.code);
    assert_eq!(
        central_client::reset(http, ordinary.name.into(), code(), OLD.into(), kept).err(),
        Some(Error::Rejected)
    );
    // An administrator makes a code for the account. Ten minutes later a
    // request by mail makes none: the administrator's stays and works.
    let site = Browser {
        address: fixture.browser.address,
        client: "198.51.100.91",
    };
    let Response::Admin {
        answer: AdminAnswer::ResetCode { code: issued, .. },
    } = site
        .answer(
            &Request::Admin {
                request: AdminRequest::ResetLogin {
                    account: ordinary.identity().account,
                },
            },
            Some(&admin.cookie),
        )
        .unwrap()
    else {
        panic!("a reset code");
    };
    age_mails(fixture, ordinary);
    let before = mailer.count();
    asked.reset(ordinary.name).unwrap();
    assert_eq!(asked.deliver(), Some(Delivery::Nothing));
    assert_eq!(mailer.count(), before);
    assert!(
        central_client::reset(
            http,
            ordinary.name.into(),
            issued,
            OLD.into(),
            ResetData::New(DOCUMENT)
        )
        .unwrap()
        .is_some()
    );
    // With the administrator's code spent, the same request mails one.
    asked.reset(ordinary.name).unwrap();
    assert_eq!(asked.deliver(), Some(Delivery::Sent));
    assert_eq!(mailer.count(), before + 1);
    assert_eq!(mailer.last().what, What::Reset);
}

/// A reset code whose mail the mailer does not take is taken back, and
/// the try stays counted.
fn unsent(fixture: &Fixture<'_>, asked: &Asked<'_>, account: &Account) {
    let mailer = fixture.mailer;
    let before = mailer.count();
    mailer.next(Outcome::Unavailable);
    asked.reset(account.name).unwrap();
    assert_eq!(asked.deliver(), Some(Delivery::Failed));
    assert_eq!(mailer.count(), before + 1);
    let mail = mailer.last();
    assert_eq!((mail.to.as_str(), mail.what), (KEPT, What::Reset));
    // The code that was in the mail starts no reset.
    assert_eq!(
        central_client::reset(
            fixture.http,
            account.name.into(),
            ResetCode::parse(&mailer.code()).unwrap(),
            NEW.into(),
            ResetData::New(DOCUMENT)
        )
        .err(),
        Some(Error::Rejected)
    );
    // Asked again at once, nothing is made: not within ten minutes.
    asked.reset(account.name).unwrap();
    assert_eq!(asked.deliver(), Some(Delivery::Nothing));
    assert_eq!(mailer.count(), before + 1);
}

/// A Central without a mailer: an address is read, given up and removed;
/// none is named, mailed a code or confirmed, and no reset code is mailed,
/// whatever the name.
fn without_a_mailer(fixture: &Fixture<'_>, mailing: &Asked<'_>, account: &Account) {
    let mailer = fixture.mailer;
    let central = fixture.central(false);
    let asked = Asked {
        service: &central,
        runtime: fixture.runtime,
    };
    let cookie = &account.cookie;
    let before = mailer.count();
    // The code of the address that waits, mailed before.
    let code = mailer
        .handed
        .lock()
        .unwrap()
        .iter()
        .rev()
        .find(|mail| mail.to == WAITING && mail.what == What::Confirm)
        .and_then(|mail| mail.code.clone())
        .unwrap();
    let state = asked.web(cookie, EmailRequest::Status).unwrap();
    assert!(!state.mail);
    assert_eq!(state.address.as_deref(), Some(KEPT));
    assert_eq!(state.pending.as_deref(), Some(WAITING));
    for request in [
        set("mail-f-third@example.org", "en"),
        resend("en"),
        confirm(&code),
        wrong(),
    ] {
        assert_eq!(asked.web(cookie, request).err(), Some(Error::Rejected));
    }
    for name in [account.name, "mail-nobody", "mail-c"] {
        assert_eq!(asked.reset(name).err(), Some(Error::Rejected));
        assert_eq!(
            asked.reset_on_the_site(name, None).err(),
            Some(Error::Rejected)
        );
    }
    assert_eq!(asked.deliver(), None);
    // Nothing was counted against the address that waits: where mail is
    // sent, it is still there with its tries.
    holds(
        &mailing.web(cookie, EmailRequest::Status).unwrap(),
        Some(KEPT),
        Some(WAITING),
    );
    let state = asked.web(cookie, EmailRequest::Withdraw).unwrap();
    assert_eq!(state.address.as_deref(), Some(KEPT));
    assert!(state.pending.is_none());
    let state = asked.native(&account.bearer, EmailRequest::Remove).unwrap();
    assert!(!state.mail && state.address.is_none() && state.pending.is_none());
    // No mailer, no notice.
    assert_eq!(asked.deliver(), None);
    assert_eq!(mailer.count(), before);
}

/// Central takes thirty requests for a reset code by mail an hour,
/// whoever asks and whatever the name; the queue is told of them and
/// worked off one piece at a time.
fn whole(fixture: &Fixture<'_>) {
    let central = fixture.central(true);
    let asked = Asked {
        service: &central,
        runtime: fixture.runtime,
    };
    let before = fixture.mailer.count();
    for turn in 0..30 {
        let name = ["mail-nobody", "mail-c", "mail-b"][turn % 3];
        let answered = if turn % 2 == 0 {
            asked.reset(name)
        } else {
            asked.reset_on_the_site(name, None)
        };
        assert!(answered.is_ok(), "request {turn}");
    }
    for name in ["mail-nobody", "mail-c", "mail-b"] {
        assert_eq!(asked.reset(name).err(), Some(Error::Busy));
        assert_eq!(asked.reset_on_the_site(name, None).err(), Some(Error::Busy));
    }
    // Told once for all thirty, and all thirty are there to be worked on.
    let told = |within: u64| {
        fixture.runtime.block_on(async {
            tokio::time::timeout(Duration::from_millis(within), central.mail_queued())
                .await
                .is_ok()
        })
    };
    assert!(told(2_000));
    assert!(!told(50));
    let mut done = 0;
    while let Some(delivery) = asked.deliver() {
        // Nobody's name, an account mailed a moment ago, an administrator.
        assert_eq!(delivery, Delivery::Nothing);
        done += 1;
    }
    assert_eq!(done, 30);
    assert_eq!(fixture.mailer.count(), before);
    assert!(!told(50));
}

/// Through the real listeners: each address has five requests for a reset
/// code by mail an hour on each of them, whatever the name, and the SDK's
/// functions reach the service.
fn listeners(fixture: &Fixture<'_>, home: &Browser, account: &Account) {
    let (http, mailer) = (fixture.http, fixture.mailer);
    let asking = |name: &str| Request::ResetMail {
        username: name.into(),
        lang: "en".into(),
    };
    let site = Browser {
        address: fixture.browser.address,
        client: "203.0.113.211",
    };
    for turn in 0..6 {
        let name = ["mail-nobody", "mail-c"][turn % 2];
        let native = http.from("203.0.113.210", &asking(name));
        let web = site.answer(&asking(name), None);
        if turn < 5 {
            assert!(matches!(native, Ok(Response::Done)), "request {turn}");
            assert!(matches!(web, Ok(Response::Done)), "request {turn}");
        } else {
            assert_eq!(native.err(), Some(Error::Busy));
            assert_eq!(web.err(), Some(Error::Busy));
        }
    }
    // Another address is served, and so is the SDK's request.
    assert!(matches!(
        http.from("203.0.113.212", &asking("mail-nobody")),
        Ok(Response::Done)
    ));
    central_client::reset_mail(http, "mail-nobody".into(), "de".into()).unwrap();
    // The SDK's session names, confirms and removes an address.
    let session = central_client::login(http, account.identity(), OLD.into()).unwrap();
    let state = session.email(http, EmailRequest::Status).unwrap();
    holds(&state, None, None);
    let address = "mail-a-sdk@example.org";
    holds(
        &session.email(http, set(address, "it")).unwrap(),
        None,
        Some(address),
    );
    assert_eq!(mailer.last().lang, "it");
    let state = session.email(http, confirm(&mailer.code())).unwrap();
    holds(&state, Some(address), None);
    // The website's listener reads the same state with the cookie.
    let Response::Email { state: read } = home
        .answer(
            &Request::Email {
                request: EmailRequest::Status,
            },
            Some(&account.cookie),
        )
        .unwrap()
    else {
        panic!("an address's state");
    };
    assert_eq!(read, state);
    assert!(matches!(
        http.exchange(
            &Request::Email {
                request: EmailRequest::Status
            },
            None
        ),
        Err(Error::Rejected)
    ));
    holds(
        &session.email(http, EmailRequest::Remove).unwrap(),
        None,
        None,
    );
    session.logout(http).unwrap();
    // What the listeners' service queued: thirteen requests, none of
    // which mails a code, and the notice to the removed address.
    let mut done = Vec::new();
    while let Some(delivery) = fixture.runtime.block_on(fixture.service.deliver_mail()) {
        done.push(delivery);
    }
    assert_eq!(done.len(), 13);
    assert_eq!(done.last(), Some(&Delivery::Sent));
    assert!(
        done[..12]
            .iter()
            .all(|delivery| *delivery == Delivery::Nothing)
    );
    let notice = mailer.last();
    assert_eq!((notice.to.as_str(), notice.what), (address, What::Removed));
}
