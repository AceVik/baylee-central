//! What the administration's pages share: what each is given, their
//! navigation, a read and what its refusal means, names, the audit's table,
//! and a change's way from its button to Central.
//!
//! **A change** (the role, a tag, a gateway's approval, a token) is never
//! sent from its button. The button opens its confirmation, in the card of
//! what it changes; only the confirmation's own button sends, once: the
//! confirmation gives way to "Asking Baylee Central …" before anything
//! leaves, and a page that is working takes no second change. Central asks
//! for a password sign-in less than 30 minutes old: where
//! `AccountSummary` says it is older, the confirmation asks for the
//! password once more (in a Worker ended at once, as the reset code's page
//! does) and only then sends. A refusal is explained by what Central says
//! afterwards (the session, the role, and whether what was changed still
//! exists); an answer that did not arrive is never a refusal, and the page
//! reads again what it shows instead of guessing.

use std::collections::HashMap;

use central_api::admin::{AdminAnswer, AdminRequest, AuditEvent};
use central_api::wire::{Request, Response};
use leptos::prelude::*;

use super::Standing;
use crate::data::{Section, id_hex};
use crate::islands::auth::{Problem, problem_text};
use crate::islands::central::{Failure, call};
use crate::islands::chat::glyph;
use crate::islands::crypto::Crypto;
use crate::islands::field::{self, Labelled};
use crate::islands::password::{Trouble, sign_in_again};
use crate::islands::{clip, focus, session};
use crate::words::{self, Lang};

/// What every page of the administration is given.
#[derive(Clone, Copy)]
pub struct Cx {
    pub lang: Lang,
    /// The crypto Worker's address, for the password once more.
    pub worker: StoredValue<String>,
    /// This browser's sign-in name, for a password manager.
    pub username: StoredValue<String>,
    /// The account this browser is signed in as.
    pub own: [u8; 16],
    /// Runs when this browser turns out not to be signed in.
    pub signed_out: Callback<()>,
}

impl Cx {
    /// This browser is not signed in any more: the area says so.
    pub fn leave(self) {
        session::forget();
        self.signed_out.run(());
    }

    /// What to say of a read that gave nothing, `missing` for what Central
    /// refuses an administrator (something gone, no legacy store); `None`
    /// once the area was told that nobody is signed in.
    pub fn unread_text(self, unread: Unread, missing: &'static str) -> Option<&'static str> {
        match unread {
            Unread::SignedOut => {
                self.leave();
                None
            }
            Unread::Role => Some(words::admin_refused_role(self.lang)),
            Unread::Missing => Some(missing),
            Unread::Other(problem) => Some(problem_text(self.lang, problem)),
        }
    }
}

/// The administration's own navigation, on each of its pages and for an
/// administrator only: the island draws it, so no page the server renders
/// links there. The feedback's page is another island's.
pub fn nav(lang: Lang, section: Section) -> impl IntoView {
    let code = lang.code();
    let link = move |target: Section, label: &'static str| {
        // one account's page is the accounts' list's
        let current = if target == section {
            Some("page")
        } else if (target == Section::AdminUsers && matches!(section, Section::AdminUser(_)))
            || (target == Section::Feedback && section == Section::Report)
        {
            Some("true")
        } else {
            None
        };
        view! { <li><a href=format!("/{code}/{}", target.slug()) aria-current=current>{label}</a></li> }
    };
    view! {
        <nav class="admin-nav" aria-label=words::account_admin(lang)>
            <ul>
                {link(Section::AdminOverview, words::account_home(lang))}
                {link(Section::AdminUsers, words::admin_accounts(lang))}
                {link(Section::AdminGateways, words::gw_title(lang))}
                {link(Section::AdminAudit, words::admin_audit(lang))}
                {link(Section::AdminLegacy, words::admin_legacy(lang))}
                {link(Section::Feedback, words::admin_feedback(lang))}
                {link(Section::Admin, words::admin_reset_title(lang))}
            </ul>
        </nav>
    }
}

/// One account's page in the administration.
pub fn user_path(lang: Lang, account: &[u8; 16]) -> String {
    format!("/{}/account/admin/users/{}", lang.code(), id_hex(account))
}

/// An account as the administration names it: its handle, else `fallback`
/// (its sign-in name, or the end of its id).
pub fn handle(display_name: Option<&str>, tag: Option<u32>) -> Option<String> {
    display_name
        .zip(tag)
        .map(|(name, tag)| central_api::profile::handle(name, tag))
}

/// The last eight hex digits of an id, where nothing else names it.
pub fn id_end(id: &[u8; 16]) -> String {
    let hex = id_hex(id);
    format!("…{}", &hex[24..])
}

/// Why a read of the administration gave nothing to show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unread {
    /// Nobody is signed in here any more.
    SignedOut,
    /// This browser's account no longer holds the role.
    Role,
    /// Central refused an administrator: what was asked for is not there.
    Missing,
    /// Anything else, in the forms' shared words.
    Other(Problem),
}

/// What Central's refusal of a read means, by where this browser's account
/// stands afterwards: a refusal is a lost session only if Central refuses
/// `AccountSummary` too.
pub fn unread(standing: Standing) -> Unread {
    match standing {
        Standing::SignedOut => Unread::SignedOut,
        Standing::NotAdmin => Unread::Role,
        Standing::Admin => Unread::Missing,
        Standing::Unknown => Unread::Other(Problem::Failed),
    }
}

/// Asks Central `request`, a read.
///
/// # Errors
/// Why it gave nothing to show.
pub async fn read(request: AdminRequest) -> Result<AdminAnswer, Unread> {
    match call(&Request::Admin { request }).await {
        Ok(Response::Admin { answer }) => Ok(answer),
        Ok(_) => Err(Unread::Other(Problem::Failed)),
        Err(Failure::Rejected) => Err(unread(super::standing().await)),
        Err(failure) => Err(Unread::Other(Problem::outside_sign_in(failure))),
    }
}

/// The browser's clock, in Unix seconds.
pub fn now() -> i64 {
    i64::try_from(crate::islands::pow::now_ms() / 1000).unwrap_or(i64::MAX)
}

/// A button that copies `text`, named "Copy" and what it copies (`what`,
/// for a screen reader), and the line that says whether it worked.
pub fn copy_button(lang: Lang, text: String, what: &'static str) -> impl IntoView {
    let said = RwSignal::new(None::<&'static str>);
    let copy = move |_| {
        let text = text.clone();
        leptos::task::spawn_local(async move {
            said.set(Some(if clip::write(&text).await {
                words::code_copied(lang)
            } else {
                words::gws_copy_failed(lang)
            }));
        });
    };
    view! {
        <button type="button" class="button quiet admin-copy" on:click=copy>
            {glyph::glyph(glyph::COPY)}
            <span>{words::code_copy(lang)}</span>
            <span class="visually-hidden">{" "}{what}</span>
        </button>
        <span class="meta" role="status">{move || said.get()}</span>
    }
}

// ---- The audit's table ------------------------------------------------------

/// One audit event as the pages show it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub id: [u8; 16],
    pub at: i64,
    pub actor: Option<[u8; 16]>,
    pub action: String,
    pub account: Option<[u8; 16]>,
    pub gateway: Option<[u8; 16]>,
}

impl From<AuditEvent> for Event {
    fn from(event: AuditEvent) -> Self {
        Event {
            id: event.id,
            at: event.at,
            actor: event.actor,
            action: event.action,
            account: event.account,
            gateway: event.gateway,
        }
    }
}

/// Today's names of what the events name: the record keeps ids only.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Names {
    /// An account's handle, where it has one.
    pub accounts: HashMap<[u8; 16], String>,
    /// A gateway's name and origin.
    pub gateways: HashMap<[u8; 16], (String, String)>,
}

/// The words for an audited action Central names; `None` for one this page
/// does not know, which is shown as Central names it.
pub fn action_text(lang: Lang, action: &str) -> Option<&'static str> {
    Some(match action {
        "account.admin_granted" => words::admin_audit_admin_granted(lang),
        "account.admin_removed" => words::admin_audit_admin_removed(lang),
        "account.reset_issued" => words::admin_audit_reset_issued(lang),
        "account.tag_changed" => words::admin_audit_tag_changed(lang),
        "gateway.approved" => words::admin_audit_gateway_approved(lang),
        "gateway.disabled" => words::admin_audit_gateway_disabled(lang),
        "gateway.token_revoked" => words::admin_audit_token_revoked(lang),
        "registration.closed" => words::admin_audit_registration_closed(lang),
        "registration.opened" => words::admin_audit_registration_opened(lang),
        _ => return None,
    })
}

fn account_cell(lang: Lang, id: [u8; 16], names: &Names) -> impl IntoView + use<> {
    let found = names.accounts.get(&id).cloned();
    let shown = match found {
        Some(name) => view! { <bdi translate="no">{name}</bdi> }.into_any(),
        None => view! { <code translate="no">{id_end(&id)}</code> }.into_any(),
    };
    view! { <a href=user_path(lang, &id) title=id_hex(&id)>{shown}</a> }
}

fn gateway_cell(id: [u8; 16], names: &Names) -> impl IntoView + use<> {
    match names.gateways.get(&id).cloned() {
        Some((name, origin)) => view! { <span title=origin><bdi>{name}</bdi></span> }.into_any(),
        None => {
            view! { <code translate="no" title=id_hex(&id)>{format!("gw {}", id_end(&id))}</code> }
                .into_any()
        }
    }
}

/// The events as table rows; each row's heading (what was done) takes the
/// keyboard after "Show more" (`#admin-event-<id>`).
fn event_rows(lang: Lang, events: &[Event], names: &Names) -> impl IntoView + use<> {
    events
        .iter()
        .map(|event| {
            let what = match action_text(lang, &event.action) {
                Some(word) => word.into_any(),
                None => view! { <code translate="no">{event.action.clone()}</code> }.into_any(),
            };
            let by = match event.actor {
                None => words::admin_audit_operator(lang).into_any(),
                Some(actor) => account_cell(lang, actor, names).into_any(),
            };
            let account = event.account.map(|id| account_cell(lang, id, names));
            let between = (event.account.is_some() && event.gateway.is_some()).then_some(" · ");
            let gateway = event.gateway.map(|id| gateway_cell(id, names));
            view! {
                <tr>
                    <td class="num">{super::super::account::when(lang, event.at)}</td>
                    <th scope="row" tabindex="-1" id=format!("admin-event-{}", id_hex(&event.id))>{what}</th>
                    <td>{by}</td>
                    <td>{account}{between}{gateway}</td>
                </tr>
            }
        })
        .collect_view()
}

/// The audit's table of `events`, in a box that scrolls on its own where it
/// is wider than the page, labelled by the heading `label`.
pub fn audit_table(lang: Lang, label: &'static str, events: &[Event], names: &Names) -> AnyView {
    if events.is_empty() {
        return view! { <p>{words::admin_audit_none(lang)}</p> }.into_any();
    }
    view! {
        <div class="admin-table" role="region" aria-labelledby=label tabindex="0">
            <table>
                <thead>
                    <tr>
                        <th scope="col">{words::admin_audit_when(lang)}</th>
                        <th scope="col">{words::admin_audit_what(lang)}</th>
                        <th scope="col">{words::admin_audit_by(lang)}</th>
                        <th scope="col">{words::admin_audit_about(lang)}</th>
                    </tr>
                </thead>
                <tbody>{event_rows(lang, events, names)}</tbody>
            </table>
        </div>
    }
    .into_any()
}

// ---- A change ----------------------------------------------------------------

/// Central's 30 minutes, less a minute for the clocks: a sign-in older than
/// this by the browser's clock asks for the password before anything is
/// sent.
const FRESH_ENOUGH: i64 = super::FRESH - 60;

/// A change an administrator asks for: each needs a password sign-in less
/// than 30 minutes old and writes its audit event (`AdminRequest::fresh`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    /// Grant or remove an account's administrator role.
    Role { account: [u8; 16], admin: bool },
    /// Give an account another tag.
    Tag { account: [u8; 16], tag: u32 },
    /// Approve or enable a gateway again, or disable it.
    Approval { gateway: [u8; 16], enabled: bool },
    /// Revoke a gateway's token.
    Revoke { gateway: [u8; 16], token: [u8; 16] },
}

impl Change {
    /// The one request that makes it.
    pub fn request(self) -> AdminRequest {
        match self {
            Change::Role { account, admin } => AdminRequest::SetAdmin { account, admin },
            Change::Tag { account, tag } => AdminRequest::SetTag { account, tag },
            Change::Approval { gateway, enabled } => {
                AdminRequest::SetGatewayApproval { gateway, enabled }
            }
            Change::Revoke { token, .. } => AdminRequest::RevokeGatewayToken { token },
        }
    }

    /// The read that tells whether what it changes still exists.
    fn target(self) -> AdminRequest {
        match self {
            Change::Role { account, .. } | Change::Tag { account, .. } => {
                AdminRequest::Account { account }
            }
            Change::Approval { gateway, .. } | Change::Revoke { gateway, .. } => {
                AdminRequest::Gateway { gateway }
            }
        }
    }

    /// What Central refuses for when what it changes is gone.
    fn missing(self) -> Came {
        match self {
            Change::Role { .. } | Change::Tag { .. } => Came::AccountGone,
            Change::Approval { .. } | Change::Revoke { .. } => Came::GatewayGone,
        }
    }

    /// What the page says once it is made.
    pub fn done_text(self, lang: Lang) -> &'static str {
        match self {
            Change::Role { admin: true, .. } => words::admin_role_granted(lang),
            Change::Role { admin: false, .. } => words::admin_role_removed(lang),
            Change::Tag { .. } => words::admin_tag_saved(lang),
            Change::Approval { enabled: true, .. } => words::admin_gw_approved(lang),
            Change::Approval { enabled: false, .. } => words::admin_gw_disabled(lang),
            Change::Revoke { .. } => words::tokens_revoked_now(lang),
        }
    }
}

/// What a change came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Came {
    /// Central made it (or found it made already).
    Done,
    /// The sign-in is older than 30 minutes: the password once more.
    Password,
    /// The password typed is not right: nothing was asked of Central.
    WrongPassword,
    /// Nobody is signed in here any more.
    SignedOut,
    /// This browser is signed in as another account now: nothing was sent.
    Switched,
    /// This browser's account no longer holds the role.
    Role,
    /// Central counts the sign-in as old even after the password.
    Stale,
    /// The account is gone.
    AccountGone,
    /// The gateway is gone (its tokens with it).
    GatewayGone,
    /// Another account holds the tag, or the account's chat keeps its own.
    Taken,
    /// No answer came back: it may have been made.
    Lost,
    /// Anything else, in the forms' shared words: nothing was made.
    Other(Problem),
}

/// Whether a password sign-in at `signed_in` is fresh enough at `now`
/// (Unix seconds) to send a change without asking for it again.
fn fresh(signed_in: i64, now: i64) -> bool {
    now.saturating_sub(signed_in) < FRESH_ENOUGH
}

/// Before a change is sent, from `AccountSummary`: another account, no
/// role, or (unless the password was proven just now) a sign-in too old;
/// `None` where it may be sent.
fn before(
    admin: bool,
    account: [u8; 16],
    own: [u8; 16],
    signed_in: i64,
    now: i64,
    proven: bool,
) -> Option<Came> {
    if account != own {
        Some(Came::Switched)
    } else if !admin {
        Some(Came::Role)
    } else if !proven && !fresh(signed_in, now) {
        Some(Came::Password)
    } else {
        None
    }
}

/// What a failed change means before it is explained: `None` for Central's
/// refusal, which is explained by what stands afterwards. An answer that
/// did not arrive is no refusal.
fn at_change(change: Change, failure: Failure) -> Option<Came> {
    match failure {
        Failure::Rejected => None,
        Failure::Conflict if matches!(change, Change::Tag { .. }) => Some(Came::Taken),
        Failure::Conflict | Failure::Forbidden => Some(Came::Other(Problem::Failed)),
        Failure::Busy => Some(Came::Other(Problem::Busy)),
        Failure::Unavailable | Failure::Malformed | Failure::Network => Some(Came::Lost),
    }
}

/// Why Central refused `change`, from what it says afterwards: whether
/// this browser's account holds the role, and whether what was changed
/// still exists (`None`: not known). Central refuses a change of an
/// administrator's with a fresh sign-in only for something gone, so with
/// it there, the sign-in is the reason: the password is asked for, once.
fn explain(change: Change, admin: bool, exists: Option<bool>, proven: bool) -> Came {
    if !admin {
        Came::Role
    } else if exists == Some(false) {
        change.missing()
    } else if !proven {
        Came::Password
    } else if exists == Some(true) {
        Came::Stale
    } else {
        Came::Other(Problem::Failed)
    }
}

/// The words for what a change came to, where the page says it as it is;
/// `None` where it says something of its own (made, lost) or asks for the
/// password.
pub fn came_text(lang: Lang, came: Came) -> Option<&'static str> {
    match came {
        Came::Done | Came::Password | Came::SignedOut | Came::Lost => None,
        Came::WrongPassword => Some(words::password_error_current(lang)),
        Came::Switched => Some(words::admin_change_switched(lang)),
        Came::Role => Some(words::admin_refused_role(lang)),
        Came::Stale => Some(words::admin_refused_stale(lang)),
        Came::AccountGone => Some(words::admin_refused_gone(lang)),
        Came::GatewayGone => Some(words::admin_refused_gateway(lang)),
        Came::Taken => Some(words::admin_tag_taken(lang)),
        Came::Other(problem) => Some(problem_text(lang, problem)),
    }
}

/// `AccountSummary`: the role, the account and its password sign-in.
async fn summary() -> Result<(bool, [u8; 16], i64), Came> {
    match call(&Request::AccountSummary).await {
        Ok(Response::AccountSummary {
            identity,
            admin,
            signed_in_at,
            ..
        }) => Ok((admin, identity.account, signed_in_at)),
        Ok(_) => Err(Came::Other(Problem::Failed)),
        Err(Failure::Rejected) => Err(Came::SignedOut),
        Err(failure) => Err(Came::Other(Problem::outside_sign_in(failure))),
    }
}

/// Sends `change`, once, after `AccountSummary` said it may be; `proven`
/// where the password was proven just now.
async fn send(change: Change, own: [u8; 16], proven: bool) -> Came {
    let (admin, account, signed_in) = match summary().await {
        Ok(found) => found,
        Err(came) => return came,
    };
    if let Some(came) = before(admin, account, own, signed_in, now(), proven) {
        return came;
    }
    let failure = match call(&Request::Admin {
        request: change.request(),
    })
    .await
    {
        Ok(Response::Done) => return Came::Done,
        // an answer of another kind says nothing of what happened
        Ok(_) => return Came::Lost,
        Err(failure) => failure,
    };
    if let Some(came) = at_change(change, failure) {
        return came;
    }
    let admin = match summary().await {
        Ok((_, account, _)) if account != own => return Came::Switched,
        Ok((admin, ..)) => admin,
        Err(came) => return came,
    };
    let exists = if admin {
        match call(&Request::Admin {
            request: change.target(),
        })
        .await
        {
            Ok(Response::Admin { .. }) => Some(true),
            Err(Failure::Rejected) => Some(false),
            _ => None,
        }
    } else {
        None
    };
    explain(change, admin, exists, proven)
}

/// Proves the password once more in a Worker ended at once, checks that the
/// session is still `own`'s, and sends `change`.
async fn send_after(change: Change, own: [u8; 16], worker: &str, password: String) -> Came {
    let Some(crypto) = Crypto::start(worker) else {
        return Came::Other(Problem::Failed);
    };
    let signed = sign_in_again(&crypto, &password).await;
    crypto.close();
    drop(password);
    match signed {
        Ok((identity, _)) if identity.account != own => Came::Switched,
        Ok(_) => send(change, own, true).await,
        Err(Trouble::Current) => Came::WrongPassword,
        Err(Trouble::SignedOut) => Came::SignedOut,
        Err(Trouble::Unsure) => Came::Other(Problem::Failed),
        Err(Trouble::Other(problem)) => Came::Other(problem),
    }
}

/// A change as its confirmation asks for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asked {
    pub change: Change,
    /// The confirmation's question.
    pub title: String,
    /// What the change does.
    pub text: String,
    /// The button that makes it.
    pub action: &'static str,
    /// It takes something away (a role, a gateway, a token).
    pub danger: bool,
    /// Where the keyboard goes back to when the person steps back.
    pub from: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Stage {
    Idle,
    Confirm(Asked),
    Password(Asked),
    Working(Asked),
}

/// What the page says of the last change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Note {
    /// Made.
    Done(&'static str),
    /// Not made, or not known to be.
    Failed(&'static str),
}

/// What a page says once what it shows was read again after a change: a
/// lost answer, as far as the reading tells (`read`: it was read again).
pub fn lost(lang: Lang, read: bool) -> Note {
    Note::Failed(if read {
        words::admin_change_lost(lang)
    } else {
        words::admin_change_unknown(lang)
    })
}

/// Puts the keyboard on the line that says what the last change came to.
pub fn focus_note(note: RwSignal<Option<Note>>) {
    let at = match note.try_get_untracked().flatten() {
        Some(Note::Failed(_)) => "#admin-note",
        _ => "#admin-done",
    };
    focus::after_render(at);
}

/// A page's changes: one at a time, from its confirmation to its outcome.
#[derive(Clone, Copy)]
pub struct Flow {
    cx: Cx,
    stage: RwSignal<Stage>,
    password: RwSignal<String>,
    /// What the password's step says of the last attempt.
    wrong: RwSignal<Option<&'static str>>,
    /// A change is on its way: nothing else is sent.
    busy: StoredValue<bool>,
    /// What the page says of the last change.
    pub note: RwSignal<Option<Note>>,
    /// Runs when a change ended (made or not), for the page to read again
    /// what it shows and to say a lost answer.
    finished: Callback<(Change, Came)>,
}

impl Flow {
    /// A page's changes; `note` is the page's own, so that what it reads
    /// again in `finished` can say a lost answer.
    pub fn new(cx: Cx, note: RwSignal<Option<Note>>, finished: Callback<(Change, Came)>) -> Self {
        Flow {
            cx,
            stage: RwSignal::new(Stage::Idle),
            password: RwSignal::new(String::new()),
            wrong: RwSignal::new(None),
            busy: StoredValue::new(false),
            note,
            finished,
        }
    }

    /// A change's button: its confirmation opens, in its card, and takes the
    /// keyboard. Nothing is sent.
    pub fn ask(self, asked: Asked) {
        if self.busy.get_value() {
            return;
        }
        self.note.set(None);
        self.wrong.set(None);
        self.password.set(String::new());
        self.stage.set(Stage::Confirm(asked));
        focus::after_render("#admin-confirm-title");
    }

    /// "Back": nothing is sent, and the keyboard returns to the button.
    fn back(self) {
        if self.busy.get_value() {
            return;
        }
        let from = match self.stage.get_untracked() {
            Stage::Confirm(asked) | Stage::Password(asked) | Stage::Working(asked) => asked.from,
            Stage::Idle => return,
        };
        self.password.set(String::new());
        self.wrong.set(None);
        self.stage.set(Stage::Idle);
        focus::after_render(from);
    }

    /// Steps to the work before anything is sent: the confirmation's
    /// button is gone, and a second press finds nothing to press.
    fn start(self, asked: &Asked) -> bool {
        if self.busy.get_value() {
            return false;
        }
        self.busy.set_value(true);
        self.stage.set(Stage::Working(asked.clone()));
        focus::after_render("#admin-working");
        true
    }

    /// The confirmation's button: sends the change, once.
    fn confirm(self) {
        let Stage::Confirm(asked) = self.stage.get_untracked() else {
            return;
        };
        if !self.start(&asked) {
            return;
        }
        let own = self.cx.own;
        leptos::task::spawn_local(async move {
            let came = send(asked.change, own, false).await;
            self.end(asked, came);
        });
    }

    /// The password's submit: proves it, then sends the change, once.
    fn submit(self, event: &leptos::ev::SubmitEvent) {
        event.prevent_default();
        let Stage::Password(asked) = self.stage.get_untracked() else {
            return;
        };
        if self.busy.get_value() {
            return;
        }
        let password = self.password.get_untracked();
        if password.is_empty() {
            self.wrong
                .set(Some(words::admin_password_missing(self.cx.lang)));
            focus::after_render("#admin-password");
            return;
        }
        self.password.set(String::new());
        self.wrong.set(None);
        if !self.start(&asked) {
            return;
        }
        let own = self.cx.own;
        let worker = self.cx.worker.get_value();
        leptos::task::spawn_local(async move {
            let came = send_after(asked.change, own, &worker, password).await;
            self.end(asked, came);
        });
    }

    /// What the change came to.
    fn end(self, asked: Asked, came: Came) {
        // a page that went meanwhile shows nothing more
        if self.busy.try_get_value().is_none() {
            return;
        }
        self.busy.set_value(false);
        let lang = self.cx.lang;
        match came {
            Came::Password => {
                self.stage.set(Stage::Password(asked));
                focus::after_render("#admin-password");
            }
            Came::WrongPassword => {
                self.wrong.set(came_text(lang, came));
                self.stage.set(Stage::Password(asked));
                focus::after_render("#admin-password");
            }
            Came::SignedOut => self.cx.leave(),
            came => {
                self.stage.set(Stage::Idle);
                self.note.set(match came {
                    Came::Done => Some(Note::Done(asked.change.done_text(lang))),
                    came => came_text(lang, came).map(Note::Failed),
                });
                self.finished.run((asked.change, came));
            }
        }
    }

    /// The lines that say what the last change came to: made (a status) or
    /// not (an alert). Each takes the keyboard when the page sends it there.
    pub fn notes(self) -> impl IntoView {
        let done = move || match self.note.get() {
            Some(Note::Done(text)) => Some(text),
            _ => None,
        };
        let failed = move || match self.note.get() {
            Some(Note::Failed(text)) => Some(text),
            _ => None,
        };
        view! {
            <p class="admin-done" role="status" id="admin-done" tabindex="-1">{done}</p>
            <p class="error" role="alert" id="admin-note" tabindex="-1">{failed}</p>
        }
    }

    /// The confirmation, the password or the work, where `here` says the
    /// change belongs: in the card of what it changes.
    pub fn panel(self, here: impl Fn(&Change) -> bool + Send + Sync + 'static) -> impl IntoView {
        move || match self.stage.get() {
            Stage::Confirm(asked) if here(&asked.change) => confirm_view(self, &asked),
            Stage::Password(asked) if here(&asked.change) => password_view(self, &asked),
            Stage::Working(asked) if here(&asked.change) => view! {
                <p class="working admin-working" role="status" tabindex="-1" id="admin-working">
                    {words::admin_working(self.cx.lang)}
                </p>
            }
            .into_any(),
            _ => ().into_any(),
        }
    }
}

fn confirm_view(flow: Flow, asked: &Asked) -> AnyView {
    let lang = flow.cx.lang;
    let class = if asked.danger {
        "button danger"
    } else {
        "button primary"
    };
    view! {
        <div class="admin-confirm" role="group" aria-labelledby="admin-confirm-title">
            <h3 id="admin-confirm-title" tabindex="-1">{asked.title.clone()}</h3>
            <p>{asked.text.clone()}</p>
            <p class="actions">
                <button type="button" class=class id="admin-confirm" on:click=move |_| flow.confirm()>{asked.action}</button>
                <button type="button" class="button quiet" on:click=move |_| flow.back()>{words::admin_back(lang)}</button>
            </p>
        </div>
    }
    .into_any()
}

fn password_view(flow: Flow, asked: &Asked) -> AnyView {
    let lang = flow.cx.lang;
    let spec = Labelled {
        id: "admin-password",
        label: words::privacy_delete_password(lang),
        hint: Some(words::admin_change_password(lang)),
        mark: glyph::LOCK,
        auto: "current-password",
        max: "1024",
    };
    let class = if asked.danger {
        "button danger"
    } else {
        "button primary"
    };
    view! {
        <div class="admin-confirm" role="group" aria-labelledby="admin-confirm-title">
            <h3 id="admin-confirm-title">{asked.title.clone()}</h3>
            <form class="form" method="post" novalidate=true on:submit=move |event| flow.submit(&event)>
                // for a password manager: whose password this is
                <input type="text" autocomplete="username" readonly=true hidden=true value=flow.cx.username.get_value() />
                {field::labelled(lang, spec, move || flow.password.get(), move |value| flow.password.set(value))}
                <p class="error" role="alert">{move || flow.wrong.get()}</p>
                <p class="actions">
                    <button type="submit" class=class>{asked.action}</button>
                    <button type="button" class="button quiet" on:click=move |_| flow.back()>{words::admin_back(lang)}</button>
                </p>
            </form>
        </div>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    const OWN: [u8; 16] = [1; 16];
    const OTHER: [u8; 16] = [2; 16];
    const NOW: i64 = 1_800_000_000;

    const FAILURES: [Failure; 7] = [
        Failure::Rejected,
        Failure::Forbidden,
        Failure::Conflict,
        Failure::Busy,
        Failure::Unavailable,
        Failure::Malformed,
        Failure::Network,
    ];

    fn changes() -> [Change; 6] {
        [
            Change::Role {
                account: OTHER,
                admin: true,
            },
            Change::Role {
                account: OTHER,
                admin: false,
            },
            Change::Tag {
                account: OTHER,
                tag: 0x1a2b,
            },
            Change::Approval {
                gateway: OTHER,
                enabled: true,
            },
            Change::Approval {
                gateway: OTHER,
                enabled: false,
            },
            Change::Revoke {
                gateway: OTHER,
                token: [3; 16],
            },
        ]
    }

    /// Nothing is sent for another account's session or without the role,
    /// and the password comes first for a sign-in older than 29 minutes,
    /// unless it was proven just now.
    #[test]
    fn what_stands_before_a_change_is_sent() {
        let fresh = NOW - 60;
        let old = NOW - FRESH_ENOUGH;
        assert_eq!(before(true, OWN, OWN, fresh, NOW, false), None);
        assert_eq!(
            before(true, OTHER, OWN, fresh, NOW, false),
            Some(Came::Switched)
        );
        assert_eq!(
            before(true, OTHER, OWN, fresh, NOW, true),
            Some(Came::Switched)
        );
        assert_eq!(before(false, OWN, OWN, fresh, NOW, false), Some(Came::Role));
        assert_eq!(
            before(true, OWN, OWN, old, NOW, false),
            Some(Came::Password)
        );
        assert_eq!(before(true, OWN, OWN, old, NOW, true), None);
        assert_eq!(
            before(true, OWN, OWN, old + 1, NOW, false),
            None,
            "a second under the margin"
        );
        // a clock behind Central's: Central decides, and is explained
        assert_eq!(before(true, OWN, OWN, NOW + 3600, NOW, false), None);
    }

    /// An answer that did not arrive is never a refusal, and only a tag
    /// reads a conflict as taken.
    #[test]
    fn a_lost_answer_is_no_refusal() {
        for change in changes() {
            for failure in FAILURES {
                let came = at_change(change, failure);
                match failure {
                    Failure::Rejected => assert_eq!(came, None),
                    Failure::Unavailable | Failure::Malformed | Failure::Network => {
                        assert_eq!(came, Some(Came::Lost));
                    }
                    Failure::Busy => assert_eq!(came, Some(Came::Other(Problem::Busy))),
                    Failure::Conflict if matches!(change, Change::Tag { .. }) => {
                        assert_eq!(came, Some(Came::Taken));
                    }
                    Failure::Conflict | Failure::Forbidden => {
                        assert_eq!(came, Some(Came::Other(Problem::Failed)));
                    }
                }
            }
        }
    }

    /// Central's refusal is explained by what stands afterwards: the role,
    /// then what was changed, then the sign-in, asked for once.
    #[test]
    fn a_refusal_is_explained_by_what_central_holds_now() {
        for change in changes() {
            let gone = match change {
                Change::Role { .. } | Change::Tag { .. } => Came::AccountGone,
                _ => Came::GatewayGone,
            };
            for proven in [false, true] {
                for exists in [None, Some(false), Some(true)] {
                    assert_eq!(explain(change, false, exists, proven), Came::Role);
                }
                assert_eq!(explain(change, true, Some(false), proven), gone);
            }
            assert_eq!(explain(change, true, Some(true), false), Came::Password);
            assert_eq!(explain(change, true, None, false), Came::Password);
            // after the password, a refusal is never answered with it again
            assert_eq!(explain(change, true, Some(true), true), Came::Stale);
            assert_eq!(
                explain(change, true, None, true),
                Came::Other(Problem::Failed)
            );
        }
    }

    /// A read Central refuses is a lost session only where Central refuses
    /// `AccountSummary` too.
    #[test]
    fn a_refused_read_is_explained_by_the_standing() {
        assert_eq!(unread(Standing::SignedOut), Unread::SignedOut);
        assert_eq!(unread(Standing::NotAdmin), Unread::Role);
        assert_eq!(unread(Standing::Admin), Unread::Missing);
        assert_eq!(unread(Standing::Unknown), Unread::Other(Problem::Failed));
    }

    /// Every outcome the page says itself has words of its own in every
    /// language, none reads as a lost answer or as another's, and every
    /// change says it was made in its own words.
    #[test]
    fn every_outcome_has_its_own_sentence() {
        let said = [
            Came::WrongPassword,
            Came::Switched,
            Came::Role,
            Came::Stale,
            Came::AccountGone,
            Came::GatewayGone,
            Came::Taken,
        ];
        for lang in Lang::ALL {
            let texts: Vec<&str> = said
                .iter()
                .map(|came| came_text(lang, *came).expect("words"))
                .collect();
            for (at, text) in texts.iter().enumerate() {
                assert_ne!(*text, "");
                assert!(!texts[..at].contains(text), "{lang:?}: {text}");
                assert_ne!(*text, words::admin_change_lost(lang));
                assert_ne!(*text, words::admin_change_unknown(lang));
            }
            for came in [Came::Done, Came::Password, Came::SignedOut, Came::Lost] {
                assert_eq!(came_text(lang, came), None);
            }
            let done: Vec<&str> = changes().iter().map(|c| c.done_text(lang)).collect();
            for (at, text) in done.iter().enumerate() {
                assert!(!done[..at].contains(text), "{lang:?}: {text}");
            }
        }
    }

    /// Each change is the one request Central reads, and its target's read.
    #[test]
    fn the_requests_serialize_as_central_reads_them() {
        let text = |request: AdminRequest| {
            serde_json::to_string(&Request::Admin { request }).expect("json")
        };
        let id = |n: u8| format!("[{}]", vec![n.to_string(); 16].join(","));
        let [grant, remove, tag, approve, disable, revoke] = changes();
        assert_eq!(
            text(grant.request()),
            format!(
                r#"{{"operation":"admin","request":{{"action":"set_admin","account":{},"admin":true}}}}"#,
                id(2)
            )
        );
        assert!(text(remove.request()).ends_with(r#""admin":false}}"#));
        assert_eq!(
            text(tag.request()),
            format!(
                r#"{{"operation":"admin","request":{{"action":"set_tag","account":{},"tag":6699}}}}"#,
                id(2)
            )
        );
        assert_eq!(
            text(approve.request()),
            format!(
                r#"{{"operation":"admin","request":{{"action":"set_gateway_approval","gateway":{},"enabled":true}}}}"#,
                id(2)
            )
        );
        assert!(text(disable.request()).ends_with(r#""enabled":false}}"#));
        assert_eq!(
            text(revoke.request()),
            format!(
                r#"{{"operation":"admin","request":{{"action":"revoke_gateway_token","token":{}}}}}"#,
                id(3)
            )
        );
        assert!(text(tag.target()).contains(r#""action":"account""#));
        assert!(text(revoke.target()).contains(r#""action":"gateway""#));
    }

    /// Every action Central writes has its words; another is shown as it is.
    #[test]
    fn every_audited_action_has_its_words() {
        let all = [
            "account.admin_granted",
            "account.admin_removed",
            "account.reset_issued",
            "account.tag_changed",
            "gateway.approved",
            "gateway.disabled",
            "gateway.token_revoked",
            "registration.closed",
            "registration.opened",
        ];
        for lang in Lang::ALL {
            let texts: Vec<&str> = all
                .iter()
                .map(|action| action_text(lang, action).expect("words"))
                .collect();
            for (at, text) in texts.iter().enumerate() {
                assert!(!texts[..at].contains(text), "{lang:?}: {text}");
            }
            assert_eq!(action_text(lang, "account.locked"), None);
        }
    }

    #[test]
    fn ids_are_named_by_their_end_where_nothing_else_names_them() {
        let id = crate::data::id_from_hex("019a2b3c4d5e708090a0b0c0d0e0f0ff").expect("id");
        assert_eq!(id_end(&id), "…d0e0f0ff");
        assert_eq!(
            handle(Some("Erika"), Some(0x1a2b)).as_deref(),
            Some("Erika#1a2b")
        );
        assert_eq!(handle(None, Some(0x1a2b)), None);
        assert_eq!(handle(Some("Erika"), None), None);
    }
}
