//! The native push socket's registry (`docs/push-socket.md`): which sockets
//! are open, for which account and session, and what each must tell its
//! client. In memory only; nothing of it is stored, presence included.
//!
//! News is addressed. A change names the accounts it concerns
//! (`central_postgres::Concerns`, a session's account, a sign-out's session),
//! and only those accounts' sockets are woken: a socket never hears, nor is
//! woken by, what concerns somebody else. What a socket is to say is a set of
//! bits, so news that comes faster than a socket writes is merged, never
//! queued: nothing overflows and nothing is dropped but repetition.
//!
//! Presence is derived, never kept: an account is online while one of its
//! sockets has said `ready`; what a socket reports is the tags of its own
//! account's friends that are online. Each account's friends are read once,
//! when its first socket opens, and again when a contact of its changes;
//! going online or offline reads nothing.
use central_api::{
    Error,
    presence::{Activity, Choice, Shown},
    social::MAX_CONTACTS,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc, Mutex, PoisonError,
        atomic::{AtomicU8, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::Notify;

/// Sockets one account may hold at once. A client holds one; a redial while
/// the server has not yet noticed the old one's end holds a second for up to
/// the idle limit (`central-http`); more is no client of ours.
pub const ACCOUNT_SOCKETS: usize = 4;
/// Dials a minute with one session. The client redials after 2 s at the
/// earliest, doubling while dials fail; a dial costs two or three
/// statements, so this keeps one session from turning dials into load.
pub const SESSION_DIALS: u32 = 12;
/// Sessions whose dials are counted at once; one entry is a digest and a
/// count. A newcomer to a table full of live minutes takes the place of the
/// oldest, so tokens of ended sessions (which still verify until their
/// `exp`) can wear down others' counts but never lock a session out.
const DIALERS: usize = 4_096;
/// Reports of what a client is doing a minute, per session: a table entered
/// and left, a game begun and ended, idle and back, with room to spare.
pub const SESSION_REPORTS: u32 = 30;
/// Changes of the chosen status a minute, per session: each is a statement.
pub const SESSION_CHOICES: u32 = 10;
const MINUTE: Duration = Duration::from_secs(60);

/// Spend one of `key`'s units in its minute, `limit` a minute, in a table of
/// at most `cap` keys where a newcomer to a full table of live minutes takes
/// the oldest's place; `false` when none is left.
fn spend(
    book: &mut BTreeMap<[u8; 32], (Instant, u32)>,
    key: [u8; 32],
    now: Instant,
    limit: u32,
    cap: usize,
) -> bool {
    if !book.contains_key(&key) && book.len() >= cap {
        book.retain(|_, (start, _)| now.saturating_duration_since(*start) < MINUTE);
        if book.len() >= cap {
            let oldest = book
                .iter()
                .min_by_key(|(_, (start, _))| *start)
                .map(|(key, _)| *key);
            if let Some(oldest) = oldest {
                book.remove(&oldest);
            }
        }
    }
    let (start, used) = book.entry(key).or_insert((now, 0));
    if now.saturating_duration_since(*start) >= MINUTE {
        *start = now;
        *used = 0;
    }
    if *used >= limit {
        return false;
    }
    *used += 1;
    true
}

/// What a socket is to do, as bits of [`Listener::take`].
pub mod news {
    /// A delivery reached the account: say `chat`.
    pub const CHAT: u8 = 1;
    /// The account's contacts, groups or devices changed: say `social`.
    pub const SOCIAL: u8 = 2;
    /// The online friends may differ: say `presence` if they do.
    pub const PRESENCE: u8 = 4;
    /// The session may have ended: ask the store, and say `revoked` if it has.
    pub const CHECK: u8 = 8;
    /// The session has ended: say `revoked` and close.
    pub const REVOKED: u8 = 16;
    /// What the socket says can no longer be kept true (its friends could not
    /// be read again): close without `revoked`, so that the client dials anew.
    pub const STALE: u8 = 32;
}

/// One socket's doorbell: the bits it is to act on, and a wake-up.
struct Signal {
    bits: AtomicU8,
    bell: Notify,
}

impl Signal {
    fn raise(&self, bits: u8) {
        self.bits.fetch_or(bits, Ordering::AcqRel);
        // A permit is kept when nobody waits, so a raise between `take` and
        // the next wait is not lost.
        self.bell.notify_one();
    }
}

struct Socket {
    account: [u8; 16],
    digest: [u8; 32],
    signal: Arc<Signal>,
    /// It has said `ready`, so its account is online.
    live: bool,
}

struct Account {
    /// Its public tag, as its friends' presence names it.
    tag: Option<u32>,
    /// How it chose to be shown (stored; read when its first socket opened).
    choice: Choice,
    /// What its client says it is doing, and whether it is idle: memory only,
    /// gone with its last socket.
    activity: Option<Activity>,
    idle: bool,
    sockets: BTreeSet<u64>,
    /// Its sockets that said `ready`.
    live: usize,
    friends: BTreeSet<[u8; 16]>,
    /// Whether `friends` was read since the entry was made.
    read: bool,
    /// The newest reading of `friends` asked for; an older one that ends
    /// later is dropped. Numbered over all accounts ([`State::readings`]),
    /// so a reading begun before the account's sockets closed and opened
    /// again never takes a later one's number.
    reading: u64,
}

#[derive(Default)]
struct State {
    next: u64,
    sockets: BTreeMap<u64, Socket>,
    accounts: BTreeMap<[u8; 16], Account>,
    /// Each session's dials in its minute.
    dials: BTreeMap<[u8; 32], (Instant, u32)>,
    /// Each session's reports and changes of its choice, in their minute.
    reports: BTreeMap<[u8; 32], (Instant, u32)>,
    choices: BTreeMap<[u8; 32], (Instant, u32)>,
    /// The last reading's number, over all accounts.
    readings: u64,
    /// News that a session may have ended, or that a choice changed, counted:
    /// a dial compares it before and after its socket is registered
    /// ([`Live::changes`]).
    changes: u64,
}

impl State {
    fn raise(&mut self, account: &[u8; 16], bits: u8) {
        if bits & (news::CHECK | news::REVOKED) != 0 {
            self.changes += 1;
        }
        if let Some(entry) = self.accounts.get(account) {
            for id in &entry.sockets {
                if let Some(socket) = self.sockets.get(id) {
                    socket.signal.raise(bits);
                }
            }
        }
    }

    /// The accounts with a socket that count `account` among their friends:
    /// those whose presence shows it. Friendship is mutual, but each side's
    /// list is read at its own time; asking each side's own list keeps what
    /// one is told consistent with what it reports.
    fn followers(&self, account: &[u8; 16]) -> Vec<[u8; 16]> {
        self.accounts
            .iter()
            .filter(|(other, entry)| *other != account && entry.friends.contains(account))
            .map(|(other, _)| *other)
            .collect()
    }

    fn raise_followers(&mut self, account: &[u8; 16], bits: u8) {
        for follower in self.followers(account) {
            self.raise(&follower, bits);
        }
    }

    /// How `account` is shown to its friends now; `None` while it is not.
    fn shown(&self, account: &[u8; 16]) -> Option<Shown> {
        self.accounts.get(account).and_then(Account::shown)
    }

    /// The followers of `account` look at their presence again, if how it is
    /// shown is not what it was: so going online, offline, away or to a table
    /// is the only news of an account its friends get, and nothing of one
    /// that chose to be offline ever reaches them.
    fn shown_changed(&mut self, account: &[u8; 16], before: Option<Shown>) {
        if self.shown(account) != before {
            self.raise_followers(account, news::PRESENCE);
        }
    }
}

impl Account {
    /// How the account is shown: only while a socket of it said `ready` and
    /// it did not choose to be offline.
    fn shown(&self) -> Option<Shown> {
        (self.live > 0 && self.choice != Choice::Offline).then_some(Shown {
            away: self.choice == Choice::Afk || self.idle,
            activity: self.activity,
        })
    }
}

/// What a socket reports: its account's friends that are shown online, by
/// tag, each with its state, and the account's own choice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Presence {
    /// The friends' tags, in order.
    pub online: Vec<u32>,
    /// For each of them, its state ([`Shown::letter`]).
    pub states: Vec<Shown>,
    /// What the account itself chose.
    pub me: Choice,
}

/// The registry; one per service, shared by both listeners' changes.
#[derive(Default)]
pub(super) struct Live {
    state: Mutex<State>,
    /// Held from a choice's write, or a dial's second reading of it, until
    /// the registry has what it found ([`Live::settle`], [`Live::recheck`]):
    /// so the registry takes choices in the order the store did, and never
    /// ends at one the store no longer holds.
    choosing: tokio::sync::Mutex<()>,
}

/// A reading begun, which ends as read or, however else it ends (an error,
/// or its future dropped), as unread ([`Live::friends_unread`]): a list that
/// was to be read again is never left as it was.
pub(super) struct Pending {
    live: Arc<Live>,
    account: [u8; 16],
    reading: Option<Reading>,
}

impl Pending {
    pub(super) const fn new(live: Arc<Live>, reading: Reading) -> Self {
        Self {
            live,
            account: reading.account,
            reading: Some(reading),
        }
    }

    /// The account whose friends are to be read.
    pub(super) const fn account(&self) -> [u8; 16] {
        self.account
    }

    /// What the reading found.
    pub(super) fn read(mut self, friends: Vec<[u8; 16]>) {
        if let Some(reading) = self.reading.take() {
            self.live.friends_read(reading, friends);
        }
    }
}

impl Drop for Pending {
    fn drop(&mut self) {
        if let Some(reading) = self.reading.take() {
            self.live.friends_unread(reading);
        }
    }
}

/// A friend list to read for an account, numbered so that only the newest
/// reading is kept ([`Live::friends_read`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Reading {
    account: [u8; 16],
    number: u64,
}

impl Live {
    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        // Nothing panics while the state is held; a poisoned one is still whole.
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Spend one of the session's dials in its minute; `Busy` when none is
    /// left. A newcomer to a full table takes the oldest entry's place.
    pub(super) fn dial(&self, digest: [u8; 32], now: Instant) -> Result<(), Error> {
        if spend(&mut self.state().dials, digest, now, SESSION_DIALS, DIALERS) {
            Ok(())
        } else {
            Err(Error::Busy)
        }
    }

    /// Spend one of the session's changes of its choice in its minute.
    pub(super) fn choose(&self, digest: [u8; 32], now: Instant) -> Result<(), Error> {
        if spend(
            &mut self.state().choices,
            digest,
            now,
            SESSION_CHOICES,
            DIALERS,
        ) {
            Ok(())
        } else {
            Err(Error::Busy)
        }
    }

    /// The account chose `choice` (it is stored): its sockets report it, and
    /// its friends see the change if it changes how it is shown.
    pub(super) fn chose(&self, account: &[u8; 16], choice: Choice) {
        let mut state = self.state();
        state.changes += 1;
        let before = state.shown(account);
        let Some(entry) = state.accounts.get_mut(account) else {
            return;
        };
        entry.choice = choice;
        state.raise(account, news::PRESENCE);
        state.shown_changed(account, before);
    }

    /// Store a choice with `write` and then take it: one step that a dropped
    /// request does not cut short (it runs in a task of its own), and that no
    /// other choice of the store's overtakes. Without it a request dropped
    /// while the store wrote would leave the registry at the old choice, and
    /// two choices at once (the website and the client) could be taken in
    /// the order other than the store's: an account could be shown after it
    /// chose `offline`.
    pub(super) async fn settle<W>(self: &Arc<Self>, choice: Choice, write: W) -> Result<(), Error>
    where
        W: Future<Output = Result<[u8; 16], Error>> + Send + 'static,
    {
        let live = Arc::clone(self);
        let step = async move {
            let _turn = live.choosing.lock().await;
            let account = write.await?;
            live.chose(&account, choice);
            Ok(())
        };
        match tokio::runtime::Handle::try_current() {
            Ok(runtime) => runtime.spawn(step).await.unwrap_or(Err(Error::Unavailable)),
            Err(_) => step.await,
        }
    }

    /// Take the choice `read` finds for `account`, a dial's second reading of
    /// it, in turn with every [`Self::settle`]: a write the reading did not
    /// see is taken after it, never before. A dial dropped meanwhile takes
    /// nothing, and its socket is gone with it.
    pub(super) async fn recheck<R>(&self, account: &[u8; 16], read: R) -> Result<(), Error>
    where
        R: Future<Output = Result<Choice, Error>>,
    {
        let _turn = self.choosing.lock().await;
        let choice = read.await?;
        self.chose(account, choice);
        Ok(())
    }

    /// What the client of the session `digest` says it is doing. Only a
    /// session with a socket is heard, within [`SESSION_REPORTS`] a minute;
    /// anything else changes nothing and keeps nothing. The last report of
    /// any of the account's sockets stands.
    pub(super) fn report(
        &self,
        digest: [u8; 32],
        activity: Option<Activity>,
        idle: bool,
        now: Instant,
    ) {
        let mut guard = self.state();
        let state = &mut *guard;
        let Some(account) = state
            .sockets
            .values()
            .find(|socket| socket.digest == digest)
            .map(|socket| socket.account)
        else {
            return;
        };
        // Charged only for a session with a socket, so the table holds no
        // more sessions than there are sockets.
        if !spend(&mut state.reports, digest, now, SESSION_REPORTS, DIALERS) {
            return;
        }
        let before = state.shown(&account);
        if let Some(entry) = state.accounts.get_mut(&account) {
            entry.activity = activity;
            entry.idle = idle;
        }
        state.shown_changed(&account, before);
    }

    /// Admit a socket of `account` with the session `digest`, unless the
    /// account holds [`ACCOUNT_SOCKETS`] already. The socket is registered
    /// before it says `ready`, so no news from then on is missed; it counts as
    /// online only once it said so ([`Listener::ready`]). The reading is the
    /// friend list to read before the socket is used, while the account's has
    /// not been read.
    pub(super) fn admit(
        self: &Arc<Self>,
        account: [u8; 16],
        tag: Option<u32>,
        choice: Choice,
        digest: [u8; 32],
    ) -> Result<(Listener, Option<Reading>), Error> {
        let mut guard = self.state();
        let state = &mut *guard;
        let id = state.next;
        // A new entry takes the stored choice; one that stands has followed
        // every change since ([`Self::chose`]).
        let entry = state.accounts.entry(account).or_insert_with(|| Account {
            tag,
            choice,
            activity: None,
            idle: false,
            sockets: BTreeSet::new(),
            live: 0,
            friends: BTreeSet::new(),
            read: false,
            reading: 0,
        });
        if entry.sockets.len() >= ACCOUNT_SOCKETS {
            return Err(Error::Busy);
        }
        entry.tag = tag;
        entry.sockets.insert(id);
        let reading = (!entry.read).then(|| {
            state.readings += 1;
            entry.reading = state.readings;
            Reading {
                account,
                number: entry.reading,
            }
        });
        let signal = Arc::new(Signal {
            bits: AtomicU8::new(0),
            bell: Notify::new(),
        });
        state.sockets.insert(
            id,
            Socket {
                account,
                digest,
                signal: Arc::clone(&signal),
                live: false,
            },
        );
        state.next += 1;
        Ok((
            Listener {
                live: Arc::clone(self),
                id,
                digest,
                signal,
            },
            reading,
        ))
    }

    /// A reading of `account`'s friends, if it holds a socket: its list may
    /// have changed and is to be read again.
    pub(super) fn reread(&self, account: [u8; 16]) -> Option<Reading> {
        let mut guard = self.state();
        let state = &mut *guard;
        let entry = state.accounts.get_mut(&account)?;
        state.readings += 1;
        entry.reading = state.readings;
        Some(Reading {
            account,
            number: entry.reading,
        })
    }

    /// What `reading` found. Kept only if no newer reading was asked for since,
    /// so a slow reading never undoes a newer one; the account's sockets then
    /// look at their presence again, and so does every account whose presence
    /// it entered or left.
    pub(super) fn friends_read(&self, reading: Reading, friends: Vec<[u8; 16]>) {
        let mut state = self.state();
        let Some(entry) = state.accounts.get_mut(&reading.account) else {
            return;
        };
        if entry.reading != reading.number {
            return;
        }
        let friends: BTreeSet<[u8; 16]> = friends.into_iter().collect();
        // The first reading changes nobody else's presence: the account is
        // not online before it said `ready`.
        let changed: Vec<[u8; 16]> = if entry.read {
            entry
                .friends
                .symmetric_difference(&friends)
                .copied()
                .collect()
        } else {
            Vec::new()
        };
        entry.friends = friends;
        entry.read = true;
        state.raise(&reading.account, news::PRESENCE);
        for other in changed {
            state.raise(&other, news::PRESENCE);
        }
    }

    /// A reading that failed or was given up: what the account's sockets
    /// would say of their friends may be wrong, an ended friendship's
    /// presence included, so they close and their clients dial again. The
    /// list is forgotten, so that a socket that opens meanwhile reads it anew.
    pub(super) fn friends_unread(&self, reading: Reading) {
        let mut state = self.state();
        let Some(entry) = state.accounts.get_mut(&reading.account) else {
            return;
        };
        if entry.reading != reading.number {
            return;
        }
        entry.read = false;
        entry.friends.clear();
        state.raise(&reading.account, news::STALE);
    }

    /// Wake `account`'s sockets with `bits`.
    pub(super) fn raise(&self, account: &[u8; 16], bits: u8) {
        self.state().raise(account, bits);
    }

    /// The session `digest` ended: its sockets say `revoked`. A sign-out ends
    /// exactly the session that asked, so nothing need be read.
    pub(super) fn ended(&self, digest: &[u8; 32]) {
        let mut state = self.state();
        state.changes += 1;
        for socket in state.sockets.values() {
            if &socket.digest == digest {
                socket.signal.raise(news::REVOKED);
            }
        }
    }

    /// `account` is deleted: its sockets check their session (which ended with
    /// it), and every account whose friends it was forgets it and is told its
    /// contacts changed.
    pub(super) fn deleted(&self, account: &[u8; 16]) {
        let mut state = self.state();
        state.raise(account, news::CHECK);
        for follower in state.followers(account) {
            if let Some(entry) = state.accounts.get_mut(&follower) {
                entry.friends.remove(account);
            }
            state.raise(&follower, news::SOCIAL | news::PRESENCE);
        }
    }

    /// `account`'s tag changed: whoever counts it among their friends sees the
    /// new one, in their contacts and their presence, whether or not the
    /// account itself holds a socket.
    pub(super) fn retagged(&self, account: &[u8; 16], tag: u32) {
        let mut state = self.state();
        if let Some(entry) = state.accounts.get_mut(account) {
            entry.tag = Some(tag);
        }
        state.raise(account, news::SOCIAL);
        state.raise_followers(account, news::SOCIAL | news::PRESENCE);
    }

    /// How much news that a session may have ended, or that a choice changed,
    /// there has been: a dial whose count changed between its status and its
    /// registration asks the store again, since that news found no socket of
    /// it to tell.
    pub(super) fn changes(&self) -> u64 {
        self.state().changes
    }

    /// The session digests of every socket, each once: what the periodic
    /// check asks the store about.
    pub(super) fn digests(&self) -> BTreeSet<[u8; 32]> {
        self.state()
            .sockets
            .values()
            .map(|socket| socket.digest)
            .collect()
    }

    /// The store's answer about `asked`: the sockets whose session is not
    /// among `standing` say `revoked`. A socket that opened since the
    /// question was asked is not judged by it.
    pub(super) fn checked(&self, asked: &BTreeSet<[u8; 32]>, standing: &BTreeSet<[u8; 32]>) {
        let state = self.state();
        for socket in state.sockets.values() {
            if asked.contains(&socket.digest) && !standing.contains(&socket.digest) {
                socket.signal.raise(news::REVOKED);
            }
        }
    }

    /// Every socket closes without `revoked`: its session could not be
    /// checked for too long, so it may say what is no longer true.
    pub(super) fn stale(&self) {
        for socket in self.state().sockets.values() {
            socket.signal.raise(news::STALE);
        }
    }

    /// What `account`'s sockets report: its friends that are shown online
    /// now, in the order of their tags, each with its state, and its own
    /// choice. At most [`MAX_CONTACTS`] friends, as many as it may have.
    fn presence(&self, account: &[u8; 16]) -> Presence {
        let state = self.state();
        let Some(entry) = state.accounts.get(account) else {
            return Presence {
                online: Vec::new(),
                states: Vec::new(),
                me: Choice::Online,
            };
        };
        let mut shown: Vec<(u32, Shown)> = entry
            .friends
            .iter()
            .filter_map(|friend| state.accounts.get(friend))
            .filter_map(|friend| Some((friend.tag?, friend.shown()?)))
            .collect();
        shown.sort_unstable_by_key(|(tag, _)| *tag);
        shown.dedup_by_key(|(tag, _)| *tag);
        shown.truncate(MAX_CONTACTS);
        Presence {
            online: shown.iter().map(|(tag, _)| *tag).collect(),
            states: shown.into_iter().map(|(_, state)| state).collect(),
            me: entry.choice,
        }
    }

    fn ready(&self, id: u64) {
        let mut state = self.state();
        let Some(socket) = state.sockets.get_mut(&id) else {
            return;
        };
        if socket.live {
            return;
        }
        socket.live = true;
        let account = socket.account;
        let before = state.shown(&account);
        let Some(entry) = state.accounts.get_mut(&account) else {
            return;
        };
        entry.live += 1;
        state.shown_changed(&account, before);
    }

    fn remove(&self, id: u64) {
        let mut state = self.state();
        let Some(socket) = state.sockets.remove(&id) else {
            return;
        };
        let account = socket.account;
        let before = state.shown(&account);
        let Some(entry) = state.accounts.get_mut(&account) else {
            return;
        };
        entry.sockets.remove(&id);
        if socket.live {
            entry.live = entry.live.saturating_sub(1);
        }
        if entry.sockets.is_empty() {
            state.accounts.remove(&account);
        }
        state.shown_changed(&account, before);
    }

    #[cfg(test)]
    fn online(&self) -> usize {
        self.state()
            .accounts
            .values()
            .filter(|entry| entry.live > 0)
            .count()
    }
}

/// One admitted socket's place in the registry. Dropping it, however the
/// socket ends, removes it, and with its account's last socket that said
/// `ready` the account goes offline.
pub struct Listener {
    live: Arc<Live>,
    id: u64,
    digest: [u8; 32],
    signal: Arc<Signal>,
}

impl Listener {
    /// The bits raised since the last call ([`news`]), cleared.
    #[must_use]
    pub fn take(&self) -> u8 {
        self.signal.bits.swap(0, Ordering::AcqRel)
    }

    /// Wait until something is raised. Cancel-safe: the bits stay until
    /// [`Self::take`], and a raise while nobody waits is kept for the next wait.
    pub async fn rung(&self) {
        self.signal.bell.notified().await;
    }

    /// The socket said `ready`: its account is online from now on.
    pub fn ready(&self) {
        self.live.ready(self.id);
    }

    pub(super) const fn digest(&self) -> &[u8; 32] {
        &self.digest
    }

    pub(super) fn account(&self) -> Option<[u8; 16]> {
        self.live
            .state()
            .sockets
            .get(&self.id)
            .map(|socket| socket.account)
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        self.live.remove(self.id);
    }
}

impl std::fmt::Debug for Listener {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Neither the session's digest nor the account.
        formatter.write_str("Listener")
    }
}

impl super::IdentityService {
    /// Admit a push socket for the native session `presented`, from `place`
    /// (`docs/push-socket.md`). Counted like a status request (`limits.rs`)
    /// after its session's own dial budget ([`SESSION_DIALS`]); the session
    /// must stand and its account must have accepted the terms in force,
    /// since a socket is neither a status nor a sign-out. Then the account's
    /// own limit ([`ACCOUNT_SOCKETS`]), and its friends are read if no socket
    /// of it has done so yet.
    ///
    /// # Errors
    /// `Rejected` for a gateway credential or a session that does not stand,
    /// `TermsRequired`, `Busy` for a limit, `Unavailable` for the store.
    pub async fn listen(
        &self,
        presented: &super::Presented,
        place: Option<super::Place>,
    ) -> Result<Listener, Error> {
        let super::Presented::Session(token) = presented else {
            return Err(Error::Rejected);
        };
        self.live.dial(token.digest(), Instant::now())?;
        // Counted before the status is read: news that a session ended which
        // comes between it and the socket's registration finds no socket.
        let changes = self.live.changes();
        let request = central_api::wire::Request::SessionStatus;
        let told = super::limits::told(&request, Some(token));
        self.limits
            .lock()
            .map_err(|_| Error::Unavailable)?
            .request(&request, Some(token), place, Instant::now())?;
        let status = self
            .store
            .session_status(central_postgres::Session::Native(token))
            .await;
        // The status answer vouches for the bearer as a status request's does.
        let answer = match &status {
            Ok(_) => Ok(super::BrowserReply {
                response: central_api::wire::Response::Done,
                session: None,
            }),
            Err(error) => Err(*error),
        };
        self.answered(told.as_ref(), Some(token), &answer).await;
        let status = status?;
        self.require_terms(Some(token), super::Surface::Native)
            .await?;
        let account = status.identity.account;
        let (listener, reading) =
            self.live
                .admit(account, status.tag, status.presence, token.digest())?;
        if let Some(reading) = reading.map(|reading| Pending::new(Arc::clone(&self.live), reading))
        {
            // The dial fails rather than report friends it could not read;
            // dropping the listener takes the socket back out, and the
            // reading's end closes any other socket that waited for it.
            let friends = self.store.live_friends(reading.account()).await?;
            reading.read(friends);
        }
        // Such news came in between: the session and the choice are asked
        // about again, now that the socket is registered and any later news
        // reaches it. The socket has not said `ready`, so a choice corrected
        // here was never shown.
        if self.live.changes() != changes {
            self.live
                .recheck(&account, async {
                    self.store
                        .session_status(central_postgres::Session::Native(token))
                        .await
                        .map(|again| again.presence)
                })
                .await?;
        }
        Ok(listener)
    }

    /// A status request: a choice, or what the native client is doing.
    pub(super) async fn presence_request(
        &self,
        request: central_api::wire::Request,
        bearer: Option<&super::Token>,
        surface: super::Surface,
    ) -> Result<central_api::wire::Response, Error> {
        match request {
            central_api::wire::Request::SetPresence { choice } => {
                self.set_presence(choice, bearer, surface).await
            }
            central_api::wire::Request::ReportPresence { activity, idle } => {
                self.report_presence(activity, idle, bearer, surface)
            }
            _ => Err(Error::Rejected),
        }
    }

    /// Choose how the session's account is shown to its friends (on either
    /// listener): stored, then its sockets and its friends' are told.
    async fn set_presence(
        &self,
        choice: Choice,
        bearer: Option<&super::Token>,
        surface: super::Surface,
    ) -> Result<central_api::wire::Response, Error> {
        let session = surface.session(bearer)?;
        self.live
            .choose(bearer.ok_or(Error::Rejected)?.digest(), Instant::now())?;
        self.live
            .settle(choice, self.store.set_presence(session, choice))
            .await?;
        Ok(central_api::wire::Response::Presence { choice })
    }

    /// What the native client says it is doing: heard only from a session
    /// with a push socket, and always answered alike, so the answer says
    /// nothing of whether anything was kept.
    fn report_presence(
        &self,
        activity: Option<Activity>,
        idle: bool,
        bearer: Option<&super::Token>,
        surface: super::Surface,
    ) -> Result<central_api::wire::Response, Error> {
        if surface != super::Surface::Native {
            return Err(Error::Rejected);
        }
        let token = bearer.ok_or(Error::Rejected)?;
        self.live
            .report(token.digest(), activity, idle, Instant::now());
        Ok(central_api::wire::Response::Done)
    }

    /// Whether `listener`'s session still stands: one statement.
    ///
    /// # Errors
    /// `Unavailable` when the store cannot say, which is not an ended session.
    pub async fn listening(&self, listener: &Listener) -> Result<bool, Error> {
        let digest = *listener.digest();
        Ok(self.store.live_sessions(&[digest]).await?.contains(&digest))
    }

    /// What `listener` reports: its account's friends that are shown online,
    /// their states, and its own choice.
    #[must_use]
    pub fn presence(&self, listener: &Listener) -> Presence {
        listener.account().map_or(
            Presence {
                online: Vec::new(),
                states: Vec::new(),
                me: Choice::Online,
            },
            |account| self.live.presence(&account),
        )
    }

    /// The periodic check of every socket's session: one statement for all of
    /// them, however many there are; the sockets whose session no longer
    /// stands say `revoked`. It catches what ended a session without passing
    /// this service: its expiry, and the operator's tools.
    ///
    /// # Errors
    /// `Unavailable` when the store cannot say; nothing is revoked then.
    pub async fn check_listeners(&self) -> Result<(), Error> {
        let asked = self.live.digests();
        if asked.is_empty() {
            return Ok(());
        }
        let named: Vec<[u8; 32]> = asked.iter().copied().collect();
        let mut standing = BTreeSet::new();
        for chunk in named.chunks(central_postgres::LIVE_CHECK) {
            standing.extend(self.store.live_sessions(chunk).await?);
        }
        self.live.checked(&asked, &standing);
        Ok(())
    }

    /// The periodic check failed too often in a row: every socket closes
    /// without `revoked`, and its client dials again (or polls) once the
    /// store answers.
    pub fn listeners_unchecked(&self) {
        self.live.stale();
    }

    /// What a social operation that committed concerns, told to the sockets
    /// of exactly those accounts. A friendship that began or ended is read
    /// again for each of the two that holds a socket.
    ///
    /// Nothing here is awaited by the request: the bits are raised at once
    /// and the readings run in a task of their own, so neither the answer's
    /// time nor the caller's connection depends on whether the other side
    /// holds a socket, and a caller that goes away cancels nothing. A reading
    /// that fails, or a task that never runs, ends as unread ([`Pending`]):
    /// presence never keeps showing an ended friendship.
    pub(super) fn tell(&self, concerns: &central_postgres::Concerns) {
        for account in &concerns.chat {
            self.live.raise(account, news::CHAT);
        }
        for account in &concerns.social {
            self.live.raise(account, news::SOCIAL);
        }
        let readings: Vec<Pending> = concerns
            .pair
            .into_iter()
            .flat_map(|(one, other)| [one, other])
            .filter_map(|account| self.live.reread(account))
            .map(|reading| Pending::new(Arc::clone(&self.live), reading))
            .collect();
        if readings.is_empty() {
            return;
        }
        let store = self.store.clone();
        // Without a runtime the readings are dropped, which ends them as unread.
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            runtime.spawn(async move {
                for reading in readings {
                    // An error drops the reading, which ends it as unread.
                    if let Ok(friends) = store.live_friends(reading.account()).await {
                        reading.read(friends);
                    }
                }
            });
        }
    }

    /// The account's sessions may have ended (a sign-in that replaced one, a
    /// new password, a reset, a deletion, a session ended on the website): its
    /// sockets ask the store.
    pub(super) fn sessions_changed(&self, account: &[u8; 16]) {
        self.live.raise(account, news::CHECK);
    }
}

#[cfg(test)]
mod tests;
