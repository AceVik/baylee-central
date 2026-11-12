//! Reported profile pictures in the administration (`docs/profile-pictures.md`
//! §"Reports and moderation"): the open reports with the reported picture,
//! who reported it and why; keeping or removing it with a reason the person
//! is told, optionally blocking new pictures; and lifting a block. Every
//! decision proves the password once more in a Worker first
//! (`password::sign_in_again`), since Central wants a sign-in less than 30
//! minutes old for it.

use central_api::admin::{AdminAnswer, AdminRequest};
use central_api::picture::{
    AdminPictureReport, BlockChoice, MAX_NOTICE_NOTE, Named, Picture, PictureAdmin,
    PictureDecision, PictureDigest, PictureReason, PictureReportState, valid_note,
};
use central_api::wire::{Request, Response};
use leptos::prelude::*;

use super::account::when;
use super::auth::{Problem, problem_text};
use super::central::{Failure, call};
use super::crypto::Crypto;
use super::password::{Trouble, sign_in_again};
use super::picture::{face, reason_text, sequence_url};
use crate::words::{self, Lang};

/// The section's signals; all of it is `Copy`.
#[derive(Clone, Copy)]
struct Section {
    lang: Lang,
    worker: StoredValue<String>,
    state: RwSignal<PictureReportState>,
    reports: RwSignal<Option<Result<Vec<AdminPictureReport>, Problem>>>,
    password: RwSignal<String>,
    said: RwSignal<Option<&'static str>>,
    busy: RwSignal<bool>,
    /// The account whose case is open: its name and current picture.
    case: RwSignal<Option<Result<Case, Problem>>>,
}

/// An open case: the account as it is called and its current picture.
type Case = (Named, Option<Picture>);

impl Section {
    /// Opens `account`'s case: its current picture, whether reported or not.
    fn open_case(self, account: [u8; 16]) {
        leptos::task::spawn_local(async move {
            let asked = Request::Admin {
                request: AdminRequest::Picture {
                    request: PictureAdmin::Case { account },
                },
            };
            let read = match call(&asked).await {
                Ok(Response::Admin {
                    answer:
                        AdminAnswer::PictureCase {
                            account, picture, ..
                        },
                }) => Ok((account, picture)),
                Ok(_) => Err(Problem::Failed),
                Err(failure) => Err(failure.into()),
            };
            self.case.set(Some(read));
        });
    }

    fn read(self) {
        let state = self.state.get_untracked();
        leptos::task::spawn_local(async move {
            let asked = Request::Admin {
                request: AdminRequest::Picture {
                    request: PictureAdmin::Reports {
                        state: Some(state),
                        before: None,
                    },
                },
            };
            let read = match call(&asked).await {
                Ok(Response::Admin {
                    answer: AdminAnswer::PictureReports { reports, .. },
                }) => Ok(reports),
                Ok(_) => Err(Problem::Failed),
                Err(failure) => Err(failure.into()),
            };
            self.reports.set(Some(read));
        });
    }

    /// Proves the password, then sends `request`; says how it went.
    fn decide(self, request: PictureAdmin, done: &'static str) {
        if self.busy.get_untracked() {
            return;
        }
        let password = self.password.get_untracked();
        if password.is_empty() {
            self.said
                .set(Some(words::admin_password_missing(self.lang)));
            return;
        }
        self.busy.set(true);
        let lang = self.lang;
        let worker = self.worker.get_value();
        leptos::task::spawn_local(async move {
            let said = match Crypto::start(&worker) {
                None => problem_text(lang, Problem::Failed),
                Some(crypto) => {
                    let signed = sign_in_again(&crypto, &password).await;
                    crypto.close();
                    match signed {
                        Err(Trouble::Current) => words::password_error_current(lang),
                        Err(Trouble::SignedOut) => words::account_signed_out(lang),
                        Err(Trouble::Unsure) => problem_text(lang, Problem::Failed),
                        Err(Trouble::Other(problem)) => problem_text(lang, problem),
                        Ok(_) => match call(&Request::Admin {
                            request: AdminRequest::Picture { request },
                        })
                        .await
                        {
                            Ok(_) => done,
                            Err(Failure::Conflict) => words::admin_pictures_changed(lang),
                            Err(Failure::Rejected) => words::admin_refused_stale(lang),
                            Err(failure) => problem_text(lang, failure.into()),
                        },
                    }
                }
            };
            self.password.set(String::new());
            self.busy.set(false);
            self.said.set(Some(said));
            self.read();
            // a decision changes the case too: it is read again
            if let Some(Ok((named, _))) = self.case.get_untracked() {
                self.open_case(named.id);
            }
        });
    }
}

fn named(lang: Lang, name: Option<&str>, tag: Option<u32>) -> String {
    match (name, tag) {
        (Some(name), Some(tag)) => central_api::profile::handle(name, tag),
        (Some(name), None) => name.to_owned(),
        _ => words::admin_pictures_reporter_gone(lang).to_owned(),
    }
}

/// The decision on `sha256`, `account`'s picture: keep (only where a
/// report is open) or remove with a reason, a note and a block.
fn decide_form(
    s: Section,
    (account, sha256): ([u8; 16], PictureDigest),
    first: PictureReason,
    id: &str,
    keeping: bool,
) -> impl IntoView + use<> {
    let id = id.to_owned();
    let lang = s.lang;
    let reason = RwSignal::new(first);
    let note = RwSignal::new(String::new());
    let block = RwSignal::new(BlockChoice::None);
    let keep = move |_| {
        s.decide(
            PictureAdmin::Decide {
                account,
                sha256,
                decision: PictureDecision::Keep,
            },
            words::admin_pictures_decided(lang),
        );
    };
    let remove = move |_| {
        let typed = note.get_untracked();
        let note = (!typed.trim().is_empty()).then_some(typed);
        if note
            .as_deref()
            .is_some_and(|n| !valid_note(n, MAX_NOTICE_NOTE))
        {
            s.said.set(Some(words::picture_report_note_hint(lang)));
            return;
        }
        s.decide(
            PictureAdmin::Decide {
                account,
                sha256,
                decision: PictureDecision::Remove {
                    reason: reason.get_untracked(),
                    note,
                    block: block.get_untracked(),
                },
            },
            words::admin_pictures_decided(lang),
        );
    };
    let reasons = PictureReason::ALL
        .into_iter()
        .map(|r| view! { <option value=r.as_str() selected=move || reason.get() == r>{reason_text(lang, r)}</option> })
        .collect_view();
    let blocks = [
        (
            "none",
            BlockChoice::None,
            words::admin_pictures_block_none(lang),
        ),
        (
            "week",
            BlockChoice::Days { days: 7 },
            words::admin_pictures_block_week(lang),
        ),
        (
            "month",
            BlockChoice::Days { days: 30 },
            words::admin_pictures_block_month(lang),
        ),
        (
            "lifted",
            BlockChoice::UntilLifted,
            words::admin_pictures_block_lifted(lang),
        ),
    ];
    let options = blocks
        .iter()
        .map(|(value, _, text)| view! { <option value=*value>{*text}</option> })
        .collect_view();
    view! {
        <div class="actions" hidden=!keeping>
            <button type="button" class="button" on:click=keep>{words::admin_pictures_keep(lang)}</button>
        </div>
        <div class="field">
            <label for=format!("{id}-reason")>{words::picture_report_reason(lang)}</label>
            <select id=format!("{id}-reason") on:change=move |ev| {
                if let Some(r) = PictureReason::parse(&event_target_value(&ev)) { reason.set(r) }
            }>{reasons}</select>
        </div>
        <div class="field">
            <label for=format!("{id}-note")>{words::admin_pictures_note(lang)}</label>
            <textarea id=format!("{id}-note") maxlength="500" rows="2" on:input=move |ev| note.set(event_target_value(&ev))></textarea>
        </div>
        <div class="field">
            <label for=format!("{id}-block")>{words::admin_pictures_block(lang)}</label>
            <select id=format!("{id}-block") on:change=move |ev| {
                let value = event_target_value(&ev);
                if let Some((_, choice, _)) = blocks.iter().find(|(v, _, _)| *v == value) { block.set(*choice) }
            }>{options}</select>
        </div>
        <div class="actions">
            <button type="button" class="button danger" on:click=remove>{words::admin_pictures_remove(lang)}</button>
        </div>
    }
}

/// An account's case: its current picture, which an administrator may
/// remove whether or not a report about it is open.
fn case_view(s: Section) -> AnyView {
    let lang = s.lang;
    match s.case.get() {
        None => ().into_any(),
        Some(Err(problem)) => {
            view! { <p class="error" role="alert">{problem_text(lang, problem)}</p> }.into_any()
        }
        Some(Ok((account, picture))) => {
            let who = named(lang, account.display_name.as_deref(), account.tag);
            let shown = picture.map_or_else(
                || view! { <p class="meta" id="picture-case-none">{words::admin_pictures_case_none(lang)}</p> }.into_any(),
                |p| {
                    let form = decide_form(
                        s,
                        (account.id, p.sha256),
                        PictureReason::Other,
                        &format!("case-{}", central_api::picture::hex(&account.id)),
                        false,
                    );
                    view! {
                        <div class="picture-current">{face(&p.key, p.sequence_bytes.is_some(), String::new())}</div>
                        {p.sequence_bytes.is_some().then(|| view! {
                            <a class="picture-open-moving" href=sequence_url(&p.key) target="_blank" rel="noopener noreferrer">{words::picture_open_moving(lang)}</a>
                        })}
                        {form}
                    }
                    .into_any()
                },
            );
            view! {
                <section class="card picture-case" aria-labelledby="picture-case-title">
                    <h3 id="picture-case-title" translate="no">{who}</h3>
                    {shown}
                </section>
            }
            .into_any()
        }
    }
}

fn report_view(s: Section, report: &AdminPictureReport) -> impl IntoView + use<> {
    let lang = s.lang;
    let account = report.account.id;
    let moving = report.moving;
    // An administrator who asked for less motion sees the still here, and
    // opens the animation itself to judge every frame.
    let shown = report.current.map_or_else(
        || view! { <p class="meta">{words::admin_pictures_not_current(lang)}</p> }.into_any(),
        |key| {
            let open = moving.then(|| {
                view! { <a class="picture-open-moving" href=sequence_url(&key) target="_blank" rel="noopener noreferrer">{words::picture_open_moving(lang)}</a> }
            });
            view! { {face(&key, moving, String::new())} {open} }.into_any()
        },
    );
    let actions = match (report.state, report.current.is_some()) {
        (PictureReportState::Open, true) => decide_form(
            s,
            (account, report.sha256),
            report.reason,
            &central_api::picture::hex(&report.id),
            true,
        )
        .into_any(),
        (PictureReportState::Removed, _) => view! {
            <div class="actions">
                <button type="button" class="button" on:click=move |_| {
                    s.decide(PictureAdmin::LiftBlock { account }, words::admin_pictures_lifted(lang));
                }>{words::admin_pictures_lift(lang)}</button>
            </div>
        }
        .into_any(),
        _ => ().into_any(),
    };
    view! {
        <li class="card picture-report-item">
            <div class="picture-current">{shown}</div>
            <dl class="facts">
                <dt>{words::admin_pictures_account(lang)}</dt>
                <dd translate="no">{named(lang, report.account.display_name.as_deref(), report.account.tag)}</dd>
                <dt>{words::admin_pictures_reporter(lang)}</dt>
                <dd translate="no">{report.reporter.as_ref().map_or_else(
                    || words::admin_pictures_reporter_gone(lang).to_owned(),
                    |r| named(lang, r.display_name.as_deref(), r.tag),
                )}</dd>
                <dt>{words::picture_report_reason(lang)}</dt>
                <dd>{reason_text(lang, report.reason)}</dd>
                <dt>{words::admin_pictures_when(lang)}</dt>
                <dd>{when(lang, report.created_at)}</dd>
            </dl>
            {report.note.clone().map(|note| view! { <blockquote translate="no">{note}</blockquote> })}
            {actions}
            <div class="actions">
                <button type="button" class="button picture-case-open" on:click=move |_| s.open_case(account)>
                    {words::admin_pictures_case(lang)}</button>
            </div>
        </li>
    }
}

/// The administrators' picture reports, below the reset tool.
pub fn section(lang: Lang, worker: String) -> impl IntoView {
    let s = Section {
        lang,
        worker: StoredValue::new(worker),
        state: RwSignal::new(PictureReportState::Open),
        reports: RwSignal::new(None),
        case: RwSignal::new(None),
        password: RwSignal::new(String::new()),
        said: RwSignal::new(None),
        busy: RwSignal::new(false),
    };
    Effect::new(move |_| s.read());
    let show = move |state| {
        s.state.set(state);
        s.said.set(None);
        s.read();
    };
    let list = move || match s.reports.get() {
        None => {
            view! { <p class="meta" role="status">{words::account_loading(lang)}</p> }.into_any()
        }
        Some(Err(problem)) => {
            view! { <p class="error" role="alert">{problem_text(lang, problem)}</p> }.into_any()
        }
        Some(Ok(reports)) if reports.is_empty() => {
            view! { <p>{words::admin_pictures_none(lang)}</p> }.into_any()
        }
        Some(Ok(reports)) => {
            let items = reports.iter().map(|r| report_view(s, r)).collect_view();
            view! { <ul class="picture-reports">{items}</ul> }.into_any()
        }
    };
    view! {
        <section class="card" aria-labelledby="admin-pictures-title">
            <h2 id="admin-pictures-title">{words::admin_pictures_title(lang)}</h2>
            <p>{words::admin_pictures_lead(lang)}</p>
            <div class="actions" role="group">
                <button type="button" class="button" aria-pressed=move || (s.state.get() == PictureReportState::Open).to_string()
                    on:click=move |_| show(PictureReportState::Open)>{words::admin_pictures_open(lang)}</button>
                <button type="button" class="button" aria-pressed=move || (s.state.get() == PictureReportState::Removed).to_string()
                    on:click=move |_| show(PictureReportState::Removed)>{words::admin_pictures_removed(lang)}</button>
            </div>
            <div class="field">
                <label for="admin-pictures-password">{words::admin_pictures_password(lang)}</label>
                <input id="admin-pictures-password" type="password" autocomplete="current-password"
                    aria-describedby="admin-pictures-password-hint"
                    prop:value=move || s.password.get() on:input=move |ev| s.password.set(event_target_value(&ev)) />
                <p class="hint" id="admin-pictures-password-hint">{words::admin_pictures_password_hint(lang)}</p>
            </div>
            <p class="meta" role="status">{move || s.said.get()}</p>
            {move || case_view(s)}
            {list}
        </section>
    }
}
