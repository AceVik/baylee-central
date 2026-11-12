//! The signed-in area (`docs/proposals/frontend-rewrite.md` §6.2): one island
//! per section page, reading the account from Central in the browser. The
//! server renders the frame and the headings; it never sees the session.

use std::fmt::Write as _;

use central_api::email::MailCode;
use central_api::owner::{
    GatewayScopes, MyGateway, OwnerAnswer, OwnerRequest, SessionInfo, SessionKind, token_text,
};
use central_api::wire::{AccountSource, Request, Response};
use leptos::prelude::*;
use serde_json::json;

use super::auth::{Problem, problem_text, sign_in};
use super::central::{Failure, call};
use super::crypto::{Crypto, field_bytes};
use super::{fragment, session};
use crate::data::Section;
use crate::words::{self, Lang};

/// What `AccountSummary` says, as the pages use it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Summary {
    /// The account this browser is signed in as.
    account: [u8; 16],
    username: String,
    display_name: Option<String>,
    tag: Option<u32>,
    moved: bool,
    recovery: bool,
    admin: bool,
    expires_at: i64,
}

impl Summary {
    fn handle(&self) -> Option<String> {
        self.display_name
            .as_deref()
            .zip(self.tag)
            .map(|(name, tag)| central_api::profile::handle(name, tag))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Load {
    Loading,
    SignedOut,
    Ready(Summary),
    Failed(Problem),
}

async fn summary() -> Load {
    if session::remembered().is_none() {
        return Load::SignedOut;
    }
    match call(&Request::AccountSummary).await {
        Ok(Response::AccountSummary {
            identity,
            username,
            display_name,
            tag,
            source,
            recovery,
            admin,
            expires_at,
            ..
        }) => Load::Ready(Summary {
            account: identity.account,
            username,
            display_name,
            tag,
            moved: source == AccountSource::Legacy,
            recovery,
            admin,
            expires_at,
        }),
        Ok(_) => Load::Failed(Problem::Failed),
        Err(Failure::Rejected) => {
            session::forget();
            Load::SignedOut
        }
        Err(failure) => Load::Failed(failure.into()),
    }
}

/// A Unix time as the page's language writes a date and time, in UTC.
pub fn when(lang: Lang, seconds: i64) -> String {
    let rest = seconds.rem_euclid(86_400);
    let time = format!("{:02}:{:02}", rest / 3600, rest % 3600 / 60);
    format!("{}, {time} UTC", day(lang, seconds))
}

/// The day of a Unix time as the page's language writes a date, in UTC.
pub fn day(lang: Lang, seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    let date = format!("{year:04}-{month:02}-{day:02}");
    crate::format::date(lang, &date)
}

/// Signs out here: Central ends the session and clears the cookie, the
/// browser forgets the session's end and the chat's kept identity.
async fn sign_out(worker: &str, lang: Lang) {
    // this device stops getting the account's notifications first
    let _ = super::push::disable().await;
    let _ = call(&Request::Logout).await;
    session::forget();
    if let Some(crypto) = Crypto::start(worker) {
        let _ = crypto.ask("forget", json!({})).await;
    }
    session::go(&format!("/{}/", lang.code()));
}

/// The password form that signs in again before a change that needs a
/// fresh sign-in (making a token, exporting).
fn reauth(lang: Lang, worker: String, done: Callback<()>) -> impl IntoView {
    // only an action asks for it, so it takes the focus
    super::focus::when_shown("#reauth-name");
    let password = RwSignal::new(String::new());
    let error = RwSignal::new(None::<&'static str>);
    let busy = RwSignal::new(false);
    let username = RwSignal::new(String::new());
    let submit = move |event: leptos::ev::SubmitEvent| {
        event.prevent_default();
        let (name, secret, worker) = (
            username.get_untracked(),
            password.get_untracked(),
            worker.clone(),
        );
        password.set(String::new());
        busy.set(true);
        leptos::task::spawn_local(async move {
            match sign_in(&worker, &name, secret).await {
                Ok(_) => {
                    error.set(None);
                    done.run(());
                }
                Err(problem) => error.set(Some(problem_text(lang, problem))),
            }
            busy.set(false);
        });
    };
    view! {
        <form class="form reauth" method="post" novalidate=true on:submit=submit>
            <p>{words::reauth_lead(lang)}</p>
            <div class="field">
                <label for="reauth-name">{words::signin_name(lang)}</label>
                <input id="reauth-name" type="text" autocomplete="username" autocapitalize="none" on:input=move |ev| username.set(event_target_value(&ev)) />
            </div>
            <div class="field">
                <label for="reauth-password">{words::signin_password(lang)}</label>
                <input id="reauth-password" type="password" autocomplete="current-password"
                    on:input=move |ev| password.set(event_target_value(&ev)) prop:value=move || password.get() />
            </div>
            <p class="error" role="alert">{move || error.get()}</p>
            <button type="submit" class="button primary" aria-disabled=move || busy.get().then_some("true")>{words::reauth_submit(lang)}</button>
        </form>
    }
}

/// Open registration's brake (`docs/registration-abuse.md`), for
/// administrators: closed during an attack, opened again by hand; a change
/// needs a sign-in under 30 minutes old, so a stale one signs in again here.
fn registration_switch(lang: Lang, worker: String) -> AnyView {
    use central_api::admin::{AdminAnswer, AdminRequest};

    let open = RwSignal::new(None::<bool>);
    let busy = RwSignal::new(false);
    let again = RwSignal::new(false);
    let failed = RwSignal::new(false);
    let read = move || {
        leptos::task::spawn_local(async move {
            let request = Request::Admin {
                request: AdminRequest::Registration,
            };
            match call(&request).await {
                Ok(Response::Admin {
                    answer: AdminAnswer::Registration { open: now, .. },
                }) => open.set(Some(now)),
                _ => failed.set(true),
            }
        });
    };
    Effect::new(move |_| read());
    let change = move || {
        let Some(now) = open.get_untracked() else {
            return;
        };
        busy.set(true);
        failed.set(false);
        leptos::task::spawn_local(async move {
            let request = Request::Admin {
                request: AdminRequest::SetRegistration { open: !now },
            };
            match call(&request).await {
                Ok(Response::Done) => open.set(Some(!now)),
                Err(Failure::Rejected) => again.set(true),
                _ => failed.set(true),
            }
            busy.set(false);
        });
    };
    let signed_again = Callback::new(move |()| {
        again.set(false);
        super::focus::after_render("#registration-switch");
        change();
    });
    view! {
        <section class="card" aria-labelledby="registration-title">
            <h2 id="registration-title">{words::admin_registration(lang)}</h2>
            <p role="status">{move || match open.get() {
                Some(true) => words::admin_registration_open(lang),
                Some(false) => words::admin_registration_closed(lang),
                None => words::account_loading(lang),
            }}</p>
            <p class="meta">{words::admin_registration_lead(lang)}</p>
            {move || failed.get().then(|| view! { <p class="error" role="alert">{words::push_failed(lang)}</p> })}
            {move || {
                if again.get() {
                    reauth(lang, worker.clone(), signed_again).into_any()
                } else {
                    // the label follows the state; the button stays, and the focus on it
                    let label = move || if open.get() == Some(false) {
                        words::admin_registration_reopen(lang)
                    } else {
                        words::admin_registration_close(lang)
                    };
                    view! {
                        <p><button type="button" class="button" id="registration-switch" aria-disabled=move || (busy.get() || open.get().is_none()).then_some("true")
                            on:click=move |_| if !busy.get_untracked() { change() }>{label}</button></p>
                    }
                    .into_any()
                }
            }}
        </section>
    }
    .into_any()
}

fn home(lang: Lang, summary: &Summary, worker: &str) -> AnyView {
    let code = lang.code();
    let needs = (!summary.recovery).then(|| {
        view! { <li><a href=format!("/{code}/account/profile")>{words::account_needs_code(lang)}</a></li> }
    });
    let name = summary.display_name.is_none().then(|| {
        view! { <li><a href=format!("/{code}/account/profile")>{words::account_needs_name(lang)}</a></li> }
    });
    let attention = (needs.is_some() || name.is_some()).then(|| {
        view! {
            <section class="card needs" aria-labelledby="needs-title">
                <h2 id="needs-title">{words::account_needs_title(lang)}</h2>
                <ul>{needs}{name}</ul>
            </section>
        }
    });
    let links = [
        (
            "profile",
            words::account_profile(lang),
            words::account_profile_lead(lang),
        ),
        (
            "security",
            words::account_security(lang),
            words::account_security_lead(lang),
        ),
        (
            "privacy",
            words::account_privacy(lang),
            words::account_privacy_lead(lang),
        ),
        (
            "gateways",
            words::account_gateways(lang),
            words::account_gateways_lead(lang),
        ),
    ]
    .map(|(slug, title, lead)| {
        view! {
            <li class="card">
                <h2><a href=format!("/{code}/account/{slug}")>{title}</a></h2>
                <p>{lead}</p>
            </li>
        }
    });
    // only an administrator is led to the administration: its overview,
    // and the page that makes reset codes
    let admin = summary.admin.then(|| {
        view! {
            {registration_switch(lang, worker.to_owned())}
            <p class="actions">
                <a class="button" href=format!("/{code}/account/admin")>{words::admin_reset_title(lang)}</a>
                <a class="button" href=format!("/{code}/account/admin/overview")>{words::account_admin(lang)}</a>
            </p>
        }
    });
    view! {
        <div class="account-who">
            <p class="handle" translate="no">{summary.handle().unwrap_or_else(|| words::account_unnamed(lang).to_owned())}</p>
            <p class="meta">{words::account_signin_name(lang, &summary.username)}</p>
            <p class="meta">{words::account_until(lang, &when(lang, summary.expires_at))}</p>
        </div>
        {attention}
        <ul class="cards-grid tiles">{links}</ul>
        {admin}
    }
    .into_any()
}

fn profile(
    lang: Lang,
    worker: &str,
    summary: &Summary,
    reload: Callback<()>,
    focus_next: StoredValue<Option<&'static str>>,
) -> AnyView {
    let chosen = RwSignal::new(String::new());
    let error = RwSignal::new(None::<&'static str>);
    let choose = summary.display_name.is_none().then(|| {
        let save = move |event: leptos::ev::SubmitEvent| {
            event.prevent_default();
            let name = chosen.get_untracked();
            if !central_api::profile::choosable(&name) {
                error.set(Some(words::register_error_display(lang)));
                return;
            }
            leptos::task::spawn_local(async move {
                match call(&Request::SetDisplayName { display_name: name }).await {
                    Ok(_) => {
                        // the form goes with the reload; the new handle takes the focus
                        focus_next.set_value(Some("#profile-who"));
                        reload.run(());
                    }
                    Err(Failure::Conflict) => error.set(Some(words::profile_name_taken(lang))),
                    Err(failure) => error.set(Some(problem_text(lang, failure.into()))),
                }
            });
        };
        view! {
            <form class="form card" method="post" novalidate=true on:submit=save>
                <h2>{words::profile_choose_title(lang)}</h2>
                <div class="field">
                    <label for="profile-name">{words::register_display(lang)}</label>
                    <input id="profile-name" type="text" maxlength="16" aria-describedby="profile-name-hint" on:input=move |ev| chosen.set(event_target_value(&ev)) />
                    <p class="hint" id="profile-name-hint">{words::register_display_hint(lang)}</p>
                </div>
                <p class="error" role="alert">{move || error.get()}</p>
                <button type="submit" class="button primary">{words::profile_choose_submit(lang)}</button>
            </form>
        }
    });
    let code = if summary.recovery {
        view! { <p>{words::profile_code_kept(lang)}</p> }.into_any()
    } else {
        let asking = RwSignal::new(false);
        let worker = worker.to_owned();
        view! {
            <p>{words::profile_code_missing(lang)}</p>
            <Show when=move || asking.get() fallback=move || view! {
                <button type="button" class="button primary" on:click=move |_| asking.set(true)>{words::profile_code_make(lang)}</button>
            }>
                {
                    let worker = worker.clone();
                    let lang_code = lang.code();
                    reauth(lang, worker, Callback::new(move |()| session::go(&format!("/{lang_code}/account/profile"))))
                }
            </Show>
        }
        .into_any()
    };
    view! {
        {choose}
        <section class="card" aria-labelledby="profile-who">
            <h2 id="profile-who" tabindex="-1">{words::profile_who_title(lang)}</h2>
            <dl class="facts">
                <dt>{words::profile_handle(lang)}</dt>
                <dd translate="no">{summary.handle().unwrap_or_else(|| "–".to_owned())}</dd>
                <dt>{words::profile_signin_name(lang)}</dt>
                <dd translate="no">{summary.username.clone()}</dd>
                <dt>{words::profile_source(lang)}</dt>
                <dd>{if summary.moved { words::profile_source_moved(lang) } else { words::profile_source_registered(lang) }}</dd>
            </dl>
        </section>
        <section class="card" aria-labelledby="profile-code">
            <h2 id="profile-code">{words::profile_code_title(lang)}</h2>
            {code}
        </section>
    }
    .into_any()
}

fn session_row(lang: Lang, info: &SessionInfo) -> impl IntoView + use<> {
    let kind = match info.kind {
        SessionKind::Browser => words::security_kind_browser(lang),
        SessionKind::Native => words::security_kind_client(lang),
    };
    let current = info
        .current
        .then(|| view! { <span class="pill accent">{words::security_this(lang)}</span> });
    let gateways = (!info.gateways.is_empty()).then(|| {
        let list = info
            .gateways
            .iter()
            .map(|origin| view! { <li><code translate="no">{origin.clone()}</code></li> })
            .collect_view();
        view! { <dt>{words::security_gateways(lang)}</dt><dd><ul>{list}</ul></dd> }
    });
    view! {
        <li class="card">
            <h3>{kind} {current}</h3>
            <dl class="facts">
                <dt>{words::security_signed_in(lang)}</dt>
                <dd>{when(lang, info.signed_in_at)}</dd>
                <dt>{words::security_ends(lang)}</dt>
                <dd>{when(lang, info.expires_at)}</dd>
                {gateways}
            </dl>
        </li>
    }
}

/// The sessions, the password's change (`password::change_form`) and the
/// account's e-mail address (`email::section`), which takes the code the
/// confirmation mail's link `handed` over, once. `signed_out` reloads the
/// area when one of them finds no session.
fn security(
    lang: Lang,
    worker: String,
    username: String,
    signed_out: Callback<()>,
    handed: StoredValue<Option<MailCode>>,
) -> AnyView {
    let sessions = RwSignal::new(None::<Result<Vec<SessionInfo>, Problem>>);
    let notice = RwSignal::new(None::<&'static str>);
    let load = move || {
        leptos::task::spawn_local(async move {
            let found = match call(&Request::Owner {
                request: OwnerRequest::Sessions,
            })
            .await
            {
                Ok(Response::Owner {
                    answer: OwnerAnswer::Sessions { sessions },
                }) => Ok(sessions),
                Ok(_) => Err(Problem::Failed),
                Err(failure) => Err(failure.into()),
            };
            sessions.set(Some(found));
        });
    };
    Effect::new(move |_| load());
    let end_client = move |_| {
        leptos::task::spawn_local(async move {
            match call(&Request::Owner {
                request: OwnerRequest::EndClientSession,
            })
            .await
            {
                Ok(_) => {
                    notice.set(Some(words::security_client_ended(lang)));
                    load();
                }
                Err(failure) => notice.set(Some(problem_text(lang, failure.into()))),
            }
        });
    };
    let leave = {
        let worker = worker.clone();
        move |_| {
            let worker = worker.clone();
            leptos::task::spawn_local(async move { sign_out(&worker, lang).await });
        }
    };
    // a change ends every other session: the list above is another then
    let changed = Callback::new(move |()| load());
    // naming an address signs in once more: this session is a newer one then
    let email = super::email::section(
        lang,
        worker.clone(),
        username.clone(),
        super::email::Begin::Read,
        changed,
        signed_out,
        handed,
    );
    view! {
        <section aria-labelledby="sessions-title">
            <h2 id="sessions-title">{words::security_sessions(lang)}</h2>
            <p>{words::security_lead(lang)}</p>
            {move || sessions.with(|found| match found {
                None => view! { <p role="status" class="meta">{words::account_loading(lang)}</p> }.into_any(),
                Some(Err(problem)) => view! { <p class="error" role="alert">{problem_text(lang, *problem)}</p> }.into_any(),
                Some(Ok(list)) => {
                    let rows = list.iter().map(|info| session_row(lang, info)).collect_view();
                    view! { <ul class="sessions">{rows}</ul> }.into_any()
                }
            })}
            <p class="meta" role="status">{move || notice.get()}</p>
            <p class="actions">
                <button type="button" class="button danger" on:click=end_client>{words::security_end_client(lang)}</button>
                <button type="button" class="button" on:click=leave>{words::security_sign_out(lang)}</button>
            </p>
        </section>
        <section class="card" aria-labelledby="password-title">
            <h2 id="password-title">{words::security_password_title(lang)}</h2>
            <p>{words::security_password_lead(lang)}</p>
            {super::password::change_form(lang, worker, username, changed, signed_out)}
        </section>
        {email}
    }
    .into_any()
}

/// The export as a JSON file the browser saves.
fn download_link(lang: Lang, json: &str) -> impl IntoView + use<> {
    let mut href = String::from("data:application/json;charset=utf-8,");
    for byte in json.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.~:,{}[]\"".contains(&byte) {
            href.push(char::from(byte));
        } else {
            let _ = write!(href, "%{byte:02X}");
        }
    }
    view! { <a class="button primary" id="export-save" href=href download="baylee-account.json">{words::privacy_export_save(lang)}</a> }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Deletion {
    Idle,
    Asking,
    Proven { gateways: Vec<String> },
    Deleted,
}

/// The export, and beside it in the same file what Central holds of the
/// account's e-mail address (`email::export_file`): the export's own
/// answer does not carry it. Where the address cannot be read the file is
/// the export alone, and the page says that the address is not in it.
fn export_view(lang: Lang, worker: String) -> impl IntoView {
    let export = RwSignal::new(None::<Result<String, Problem>>);
    let without_email = RwSignal::new(false);
    let needs_reauth = RwSignal::new(false);
    let run_export = move || {
        leptos::task::spawn_local(async move {
            match call(&Request::AccountExport).await {
                Ok(Response::AccountExport { export: data }) => {
                    // asked once the export answered: a refusal here is not the export's
                    let email = super::email::for_export().await;
                    without_email.set(email.is_none());
                    export.set(Some(
                        super::email::export_file(&data, email.as_ref()).ok_or(Problem::Failed),
                    ));
                }
                Ok(_) => export.set(Some(Err(Problem::Failed))),
                Err(Failure::Rejected) => needs_reauth.set(true),
                Err(failure) => export.set(Some(Err(failure.into()))),
            }
        });
    };
    view! {
        <section class="card" aria-labelledby="export-title">
            <h2 id="export-title">{words::privacy_export_title(lang)}</h2>
            <p>{words::privacy_export_lead(lang)}</p>
            {move || match export.get() {
                Some(Ok(json)) => {
                    super::focus::when_shown("#export-save");
                    let lacks = without_email.get().then(|| {
                        view! { <p class="meta" id="export-no-email">{words::privacy_export_no_email(lang)}</p> }
                    });
                    view! { {download_link(lang, &json)}{lacks} }.into_any()
                }
                Some(Err(problem)) => {
                    super::focus::when_shown("#export-error");
                    view! { <p class="error" id="export-error" role="alert" tabindex="-1">{problem_text(lang, problem)}</p> }.into_any()
                }
                None => view! { <button type="button" class="button" on:click=move |_| run_export()>{words::privacy_export_make(lang)}</button> }.into_any(),
            }}
            <Show when=move || needs_reauth.get()>
                {reauth(lang, worker.clone(), Callback::new(move |()| { needs_reauth.set(false); run_export(); }))}
            </Show>
        </section>
    }
}

fn deletion_view(lang: Lang, worker: String) -> impl IntoView {
    let deletion = RwSignal::new(Deletion::Idle);
    let password = RwSignal::new(String::new());
    let error = RwSignal::new(None::<&'static str>);
    let proof = StoredValue::new_local(None::<(central_api::Token, Vec<u8>)>);
    let worker_delete = worker.clone();
    let prove = move |event: leptos::ev::SubmitEvent| {
        event.prevent_default();
        let (secret, worker) = (password.get_untracked(), worker_delete.clone());
        password.set(String::new());
        leptos::task::spawn_local(async move {
            match prove_deletion(&worker, secret).await {
                Ok((attempt, kept, gateways)) => {
                    proof.set_value(Some((attempt, kept)));
                    error.set(None);
                    deletion.set(Deletion::Proven { gateways });
                }
                Err(problem) => error.set(Some(problem_text(lang, problem))),
            }
        });
    };
    let delete = move |_| {
        let Some((attempt, kept)) = proof.try_update_value(Option::take).flatten() else {
            return;
        };
        let worker = worker.clone();
        leptos::task::spawn_local(async move {
            match call(&Request::DeleteAccountFinish {
                attempt,
                proof: kept,
            })
            .await
            {
                Ok(_) => {
                    session::forget();
                    if let Some(crypto) = Crypto::start(&worker) {
                        let _ = crypto.ask("forget", json!({})).await;
                    }
                    deletion.set(Deletion::Deleted);
                }
                Err(failure) => {
                    error.set(Some(problem_text(lang, failure.into())));
                    deletion.set(Deletion::Idle);
                    super::focus::after_render("#delete-start");
                }
            }
        });
    };
    let code = lang.code();
    view! {
        <section class="card danger-zone" aria-labelledby="delete-title">
            <h2 id="delete-title">{words::privacy_delete_title(lang)}</h2>
            <p>{words::privacy_delete_lead(lang)}</p>
            {move || match deletion.get() {
                Deletion::Idle => view! {
                    <button type="button" class="button danger" id="delete-start" on:click=move |_| deletion.set(Deletion::Asking)>{words::privacy_delete_start(lang)}</button>
                }.into_any(),
                Deletion::Asking => {
                    super::focus::when_shown("#delete-password");
                    view! {
                        <form class="form" method="post" novalidate=true on:submit=prove.clone()>
                            <div class="field">
                                <label for="delete-password">{words::privacy_delete_password(lang)}</label>
                                <input id="delete-password" type="password" autocomplete="current-password" on:input=move |ev| password.set(event_target_value(&ev)) />
                            </div>
                            <button type="submit" class="button danger">{words::privacy_delete_prove(lang)}</button>
                        </form>
                    }.into_any()
                }
                Deletion::Proven { gateways } => {
                    let owned = (!gateways.is_empty()).then(|| {
                        let list = gateways.into_iter().map(|origin| view! { <li><code translate="no">{origin}</code></li> }).collect_view();
                        view! { <p>{words::privacy_delete_gateways(lang)}</p><ul>{list}</ul> }
                    });
                    // the last word before the end, not the button that ends it
                    super::focus::when_shown("#delete-last");
                    view! {
                        <p class="notice" id="delete-last" role="alert" tabindex="-1">{words::privacy_delete_last(lang)}</p>
                        {owned}
                        <p class="actions">
                            <button type="button" class="button danger" on:click=delete.clone()>{words::privacy_delete_confirm(lang)}</button>
                            <button type="button" class="button quiet" on:click=move |_| {
                                deletion.set(Deletion::Idle);
                                super::focus::after_render("#delete-start");
                            }>{words::privacy_delete_cancel(lang)}</button>
                        </p>
                    }.into_any()
                }
                Deletion::Deleted => {
                    super::focus::when_shown("#deleted");
                    view! {
                        <p role="status" id="deleted" tabindex="-1">{words::privacy_deleted(lang)}</p>
                        <p><a class="button" href=format!("/{code}/")>{words::error_home(lang)}</a></p>
                    }.into_any()
                }
            }}
            <p class="error" role="alert">{move || error.get()}</p>
        </section>
    }
}

fn privacy(lang: Lang, worker: String) -> AnyView {
    view! {
        {export_view(lang, worker.clone())}
        {deletion_view(lang, worker)}
    }
    .into_any()
}

/// Proves the password for a deletion: KE1 in a fresh Worker,
/// `DeleteAccountStart`, KE3 back. The attempt lives 60 seconds.
async fn prove_deletion(
    worker: &str,
    password: String,
) -> Result<(central_api::Token, Vec<u8>, Vec<String>), Problem> {
    let crypto = Crypto::start(worker).ok_or(Problem::Failed)?;
    let Response::AccountSummary { identity, .. } = call(&Request::AccountSummary).await? else {
        return Err(Problem::Failed);
    };
    let start = crypto
        .ask(
            "login_start",
            json!({ "password": password, "account": identity.account, "epoch": identity.epoch }),
        )
        .await?;
    drop(password);
    let message = field_bytes(&start, "message", Some(96))?;
    let Response::DeleteChallenge {
        attempt,
        login,
        gateways,
    } = call(&Request::DeleteAccountStart { login: message }).await?
    else {
        return Err(Problem::Failed);
    };
    let finish = crypto
        .ask("login_finish", json!({ "response": login }))
        .await?;
    let proof = field_bytes(&finish, "message", Some(64))?;
    Ok((
        attempt,
        proof,
        gateways.into_iter().map(|gateway| gateway.origin).collect(),
    ))
}

/// A gateway's part of element ids: its id in hex, unique on the page.
fn gateway_key(id: &[u8; 16]) -> String {
    id.iter().fold(String::with_capacity(32), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

fn gateway_card(
    lang: Lang,
    gateway: &MyGateway,
    revoke: Callback<([u8; 16], [u8; 16])>,
    mint: Callback<([u8; 16], String, u16, GatewayScopes)>,
) -> impl IntoView + use<> {
    let state = match gateway.state {
        central_api::gateway::GatewayState::Approved => words::gateways_approved(lang),
        central_api::gateway::GatewayState::Pending => words::gateways_pending(lang),
        central_api::gateway::GatewayState::Disabled => words::gateways_disabled(lang),
    };
    let gateway_id = gateway.id;
    let tokens = gateway
        .tokens
        .iter()
        .map(|token| {
            let id = token.id;
            let live = token.revoked_at.is_none();
            let what = match (token.scopes.sso, token.scopes.listing) {
                (true, true) => words::tokens_both(lang),
                (true, false) => words::tokens_sso(lang),
                _ => words::tokens_listing(lang),
            };
            let action = live.then(|| view! {
                <button type="button" class="button danger" on:click=move |_| revoke.run((gateway_id, id))>{words::tokens_revoke(lang)}</button>
            });
            view! {
                <li class="token">
                    <p><strong>{token.name.clone()}</strong> " · " {what}</p>
                    <p class="meta">
                        {if live { words::tokens_until(lang, &when(lang, token.expires_at)) } else { words::tokens_revoked(lang).to_owned() }}
                    </p>
                    {action}
                </li>
            }
        })
        .collect_view();
    let id = gateway.id;
    let name = RwSignal::new(String::new());
    let days = RwSignal::new(90_u16);
    let sso = RwSignal::new(true);
    let listing = RwSignal::new(true);
    let make = move |event: leptos::ev::SubmitEvent| {
        event.prevent_default();
        mint.run((
            id,
            name.get_untracked(),
            days.get_untracked(),
            GatewayScopes {
                sso: sso.get_untracked(),
                listing: listing.get_untracked(),
            },
        ));
    };
    let form_id = format!("token-{}", gateway_key(&gateway.id));
    view! {
        <article class="card gateway-own">
            <h3 id=format!("{form_id}-title") tabindex="-1"><bdi>{gateway.metadata.name.clone()}</bdi></h3>
            <p class="pills"><span class="pill">{state}</span>{gateway.online.then(|| view! { <span class="pill ok">{words::gws_online(lang)}</span> })}</p>
            <p><code class="address" translate="no">{gateway.origin.clone()}</code></p>
            <h4>{words::tokens_title(lang)}</h4>
            <ul class="tokens">{tokens}</ul>
            <form class="form" method="post" novalidate=true on:submit=make>
                <div class="field">
                    <label for=format!("{form_id}-name")>{words::tokens_name(lang)}</label>
                    <input id=format!("{form_id}-name") type="text" maxlength="64" on:input=move |ev| name.set(event_target_value(&ev)) />
                </div>
                <div class="field">
                    <label for=format!("{form_id}-days")>{words::tokens_days(lang)}</label>
                    <select id=format!("{form_id}-days") on:change=move |ev| days.set(event_target_value(&ev).parse().unwrap_or(90))>
                        <option value="30">{"30"}</option>
                        <option value="90" selected=true>"90"</option>
                        <option value="180">{"180"}</option>
                        <option value="365">{"365"}</option>
                    </select>
                </div>
                <div class="check">
                    <input id=format!("{form_id}-sso") type="checkbox" checked=true on:change=move |ev| sso.set(event_target_checked(&ev)) />
                    <label for=format!("{form_id}-sso")>{words::tokens_sso(lang)}</label>
                </div>
                <div class="check">
                    <input id=format!("{form_id}-listing") type="checkbox" checked=true on:change=move |ev| listing.set(event_target_checked(&ev)) />
                    <label for=format!("{form_id}-listing")>{words::tokens_listing(lang)}</label>
                </div>
                <button type="submit" class="button">{words::tokens_make(lang)}</button>
            </form>
        </article>
    }
}

/// Registering a gateway: its origin and its name.
fn add_form(
    lang: Lang,
    notice: RwSignal<Option<&'static str>>,
    reload: Callback<()>,
) -> impl IntoView {
    let origin = RwSignal::new(String::new());
    let label = RwSignal::new(String::new());
    let add = move |event: leptos::ev::SubmitEvent| {
        event.prevent_default();
        let (origin_value, name) = (
            origin.get_untracked().trim().to_owned(),
            label.get_untracked().trim().to_owned(),
        );
        if central_api::gateway::validate_origin(&origin_value, false).is_err() || name.is_empty() {
            notice.set(Some(words::gateways_invalid(lang)));
            return;
        }
        leptos::task::spawn_local(async move {
            match call(&Request::Owner {
                request: OwnerRequest::AddGateway {
                    origin: origin_value,
                    name,
                },
            })
            .await
            {
                Ok(_) => {
                    notice.set(Some(words::gateways_added(lang)));
                    reload.run(());
                }
                Err(Failure::Conflict) => notice.set(Some(words::gateways_taken(lang))),
                Err(failure) => notice.set(Some(problem_text(lang, failure.into()))),
            }
        });
    };
    view! {
        <form class="form card" method="post" novalidate=true on:submit=add>
            <h2>{words::gateways_add_title(lang)}</h2>
            <div class="field">
                <label for="gateway-origin">{words::gateways_origin(lang)}</label>
                <input id="gateway-origin" type="url" inputmode="url" placeholder="https://" aria-describedby="gateway-origin-hint" on:input=move |ev| origin.set(event_target_value(&ev)) />
                <p class="hint" id="gateway-origin-hint">{words::gateways_origin_hint(lang)}</p>
            </div>
            <div class="field">
                <label for="gateway-name">{words::gateways_name(lang)}</label>
                <input id="gateway-name" type="text" maxlength="80" on:input=move |ev| label.set(event_target_value(&ev)) />
            </div>
            <button type="submit" class="button">{words::gateways_add(lang)}</button>
        </form>
    }
}

/// A new token, shown once; kept, the focus goes back to its gateway.
fn token_box(
    lang: Lang,
    shown: RwSignal<Option<String>>,
    minting: StoredValue<Option<[u8; 16]>>,
) -> impl IntoView {
    view! {
        <Show when=move || shown.with(Option::is_some)>
            <section class="card secret-box" role="alert" aria-labelledby="token-shown">
                <h2 id="token-shown" tabindex="-1">{words::tokens_shown_title(lang)}</h2>
                <p class="eyebrow">{words::code_shown_once(lang)}</p>
                <p class="secret" translate="no"><code>{move || shown.get().unwrap_or_default()}</code></p>
                <button type="button" class="button primary" on:click=move |_| {
                    shown.set(None);
                    if let Some(gateway) = minting.get_value() {
                        super::focus::after_render(format!("#token-{}-title", gateway_key(&gateway)));
                    }
                }>{words::tokens_kept(lang)}</button>
            </section>
        </Show>
    }
}

fn gateways(lang: Lang, worker: String) -> AnyView {
    let list = RwSignal::new(None::<Result<Vec<MyGateway>, Problem>>);
    let notice = RwSignal::new(None::<&'static str>);
    let shown = RwSignal::new(None::<String>);
    let needs_reauth = RwSignal::new(false);
    // the gateway a token is being made for, to come back to
    let minting = StoredValue::new(None::<[u8; 16]>);
    // A reload draws the cards anew, the focused control among them:
    // `then` names where the focus goes once they are drawn.
    let load = move |then: Option<String>| {
        leptos::task::spawn_local(async move {
            let found = match call(&Request::Owner {
                request: OwnerRequest::Gateways,
            })
            .await
            {
                Ok(Response::Owner {
                    answer: OwnerAnswer::Gateways { gateways },
                }) => Ok(gateways),
                Ok(_) => Err(Problem::Failed),
                Err(failure) => Err(failure.into()),
            };
            list.set(Some(found));
            if let Some(then) = then {
                super::focus::after_render(then);
            }
        });
    };
    Effect::new(move |_| load(None));
    let revoke = Callback::new(move |(gateway, token): ([u8; 16], [u8; 16])| {
        leptos::task::spawn_local(async move {
            match call(&Request::Owner {
                request: OwnerRequest::RevokeToken { token },
            })
            .await
            {
                Ok(_) => {
                    notice.set(Some(words::tokens_revoked_now(lang)));
                    load(Some(format!("#token-{}-title", gateway_key(&gateway))));
                }
                Err(failure) => notice.set(Some(problem_text(lang, failure.into()))),
            }
        });
    });
    let mint = Callback::new(
        move |(gateway, name, days, scopes): ([u8; 16], String, u16, GatewayScopes)| {
            if central_api::owner::token_name(&name).is_err() || !(scopes.sso || scopes.listing) {
                notice.set(Some(words::tokens_invalid(lang)));
                return;
            }
            minting.set_value(Some(gateway));
            leptos::task::spawn_local(async move {
                match call(&Request::Owner {
                    request: OwnerRequest::CreateToken {
                        gateway,
                        name,
                        scopes,
                        days,
                    },
                })
                .await
                {
                    Ok(Response::Owner {
                        answer: OwnerAnswer::TokenCreated { token, .. },
                    }) => {
                        shown.set(Some(token_text(&token).to_string()));
                        super::focus::after_render("#token-shown");
                        load(None);
                    }
                    Ok(_) => notice.set(Some(problem_text(lang, Problem::Failed))),
                    Err(Failure::Rejected) => needs_reauth.set(true),
                    Err(failure) => notice.set(Some(problem_text(lang, failure.into()))),
                }
            });
        },
    );
    view! {
        {token_box(lang, shown, minting)}
        <Show when=move || needs_reauth.get()>
            {reauth(lang, worker.clone(), Callback::new(move |()| {
                needs_reauth.set(false);
                notice.set(Some(words::tokens_again(lang)));
                // signed in again: back to the form that asks to make it again
                if let Some(gateway) = minting.get_value() {
                    super::focus::after_render(format!("#token-{}-name", gateway_key(&gateway)));
                }
            }))}
        </Show>
        <p class="meta" role="status">{move || notice.get()}</p>
        {move || list.with(|found| match found {
            None => view! { <p role="status" class="meta">{words::account_loading(lang)}</p> }.into_any(),
            Some(Err(problem)) => view! { <p class="error" role="alert">{problem_text(lang, *problem)}</p> }.into_any(),
            Some(Ok(all)) if all.is_empty() => view! { <p>{words::gateways_none(lang)}</p> }.into_any(),
            Some(Ok(all)) => all.iter().map(|gateway| gateway_card(lang, gateway, revoke, mint)).collect_view().into_any(),
        })}
        {add_form(lang, notice, Callback::new(move |()| load(None)))}
    }
    .into_any()
}

/// The account's categories, each a switch.
fn push_switches(
    lang: Lang,
    admin: bool,
    now: RwSignal<Option<Result<super::push::Now, Problem>>>,
    busy: RwSignal<bool>,
    error: RwSignal<Option<&'static str>>,
) -> AnyView {
    use super::push;

    let switches = push::categories(admin)
        .into_iter()
        .map(|category| {
            let id = format!("push-{}", push::bit(category));
            let on = move || now.with(|n| matches!(n, Some(Ok(n)) if n.enabled & push::bit(category) != 0));
            let change = move |event: leptos::ev::Event| {
                let wanted = event_target_checked(&event);
                let Some(Ok(current)) = now.get_untracked() else {
                    return;
                };
                if busy.get_untracked() {
                    // one change at a time: this one is undone
                    event_target::<leptos::web_sys::HtmlInputElement>(&event).set_checked(!wanted);
                    return;
                }
                busy.set(true);
                error.set(None);
                leptos::task::spawn_local(async move {
                    match push::set(&current, category, wanted).await {
                        Ok(enabled) => now.update(|n| {
                            if let Some(Ok(n)) = n {
                                n.enabled = enabled;
                            }
                        }),
                        Err(_) => error.set(Some(words::push_failed(lang))),
                    }
                    busy.set(false);
                });
            };
            let name = match category {
                central_api::notifications::Category::Chat => words::push_chat(lang),
                central_api::notifications::Category::Contacts => words::push_contacts(lang),
                central_api::notifications::Category::Updates => words::push_updates(lang),
                central_api::notifications::Category::Feedback => words::push_feedback(lang),
                central_api::notifications::Category::Gateways => words::push_gateways(lang),
            };
            view! {
                <div class="check">
                    <input id=id.clone() type="checkbox" role="switch" prop:checked=on on:change=change
                        aria-disabled=move || busy.get().then_some("true")
                        disabled=move || now.with(|n| !matches!(n, Some(Ok(_)))) />
                    <label for=id>{name}</label>
                </div>
            }
        })
        .collect_view();
    switches.into_any()
}

/// Web Push on this device and the account's categories.
fn notifications(lang: Lang, admin: bool) -> AnyView {
    use super::push::{self, Refusal};

    let now = RwSignal::new(None::<Result<push::Now, Problem>>);
    let busy = RwSignal::new(false);
    let error = RwSignal::new(None::<&'static str>);
    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            now.set(Some(push::load().await.map_err(Problem::from)));
        });
    });
    let supported = push::browser::supported();
    let active = move || now.with(|n| matches!(n, Some(Ok(n)) if n.active));
    let key = move || {
        now.with(|n| {
            n.as_ref()
                .and_then(|n| n.as_ref().ok())
                .and_then(|n| n.public_key.clone())
        })
    };
    let device = move |_| {
        if busy.get_untracked() || !supported {
            return;
        }
        error.set(None);
        if active() {
            busy.set(true);
            leptos::task::spawn_local(async move {
                if push::disable().await.is_err() {
                    error.set(Some(words::push_failed(lang)));
                }
                now.update(|n| {
                    if let Some(Ok(n)) = n {
                        n.active = false;
                    }
                });
                busy.set(false);
            });
            return;
        }
        let Some(public_key) = key() else {
            return;
        };
        // the browser asks inside the click, before anything is awaited
        let asked = push::browser::ask();
        busy.set(true);
        leptos::task::spawn_local(async move {
            match push::enable(asked, lang, &public_key).await {
                Ok(enabled) => now.update(|n| {
                    if let Some(Ok(n)) = n {
                        n.enabled = enabled;
                        n.active = true;
                    }
                }),
                Err(Refusal::Denied) => error.set(Some(words::push_denied(lang))),
                Err(Refusal::Failed) => error.set(Some(words::push_failed(lang))),
            }
            busy.set(false);
        });
    };
    let switches = push_switches(lang, admin, now, busy, error);
    view! {
        <section class="card" aria-labelledby="push-title">
            <h2 id="push-title">{words::push_title(lang)}</h2>
            <p>{words::push_lead(lang)}</p>
            {(!supported).then(|| view! { <p class="meta">{words::push_unsupported(lang)}</p> })}
            {move || matches!(now.get(), Some(Ok(ref n)) if n.public_key.is_none())
                .then(|| view! { <p class="meta">{words::push_unconfigured(lang)}</p> })}
            {move || matches!(now.get(), Some(Err(_))).then(|| view! { <p class="error" role="alert">{words::push_failed(lang)}</p> })}
            <p class="error" role="alert">{move || error.get()}</p>
            <div class="push-device">
                <button type="button" class="button" role="switch" aria-checked=move || active().to_string()
                    aria-disabled=move || (busy.get() || !supported || key().is_none()).then_some("true")
                    on:click=device>{words::push_device(lang)}</button>
                <p class="meta">{move || if active() { words::push_on(lang) } else { words::push_off(lang) }}</p>
            </div>
            <fieldset class="push-categories">
                <legend>{words::push_categories(lang)}</legend>
                {switches}
            </fieldset>
            <p class="meta">{words::push_scope(lang)}</p>
        </section>
    }
    .into_any()
}

#[island]
pub fn Account(lang: Lang, section: Section, worker: String) -> impl IntoView {
    let state = RwSignal::new(Load::Loading);
    // where the focus goes once a reload an action asked for is drawn
    let focus_next = StoredValue::new(None::<&'static str>);
    // A confirmation code the mail's link handed in the address's
    // fragment (`fragment`): gone from the address bar at once, on every
    // page of the area, and kept on the security page only, until the
    // e-mail section learns whether an address waits for it. Signed out,
    // or with the area not read, it is dropped unseen. One that arrives
    // while the page is open is removed and fills nothing (`fragment::watch`).
    fragment::watch();
    let handed = StoredValue::new(
        fragment::code(&fragment::take(), MailCode::parse).filter(|_| section == Section::Security),
    );
    let reload = Callback::new(move |()| {
        leptos::task::spawn_local(async move {
            let load = summary().await;
            if !matches!(load, Load::Ready(_)) {
                handed.set_value(None);
            }
            state.set(load);
        });
    });
    // Effects run in the browser only: the server renders "loading".
    Effect::new(move |_| reload.run(()));
    let code = lang.code();
    move || {
        match state.get() {
        Load::Loading => view! { <p role="status" class="meta">{words::account_loading(lang)}</p> }.into_any(),
        Load::SignedOut => view! {
            <section class="card app-empty">
                <p>{words::account_signed_out(lang)}</p>
                <p><a class="button primary" href=format!("/{code}/signin")>{words::nav_sign_in(lang)}</a></p>
            </section>
        }
        .into_any(),
        Load::Failed(problem) => view! {
            <p class="error" role="alert">{problem_text(lang, problem)}</p>
            <p><button type="button" class="button" on:click=move |_| reload.run(())>{words::error_again(lang)}</button></p>
        }
        .into_any(),
        Load::Ready(summary) => {
            if let Some(target) = focus_next.try_update_value(Option::take).flatten() {
                super::focus::after_render(target);
            }
            match section {
            Section::Home => home(lang, &summary, &worker),
            Section::Profile => profile(lang, &worker, &summary, reload, focus_next),
            Section::Security => {
                security(lang, worker.clone(), summary.username.clone(), reload, handed)
            }
            Section::Privacy => privacy(lang, worker.clone()),
            Section::Gateways => gateways(lang, worker.clone()),
            Section::Notifications => notifications(lang, summary.admin),
            // the role is Central's to grant: without it, nothing is asked
            section if section.admin() && summary.admin => super::admin::area(
                lang,
                section,
                worker.clone(),
                summary.username.clone(),
                summary.account,
                reload,
            ),
            Section::Feedback | Section::Report if summary.admin => super::feedback::page(
                lang,
                section,
                worker.clone(),
                summary.username.clone(),
                summary.account,
                reload,
            ),
            Section::Feedback | Section::Report => super::admin::refused(lang).into_any(),
            // the messenger is its own island
            Section::Chat | Section::Contacts => ().into_any(),
            // what is left is the administration, for someone without the role
            Section::Admin
            | Section::AdminOverview
            | Section::AdminUsers
            | Section::AdminUser(_)
            | Section::AdminGateways
            | Section::AdminAudit
            | Section::AdminLegacy => super::admin::refused(lang).into_any(),
            }
        }
    }
    }
}
