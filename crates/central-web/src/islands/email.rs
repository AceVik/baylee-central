//! An account's optional e-mail address (`central_api::email`) in the
//! account area's security section: naming one, confirming it with the code
//! Central mails to it, changing it and removing it. Whoever reads that
//! mailbox can reset the sign-in, so naming an address proves the current
//! password first: the form signs in once more as the password's change
//! does (`password::sign_in_again`), never with a name somebody typed.
//!
//! Central answers every request with the state after it, and the section
//! draws that state and nothing it guessed. `Rejected` alone says little
//! here (a wrong code, no address that waits, no mail, no session): the
//! page reads the state again, and only Central's refusal of
//! `AccountSummary` means that this browser is signed out.

use central_api::email::{EmailRequest, EmailState, MailCode, address};
use central_api::export::AccountExport;
use central_api::wire::{Request, Response};
use leptos::prelude::*;
use serde::Serialize;

use super::account::when;
use super::auth::{Problem, problem_text};
use super::central::{Failure, call};
use super::chat::glyph;
use super::crypto::Crypto;
use super::field::{self, Labelled};
use super::password::{Trouble, sign_in_again};
use super::{focus, session};
use crate::rich::rich;
use crate::words::{self, Lang};

/// What the page asks Central to do about the address.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Set,
    Resend,
    Confirm,
    Withdraw,
    Remove,
}

/// What the page says of an action Central did not carry out, or whose
/// answer did not arrive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Said {
    /// The code is not the one Central mailed.
    WrongCode,
    /// The address is the account's already.
    Same,
    /// No mail could be handed over, or too many were asked for.
    Mails,
    /// No answer arrived: the action may have happened all the same.
    Unsure,
    /// Anything else, in the forms' shared words: nothing was changed.
    Other(Problem),
}

fn said_text(lang: Lang, said: Said) -> &'static str {
    match said {
        Said::WrongCode => words::email_wrong_code(lang),
        Said::Same => words::email_same(lang),
        Said::Mails => words::email_busy(lang),
        Said::Unsure => words::email_unsure(lang),
        Said::Other(problem) => problem_text(lang, problem),
    }
}

/// What Central's refusal of `action` means. Only a refused code is a
/// wrong code, and an answer that did not arrive is never a refusal.
fn refused(action: Action, failure: Failure) -> Said {
    match (failure, action) {
        (Failure::Rejected, Action::Confirm) => Said::WrongCode,
        (Failure::Conflict, Action::Set) => Said::Same,
        (Failure::Busy, _) => Said::Mails,
        (Failure::Unavailable | Failure::Malformed | Failure::Network, _) => Said::Unsure,
        (Failure::Rejected | Failure::Forbidden | Failure::Conflict, _) => {
            Said::Other(Problem::Failed)
        }
    }
}

/// Whether the page reads the state again after `failure`: after a
/// refusal that may have changed what waits (some wrong codes give the
/// address up) or that may be a lost session, and after a lost answer.
fn rereads(failure: Failure) -> bool {
    !matches!(
        failure,
        Failure::Busy | Failure::Conflict | Failure::Forbidden
    )
}

/// Why the state could not be read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Unread {
    /// Central no longer knows this browser's session.
    SignedOut,
    /// Central knows the session and refuses the question: a Central that
    /// does not keep addresses (yet). Nobody is signed out for that.
    Refused,
    /// Anything else, in the forms' shared words.
    Problem(Problem),
}

/// Reads what Central holds. A refusal is a lost session only if Central
/// refuses `AccountSummary` too.
async fn read() -> Result<EmailState, Unread> {
    let request = Request::Email {
        request: EmailRequest::Status,
    };
    match call(&request).await {
        Ok(Response::Email { state }) => Ok(state),
        Ok(_) => Err(Unread::Problem(Problem::Failed)),
        Err(Failure::Rejected) => {
            if matches!(call(&Request::AccountSummary).await, Err(Failure::Rejected)) {
                Err(Unread::SignedOut)
            } else {
                Err(Unread::Refused)
            }
        }
        Err(failure) => Err(Unread::Problem(Problem::outside_sign_in(failure))),
    }
}

/// How an action ended.
enum Outcome {
    /// Central did it: what it holds now.
    Now(EmailState),
    /// This browser is not signed in any more.
    SignedOut,
    /// Not done, or not known to be: what to say, and what Central holds
    /// now where the page read it again.
    Said(Said, Option<EmailState>),
}

/// Sends `request` and reads what came of it.
async fn act(action: Action, request: EmailRequest) -> Outcome {
    let failure = match call(&Request::Email { request }).await {
        Ok(Response::Email { state }) => return Outcome::Now(state),
        // an answer of another kind says nothing of what happened
        Ok(_) => Failure::Malformed,
        Err(failure) => failure,
    };
    let said = refused(action, failure);
    if !rereads(failure) {
        return Outcome::Said(said, None);
    }
    match read().await {
        Ok(state) => Outcome::Said(said, Some(state)),
        Err(Unread::SignedOut) => Outcome::SignedOut,
        Err(Unread::Refused | Unread::Problem(_)) => Outcome::Said(said, None),
    }
}

/// Signs the session's account in once more with `current` in a Worker
/// that is ended at once: only the fresh sign-in is wanted, not its key.
async fn sign_again(worker: &str, current: &str) -> Result<(), Trouble> {
    let crypto = Crypto::start(worker).ok_or(Trouble::Other(Problem::Failed))?;
    let signed = sign_in_again(&crypto, current).await;
    crypto.close();
    signed.map(|_| ())
}

/// What the form lacks, before anything is sent: the address is checked
/// by the rule Central checks it with.
fn form_problem(lang: Lang, typed: &str, password: &str) -> Option<&'static str> {
    if typed.is_empty() || password.is_empty() {
        return Some(words::email_missing(lang));
    }
    if address(typed).is_err() {
        return Some(words::email_error_address(lang));
    }
    None
}

/// What the section holds of Central's state.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Held {
    Loading,
    /// Not read: why, or `None` where Central refused a signed-in account.
    Unread(Option<Problem>),
    Ready(EmailState),
}

/// What the person is in the middle of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Rest,
    /// Naming another address while the confirmed one stays.
    Changing,
    /// Asked whether the address is to be removed.
    Removing,
    /// Signing in once more, then naming the address.
    Working,
}

/// Which of its faces the section shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Face {
    /// This Central sends no mail: what is stored, and the ways to drop it.
    NoMail,
    /// The explanation and the form that names an address.
    Form,
    /// A code was mailed: the field for it.
    Pending,
    /// The confirmed address.
    Confirmed,
    /// The question before the address is removed.
    Removing,
    /// A sign-in once more is under way.
    Working,
}

/// The face for a state. An address that waits comes before everything
/// but the work under way; without mail there is no form and no code.
fn face(state: &EmailState, mode: Mode) -> Face {
    let confirmed = state.address.is_some();
    match mode {
        Mode::Working => Face::Working,
        Mode::Removing if confirmed => Face::Removing,
        _ if !state.mail => Face::NoMail,
        _ if state.pending.is_some() => Face::Pending,
        Mode::Rest if confirmed => Face::Confirmed,
        _ => Face::Form,
    }
}

/// The mode that still has its subject in `state`: a change or a removal
/// is of a confirmed address, and the work is over once Central answered.
fn settled(mode: Mode, state: &EmailState) -> Mode {
    match mode {
        Mode::Changing | Mode::Removing if state.address.is_some() => mode,
        _ => Mode::Rest,
    }
}

/// After Central carried `action` out: what the page says and what the
/// person is in the middle of next. A withdrawn address makes room for
/// another, so the form follows it where one can be named.
fn after(lang: Lang, action: Action, state: &EmailState) -> (Option<&'static str>, Mode) {
    match action {
        Action::Set => (None, Mode::Rest),
        Action::Resend => (Some(words::email_resent(lang)), Mode::Rest),
        Action::Confirm => (Some(words::email_now_confirmed(lang)), Mode::Rest),
        Action::Withdraw => (
            Some(words::email_withdrawn(lang)),
            settled(Mode::Changing, state),
        ),
        Action::Remove => (Some(words::email_removed(lang)), Mode::Rest),
    }
}

/// Where the keyboard goes on once a face was drawn anew: to the field
/// the person fills in next, else to the line that says what happened
/// (`ok`) or what did not.
fn focus_in(face: Face, ok: bool) -> &'static str {
    match (face, ok) {
        (Face::Pending, _) => "#email-code",
        (Face::Form, _) => "#email-address",
        (_, true) => "#email-said",
        (_, false) => "#email-error",
    }
}

/// Where it goes once Central carried `action` out. Whoever removed the
/// address is told so, and is not put into the field for a new one.
fn focus_after(action: Action, face: Face) -> &'static str {
    match action {
        Action::Remove => "#email-said",
        _ => focus_in(face, true),
    }
}

/// The export's file: Central's export as it is, and beside it what
/// Central holds of the e-mail address where the page could read it. Only
/// the account's own data: whether this Central sends mail is not in it.
#[derive(Serialize)]
struct ExportFile<'a> {
    #[serde(flatten)]
    account: &'a AccountExport,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<ExportedEmail<'a>>,
}

#[derive(Serialize)]
struct ExportedEmail<'a> {
    address: Option<&'a str>,
    confirmed_at: Option<i64>,
    pending: Option<&'a str>,
    pending_until: Option<i64>,
}

/// The export as the text of a JSON file, with the e-mail address's state
/// under `email` where `state` was read.
pub fn export_file(export: &AccountExport, state: Option<&EmailState>) -> Option<String> {
    let file = ExportFile {
        account: export,
        email: state.map(|state| ExportedEmail {
            address: state.address.as_deref(),
            confirmed_at: state.confirmed_at,
            pending: state.pending.as_deref(),
            pending_until: state.pending_until,
        }),
    };
    serde_json::to_string_pretty(&file).ok()
}

/// What Central holds of the address, for the export; `None` where it
/// could not be read (the export then says so beside its file).
pub async fn for_export() -> Option<EmailState> {
    read().await.ok()
}

/// The section's signals and what it was given. All of it is `Copy`, so
/// every handler takes the whole.
#[derive(Clone, Copy)]
struct Section {
    lang: Lang,
    worker: StoredValue<String>,
    username: StoredValue<String>,
    held: RwSignal<Held>,
    mode: RwSignal<Mode>,
    typed: RwSignal<String>,
    password: RwSignal<String>,
    code: RwSignal<String>,
    error: RwSignal<Option<&'static str>>,
    note: RwSignal<Option<&'static str>>,
    busy: RwSignal<bool>,
    signed_again: Callback<()>,
    signed_out: Callback<()>,
}

impl Section {
    /// Reads the state: when the section is first shown, and on "Try again".
    fn load(self) {
        self.held.set(Held::Loading);
        leptos::task::spawn_local(async move {
            match read().await {
                Ok(state) => self.held.set(Held::Ready(state)),
                Err(Unread::SignedOut) => self.leave(),
                Err(Unread::Refused) => self.held.set(Held::Unread(None)),
                Err(Unread::Problem(problem)) => self.held.set(Held::Unread(Some(problem))),
            }
        });
    }

    /// This browser is not signed in any more: as the password's change
    /// handles it, the area says so.
    fn leave(self) {
        session::forget();
        self.signed_out.run(());
    }

    /// Before an action: what the last one said is over.
    fn begin(self) {
        self.error.set(None);
        self.note.set(None);
    }

    /// The face as it stands, if a state is held. An answer may arrive
    /// after the area drew this section anew (a session that ended
    /// elsewhere): its signals are gone then, and there is no face.
    fn face_now(self) -> Option<Face> {
        let mode = self.mode.try_get_untracked()?;
        match self.held.try_get_untracked()? {
            Held::Ready(state) => Some(face(&state, mode)),
            _ => None,
        }
    }

    /// Sends one request about the address and shows what came of it.
    fn ask(self, action: Action, request: EmailRequest) {
        if self.busy.get_untracked() {
            return;
        }
        self.begin();
        self.busy.set(true);
        leptos::task::spawn_local(async move {
            let outcome = act(action, request).await;
            self.busy.set(false);
            self.settle(action, outcome);
        });
    }

    fn settle(self, action: Action, outcome: Outcome) {
        match outcome {
            Outcome::Now(state) => self.done(action, state),
            Outcome::SignedOut => self.leave(),
            Outcome::Said(said, state) => self.failed(action, said, state, self.face_now()),
        }
    }

    /// Central carried `action` out and holds `state` now.
    fn done(self, action: Action, state: EmailState) {
        let (note, mode) = after(self.lang, action, &state);
        // what was typed for a step that is over goes with it
        self.code.set(String::new());
        if action == Action::Confirm {
            self.typed.set(String::new());
        }
        let shown = face(&state, mode);
        self.note.set(note);
        self.mode.set(mode);
        self.held.set(Held::Ready(state));
        focus::after_render(focus_after(action, shown));
    }

    /// `action` was not carried out, or nobody knows: the page says so,
    /// and draws the state anew where it read another. `was` is the face
    /// the person acted in.
    fn failed(self, action: Action, said: Said, state: Option<EmailState>, was: Option<Face>) {
        // a section that is gone has nowhere to say it
        let (Some(mode), Some(held)) =
            (self.mode.try_get_untracked(), self.held.try_get_untracked())
        else {
            return;
        };
        self.error.set(Some(said_text(self.lang, said)));
        // a signal that is set draws its face anew, and a face drawn anew
        // has lost the focus: what did not change is left alone
        if let Some(state) = state {
            let still = settled(mode, &state);
            if still != mode {
                self.mode.set(still);
            }
            if held != Held::Ready(state.clone()) {
                self.held.set(Held::Ready(state));
            }
        }
        let now = self.face_now();
        // a code that was refused is typed again; otherwise a face that
        // stayed keeps the focus on the button the person pressed
        if let Some(now) = now
            && (Some(now) != was || action == Action::Confirm)
        {
            focus::after_render(focus_in(now, false));
        }
    }

    /// The form's submit: checks it, signs in once more, names the address.
    fn name(self, event: &leptos::ev::SubmitEvent) {
        event.prevent_default();
        let lang = self.lang;
        let typed = self.typed.get_untracked().trim().to_owned();
        let current = self.password.get_untracked();
        if let Some(problem) = form_problem(lang, &typed, &current) {
            self.error.set(Some(problem));
            return;
        }
        self.password.set(String::new());
        self.begin();
        let back = self.mode.get_untracked();
        self.mode.set(Mode::Working);
        focus::after_render("#email-working");
        let worker = self.worker.get_value();
        leptos::task::spawn_local(async move {
            if let Err(trouble) = sign_again(&worker, &current).await {
                drop(current);
                self.unsigned(back, trouble);
                return;
            }
            drop(current);
            // the session listed beside this section is a newer one now
            self.signed_again.run(());
            let request = EmailRequest::Set {
                address: typed,
                lang: lang.code().to_owned(),
            };
            match act(Action::Set, request).await {
                Outcome::Said(said, state) => {
                    // the form comes back as it was left, unless what
                    // Central holds now has another face
                    self.mode.set(back);
                    self.failed(Action::Set, said, state, Some(Face::Working));
                }
                outcome => self.settle(Action::Set, outcome),
            }
        });
    }

    /// Nobody was signed in again: nothing was sent about the address.
    fn unsigned(self, back: Mode, trouble: Trouble) {
        let (said, field) = match trouble {
            Trouble::SignedOut => return self.leave(),
            Trouble::Current => (words::password_error_current(self.lang), "#email-password"),
            Trouble::Unsure => (words::email_unsure(self.lang), "#email-address"),
            Trouble::Other(problem) => (problem_text(self.lang, problem), "#email-address"),
        };
        self.error.set(Some(said));
        self.mode.set(back);
        focus::after_render(field);
    }

    /// The code's submit: a text that is no code is not sent, and is no
    /// wrong code either.
    fn confirm(self, event: &leptos::ev::SubmitEvent) {
        event.prevent_default();
        match MailCode::parse(&self.code.get_untracked()) {
            Ok(code) => self.ask(Action::Confirm, EmailRequest::Confirm { code }),
            Err(_) => self.error.set(Some(words::email_error_code(self.lang))),
        }
    }

    /// Steps into or out of a change or a removal, and puts the keyboard
    /// where that step begins.
    fn step(self, mode: Mode, focus_on: &'static str) {
        self.begin();
        self.password.set(String::new());
        self.mode.set(mode);
        focus::after_render(focus_on);
    }

    fn alert(self) -> impl IntoView {
        view! { <p class="error" role="alert" id="email-error" tabindex="-1">{move || self.error.get()}</p> }
    }

    /// A button of an action: it does nothing while another is under way.
    fn button(
        self,
        class: &'static str,
        id: Option<&'static str>,
        label: &'static str,
        act: impl Fn(Section) + 'static,
    ) -> impl IntoView {
        view! {
            <button type="button" class=class id=id aria-disabled=move || self.busy.get().then_some("true")
                on:click=move |_| if !self.busy.get_untracked() { act(self) }>{label}</button>
        }
    }
}

/// The confirmed address and since when, and an address that waits where
/// no code can be asked of it.
fn facts(lang: Lang, state: &EmailState, waiting: bool) -> impl IntoView + use<> {
    let confirmed = state.address.clone().map(|address| {
        let since = state.confirmed_at.map(|at| {
            view! { <dt>{words::email_confirmed_at(lang)}</dt><dd>{when(lang, at)}</dd> }
        });
        view! { <dt>{words::email_current(lang)}</dt><dd translate="no">{address}</dd>{since} }
    });
    let waits = state.pending.clone().filter(|_| waiting).map(|address| {
        view! { <dt>{words::email_waiting(lang)}</dt><dd translate="no">{address}</dd> }
    });
    (confirmed.is_some() || waits.is_some())
        .then(|| view! { <dl class="facts">{confirmed}{waits}</dl> })
}

fn unread_view(s: Section, problem: Option<Problem>) -> AnyView {
    let lang = s.lang;
    let said = match problem {
        Some(problem) => {
            view! { <p class="error" role="alert">{problem_text(lang, problem)}</p> }.into_any()
        }
        None => view! { <p class="meta">{words::email_unavailable(lang)}</p> }.into_any(),
    };
    view! {
        {said}
        <p class="actions">
            <button type="button" class="button" on:click=move |_| s.load()>{words::error_again(lang)}</button>
        </p>
    }
    .into_any()
}

fn no_mail_view(s: Section, state: &EmailState) -> AnyView {
    let lang = s.lang;
    let remove = state.address.is_some().then(|| {
        s.button(
            "button danger",
            Some("email-remove"),
            words::email_remove(lang),
            |s| s.step(Mode::Removing, "#email-remove-ask"),
        )
    });
    let withdraw = state.pending.is_some().then(|| {
        s.button("button quiet", None, words::email_withdraw(lang), |s| {
            s.ask(Action::Withdraw, EmailRequest::Withdraw);
        })
    });
    let actions = (remove.is_some() || withdraw.is_some())
        .then(|| view! { <p class="actions">{remove}{withdraw}</p> });
    view! {
        <p>{words::email_no_mail(lang)}</p>
        {facts(lang, state, true)}
        {s.alert()}
        {actions}
    }
    .into_any()
}

fn form_view(s: Section, state: &EmailState) -> AnyView {
    let lang = s.lang;
    let changing = state.address.is_some();
    let stays = changing.then(|| view! { <p class="meta">{words::email_change_lead(lang)}</p> });
    let cancel = changing.then(|| {
        s.button(
            "button quiet",
            None,
            words::email_change_cancel(lang),
            |s| s.step(Mode::Rest, "#email-change"),
        )
    });
    let password = Labelled {
        id: "email-password",
        label: words::privacy_delete_password(lang),
        hint: Some(words::email_password_hint(lang)),
        mark: glyph::LOCK,
        auto: "current-password",
        max: "1024",
    };
    view! {
        <p>{words::email_lead(lang)}</p>
        <p class="notice">{words::email_warning(lang)}</p>
        {facts(lang, state, false)}
        {stays}
        <form class="form" method="post" novalidate=true on:submit=move |event| s.name(&event)>
            <div class="field">
                <label for="email-address">{words::email_title(lang)}</label>
                <div class="inset">
                    {field::mark(glyph::MAIL)}
                    <input id="email-address" type="email" inputmode="email" autocomplete="email" autocapitalize="none" spellcheck="false" maxlength="254"
                        on:input=move |ev| s.typed.set(event_target_value(&ev)) prop:value=move || s.typed.get() />
                </div>
            </div>
            // for a password manager: whose password this is (not the address's)
            <input type="text" autocomplete="username" readonly=true hidden=true value=s.username.get_value() />
            {field::labelled(lang, password, move || s.password.get(), move |value| s.password.set(value))}
            {s.alert()}
            <p class="actions">
                <button type="submit" class="button primary">{words::email_submit(lang)}</button>
                {cancel}
            </p>
        </form>
    }
    .into_any()
}

fn pending_view(s: Section, state: &EmailState) -> AnyView {
    let lang = s.lang;
    let waits = state.pending.clone().unwrap_or_default();
    let keeps = state.address.as_deref().map(|address| {
        view! { <p class="meta">{rich(words::email_pending_keeps(lang, address), |_| None)}</p> }
    });
    view! {
        <p>{rich(words::email_pending(lang, &waits), |_| None)}</p>
        {keeps}
        <form class="form" method="post" novalidate=true on:submit=move |event| s.confirm(&event)>
            <div class="field">
                <label for="email-code">{words::email_code(lang)}</label>
                <div class="inset">
                    {field::mark(glyph::KEY)}
                    <input id="email-code" class="code-entry" type="text" inputmode="text" autocomplete="one-time-code" autocapitalize="characters" spellcheck="false" maxlength="32" aria-describedby="email-code-hint"
                        on:input=move |ev| s.code.set(event_target_value(&ev)) prop:value=move || s.code.get() />
                </div>
                <p class="hint" id="email-code-hint">{words::email_code_hint(lang)}</p>
            </div>
            {s.alert()}
            <p class="actions">
                <button type="submit" class="button primary" aria-disabled=move || s.busy.get().then_some("true")>{words::email_confirm(lang)}</button>
                {s.button("button quiet", None, words::email_resend(lang), move |s| {
                    s.ask(Action::Resend, EmailRequest::Resend { lang: lang.code().to_owned() });
                })}
                {s.button("button quiet", None, words::email_other(lang), |s| {
                    s.ask(Action::Withdraw, EmailRequest::Withdraw);
                })}
            </p>
        </form>
    }
    .into_any()
}

fn confirmed_view(s: Section, state: &EmailState) -> AnyView {
    let lang = s.lang;
    view! {
        {facts(lang, state, false)}
        {s.alert()}
        <p class="actions">
            {s.button("button", Some("email-change"), words::email_change(lang), |s| {
                s.step(Mode::Changing, "#email-address");
            })}
            {s.button("button danger", Some("email-remove"), words::email_remove(lang), |s| {
                s.step(Mode::Removing, "#email-remove-ask");
            })}
        </p>
    }
    .into_any()
}

/// The question before the address goes. Where no mail is sent the
/// address cannot be told, so the question does not say that it is.
fn removing_view(s: Section, state: &EmailState) -> AnyView {
    let lang = s.lang;
    let ask = if state.mail {
        words::email_remove_ask(lang)
    } else {
        words::email_remove_ask_no_mail(lang)
    };
    view! {
        {facts(lang, state, true)}
        // the last word before the end, not the button that ends it
        <p class="notice" id="email-remove-ask" tabindex="-1">{ask}</p>
        {s.alert()}
        <p class="actions">
            {s.button("button danger", None, words::email_remove_confirm(lang), |s| {
                s.ask(Action::Remove, EmailRequest::Remove);
            })}
            {s.button("button quiet", None, words::email_remove_keep(lang), |s| {
                s.step(Mode::Rest, "#email-remove");
            })}
        </p>
    }
    .into_any()
}

fn draw(s: Section) -> AnyView {
    let lang = s.lang;
    match s.held.get() {
        Held::Loading => {
            view! { <p role="status" class="meta">{words::account_loading(lang)}</p> }.into_any()
        }
        Held::Unread(problem) => unread_view(s, problem),
        Held::Ready(state) => match face(&state, s.mode.get()) {
            Face::Working => view! {
                <p class="working" role="status" tabindex="-1" id="email-working">{words::signin_working(lang)}</p>
            }
            .into_any(),
            Face::NoMail => no_mail_view(s, &state),
            Face::Form => form_view(s, &state),
            Face::Pending => pending_view(s, &state),
            Face::Confirmed => confirmed_view(s, &state),
            Face::Removing => removing_view(s, &state),
        },
    }
}

/// The section in the account area's security page. `signed_again` runs
/// after the form signed in once more (the sessions listed beside it are
/// others now), `signed_out` when this browser turns out not to be signed
/// in.
pub fn section(
    lang: Lang,
    worker: String,
    username: String,
    signed_again: Callback<()>,
    signed_out: Callback<()>,
) -> impl IntoView {
    let s = Section {
        lang,
        worker: StoredValue::new(worker),
        username: StoredValue::new(username),
        held: RwSignal::new(Held::Loading),
        mode: RwSignal::new(Mode::Rest),
        typed: RwSignal::new(String::new()),
        password: RwSignal::new(String::new()),
        code: RwSignal::new(String::new()),
        error: RwSignal::new(None),
        note: RwSignal::new(None),
        busy: RwSignal::new(false),
        signed_again,
        signed_out,
    };
    // Effects run in the browser only.
    Effect::new(move |_| s.load());
    view! {
        <section class="card" aria-labelledby="email-title">
            <h2 id="email-title">{words::email_title(lang)}</h2>
            <p class="meta" role="status" id="email-said" tabindex="-1">{move || s.note.get()}</p>
            {move || draw(s)}
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAILURES: [Failure; 7] = [
        Failure::Rejected,
        Failure::Forbidden,
        Failure::Conflict,
        Failure::Busy,
        Failure::Unavailable,
        Failure::Malformed,
        Failure::Network,
    ];
    const ACTIONS: [Action; 5] = [
        Action::Set,
        Action::Resend,
        Action::Confirm,
        Action::Withdraw,
        Action::Remove,
    ];

    fn state(mail: bool, address: Option<&str>, pending: Option<&str>) -> EmailState {
        EmailState {
            mail,
            address: address.map(str::to_owned),
            confirmed_at: address.map(|_| 1_794_000_000),
            pending: pending.map(str::to_owned),
            pending_until: pending.map(|_| 1_794_000_900),
        }
    }

    /// The page refuses what Central would refuse, by Central's own rule,
    /// and sends nothing without the password.
    #[test]
    fn the_form_checks_the_address_as_central_does() {
        let lang = Lang::En;
        assert_eq!(form_problem(lang, "player@example.org", "the words"), None);
        assert_eq!(
            form_problem(lang, "first.last+tag@mail.example.co.uk", "x"),
            None
        );
        for (typed, password) in [("", "the words"), ("player@example.org", ""), ("", "")] {
            assert_eq!(
                form_problem(lang, typed, password),
                Some(words::email_missing(lang))
            );
        }
        for bad in [
            "plain",
            "player@localhost",
            "Name <player@example.org>",
            "üser@example.org",
            "player@exämple.org",
            "a b@example.org",
            "player@example.org\r\nBcc: other@example.org",
            " player@example.org",
        ] {
            assert_eq!(
                form_problem(lang, bad, "the words"),
                Some(words::email_error_address(lang)),
                "{bad}"
            );
            assert!(address(bad).is_err(), "{bad}");
        }
    }

    /// The code field reads what a person types as Central reads it, and
    /// what travels is the printed form whatever was typed.
    #[test]
    fn a_typed_code_travels_in_its_printed_form() {
        for typed in [
            "000G-40R4",
            "000g40r4",
            " ooog 4or4 ",
            "OOOG-4OR4",
            "0 0 0 g - 4 0 r 4",
        ] {
            let code = MailCode::parse(typed).expect("a code");
            let request = Request::Email {
                request: EmailRequest::Confirm { code },
            };
            assert_eq!(
                serde_json::to_string(&request).expect("json"),
                r#"{"operation":"email","request":{"action":"confirm","code":"000G-40R4"}}"#,
                "{typed}"
            );
        }
        for bad in [
            "",
            "000G-40R",
            "000G-40R4-0",
            "000G-40RU",
            "000G_40R4",
            "RESET-000G-40R4",
        ] {
            assert!(MailCode::parse(bad).is_err(), "{bad}");
        }
    }

    /// The requests as Central reads them and its answer as the page
    /// reads it: what the browser tests' stand-in speaks.
    #[test]
    fn the_requests_and_the_answer_are_central_s() {
        let json = |request: EmailRequest| {
            serde_json::to_string(&Request::Email { request }).expect("json")
        };
        assert_eq!(
            json(EmailRequest::Status),
            r#"{"operation":"email","request":{"action":"status"}}"#
        );
        assert_eq!(
            json(EmailRequest::Set {
                address: "player@example.org".into(),
                lang: Lang::PtBr.code().into(),
            }),
            r#"{"operation":"email","request":{"action":"set","address":"player@example.org","lang":"pt-br"}}"#
        );
        assert_eq!(
            json(EmailRequest::Resend { lang: "de".into() }),
            r#"{"operation":"email","request":{"action":"resend","lang":"de"}}"#
        );
        assert_eq!(
            json(EmailRequest::Withdraw),
            r#"{"operation":"email","request":{"action":"withdraw"}}"#
        );
        assert_eq!(
            json(EmailRequest::Remove),
            r#"{"operation":"email","request":{"action":"remove"}}"#
        );
        let answer = r#"{"result":"email","state":{"mail":true,"address":null,"confirmed_at":null,"pending":"player@example.org","pending_until":1794000900}}"#;
        let Ok(Response::Email { state: read }) = serde_json::from_str::<Response>(answer) else {
            panic!("an e-mail answer");
        };
        assert_eq!(read, state(true, None, Some("player@example.org")));
        let extra = r#"{"result":"email","state":{"mail":true,"address":null,"confirmed_at":null,"pending":null,"pending_until":null,"code":"000G-40R4"}}"#;
        assert!(serde_json::from_str::<Response>(extra).is_err());
    }

    /// A mail is asked for in the page's language, and every language's
    /// code is a tag Central takes.
    #[test]
    fn every_language_is_a_tag_central_takes() {
        for lang in Lang::ALL {
            assert!(
                central_api::email::language(lang.code()).is_ok(),
                "{lang:?}"
            );
        }
    }

    /// Only a refused confirmation is a wrong code, only a refused naming
    /// "yours already", and nothing here reads as a wrong password.
    #[test]
    fn a_refusal_means_what_its_action_asked() {
        for action in ACTIONS {
            for failure in FAILURES {
                let said = refused(action, failure);
                assert_eq!(
                    said == Said::WrongCode,
                    action == Action::Confirm && failure == Failure::Rejected,
                    "{action:?} {failure:?}"
                );
                assert_eq!(
                    said == Said::Same,
                    action == Action::Set && failure == Failure::Conflict,
                    "{action:?} {failure:?}"
                );
                assert_eq!(said == Said::Mails, failure == Failure::Busy);
                assert_ne!(said, Said::Other(Problem::WrongLogin));
                assert_ne!(said, Said::Other(Problem::NotMoved));
            }
        }
        for lang in Lang::ALL {
            assert_ne!(said_text(lang, Said::WrongCode), words::signin_wrong(lang));
        }
    }

    /// An answer that did not arrive is never called a refusal, and the
    /// state is read again after it; so it is after a refusal that may
    /// have given the waiting address up or that may be a lost session.
    #[test]
    fn a_lost_answer_is_unsure_and_the_state_is_read_again() {
        for action in ACTIONS {
            for lost in [Failure::Network, Failure::Unavailable, Failure::Malformed] {
                assert_eq!(refused(action, lost), Said::Unsure);
                assert!(rereads(lost));
            }
        }
        assert!(rereads(Failure::Rejected));
        for kept in [Failure::Busy, Failure::Conflict, Failure::Forbidden] {
            assert!(!rereads(kept));
        }
    }

    /// Every state has one face. Without mail there is never a form or a
    /// code field; an address that waits is shown before a confirmed one.
    #[test]
    fn every_state_has_its_face() {
        let (a, b) = (Some("old@example.org"), Some("new@example.org"));
        let cases = [
            (state(true, None, None), Mode::Rest, Face::Form),
            (state(true, None, b), Mode::Rest, Face::Pending),
            (state(true, a, None), Mode::Rest, Face::Confirmed),
            (state(true, a, None), Mode::Changing, Face::Form),
            (state(true, a, None), Mode::Removing, Face::Removing),
            (state(true, a, b), Mode::Rest, Face::Pending),
            (state(true, a, b), Mode::Changing, Face::Pending),
            (state(true, None, None), Mode::Working, Face::Working),
            (state(false, None, None), Mode::Rest, Face::NoMail),
            (state(false, a, None), Mode::Rest, Face::NoMail),
            (state(false, a, None), Mode::Removing, Face::Removing),
            (state(false, None, b), Mode::Rest, Face::NoMail),
            (state(false, a, b), Mode::Changing, Face::NoMail),
            // nothing confirmed is there to remove or to change
            (state(true, None, None), Mode::Removing, Face::Form),
            (state(false, None, None), Mode::Removing, Face::NoMail),
        ];
        for (held, mode, want) in cases {
            assert_eq!(face(&held, mode), want, "{held:?} {mode:?}");
        }
        for mail in [false, true] {
            for mode in [Mode::Rest, Mode::Changing, Mode::Removing] {
                for (address, pending) in [(None, None), (a, None), (None, b), (a, b)] {
                    let shown = face(&state(mail, address, pending), mode);
                    if !mail {
                        assert!(matches!(shown, Face::NoMail | Face::Removing));
                    }
                }
            }
        }
    }

    /// A mode whose subject is gone does not outlive it.
    #[test]
    fn a_change_or_a_removal_needs_a_confirmed_address() {
        let (with, without) = (
            state(true, Some("old@example.org"), None),
            state(true, None, None),
        );
        for mode in [Mode::Changing, Mode::Removing] {
            assert_eq!(settled(mode, &with), mode);
            assert_eq!(settled(mode, &without), Mode::Rest);
        }
        assert_eq!(settled(Mode::Working, &with), Mode::Rest);
        assert_eq!(settled(Mode::Rest, &with), Mode::Rest);
    }

    /// After each step the page says what happened, and the keyboard goes
    /// on where the person does: at the code after a mail, at the form
    /// after a withdrawal, at what was said otherwise.
    #[test]
    fn each_step_says_what_happened_and_moves_the_keyboard() {
        let lang = Lang::En;
        let (a, b) = (Some("old@example.org"), Some("new@example.org"));
        let step = |action: Action, held: EmailState| {
            let (note, mode) = after(lang, action, &held);
            (note, focus_after(action, face(&held, mode)))
        };
        assert_eq!(
            step(Action::Set, state(true, None, b)),
            (None, "#email-code")
        );
        assert_eq!(
            step(Action::Resend, state(true, a, b)),
            (Some(words::email_resent(lang)), "#email-code")
        );
        assert_eq!(
            step(Action::Confirm, state(true, b, None)),
            (Some(words::email_now_confirmed(lang)), "#email-said")
        );
        // another address can be named at once, beside a confirmed one too
        for held in [state(true, None, None), state(true, a, None)] {
            assert_eq!(
                step(Action::Withdraw, held),
                (Some(words::email_withdrawn(lang)), "#email-address")
            );
        }
        assert_eq!(
            step(Action::Withdraw, state(false, a, None)),
            (Some(words::email_withdrawn(lang)), "#email-said")
        );
        // the form follows a removal, and the keyboard stays with what was said
        for mail in [false, true] {
            assert_eq!(
                step(Action::Remove, state(mail, None, None)),
                (Some(words::email_removed(lang)), "#email-said")
            );
        }
        assert_eq!(focus_in(Face::Confirmed, false), "#email-error");
        assert_eq!(focus_in(Face::Pending, false), "#email-code");
    }

    fn export() -> AccountExport {
        AccountExport {
            account: [7; 16],
            username: "erika".into(),
            display_name: Some("Erika".into()),
            display_name_at: Some(1),
            tag: Some(0x1a2b),
            source: central_api::wire::AccountSource::Registered,
            legacy_id: None,
            created_at: 1,
            last_sign_in_at: Some(2),
            vault_epoch: 1,
            document_revision: 1,
            state: central_api::admin::AccountState::Enrolled,
            admin: false,
            recovery: true,
            reset_until: None,
            objects: 0,
            object_bytes: 0,
            archives: 0,
            sessions: Vec::new(),
            terms: Vec::new(),
            gateways: Vec::new(),
            administers: Vec::new(),
            audit: Vec::new(),
            complete: true,
        }
    }

    /// The file is Central's export, field for field and in its order,
    /// with the address's state beside it; where the state was not read,
    /// the file is the export alone.
    #[test]
    fn the_export_s_file_holds_the_address_beside_the_export() {
        let export = export();
        let alone = export_file(&export, None).expect("a file");
        assert_eq!(
            alone,
            serde_json::to_string_pretty(&export).expect("json"),
            "without a state the file is the export as before"
        );
        assert!(serde_json::from_str::<AccountExport>(&alone).is_ok());
        let held = state(false, Some("old@example.org"), Some("new@example.org"));
        let with = export_file(&export, Some(&held)).expect("a file");
        let rest = with
            .strip_prefix(alone.strip_suffix("\n}").expect("an object"))
            .expect("the export's own fields come first, unchanged");
        let email: serde_json::Value =
            serde_json::from_str(&format!("{{{}", rest.trim_start_matches(','))).expect("json");
        assert_eq!(
            email,
            serde_json::json!({ "email": {
                "address": "old@example.org", "confirmed_at": 1_794_000_000,
                "pending": "new@example.org", "pending_until": 1_794_000_900,
            } })
        );
        // an account without an address says so, rather than nothing
        let none = export_file(&export, Some(&state(true, None, None))).expect("a file");
        let none: serde_json::Value = serde_json::from_str(&none).expect("json");
        assert_eq!(
            none["email"],
            serde_json::json!({ "address": null, "confirmed_at": null, "pending": null, "pending_until": null })
        );
        assert_eq!(none["username"], "erika");
    }
}
