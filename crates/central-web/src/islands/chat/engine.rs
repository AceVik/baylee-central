//! The open chat in the browser: one Worker holding this account's chat
//! identity, and one task that does the chat's work strictly one job after
//! another (start, read the inbox, read keys, send, try again, confirm), as
//! the previous site's serial Worker did, so no answer can overtake another.
//! What the page shows lives in a signal ([`Shown`]); the Worker's keys and
//! every message's key stay in the Worker.
//!
//! The inbox is read every 2 s while the page is in view, at once while
//! Central says more is waiting, and backing off to 30 s after failures;
//! a hidden page reads nothing. The cursor is in memory, so each page load
//! replays what Central retains (`docs/central-chat.md`). Central's refusal
//! of the session ends the chat and forgets the identity kept here.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt::Write as _;
use std::future::Future;
use std::pin::Pin;
use std::rc::{Rc, Weak};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use central_api::chat::{AccountKeys, Delivery, DeviceRegistration, Sealed};
use leptos::prelude::*;
use serde::Deserialize;
use serde_json::json;

use super::super::b64;
use super::super::central::Failure;
use super::super::crypto::{Crypto, field_bytes};
use super::super::session;
use super::model::{Message, Review, merge, tag_of};
use super::{Pending, Phase, Shown, time, wire};

const QUICK: u32 = 2_000;
const SLOWEST: u32 = 30_000;

/// The chat's work, in the order it was asked for.
pub enum Job {
    /// Restore the trusted keys, register this device, read the own keys.
    Start,
    /// Read the next inbox page.
    Poll,
    /// Read the keys of these tags.
    Keys(Vec<u32>),
    /// Seal and send a text.
    Send {
        conv: String,
        text: String,
        members: Vec<u32>,
    },
    /// Send the waiting message again, unchanged.
    Retry,
    /// Confirm exactly these keys after comparing the safety number.
    Confirm(AccountKeys),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Opened {
    seq: u64,
    message: Option<Message>,
}

/// The open chat.
pub struct Engine {
    crypto: Crypto,
    shown: RwSignal<Shown>,
    own: u32,
    device: String,
    registration: DeviceRegistration,
    /// localStorage `baylee-chat-trust:v1:<account hex>`: the peer keys
    /// trusted, encrypted and authenticated by the Worker.
    trust_key: String,
    jobs: RefCell<VecDeque<Job>>,
    waker: RefCell<Option<Waker>>,
    closed: Cell<bool>,
    restored: Cell<bool>,
    cursor: Cell<u64>,
    delay: Cell<u32>,
    timer: Cell<Option<TimeoutHandle>>,
    listener: RefCell<Option<WindowListenerHandle>>,
    /// The keys last given to the Worker, by tag, as JSON.
    directory: RefCell<BTreeMap<String, String>>,
    /// Tags that have at least one device to write to.
    available: RefCell<BTreeSet<u32>>,
}

impl Engine {
    /// The chat of the identity the Worker holds, for `account`'s `own` tag.
    pub fn new(
        crypto: Crypto,
        shown: RwSignal<Shown>,
        own: u32,
        account: &[u8; 16],
        device: String,
        registration: DeviceRegistration,
    ) -> Rc<Engine> {
        let hex = account.iter().fold(String::new(), |mut hex, b| {
            let _ = write!(hex, "{b:02x}");
            hex
        });
        let engine = Rc::new(Engine {
            crypto,
            shown,
            own,
            device,
            registration,
            trust_key: format!("baylee-chat-trust:v1:{hex}"),
            jobs: RefCell::default(),
            waker: RefCell::default(),
            closed: Cell::new(false),
            restored: Cell::new(false),
            cursor: Cell::new(0),
            delay: Cell::new(QUICK),
            timer: Cell::new(None),
            listener: RefCell::new(None),
            directory: RefCell::default(),
            available: RefCell::default(),
        });
        let weak = Rc::downgrade(&engine);
        let listener = window_event_listener_untyped("visibilitychange", move |_| {
            if let Some(engine) = weak.upgrade()
                && !document().hidden()
            {
                engine.push(Job::Poll);
            }
        });
        *engine.listener.borrow_mut() = Some(listener);
        leptos::task::spawn_local(Rc::clone(&engine).run());
        engine.push(Job::Start);
        engine
    }

    /// Asks for `job`. A read already waiting is not asked for twice.
    pub fn push(&self, job: Job) {
        if self.closed.get() {
            return;
        }
        {
            let mut jobs = self.jobs.borrow_mut();
            if matches!(job, Job::Poll) && jobs.iter().any(|j| matches!(j, Job::Poll)) {
                return;
            }
            jobs.push_back(job);
        }
        self.wake();
    }

    fn wake(&self) {
        if let Some(waker) = self.waker.borrow_mut().take() {
            waker.wake();
        }
    }

    /// Ends the chat: no more work, the Worker and its keys are gone.
    pub fn close(&self) {
        if self.closed.replace(true) {
            return;
        }
        if let Some(timer) = self.timer.take() {
            timer.clear();
        }
        if let Some(listener) = self.listener.borrow_mut().take() {
            listener.remove();
        }
        self.jobs.borrow_mut().clear();
        self.crypto.end();
        self.wake();
    }

    /// Takes back the message waiting to be sent, unsent.
    pub fn discard(&self) {
        self.shown.update(|s| {
            if !s.busy {
                s.pending = None;
            }
        });
    }

    /// The next job, once there is one; none once the chat is closed.
    fn next(&self) -> Next<'_> {
        Next(self)
    }

    async fn run(self: Rc<Self>) {
        while let Some(job) = self.next().await {
            let outcome = match job {
                Job::Start => self.start().await,
                Job::Poll => self.poll().await,
                Job::Keys(tags) => self.observe(&tags).await,
                Job::Send {
                    conv,
                    text,
                    members,
                } => self.send(conv, text, members).await,
                Job::Retry => self.retry().await,
                Job::Confirm(keys) => self.confirm(keys).await,
            };
            if let Err(Failure::Rejected) = outcome {
                self.ended().await;
            }
        }
    }

    /// Central refused the session: it ended. The identity kept in this
    /// browser goes with it.
    async fn ended(&self) {
        session::forget();
        let _ = self.crypto.ask("forget", json!({})).await;
        self.shown.update(|s| s.phase = Phase::Ended);
        self.close();
    }

    fn schedule(self: &Rc<Self>, wait: u32) {
        if self.closed.get() {
            return;
        }
        if let Some(timer) = self.timer.take() {
            timer.clear();
        }
        let weak: Weak<Engine> = Rc::downgrade(self);
        let timer = set_timeout_with_handle(
            move || {
                if let Some(engine) = weak.upgrade() {
                    engine.push(Job::Poll);
                }
            },
            Duration::from_millis(u64::from(wait)),
        );
        self.timer.set(timer.ok());
    }

    fn failed(&self) {
        self.delay.set((self.delay.get() * 2).min(SLOWEST));
        self.shown.update(|s| {
            s.error = true;
            s.loading = false;
        });
    }

    fn read_trust(&self) -> Option<Vec<u8>> {
        session::read(&self.trust_key).map(|raw| {
            // unreadable trust is not dropped silently: the Worker refuses it
            if raw.len() > 1_400_000 {
                Vec::new()
            } else {
                b64::decode_standard(&raw).unwrap_or_default()
            }
        })
    }

    fn write_trust(&self, bytes: &[u8]) {
        session::write(&self.trust_key, &b64::encode(bytes));
    }

    async fn start(self: &Rc<Self>) -> Result<(), Failure> {
        let started = async {
            if !self.restored.get() {
                if let Some(trust) = self.read_trust() {
                    self.crypto
                        .ask("restore", json!({ "trust": trust }))
                        .await?;
                }
                self.restored.set(true);
            }
            let id = wire::register(self.registration.clone()).await?;
            if id != self.device {
                return Err(Failure::Malformed);
            }
            self.observe(&[self.own]).await
        }
        .await;
        match started {
            Ok(()) => {
                self.shown.update(|s| {
                    s.phase = Phase::Ready;
                    s.error = false;
                });
                self.push(Job::Poll);
                Ok(())
            }
            Err(failure) => {
                self.shown.update(|s| {
                    s.phase = Phase::Failed;
                    s.loading = false;
                });
                Err(failure)
            }
        }
    }

    async fn poll(self: &Rc<Self>) -> Result<(), Failure> {
        if self.shown.with_untracked(|s| s.phase != Phase::Ready) {
            return Ok(());
        }
        if document().hidden() {
            return Ok(());
        }
        match self.read().await {
            Ok(true) => {
                self.push(Job::Poll);
                Ok(())
            }
            Ok(false) => {
                self.schedule(self.delay.get());
                Ok(())
            }
            Err(failure) => {
                self.failed();
                self.schedule(self.delay.get());
                Err(failure)
            }
        }
    }

    async fn read(&self) -> Result<bool, Failure> {
        let (deliveries, more) = wire::inbox(&self.device, self.cursor.get()).await?;
        let senders: BTreeSet<u32> = deliveries.iter().map(|d| d.sender.tag).collect();
        if !senders.is_empty() {
            self.observe(&senders.into_iter().collect::<Vec<_>>())
                .await?;
        }
        let opened = self
            .crypto
            .ask("open", json!({ "messages": deliveries }))
            .await?;
        let opened: Vec<Opened> = opened
            .get("messages")
            .cloned()
            .and_then(|list| serde_json::from_value(list).ok())
            .ok_or(Failure::Malformed)?;
        let order_kept = opened.len() == deliveries.len()
            && opened.iter().zip(&deliveries).all(|(o, d)| o.seq == d.seq);
        if !order_kept {
            return Err(Failure::Malformed);
        }
        let mut refused = 0;
        let mut fresh = Vec::new();
        for (item, delivery) in opened.into_iter().zip(&deliveries) {
            match item.message {
                Some(m) if m.sender == delivery.sender.tag => fresh.push(m),
                _ => refused += 1,
            }
        }
        if let Some(last) = deliveries.last() {
            self.cursor.set(last.seq);
        }
        self.delay.set(QUICK);
        let names: Vec<(u32, String)> = deliveries
            .iter()
            .map(|d: &Delivery| (d.sender.tag, d.sender.display_name.clone()))
            .collect();
        // a quiet read changes nothing the page shows
        let quiet = fresh.is_empty()
            && refused == 0
            && self.shown.with_untracked(|s| {
                !s.loading
                    && !s.error
                    && (s.seen.is_some() || more)
                    && names
                        .iter()
                        .all(|(tag, name)| s.known.get(tag) == Some(name))
            });
        if quiet {
            return Ok(more);
        }
        self.shown.update(|s| {
            s.refused += refused + merge(&mut s.messages, fresh);
            s.known.extend(names);
            s.loading = false;
            s.error = false;
            if s.seen.is_none() && !more {
                let mut first = BTreeMap::new();
                for m in &s.messages {
                    first.insert(m.conv.clone(), m.at);
                }
                s.seen = Some(first);
            }
        });
        Ok(more)
    }

    /// Reads the keys of `tags` and gives what changed to the Worker, which
    /// keeps the trust and judges each peer's keys.
    async fn observe(&self, tags: &[u32]) -> Result<(), Failure> {
        let mut unique: Vec<u32> = tags.to_vec();
        unique.sort_unstable();
        unique.dedup();
        let keys = wire::keys(&unique).await?;
        {
            let mut available = self.available.borrow_mut();
            for tag in &unique {
                available.remove(tag);
            }
            for k in &keys {
                if let Some(tag) = tag_of(&k.tag).filter(|_| !k.devices.is_empty()) {
                    available.insert(tag);
                }
            }
        }
        let changed: Vec<(AccountKeys, String)> = keys
            .into_iter()
            .filter(|k| !k.devices.is_empty())
            .filter_map(|k| {
                let text = serde_json::to_string(&k).ok()?;
                (self.directory.borrow().get(&k.tag) != Some(&text)).then_some((k, text))
            })
            .collect();
        if changed.is_empty() {
            return Ok(());
        }
        let snapshots: Vec<&AccountKeys> = changed.iter().map(|(k, _)| k).collect();
        let result = self
            .crypto
            .ask("observe", json!({ "keys": snapshots }))
            .await?;
        let reviews: Vec<Review> = result
            .get("reviews")
            .cloned()
            .and_then(|list| serde_json::from_value(list).ok())
            .ok_or(Failure::Malformed)?;
        self.write_trust(&field_bytes(&result, "trust", None)?);
        {
            let mut directory = self.directory.borrow_mut();
            for (k, text) in changed {
                directory.insert(k.tag, text);
            }
        }
        self.shown.update(|s| {
            for review in reviews {
                if let Some(tag) = review.peer() {
                    s.reviews.insert(tag, review);
                }
            }
        });
        Ok(())
    }

    /// Whether every one of `members` can be written to now: keys present
    /// and none changed since they were trusted.
    fn writable(&self, members: &[u32]) -> bool {
        let available = self.available.borrow();
        self.shown.with_untracked(|s| {
            members
                .iter()
                .all(|tag| available.contains(tag) && s.reviews.get(tag).is_none_or(|r| !r.changed))
        })
    }

    /// The members a message to `conv` goes to now: a group's current
    /// members as Central lists them, while this account is one.
    async fn recipients(&self, conv: &str, members: Vec<u32>) -> Result<Vec<u32>, Failure> {
        let Some(id) = conv.strip_prefix("grp:") else {
            return Ok(members);
        };
        let snapshot = wire::groups().await?;
        let group = snapshot
            .groups
            .iter()
            .find(|g| g.id == id)
            .ok_or(Failure::Forbidden)?;
        let tags: Vec<u32> = group.members.iter().map(|m| m.person.tag).collect();
        if tags.contains(&self.own) {
            Ok(tags)
        } else {
            Err(Failure::Forbidden)
        }
    }

    async fn send(
        self: &Rc<Self>,
        conv: String,
        text: String,
        members: Vec<u32>,
    ) -> Result<(), Failure> {
        if self
            .shown
            .with_untracked(|s| s.pending.is_some() || s.phase != Phase::Ready)
        {
            self.shown.update(|s| s.returned = Some((conv, text)));
            return Ok(());
        }
        self.shown.update(|s| s.busy = true);
        let sealed = async {
            let members = self.recipients(&conv, members).await?;
            self.observe(&members).await?;
            if !self.writable(&members) {
                return Err(Failure::Conflict);
            }
            let answer = self
                .crypto
                .ask(
                    "seal",
                    json!({ "conv": conv, "text": text, "members": members, "now": time::now() }),
                )
                .await?;
            let sealed: Sealed = answer
                .get("sealed")
                .cloned()
                .and_then(|s| serde_json::from_value(s).ok())
                .ok_or(Failure::Malformed)?;
            Ok((members, sealed))
        }
        .await;
        let outcome = match sealed {
            Ok((members, sealed)) => {
                self.shown.update(|s| {
                    s.pending = Some(Pending {
                        conv: conv.clone(),
                        text: text.clone(),
                        members,
                        sealed,
                        failed: false,
                    });
                });
                self.deliver().await
            }
            Err(failure) => {
                self.shown.update(|s| {
                    s.error = true;
                    s.returned = Some((conv, text));
                });
                Err(failure)
            }
        };
        self.shown.update(|s| s.busy = false);
        outcome
    }

    async fn retry(self: &Rc<Self>) -> Result<(), Failure> {
        let Some(pending) = self.shown.with_untracked(|s| s.pending.clone()) else {
            return Ok(());
        };
        self.shown.update(|s| s.busy = true);
        let checked = async {
            let now = self
                .recipients(&pending.conv, pending.members.clone())
                .await?;
            let same = now.len() == pending.members.len()
                && now.iter().all(|tag| pending.members.contains(tag));
            self.observe(&pending.members).await?;
            Ok::<bool, Failure>(same && self.writable(&pending.members))
        }
        .await;
        let outcome = match checked {
            Ok(true) => self.deliver().await,
            // the members or their keys changed: this envelope is not
            // theirs any more; it can only be discarded and written again
            Ok(false) => {
                self.shown.update(|s| s.error = true);
                Ok(())
            }
            Err(failure) => {
                self.shown.update(|s| s.error = true);
                Err(failure)
            }
        };
        self.shown.update(|s| s.busy = false);
        outcome
    }

    async fn deliver(self: &Rc<Self>) -> Result<(), Failure> {
        let Some(pending) = self.shown.with_untracked(|s| s.pending.clone()) else {
            return Ok(());
        };
        match wire::send(&pending.conv, &pending.sealed).await {
            Ok(()) => {
                self.shown.update(|s| {
                    s.pending = None;
                    s.error = false;
                });
                self.push(Job::Poll);
                Ok(())
            }
            // Central did not take this exact envelope: the text goes back
            // to the composer to be sealed again for who is there now
            Err(Failure::Conflict) => {
                self.shown.update(|s| {
                    s.pending = None;
                    s.error = true;
                    s.returned = Some((pending.conv, pending.text));
                });
                Ok(())
            }
            Err(failure) => {
                self.shown.update(|s| {
                    if let Some(p) = s.pending.as_mut() {
                        p.failed = true;
                    }
                });
                Err(failure)
            }
        }
    }

    async fn confirm(self: &Rc<Self>, keys: AccountKeys) -> Result<(), Failure> {
        let result = self
            .crypto
            .ask("confirm", json!({ "snapshot": keys }))
            .await;
        let result = match result {
            Ok(result) => result,
            Err(failure) => {
                self.shown.update(|s| s.error = true);
                return Err(failure);
            }
        };
        self.write_trust(&field_bytes(&result, "trust", None)?);
        let tag = tag_of(&keys.tag);
        self.shown.update(|s| {
            if let Some(review) = tag.and_then(|t| s.reviews.get_mut(&t)) {
                review.changed = false;
            }
            s.refused = 0;
        });
        // messages held back while the keys were unconfirmed are read again
        self.cursor.set(0);
        self.push(Job::Poll);
        Ok(())
    }
}

/// Waits for the next job without a timer: `push` wakes it.
struct Next<'a>(&'a Engine);

impl Future for Next<'_> {
    type Output = Option<Job>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Job>> {
        let engine = self.0;
        if engine.closed.get() {
            return Poll::Ready(None);
        }
        if let Some(job) = engine.jobs.borrow_mut().pop_front() {
            return Poll::Ready(Some(job));
        }
        *engine.waker.borrow_mut() = Some(cx.waker().clone());
        Poll::Pending
    }
}
