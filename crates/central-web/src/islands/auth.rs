//! Signing in and registering (`docs/web-accounts.md` §"Flows"), and the
//! two steps that may follow either: a recovery code shown once, and
//! Central's terms to accept. The password goes only to the crypto Worker;
//! Central sees OPAQUE messages, never the password.

use central_api::Token;
use central_api::terms::{TermsDocument, TermsSubject, TermsText, TermsVersion, digest};
use central_api::wire::{AccountSource, Discovery, Identity, Request, Response};
use leptos::prelude::*;
use serde_json::json;

use super::central::{Failure, call, uuid_v7, valid_username};
use super::crypto::{Crypto, field_bytes};
use super::session;
use crate::markdown;
use crate::rich::rich;
use crate::words::{self, Lang};

/// What went wrong, in the words the page has for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Problem {
    /// The name or the password is not right.
    WrongLogin,
    /// An account that has not moved from the official gateway yet.
    NotMoved,
    /// Too many attempts just now.
    Busy,
    /// Central does not answer.
    Unavailable,
    /// No network.
    Network,
    /// Anything else, without detail.
    Failed,
}

impl From<Failure> for Problem {
    fn from(failure: Failure) -> Self {
        match failure {
            Failure::Rejected => Problem::WrongLogin,
            Failure::Busy => Problem::Busy,
            Failure::Unavailable => Problem::Unavailable,
            Failure::Network => Problem::Network,
            Failure::Forbidden | Failure::Conflict | Failure::Malformed => Problem::Failed,
        }
    }
}

/// The words for a problem.
pub fn problem_text(lang: Lang, problem: Problem) -> &'static str {
    match problem {
        Problem::WrongLogin => words::signin_wrong(lang),
        Problem::NotMoved => words::signin_not_moved(lang),
        Problem::Busy => words::error_busy(lang),
        Problem::Unavailable => words::error_unavailable(lang),
        Problem::Network => words::error_network(lang),
        Problem::Failed => words::error_failed(lang),
    }
}

/// A recovery code made at this sign-in, not stored yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewCode {
    /// Its written form, `XXXX-XXXX-…`.
    pub code: String,
    /// The master key under it (73 bytes), stored only once it was kept.
    pub recovery: Vec<u8>,
    /// Whether the account had none before (a moved account): it may say "Not now".
    pub optional: bool,
}

/// A finished sign-in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignedIn {
    /// The account's handle, `Name#3fa2`, once it has one.
    pub handle: Option<String>,
    /// A code made for an account that had none.
    pub new_code: Option<NewCode>,
}

/// Signs `username` in with `password` in a fresh Worker: discovery, the
/// two OPAQUE steps, then the master key opened in the Worker to check it,
/// to make a recovery code for an account without one, or to keep the
/// chat's identity in this browser. Any failure after Central said yes signs
/// out again. Returns once the Worker is done.
///
/// # Errors
/// What went wrong.
pub async fn sign_in(worker: &str, username: &str, password: String) -> Result<SignedIn, Problem> {
    let crypto = Crypto::start(worker).ok_or(Problem::Failed)?;
    sign_in_with(&crypto, username, password).await
}

/// [`sign_in`] in a Worker the caller keeps: after an account with a tag
/// signed in, it holds the chat's identity (the chat page's way back when
/// this browser kept none).
///
/// # Errors
/// What went wrong.
pub async fn sign_in_with(
    crypto: &Crypto,
    username: &str,
    password: String,
) -> Result<SignedIn, Problem> {
    let username = username.trim().to_ascii_lowercase();
    if !valid_username(&username) {
        return Err(Problem::WrongLogin);
    }
    let Response::Discovery { discovery } = call(&Request::Discover {
        username: username.clone(),
    })
    .await?
    else {
        return Err(Problem::Failed);
    };
    let identity = match discovery {
        Discovery::Central { identity } if uuid_v7(&identity.account) => identity,
        Discovery::Central { .. } => return Err(Problem::Failed),
        Discovery::Legacy => return Err(Problem::NotMoved),
    };
    let start = crypto
        .ask(
            "login_start",
            json!({ "password": password, "account": identity.account, "epoch": identity.epoch }),
        )
        .await?;
    drop(password);
    let message = field_bytes(&start, "message", Some(96))?;
    let Response::LoginChallenge {
        attempt,
        message: challenge,
    } = call(&Request::LoginStart { identity, message }).await?
    else {
        return Err(Problem::Failed);
    };
    // A wrong password fails here, in the Worker, before anything is sent.
    let finish = crypto
        .ask("login_finish", json!({ "response": challenge }))
        .await?;
    let proof = field_bytes(&finish, "message", Some(64))?;
    let Response::BrowserSession {
        identity: signed,
        wrapped_master,
        expires_at,
        ..
    } = call(&Request::LoginFinish {
        attempt,
        message: proof,
    })
    .await?
    else {
        return Err(Problem::Failed);
    };
    if signed.account != identity.account || signed.epoch != identity.epoch {
        let _ = call(&Request::Logout).await;
        return Err(Problem::Failed);
    }
    session::remember(expires_at);
    match finish_sign_in(crypto, &identity, wrapped_master).await {
        Ok(done) => Ok(done),
        Err(problem) => {
            let _ = call(&Request::Logout).await;
            session::forget();
            Err(problem)
        }
    }
}

/// After Central's yes: read the summary, then open the master key in the
/// Worker in the way the account needs.
async fn finish_sign_in(
    crypto: &Crypto,
    identity: &Identity,
    wrapped_master: Vec<u8>,
) -> Result<SignedIn, Problem> {
    let summary = call(&Request::AccountSummary).await.ok();
    let (recovery, tag, display_name, source) = match summary {
        Some(Response::AccountSummary {
            recovery,
            tag,
            display_name,
            source,
            ..
        }) => (recovery, tag, display_name, Some(source)),
        _ => (true, None, None, None),
    };
    if source == Some(AccountSource::Registered) && !recovery {
        return Err(Problem::Failed);
    }
    let handle = display_name
        .zip(tag)
        .map(|(name, tag)| central_api::profile::handle(&name, tag));
    if !recovery {
        let made = crypto
            .ask(
                "recovery_create",
                json!({ "wrapped_master": wrapped_master }),
            )
            .await?;
        let code = made
            .get("recovery_code")
            .and_then(serde_json::Value::as_str)
            .ok_or(Problem::Failed)?
            .to_owned();
        let recovery = field_bytes(&made, "recovery", Some(73))?;
        return Ok(SignedIn {
            handle,
            new_code: Some(NewCode {
                code,
                recovery,
                optional: true,
            }),
        });
    }
    if let Some(tag) = tag {
        crypto
            .ask(
                "chat_unlock",
                json!({ "wrapped_master": wrapped_master, "tag": tag }),
            )
            .await?;
        // Kept in this browser so the chat opens without a password while
        // the sign-in lasts (owner, 03.11.2026); losing it is no failure.
        let _ = crypto
            .ask(
                "keep",
                json!({ "account": identity.account, "epoch": identity.epoch, "tag": tag }),
            )
            .await;
    } else {
        crypto
            .ask("login_confirm", json!({ "wrapped_master": wrapped_master }))
            .await?;
    }
    Ok(SignedIn {
        handle,
        new_code: None,
    })
}

/// Central's terms when a version in force is not accepted yet, checked:
/// the texts must hash to the version's digest, so an acceptance names
/// exactly what was shown.
async fn terms_due() -> Result<Option<TermsDocument>, Problem> {
    let Response::TermsStatus { current, accepted } = call(&Request::TermsStatus).await? else {
        return Err(Problem::Failed);
    };
    let Some(current) = current.filter(|_| !accepted) else {
        return Ok(None);
    };
    let Response::Terms {
        terms: Some(document),
    } = call(&Request::Terms).await?
    else {
        return Err(Problem::Failed);
    };
    if document.version != current || digest(&document.texts) != current.digest {
        return Err(Problem::Failed);
    }
    Ok(Some(document))
}

/// The text of a document in `lang`, else English, else the first.
fn text_in(document: &TermsDocument, lang: Lang) -> Option<&TermsText> {
    document
        .texts
        .iter()
        .find(|text| text.lang == lang.code())
        .or_else(|| document.texts.iter().find(|text| text.lang == "en"))
        .or_else(|| document.texts.first())
}

/// Where a step leads after the sign-in.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Step {
    Form,
    Working,
    Code(NewCode, Option<String>),
    Terms(TermsDocument, Option<NewCode>, Option<String>),
    Failed(Problem),
}

/// After a sign-in: a new account's code first (it is stored already, and
/// declining the terms must not lose it); otherwise the terms first (storing
/// a moved account's new code needs them), then the code, then the account.
async fn after_sign_in(lang: Lang, done: SignedIn) -> Step {
    if let Some(code) = done.new_code.as_ref().filter(|code| !code.optional) {
        return Step::Code(code.clone(), done.handle);
    }
    match terms_due().await {
        Ok(Some(document)) => Step::Terms(document, done.new_code, done.handle),
        Ok(None) => {
            if let Some(code) = done.new_code {
                Step::Code(code, done.handle)
            } else {
                session::go(&format!("/{}/account", lang.code()));
                Step::Working
            }
        }
        Err(problem) => Step::Failed(problem),
    }
}

/// The code step: shown once, downloadable, kept only when the person says so.
fn code_view(
    lang: Lang,
    step: RwSignal<Step>,
    code: &NewCode,
    handle: Option<String>,
) -> impl IntoView + use<> {
    let kept = RwSignal::new(false);
    let error = RwSignal::new(None::<&'static str>);
    let download = format!(
        "data:text/plain;charset=utf-8,Baylee%20recovery%20code%3A%20{}%0A",
        code.code
    );
    let handle_line = handle.map(|handle| view! { <p>{words::code_handle(lang, &handle)}</p> });
    let optional = code.optional;
    let recovery = code.recovery.clone();
    let go_on = move |_| {
        if !kept.get_untracked() {
            error.set(Some(words::code_not_kept(lang)));
            return;
        }
        let recovery = recovery.clone();
        leptos::task::spawn_local(async move {
            if optional {
                match call(&Request::SetRecovery { recovery }).await {
                    Ok(_) | Err(Failure::Conflict) => {}
                    Err(failure) => {
                        error.set(Some(problem_text(lang, failure.into())));
                        return;
                    }
                }
            }
            // a new account meets Central's terms after its code
            match terms_due().await {
                Ok(Some(document)) => step.set(Step::Terms(document, None, None)),
                Ok(None) => {
                    session::go(&format!("/{}/account", lang.code()));
                    step.set(Step::Working);
                }
                Err(problem) => error.set(Some(problem_text(lang, problem))),
            }
        });
    };
    let later = optional.then(|| {
        let skip = move |_| session::go(&format!("/{}/account", lang.code()));
        view! { <button type="button" class="button quiet" on:click=skip>{words::code_later(lang)}</button> }
    });
    view! {
        <section class="card step" aria-labelledby="code-title">
            <h2 id="code-title">{words::code_title(lang)}</h2>
            {handle_line}
            <p>{words::code_lead(lang)}</p>
            <p class="eyebrow">{words::code_shown_once(lang)}</p>
            <p class="secret" translate="no"><code>{code.code.clone()}</code></p>
            <p class="actions">
                <a class="button" href=download download="baylee-recovery-code.txt">{words::code_download(lang)}</a>
            </p>
            <div class="check">
                <input id="code-kept" type="checkbox" on:change=move |ev| kept.set(event_target_checked(&ev)) />
                <label for="code-kept">{words::code_kept(lang)}</label>
            </div>
            <p class="error" role="alert">{move || error.get()}</p>
            <p class="actions">
                <button type="button" class="button primary" on:click=go_on>{words::code_continue(lang)}</button>
                {later}
            </p>
        </section>
    }
}

/// The terms step: the texts as Central serves them, accept or decline.
fn terms_view(
    lang: Lang,
    step: RwSignal<Step>,
    document: &TermsDocument,
    code: Option<NewCode>,
    handle: Option<String>,
) -> impl IntoView + use<> {
    let text = text_in(document, lang).map(|text| markdown::render(&text.markdown));
    let TermsVersion { version, digest } = document.version.clone();
    let accept = move |_| {
        let (version, code, handle) = (version.clone(), code.clone(), handle.clone());
        step.set(Step::Working);
        leptos::task::spawn_local(async move {
            let accepted = call(&Request::AcceptTerms {
                subject: TermsSubject::Central,
                version,
                digest,
            })
            .await;
            step.set(match (accepted, code) {
                (Ok(_), Some(code)) => Step::Code(code, handle),
                (Ok(_), None) => {
                    session::go(&format!("/{}/account", lang.code()));
                    Step::Working
                }
                (Err(failure), _) => Step::Failed(failure.into()),
            });
        });
    };
    let decline = move |_| {
        leptos::task::spawn_local(async move {
            let _ = call(&Request::Logout).await;
            session::forget();
            session::go(&format!("/{}/", lang.code()));
        });
    };
    view! {
        <section class="card step" aria-labelledby="terms-step-title">
            <h2 id="terms-step-title">{words::terms_step_title(lang)}</h2>
            <p>{words::terms_step_lead(lang)}</p>
            <div class="terms-text prose" tabindex="0" role="region" aria-labelledby="terms-step-title">{text}</div>
            <p class="actions">
                <button type="button" class="button primary" on:click=accept>{words::terms_accept(lang)}</button>
                <button type="button" class="button quiet" on:click=decline>{words::terms_decline(lang)}</button>
            </p>
        </section>
    }
}

/// The steps after the form, shared by signing in and registering.
fn steps_view(
    lang: Lang,
    step: RwSignal<Step>,
    form: impl Fn() -> AnyView + Send + Sync + 'static,
) -> impl IntoView {
    move || {
        match step.get() {
        Step::Form => form(),
        Step::Working => view! { <p class="working" role="status">{words::signin_working(lang)}</p> }.into_any(),
        Step::Code(code, handle) => code_view(lang, step, &code, handle).into_any(),
        Step::Terms(document, code, handle) => terms_view(lang, step, &document, code, handle).into_any(),
        Step::Failed(problem) => view! {
            <p class="error" role="alert">{problem_text(lang, problem)}</p>
            <p><button type="button" class="button" on:click=move |_| step.set(Step::Form)>{words::error_again(lang)}</button></p>
        }
        .into_any(),
    }
    }
}

#[island]
pub fn SignIn(lang: Lang, worker: String) -> impl IntoView {
    let step = RwSignal::new(Step::Form);
    let name = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let shown = RwSignal::new(false);
    let error = RwSignal::new(None::<&'static str>);
    let submit = move |event: leptos::ev::SubmitEvent| {
        event.prevent_default();
        let (username, secret) = (name.get_untracked(), password.get_untracked());
        if username.trim().is_empty() || secret.is_empty() {
            error.set(Some(words::signin_missing(lang)));
            return;
        }
        password.set(String::new());
        error.set(None);
        step.set(Step::Working);
        let worker = worker.clone();
        leptos::task::spawn_local(async move {
            let next = match sign_in(&worker, &username, secret).await {
                Ok(done) => after_sign_in(lang, done).await,
                Err(problem) => Step::Failed(problem),
            };
            step.set(next);
        });
    };
    let register = format!("/{}/register", lang.code());
    let form = move || {
        let register = register.clone();
        view! {
            <form class="form" method="post" novalidate=true on:submit=submit.clone()>
                <div class="field">
                    <label for="signin-name">{words::signin_name(lang)}</label>
                    <input id="signin-name" name="username" type="text" autocomplete="username" autocapitalize="none" spellcheck="false" maxlength="64" required=true
                        on:input=move |ev| name.set(event_target_value(&ev)) prop:value=move || name.get() />
                </div>
                <div class="field">
                    <label for="signin-password">{words::signin_password(lang)}</label>
                    <div class="with-tool">
                        <input id="signin-password" name="password" autocomplete="current-password" maxlength="1024" required=true
                            type=move || if shown.get() { "text" } else { "password" }
                            on:input=move |ev| password.set(event_target_value(&ev)) prop:value=move || password.get() />
                        <button type="button" class="button quiet" aria-pressed=move || if shown.get() { "true" } else { "false" }
                            on:click=move |_| shown.update(|s| *s = !*s)>{words::password_show(lang)}</button>
                    </div>
                </div>
                <p class="error" role="alert">{move || error.get()}</p>
                <button type="submit" class="button primary">{words::signin_submit(lang)}</button>
                <p class="hint">{rich(words::signin_no_account(lang), move |tag| (tag == "register").then(|| register.clone()))}</p>
            </form>
        }
        .into_any()
    };
    steps_view(lang, step, form)
}

/// The registration form's checks, as Central would refuse (`docs/web-accounts.md`).
fn register_problem(lang: Lang, f: &RegisterFields) -> Option<&'static str> {
    if f.key.trim().is_empty() {
        return Some(words::register_error_key(lang));
    }
    if !valid_username(&f.username.trim().to_ascii_lowercase()) {
        return Some(words::register_error_name(lang));
    }
    if !central_api::profile::valid_display_name(&f.display) {
        return Some(words::register_error_display(lang));
    }
    if central_api::profile::reserved(&f.display) {
        return Some(words::register_error_reserved(lang));
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
    if !f.age {
        return Some(words::register_error_age(lang));
    }
    if !f.consent {
        return Some(words::register_error_consent(lang));
    }
    None
}

#[derive(Clone, Debug, Default)]
struct RegisterFields {
    key: String,
    username: String,
    display: String,
    password: String,
    repeat: String,
    age: bool,
    consent: bool,
}

/// A token's bytes. `Token` is deliberately not `Clone`; the enrolment
/// ticket is sent twice, so the page keeps its bytes for the two requests.
fn token_bytes(token: &Token) -> [u8; 32] {
    serde_json::to_value(token)
        .ok()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or([0; 32])
}

/// A token from its bytes.
fn token(bytes: [u8; 32]) -> Token {
    serde_json::from_value(json!(bytes)).unwrap_or_else(|_| unreachable!("32 bytes are a token"))
}

/// Registers an account and signs it in: `Register`, the OPAQUE enrolment
/// with a new master key and recovery code in the Worker, then a sign-in
/// with the same password, which is let go after it.
async fn register(
    worker: &str,
    f: RegisterFields,
) -> Result<(NewCode, SignedIn), (Problem, Option<&'static str>)> {
    let username = f.username.trim().to_ascii_lowercase();
    let crypto = Crypto::start(worker).ok_or((Problem::Failed, None))?;
    let answer = call(&Request::Register {
        username: username.clone(),
        beta_key: f.key.trim().to_owned(),
        display_name: Some(f.display.clone()),
    })
    .await;
    let (identity, ticket): (Identity, [u8; 32]) = match answer {
        Ok(Response::Enrollment { identity, ticket }) if uuid_v7(&identity.account) => {
            (identity, token_bytes(&ticket))
        }
        Ok(_) => return Err((Problem::Failed, None)),
        Err(Failure::Rejected) => return Err((Problem::Failed, Some("key"))),
        Err(Failure::Conflict) => return Err((Problem::Failed, Some("taken"))),
        Err(failure) => return Err((failure.into(), None)),
    };
    let fail = |failure: Failure| (Problem::from(failure), None);
    let start = crypto
        .ask(
            "enrollment_start",
            json!({ "password": f.password, "account": identity.account, "epoch": identity.epoch }),
        )
        .await
        .map_err(fail)?;
    let message = field_bytes(&start, "message", Some(32)).map_err(fail)?;
    let Response::EnrollmentChallenge { message: challenge } = call(&Request::EnrollmentStart {
        ticket: token(ticket),
        message,
    })
    .await
    .map_err(fail)?
    else {
        return Err((Problem::Failed, None));
    };
    let done = crypto
        .ask("enrollment_finish", json!({ "response": challenge }))
        .await
        .map_err(fail)?;
    let code = done
        .get("recovery_code")
        .and_then(serde_json::Value::as_str)
        .ok_or((Problem::Failed, None))?
        .to_owned();
    let recovery = field_bytes(&done, "recovery", Some(73)).map_err(fail)?;
    let record = field_bytes(&done, "record", None).map_err(fail)?;
    let wrapped_master = field_bytes(&done, "wrapped_master", Some(73)).map_err(fail)?;
    let document = field_bytes(&done, "document", None).map_err(fail)?;
    crypto.close();
    call(&Request::EnrollmentFinish {
        ticket: token(ticket),
        record,
        wrapped_master,
        document,
        recovery: Some(recovery.clone()),
    })
    .await
    .map_err(fail)?;
    let signed = sign_in(worker, &username, f.password)
        .await
        .map_err(|problem| (problem, None))?;
    Ok((
        NewCode {
            code,
            recovery,
            optional: false,
        },
        signed,
    ))
}

#[island]
pub fn Register(lang: Lang, worker: String) -> impl IntoView {
    let step = RwSignal::new(Step::Form);
    let fields = RwSignal::new(RegisterFields::default());
    let error = RwSignal::new(None::<&'static str>);
    let submit = move |event: leptos::ev::SubmitEvent| {
        event.prevent_default();
        let f = fields.get_untracked();
        if let Some(problem) = register_problem(lang, &f) {
            error.set(Some(problem));
            return;
        }
        fields.update(|f| {
            f.password.clear();
            f.repeat.clear();
        });
        error.set(None);
        step.set(Step::Working);
        let worker = worker.clone();
        leptos::task::spawn_local(async move {
            match register(&worker, f).await {
                Ok((code, signed)) => {
                    // The code first: it is shown once, before anything
                    // else; Central's terms follow it (`code_view`).
                    step.set(Step::Code(code, signed.handle));
                }
                Err((_, Some("key"))) => {
                    error.set(Some(words::register_error_key_refused(lang)));
                    step.set(Step::Form);
                }
                Err((_, Some("taken"))) => {
                    error.set(Some(words::register_error_taken(lang)));
                    step.set(Step::Form);
                }
                Err((problem, _)) => step.set(Step::Failed(problem)),
            }
        });
    };
    let terms = format!("/{}/terms", lang.code());
    let privacy = format!("/{}/privacy", lang.code());
    let text_field = move |id: &'static str,
                           label: &'static str,
                           hint: &'static str,
                           kind: &'static str,
                           auto: &'static str,
                           set: fn(&mut RegisterFields, String)| {
        let hint_id = format!("{id}-hint");
        view! {
            <div class="field">
                <label for=id>{label}</label>
                <input id=id type=kind autocomplete=auto autocapitalize="none" spellcheck="false" maxlength="1024" aria-describedby=hint_id.clone()
                    on:input=move |ev| { let value = event_target_value(&ev); fields.update(|f| set(f, value)); } />
                <p class="hint" id=hint_id>{hint}</p>
            </div>
        }
    };
    let form = move || {
        let (terms, privacy) = (terms.clone(), privacy.clone());
        view! {
            <form class="form" method="post" novalidate=true on:submit=submit.clone()>
                {text_field("register-key", words::register_key(lang), words::register_key_hint(lang), "text", "off", |f, v| f.key = v)}
                {text_field("register-name", words::register_name(lang), words::register_name_hint(lang), "text", "username", |f, v| f.username = v)}
                {text_field("register-display", words::register_display(lang), words::register_display_hint(lang), "text", "nickname", |f, v| f.display = v)}
                {text_field("register-password", words::register_password(lang), words::register_password_hint(lang), "password", "new-password", |f, v| f.password = v)}
                {text_field("register-repeat", words::register_repeat(lang), words::register_repeat_hint(lang), "password", "new-password", |f, v| f.repeat = v)}
                <div class="check">
                    <input id="register-age" type="checkbox" on:change=move |ev| { let on = event_target_checked(&ev); fields.update(|f| f.age = on); } />
                    <label for="register-age">{words::register_age(lang)}</label>
                </div>
                <div class="check">
                    <input id="register-consent" type="checkbox" on:change=move |ev| { let on = event_target_checked(&ev); fields.update(|f| f.consent = on); } />
                    <label for="register-consent">{rich(words::register_consent(lang), move |tag| match tag {
                        "terms" => Some(terms.clone()),
                        "privacy" => Some(privacy.clone()),
                        _ => None,
                    })}</label>
                </div>
                <p class="error" role="alert">{move || error.get()}</p>
                <button type="submit" class="button primary">{words::register_submit(lang)}</button>
            </form>
        }
        .into_any()
    };
    steps_view(lang, step, form)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields() -> RegisterFields {
        RegisterFields {
            key: "CENTRAL-AAAA".into(),
            username: "erika".into(),
            display: "Erika".into(),
            password: "long enough".into(),
            repeat: "long enough".into(),
            age: true,
            consent: true,
        }
    }

    /// A change to a good form, and the words it must bring.
    type Case = (fn(&mut RegisterFields), fn(Lang) -> &'static str);

    #[test]
    fn the_form_says_early_what_central_would_refuse() {
        assert_eq!(register_problem(Lang::En, &fields()), None);
        let cases: [Case; 7] = [
            (|f| f.username = "E!".into(), words::register_error_name),
            (|f| f.display = "x".into(), words::register_error_display),
            (
                |f| f.display = "Admin".into(),
                words::register_error_reserved,
            ),
            (
                |f| {
                    f.password = "short".into();
                    f.repeat = "short".into();
                },
                words::register_error_short,
            ),
            (
                |f| f.repeat = "other words".into(),
                words::register_error_repeat,
            ),
            (|f| f.age = false, words::register_error_age),
            (|f| f.consent = false, words::register_error_consent),
        ];
        for (change, want) in cases {
            let mut f = fields();
            change(&mut f);
            assert_eq!(register_problem(Lang::En, &f), Some(want(Lang::En)));
        }
    }

    #[test]
    fn terms_in_the_page_language_else_english() {
        let text = |lang: &str| TermsText {
            lang: lang.into(),
            updated: None,
            markdown: lang.into(),
        };
        let document = TermsDocument {
            version: TermsVersion {
                version: "1".into(),
                digest: [0; 32],
            },
            texts: vec![text("de"), text("en")],
        };
        assert_eq!(
            text_in(&document, Lang::De).map(|t| t.lang.as_str()),
            Some("de")
        );
        assert_eq!(
            text_in(&document, Lang::En).map(|t| t.lang.as_str()),
            Some("en")
        );
    }
}
