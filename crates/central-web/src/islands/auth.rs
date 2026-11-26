//! Signing in and registering (`docs/web-accounts.md` §"Flows"), and the
//! two steps that may follow either: a recovery code shown once, and
//! Central's terms to accept. The password goes only to the crypto Worker;
//! Central sees OPAQUE messages, never the password.

use central_api::Token;
use central_api::email::EmailRequest;
use central_api::profile::tag_text;
use central_api::social::{FriendInvitePreview, FriendInviteStatus, valid_friend_invite_code};
use central_api::terms::{TermsDocument, TermsSubject, TermsText, TermsVersion, digest};
use central_api::wire::{AccountSource, Discovery, Identity, Request, Response};
use leptos::prelude::*;
use serde_json::json;

use super::central::{Failure, call, uuid_v7, valid_username};
use super::chat::glyph;
use super::crypto::{Crypto, field_bytes};
use super::invite::{normalize_invite_code, percent_encode_component, person_handle, query_param};
use super::{field, session};
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

impl Problem {
    /// A failure where no name and password were judged: `Rejected` is then
    /// no wrong sign-in, and never reads as one.
    pub fn outside_sign_in(failure: Failure) -> Self {
        match failure {
            Failure::Rejected => Problem::Failed,
            other => other.into(),
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
    /// The sign-in name, as Central wrote it back.
    pub username: String,
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
        username: canonical,
        wrapped_master,
        expires_at,
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
        Ok(done) => Ok(SignedIn {
            username: canonical,
            ..done
        }),
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
    // the header names who signed in from now on
    session::note_name(display_name.as_deref(), tag);
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
            username: String::new(),
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
        username: String::new(),
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

/// Who signed in, and what the steps after the sign-in carry along.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Who {
    /// The account's handle, once it has one.
    handle: Option<String>,
    /// The sign-in name.
    username: String,
    /// True when an invite-linked email confirmation mail was dispatched during signup.
    email_linked: bool,
}

impl Who {
    /// Who a sign-in signed in.
    fn of(done: SignedIn) -> Self {
        Self {
            handle: done.handle,
            username: done.username,
            email_linked: false,
        }
    }
}

/// Where a step leads after the sign-in.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Step {
    Form,
    Working,
    Code(NewCode, Who),
    Terms(TermsDocument, Option<NewCode>, Who),
    Failed(Problem),
}

fn safe_next_target() -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        query_param("next").filter(|dest| session::local(dest) && !session::form_address(dest))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
}

/// After a sign-in: a new account's code first (it is stored already, and
/// declining the terms must not lose it); otherwise the terms first (storing
/// a moved account's new code needs them), then the code, then the account.
async fn after_sign_in(lang: Lang, done: SignedIn, next: Option<&str>) -> Step {
    let new_code = done.new_code.clone();
    let who = Who::of(done);
    if let Some(code) = new_code.as_ref().filter(|code| !code.optional) {
        return Step::Code(code.clone(), who);
    }
    match terms_due().await {
        Ok(Some(document)) => Step::Terms(document, new_code, who),
        Ok(None) => match new_code {
            Some(code) => Step::Code(code, who),
            None => onward(lang, next),
        },
        Err(problem) => Step::Failed(problem),
    }
}

/// Past the code and the terms: send the browser to `next` (when safe & local)
/// or to the account area.
fn onward(lang: Lang, next: Option<&str>) -> Step {
    let fallback = format!("/{}/account", lang.code());
    let dest = next.filter(|p| !p.is_empty()).unwrap_or(&fallback);
    session::go(dest);
    Step::Working
}

/// Confirmed e-mail address already on the account, if any (used to prefill
/// the recovery backup mail address for migrating accounts).
async fn confirmed_email() -> Option<String> {
    let request = Request::Email {
        request: EmailRequest::Status,
    };
    match call(&request).await {
        Ok(Response::Email { state }) => state.address,
        _ => None,
    }
}

/// The handle a new account got, with an inline copy button.
fn handle_line(lang: Lang, handle: String) -> impl IntoView + use<> {
    let shown: AnyView = match handle.rsplit_once('#') {
        Some((name, tag)) => {
            super::ui::copy_handle::handle_with_copy(lang, name.to_owned(), tag.to_owned(), false)
        }
        None => view! { <span class="who-name" translate="no">{handle}</span> }.into_any(),
    };
    view! {
        <p class="who">
            <span class="who-label">{words::code_handle_label(lang)}</span>
            <span class="who-handle">{shown}</span>
        </p>
    }
}

/// Inline form error alert rendered with an alert icon whenever `error` is set.
pub(super) fn auth_alert(error: RwSignal<Option<&'static str>>) -> impl IntoView {
    move || {
        error.get().map(|msg| {
            view! {
                <p class="error auth-alert" role="alert">
                    {glyph::glyph(glyph::SHIELD_ALERT)}
                    <span>{msg}</span>
                </p>
            }
        })
    }
}

struct CodeSubmit {
    lang: Lang,
    optional: bool,
    recovery: Vec<u8>,
    mail: Option<(String, String, String)>,
    who: Who,
    next: Option<String>,
}

async fn finish_code(submit: CodeSubmit) -> Result<Step, &'static str> {
    let CodeSubmit {
        lang,
        optional,
        recovery,
        mail,
        who,
        next,
    } = submit;
    if optional {
        match call(&Request::SetRecovery { recovery }).await {
            Ok(_) | Err(Failure::Conflict) => {}
            Err(failure) => return Err(problem_text(lang, failure.into())),
        }
    }
    if let Some((email, sheet, zip_pw)) = mail {
        let Ok((b64, encrypted)) = super::zip::recovery_zip_base64(&sheet, &zip_pw).await else {
            return Err(words::keep_mail_send_error(lang));
        };
        if super::central::send_recovery_backup_mail(&email, &b64, encrypted, lang.code())
            .await
            .is_err()
        {
            return Err(words::keep_mail_send_error(lang));
        }
    }
    match terms_due().await {
        Ok(Some(document)) => Ok(Step::Terms(document, None, who)),
        Ok(None) => Ok(onward(lang, next.as_deref())),
        Err(problem) => Err(problem_text(lang, problem)),
    }
}

fn prefill_confirmed_email(email: RwSignal<String>) {
    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            if let Some(addr) = confirmed_email().await
                && email.get_untracked().trim().is_empty()
            {
                email.set(addr);
            }
        });
    });
}

fn submit_code_step(
    step: RwSignal<Step>,
    busy: RwSignal<bool>,
    error: RwSignal<Option<&'static str>>,
    job: CodeSubmit,
) {
    error.set(None);
    busy.set(true);
    leptos::task::spawn_local(async move {
        match finish_code(job).await {
            Ok(next) => {
                busy.set(false);
                step.set(next);
            }
            Err(msg) => {
                busy.set(false);
                error.set(Some(msg));
            }
        }
    });
}

/// The code step: shown once, with the ways to keep it (`keep.rs`), and
/// left only once the person says it is kept. Selecting "Mail it to myself"
/// unfolds an e-mail input (saved to the account on send) and an optional
/// ZIP password input to mail `baylee-recovery-code.zip` before advancing.
fn code_view(
    lang: Lang,
    step: RwSignal<Step>,
    code: &NewCode,
    who: Who,
    next_dest: StoredValue<Option<String>>,
) -> impl IntoView + use<> {
    let kept = RwSignal::new(false);
    let busy = RwSignal::new(false);
    let error = RwSignal::new(None::<&'static str>);
    let mail = super::keep::MailBackup {
        chosen: RwSignal::new(false),
        email: RwSignal::new(String::new()),
        zip_pw: RwSignal::new(String::new()),
    };
    let optional = code.optional;
    let email_linked = who.email_linked;
    prefill_confirmed_email(mail.email);
    let ways = super::keep::ways_onboarding(lang, &code.code, &who.username, mail);
    let handle_line = who.handle.clone().map(|handle| handle_line(lang, handle));
    let created = (!optional).then(|| {
        view! {
            <p class="step-badge">
                {glyph::glyph(glyph::SHIELD_CHECK)}
                <span>{words::code_created(lang)}</span>
            </p>
        }
    });
    let sheet = StoredValue::new(super::keep::sheet_text(lang, &code.code, &who.username));
    let recovery = code.recovery.clone();
    let go_on = move |_| {
        if busy.get_untracked() {
            return;
        }
        if !kept.get_untracked() {
            error.set(Some(words::code_not_kept(lang)));
            return;
        }
        let chosen = mail.chosen.get_untracked();
        let named = mail.email.get_untracked().trim().to_owned();
        if chosen && central_api::email::address(&named).is_err() {
            error.set(Some(words::email_error_address(lang)));
            super::focus::after_render("#code-email");
            return;
        }
        let job = CodeSubmit {
            lang,
            optional,
            recovery: recovery.clone(),
            mail: chosen.then(|| (named, sheet.get_value(), mail.zip_pw.get_untracked())),
            who: who.clone(),
            next: next_dest.get_value(),
        };
        submit_code_step(step, busy, error, job);
    };
    let later = optional.then(|| {
        let skip = move |_| {
            let _ = onward(lang, next_dest.get_value().as_deref());
        };
        view! { <button type="button" class="button quiet" on:click=skip>{words::code_later(lang)}</button> }
    });
    view! {
        <section class="card step step-code" aria-labelledby="code-title">
            <header class="step-head">
                {created}
                <h2 id="code-title" tabindex="-1" data-step-focus>{words::code_title(lang)}</h2>
                {handle_line}
            </header>
            <p class="step-lead">{words::code_lead(lang)}</p>
            {email_linked.then(|| view! {
                <p class="notice" role="status">{words::invite_page_email_pending_ok(lang)}</p>
            })}
            {super::keep::secret_frame(lang, code.code.clone())}
            {ways}
            <div class="check">
                <input id="code-kept" type="checkbox" on:change=move |ev| kept.set(event_target_checked(&ev)) />
                <label for="code-kept">{words::code_kept(lang)}</label>
            </div>
            {auth_alert(error)}
            <div class="step-actions">
                <button type="button" class="button primary step-go" aria-disabled=move || busy.get().then_some("true") on:click=go_on>
                    <span>{words::code_continue(lang)}</span>
                    {glyph::glyph(glyph::FORWARD)}
                </button>
                {later}
            </div>
        </section>
    }
}

fn terms_details(lang: Lang, document: &TermsDocument) -> (Option<AnyView>, String, TermsVersion) {
    let found = text_in(document, lang);
    let updated = found
        .and_then(|t| t.updated.as_deref())
        .map(|d| crate::format::date(lang, d));
    let text = found.map(|text| markdown::render(&text.markdown));
    let version_info = document.version.clone();
    let version_display = crate::format::date(lang, &version_info.version);
    let named = match updated {
        Some(ref date) => words::terms_version(lang, &version_display, date),
        None => words::terms_step_version(lang, &version_display),
    };
    (text, named, version_info)
}

fn measure_terms_scroll(
    seen: RwSignal<u8>,
    read: RwSignal<bool>,
    top: f64,
    shown: f64,
    whole: f64,
) {
    let rest = whole - shown;
    let share = if rest <= 4.0 {
        1.0
    } else {
        (top / rest).clamp(0.0, 1.0)
    };
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a share of one, in eighths"
    )]
    seen.set((share * 8.0).floor() as u8);
    if rest - top <= 4.0 {
        read.set(true);
    }
}

/// The terms step: the texts as Central serves them, accept or decline.
fn terms_view(
    lang: Lang,
    step: RwSignal<Step>,
    document: &TermsDocument,
    code: Option<NewCode>,
    who: Who,
    next_dest: StoredValue<Option<String>>,
) -> impl IntoView + use<> {
    let (text, named, TermsVersion { version, digest }) = terms_details(lang, document);
    let read = RwSignal::new(false);
    let seen = RwSignal::new(0_u8);
    let pressed = RwSignal::new(false);
    let scrolled = move |event: leptos::ev::Event| {
        let target = event_target::<leptos::web_sys::Element>(&event);
        measure_terms_scroll(
            seen,
            read,
            f64::from(target.scroll_top()),
            f64::from(target.client_height()),
            f64::from(target.scroll_height()),
        );
    };
    let text_box = NodeRef::<leptos::html::Div>::new();
    Effect::new(move |_| {
        if let Some(text_box) = text_box.get() {
            measure_terms_scroll(
                seen,
                read,
                f64::from(text_box.scroll_top()),
                f64::from(text_box.client_height()),
                f64::from(text_box.scroll_height()),
            );
        }
    });
    let accept = move |_| {
        if !read.get_untracked() {
            pressed.set(true);
            return;
        }
        let (version, code, who) = (version.clone(), code.clone(), who.clone());
        step.set(Step::Working);
        leptos::task::spawn_local(async move {
            let accepted = call(&Request::AcceptTerms {
                subject: TermsSubject::Central,
                version,
                digest,
            })
            .await;
            step.set(match (accepted, code) {
                (Ok(_), Some(code)) => Step::Code(code, who),
                (Ok(_), None) => onward(lang, next_dest.get_value().as_deref()),
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
        <section class="card step step-terms" aria-labelledby="terms-step-title">
            <header class="step-head">
                <h2 id="terms-step-title" tabindex="-1" data-step-focus>{words::terms_step_title(lang)}</h2>
                <p class="step-version">{named}</p>
            </header>
            <p class="step-lead">{words::terms_step_lead(lang)}</p>
            <div class="terms-box">
                <div class="terms-text prose" tabindex="0" role="region" aria-labelledby="terms-step-title"
                    node_ref=text_box on:scroll=scrolled>{text}</div>
                <p class="terms-seen" aria-hidden="true" data-seen=move || seen.get().to_string() data-read=move || read.get().then_some("")></p>
            </div>
            <p class="terms-hint" id="terms-read-hint" role="status" class:asked=move || pressed.get() && !read.get()>
                {move || if read.get() { words::terms_read_done(lang) } else { words::terms_read_hint(lang) }}
            </p>
            <div class="step-actions two">
                <button type="button" class="button quiet" on:click=decline>{words::terms_decline(lang)}</button>
                <button type="button" class="button primary step-go" aria-describedby="terms-read-hint"
                    aria-disabled=move || if read.get() { "false" } else { "true" } on:click=accept>
                    {glyph::glyph(glyph::SHIELD_CHECK)}
                    <span>{words::terms_accept(lang)}</span>
                </button>
            </div>
        </section>
    }
}

/// The steps after the form, shared by signing in and registering. `id`
/// names the element they stand in: a page can hold both forms (each in its
/// dialog) beside a form of its own, and a step's focus stays in its own.
fn steps_view(
    lang: Lang,
    id: &'static str,
    step: RwSignal<Step>,
    solving: Option<RwSignal<Solving>>,
    next_dest: StoredValue<Option<String>>,
    form: impl Fn() -> AnyView + Send + Sync + 'static,
) -> impl IntoView {
    // A new step replaces the old one's elements, focus among them: it
    // takes the focus where it begins, so the keyboard and a screen reader
    // go on there and not at the page's end. The first render leaves the
    // focus where the page put it.
    Effect::new(move |shown: Option<()>| {
        step.track();
        if shown.is_some() {
            // its heading, its status, the way back, or the form's first field
            super::focus::after_render(format!("#{id} [data-step-focus], #{id} form.form input"));
        }
    });
    let steps = move || {
        match step.get() {
            Step::Form => form(),
            Step::Working => view! {
                <div class="working-step">
                    <p class="working" role="status" tabindex="-1" data-step-focus>{words::signin_working(lang)}</p>
                </div>
            }
            .into_any(),
            Step::Code(code, who) => code_view(lang, step, &code, who, next_dest).into_any(),
            Step::Terms(document, code, who) => {
                terms_view(lang, step, &document, code, who, next_dest).into_any()
            }
            Step::Failed(problem) => view! {
                <section class="card step auth-failed" aria-labelledby="auth-failed-title">
                    <div class="step-head">
                        <span class="auth-failed-icon" aria-hidden="true">{glyph::glyph(glyph::SHIELD_ALERT)}</span>
                        <h2 id="auth-failed-title" tabindex="-1">{words::signin_failed_title(lang)}</h2>
                        <p class="auth-failed-msg" role="alert">{problem_text(lang, problem)}</p>
                        <p class="step-lead">{words::signin_failed_hint(lang)}</p>
                    </div>
                    <div class="step-actions">
                        <button type="button" class="button primary step-go" data-step-focus on:click=move |_| step.set(Step::Form)>
                            {glyph::glyph(glyph::REFRESH_CW)}
                            <span>{words::error_again(lang)}</span>
                        </button>
                    </div>
                </section>
            }
            .into_any(),
        }
    };
    let progress = solving.map(|s| solving_view(lang, s));
    view! { <div id=id class="steps">{steps}{progress}</div> }
}

#[island]
pub fn SignIn(lang: Lang, worker: String) -> impl IntoView {
    let step = RwSignal::new(Step::Form);
    // what was typed before the islands ran stays (`early`)
    let name = RwSignal::new(super::early::typed("signin-name"));
    let password = RwSignal::new(super::early::typed("signin-password"));
    let shown = RwSignal::new(false);
    let error = RwSignal::new(None::<&'static str>);
    let next_dest = StoredValue::new(safe_next_target());
    let submit = move |event: leptos::ev::SubmitEvent| {
        event.prevent_default();
        let (username, secret) = (name.get_untracked(), password.get_untracked());
        if username.trim().is_empty() || secret.is_empty() {
            error.set(Some(words::signin_missing(lang)));
            return;
        }
        password.set(String::new());
        // a field left readable is hidden again for the next try
        shown.set(false);
        error.set(None);
        step.set(Step::Working);
        let worker = worker.clone();
        leptos::task::spawn_local(async move {
            // another account's notifications never reach this one
            let _ = super::push::disable().await;
            let next = match sign_in(&worker, &username, secret).await {
                Ok(done) => after_sign_in(lang, done, next_dest.get_value().as_deref()).await,
                Err(problem) => Step::Failed(problem),
            };
            step.set(next);
        });
    };
    let invite_prefix = format!("/{}/invite/", lang.code());
    let register = next_dest
        .get_value()
        .and_then(|dest| {
            dest.strip_prefix(&invite_prefix)
                .filter(|code| !code.is_empty() && !code.contains('/'))
                .map(|code| {
                    format!(
                        "/{}/register?invite={}",
                        lang.code(),
                        percent_encode_component(code)
                    )
                })
        })
        .unwrap_or_else(|| format!("/{}/register", lang.code()));
    // a page of its own, in the dialog as on the page: a plain link
    let reset = format!("/{}/reset", lang.code());
    let form = move || {
        let (register, reset) = (register.clone(), reset.clone());
        view! {
            <form class="form" method="post" novalidate=true on:submit=submit.clone()>
                <div class="field">
                    <label for="signin-name">{words::signin_name(lang)}</label>
                    <div class="inset">
                        {field::mark(glyph::USER)}
                        <input id="signin-name" type="text" autocomplete="username" autocapitalize="none" spellcheck="false" maxlength="64" required=true
                            on:input=move |ev| name.set(event_target_value(&ev)) prop:value=move || name.get() />
                    </div>
                </div>
                <div class="field">
                    <label for="signin-password">{words::signin_password(lang)}</label>
                    <div class="inset">
                        {field::mark(glyph::LOCK)}
                        <input id="signin-password" autocomplete="current-password" maxlength="1024" required=true
                            type=move || if shown.get() { "text" } else { "password" }
                            on:input=move |ev| password.set(event_target_value(&ev)) prop:value=move || password.get() />
                        {field::eye(lang, shown, "signin-password")}
                    </div>
                </div>
                {auth_alert(error)}
                <button type="submit" class="button primary">{words::signin_submit(lang)}</button>
                <div class="hints">
                    <p class="hint">{rich(words::signin_forgot(lang), move |tag| (tag == "reset").then(|| reset.clone()))}</p>
                    <p class="hint">{rich(words::signin_no_account(lang), move |tag| (tag == "register").then(|| register.clone()))}</p>
                </div>
            </form>
        }
        .into_any()
    };
    steps_view(lang, "signin-steps", step, None, next_dest, form)
}

/// The registration form's checks, as Central would refuse (`docs/web-accounts.md`).
fn register_problem(lang: Lang, f: &RegisterFields) -> Option<&'static str> {
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
    username: String,
    display: String,
    password: String,
    repeat: String,
    age: bool,
    consent: bool,
    invite_code: Option<String>,
    link_email: Option<String>,
}

impl RegisterFields {
    /// The form as it stands when the islands start: what was typed or
    /// ticked before they ran stays (`early`).
    fn early() -> Self {
        Self {
            username: super::early::typed("register-name"),
            display: super::early::typed("register-display"),
            password: super::early::typed("register-password"),
            repeat: super::early::typed("register-repeat"),
            age: super::early::ticked("register-age"),
            consent: super::early::ticked("register-consent"),
            invite_code: None,
            link_email: None,
        }
    }
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

/// How far the proof of work is, for the line a person reads while it
/// takes long.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Solving {
    /// Nothing is being solved.
    #[default]
    Idle,
    /// Solving, this share done, and quick so far: nothing is said.
    Quiet(u32),
    /// Solving for longer than [`SLOW_MS`]: the share done is shown.
    Shown(u32),
}

/// How long a solution is held back after the challenge arrived, in
/// milliseconds: Central takes none sooner than three seconds after it
/// issued the challenge, which was before the answer came.
const HANDOVER_MS: u64 = 3_200;
/// How long the proof of work may take before the page says what it does.
const SLOW_MS: u32 = 3_000;

/// How long to wait still, `now`, with a challenge that arrived at
/// `answered` (both in the browser clock's milliseconds). The time counts
/// from the answer and not from the request: a slow connection spends its
/// time before Central issues the challenge, and a solution handed in
/// three seconds after the request would be too early for Central, which
/// spends the challenge on it. A clock set back waits the whole time.
fn handover_wait(answered: u64, now: u64) -> u32 {
    u32::try_from(HANDOVER_MS.saturating_sub(now.saturating_sub(answered))).unwrap_or(u32::MAX)
}

/// A registration's challenge as the page takes it.
enum Challenge {
    /// The proof that costs memory (`Response::RegisterWork`), at Central's
    /// price: what a Central of this build asks.
    Work {
        nonce: String,
        memory: u32,
        passes: u32,
        parts: u8,
        difficulty: u8,
    },
    /// The older, SHA-256 proof (`Response::RegisterChallenge`): what a
    /// Central from before asks.
    Older {
        nonce: String,
        parts: u8,
        difficulty: u8,
    },
}

/// Asks Central for a registration's challenge. The page says what it saw
/// of its own browser, which only ever makes the challenge harder. A price
/// above what this page and Central agreed on at build time
/// (`central_api::register`) is refused before any work, so the page never
/// starts on what it could not hand in or what would exhaust a phone.
///
/// # Errors
/// The problem, tagged `closed` while an administrator closed registration.
async fn challenge(
    invite_code: Option<&str>,
) -> Result<Challenge, (Problem, Option<&'static str>)> {
    use central_api::register::{MAX_DIFFICULTY, PARTS, valid_nonce, work_acceptable};

    let signals = super::pow::browser_signals();
    let answer = if let Some(code) = invite_code {
        call(&Request::RegisterInviteChallenge {
            signals,
            invite_code: code.to_owned(),
        })
        .await
    } else {
        let reported = Request::RegisterChallengeFor { signals };
        match call(&reported).await {
            // a Central from before the page reported its browser
            Err(Failure::Rejected) => call(&Request::RegisterChallenge).await,
            answer => answer,
        }
    };
    let challenge = match answer.map_err(|failure| (Problem::from(failure), None))? {
        Response::RegisterWork {
            nonce,
            parts,
            difficulty,
            memory,
            passes,
            ..
        } if valid_nonce(&nonce) && work_acceptable(memory, passes, parts, difficulty) => {
            Challenge::Work {
                nonce,
                memory,
                passes,
                parts,
                difficulty,
            }
        }
        // a Central from before the proof cost memory
        Response::RegisterChallenge {
            nonce,
            parts,
            difficulty,
            ..
        } if valid_nonce(&nonce) && parts == PARTS && difficulty <= MAX_DIFFICULTY => {
            Challenge::Older {
                nonce,
                parts,
                difficulty,
            }
        }
        Response::RegistrationClosed => {
            let tag = if invite_code.is_some() {
                "invite_invalid"
            } else {
                "closed"
            };
            return Err((Problem::Failed, Some(tag)));
        }
        _ => return Err((Problem::Failed, None)),
    };
    Ok(challenge)
}

/// Counters one request to the Worker tries: a few evaluations, so that a
/// slow device answers within a moment and the Worker can be ended between
/// two of them.
const WORK_TRIES: u32 = 8;

/// Solves a memory-hard challenge in the crypto Worker: for each part, the
/// module's own search (`central_crypto::proof`, the function Central
/// checks with) in turns of [`WORK_TRIES`], telling `progress` the share
/// of parts done. The caller ends this Worker afterwards: its memory
/// does not shrink, and the password is stretched in a fresh one.
async fn work(
    crypto: &Crypto,
    nonce: &str,
    (memory, passes, parts, difficulty): (u32, u32, u8, u8),
    progress: impl Fn(u32),
) -> Option<Vec<u64>> {
    let price = json!({
        "nonce": nonce, "memory": memory, "passes": passes, "parts": parts, "difficulty": difficulty,
    });
    crypto.ask("work_start", price).await.ok()?;
    // A part needs 2^difficulty tries on average. One that found nothing
    // in 64 times that is not unlucky (e^-64): the search is broken.
    let most = 64_u64 << difficulty.min(32);
    let mut counters = Vec::with_capacity(usize::from(parts));
    for part in 0..parts {
        let mut from = 0_u64;
        let counter = loop {
            let turn = json!({ "part": part, "from": from, "tries": WORK_TRIES });
            let answer = crypto.ask("work_search", turn).await.ok()?;
            match answer.get("counter")? {
                serde_json::Value::Null => from += u64::from(WORK_TRIES),
                found => break found.as_u64()?,
            }
            if from > most {
                return None;
            }
        };
        counters.push(counter);
        progress(u32::from(part + 1) * 100 / u32::from(parts));
    }
    crypto.ask("work_end", json!({})).await.ok()?;
    Some(counters)
}

/// Central's proof of work for one registration (`docs/registration-abuse.md`),
/// solved off the page's thread: the proof that costs memory in the crypto
/// Worker `crypto`, the older one of a Central from before in the Worker
/// at `pow`. It is handed over no sooner than Central takes it, three
/// seconds after the challenge. `solving` follows the work, and shows it
/// once it takes long.
///
/// # Errors
/// The problem, tagged `closed` while an administrator closed registration.
async fn proof(
    crypto: &Crypto,
    pow: &str,
    invite_code: Option<&str>,
    solving: RwSignal<Solving>,
) -> Result<(String, Vec<u64>), (Problem, Option<&'static str>)> {
    let challenge = challenge(invite_code).await?;
    let answered = super::pow::now_ms();
    solving.set(Solving::Quiet(0));
    leptos::task::spawn_local(async move {
        super::pow::wait(SLOW_MS).await;
        solving.update(|solving| {
            if let Solving::Quiet(percent) = *solving {
                *solving = Solving::Shown(percent);
            }
        });
    });
    let progress = move |percent: u32| {
        solving.update(|solving| match *solving {
            Solving::Quiet(_) => *solving = Solving::Quiet(percent),
            Solving::Shown(_) => *solving = Solving::Shown(percent),
            Solving::Idle => {}
        });
    };
    let (nonce, counters) = match challenge {
        Challenge::Work {
            nonce,
            memory,
            passes,
            parts,
            difficulty,
        } => {
            let price = (memory, passes, parts, difficulty);
            let counters = work(crypto, &nonce, price, progress).await;
            (nonce, counters)
        }
        Challenge::Older {
            nonce,
            parts,
            difficulty,
        } => {
            let (parts, difficulty) = (u32::from(parts), u32::from(difficulty));
            let counters = super::pow::solve(pow, &nonce, parts, difficulty, progress).await;
            (nonce, counters)
        }
    };
    solving.set(Solving::Idle);
    let counters = counters.ok_or((Problem::Failed, None))?;
    let wait = handover_wait(answered, super::pow::now_ms());
    if wait > 0 {
        super::pow::wait(wait).await;
    }
    Ok((nonce, counters))
}

/// How long the page waits before it sends a registration once more that
/// Central answered with `Busy`, in milliseconds.
const BUSY_MS: u32 = 1_500;

async fn complete_enrollment(
    crypto: Crypto,
    identity: Identity,
    ticket: [u8; 32],
    password: &str,
) -> Result<(String, Vec<u8>), (Problem, Option<&'static str>)> {
    let fail = |failure: Failure| (Problem::from(failure), None);
    let start = crypto
        .ask(
            "enrollment_start",
            json!({ "password": password, "account": identity.account, "epoch": identity.epoch }),
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
    Ok((code, recovery))
}

/// Registers an account and signs it in: `Register`, the OPAQUE enrolment
/// with a new master key and recovery code in the Worker, then a sign-in
/// with the same password, which is let go after it.
async fn register(
    worker: &str,
    pow: &str,
    lang: Lang,
    f: RegisterFields,
    solving: RwSignal<Solving>,
) -> Result<(NewCode, SignedIn, bool), (Problem, Option<&'static str>)> {
    let username = f.username.trim().to_ascii_lowercase();
    let (nonce, counters) = {
        let proving = Crypto::start(worker).ok_or((Problem::Failed, None))?;
        proof(&proving, pow, f.invite_code.as_deref(), solving).await?
    };
    let crypto = Crypto::start(worker).ok_or((Problem::Failed, None))?;
    let request = Request::Register {
        username: username.clone(),
        display_name: Some(f.display.clone()),
        nonce,
        counters,
        invite_code: f.invite_code.clone(),
    };
    let answer = match call(&request).await {
        Err(Failure::Busy) => {
            super::pow::wait(BUSY_MS).await;
            call(&request).await
        }
        answer => answer,
    };
    let (identity, ticket): (Identity, [u8; 32]) = match answer {
        Ok(Response::Enrollment { identity, ticket }) if uuid_v7(&identity.account) => {
            (identity, token_bytes(&ticket))
        }
        Ok(_) => return Err((Problem::Failed, None)),
        Err(Failure::Rejected) => {
            if let Some(code) = f.invite_code.as_deref()
                && !matches!(
                    call(&Request::FriendInviteInfo { code: code.to_owned() }).await,
                    Ok(Response::FriendInviteInfo { invite }) if invite.status == FriendInviteStatus::Active
                )
            {
                return Err((Problem::Failed, Some("invite_invalid")));
            }
            return Err((Problem::Failed, Some("refused")));
        }
        Err(Failure::Conflict) => return Err((Problem::Failed, Some("taken"))),
        Err(failure) => return Err((failure.into(), None)),
    };
    let (code, recovery) = complete_enrollment(crypto, identity, ticket, &f.password).await?;
    let signed = sign_in(worker, &username, f.password)
        .await
        .map_err(|problem| (problem, None))?;
    let email_linked = if let Some(address) = f.link_email {
        matches!(
            call(&Request::Email {
                request: EmailRequest::Set {
                    address,
                    lang: lang.code().to_owned(),
                },
            })
            .await,
            Ok(Response::Email { .. })
        )
    } else {
        false
    };
    Ok((
        NewCode {
            code,
            recovery,
            optional: false,
        },
        signed,
        email_linked,
    ))
}

fn invite_banner_view(
    lang: Lang,
    invite_preview: RwSignal<Option<Result<FriendInvitePreview, ()>>>,
) -> impl IntoView {
    move || {
        let Some(Ok(preview)) = invite_preview.get() else {
            return None;
        };
        let handle = person_handle(&preview.inviter);
        let note = preview.note.filter(|n| !n.trim().is_empty());
        Some(view! {
            <div class="invite-banner" role="status">
                <div class="invite-banner-head">
                    {glyph::glyph(glyph::USER_PLUS)}
                    <strong>{words::invite_page_banner(lang, &handle)}</strong>
                </div>
                {note.map(|text| view! { <p class="invite-banner-note"><bdi>{text}</bdi></p> })}
            </div>
        })
    }
}

fn invite_code_field(
    lang: Lang,
    invite_input: RwSignal<String>,
    invite_preview: RwSignal<Option<Result<FriendInvitePreview, ()>>>,
    registration_closed: RwSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="field">
            <label for="register-invite-code">
                {move || if registration_closed.get() {
                    words::register_invite_code_required(lang)
                } else {
                    words::register_invite_code(lang)
                }}
            </label>
            <div class="inset">
                {field::mark(glyph::KEY)}
                <input
                    id="register-invite-code"
                    type="text"
                    autocomplete="off"
                    autocapitalize="none"
                    spellcheck="false"
                    maxlength="256"
                    aria-describedby="register-invite-hint"
                    prop:value=move || invite_input.get()
                    on:input=move |ev| invite_input.set(event_target_value(&ev))
                />
            </div>
            {move || if matches!(invite_preview.get(), Some(Err(()))) {
                view! { <p class="hint auth-invite-invalid" id="register-invite-hint" role="alert">{words::register_invite_invalid(lang)}</p> }.into_any()
            } else {
                view! { <p class="hint" id="register-invite-hint">{words::register_invite_code_hint(lang)}</p> }.into_any()
            }}
        </div>
    }
}

fn invite_email_optin_view(
    lang: Lang,
    invite_preview: RwSignal<Option<Result<FriendInvitePreview, ()>>>,
    link_email: RwSignal<bool>,
) -> impl IntoView {
    move || {
        let Some(Ok(preview)) = invite_preview.get() else {
            return None;
        };
        let addr = preview.invited_email?;
        Some(view! {
            <div class="invite-email-optin">
                <div class="check">
                    <input
                        id="register-link-email"
                        type="checkbox"
                        prop:checked=move || link_email.get()
                        on:change=move |ev| link_email.set(event_target_checked(&ev))
                    />
                    <label for="register-link-email">{words::invite_page_link_email_new(lang, &addr)}</label>
                </div>
                <p class="hint">{words::invite_page_email_verify_hint(lang)}</p>
            </div>
        })
    }
}

fn bind_invite_preview(
    lang: Lang,
    invite_input: RwSignal<String>,
    invite_preview: RwSignal<Option<Result<FriendInvitePreview, ()>>>,
    link_email: RwSignal<bool>,
    registration_closed: RwSignal<bool>,
    next_dest: StoredValue<Option<String>>,
) {
    Effect::new(move |_| {
        let raw = invite_input.get();
        let code = normalize_invite_code(&raw);
        link_email.set(false);
        next_dest.set_value(None);
        if code.is_empty() {
            invite_preview.set(None);
            return;
        }
        if !valid_friend_invite_code(&code) {
            invite_preview.set(Some(Err(())));
            return;
        }
        let expected = code.clone();
        leptos::task::spawn_local(async move {
            let outcome = call(&Request::FriendInviteInfo {
                code: expected.clone(),
            })
            .await;
            if normalize_invite_code(&invite_input.get_untracked()) != expected {
                return;
            }
            match outcome {
                Ok(Response::FriendInviteInfo { invite }) => {
                    if invite.invite_only {
                        registration_closed.set(true);
                    }
                    if invite.status == FriendInviteStatus::Active {
                        let chat = format!(
                            "/{}/account/chat?with={}",
                            lang.code(),
                            tag_text(invite.inviter.tag)
                        );
                        next_dest.set_value(Some(chat));
                        invite_preview.set(Some(Ok(invite)));
                    } else {
                        invite_preview.set(Some(Err(())));
                    }
                }
                Err(Failure::Rejected) => invite_preview.set(Some(Err(()))),
                _ => invite_preview.set(None),
            }
        });
    });
}

/// What the registration asks to be ticked, and the field no person sees
/// (`docs/registration-abuse.md`).
fn register_checks(
    lang: Lang,
    fields: RwSignal<RegisterFields>,
    honey: RwSignal<String>,
    terms: String,
    privacy: String,
) -> impl IntoView {
    view! {
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
        <div class="trap" aria-hidden="true">
            <label for="register-website">{"Website"}</label>
            <input id="register-website" type="text" tabindex="-1" autocomplete="off"
                on:input=move |ev| honey.set(event_target_value(&ev)) />
        </div>
    }
}

#[derive(Clone, Copy)]
struct RegisterTextField {
    id: &'static str,
    label: &'static str,
    hint: &'static str,
    mark: &'static [glyph::Shape],
    auto: &'static str,
    set: fn(&mut RegisterFields, String),
}

fn register_text_field(
    lang: Lang,
    spec: RegisterTextField,
    fields: RwSignal<RegisterFields>,
) -> impl IntoView {
    let hint_id = format!("{}-hint", spec.id);
    let secret = spec.auto == "new-password";
    let shown = RwSignal::new(!secret);
    let eye = secret.then(|| field::eye(lang, shown, spec.id));
    let set = spec.set;
    view! {
        <div class="field">
            <label for=spec.id>{spec.label}</label>
            <div class="inset">
                {field::mark(spec.mark)}
                <input id=spec.id autocomplete=spec.auto autocapitalize="none" spellcheck="false" maxlength="1024" aria-describedby=hint_id.clone()
                    type=move || if shown.get() { "text" } else { "password" }
                    on:input=move |ev| { let value = event_target_value(&ev); fields.update(|f| set(f, value)); } />
                {eye}
            </div>
            <p class="hint" id=hint_id>{spec.hint}</p>
        </div>
    }
}

#[derive(Clone, Copy)]
struct RegisterState {
    lang: Lang,
    step: RwSignal<Step>,
    fields: RwSignal<RegisterFields>,
    error: RwSignal<Option<&'static str>>,
    invite_input: RwSignal<String>,
    invite_preview: RwSignal<Option<Result<FriendInvitePreview, ()>>>,
    link_email: RwSignal<bool>,
    registration_closed: RwSignal<bool>,
    shown: StoredValue<u64>,
    solving: RwSignal<Solving>,
    honey: RwSignal<String>,
}

fn submit_registration(state: RegisterState, worker: &str, pow: &str) {
    let lang = state.lang;
    let mut f = state.fields.get_untracked();
    if let Some(problem) = register_problem(lang, &f) {
        state.error.set(Some(problem));
        return;
    }
    let code = normalize_invite_code(&state.invite_input.get_untracked());
    if !code.is_empty() && !valid_friend_invite_code(&code) {
        state.error.set(Some(words::register_invite_invalid(lang)));
        return;
    }
    if code.is_empty() && state.registration_closed.get_untracked() {
        state.error.set(Some(words::register_closed(lang)));
        return;
    }
    let quick = super::pow::now_ms().saturating_sub(state.shown.get_value()) < 3_000;
    if quick || !state.honey.with_untracked(String::is_empty) {
        state.error.set(Some(words::register_refused(lang)));
        return;
    }
    f.invite_code = (!code.is_empty()).then_some(code);
    f.link_email = state
        .link_email
        .get_untracked()
        .then(|| {
            state
                .invite_preview
                .get_untracked()
                .and_then(Result::ok)
                .and_then(|p| p.invited_email)
        })
        .flatten();
    state.fields.update(|cur| {
        cur.password.clear();
        cur.repeat.clear();
    });
    state.error.set(None);
    state.step.set(Step::Working);
    let (worker, pow) = (worker.to_owned(), pow.to_owned());
    leptos::task::spawn_local(async move {
        let _ = super::push::disable().await;
        match register(&worker, &pow, lang, f, state.solving).await {
            Ok((code, signed, email_linked)) => {
                let mut who = Who::of(signed);
                who.email_linked = email_linked;
                state.step.set(Step::Code(code, who));
            }
            Err((_, Some("closed"))) => {
                state.registration_closed.set(true);
                state.error.set(Some(words::register_closed(lang)));
                state.step.set(Step::Form);
            }
            Err((_, Some("invite_invalid"))) => {
                state.error.set(Some(words::register_invite_invalid(lang)));
                state.step.set(Step::Form);
            }
            Err((_, Some("refused"))) => {
                state.error.set(Some(words::register_refused(lang)));
                state.step.set(Step::Form);
            }
            Err((_, Some("taken"))) => {
                state.error.set(Some(words::register_error_taken(lang)));
                state.step.set(Step::Form);
            }
            Err((problem, _)) => state.step.set(Step::Failed(problem)),
        }
    });
}

#[island]
pub fn Register(lang: Lang, worker: String, pow: String) -> impl IntoView {
    let initial_invite = query_param("invite").map_or_default(|raw| normalize_invite_code(&raw));
    let state = RegisterState {
        lang,
        step: RwSignal::new(Step::Form),
        fields: RwSignal::new(RegisterFields::early()),
        error: RwSignal::new(None),
        invite_input: RwSignal::new(initial_invite),
        invite_preview: RwSignal::new(None),
        link_email: RwSignal::new(false),
        registration_closed: RwSignal::new(false),
        shown: StoredValue::new(0u64),
        solving: RwSignal::new(Solving::Idle),
        honey: RwSignal::new(super::early::typed("register-website")),
    };
    let next_dest = StoredValue::new(None::<String>);
    bind_invite_preview(
        lang,
        state.invite_input,
        state.invite_preview,
        state.link_email,
        state.registration_closed,
        next_dest,
    );
    Effect::new(move |_| state.shown.set_value(super::pow::now_ms()));

    let terms = format!("/{}/terms", lang.code());
    let privacy = format!("/{}/privacy", lang.code());
    let form = move || {
        let (terms, privacy, worker, pow) =
            (terms.clone(), privacy.clone(), worker.clone(), pow.clone());
        let signin = move || {
            let code = normalize_invite_code(&state.invite_input.get());
            if !code.is_empty() && valid_friend_invite_code(&code) {
                let target = format!("/{}/invite/{code}", lang.code());
                format!(
                    "/{}/signin?next={}",
                    lang.code(),
                    percent_encode_component(&target)
                )
            } else {
                format!("/{}/signin", lang.code())
            }
        };
        view! {
            <form class="form" method="post" novalidate=true on:submit=move |event| {
                event.prevent_default();
                submit_registration(state, &worker, &pow);
            }>
                {invite_banner_view(lang, state.invite_preview)}
                {move || state.registration_closed.get().then(|| view! {
                    <p class="notice" role="status">{words::invite_page_invite_only_note(lang)}</p>
                })}
                {register_text_field(lang, RegisterTextField { id: "register-name", label: words::register_name(lang), hint: words::register_name_hint(lang), mark: glyph::USER, auto: "username", set: |f, v| f.username = v }, state.fields)}
                {register_text_field(lang, RegisterTextField { id: "register-display", label: words::register_display(lang), hint: words::register_display_hint(lang), mark: glyph::ID_CARD, auto: "nickname", set: |f, v| f.display = v }, state.fields)}
                {register_text_field(lang, RegisterTextField { id: "register-password", label: words::register_password(lang), hint: words::register_password_hint(lang), mark: glyph::LOCK, auto: "new-password", set: |f, v| f.password = v }, state.fields)}
                {register_text_field(lang, RegisterTextField { id: "register-repeat", label: words::register_repeat(lang), hint: words::register_repeat_hint(lang), mark: glyph::LOCK_KEYHOLE, auto: "new-password", set: |f, v| f.repeat = v }, state.fields)}
                {invite_code_field(lang, state.invite_input, state.invite_preview, state.registration_closed)}
                {invite_email_optin_view(lang, state.invite_preview, state.link_email)}
                {register_checks(lang, state.fields, state.honey, terms, privacy)}
                {auth_alert(state.error)}
                <button type="submit" class="button primary">{words::register_submit(lang)}</button>
                <p class="hint">{move || rich(words::register_have_account(lang), |tag| (tag == "signin").then(signin))}</p>
            </form>
        }
        .into_any()
    };
    steps_view(
        lang,
        "register-steps",
        state.step,
        Some(state.solving),
        next_dest,
        form,
    )
}

/// The proof of work's progress inside `Step::Working`, once it takes long:
/// the share done in words and as a bar, said politely where a screen reader
/// hears it. Always mounted during `Step::Working` so CSS `min-block-size`
/// reserves space and prevents vertical jumps when the bar appears or clears.
fn solving_view(lang: Lang, solving: RwSignal<Solving>) -> impl IntoView {
    let progress = move || match solving.get() {
        Solving::Shown(percent) => Some(view! {
            <p id="register-solving">{words::register_solving(lang, &percent.to_string())}</p>
            <progress
                max="100"
                value=percent.to_string()
                aria-labelledby="register-solving"
            ></progress>
        }),
        Solving::Idle | Solving::Quiet(_) => None,
    };
    view! {
        <div class="solving" aria-live="polite" hidden=move || !matches!(solving.get(), Solving::Shown(_))>
            {progress}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Central counts its three seconds from the challenge it issued; the
    /// page counts from the answer's arrival, which is never earlier.
    #[test]
    fn a_solution_waits_three_seconds_from_the_challenge_s_arrival() {
        // solved at once: the whole time
        assert_eq!(handover_wait(10_000, 10_000), 3_200);
        // A slow connection: asked at 9,000, answered at 10,000, solved at
        // 10,300. From the request it would be 1,900 more, which Central
        // could still find too early; from the answer it is 2,900.
        assert_eq!(handover_wait(10_000, 10_300), 2_900);
        assert_eq!(handover_wait(10_000, 13_199), 1);
        assert_eq!(handover_wait(10_000, 13_200), 0);
        // a long proof waits no more
        assert_eq!(handover_wait(10_000, 140_000), 0);
        // a clock set back meanwhile waits the whole time, never longer
        assert_eq!(handover_wait(10_000, 4_000), 3_200);
        assert_eq!(handover_wait(u64::MAX, 0), 3_200);
    }

    fn fields() -> RegisterFields {
        RegisterFields {
            username: "erika".into(),
            display: "Erika".into(),
            password: "long enough".into(),
            repeat: "long enough".into(),
            age: true,
            consent: true,
            invite_code: None,
            link_email: None,
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
