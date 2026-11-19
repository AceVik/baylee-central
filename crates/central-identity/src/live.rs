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
use central_api::{Error, social::MAX_CONTACTS};
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
/// Sessions whose dials are counted at once. A full table refuses a session
/// it does not hold; one entry is a digest and a count.
const DIALERS: usize = 4_096;
const MINUTE: Duration = Duration::from_secs(60);

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
    sockets: BTreeSet<u64>,
    /// Its sockets that said `ready`.
    live: usize,
    friends: BTreeSet<[u8; 16]>,
    /// Whether `friends` was read since the entry was made.
    read: bool,
    /// The newest reading of `friends` asked for; an older one that ends
    /// later is dropped.
    reading: u64,
}

#[derive(Default)]
struct State {
    next: u64,
    sockets: BTreeMap<u64, Socket>,
    accounts: BTreeMap<[u8; 16], Account>,
    /// Each session's dials in its minute.
    dials: BTreeMap<[u8; 32], (Instant, u32)>,
}

impl State {
    fn raise(&self, account: &[u8; 16], bits: u8) {
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

    fn raise_followers(&self, account: &[u8; 16], bits: u8) {
        for follower in self.followers(account) {
            self.raise(&follower, bits);
        }
    }
}

/// The registry; one per service, shared by both listeners' changes.
#[derive(Default)]
pub(super) struct Live {
    state: Mutex<State>,
}

/// A friend list to read for an account, numbered so that only the newest
/// reading is kept ([`Live::friends_read`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Reading {
    account: [u8; 16],
    number: u64,
}

impl Reading {
    pub(super) const fn account(self) -> [u8; 16] {
        self.account
    }
}

impl Live {
    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        // Nothing panics while the state is held; a poisoned one is still whole.
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Spend one of the session's dials in its minute; `Busy` when none is left
    /// or the table holds as many sessions as it may.
    pub(super) fn dial(&self, digest: [u8; 32], now: Instant) -> Result<(), Error> {
        let mut state = self.state();
        if !state.dials.contains_key(&digest) && state.dials.len() >= DIALERS {
            state
                .dials
                .retain(|_, (start, _)| now.saturating_duration_since(*start) < MINUTE);
            if state.dials.len() >= DIALERS {
                return Err(Error::Busy);
            }
        }
        let (start, used) = state.dials.entry(digest).or_insert((now, 0));
        if now.saturating_duration_since(*start) >= MINUTE {
            *start = now;
            *used = 0;
        }
        if *used >= SESSION_DIALS {
            return Err(Error::Busy);
        }
        *used += 1;
        Ok(())
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
        digest: [u8; 32],
    ) -> Result<(Listener, Option<Reading>), Error> {
        let mut state = self.state();
        let id = state.next;
        let entry = state.accounts.entry(account).or_insert_with(|| Account {
            tag,
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
            entry.reading += 1;
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
        let mut state = self.state();
        let entry = state.accounts.get_mut(&account)?;
        entry.reading += 1;
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

    /// A reading that failed: what the account's sockets would say of their
    /// friends may be wrong, an ended friendship's presence included, so they
    /// close and their clients dial again, which reads it anew.
    pub(super) fn friends_unread(&self, reading: Reading) {
        let state = self.state();
        if state
            .accounts
            .get(&reading.account)
            .is_some_and(|entry| entry.reading == reading.number)
        {
            state.raise(&reading.account, news::STALE);
        }
    }

    /// Wake `account`'s sockets with `bits`.
    pub(super) fn raise(&self, account: &[u8; 16], bits: u8) {
        self.state().raise(account, bits);
    }

    /// The session `digest` ended: its sockets say `revoked`. A sign-out ends
    /// exactly the session that asked, so nothing need be read.
    pub(super) fn ended(&self, digest: &[u8; 32]) {
        let state = self.state();
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
    /// new one, in their contacts and their presence.
    pub(super) fn retagged(&self, account: &[u8; 16], tag: u32) {
        let mut state = self.state();
        let Some(entry) = state.accounts.get_mut(account) else {
            return;
        };
        entry.tag = Some(tag);
        state.raise(account, news::SOCIAL);
        state.raise_followers(account, news::SOCIAL | news::PRESENCE);
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

    /// The tags of `account`'s friends that are online now, in order: what
    /// its sockets report. At most [`MAX_CONTACTS`], as many as it may have.
    fn presence(&self, account: &[u8; 16]) -> Vec<u32> {
        let state = self.state();
        let Some(entry) = state.accounts.get(account) else {
            return Vec::new();
        };
        let mut tags: Vec<u32> = entry
            .friends
            .iter()
            .filter_map(|friend| state.accounts.get(friend))
            .filter(|friend| friend.live > 0)
            .filter_map(|friend| friend.tag)
            .collect();
        tags.sort_unstable();
        tags.dedup();
        tags.truncate(MAX_CONTACTS);
        tags
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
        let Some(entry) = state.accounts.get_mut(&account) else {
            return;
        };
        entry.live += 1;
        if entry.live == 1 {
            state.raise_followers(&account, news::PRESENCE);
        }
    }

    fn remove(&self, id: u64) {
        let mut state = self.state();
        let Some(socket) = state.sockets.remove(&id) else {
            return;
        };
        let account = socket.account;
        let Some(entry) = state.accounts.get_mut(&account) else {
            return;
        };
        entry.sockets.remove(&id);
        let offline = if socket.live {
            entry.live = entry.live.saturating_sub(1);
            entry.live == 0
        } else {
            false
        };
        if entry.sockets.is_empty() {
            state.accounts.remove(&account);
        }
        if offline {
            state.raise_followers(&account, news::PRESENCE);
        }
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
        let (listener, reading) =
            self.live
                .admit(status.identity.account, status.tag, token.digest())?;
        if let Some(reading) = reading {
            // The dial fails rather than report friends it could not read;
            // dropping the listener takes the socket back out.
            let friends = self.store.live_friends(reading.account()).await?;
            self.live.friends_read(reading, friends);
        }
        Ok(listener)
    }

    /// Whether `listener`'s session still stands: one statement.
    ///
    /// # Errors
    /// `Unavailable` when the store cannot say, which is not an ended session.
    pub async fn listening(&self, listener: &Listener) -> Result<bool, Error> {
        let digest = *listener.digest();
        Ok(self.store.live_sessions(&[digest]).await?.contains(&digest))
    }

    /// What `listener` reports as its account's online friends.
    #[must_use]
    pub fn presence(&self, listener: &Listener) -> Vec<u32> {
        listener
            .account()
            .map(|account| self.live.presence(&account))
            .unwrap_or_default()
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
    /// of exactly those accounts. A friendship that may have begun or ended is
    /// read again for each of the two that holds a socket, before anyone is
    /// told, so presence never shows an ended friendship's.
    pub(super) async fn tell(&self, concerns: &central_postgres::Concerns) {
        if let Some((one, other)) = concerns.pair {
            for account in [one, other] {
                if let Some(reading) = self.live.reread(account) {
                    match self.store.live_friends(account).await {
                        Ok(friends) => self.live.friends_read(reading, friends),
                        Err(_) => self.live.friends_unread(reading),
                    }
                }
            }
        }
        for account in &concerns.chat {
            self.live.raise(account, news::CHAT);
        }
        for account in &concerns.social {
            self.live.raise(account, news::SOCIAL);
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
