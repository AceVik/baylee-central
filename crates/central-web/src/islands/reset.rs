//! Setting a new password with a reset code, one an administrator made or
//! one Central mailed (`docs/account-reset.md`): the account's data kept
//! with the recovery code, or a new, empty vault after the warning. The
//! password and the recovery code go only to the crypto Worker; Central
//! sees the reset code and OPAQUE messages. Everything the form asks is in
//! hand before the first request: Central's attempt lives 60 seconds.
//!
//! Before the form stands the block for someone who has no code yet: it
//! asks Central to mail one to the account's confirmed address
//! (`Request::ResetMail`). Central answers the same whatever the name, and
//! the block says no more than that.
//!
//! The reset mail's button links here with its code in the fragment
//! (`#code=RESET-…`, `fragment`): the form starts with it in the code's
//! field, the fragment is gone from the address bar, and nothing is sent
//! until the person sends the form.

use central_api::Token;
use central_api::reset_code::ResetCode;
use central_api::wire::{Discovery, Identity, Request, ResetVault, Response};
use leptos::prelude::*;
use serde_json::{Value, json};

use super::auth::{Problem, problem_text};
use super::central::{Failure, call, uuid_v7, valid_username};
use super::chat::glyph;
use super::crypto::{Crypto, field_bytes};
use super::field::{self, Labelled};
use super::{clip, early, fragment, session};
use crate::words::{self, Lang};

/// What the form holds when it is sent.
#[derive(Clone, Default)]
struct Fields {
    username: String,
    code: String,
    /// Whether the data is kept with the recovery code; else a new vault.
    keep: bool,
    recovery: String,
    /// Whether the warning before a new vault was confirmed.
    confirmed: bool,
    password: String,
    repeat: String,
}

/// What the form lacks, before anything is sent anywhere. A new vault is
/// never asked for without the warning's confirmation.
fn form_problem(lang: Lang, f: &Fields) -> Option<&'static str> {
    if f.username.trim().is_empty() || f.code.trim().is_empty() || f.password.is_empty() {
        return Some(words::reset_missing(lang));
    }
    if ResetCode::parse(&f.code).is_err() {
        return Some(words::reset_error_code(lang));
    }
    if f.keep && f.recovery.trim().is_empty() {
        return Some(words::reset_recovery_missing(lang));
    }
    if !f.keep && !f.confirmed {
        return Some(words::reset_error_confirm(lang));
    }
    if f.password.chars().count() < 8 {
        return Some(words::register_error_short(lang));
    }
    if f.password.len() > 1024 {
        return Some(words::register_error_long(lang));
    }
    if f.password != f.repeat {
        return Some(words::register_error_repeat(lang));
    }
    None
}

/// Why a reset did not happen. Nothing was changed in any of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Refusal {
    /// The name and the code do not go together: a name Central does not
    /// know, an account still at the official gateway, a wrong, expired or
    /// used code. One text for all of them; the page tells them apart no
    /// further than that, whatever Central's answers would allow.
    Refused,
    /// The recovery code does not open the account's data.
    Recovery,
    /// Anything else, in the forms' shared words.
    Other(Problem),
}

fn refusal_text(lang: Lang, refusal: Refusal) -> &'static str {
    match refusal {
        Refusal::Refused => words::reset_refused(lang),
        Refusal::Recovery => words::reset_error_recovery(lang),
        Refusal::Other(problem) => problem_text(lang, problem),
    }
}

/// A failure that says nothing about the name and the code.
fn other(failure: Failure) -> Refusal {
    Refusal::Other(Problem::outside_sign_in(failure))
}

/// Central's refusal of a name, or of a name with its code.
fn by_central(failure: Failure) -> Refusal {
    match failure {
        Failure::Rejected => Refusal::Refused,
        failure => other(failure),
    }
}

/// The Worker's refusal: where a recovery code was given, it is the code
/// that does not fit (its form at the start, its key at the finish).
fn by_worker(failure: Failure, keep: bool) -> Refusal {
    match failure {
        Failure::Rejected if keep => Refusal::Recovery,
        _ => Refusal::Other(Problem::Failed),
    }
}

/// What a refused finish means; `None` when its answer is lost and the
/// reset may have happened all the same.
fn at_finish(failure: Failure) -> Option<Refusal> {
    match failure {
        // the attempt ran out, or another finish spent the code first
        Failure::Rejected | Failure::Forbidden | Failure::Conflict => {
            Some(Refusal::Other(Problem::Failed))
        }
        Failure::Busy => Some(Refusal::Other(Problem::Busy)),
        Failure::Unavailable | Failure::Malformed | Failure::Network => None,
    }
}

/// The sign-in name a code is asked for by mail, as Central keeps names;
/// `None` for a text that is none. Only a name goes to Central: its
/// refusal there means that it sends no mail, never that the text was
/// something else (an e-mail address, say).
fn mail_name(typed: &str) -> Option<String> {
    let name = typed.trim().to_ascii_lowercase();
    valid_username(&name).then_some(name)
}

/// What came of asking for a reset code by mail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mailed {
    /// Central took the request. Whether a code went out it does not say:
    /// the answer is the same for a name without an account or without a
    /// confirmed address.
    Asked,
    /// This Central sends no mail.
    NoMail,
    /// Too many were asked for.
    Busy,
    /// Anything else, in the forms' shared words.
    Other(Problem),
}

fn mailed(answer: &Result<Response, Failure>) -> Mailed {
    match answer {
        Ok(Response::Done) => Mailed::Asked,
        Ok(_) => Mailed::Other(Problem::Failed),
        Err(Failure::Rejected) => Mailed::NoMail,
        Err(Failure::Busy) => Mailed::Busy,
        Err(failure) => Mailed::Other(Problem::outside_sign_in(*failure)),
    }
}

fn mailed_text(lang: Lang, mailed: Mailed) -> &'static str {
    match mailed {
        Mailed::Asked => words::reset_mail_sent(lang),
        Mailed::NoMail => words::reset_mail_none(lang),
        Mailed::Busy => words::reset_mail_busy(lang),
        Mailed::Other(problem) => problem_text(lang, problem),
    }
}

/// How a reset ended once its finish was sent.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Outcome {
    /// Whether Central said that it took; a lost answer leaves it open.
    sure: bool,
    /// A new vault's recovery code, to show once.
    code: Option<String>,
}

/// The account behind `username`.
async fn discover(username: &str) -> Result<Identity, Refusal> {
    let request = Request::Discover {
        username: username.to_owned(),
    };
    match call(&request).await.map_err(by_central)? {
        Response::Discovery {
            discovery: Discovery::Central { identity },
        } if uuid_v7(&identity.account) => Ok(identity),
        // an account that has not moved has no reset here
        Response::Discovery {
            discovery: Discovery::Legacy,
        } => Err(Refusal::Refused),
        _ => Err(Refusal::Other(Problem::Failed)),
    }
}

/// The finish that keeps the vault: the Worker opens the master key with
/// the recovery code and wraps it under the new password. A code that does
/// not open it fails here, before the reset code is spent.
async fn keep_request(
    crypto: &Crypto,
    attempt: Token,
    response: Vec<u8>,
    recovery: Option<Vec<u8>>,
) -> Result<Request, Refusal> {
    // an account without a recovery wrapper has no code that opens it
    let wrapper = recovery.ok_or(Refusal::Recovery)?;
    let done = crypto
        .ask(
            "reset_keep",
            json!({ "response": response, "recovery": wrapper }),
        )
        .await
        .map_err(|failure| by_worker(failure, true))?;
    Ok(Request::ResetFinish {
        attempt,
        record: field_bytes(&done, "record", None).map_err(other)?,
        wrapped_master: field_bytes(&done, "wrapped_master", Some(73)).map_err(other)?,
        vault: ResetVault::Keep,
    })
}

/// The finish that makes a new vault, and its new recovery code.
async fn new_request(
    crypto: &Crypto,
    attempt: Token,
    response: Vec<u8>,
) -> Result<(Request, String), Refusal> {
    let made = crypto
        .ask("reset_new", json!({ "response": response }))
        .await
        .map_err(|failure| by_worker(failure, false))?;
    let code = made
        .get("recovery_code")
        .and_then(Value::as_str)
        .ok_or(Refusal::Other(Problem::Failed))?
        .to_owned();
    let request = Request::ResetFinish {
        attempt,
        record: field_bytes(&made, "record", None).map_err(other)?,
        wrapped_master: field_bytes(&made, "wrapped_master", Some(73)).map_err(other)?,
        vault: ResetVault::New {
            document: field_bytes(&made, "document", None).map_err(other)?,
            recovery: Some(field_bytes(&made, "recovery", Some(73)).map_err(other)?),
        },
    };
    Ok((request, code))
}

/// Redeems the code in a fresh Worker: discovery, the OPAQUE registration
/// for the new password, then the finish that keeps the vault or makes a
/// new one.
///
/// # Errors
/// Why nothing was changed.
async fn reset(worker: &str, f: Fields) -> Result<Outcome, Refusal> {
    let username = f.username.trim().to_ascii_lowercase();
    let code = ResetCode::parse(&f.code).map_err(|_| Refusal::Refused)?;
    if !valid_username(&username) {
        return Err(Refusal::Refused);
    }
    let crypto = Crypto::start(worker).ok_or(Refusal::Other(Problem::Failed))?;
    let discovered = discover(&username).await?;
    // The Worker reads the recovery code here: one that is none stops the
    // reset before the reset code goes anywhere.
    let start = crypto
        .ask(
            "reset_start",
            json!({
                "password": f.password,
                "account": discovered.account,
                "epoch": discovered.epoch,
                "recovery_code": f.keep.then_some(f.recovery),
            }),
        )
        .await
        .map_err(|failure| by_worker(failure, f.keep))?;
    let registration = field_bytes(&start, "message", Some(32)).map_err(other)?;
    let Response::ResetChallenge {
        attempt,
        identity,
        registration: response,
        recovery,
    } = call(&Request::ResetStart {
        username,
        code,
        registration,
    })
    .await
    .map_err(by_central)?
    else {
        return Err(Refusal::Other(Problem::Failed));
    };
    // the account Central resets is the one the password was bound to
    if identity.account != discovered.account || identity.epoch != discovered.epoch {
        return Err(Refusal::Other(Problem::Failed));
    }
    let (finish, code) = if f.keep {
        let request = keep_request(&crypto, attempt, response, recovery).await?;
        (request, None)
    } else {
        let (request, code) = new_request(&crypto, attempt, response).await?;
        (request, Some(code))
    };
    crypto.close();
    let sure = match call(&finish).await {
        Ok(Response::Done) => true,
        Ok(_) => false,
        Err(failure) => match at_finish(failure) {
            Some(refusal) => return Err(refusal),
            None => false,
        },
    };
    Ok(Outcome { sure, code })
}

/// A reset ends every session of the account. Where this browser remembers
/// one that Central no longer knows, it forgets it, with the chat identity
/// it kept for it.
async fn forget_ended(worker: &str) {
    if session::remembered().is_none() {
        return;
    }
    if matches!(call(&Request::AccountSummary).await, Err(Failure::Rejected)) {
        session::forget();
        if let Some(crypto) = Crypto::start(worker) {
            let _ = crypto.ask("forget", json!({})).await;
        }
    }
}

/// The form's state. What it holds stays while the person corrects it;
/// the passwords go when it is sent.
#[derive(Clone, Copy)]
struct Form {
    username: RwSignal<String>,
    code: RwSignal<String>,
    keep: RwSignal<bool>,
    recovery: RwSignal<String>,
    confirmed: RwSignal<bool>,
    password: RwSignal<String>,
    repeat: RwSignal<String>,
    error: RwSignal<Option<&'static str>>,
}

impl Form {
    /// The form as it stands when the island starts: what was typed or
    /// chosen before it ran stays (`early`). The data is kept unless the
    /// person says otherwise. A reset code the mail's link handed in the
    /// address's fragment fills an empty code field; the fragment leaves
    /// the address bar either way.
    fn early() -> Self {
        let handed = fragment::code(&fragment::take(), ResetCode::parse);
        let typed = early::typed("reset-code");
        let code = match handed {
            Some(code) if typed.is_empty() => code.display().to_string(),
            _ => typed,
        };
        Self {
            username: RwSignal::new(early::typed("reset-name")),
            code: RwSignal::new(code),
            keep: RwSignal::new(!early::ticked("reset-new")),
            recovery: RwSignal::new(early::typed("reset-recovery")),
            confirmed: RwSignal::new(early::ticked("reset-confirm")),
            password: RwSignal::new(early::typed("reset-password")),
            repeat: RwSignal::new(early::typed("reset-repeat")),
            error: RwSignal::new(None),
        }
    }

    fn fields(self) -> Fields {
        Fields {
            username: self.username.get_untracked(),
            code: self.code.get_untracked(),
            keep: self.keep.get_untracked(),
            recovery: self.recovery.get_untracked(),
            confirmed: self.confirmed.get_untracked(),
            password: self.password.get_untracked(),
            repeat: self.repeat.get_untracked(),
        }
    }

    /// Lets go of the passwords; of everything else too once the reset is
    /// through.
    fn clear(self, all: bool) {
        self.password.set(String::new());
        self.repeat.set(String::new());
        if all {
            self.username.set(String::new());
            self.code.set(String::new());
            self.recovery.set(String::new());
            self.confirmed.set(false);
        }
    }
}

/// The block that asks for a code by mail. Its name is its own field; a
/// name it was sent with is carried into the form, where the code that
/// arrives is redeemed.
#[derive(Clone, Copy)]
struct Mail {
    name: RwSignal<String>,
    error: RwSignal<Option<&'static str>>,
    said: RwSignal<Option<&'static str>>,
    busy: RwSignal<bool>,
}

impl Mail {
    /// As [`Form::early`]: a name typed before the island ran stays.
    fn early() -> Self {
        Self {
            name: RwSignal::new(early::typed("reset-mail-name")),
            error: RwSignal::new(None),
            said: RwSignal::new(None),
            busy: RwSignal::new(false),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Step {
    Form,
    Working,
    Done(Outcome),
}

/// The block before the form, for someone who has no reset code: a
/// sign-in name, and the request that Central mails a code.
fn mail_view(
    lang: Lang,
    mail: Mail,
    submit: impl Fn(leptos::ev::SubmitEvent) + 'static,
) -> impl IntoView {
    let name = Labelled {
        id: "reset-mail-name",
        label: words::signin_name(lang),
        hint: Some(words::reset_mail_name_hint(lang)),
        mark: glyph::USER,
        auto: "username",
        max: "64",
    };
    view! {
        <h2 id="reset-mail-title">{words::reset_mail_title(lang)}</h2>
        <p>{words::reset_mail_lead(lang)}</p>
        <form class="form" method="post" novalidate=true on:submit=submit>
            {field::labelled(lang, name, move || mail.name.get(), move |value| mail.name.set(value))}
            <p class="error" role="alert">{move || mail.error.get()}</p>
            <button type="submit" class="button quiet" aria-disabled=move || mail.busy.get().then_some("true")>
                {glyph::glyph(glyph::MAIL)}
                <span>{words::reset_mail_submit(lang)}</span>
            </button>
            <p class="meta" role="status">{move || mail.said.get()}</p>
        </form>
    }
}

/// What happens to the data: kept with the recovery code, or a new vault.
/// The second choice is described by the warning, which is in the page
/// before it is chosen. The two share a name, which makes them one group;
/// they are the form's only named fields, and the name says nothing typed.
fn choice(lang: Lang, keep: RwSignal<bool>) -> impl IntoView {
    view! {
        <fieldset class="choice">
            <legend>{words::reset_data(lang)}</legend>
            <div class="check">
                <input id="reset-keep" type="radio" name="reset-data" value="keep" checked=true
                    prop:checked=move || keep.get() on:change=move |_| keep.set(true) />
                <label for="reset-keep">{words::reset_keep(lang)}</label>
            </div>
            <div class="check">
                <input id="reset-new" type="radio" name="reset-data" value="new" aria-describedby="reset-warning"
                    prop:checked=move || !keep.get() on:change=move |_| keep.set(false) />
                <label for="reset-new">{words::reset_new(lang)}</label>
            </div>
        </fieldset>
    }
}

fn form_view(
    lang: Lang,
    form: Form,
    submit: impl Fn(leptos::ev::SubmitEvent) + 'static,
) -> impl IntoView {
    let Form {
        keep,
        confirmed,
        error,
        ..
    } = form;
    let text = move |what: Labelled, held: RwSignal<String>| {
        field::labelled(lang, what, move || held.get(), move |value| held.set(value))
    };
    let spec = |id, label, hint, mark, auto, max| Labelled {
        id,
        label,
        hint,
        mark,
        auto,
        max,
    };
    view! {
        <form class="form" method="post" novalidate=true on:submit=submit>
            {text(spec("reset-name", words::signin_name(lang), None, glyph::USER, "username", "64"), form.username)}
            {text(spec("reset-code", words::reset_code(lang), Some(words::reset_code_hint(lang)), glyph::KEY, "off", "128"), form.code)}
            {choice(lang, keep)}
            <div class="form-part" hidden=move || !keep.get()>
                {text(spec("reset-recovery", words::reset_recovery(lang), Some(words::reset_recovery_hint(lang)), glyph::SHIELD_CHECK, "off", "128"), form.recovery)}
            </div>
            <div class="form-part" hidden=move || keep.get()>
                <p class="notice" id="reset-warning">{words::reset_warning(lang)}</p>
                <div class="check">
                    <input id="reset-confirm" type="checkbox"
                        prop:checked=move || confirmed.get() on:change=move |ev| confirmed.set(event_target_checked(&ev)) />
                    <label for="reset-confirm">{words::reset_warning_confirm(lang)}</label>
                </div>
            </div>
            {text(spec("reset-password", words::password_new(lang), Some(words::register_password_hint(lang)), glyph::LOCK, "new-password", "1024"), form.password)}
            {text(spec("reset-repeat", words::password_new_repeat(lang), Some(words::register_repeat_hint(lang)), glyph::LOCK_KEYHOLE, "new-password", "1024"), form.repeat)}
            <p class="error" role="alert">{move || error.get()}</p>
            <button type="submit" class="button primary">{words::reset_submit(lang)}</button>
        </form>
    }
}

/// A new vault's recovery code: shown once, to copy or save, and the way
/// on only after the person says it is saved, as after a registration.
fn code_view(lang: Lang, code: String, sure: bool, signin: String) -> impl IntoView {
    let kept = RwSignal::new(false);
    let error = RwSignal::new(None::<&'static str>);
    let download = format!("data:text/plain;charset=utf-8,Baylee%20recovery%20code%3A%20{code}%0A");
    let (copy, copied) = clip::copier(lang, code.clone());
    let go_on = move |_| {
        if kept.get_untracked() {
            session::go(&signin);
        } else {
            error.set(Some(words::code_not_kept(lang)));
        }
    };
    // after a lost answer the code may be worth nothing: said before it
    let unsure = (!sure).then(|| view! { <p class="notice">{words::reset_unsure_code(lang)}</p> });
    view! {
        {unsure}
        <p class="eyebrow">{words::code_shown_once(lang)}</p>
        <p class="secret" translate="no"><code>{code}</code></p>
        <p class="actions">
            {copy}
            <a class="button" href=download download="baylee-recovery-code.txt">{words::code_download(lang)}</a>
        </p>
        {copied}
        <div class="check">
            <input id="reset-code-kept" type="checkbox" on:change=move |ev| kept.set(event_target_checked(&ev)) />
            <label for="reset-code-kept">{words::code_kept(lang)}</label>
        </div>
        <p class="error" role="alert">{move || error.get()}</p>
        <p class="actions">
            <button type="button" class="button primary" on:click=go_on>{words::reset_to_signin(lang)}</button>
        </p>
    }
}

/// The result: what happened to the data and the sessions, a new vault's
/// code, and the way to the sign-in.
fn done_view(lang: Lang, outcome: Outcome) -> impl IntoView {
    let Outcome { sure, code } = outcome;
    let (title, text) = match (sure, code.is_some()) {
        (true, false) => (words::reset_done_title(lang), words::reset_done_kept(lang)),
        (true, true) => (words::reset_done_title(lang), words::reset_done_new(lang)),
        (false, _) => (words::reset_unsure_title(lang), words::reset_unsure(lang)),
    };
    let signin = format!("/{}/signin", lang.code());
    let way_on = match code {
        Some(code) => code_view(lang, code, sure, signin).into_any(),
        None => view! {
            <p class="actions">
                <a class="button primary" href=signin>{words::reset_to_signin(lang)}</a>
            </p>
        }
        .into_any(),
    };
    view! {
        <section class="card step" aria-labelledby="reset-done-title">
            <h2 id="reset-done-title" tabindex="-1" data-step-focus>{title}</h2>
            <p>{text}</p>
            {way_on}
        </section>
    }
}

#[island]
pub fn Reset(lang: Lang, worker: String) -> impl IntoView {
    let step = RwSignal::new(Step::Form);
    let form = Form::early();
    // a code the link hands while the page is open: taken as at the start
    fragment::watch();
    let submit = move |event: leptos::ev::SubmitEvent| {
        event.prevent_default();
        let fields = form.fields();
        if let Some(problem) = form_problem(lang, &fields) {
            form.error.set(Some(problem));
            return;
        }
        form.clear(false);
        form.error.set(None);
        step.set(Step::Working);
        let worker = worker.clone();
        leptos::task::spawn_local(async move {
            match reset(&worker, fields).await {
                Ok(outcome) => {
                    form.clear(true);
                    if outcome.sure {
                        forget_ended(&worker).await;
                    }
                    step.set(Step::Done(outcome));
                }
                Err(refusal) => {
                    form.error.set(Some(refusal_text(lang, refusal)));
                    step.set(Step::Form);
                }
            }
        });
    };
    let mail = Mail::early();
    // As a registration's form: one sent faster than a person types is not
    // sent (`docs/registration-abuse.md`). Here the person is told, and
    // may try again.
    let shown = StoredValue::new(0u64);
    Effect::new(move |_| shown.set_value(super::pow::now_ms()));
    let ask_mail = move |event: leptos::ev::SubmitEvent| {
        event.prevent_default();
        if mail.busy.get_untracked() {
            return;
        }
        mail.said.set(None);
        let Some(name) = mail_name(&mail.name.get_untracked()) else {
            mail.error.set(Some(words::reset_mail_error_name(lang)));
            return;
        };
        if super::pow::now_ms().saturating_sub(shown.get_value()) < 3_000 {
            mail.error.set(Some(words::reset_mail_early(lang)));
            return;
        }
        mail.error.set(None);
        // the code that arrives is redeemed below, for this name
        form.username.set(name.clone());
        mail.busy.set(true);
        leptos::task::spawn_local(async move {
            let request = Request::ResetMail {
                username: name,
                lang: lang.code().to_owned(),
            };
            let answer = mailed(&call(&request).await);
            mail.busy.set(false);
            if answer == Mailed::Asked {
                mail.said.set(Some(mailed_text(lang, answer)));
            } else {
                mail.error.set(Some(mailed_text(lang, answer)));
            }
        });
    };
    // A new step replaces the old one's elements, focus among them: it
    // takes the focus where it begins (as the sign-in's steps do). The
    // first render leaves the focus where the page put it.
    Effect::new(move |shown: Option<()>| {
        step.track();
        if shown.is_some() {
            super::focus::after_render(
                "#reset-steps [data-step-focus], #reset-steps form.form input",
            );
        }
    });
    let steps = move || {
        match step.get() {
        Step::Form => form_view(lang, form, submit.clone()).into_any(),
        Step::Working => view! {
            <p class="working" role="status" tabindex="-1" data-step-focus>{words::signin_working(lang)}</p>
        }
        .into_any(),
        Step::Done(outcome) => done_view(lang, outcome).into_any(),
    }
    };
    // the block belongs to the form: once that is sent, both are over
    view! {
        <section class="reset-mail" aria-labelledby="reset-mail-title" hidden=move || step.get() != Step::Form>
            {mail_view(lang, mail, ask_mail)}
        </section>
        <div id="reset-steps" class="steps">{steps}</div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CODE: &str = "RESET-0000-0000-0000-0000-0000-0000-0000-0001";

    fn fields() -> Fields {
        Fields {
            username: "erika".into(),
            code: CODE.into(),
            keep: true,
            recovery: "0000-0000-0000-0000-0000-0000-0000-0002".into(),
            confirmed: false,
            password: "long enough".into(),
            repeat: "long enough".into(),
        }
    }

    /// A change to a good form, and the words it must bring.
    type Case = (fn(&mut Fields), fn(Lang) -> &'static str);

    #[test]
    fn the_form_says_what_it_lacks_before_anything_is_sent() {
        assert_eq!(form_problem(Lang::En, &fields()), None);
        let cases: [Case; 9] = [
            (|f| f.username = "  ".into(), words::reset_missing),
            (|f| f.code.clear(), words::reset_missing),
            (|f| f.password.clear(), words::reset_missing),
            // a recovery code is no reset code: the prefix is required
            (
                |f| f.code = "0000-0000-0000-0000-0000-0000-0000-0001".into(),
                words::reset_error_code,
            ),
            (|f| f.recovery = " ".into(), words::reset_recovery_missing),
            (
                |f| {
                    f.password = "short".into();
                    f.repeat = "short".into();
                },
                words::register_error_short,
            ),
            (
                |f| {
                    f.password = "x".repeat(1025);
                    f.repeat = f.password.clone();
                },
                words::register_error_long,
            ),
            (
                |f| f.repeat = "other words".into(),
                words::register_error_repeat,
            ),
            (|f| f.keep = false, words::reset_error_confirm),
        ];
        for (change, want) in cases {
            let mut f = fields();
            change(&mut f);
            assert_eq!(form_problem(Lang::En, &f), Some(want(Lang::En)));
        }
    }

    #[test]
    fn a_new_vault_is_asked_for_only_with_the_warning_confirmed() {
        let mut f = fields();
        f.keep = false;
        f.recovery.clear();
        assert_eq!(
            form_problem(Lang::En, &f),
            Some(words::reset_error_confirm(Lang::En))
        );
        f.confirmed = true;
        assert_eq!(form_problem(Lang::En, &f), None);
        // the confirmation is the new vault's: with the recovery code it
        // is not asked
        let mut kept = fields();
        kept.confirmed = false;
        assert_eq!(form_problem(Lang::En, &kept), None);
    }

    /// The page says of a refused name and code no more than that they do
    /// not go together: never which of them, and never in the words of a
    /// wrong password.
    #[test]
    fn central_s_refusals_all_read_the_same() {
        assert_eq!(by_central(Failure::Rejected), Refusal::Refused);
        for lang in Lang::ALL {
            let refused = refusal_text(lang, Refusal::Refused);
            assert_eq!(refused, words::reset_refused(lang));
            assert_ne!(refused, words::signin_wrong(lang));
            assert_ne!(refused, words::signin_not_moved(lang));
        }
        // what is no refusal of the name and code keeps its own words
        assert_eq!(by_central(Failure::Busy), Refusal::Other(Problem::Busy));
        assert_eq!(
            by_central(Failure::Network),
            Refusal::Other(Problem::Network)
        );
        assert_eq!(
            by_central(Failure::Unavailable),
            Refusal::Other(Problem::Unavailable)
        );
        // and nothing here ever reads as a wrong sign-in
        for failure in [
            Failure::Rejected,
            Failure::Forbidden,
            Failure::Conflict,
            Failure::Busy,
            Failure::Unavailable,
            Failure::Malformed,
            Failure::Network,
        ] {
            for refusal in [
                by_central(failure),
                other(failure),
                by_worker(failure, true),
                by_worker(failure, false),
            ] {
                assert_ne!(refusal, Refusal::Other(Problem::WrongLogin));
                assert_ne!(refusal, Refusal::Other(Problem::NotMoved));
            }
            if let Some(refusal) = at_finish(failure) {
                assert_ne!(refusal, Refusal::Other(Problem::WrongLogin));
            }
        }
    }

    #[test]
    fn the_worker_s_refusal_is_the_recovery_code_s_only_where_one_was_given() {
        assert_eq!(by_worker(Failure::Rejected, true), Refusal::Recovery);
        assert_eq!(
            by_worker(Failure::Rejected, false),
            Refusal::Other(Problem::Failed)
        );
        assert_eq!(
            by_worker(Failure::Malformed, true),
            Refusal::Other(Problem::Failed)
        );
    }

    /// A finish that Central refused changed nothing; one whose answer did
    /// not arrive may have, and is never reported as a refusal.
    #[test]
    fn a_lost_finish_is_no_refusal() {
        for lost in [Failure::Network, Failure::Unavailable, Failure::Malformed] {
            assert_eq!(at_finish(lost), None);
        }
        assert_eq!(
            at_finish(Failure::Rejected),
            Some(Refusal::Other(Problem::Failed))
        );
        assert_eq!(
            at_finish(Failure::Busy),
            Some(Refusal::Other(Problem::Busy))
        );
    }

    /// Only a sign-in name is sent for a code by mail, as Central keeps
    /// names: an e-mail address, or anything else that is none, never goes.
    #[test]
    fn a_code_by_mail_is_asked_for_a_sign_in_name_only() {
        assert_eq!(mail_name("erika"), Some("erika".into()));
        assert_eq!(mail_name("  Erika "), Some("erika".into()));
        assert_eq!(mail_name("e.r-k_9"), Some("e.r-k_9".into()));
        for none in [
            "",
            "  ",
            "er",
            "erika@example.org",
            "erika mustermann",
            "ërika",
            "-erika",
            "a-name-longer-than-central-keeps",
        ] {
            assert_eq!(mail_name(none), None, "{none}");
        }
        let request = Request::ResetMail {
            username: "erika".into(),
            lang: Lang::ZhHans.code().into(),
        };
        assert_eq!(
            serde_json::to_string(&request).expect("json"),
            r#"{"operation":"reset_mail","username":"erika","lang":"zh-hans"}"#
        );
    }

    /// Central's answer to the request is the same for every name, and so
    /// is what the page says: it promises no mail. Only where Central
    /// sends none is that said, and nothing reads as a wrong sign-in or as
    /// the refusal of a name with its code.
    #[test]
    fn what_a_code_by_mail_s_answer_says() {
        assert_eq!(mailed(&Ok(Response::Done)), Mailed::Asked);
        assert_eq!(mailed(&Err(Failure::Rejected)), Mailed::NoMail);
        assert_eq!(mailed(&Err(Failure::Busy)), Mailed::Busy);
        assert_eq!(
            mailed(&Err(Failure::Network)),
            Mailed::Other(Problem::Network)
        );
        assert_eq!(
            mailed(&Err(Failure::Unavailable)),
            Mailed::Other(Problem::Unavailable)
        );
        for failure in [Failure::Forbidden, Failure::Conflict, Failure::Malformed] {
            assert_eq!(mailed(&Err(failure)), Mailed::Other(Problem::Failed));
        }
        for lang in Lang::ALL {
            let asked = mailed_text(lang, Mailed::Asked);
            assert_eq!(asked, words::reset_mail_sent(lang));
            for other in [Mailed::NoMail, Mailed::Busy] {
                let text = mailed_text(lang, other);
                assert_ne!(text, asked);
                assert_ne!(text, words::reset_refused(lang));
                assert_ne!(text, words::signin_wrong(lang));
            }
        }
    }

    /// The mail is asked for in the page's language, and every language's
    /// code is a tag Central takes.
    #[test]
    fn every_language_is_a_tag_central_mails_in() {
        for lang in Lang::ALL {
            assert!(
                central_api::email::language(lang.code()).is_ok(),
                "{lang:?}"
            );
        }
    }
}
