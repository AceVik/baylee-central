//! An account's optional e-mail address and a reset code by mail
//! (`docs/email.md`).
//!
//! The address is the session's own business: every [`EmailRequest`] acts
//! on the account of the session that asks. A code that confirms an
//! address is mailed first and kept only once the mailer took the mail, so
//! a mail that could not be handed over leaves nothing new behind.
//!
//! A reset code by mail is asked for without a session. Its request reads
//! nothing of any account: it is checked, counted, queued and answered,
//! the same for every name. The code is made and mailed afterwards, off
//! the request ([`IdentityService::deliver_mail`]), and so are the notices
//! to an address that was replaced or removed.
//!
//! A change to the login that the store committed (a reset's finish, a
//! password change, an administrator's reset code) is told to the account's
//! confirmed address, which the store read in the change's own
//! transaction ([`Mails::committed`]). The notice is queued like the others:
//! it never makes the request fail or wait, and a refusal tells nobody.
//!
//! Nothing here keeps or prints a code. A queued request holds a name as
//! typed, a queued notice an address; neither has a Debug output.
use super::{Error, IdentityService, Response, Surface, Token};
use central_api::{
    email::{EmailRequest, EmailState, MailCode},
    reset_code::ResetCode,
};
use central_mail::{Mail, Mailer, Outcome};
use central_postgres::{
    Email, ISSUED_RESET_HOURS, MAIL_CODE_MINUTES, MAILED_RESET_HOURS, Session, Told,
};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::{Notify, Semaphore};

/// Mails with a confirmation code one account may ask for in an hour: room
/// for a mistyped address and a mail that did not come, and no more of a
/// stranger's mailbox than that.
pub(super) const CODE_MAILS_PER_HOUR: usize = 5;
/// The same in a day: an address is named once and rarely changed.
pub(super) const CODE_MAILS_PER_DAY: usize = 10;
/// Accounts whose mails are counted at once, each for a day after its last
/// one; one more is refused until a place is free, so the count is never
/// forgotten for room. A few megabytes at most.
const CODE_MAIL_ACCOUNTS: usize = 16_384;
/// Reset codes Central mails in an hour, for all accounts together: far
/// more than people who forgot a password need, and a bound on what
/// Central asks of the mailer. Counted when a code is made for a mail, not
/// when it is asked for: requests for names that get no mail spend none of
/// it, so a few addresses asking about made-up names keep nobody's mail
/// back. To spend it, someone needs accounts with confirmed addresses, and
/// each of those has three mails a day.
pub(super) const RESET_MAILS_PER_HOUR: usize = 30;
/// Requests for a reset code that wait for the worker. Every request
/// without a session shares 40 a second ([`crate::limits`]), and the
/// worker takes one in a few milliseconds, so the queue fills only while
/// the store does not answer.
const RESET_QUEUE: usize = 256;
/// Notices that wait for their turn; one more is not sent. A notice
/// follows a confirmed or removed address, a reset, a password change or
/// an administrator's reset code, which are rare; one that already waits
/// is not queued twice ([`Mails::notice`]).
const NOTICE_QUEUE: usize = 64;
/// Mails with a confirmation code handed to the mailer at once: each holds
/// a thread for as long as the mailer takes, three seconds at most.
const SENDING: usize = 4;
/// The longest text that can be a sign-in name; a longer one is no
/// account's and is not kept.
const NAME_BYTES: usize = 24;
/// Minutes a mailed confirmation code lasts, as the mail says them.
const CODE_MINUTES: u32 = MAIL_CODE_MINUTES.unsigned_abs();
/// Minutes a mailed reset code lasts, as the mail says them.
const RESET_MINUTES: u32 = MAILED_RESET_HOURS.unsigned_abs() * 60;
/// Hours an administrator's reset code lasts, as its notice says them.
pub(super) const ISSUED_HOURS: u32 = ISSUED_RESET_HOURS.unsigned_abs();

const HOUR: Duration = Duration::from_secs(3_600);
const DAY: Duration = Duration::from_hours(24);

/// A request for a reset code by mail that waits for its turn: the name as
/// typed and the language asked in. No Debug.
struct Asked {
    name: String,
    lang: String,
}

/// What became of one piece of queued mail work
/// ([`IdentityService::deliver_mail`]). Nothing of a mail, a name or an
/// account is in it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Delivery {
    /// The mailer took a mail: a reset code or a notice.
    Sent,
    /// Nothing was to be mailed: the name asked about is no account a
    /// reset code is mailed to now.
    Nothing,
    /// The mailer did not take the mail, or the store could not be asked.
    /// A reset code made for the mail has ended.
    Failed,
    /// The hour's reset mails ([`RESET_MAILS_PER_HOUR`]) were spent; the
    /// request was dropped without asking the store.
    Held,
}

/// The mails that were asked for and the work that waits, in memory only:
/// a restart forgets both, and the mailer's own limit for an account
/// stays.
#[derive(Default)]
struct Counted {
    /// When each account asked for a mail with a confirmation code, within
    /// the last day, oldest first.
    codes: BTreeMap<[u8; 16], VecDeque<Instant>>,
    /// When reset codes were made for a mail, within the last hour,
    /// oldest first.
    reset_mails: VecDeque<Instant>,
    asked: VecDeque<Asked>,
    notices: VecDeque<Mail>,
}

impl Counted {
    /// Count one mail with a confirmation code for `account` at `now`.
    /// Nothing is counted when the account's hour or day is used up, or
    /// when a new account finds no place.
    fn code_mail(&mut self, account: [u8; 16], now: Instant) -> Result<(), Error> {
        self.codes.retain(|_, times| {
            while times
                .front()
                .is_some_and(|at| now.saturating_duration_since(*at) >= DAY)
            {
                times.pop_front();
            }
            !times.is_empty()
        });
        let fits = match self.codes.get(&account) {
            Some(times) => {
                let hour = times
                    .iter()
                    .filter(|at| now.saturating_duration_since(**at) < HOUR)
                    .count();
                times.len() < CODE_MAILS_PER_DAY && hour < CODE_MAILS_PER_HOUR
            }
            None => self.codes.len() < CODE_MAIL_ACCOUNTS,
        };
        if !fits {
            return Err(Error::Busy);
        }
        self.codes.entry(account).or_default().push_back(now);
        Ok(())
    }

    /// Queue one request for a reset code by mail. A request without a
    /// name that could be an account's is taken and not queued. `Busy` when
    /// the queue is full.
    fn reset_request(&mut self, asked: Option<Asked>) -> Result<(), Error> {
        if self.asked.len() >= RESET_QUEUE {
            return Err(Error::Busy);
        }
        self.asked.extend(asked);
        Ok(())
    }

    /// Take a place for one reset mail at `now`, if the hour has one left.
    fn reset_mail(&mut self, now: Instant) -> bool {
        while self
            .reset_mails
            .front()
            .is_some_and(|at| now.saturating_duration_since(*at) >= HOUR)
        {
            self.reset_mails.pop_front();
        }
        if self.reset_mails.len() >= RESET_MAILS_PER_HOUR {
            return false;
        }
        self.reset_mails.push_back(now);
        true
    }
}

/// The service's mail: the mailer, if this Central has one, what was asked
/// of it and what waits for it.
pub(super) struct Mails {
    /// Whatever sends the mail; without it no code is mailed.
    pub(super) mailer: Option<Arc<dyn Mailer>>,
    counted: Mutex<Counted>,
    /// Told whenever work is queued.
    queued: Notify,
    /// The places of the mails requests hand over themselves.
    sending: Arc<Semaphore>,
}

impl Default for Mails {
    fn default() -> Self {
        Self {
            mailer: None,
            counted: Mutex::default(),
            queued: Notify::new(),
            sending: Arc::new(Semaphore::new(SENDING)),
        }
    }
}

/// One piece of mail work that waits. No Debug.
enum Work {
    /// A reset code to make and mail, if the name is an account's.
    Reset(Asked),
    /// A notice to send.
    Notice(Mail),
}

impl Mails {
    /// The mailer, for an operation that needs one.
    fn needed(&self) -> Result<Arc<dyn Mailer>, Error> {
        self.mailer.clone().ok_or(Error::Rejected)
    }

    /// What a session is told of its account's address.
    fn state(&self, held: Email) -> EmailState {
        EmailState {
            mail: self.mailer.is_some(),
            address: held.address,
            confirmed_at: held.confirmed_at,
            pending: held.pending,
            pending_until: held.pending_until,
        }
    }

    /// Make a confirmation code for `account` and mail it to `address`.
    /// The mail is counted before it is handed over and stays counted
    /// whatever the mailer says: the count bounds what Central asks of the
    /// mailer.
    ///
    /// `Busy` when the account asked too often, when no place to send is
    /// free and when the mailer did not take the mail; `Rejected` when the
    /// mailer refuses the address itself.
    async fn code(
        &self,
        mailer: Arc<dyn Mailer>,
        account: [u8; 16],
        address: &str,
        lang: &str,
    ) -> Result<MailCode, Error> {
        // The place first: a request that finds none spent no mail of the
        // account's.
        let permit = self
            .sending
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::Busy)?;
        self.counted
            .lock()
            .map_err(|_| Error::Unavailable)?
            .code_mail(account, Instant::now())?;
        let code = MailCode::generate()?;
        let mail = Mail::confirm(address, account, lang, &code, CODE_MINUTES);
        let outcome = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            mailer.send(&mail)
        })
        .await
        .map_err(|_| Error::Unavailable)?;
        match outcome {
            Outcome::Sent => Ok(code),
            Outcome::Rejected => Err(Error::Rejected),
            Outcome::Busy | Outcome::Unavailable => Err(Error::Busy),
        }
    }

    /// Queue the notice `mail` writes to the address `told` of `account`:
    /// one that lost its place, or the account's own when its login
    /// changed. It is tried once, later, and is never a reason to refuse:
    /// the change it tells of is committed. Without a mailer, or with the
    /// queue full, it is not sent. The same notice to the same address of
    /// the same account is queued once while it waits, so that an account
    /// that changes its password again and again holds one place, not all.
    fn notice(
        &self,
        told: Option<Told>,
        account: [u8; 16],
        mail: impl FnOnce(&str, [u8; 16], &str) -> Mail,
    ) {
        let Some(told) = told else {
            return;
        };
        if self.mailer.is_none() {
            return;
        }
        let mail = mail(&told.address, account, &told.lang);
        let Ok(mut counted) = self.counted.lock() else {
            return;
        };
        let waits = counted.notices.iter().any(|queued| {
            queued.account() == account && queued.what() == mail.what() && queued.to() == mail.to()
        });
        if waits || counted.notices.len() >= NOTICE_QUEUE {
            return;
        }
        counted.notices.push_back(mail);
        drop(counted);
        self.queued.notify_one();
    }

    /// The store's answer `done` to a change of the login of `account`,
    /// with the account's confirmed address as the change's transaction
    /// read it. Once the store committed the change, `mail` writes the
    /// notice to that address and it is queued ([`Self::notice`]): no
    /// address, no mailer or a full queue tells nobody. A refusal tells
    /// nobody either and is answered as it is. Nothing here waits: the
    /// mailer is asked later, by the worker, so the answer is the store's
    /// whatever becomes of the notice.
    pub(super) fn committed<T>(
        &self,
        done: Result<(T, Option<Told>), Error>,
        account: [u8; 16],
        mail: impl FnOnce(&str, [u8; 16], &str) -> Mail,
    ) -> Result<T, Error> {
        let (answer, told) = done?;
        self.notice(told, account, mail);
        Ok(answer)
    }

    /// Take a request for a reset code by mail: check and queue it.
    /// Nothing of any account is read. `Rejected` for every name without a
    /// mailer and for a language tag that is none; `Busy` while the queue
    /// is full.
    fn reset_request(&self, username: String, lang: String) -> Result<(), Error> {
        if self.mailer.is_none() {
            return Err(Error::Rejected);
        }
        central_api::email::language(&lang)?;
        let asked = (username.len() <= NAME_BYTES).then_some(Asked {
            name: username,
            lang,
        });
        self.counted
            .lock()
            .map_err(|_| Error::Unavailable)?
            .reset_request(asked)?;
        self.queued.notify_one();
        Ok(())
    }

    /// Take a place for one reset mail in the hour's, or say that none is
    /// left.
    fn reset_mail_place(&self) -> bool {
        self.counted
            .lock()
            .is_ok_and(|mut counted| counted.reset_mail(Instant::now()))
    }

    /// Give back the place a reset mail took when no code was made for it.
    fn reset_mail_unused(&self) {
        if let Ok(mut counted) = self.counted.lock() {
            counted.reset_mails.pop_back();
        }
    }

    /// The next piece of work that waits and the mailer for it: requests
    /// for a reset code first, whose codes last an hour, then notices.
    /// `None` when nothing waits, and always without a mailer.
    fn next(&self) -> Option<(Arc<dyn Mailer>, Work)> {
        let mailer = self.mailer.clone()?;
        let mut counted = self.counted.lock().ok()?;
        let work = match counted.asked.pop_front() {
            Some(asked) => Work::Reset(asked),
            None => Work::Notice(counted.notices.pop_front()?),
        };
        Some((mailer, work))
    }
}

/// Hand `mail` to `mailer` off the asynchronous threads. A send that could
/// not run is a mail that was not sent.
async fn send(mailer: Arc<dyn Mailer>, mail: Mail) -> Outcome {
    tokio::task::spawn_blocking(move || mailer.send(&mail))
        .await
        .unwrap_or(Outcome::Unavailable)
}

impl IdentityService {
    /// What a session asks about its account's address. The terms were
    /// looked at before (`terms.rs`).
    pub(super) async fn email_request(
        &self,
        request: EmailRequest,
        bearer: Option<&Token>,
        surface: Surface,
    ) -> Result<Response, Error> {
        let session = surface.session(bearer)?;
        let held = match request {
            EmailRequest::Status => self.store.email(session).await?,
            EmailRequest::Set { address, lang } => self.email_set(session, &address, &lang).await?,
            EmailRequest::Resend { lang } => self.email_resend(session, &lang).await?,
            EmailRequest::Confirm { code } => self.email_confirm(session, &code).await?,
            EmailRequest::Withdraw => self.store.email_withdraw(session).await?,
            EmailRequest::Remove => {
                let (held, told) = self.store.email_remove(session).await?;
                self.mails.notice(told, held.account, Mail::removed);
                held
            }
        };
        Ok(Response::Email {
            state: self.mails.state(held),
        })
    }

    /// Name an address: everything the store would refuse is refused
    /// before a mail goes out, and the store, which decides, is asked
    /// again once the mailer took the mail.
    async fn email_set(
        &self,
        session: Session<'_>,
        address: &str,
        lang: &str,
    ) -> Result<Email, Error> {
        let mailer = self.mails.needed()?;
        central_api::email::address(address)?;
        central_api::email::language(lang)?;
        let held = self.store.email(session).await?;
        if !held.fresh {
            return Err(Error::Rejected);
        }
        if held.address.as_deref() == Some(address) {
            return Err(Error::Conflict);
        }
        let code = self.mails.code(mailer, held.account, address, lang).await?;
        self.store
            .email_pending(session, address, lang, &code.digest(&held.account))
            .await
    }

    /// Mail a new code to the address that waits. The store renews that
    /// address and no other: one named meanwhile keeps its own code.
    async fn email_resend(&self, session: Session<'_>, lang: &str) -> Result<Email, Error> {
        let mailer = self.mails.needed()?;
        central_api::email::language(lang)?;
        let held = self.store.email(session).await?;
        let Some(address) = held.pending.as_deref() else {
            return Err(Error::Rejected);
        };
        let code = self.mails.code(mailer, held.account, address, lang).await?;
        self.store
            .email_renew(session, address, lang, &code.digest(&held.account))
            .await
    }

    /// Confirm the address that waits. The store compares, counts a wrong
    /// code and gives the address up after the last one.
    async fn email_confirm(&self, session: Session<'_>, code: &MailCode) -> Result<Email, Error> {
        self.mails.needed()?;
        let account = self.store.email(session).await?.account;
        let (held, told) = self
            .store
            .email_confirm(session, account, &code.digest(&account))
            .await?;
        self.mails.notice(told, account, Mail::replaced);
        Ok(held)
    }

    /// A request for a reset code by mail, on either listener without a
    /// session. Nothing of any account is read here: the answer and the
    /// time it takes are the same for a name that is nobody's, one without
    /// an address, an administrator's and one a code is mailed for.
    /// `Rejected` for every name where Central sends no mail and for a
    /// language tag that is none; `Busy` while the worker's queue is full.
    pub(super) fn reset_mail(&self, username: String, lang: String) -> Result<Response, Error> {
        self.mails.reset_request(username, lang)?;
        Ok(Response::Done)
    }

    /// Resolves once mail work was queued since this was last awaited. A
    /// launcher awaits it between two rounds of [`Self::deliver_mail`].
    pub async fn mail_queued(&self) {
        self.mails.queued.notified().await;
    }

    /// Do one piece of the mail work that waits, to its end: make and
    /// mail the reset code a request asked for, or send a notice. `None`
    /// when nothing waits, and always without a mailer. Called off the
    /// requests, one piece at a time, until it returns `None`.
    ///
    /// A reset code is issued by the statement an administrator's is
    /// issued by, with the same refusals, and is taken back when the
    /// mailer does not take its mail.
    pub async fn deliver_mail(&self) -> Option<Delivery> {
        let (mailer, work) = self.mails.next()?;
        Some(match work {
            Work::Reset(asked) => self.mail_reset(mailer, &asked).await,
            Work::Notice(mail) => match send(mailer, mail).await {
                Outcome::Sent => Delivery::Sent,
                Outcome::Busy | Outcome::Rejected | Outcome::Unavailable => Delivery::Failed,
            },
        })
    }

    async fn mail_reset(&self, mailer: Arc<dyn Mailer>, asked: &Asked) -> Delivery {
        // The hour's mails are spent: the request is dropped, as one for a
        // name that gets no mail is. Its requester was answered long ago.
        if !self.mails.reset_mail_place() {
            return Delivery::Held;
        }
        let Ok(code) = ResetCode::generate() else {
            self.mails.reset_mail_unused();
            return Delivery::Failed;
        };
        let digest = code.digest();
        let reset = match self.store.issue_mailed_reset(&asked.name, &digest).await {
            Ok(Some(reset)) => reset,
            Ok(None) => {
                self.mails.reset_mail_unused();
                return Delivery::Nothing;
            }
            Err(_) => {
                self.mails.reset_mail_unused();
                return Delivery::Failed;
            }
        };
        let mail = Mail::reset(
            &reset.address,
            reset.account,
            &asked.lang,
            &reset.username,
            &code,
            RESET_MINUTES,
        );
        if send(mailer, mail).await == Outcome::Sent {
            return Delivery::Sent;
        }
        // No mail carries the code, so it ends. An administrator's code
        // made since stays, and the try stays counted.
        let _ = self
            .store
            .withdraw_mailed_reset(reset.account, &digest)
            .await;
        Delivery::Failed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use central_mail::{Lang, What};
    use std::task::{Context, Waker};

    const SECOND: Duration = Duration::from_secs(1);

    fn asked(name: &str) -> Option<Asked> {
        let lang = String::from("en");
        (name.len() <= NAME_BYTES).then_some(Asked {
            name: name.into(),
            lang,
        })
    }

    #[test]
    fn an_account_has_five_mails_an_hour_and_ten_a_day() {
        let start = Instant::now();
        let mut counted = Counted::default();
        let (one, other) = ([1; 16], [2; 16]);
        for _ in 0..CODE_MAILS_PER_HOUR {
            assert_eq!(counted.code_mail(one, start), Ok(()));
        }
        // The sixth in the hour is refused and not counted.
        assert_eq!(counted.code_mail(one, start), Err(Error::Busy));
        assert_eq!(
            counted.code_mail(one, start + HOUR.saturating_sub(SECOND)),
            Err(Error::Busy)
        );
        // Another account has its own.
        assert_eq!(counted.code_mail(other, start), Ok(()));
        // The hour passed: five more, and then the day is used up.
        for _ in 0..CODE_MAILS_PER_HOUR {
            assert_eq!(counted.code_mail(one, start + HOUR), Ok(()));
        }
        assert_eq!(counted.codes[&one].len(), CODE_MAILS_PER_DAY);
        for later in [HOUR * 2, HOUR * 12, DAY.saturating_sub(SECOND)] {
            assert_eq!(counted.code_mail(one, start + later), Err(Error::Busy));
        }
        // A day after the first five, those are forgotten and no others.
        assert_eq!(counted.code_mail(one, start + DAY), Ok(()));
        assert_eq!(counted.codes[&one].len(), CODE_MAILS_PER_HOUR + 1);
        // The other account's one mail is forgotten with its day.
        assert!(!counted.codes.contains_key(&other));
    }

    #[test]
    fn the_accounts_counted_are_bounded_and_none_is_forgotten_for_room() {
        let start = Instant::now();
        let mut counted = Counted::default();
        let account = |n: usize| {
            let mut id = [0; 16];
            id[..8].copy_from_slice(&u64::try_from(n).unwrap().to_be_bytes());
            id
        };
        for n in 0..CODE_MAIL_ACCOUNTS {
            assert_eq!(counted.code_mail(account(n), start), Ok(()));
        }
        // No place for one more, while every counted account keeps its own.
        assert_eq!(
            counted.code_mail(account(CODE_MAIL_ACCOUNTS), start),
            Err(Error::Busy)
        );
        assert_eq!(counted.code_mail(account(0), start), Ok(()));
        assert_eq!(counted.codes.len(), CODE_MAIL_ACCOUNTS);
        // A day later the places are free again.
        assert_eq!(
            counted.code_mail(account(CODE_MAIL_ACCOUNTS), start + DAY),
            Ok(())
        );
        assert_eq!(counted.codes.len(), 1);
    }

    #[test]
    fn central_mails_thirty_reset_codes_an_hour_and_requests_spend_none() {
        let start = Instant::now();
        let mut counted = Counted::default();
        // Requests are taken whatever the name and spend nothing of the
        // hour's mails.
        for n in 0..RESET_MAILS_PER_HOUR * 4 {
            let request = if n % 2 == 0 { asked("synthetic") } else { None };
            assert!(counted.reset_request(request).is_ok());
        }
        assert_eq!(counted.asked.len(), RESET_MAILS_PER_HOUR * 2);
        assert!(counted.reset_mails.is_empty());
        for _ in 0..RESET_MAILS_PER_HOUR {
            assert!(counted.reset_mail(start));
        }
        assert!(!counted.reset_mail(start));
        assert!(!counted.reset_mail(start + HOUR.saturating_sub(SECOND)));
        assert_eq!(counted.reset_mails.len(), RESET_MAILS_PER_HOUR);
        // The window rolls: an hour after the first, one more fits.
        assert!(counted.reset_mail(start + HOUR));
        assert_eq!(counted.reset_mails.len(), 1);
    }

    #[test]
    fn a_full_queue_refuses_and_queues_nothing() {
        let mut counted = Counted::default();
        for _ in 0..RESET_QUEUE {
            assert!(counted.reset_request(asked("synthetic")).is_ok());
        }
        assert_eq!(counted.reset_request(asked("synthetic")), Err(Error::Busy));
        assert_eq!(counted.reset_request(None), Err(Error::Busy));
        assert_eq!(counted.asked.len(), RESET_QUEUE);
        counted.asked.pop_front();
        assert!(counted.reset_request(asked("synthetic")).is_ok());
    }

    /// One mail as it was handed to the test's mailer.
    struct Handed {
        to: String,
        account: [u8; 16],
        what: What,
        code: Option<String>,
        ttl: Option<u32>,
    }

    /// A mailer of the test's own behind the trait: it keeps every mail
    /// handed to it and says what the test scripted, `Sent` when nothing
    /// is scripted.
    #[derive(Default)]
    struct Fake {
        handed: Mutex<Vec<Handed>>,
        answers: Mutex<VecDeque<Outcome>>,
    }

    impl Mailer for Fake {
        fn send(&self, mail: &Mail) -> Outcome {
            self.handed.lock().unwrap().push(Handed {
                to: mail.to().to_owned(),
                account: mail.account(),
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

    impl Fake {
        fn count(&self) -> usize {
            self.handed.lock().unwrap().len()
        }

        fn next(&self, outcome: Outcome) {
            self.answers.lock().unwrap().push_back(outcome);
        }
    }

    /// A mailer that cannot send at all.
    struct Broken;

    impl Mailer for Broken {
        fn send(&self, _: &Mail) -> Outcome {
            panic!("a mailer that cannot send");
        }
    }

    fn mailing(fake: &Arc<Fake>) -> Mails {
        Mails {
            mailer: Some(fake.clone()),
            ..Mails::default()
        }
    }

    fn wait<T>(work: impl Future<Output = T>) -> T {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(work)
    }

    /// Whether the worker was told of queued work since it was last told.
    fn told(mails: &Mails) -> bool {
        let waiting = std::pin::pin!(mails.queued.notified());
        waiting
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_ready()
    }

    const TO: &str = "player@example.org";

    #[test]
    fn a_code_goes_through_the_mailer_and_is_counted_whatever_the_mailer_says() {
        let fake = Arc::new(Fake::default());
        let mails = mailing(&fake);
        let (one, other) = ([1; 16], [2; 16]);
        let code = wait(mails.code(fake.clone(), one, TO, "de")).unwrap();
        {
            let handed = fake.handed.lock().unwrap();
            let mail = &handed[0];
            assert_eq!((mail.to.as_str(), mail.account), (TO, one));
            assert_eq!((mail.what, mail.ttl), (What::Confirm, Some(900)));
            assert_eq!(mail.code.as_deref(), Some(code.display().as_str()));
        }
        // A mail the mailer does not take is no code, and is counted.
        for (said, answered) in [
            (Outcome::Busy, Error::Busy),
            (Outcome::Unavailable, Error::Busy),
            (Outcome::Rejected, Error::Rejected),
        ] {
            let before = fake.count();
            fake.next(said);
            assert_eq!(
                wait(mails.code(fake.clone(), one, TO, "en")).err(),
                Some(answered)
            );
            assert_eq!(fake.count(), before + 1);
        }
        // The fifth of the hour is handed over, the sixth is not.
        assert!(wait(mails.code(fake.clone(), one, TO, "en")).is_ok());
        assert_eq!(fake.count(), CODE_MAILS_PER_HOUR);
        assert_eq!(
            wait(mails.code(fake.clone(), one, TO, "en")).err(),
            Some(Error::Busy)
        );
        assert_eq!(fake.count(), CODE_MAILS_PER_HOUR);
        // Another account's mails are its own, to the same address too.
        assert!(wait(mails.code(fake.clone(), other, TO, "en")).is_ok());
        assert_eq!(fake.handed.lock().unwrap().last().unwrap().account, other);
        // Two codes are never the same one.
        let codes: Vec<_> = fake
            .handed
            .lock()
            .unwrap()
            .iter()
            .map(|mail| mail.code.clone().unwrap())
            .collect();
        for (at, code) in codes.iter().enumerate() {
            assert!(!codes[..at].contains(code));
        }
    }

    #[test]
    fn without_a_free_place_no_mail_is_handed_over_and_none_is_counted() {
        let fake = Arc::new(Fake::default());
        let mails = mailing(&fake);
        let places: Vec<_> = (0..SENDING)
            .map(|_| mails.sending.clone().try_acquire_owned().unwrap())
            .collect();
        assert_eq!(
            wait(mails.code(fake.clone(), [1; 16], TO, "en")).err(),
            Some(Error::Busy)
        );
        assert_eq!(fake.count(), 0);
        assert!(mails.counted.lock().unwrap().codes.is_empty());
        drop(places);
        assert!(wait(mails.code(fake.clone(), [1; 16], TO, "en")).is_ok());
        // The place is free again once the mail was handed over.
        assert_eq!(mails.sending.available_permits(), SENDING);
    }

    #[test]
    fn a_mailer_that_cannot_send_is_a_mail_not_sent() {
        let mails = Mails::default();
        let code = MailCode::generate().unwrap();
        let mail = Mail::confirm(TO, [1; 16], "en", &code, CODE_MINUTES);
        assert_eq!(wait(send(Arc::new(Broken), mail)), Outcome::Unavailable);
        assert_eq!(
            wait(mails.code(Arc::new(Broken), [1; 16], TO, "en")).err(),
            Some(Error::Unavailable)
        );
        // The try is counted and its place is free again.
        assert_eq!(mails.counted.lock().unwrap().codes[&[1; 16]].len(), 1);
        assert_eq!(mails.sending.available_permits(), SENDING);
    }

    #[test]
    fn a_reset_request_is_answered_alike_for_every_name_and_queued_for_later() {
        // Without a mailer every name is refused and nothing is kept.
        let none = Mails::default();
        for name in ["synthetic", "", "nobody"] {
            assert_eq!(
                none.reset_request(name.into(), "en".into()),
                Err(Error::Rejected)
            );
        }
        assert!(none.counted.lock().unwrap().asked.is_empty());
        assert!(none.next().is_none() && !told(&none));
        // With one, a tag that is no language is refused and not counted.
        let fake = Arc::new(Fake::default());
        let mails = mailing(&fake);
        for lang in ["", "EN", "en_GB", "x"] {
            assert_eq!(
                mails.reset_request("synthetic".into(), lang.into()),
                Err(Error::Rejected)
            );
        }
        assert!(mails.counted.lock().unwrap().asked.is_empty());
        assert!(!told(&mails));
        // Every name is taken alike; one too long to be a name is taken
        // and not kept. No mail goes out on the request, and none of the
        // hour's mails is spent.
        let long = "a".repeat(NAME_BYTES + 1);
        for name in ["synthetic", "", "name\r\nwith: more", long.as_str()] {
            assert_eq!(mails.reset_request(name.into(), "pt-br".into()), Ok(()));
        }
        assert_eq!(fake.count(), 0);
        assert_eq!(mails.counted.lock().unwrap().asked.len(), 3);
        assert!(mails.counted.lock().unwrap().reset_mails.is_empty());
        // The worker is told once, and finds the three in their order.
        assert!(told(&mails) && !told(&mails));
        for name in ["synthetic", "", "name\r\nwith: more"] {
            let Some((_, Work::Reset(asked))) = mails.next() else {
                panic!("a request that waits");
            };
            assert_eq!((asked.name.as_str(), asked.lang.as_str()), (name, "pt-br"));
        }
        assert!(mails.next().is_none());
        // A full queue, whatever the name; then every name is busy.
        for _ in 0..RESET_QUEUE {
            assert_eq!(mails.reset_request("synthetic".into(), "en".into()), Ok(()));
        }
        for name in ["synthetic", "nobody", long.as_str()] {
            assert_eq!(
                mails.reset_request(name.into(), "en".into()),
                Err(Error::Busy)
            );
        }
    }

    #[test]
    fn a_reset_mail_takes_a_place_of_the_hours_and_one_unused_is_given_back() {
        let fake = Arc::new(Fake::default());
        let mails = mailing(&fake);
        for _ in 0..RESET_MAILS_PER_HOUR {
            assert!(mails.reset_mail_place());
        }
        assert!(!mails.reset_mail_place());
        mails.reset_mail_unused();
        assert!(mails.reset_mail_place());
        assert!(!mails.reset_mail_place());
    }

    #[test]
    fn a_notice_waits_only_where_mail_is_sent_and_behind_the_reset_codes() {
        let told_of = |address: &str| {
            Some(Told {
                address: address.into(),
                lang: "de".into(),
            })
        };
        // Without a mailer nothing is queued.
        let none = Mails::default();
        none.notice(told_of("old@example.org"), [1; 16], Mail::replaced);
        assert!(none.counted.lock().unwrap().notices.is_empty() && !told(&none));
        let fake = Arc::new(Fake::default());
        let mails = mailing(&fake);
        // No address lost its place: nothing to tell.
        mails.notice(None, [1; 16], Mail::removed);
        assert!(mails.next().is_none() && !told(&mails));
        mails.notice(told_of("old@example.org"), [1; 16], Mail::replaced);
        mails.notice(told_of("other@example.org"), [2; 16], Mail::removed);
        assert!(told(&mails));
        // A reset code asked for later is worked on first.
        mails
            .reset_request("synthetic".into(), "en".into())
            .unwrap();
        assert!(matches!(mails.next(), Some((_, Work::Reset(_)))));
        for (to, account, what) in [
            ("old@example.org", [1; 16], What::Replaced),
            ("other@example.org", [2; 16], What::Removed),
        ] {
            let Some((_, Work::Notice(mail))) = mails.next() else {
                panic!("a notice that waits");
            };
            assert_eq!(
                (mail.to(), mail.account(), mail.what()),
                (to, account, what)
            );
            assert!(mail.code().is_none() && mail.ttl_secs().is_none());
        }
        assert!(mails.next().is_none());
        // Queuing sends nothing, and the queue is bounded: one more is
        // dropped.
        for n in 0..=NOTICE_QUEUE {
            let account = [u8::try_from(n).unwrap(); 16];
            mails.notice(told_of("old@example.org"), account, Mail::replaced);
        }
        assert_eq!(mails.counted.lock().unwrap().notices.len(), NOTICE_QUEUE);
        assert_eq!(fake.count(), 0);
    }

    /// What writes a notice to an address of an account.
    type Writes = fn(&str, [u8; 16], &str) -> Mail;

    fn told_to(address: &str, lang: &str) -> Told {
        Told {
            address: address.into(),
            lang: lang.into(),
        }
    }

    /// The notice that waits next, as the worker finds it.
    fn waiting(mails: &Mails) -> Option<(String, [u8; 16], What, Lang)> {
        let Some((_, Work::Notice(mail))) = mails.next() else {
            return None;
        };
        assert!(mail.code().is_none() && mail.ttl_secs().is_none());
        Some((
            mail.to().to_owned(),
            mail.account(),
            mail.what(),
            mail.lang(),
        ))
    }

    #[test]
    fn a_committed_change_is_told_later_and_a_refusal_never() {
        let fake = Arc::new(Fake::default());
        let mails = mailing(&fake);
        let account = [3; 16];
        // A refusal is answered as it is: no notice is written, none waits
        // and the worker is not told.
        for refused in [
            Error::Rejected,
            Error::Conflict,
            Error::Busy,
            Error::Unavailable,
        ] {
            let done: Result<((), Option<Told>), Error> = Err(refused);
            assert_eq!(
                mails.committed(done, account, |_, _, _| panic!("no notice for a refusal")),
                Err(refused)
            );
        }
        assert!(mails.next().is_none() && !told(&mails));
        // A committed change is answered with the store's answer, and its
        // notice waits for the worker: nothing went to the mailer yet.
        let changes: [(Writes, What); 4] = [
            (
                |to, account, lang| Mail::reset_done(to, account, lang, false),
                What::ResetDone { new_vault: false },
            ),
            (
                |to, account, lang| Mail::reset_done(to, account, lang, true),
                What::ResetDone { new_vault: true },
            ),
            (Mail::password_changed, What::PasswordChanged),
            (
                |to, account, lang| Mail::reset_issued(to, account, lang, ISSUED_HOURS),
                What::ResetIssued,
            ),
        ];
        for (at, (mail, what)) in changes.into_iter().enumerate() {
            let done = Ok((at, Some(told_to("own@example.org", "de"))));
            assert_eq!(mails.committed(done, account, mail), Ok(at));
            assert_eq!(fake.count(), 0, "nothing is handed over on the request");
            assert!(told(&mails));
            assert_eq!(
                waiting(&mails),
                Some(("own@example.org".into(), account, what, Lang::De))
            );
        }
        // An account without a confirmed address: the answer, and nobody
        // to tell.
        assert_eq!(
            mails.committed(Ok(("issued", None)), account, |_, _, _| panic!(
                "no address, no notice"
            )),
            Ok("issued")
        );
        assert!(mails.next().is_none() && !told(&mails));
        assert_eq!(fake.count(), 0);
    }

    #[test]
    fn without_a_mailer_a_change_is_answered_and_tells_nobody() {
        let none = Mails::default();
        for mail in [Mail::password_changed as Writes, Mail::replaced] {
            assert_eq!(
                none.committed(
                    Ok((5, Some(told_to("own@example.org", "en")))),
                    [4; 16],
                    mail
                ),
                Ok(5)
            );
        }
        assert!(none.counted.lock().unwrap().notices.is_empty());
        assert!(none.next().is_none() && !told(&none));
    }

    #[test]
    fn a_notice_never_waits_for_the_mailer_nor_fails_the_answer() {
        // A mailer that panics when asked: the request never asks it.
        let mails = Mails {
            mailer: Some(Arc::new(Broken)),
            ..Mails::default()
        };
        assert_eq!(
            mails.committed(
                Ok(((), Some(told_to("own@example.org", "en")))),
                [5; 16],
                Mail::password_changed
            ),
            Ok(())
        );
        assert_eq!(mails.counted.lock().unwrap().notices.len(), 1);
        // The worker asks it later, and a mailer that cannot send is a
        // notice not sent; the change stays done.
        let Some((mailer, Work::Notice(mail))) = mails.next() else {
            panic!("a notice that waits");
        };
        assert_eq!(wait(send(mailer, mail)), Outcome::Unavailable);
        // A full queue drops the notice and still answers.
        let fake = Arc::new(Fake::default());
        let full = mailing(&fake);
        for n in 0..NOTICE_QUEUE {
            let account = [u8::try_from(n).unwrap(); 16];
            full.notice(
                Some(told_to("other@example.org", "en")),
                account,
                Mail::removed,
            );
        }
        assert_eq!(
            full.committed(
                Ok((9, Some(told_to("own@example.org", "en")))),
                [200; 16],
                Mail::password_changed
            ),
            Ok(9)
        );
        let queued = &full.counted.lock().unwrap().notices;
        assert_eq!(queued.len(), NOTICE_QUEUE);
        assert!(queued.iter().all(|mail| mail.what() == What::Removed));
        assert_eq!(fake.count(), 0);
    }

    #[test]
    fn a_notice_that_waits_is_not_queued_twice() {
        let fake = Arc::new(Fake::default());
        let mails = mailing(&fake);
        let (one, other) = ([1; 16], [2; 16]);
        // An account that changes its password again and again holds one
        // place while its notice waits.
        for _ in 0..NOTICE_QUEUE * 2 {
            mails.notice(
                Some(told_to("own@example.org", "en")),
                one,
                Mail::password_changed,
            );
        }
        assert_eq!(mails.counted.lock().unwrap().notices.len(), 1);
        // Another notice, another address or another account each have
        // their own.
        mails.notice(
            Some(told_to("own@example.org", "en")),
            one,
            |to, account, lang| Mail::reset_done(to, account, lang, false),
        );
        mails.notice(
            Some(told_to("own@example.org", "en")),
            one,
            |to, account, lang| Mail::reset_done(to, account, lang, true),
        );
        mails.notice(
            Some(told_to("new@example.org", "en")),
            one,
            Mail::password_changed,
        );
        mails.notice(
            Some(told_to("own@example.org", "en")),
            other,
            Mail::password_changed,
        );
        let mut found = Vec::new();
        while let Some((to, account, what, _)) = waiting(&mails) {
            found.push((to, account, what));
        }
        assert_eq!(
            found,
            [
                ("own@example.org".into(), one, What::PasswordChanged),
                (
                    "own@example.org".into(),
                    one,
                    What::ResetDone { new_vault: false }
                ),
                (
                    "own@example.org".into(),
                    one,
                    What::ResetDone { new_vault: true }
                ),
                ("new@example.org".into(), one, What::PasswordChanged),
                ("own@example.org".into(), other, What::PasswordChanged),
            ]
        );
        // Once it was taken, the same notice waits again.
        mails.notice(
            Some(told_to("own@example.org", "en")),
            one,
            Mail::password_changed,
        );
        assert_eq!(mails.counted.lock().unwrap().notices.len(), 1);
    }

    #[test]
    fn the_state_says_whether_this_central_sends_mail() {
        let held = || Email {
            account: [1; 16],
            fresh: true,
            address: Some(TO.into()),
            confirmed_at: Some(1_794_000_000),
            pending: Some("other@example.org".into()),
            pending_until: Some(1_794_000_900),
        };
        let none = Mails::default();
        assert_eq!(none.needed().err(), Some(Error::Rejected));
        let state = none.state(held());
        assert!(!state.mail);
        let fake = Arc::new(Fake::default());
        let mails = mailing(&fake);
        assert!(mails.needed().is_ok());
        let with = mails.state(held());
        assert!(with.mail);
        assert_eq!(with.address.as_deref(), Some(TO));
        assert_eq!(with.confirmed_at, Some(1_794_000_000));
        assert_eq!(with.pending.as_deref(), Some("other@example.org"));
        assert_eq!(with.pending_until, Some(1_794_000_900));
        assert_eq!((state.address, state.pending), (with.address, with.pending));
    }

    #[test]
    fn the_numbers_are_the_documented_ones() {
        assert_eq!((CODE_MAILS_PER_HOUR, CODE_MAILS_PER_DAY), (5, 10));
        assert_eq!(RESET_MAILS_PER_HOUR, 30);
        assert_eq!((CODE_MINUTES, RESET_MINUTES, ISSUED_HOURS), (15, 60, 24));
        assert_eq!((RESET_QUEUE, NOTICE_QUEUE, SENDING), (256, 64, 4));
    }

    #[test]
    fn what_became_of_mail_work_prints_nothing_of_it() {
        for delivery in [
            Delivery::Sent,
            Delivery::Nothing,
            Delivery::Failed,
            Delivery::Held,
        ] {
            let printed = format!("{delivery:?}");
            assert!(["Sent", "Nothing", "Failed", "Held"].contains(&printed.as_str()));
        }
    }
}
