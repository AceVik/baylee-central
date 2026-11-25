//! `/{lang}/account/admin/feedback-tokens`: the report tokens
//! (`docs/reports.md` §"Report tokens"), the live ones first
//! (`AdminRequest::ReportTokens`): what each may do, who made it and when,
//! when it ends and when it was last used. An administrator makes one from a
//! preset or from its rights (`CreateReportToken`), and revokes one after
//! its confirmation (`common::Flow`).
//!
//! A new token is shown once. It is held by the step that shows it, as a
//! `Token`, which wipes its bytes when it goes, and goes when that step is
//! left (by its button, by leaving the page, or by the page being put away
//! into the browser's back-forward cache). It never goes into an address, a
//! log, the console, storage or a field with a name.

use std::sync::Arc;

use central_api::Token;
use central_api::admin::{AdminAnswer, AdminRequest};
use central_api::owner::{MAX_TOKEN_DAYS, MAX_TOKEN_NAME};
use central_api::report_tokens::{
    ReportScope, ReportScopes, ReportTokenInfo, token_text, validate_new,
};
use central_api::wire::{Request, Response};
use leptos::prelude::*;

use super::Listing;
use super::common::{self, Asked, Came, Change, Cx, Flow, Note};
use crate::data::id_hex;
use crate::islands::account::when;
use crate::islands::auth::{Problem, problem_text};
use crate::islands::central::{Failure, call};
use crate::islands::chat::glyph;
use crate::islands::field::{self, Labelled};
use crate::islands::{clip, focus};
use crate::words::{self, Lang};

/// How long a token is offered to live, in days.
const DAYS: [u16; 4] = [30, 90, 180, 365];
const _: () = assert!(DAYS[3] <= MAX_TOKEN_DAYS);

/// Where a token stands at a moment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Standing {
    /// It works.
    Live,
    /// Its maker is no administrator now.
    Suspended,
    /// Its time is over.
    Expired,
    /// It was revoked.
    Revoked,
}

fn standing(token: &ReportTokenInfo, now: i64) -> Standing {
    if token.revoked_at.is_some() {
        Standing::Revoked
    } else if token.expires_at <= now {
        Standing::Expired
    } else if token.suspended {
        Standing::Suspended
    } else {
        Standing::Live
    }
}

fn standing_text(lang: Lang, standing: Standing) -> (&'static str, &'static str) {
    match standing {
        Standing::Live => (words::rt_live(lang), "ok"),
        Standing::Suspended => (words::rt_suspended(lang), "warn"),
        Standing::Expired => (words::rt_expired(lang), "off"),
        Standing::Revoked => (words::tokens_revoked(lang), "bad"),
    }
}

/// A preset's words; `None` for rights no preset holds.
fn preset_text(lang: Lang, scopes: ReportScopes) -> Option<&'static str> {
    let [post, read, edit, all] = ReportScopes::PRESETS;
    if scopes == post {
        Some(words::rt_preset_post(lang))
    } else if scopes == read {
        Some(words::rt_preset_read(lang))
    } else if scopes == edit {
        Some(words::rt_preset_edit(lang))
    } else if scopes == all {
        Some(words::rt_preset_all(lang))
    } else {
        None
    }
}

fn right_text(lang: Lang, right: ReportScope) -> &'static str {
    match right {
        ReportScope::File => words::rt_right_file(lang),
        ReportScope::Read => words::rt_right_read(lang),
        ReportScope::Triage => words::rt_right_triage(lang),
        ReportScope::Purge => words::rt_right_purge(lang),
    }
}

/// A person a token names: their handle, else the end of their id.
fn person(id: &[u8; 16], name: Option<&str>, tag: Option<u32>) -> String {
    common::handle(name, tag).unwrap_or_else(|| common::id_end(id))
}

/// A token just made, shown once.
struct Made {
    token: Token,
    name: String,
}

/// The making of a token, step by step.
#[derive(Clone)]
enum Step {
    Form,
    Password,
    Working,
    Shown(Arc<Made>),
}

/// What a new token is asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
struct New {
    name: String,
    scopes: ReportScopes,
    days: u16,
}

/// What a failed making means before it is explained: `None` for Central's
/// refusal, which is explained by what stands afterwards. An answer that did
/// not arrive is no refusal: a token may have been made.
fn at_making(failure: Failure) -> Option<Came> {
    match failure {
        Failure::Rejected => None,
        Failure::Busy => Some(Came::Other(Problem::Busy)),
        Failure::Conflict | Failure::Forbidden => Some(Came::Other(Problem::Failed)),
        Failure::Unavailable | Failure::Malformed | Failure::Network => Some(Came::Lost),
    }
}

/// The words for what a making came to; `None` where the page asks for the
/// password or says nobody is signed in.
fn making_text(lang: Lang, came: Came) -> Option<&'static str> {
    match came {
        Came::Other(Problem::Busy) => Some(words::rt_full(lang)),
        Came::Lost => Some(words::rt_lost(lang)),
        came => common::came_text(lang, came),
    }
}

/// Asks Central for `new`, once, after `AccountSummary` said it may be;
/// `proven` where the password was proven just now.
async fn make(new: New, own: [u8; 16], proven: bool) -> Result<Made, Came> {
    common::may_send(own, proven).await?;
    let request = AdminRequest::CreateReportToken {
        name: new.name.clone(),
        scopes: new.scopes,
        days: new.days,
    };
    match call(&Request::Admin { request }).await {
        Ok(Response::Admin {
            answer: AdminAnswer::ReportTokenCreated { token, .. },
        }) => Ok(Made {
            token,
            name: new.name,
        }),
        // an answer of another kind says nothing of what happened
        Ok(_) => Err(Came::Lost),
        Err(failure) => match at_making(failure) {
            Some(came) => Err(came),
            None => Err(common::refused(own, proven).await),
        },
    }
}

/// The page's signals. All of it is `Copy`.
#[derive(Clone, Copy)]
struct Page {
    cx: Cx,
    rows: RwSignal<Vec<ReportTokenInfo>>,
    listing: RwSignal<Listing>,
    /// The newest list request: an older one's answer is not drawn.
    generation: StoredValue<u32>,
    step: RwSignal<Step>,
    name: RwSignal<String>,
    scopes: RwSignal<ReportScopes>,
    days: RwSignal<u16>,
    password: RwSignal<String>,
    /// What the form or the password's step says of the last attempt.
    wrong: RwSignal<Option<&'static str>>,
    /// A making is on its way: nothing else is sent.
    busy: StoredValue<bool>,
    note: RwSignal<Option<Note>>,
    flow: Flow,
}

impl Page {
    /// Reads the tokens, and after a revocation says what came of it.
    fn read(self, ended: Option<Came>) {
        let Some(generation) = self.generation.try_get_value() else {
            return;
        };
        let generation = generation.wrapping_add(1);
        self.generation.set_value(generation);
        let lang = self.cx.lang;
        leptos::task::spawn_local(async move {
            let answer = common::read(AdminRequest::ReportTokens).await;
            if self.generation.try_get_value() != Some(generation) {
                return;
            }
            let read = match answer {
                Ok(AdminAnswer::ReportTokens { tokens }) => {
                    self.rows.set(tokens);
                    self.listing.set(Listing::Ready);
                    true
                }
                Ok(_) => {
                    self.listing
                        .set(Listing::Failed(problem_text(lang, Problem::Failed)));
                    false
                }
                Err(unread) => {
                    let missing = problem_text(lang, Problem::Failed);
                    match self.cx.unread_text(unread, missing) {
                        Some(text) => self.listing.set(Listing::Failed(text)),
                        None => return,
                    }
                    false
                }
            };
            if let Some(came) = ended {
                if came == Came::Lost {
                    self.note.set(Some(common::lost(lang, read)));
                }
                common::focus_note(self.note);
            }
        });
    }

    /// What the form asks for, if a token may be asked for it.
    fn asked(self) -> Option<New> {
        let new = New {
            name: self.name.get_untracked().trim().to_owned(),
            scopes: self.scopes.get_untracked(),
            days: self.days.get_untracked(),
        };
        validate_new(&new.name, new.scopes, new.days)
            .is_ok()
            .then_some(new)
    }

    /// The form's submit, and the password's: asks once, after the password
    /// where `password` is one.
    fn submit(self, password: Option<String>) {
        if self.busy.get_value() {
            return;
        }
        let lang = self.cx.lang;
        self.note.set(None);
        let Some(new) = self.asked() else {
            self.step.set(Step::Form);
            self.wrong.set(Some(words::tokens_invalid(lang)));
            focus::after_render("#rt-name");
            return;
        };
        self.busy.set_value(true);
        self.wrong.set(None);
        self.step.set(Step::Working);
        focus::after_render("#rt-working");
        let own = self.cx.own;
        let worker = self.cx.worker.get_value();
        leptos::task::spawn_local(async move {
            let made = match password {
                Some(password) => match common::prove(own, &worker, password).await {
                    Ok(()) => make(new, own, true).await,
                    Err(came) => Err(came),
                },
                None => make(new, own, false).await,
            };
            self.ended(made);
        });
    }

    /// What a making came to.
    fn ended(self, made: Result<Made, Came>) {
        // a page that went meanwhile shows nothing more
        if self.busy.try_get_value().is_none() {
            return;
        }
        self.busy.set_value(false);
        let lang = self.cx.lang;
        match made {
            Ok(made) => {
                self.name.set(String::new());
                self.step.set(Step::Shown(Arc::new(made)));
                focus::after_render("#rt-shown");
                self.read(None);
            }
            Err(Came::Password) => {
                self.step.set(Step::Password);
                focus::after_render("#admin-password");
            }
            Err(Came::WrongPassword) => {
                self.wrong.set(making_text(lang, Came::WrongPassword));
                self.step.set(Step::Password);
                focus::after_render("#admin-password");
            }
            Err(Came::SignedOut) => self.cx.leave(),
            Err(came) => {
                self.step.set(Step::Form);
                self.wrong.set(making_text(lang, came));
                focus::after_render("#rt-wrong");
                if came == Came::Lost {
                    self.read(None);
                }
            }
        }
    }

    /// The token shown is put away, and the form is there again.
    fn kept(self) {
        self.step.set(Step::Form);
        focus::after_render("#rt-new");
    }

    /// A page put away takes no token with it.
    #[cfg(target_arch = "wasm32")]
    fn forget(self) {
        if let Some(Step::Shown(_)) = self.step.try_get_untracked() {
            self.step.set(Step::Form);
        }
        let _ = self.password.try_set(String::new());
    }

    /// A token's "Revoke": its confirmation.
    fn revoke(self, token: &ReportTokenInfo) {
        let lang = self.cx.lang;
        self.flow.ask(Asked {
            change: Change::RevokeReport { token: token.id },
            title: words::rt_revoke_title(lang, &token.name),
            text: words::rt_revoke_text(lang).to_owned(),
            action: words::tokens_revoke(lang),
            danger: true,
            from: format!("#rt-revoke-{}", id_hex(&token.id)),
        });
    }
}

/// The presets as one choice of four, and the rights behind it: a right
/// brings what it needs, and takes what needs it along.
fn rights_view(p: &Page) -> impl IntoView + use<> {
    let p = *p;
    let lang = p.cx.lang;
    let presets = ReportScopes::PRESETS
        .into_iter()
        .map(|preset| {
            let pressed = move || (p.scopes.get() == preset).to_string();
            view! {
                <button type="button" class="chip-btn" aria-pressed=pressed
                    on:click=move |_| p.scopes.set(preset)>{preset_text(lang, preset)}</button>
            }
        })
        .collect_view();
    let boxes = ReportScope::ALL
        .into_iter()
        .map(|right| {
            let id = format!("rt-right-{}", right.as_str());
            view! {
                <div class="check">
                    <input id=id.clone() type="checkbox" aria-describedby="rt-rights-hint"
                        prop:checked=move || p.scopes.get().allows(right)
                        on:change=move |ev| {
                            let held = event_target_checked(&ev);
                            p.scopes.update(|scopes| *scopes = scopes.set(right, held));
                        } />
                    <label for=id>{right_text(lang, right)}</label>
                </div>
            }
        })
        .collect_view();
    view! {
        <fieldset class="admin-chips">
            <legend>{words::rt_preset(lang)}</legend>
            {presets}
        </fieldset>
        <fieldset class="choice">
            <legend>{words::rt_rights(lang)}</legend>
            {boxes}
            <p class="hint" id="rt-rights-hint">{words::rt_rights_hint(lang)}</p>
        </fieldset>
    }
}

fn form_view(p: &Page) -> AnyView {
    let p = *p;
    let lang = p.cx.lang;
    let spec = Labelled {
        id: "rt-name",
        label: words::tokens_name(lang),
        hint: Some(words::rt_name_hint(lang)),
        mark: glyph::KEY,
        auto: "off",
        max: "64",
    };
    let days = DAYS
        .into_iter()
        .map(|days| {
            view! { <option value=days.to_string() prop:selected=move || p.days.get() == days>{words::number(lang, u64::from(days))}</option> }
        })
        .collect_view();
    view! {
        <form class="form" method="post" novalidate=true on:submit=move |event: leptos::ev::SubmitEvent| {
            event.prevent_default();
            p.submit(None);
        }>
            {field::labelled(lang, spec, move || p.name.get(), move |value| p.name.set(value))}
            {rights_view(&p)}
            <div class="field">
                <label for="rt-days">{words::tokens_days(lang)}</label>
                <select id="rt-days" on:change=move |ev| p.days.set(event_target_value(&ev).parse().unwrap_or(DAYS[1]))>
                    {days}
                </select>
            </div>
            <p class="error" role="alert" id="rt-wrong" tabindex="-1">{move || p.wrong.get()}</p>
            <p class="actions">
                <button type="submit" class="button primary">{words::tokens_make(lang)}</button>
            </p>
        </form>
    }
    .into_any()
}

fn password_view(p: &Page) -> AnyView {
    let p = *p;
    let lang = p.cx.lang;
    let spec = Labelled {
        id: "admin-password",
        label: words::privacy_delete_password(lang),
        hint: Some(words::admin_change_password(lang)),
        mark: glyph::LOCK,
        auto: "current-password",
        max: "1024",
    };
    let submit = move |event: leptos::ev::SubmitEvent| {
        event.prevent_default();
        let password = p.password.get_untracked();
        if password.is_empty() {
            p.wrong.set(Some(words::admin_password_missing(lang)));
            focus::after_render("#admin-password");
            return;
        }
        p.password.set(String::new());
        p.submit(Some(password));
    };
    view! {
        <form class="form" method="post" novalidate=true on:submit=submit>
            // for a password manager: whose password this is
            <input type="text" autocomplete="username" readonly=true hidden=true value=p.cx.username.get_value() />
            {field::labelled(lang, spec, move || p.password.get(), move |value| p.password.set(value))}
            <p class="error" role="alert">{move || p.wrong.get()}</p>
            <p class="actions">
                <button type="submit" class="button primary">{words::tokens_make(lang)}</button>
                <button type="button" class="button quiet" on:click=move |_| {
                    p.password.set(String::new());
                    p.wrong.set(None);
                    p.step.set(Step::Form);
                    focus::after_render("#rt-name");
                }>{words::admin_back(lang)}</button>
            </p>
        </form>
    }
    .into_any()
}

fn shown_view(p: &Page, made: &Arc<Made>) -> AnyView {
    let p = *p;
    let lang = p.cx.lang;
    let text = token_text(&made.token).as_str().to_owned();
    let (copy, copied) = clip::copier(lang, text.clone());
    view! {
        <div class="secret-box" role="group" aria-labelledby="rt-shown">
            <h3 id="rt-shown" tabindex="-1">{words::tokens_shown_title(lang)}</h3>
            <p class="meta"><bdi translate="no">{made.name.clone()}</bdi></p>
            <p class="eyebrow">{words::code_shown_once(lang)}</p>
            <p class="secret" translate="no"><code>{text}</code></p>
            <p>{words::rt_shown_lead(lang)}</p>
            <p class="actions">
                {copy}
                <button type="button" class="button primary" on:click=move |_| p.kept()>{words::tokens_kept(lang)}</button>
            </p>
            {copied}
        </div>
    }
    .into_any()
}

fn making_view(p: &Page) -> impl IntoView + use<> {
    let p = *p;
    let lang = p.cx.lang;
    let body = move || {
        match p.step.get() {
        Step::Form => form_view(&p),
        Step::Password => password_view(&p),
        Step::Working => view! {
            <p class="working" role="status" tabindex="-1" id="rt-working">{words::admin_working(lang)}</p>
        }
        .into_any(),
        Step::Shown(made) => shown_view(&p, &made),
    }
    };
    view! {
        <section class="card rt-making" aria-labelledby="rt-new">
            <h2 id="rt-new" tabindex="-1">{words::rt_new(lang)}</h2>
            {body}
        </section>
    }
}

fn card(p: &Page, token: &ReportTokenInfo, now: i64) -> impl IntoView + use<> {
    let p = *p;
    let lang = p.cx.lang;
    let id = token.id;
    let key = id_hex(&id);
    let heading = format!("rt-token-{key}");
    let at = standing(token, now);
    let (state, tone) = standing_text(lang, at);
    let rights = match preset_text(lang, token.scopes) {
        Some(preset) => view! { {preset} }.into_any(),
        None => token
            .scopes
            .rights()
            .map(|right| right_text(lang, right))
            .collect::<Vec<_>>()
            .join(" · ")
            .into_any(),
    };
    let maker = person(
        &token.created_by,
        token.created_by_name.as_deref(),
        token.created_by_tag,
    );
    let by = common::user_path(lang, &token.created_by);
    let used = token
        .last_used_at
        .map_or_else(|| words::rt_never(lang).to_owned(), |at| when(lang, at));
    let revoked = token.revoked_at.map(|time| {
        let time = when(lang, time);
        let text = match token.revoked_by {
            Some(admin) => {
                let name = person(
                    &admin,
                    token.revoked_by_name.as_deref(),
                    token.revoked_by_tag,
                );
                words::rt_revoked_by(lang, &name, &time)
            }
            None => words::rt_revoked_at(lang, &time),
        };
        view! { <p class="meta"><bdi>{text}</bdi></p> }
    });
    let suspended = (at == Standing::Suspended)
        .then(|| view! { <p class="meta">{words::rt_suspended_text(lang)}</p> });
    let action = matches!(at, Standing::Live | Standing::Suspended).then(|| {
        let asked = token.clone();
        view! {
            <p class="actions">
                <button type="button" class="button danger" id=format!("rt-revoke-{key}") aria-describedby=heading.clone()
                    on:click=move |_| p.revoke(&asked)>{words::tokens_revoke(lang)}</button>
            </p>
        }
    });
    view! {
        <li class="card admin-gw" data-standing=tone>
            <div class="admin-gw-head">
                <h2 class="admin-gw-name" id=heading tabindex="-1"><bdi translate="no">{token.name.clone()}</bdi></h2>
                <p class="pills"><span class=format!("pill {tone}")>{state}</span></p>
            </div>
            <dl class="facts">
                <dt>{words::rt_rights(lang)}</dt>
                <dd>{rights}</dd>
                <dt>{words::rt_by(lang)}</dt>
                <dd><a href=by><bdi translate="no">{maker}</bdi></a></dd>
                <dt>{words::admin_created(lang)}</dt>
                <dd>{when(lang, token.created_at)}</dd>
                <dt>{words::rt_expires(lang)}</dt>
                <dd>{when(lang, token.expires_at)}</dd>
                <dt>{words::rt_used(lang)}</dt>
                <dd>{used}</dd>
            </dl>
            {revoked}
            {suspended}
            {action}
            {p.flow.panel(move |change| *change == Change::RevokeReport { token: id })}
        </li>
    }
}

fn results(p: &Page) -> impl IntoView + use<> {
    let p = *p;
    let lang = p.cx.lang;
    move || {
        let rows = p.rows.get();
        match p.listing.get() {
            Listing::Loading => {
                view! { <p role="status" class="meta">{words::account_loading(lang)}</p> }
                    .into_any()
            }
            Listing::Failed(text) => view! {
                <p class="error" role="alert">{text}</p>
                <p class="actions">
                    <button type="button" class="button" on:click=move |_| {
                        p.listing.set(Listing::Loading);
                        p.read(None);
                    }>{words::error_again(lang)}</button>
                </p>
            }
            .into_any(),
            Listing::Ready if rows.is_empty() => {
                view! { <p role="status">{words::rt_none(lang)}</p> }.into_any()
            }
            Listing::Ready => {
                let now = common::now();
                let cards = rows.iter().map(|token| card(&p, token, now)).collect_view();
                view! { <ul class="admin-gws" id="rt-tokens">{cards}</ul> }.into_any()
            }
        }
    }
}

/// The page: what a token is for, its making, and the tokens.
pub fn page(cx: Cx) -> impl IntoView {
    let lang = cx.lang;
    let note = RwSignal::new(None::<Note>);
    let ended = StoredValue::new(None::<Page>);
    let finished = Callback::new(move |(_, came): (Change, Came)| {
        if let Some(Some(p)) = ended.try_get_value() {
            p.read(Some(came));
        }
    });
    let p = Page {
        cx,
        rows: RwSignal::new(Vec::new()),
        listing: RwSignal::new(Listing::Loading),
        generation: StoredValue::new(0),
        step: RwSignal::new(Step::Form),
        name: RwSignal::new(String::new()),
        // the least a token can do, so that more is always a choice
        scopes: RwSignal::new(ReportScopes::ONLY_POST),
        days: RwSignal::new(DAYS[1]),
        password: RwSignal::new(String::new()),
        wrong: RwSignal::new(None),
        busy: StoredValue::new(false),
        note,
        flow: Flow::new(cx, note, finished),
    };
    ended.set_value(Some(p));
    // Effects run in the browser only.
    Effect::new(move |_| p.read(None));
    #[cfg(target_arch = "wasm32")]
    {
        let away = window_event_listener_untyped("pagehide", move |_| p.forget());
        on_cleanup(move || away.remove());
    }
    view! {
        <p class="admin-lead">{words::rt_lead(lang)}</p>
        {making_view(&p)}
        {p.flow.notes()}
        <div class="admin-results" aria-busy=move || (p.listing.get() == Listing::Loading).then_some("true")>
            {results(&p)}
        </div>
    }
}

const _: () = assert!(MAX_TOKEN_NAME == 64, "the name's field takes as many");

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_800_000_000;

    fn token() -> ReportTokenInfo {
        ReportTokenInfo {
            id: [1; 16],
            name: "CI".into(),
            scopes: ReportScopes::ONLY_POST,
            created_by: [2; 16],
            created_by_name: Some("Erika".into()),
            created_by_tag: Some(0x1a2b),
            suspended: false,
            created_at: NOW - 100,
            expires_at: NOW + 100,
            revoked_at: None,
            revoked_by: None,
            revoked_by_name: None,
            revoked_by_tag: None,
            last_used_at: None,
        }
    }

    /// A token stands as its worst says: revoked, then over, then its maker.
    #[test]
    fn where_a_token_stands() {
        let live = token();
        assert_eq!(standing(&live, NOW), Standing::Live);
        assert_eq!(standing(&live, NOW + 99), Standing::Live);
        assert_eq!(standing(&live, NOW + 100), Standing::Expired);
        let suspended = ReportTokenInfo {
            suspended: true,
            ..token()
        };
        assert_eq!(standing(&suspended, NOW), Standing::Suspended);
        assert_eq!(standing(&suspended, NOW + 100), Standing::Expired);
        let revoked = ReportTokenInfo {
            revoked_at: Some(NOW),
            ..suspended
        };
        assert_eq!(standing(&revoked, NOW), Standing::Revoked);
        assert_eq!(standing(&revoked, NOW + 100), Standing::Revoked);
        assert_eq!(person(&[2; 16], Some("Erika"), Some(0x1a2b)), "Erika#1a2b");
        assert_eq!(person(&[2; 16], None, None), "…02020202");
    }

    /// Every preset, right and standing has words of its own in every
    /// language, and rights no preset holds have none of a preset's.
    #[test]
    fn every_right_preset_and_standing_has_its_words() {
        for lang in Lang::ALL {
            let presets: Vec<&str> = ReportScopes::PRESETS
                .into_iter()
                .map(|preset| preset_text(lang, preset).expect("words"))
                .collect();
            let rights: Vec<&str> = ReportScope::ALL
                .into_iter()
                .map(|right| right_text(lang, right))
                .collect();
            let standings: Vec<&str> = [
                Standing::Live,
                Standing::Suspended,
                Standing::Expired,
                Standing::Revoked,
            ]
            .into_iter()
            .map(|at| standing_text(lang, at).0)
            .collect();
            for texts in [presets, rights, standings] {
                for (at, text) in texts.iter().enumerate() {
                    assert_ne!(*text, "");
                    assert!(!texts[..at].contains(text), "{lang:?}: {text}");
                }
            }
            let mixed = ReportScopes::of(&[ReportScope::File, ReportScope::Read]);
            assert!(mixed.validate().is_ok());
            assert_eq!(preset_text(lang, mixed), None);
        }
    }

    /// An answer that did not arrive is never a refusal, a full table says
    /// so in its own words, and Central's refusal is explained.
    #[test]
    fn what_a_failed_making_means() {
        for failure in [Failure::Unavailable, Failure::Malformed, Failure::Network] {
            assert_eq!(at_making(failure), Some(Came::Lost));
        }
        assert_eq!(at_making(Failure::Rejected), None);
        assert_eq!(at_making(Failure::Busy), Some(Came::Other(Problem::Busy)));
        for lang in Lang::ALL {
            let full = making_text(lang, Came::Other(Problem::Busy));
            let lost = making_text(lang, Came::Lost);
            assert_eq!(full, Some(words::rt_full(lang)));
            assert!(full != lost && lost.is_some());
            for came in [Came::Password, Came::SignedOut] {
                assert_eq!(making_text(lang, came), None);
            }
            assert!(making_text(lang, Came::Role).is_some());
        }
    }

    /// The one request that makes a token, as Central reads it.
    #[test]
    fn the_request_serializes_as_central_reads_it() {
        let request = Request::Admin {
            request: AdminRequest::CreateReportToken {
                name: "CI".into(),
                scopes: ReportScopes::READ_AND_EDIT,
                days: 90,
            },
        };
        assert_eq!(
            serde_json::to_string(&request).expect("json"),
            r#"{"operation":"admin","request":{"action":"create_report_token","name":"CI","scopes":["read","triage"],"days":90}}"#
        );
    }
}
