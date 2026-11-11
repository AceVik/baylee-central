//! The messenger (`docs/proposals/frontend-rewrite.md` §6.3): contacts,
//! groups and the end-to-end encrypted chat in one island, so moving
//! between conversations and the contacts never reloads the page (a reload
//! starts the Worker again and replays the inbox). The server renders the
//! frame; everything here runs in the browser.
//!
//! The chat opens without a password while the sign-in lasts (owner,
//! 03.11.2026): the sign-in kept the identity in this browser and the
//! Worker resumes it. Only where nothing is kept (another browser, cleared
//! storage) does the page ask to sign in again, in the chat's own Worker.

mod glyph;
mod model;
mod people;
mod talk;
mod time;
mod wire;

mod engine;

use std::collections::BTreeMap;

use central_api::chat::{AccountKeys, Sealed};
use central_api::groups::{Change, Groups};
use central_api::social::{Contact, ContactAction};
use central_api::wire::{Request, Response};
use leptos::prelude::*;

use super::auth::{Problem, problem_text};
use super::central::{Failure, call};
use super::session;
use crate::data::Section;
use crate::words::{self, Lang};
use model::{Message, Review};

/// Who is signed in, as the messenger needs it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Me {
    /// The public tag.
    pub tag: u32,
    /// The display name.
    pub name: String,
    /// The account's id, to which the kept identity is bound.
    pub account: [u8; 16],
    /// Its key epoch.
    pub epoch: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Who {
    Loading,
    SignedOut,
    /// No display name yet, so no tag: nobody can be written to.
    Unnamed,
    Failed(Problem),
    Ready(Me),
}

async fn who() -> Who {
    if session::remembered().is_none() {
        return Who::SignedOut;
    }
    match call(&Request::AccountSummary).await {
        Ok(Response::AccountSummary {
            identity,
            display_name,
            tag,
            ..
        }) => match (display_name, tag) {
            (Some(name), Some(tag)) => Who::Ready(Me {
                tag,
                name,
                account: identity.account,
                epoch: identity.epoch,
            }),
            _ => Who::Unnamed,
        },
        Ok(_) => Who::Failed(Problem::Failed),
        Err(Failure::Rejected) => {
            session::forget();
            Who::SignedOut
        }
        Err(failure) => Who::Failed(failure.into()),
    }
}

/// How far the chat is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Phase {
    /// Opening the kept identity.
    #[default]
    Opening,
    /// Nothing is kept in this browser: sign in again to open the chat.
    Missing,
    /// Open.
    Ready,
    /// Central ended the session.
    Ended,
    /// Could not be opened.
    Failed,
}

/// A message sealed and on its way, kept until Central took it.
#[derive(Clone)]
pub struct Pending {
    /// The conversation.
    pub conv: String,
    /// What it says.
    pub text: String,
    /// Who it was sealed for.
    pub members: Vec<u32>,
    /// The envelope, sent again unchanged on "Try again".
    pub sealed: Sealed,
    /// Sending failed; it waits for "Try again" or "Discard".
    pub failed: bool,
}

/// What the chat shows.
#[derive(Clone)]
pub struct Shown {
    /// How far the chat is.
    pub phase: Phase,
    /// The messages, oldest first.
    pub messages: Vec<Message>,
    /// The display names the inbox gave senders.
    pub known: BTreeMap<u32, String>,
    /// The keys of each peer as judged, by tag.
    pub reviews: BTreeMap<u32, Review>,
    /// The message being sent.
    pub pending: Option<Pending>,
    /// Messages that did not authenticate or contradicted one shown.
    pub refused: usize,
    /// The inbox was not read yet.
    pub loading: bool,
    /// The last read or send failed.
    pub error: bool,
    /// A send is being prepared.
    pub busy: bool,
    /// The newest message time looked at, by conversation; in memory only.
    pub seen: Option<BTreeMap<String, u64>>,
    /// A text that could not be sent, back to its composer.
    pub returned: Option<(String, String)>,
}

impl Default for Shown {
    fn default() -> Self {
        Shown {
            phase: Phase::Opening,
            messages: Vec::new(),
            known: BTreeMap::new(),
            reviews: BTreeMap::new(),
            pending: None,
            refused: 0,
            loading: true,
            error: false,
            busy: false,
            seen: None,
            returned: None,
        }
    }
}

type Handle = std::rc::Rc<engine::Engine>;

/// A loaded list: not yet, or what Central answered.
pub type Loaded<T> = Option<Result<T, Failure>>;

/// The messenger's state and what the page can ask of it.
#[derive(Clone, Copy)]
pub struct Ctl {
    /// The chat.
    pub shown: RwSignal<Shown>,
    /// The contacts.
    pub contacts: RwSignal<Loaded<Vec<Contact>>>,
    /// The groups and invitations.
    pub groups: RwSignal<Loaded<Groups>>,
    /// A contact or group change is on its way.
    pub social_busy: RwSignal<bool>,
    /// The last contact or group change failed.
    pub social_failed: RwSignal<bool>,
    engine: StoredValue<Option<Handle>, LocalStorage>,
}

impl Ctl {
    fn new() -> Ctl {
        Ctl {
            shown: RwSignal::new(Shown::default()),
            contacts: RwSignal::new(None),
            groups: RwSignal::new(None),
            social_busy: RwSignal::new(false),
            social_failed: RwSignal::new(false),
            engine: StoredValue::new_local(None),
        }
    }

    fn push(self, job: engine::Job) {
        self.engine.with_value(|engine| {
            if let Some(engine) = engine {
                engine.push(job);
            }
        });
    }

    /// Seals `text` for `members` and sends it to `conv`.
    pub fn send(self, conv: String, text: String, members: Vec<u32>) {
        self.push(engine::Job::Send {
            conv,
            text,
            members,
        });
    }

    /// Sends the waiting message again, unchanged.
    pub fn retry(self) {
        self.push(engine::Job::Retry);
    }

    /// Takes back the waiting message.
    pub fn discard(self) {
        self.engine.with_value(|engine| {
            if let Some(engine) = engine {
                engine.discard();
            }
        });
    }

    /// Trusts exactly these keys, after the safety number was compared.
    pub fn confirm(self, keys: AccountKeys) {
        self.push(engine::Job::Confirm(keys));
    }

    /// Reads the keys of `tags` (a conversation's members).
    pub fn keys(self, tags: Vec<u32>) {
        self.push(engine::Job::Keys(tags));
    }

    /// Reads the inbox now.
    pub fn poll(self) {
        self.push(engine::Job::Poll);
    }

    /// The messages of `conv` up to now count as read.
    pub fn seen(self, conv: &str) {
        let newer = self.shown.with_untracked(|s| {
            let last = s.messages.iter().rev().find(|m| m.conv == conv)?.at;
            let seen = s.seen.as_ref()?;
            (seen.get(conv).copied().unwrap_or(0) < last).then_some(last)
        });
        if let Some(last) = newer {
            self.shown.update(|s| {
                if let Some(seen) = s.seen.as_mut() {
                    seen.insert(conv.to_owned(), last);
                }
            });
        }
    }

    fn ended(self, failure: Failure) {
        if failure == Failure::Rejected {
            session::forget();
            self.shown.update(|s| s.phase = Phase::Ended);
        }
    }

    /// Reads the contacts again.
    pub fn reload_contacts(self) {
        leptos::task::spawn_local(async move {
            let answer = wire::contacts().await;
            if let Err(failure) = answer {
                self.ended(failure);
            }
            // unchanged lists stay as they are, and so does the focus in them
            if self
                .contacts
                .with_untracked(|c| c.as_ref() != Some(&answer))
            {
                self.contacts.set(Some(answer));
            }
        });
    }

    /// Reads the groups again.
    pub fn reload_groups(self) {
        leptos::task::spawn_local(async move {
            let answer = wire::groups().await;
            if let Err(failure) = answer {
                self.ended(failure);
            }
            if self.groups.with_untracked(|g| g.as_ref() != Some(&answer)) {
                self.groups.set(Some(answer));
            }
        });
    }

    /// Changes the relation with `tag`; `done` learns whether it worked.
    pub fn contact(self, tag: u32, change: ContactAction, done: impl FnOnce(bool) + 'static) {
        self.social_busy.set(true);
        leptos::task::spawn_local(async move {
            let answer = wire::contact(tag, change).await;
            self.finish(answer.err());
            self.reload_contacts();
            done(answer.is_ok());
        });
    }

    /// Changes group `id`; `done` learns whether it worked.
    pub fn group(self, id: String, change: Change, done: impl FnOnce(bool) + 'static) {
        self.social_busy.set(true);
        leptos::task::spawn_local(async move {
            let answer = wire::change_group(&id, change).await;
            self.finish(answer.err());
            self.reload_groups();
            done(answer.is_ok());
        });
    }

    /// Creates a group; `done` gets its id.
    pub fn create_group(self, name: String, done: impl FnOnce(Option<String>) + 'static) {
        self.social_busy.set(true);
        leptos::task::spawn_local(async move {
            let answer = wire::create_group(&name).await;
            self.finish(answer.as_ref().err().copied());
            self.reload_groups();
            done(answer.ok());
        });
    }

    fn finish(self, failure: Option<Failure>) {
        self.social_busy.set(false);
        self.social_failed.set(failure.is_some());
        if let Some(failure) = failure {
            self.ended(failure);
        }
    }
}

/// Opens the chat: the identity kept in this browser, in a fresh Worker.
async fn open(worker: String, me: Me, ctl: Ctl) {
    use super::crypto::Crypto;

    let Some(crypto) = Crypto::start(&worker) else {
        ctl.shown.update(|s| s.phase = Phase::Failed);
        return;
    };
    let owner = serde_json::json!({ "account": me.account, "epoch": me.epoch, "tag": me.tag });
    if let Ok(ready) = crypto.ask("resume", owner).await {
        begin(crypto, &ready, &me, ctl);
    } else {
        crypto.close();
        ctl.shown.update(|s| {
            s.phase = Phase::Missing;
            s.loading = false;
        });
    }
}

/// Starts the chat with the identity the Worker holds.
fn begin(crypto: super::crypto::Crypto, ready: &serde_json::Value, me: &Me, ctl: Ctl) {
    let device = ready
        .get("device")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    let registration = ready
        .get("registration")
        .cloned()
        .and_then(|r| serde_json::from_value(r).ok());
    let (Some(device), Some(registration)) = (device, registration) else {
        crypto.close();
        ctl.shown.update(|s| s.phase = Phase::Failed);
        return;
    };
    let engine = engine::Engine::new(crypto, ctl.shown, me.tag, &me.account, device, registration);
    ctl.engine.set_value(Some(engine));
}

/// Where nothing was kept: signs in again in a Worker that then holds the
/// chat's identity, and starts the chat with it.
async fn rescue(
    lang: Lang,
    worker: String,
    me: Me,
    username: String,
    password: String,
    ctl: Ctl,
) -> Result<(), Problem> {
    use super::crypto::Crypto;

    let crypto = Crypto::start(&worker).ok_or(Problem::Failed)?;
    let signed = super::auth::sign_in_with(&crypto, &username, password).await?;
    match who().await {
        Who::Ready(now)
            if now.account == me.account && now.tag == me.tag && signed.new_code.is_none() =>
        {
            let ready = crypto
                .ask("unlocked", serde_json::json!({}))
                .await
                .map_err(Problem::from)?;
            ctl.shown.update(|s| s.phase = Phase::Opening);
            begin(crypto, &ready, &now, ctl);
            Ok(())
        }
        // another account, or one that needs its recovery code first:
        // start over from the account page
        _ => {
            session::go(&format!("/{}/account", lang.code()));
            Ok(())
        }
    }
}

/// The messenger's two halves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    /// Conversations.
    Chats,
    /// Contacts.
    Contacts,
}

/// Where the page is, read from its address.
fn address(own: u32) -> (Tab, Option<String>) {
    let Some(location) = leptos::web_sys::window().map(|w| w.location()) else {
        return (Tab::Chats, None);
    };
    let path = location.pathname().unwrap_or_default();
    let tab = if path.ends_with("/account/contacts") {
        Tab::Contacts
    } else {
        Tab::Chats
    };
    let search = location.search().unwrap_or_default();
    let mut conv = None;
    for pair in search.trim_start_matches('?').split('&') {
        match pair.split_once('=') {
            Some(("c", value)) => {
                let value = value.replace("%3A", ":").replace("%3a", ":");
                let valid = model::partner(&value, own).is_some()
                    || value.strip_prefix("grp:").is_some_and(wire::uuid_v7);
                if valid {
                    conv = Some(value);
                }
            }
            Some(("with", value)) => {
                if let Some(tag) = model::tag_of(value).filter(|t| *t != own) {
                    conv = Some(model::direct(own, tag));
                }
            }
            _ => {}
        }
    }
    (tab, conv)
}

/// The address of a tab and conversation.
#[must_use]
pub fn href(lang: Lang, tab: Tab, conv: Option<&str>) -> String {
    let code = lang.code();
    match (tab, conv) {
        (Tab::Contacts, _) => format!("/{code}/account/contacts"),
        (Tab::Chats, Some(conv)) => format!("/{code}/account/chat?c={conv}"),
        (Tab::Chats, None) => format!("/{code}/account/chat"),
    }
}

/// Puts the tab and conversation in the address bar, as a new entry.
pub fn remember(lang: Lang, tab: Tab, conv: Option<&str>) {
    if let Some(history) = leptos::web_sys::window().and_then(|w| w.history().ok()) {
        let _ = history.push_state_with_url(
            &leptos::wasm_bindgen::JsValue::NULL,
            "",
            Some(&href(lang, tab, conv)),
        );
    }
}

/// Where nothing is kept in this browser: sign in again to open the chat.
pub fn missing_view(lang: Lang, worker: String, me: Me, ctl: Ctl) -> impl IntoView {
    let username = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let error = RwSignal::new(None::<&'static str>);
    let busy = RwSignal::new(false);
    let submit = move |event: leptos::ev::SubmitEvent| {
        event.prevent_default();
        if busy.get_untracked() {
            return;
        }
        let (name, secret) = (username.get_untracked(), password.get_untracked());
        password.set(String::new());
        busy.set(true);
        let (worker, me) = (worker.clone(), me.clone());
        leptos::task::spawn_local(async move {
            if let Err(problem) = rescue(lang, worker, me, name, secret, ctl).await {
                error.set(Some(problem_text(lang, problem)));
            }
            busy.set(false);
        });
    };
    view! {
        <section class="chat-missing" aria-labelledby="chat-missing-title">
            <span class="chat-missing-icon">{glyph::glyph(glyph::LOCK)}</span>
            <h2 id="chat-missing-title">{words::chat_missing_title(lang)}</h2>
            <p>{words::chat_missing_lead(lang)}</p>
            <form class="form" method="post" novalidate=true on:submit=submit>
                <div class="field">
                    <label for="chat-name">{words::signin_name(lang)}</label>
                    <input id="chat-name" type="text" autocomplete="username" autocapitalize="none" spellcheck="false"
                        on:input=move |ev| username.set(event_target_value(&ev)) />
                </div>
                <div class="field">
                    <label for="chat-password">{words::signin_password(lang)}</label>
                    <input id="chat-password" type="password" autocomplete="current-password"
                        on:input=move |ev| password.set(event_target_value(&ev)) prop:value=move || password.get() />
                </div>
                <p class="error" role="alert">{move || error.get()}</p>
                <button type="submit" class="button primary" aria-disabled=move || busy.get().then_some("true")>
                    {move || if busy.get() { words::chat_opening(lang) } else { words::chat_open(lang) }}
                </button>
            </form>
        </section>
    }
}

#[island]
pub fn Messenger(lang: Lang, section: Section, worker: String) -> impl IntoView {
    let who_is = RwSignal::new(Who::Loading);
    let ctl = Ctl::new();
    let tab = RwSignal::new(if section == Section::Contacts {
        Tab::Contacts
    } else {
        Tab::Chats
    });
    let selected = RwSignal::new(None::<String>);
    let worker = StoredValue::new(worker);
    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            let found = who().await;
            if matches!(found, Who::Ready(_)) {
                ctl.reload_contacts();
                ctl.reload_groups();
            }
            if let Who::Ready(me) = &found {
                {
                    let (now_tab, conv) = address(me.tag);
                    tab.set(now_tab);
                    selected.set(conv);
                    let own = me.tag;
                    // for the page's life: the island is never taken down
                    let _ = window_event_listener(leptos::ev::popstate, move |_| {
                        let (now_tab, conv) = address(own);
                        tab.set(now_tab);
                        selected.set(conv);
                    });
                    leptos::task::spawn_local(open(worker.get_value(), me.clone(), ctl));
                    // contacts every 10 s and groups every 5 s while in view
                    let refresh = |every: u64, what: fn(Ctl)| {
                        set_interval(
                            move || {
                                if !document().hidden() {
                                    what(ctl);
                                }
                            },
                            std::time::Duration::from_millis(every),
                        );
                    };
                    refresh(10_000, Ctl::reload_contacts);
                    refresh(5_000, Ctl::reload_groups);
                }
            }
            who_is.set(found);
        });
    });
    let code = lang.code();
    // The server renders it, and it is the page's first sizeable text. It
    // stays one element until the messenger itself opens (then it hides),
    // so the page's largest paint never waits for the WebAssembly.
    let opening = view! {
        <p class="lead msgr-lead" hidden=move || matches!(who_is.get(), Who::Ready(_))>{words::chat_opening_lead(lang)}</p>
    };
    let state = move || {
        match who_is.get() {
        Who::Loading => view! { <p role="status" class="meta">{words::account_loading(lang)}</p> }.into_any(),
        Who::SignedOut => view! {
            <section class="card">
                <p>{words::account_signed_out(lang)}</p>
                <p><a class="button primary" href=format!("/{code}/signin")>{words::nav_sign_in(lang)}</a></p>
            </section>
        }
        .into_any(),
        Who::Unnamed => view! {
            <section class="card">
                <p>{words::chat_unnamed(lang)}</p>
                <p><a class="button primary" href=format!("/{code}/account/profile")>{words::account_needs_name(lang)}</a></p>
            </section>
        }
        .into_any(),
        Who::Failed(problem) => view! {
            <p class="error" role="alert">{problem_text(lang, problem)}</p>
        }
        .into_any(),
        Who::Ready(me) => talk::messenger(lang, &me, ctl, tab, selected, worker).into_any(),
    }
    };
    view! { {opening} {state} }
}
