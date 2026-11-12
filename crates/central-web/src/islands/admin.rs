//! An administrator makes a reset code (`docs/account-reset.md`, "Issuing a
//! reset code"), the one piece of the administration on the new site: find
//! the account (`AdminRequest::Accounts`), open it (`AdminRequest::Account`,
//! which alone says whether it has a recovery code), read the warning,
//! prove the password once more in a Worker (`password::sign_in_again`),
//! and only then `AdminRequest::ResetLogin`. The code is shown once.
//!
//! The code never goes into an address, a log, the console, storage or a
//! field with a name: it is held by the step that shows it, as a
//! `ResetCode`, which wipes its bytes when it goes, and goes when that step
//! is left (by its button, by leaving the page, or by the page being put
//! away into the browser's back-forward cache).
//!
//! Central refuses every code it does not make with the same `Rejected`.
//! What the page says comes from what Central says afterwards: whether this
//! browser's account still holds the role, and the account's row read
//! again. An answer that did not arrive is never a refusal: a code may have
//! been made.

use std::sync::Arc;

use central_api::admin::{AccountQuery, AccountState, AdminAccount, AdminAnswer, AdminRequest};
use central_api::reset_code::ResetCode;
use central_api::wire::{AccountSource, Request, Response};
use leptos::prelude::*;

use super::account::when;
use super::auth::{Problem, problem_text};
use super::central::{Failure, call};
use super::chat::glyph;
use super::crypto::Crypto;
use super::field::{self, Labelled};
use super::password::{Trouble, sign_in_again};
use super::{clip, focus, session};
use crate::words::{self, Lang};

/// Central's password sign-in that a change needs, in seconds.
const FRESH: i64 = 30 * 60;

/// One account as the page draws it: what `AdminAccount` says.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Row {
    id: [u8; 16],
    username: String,
    display_name: Option<String>,
    tag: Option<u32>,
    moved: bool,
    state: AccountState,
    created_at: i64,
    last_sign_in_at: Option<i64>,
    admin: bool,
    reset_until: Option<i64>,
}

impl From<AdminAccount> for Row {
    fn from(account: AdminAccount) -> Self {
        Row {
            id: account.id,
            username: account.username,
            display_name: account.display_name,
            tag: account.tag,
            moved: account.source == AccountSource::Legacy,
            state: account.state,
            created_at: account.created_at,
            last_sign_in_at: account.last_sign_in_at,
            admin: account.admin,
            reset_until: account.reset_until,
        }
    }
}

impl Row {
    /// How the account is named: its handle, or its sign-in name until it
    /// has one.
    fn name(&self) -> String {
        match (&self.display_name, self.tag) {
            (Some(name), Some(tag)) => central_api::profile::handle(name, tag),
            _ => self.username.clone(),
        }
    }

    /// Its part of element ids: its id in hex, unique on the page.
    fn key(&self) -> String {
        use std::fmt::Write as _;
        self.id
            .iter()
            .fold(String::with_capacity(32), |mut out, byte| {
                let _ = write!(out, "{byte:02x}");
                out
            })
    }
}

/// An account opened: its row read afresh, and whether it has a recovery
/// code (only `AdminRequest::Account` says so).
#[derive(Clone, Debug, PartialEq, Eq)]
struct Opened {
    row: Row,
    recovery: bool,
}

/// Why Central makes no code for an account.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Refusal {
    /// It holds the administrator role: the role goes first.
    Administrator,
    /// It is the administrator's own.
    Own,
    /// It was reserved and never set up.
    Pending,
    /// It moved from the official gateway and the move is not finished:
    /// the bridge resets it.
    Moving,
    /// Central counts the administrator's password sign-in as older than
    /// 30 minutes.
    Stale,
    /// This browser's account no longer holds the administrator role.
    Role,
    /// The account is gone.
    Gone,
}

/// What the page says when no code is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Said {
    /// Central refused, for this reason.
    Refused(Refusal),
    /// The password is not right: nothing was asked of Central.
    Password,
    /// The password was not typed.
    NoPassword,
    /// This browser is signed in as another account than the page was
    /// opened as: nothing was asked.
    Switched,
    /// No answer came back: a code may have been made.
    Lost,
    /// Anything else, in the forms' shared words: no code was made.
    Other(Problem),
}

fn said_text(lang: Lang, said: Said) -> &'static str {
    match said {
        Said::Refused(refusal) => refusal_text(lang, refusal),
        Said::Password => words::password_error_current(lang),
        Said::NoPassword => words::admin_password_missing(lang),
        Said::Switched => words::admin_switched(lang),
        Said::Lost => words::admin_lost(lang),
        Said::Other(problem) => problem_text(lang, problem),
    }
}

fn refusal_text(lang: Lang, refusal: Refusal) -> &'static str {
    match refusal {
        Refusal::Administrator => words::admin_refused_admin(lang),
        Refusal::Own => words::admin_refused_own(lang),
        Refusal::Pending => words::admin_refused_pending(lang),
        Refusal::Moving => words::admin_refused_moving(lang),
        Refusal::Stale => words::admin_refused_stale(lang),
        Refusal::Role => words::admin_refused_role(lang),
        Refusal::Gone => words::admin_refused_gone(lang),
    }
}

/// Why Central would refuse a code for `row`, as far as the row tells,
/// before anything is asked; `None` where it may make one. A moved account
/// whose move is closed is in use (migration 000005 requires it), so one
/// that is not in use has an unfinished move; one in use may or may not.
fn refusal(row: &Row, own: [u8; 16]) -> Option<Refusal> {
    if row.id == own {
        Some(Refusal::Own)
    } else if row.admin {
        Some(Refusal::Administrator)
    } else if row.state == AccountState::Pending {
        Some(Refusal::Pending)
    } else if row.moved && row.state == AccountState::Enrolled {
        Some(Refusal::Moving)
    } else {
        None
    }
}

/// Why Central refused the code it was asked for, from what it says
/// afterwards: whether this browser's account still holds the role and
/// when it last proved its password (`signed_in`, Unix seconds, against
/// the browser's clock `now`), and the account read again. Central refuses
/// a resettable account's code only for a stale sign-in; for a moved one
/// in use, the move may not be finished either, which the row does not
/// say, so that is the answer while the sign-in is fresh by this clock.
fn explain(
    admin: bool,
    signed_in: i64,
    now: i64,
    row: Result<&Row, Failure>,
    own: [u8; 16],
) -> Said {
    if !admin {
        return Said::Refused(Refusal::Role);
    }
    let row = match row {
        Ok(row) => row,
        Err(Failure::Rejected) => return Said::Refused(Refusal::Gone),
        Err(failure) => return Said::Other(Problem::outside_sign_in(failure)),
    };
    if let Some(refusal) = refusal(row, own) {
        return Said::Refused(refusal);
    }
    if row.moved && now - signed_in < FRESH {
        Said::Refused(Refusal::Moving)
    } else {
        Said::Refused(Refusal::Stale)
    }
}

/// What a failure of `ResetLogin` means before it is explained: `None` for
/// Central's refusal (to be explained), and an answer that did not arrive
/// is no refusal.
fn at_reset(failure: Failure) -> Option<Said> {
    match failure {
        Failure::Rejected => None,
        Failure::Busy => Some(Said::Other(Problem::Busy)),
        Failure::Forbidden | Failure::Conflict => Some(Said::Other(Problem::Failed)),
        Failure::Unavailable | Failure::Malformed | Failure::Network => Some(Said::Lost),
    }
}

/// What a failed sign-in once more means; `None` when this browser turns
/// out not to be signed in.
fn at_sign_in(trouble: Trouble) -> Option<Said> {
    match trouble {
        Trouble::Current => Some(Said::Password),
        Trouble::SignedOut => None,
        Trouble::Unsure => Some(Said::Other(Problem::Failed)),
        Trouble::Other(problem) => Some(Said::Other(problem)),
    }
}

/// The search as Central reads it, checked by Central's own rule before
/// it is sent: `Ok(None)` lists every account.
fn query(typed: &str) -> Result<Option<String>, ()> {
    let typed = typed.trim();
    if typed.is_empty() {
        return Ok(None);
    }
    AccountQuery::parse(typed).map_err(|_| ())?;
    Ok(Some(typed.to_owned()))
}

/// A code Central made, held by the step that shows it. Neither `Clone`
/// nor `Debug`: the step shares it, and `ResetCode` wipes its bytes when
/// the last share goes.
struct Made {
    code: ResetCode,
    expires_at: i64,
    row: Row,
}

/// Where the page stands.
#[derive(Clone)]
enum Step {
    /// The search and its list.
    List,
    /// Reading one account.
    Reading(Row),
    /// One account: its facts and the way to a code.
    Account(Opened),
    /// The warning before a code.
    Warning(Opened),
    /// The password once more.
    Password(Opened),
    /// Signing in once more and asking for the code.
    Working(Opened),
    /// The code, shown once.
    Code(Arc<Made>),
}

/// The list as it was read.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Listing {
    Loading,
    Ready,
    Failed(&'static str),
}

/// The page's signals and what it was given. All of it is `Copy`.
#[derive(Clone, Copy)]
struct Admin {
    lang: Lang,
    worker: StoredValue<String>,
    username: StoredValue<String>,
    own: [u8; 16],
    step: RwSignal<Step>,
    rows: RwSignal<Vec<Row>>,
    more: RwSignal<bool>,
    listing: RwSignal<Listing>,
    adding: RwSignal<bool>,
    /// The search the list shows.
    asked: StoredValue<Option<String>>,
    /// The newest list request: an older one's answer is not drawn.
    generation: StoredValue<u32>,
    typed: RwSignal<String>,
    search_error: RwSignal<Option<&'static str>>,
    /// What the account step says of the last attempt.
    said: RwSignal<Option<Said>>,
    password: RwSignal<String>,
    signed_out: Callback<()>,
}

/// Where this browser's account stands, asked after Central refused a
/// read: a refusal is a lost session only if Central refuses this too.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Standing {
    SignedOut,
    /// Signed in, without the role.
    NotAdmin,
    /// Signed in, with the role.
    Admin,
    /// Not known.
    Unknown,
}

async fn standing() -> Standing {
    match call(&Request::AccountSummary).await {
        Err(Failure::Rejected) => Standing::SignedOut,
        Ok(Response::AccountSummary { admin: false, .. }) => Standing::NotAdmin,
        Ok(Response::AccountSummary { admin: true, .. }) => Standing::Admin,
        _ => Standing::Unknown,
    }
}

/// What the page says of a refused read in `standing`; `None` where
/// nobody is signed in. An administrator is refused an account that is
/// gone, and no list (the search was checked before it was sent).
fn unread(standing: Standing) -> Option<Said> {
    match standing {
        Standing::SignedOut => None,
        Standing::NotAdmin => Some(Said::Refused(Refusal::Role)),
        Standing::Admin => Some(Said::Refused(Refusal::Gone)),
        Standing::Unknown => Some(Said::Other(Problem::Failed)),
    }
}

impl Admin {
    /// This browser is not signed in any more: the area says so.
    fn leave(self) {
        session::forget();
        self.signed_out.run(());
    }

    /// What to say of a read Central refused, or `None` once the area was
    /// told that nobody is signed in.
    async fn refused_read(self) -> Option<Said> {
        let said = unread(standing().await);
        if said.is_none() {
            self.leave();
        }
        said
    }

    /// Reads a page of accounts: the first of the search `asked`, or the
    /// one after `before`.
    fn read(self, before: Option<[u8; 16]>) {
        let Some(generation) = self.generation.try_get_value() else {
            return;
        };
        let generation = if before.is_none() {
            generation.wrapping_add(1)
        } else {
            generation
        };
        self.generation.set_value(generation);
        let query = self.asked.get_value();
        leptos::task::spawn_local(async move {
            let request = Request::Admin {
                request: AdminRequest::Accounts { query, before },
            };
            let answer = call(&request).await;
            // a newer search, or a page that is gone, draws its own
            if self.generation.try_get_value() != Some(generation) {
                return;
            }
            match answer {
                Ok(Response::Admin {
                    answer: AdminAnswer::Accounts { accounts, more },
                }) => {
                    let found: Vec<Row> = accounts.into_iter().map(Row::from).collect();
                    let next = found.first().map(Row::key);
                    if before.is_some() {
                        self.rows.update(|rows| rows.extend(found));
                    } else {
                        self.rows.set(found);
                    }
                    self.more.set(more);
                    self.listing.set(Listing::Ready);
                    // after "Show more" the keyboard goes on at the first new account
                    if before.is_some()
                        && let Some(key) = next
                    {
                        focus::after_render(format!("#admin-open-{key}"));
                    }
                }
                Ok(_) => self
                    .listing
                    .set(Listing::Failed(problem_text(self.lang, Problem::Failed))),
                Err(Failure::Rejected) => {
                    // the role, or a session, that is gone; never an account
                    if let Some(said) = self.refused_read().await {
                        let said = match said {
                            Said::Refused(Refusal::Gone) => Said::Other(Problem::Failed),
                            said => said,
                        };
                        self.listing
                            .set(Listing::Failed(said_text(self.lang, said)));
                    }
                }
                Err(failure) => self.listing.set(Listing::Failed(problem_text(
                    self.lang,
                    Problem::outside_sign_in(failure),
                ))),
            }
            self.adding.set(false);
        });
    }

    /// The search's submit: a search Central would refuse is not sent.
    fn search(self, event: &leptos::ev::SubmitEvent) {
        event.prevent_default();
        let Ok(asked) = query(&self.typed.get_untracked()) else {
            self.search_error
                .set(Some(words::admin_search_error(self.lang)));
            focus::after_render("#admin-search");
            return;
        };
        self.search_error.set(None);
        self.asked.set_value(asked);
        self.rows.set(Vec::new());
        self.more.set(false);
        self.listing.set(Listing::Loading);
        self.read(None);
    }

    /// "Show more": the page after the last account shown.
    fn more(self) {
        if self.adding.get_untracked() {
            return;
        }
        let Some(last) = self
            .rows
            .with_untracked(|rows| rows.last().map(|row| row.id))
        else {
            return;
        };
        self.adding.set(true);
        self.read(Some(last));
    }

    /// Steps to `step` and puts the keyboard where it begins.
    fn go(self, step: Step, focus_on: &str) {
        self.step.set(step);
        focus::after_render(focus_on.to_owned());
    }

    /// Back to the list, at the account the person came from.
    fn back(self, row: &Row) {
        self.said.set(None);
        self.password.set(String::new());
        self.go(Step::List, &format!("#admin-open-{}", row.key()));
    }

    /// Opens one account: reads it afresh with its recovery state.
    fn open(self, row: Row) {
        self.said.set(None);
        self.go(Step::Reading(row.clone()), "#admin-reading");
        leptos::task::spawn_local(async move {
            let request = Request::Admin {
                request: AdminRequest::Account { account: row.id },
            };
            match call(&request).await {
                Ok(Response::Admin {
                    answer:
                        AdminAnswer::Account {
                            account, recovery, ..
                        },
                }) => {
                    let fresh = Row::from(account);
                    self.replace(&fresh);
                    self.go(
                        Step::Account(Opened {
                            row: fresh,
                            recovery,
                        }),
                        "#admin-account-title",
                    );
                }
                Ok(_) => self.unopened(row, Said::Other(Problem::Failed)),
                Err(Failure::Rejected) => {
                    if let Some(said) = self.refused_read().await {
                        self.unopened(row, said);
                    }
                }
                Err(failure) => {
                    self.unopened(row, Said::Other(Problem::outside_sign_in(failure)));
                }
            }
        });
    }

    /// The account could not be read: said where it was being read.
    fn unopened(self, row: Row, said: Said) {
        self.said.set(Some(said));
        self.go(Step::Reading(row), "#admin-error");
    }

    /// Puts `fresh` in the list in place of the row it read again.
    fn replace(self, fresh: &Row) {
        let _ = self.rows.try_update(|rows| {
            if let Some(row) = rows.iter_mut().find(|row| row.id == fresh.id) {
                row.clone_from(fresh);
            }
        });
    }

    /// The password's submit: signs in once more, then asks for the code.
    fn make(self, event: &leptos::ev::SubmitEvent, opened: Opened) {
        event.prevent_default();
        let password = self.password.get_untracked();
        if password.is_empty() {
            self.said.set(Some(Said::NoPassword));
            focus::after_render("#admin-password");
            return;
        }
        self.password.set(String::new());
        self.said.set(None);
        self.go(Step::Working(opened.clone()), "#admin-working");
        let worker = self.worker.get_value();
        let own = self.own;
        leptos::task::spawn_local(async move {
            let outcome = issue(&worker, password, own, opened.row.id).await;
            match outcome {
                Issued::Code(code, expires_at) => {
                    let mut row = opened.row;
                    row.reset_until = Some(expires_at);
                    self.replace(&row);
                    let made = Made {
                        code,
                        expires_at,
                        row,
                    };
                    self.go(Step::Code(Arc::new(made)), "#admin-code-title");
                }
                Issued::SignedOut => self.leave(),
                Issued::Said(Said::Password) => {
                    self.said.set(Some(Said::Password));
                    self.go(Step::Password(opened), "#admin-password");
                }
                Issued::Said(said) => self.settle(opened, said).await,
                Issued::Refused => {
                    let explained = why(own, opened.row.id).await;
                    match explained {
                        Why::SignedOut => self.leave(),
                        Why::Said(said, fresh) => {
                            let opened = fresh.unwrap_or(opened);
                            self.replace(&opened.row);
                            // where the account read again says it, the
                            // reason stands in the button's place and takes
                            // the keyboard; it is not said twice
                            let shown = matches!(said, Said::Refused(why) if refusal(&opened.row, own) == Some(why));
                            self.said.set((!shown).then_some(said));
                            let at = if shown {
                                "#admin-refusal"
                            } else {
                                "#admin-error"
                            };
                            self.go(Step::Account(opened), at);
                        }
                    }
                }
            }
        });
    }

    /// No code is shown: the account step says why, with the account read
    /// again after an answer that did not arrive (a code may stand now).
    async fn settle(self, opened: Opened, said: Said) {
        let opened = if said == Said::Lost {
            read_account(opened.row.id).await.unwrap_or(opened)
        } else {
            opened
        };
        self.replace(&opened.row);
        self.said.set(Some(said));
        self.go(Step::Account(opened), "#admin-error");
    }

    /// The code step is left: the code goes with it, and the list shows the
    /// account with its code's expiry.
    fn done(self, made: &Made) {
        self.back(&made.row);
    }

    /// Leaves the code step without a word, for a page that is put away.
    #[cfg(target_arch = "wasm32")]
    fn forget(self) {
        let showing = self
            .step
            .try_with_untracked(|step| matches!(step, Step::Code(_)))
            .unwrap_or(false);
        if showing {
            self.step.set(Step::List);
        }
    }
}

/// How asking for a code ended.
enum Issued {
    Code(ResetCode, i64),
    SignedOut,
    /// Central refused: to be explained.
    Refused,
    Said(Said),
}

/// Signs in once more with `password` in a Worker ended at once, checks
/// that the session is still `own`'s, and asks for `account`'s code.
async fn issue(worker: &str, password: String, own: [u8; 16], account: [u8; 16]) -> Issued {
    let Some(crypto) = Crypto::start(worker) else {
        return Issued::Said(Said::Other(Problem::Failed));
    };
    let signed = sign_in_again(&crypto, &password).await;
    crypto.close();
    drop(password);
    let identity = match signed {
        Ok((identity, _)) => identity,
        Err(trouble) => {
            return at_sign_in(trouble).map_or(Issued::SignedOut, Issued::Said);
        }
    };
    if identity.account != own {
        return Issued::Said(Said::Switched);
    }
    let request = Request::Admin {
        request: AdminRequest::ResetLogin { account },
    };
    match call(&request).await {
        Ok(Response::Admin {
            answer: AdminAnswer::ResetCode { code, expires_at },
        }) => Issued::Code(code, expires_at),
        // an answer of another kind says nothing of what happened
        Ok(_) => Issued::Said(Said::Lost),
        Err(failure) => at_reset(failure).map_or(Issued::Refused, Issued::Said),
    }
}

/// An account read again, with its recovery state.
async fn read_account(account: [u8; 16]) -> Option<Opened> {
    let request = Request::Admin {
        request: AdminRequest::Account { account },
    };
    match call(&request).await {
        Ok(Response::Admin {
            answer: AdminAnswer::Account {
                account, recovery, ..
            },
        }) => Some(Opened {
            row: Row::from(account),
            recovery,
        }),
        _ => None,
    }
}

/// What Central's refusal of a code is explained as.
enum Why {
    SignedOut,
    /// The reason, and the account as Central holds it now where it was read.
    Said(Said, Option<Opened>),
}

/// Asks Central what stands now, to explain its refusal of `account`'s code.
async fn why(own: [u8; 16], account: [u8; 16]) -> Why {
    let (admin, signed_in) = match call(&Request::AccountSummary).await {
        Ok(Response::AccountSummary {
            admin,
            signed_in_at,
            ..
        }) => (admin, signed_in_at),
        Err(Failure::Rejected) => return Why::SignedOut,
        _ => return Why::Said(Said::Other(Problem::Failed), None),
    };
    if !admin {
        return Why::Said(Said::Refused(Refusal::Role), None);
    }
    let now = i64::try_from(super::pow::now_ms() / 1000).unwrap_or(i64::MAX);
    let request = Request::Admin {
        request: AdminRequest::Account { account },
    };
    let read = match call(&request).await {
        Ok(Response::Admin {
            answer: AdminAnswer::Account {
                account, recovery, ..
            },
        }) => Ok(Opened {
            row: Row::from(account),
            recovery,
        }),
        Ok(_) => Err(Failure::Malformed),
        Err(failure) => Err(failure),
    };
    let said = explain(
        true,
        signed_in,
        now,
        read.as_ref().map(|opened| &opened.row).map_err(|f| *f),
        own,
    );
    Why::Said(said, read.ok())
}

/// The facts of an account, as the list and the account step show them.
fn facts(lang: Lang, row: &Row, recovery: Option<bool>) -> impl IntoView + use<> {
    let state = match row.state {
        AccountState::Pending => words::admin_state_pending(lang),
        AccountState::Enrolled => words::admin_state_enrolled(lang),
        AccountState::Restored => words::admin_state_restored(lang),
    };
    let source = if row.moved {
        words::profile_source_moved(lang)
    } else {
        words::profile_source_registered(lang)
    };
    let last = row
        .last_sign_in_at
        .map_or_else(|| "–".to_owned(), |at| when(lang, at));
    let recovery = recovery.map(|has| {
        let answer = if has {
            words::admin_yes(lang)
        } else {
            words::admin_no(lang)
        };
        view! { <dt>{words::admin_recovery(lang)}</dt><dd>{answer}</dd> }
    });
    let reset = row.reset_until.map(|until| {
        view! { <dt>{words::reset_code(lang)}</dt><dd>{words::tokens_until(lang, &when(lang, until))}</dd> }
    });
    view! {
        <dl class="facts">
            <dt>{words::profile_signin_name(lang)}</dt>
            <dd><code translate="no">{row.username.clone()}</code></dd>
            <dt>{words::admin_state(lang)}</dt>
            <dd>{state}</dd>
            <dt>{words::profile_source(lang)}</dt>
            <dd>{source}</dd>
            <dt>{words::admin_created(lang)}</dt>
            <dd>{when(lang, row.created_at)}</dd>
            <dt>{words::admin_last_sign_in(lang)}</dt>
            <dd>{last}</dd>
            {recovery}
            {reset}
        </dl>
    }
}

/// An account's name with the role's pill.
fn title(lang: Lang, row: &Row) -> impl IntoView + use<> {
    let pill = row
        .admin
        .then(|| view! { <span class="pill info">{words::admin_pill(lang)}</span> });
    view! { <bdi translate="no">{row.name()}</bdi>{pill} }
}

fn row_view(a: Admin, row: &Row) -> impl IntoView + use<> {
    let lang = a.lang;
    let key = row.key();
    let name = row.name();
    let opened = row.clone();
    view! {
        <li class="card admin-account">
            <h3 class="admin-title">{title(lang, row)}</h3>
            {facts(lang, row, None)}
            <p class="actions">
                <button type="button" class="button" id=format!("admin-open-{key}") on:click=move |_| a.open(opened.clone())>
                    {words::admin_open(lang)}<span class="visually-hidden">{" "}<bdi translate="no">{name}</bdi></span>
                </button>
            </p>
        </li>
    }
}

/// The search and the accounts it found.
fn list_view(a: Admin) -> impl IntoView {
    let lang = a.lang;
    let results = move || {
        let rows = a.rows.get();
        match a.listing.get() {
            Listing::Loading => {
                view! { <p role="status" class="meta">{words::account_loading(lang)}</p> }
                    .into_any()
            }
            Listing::Failed(text) => view! {
                <p class="error" role="alert">{text}</p>
                <p class="actions">
                    <button type="button" class="button" on:click=move |_| {
                        a.listing.set(Listing::Loading);
                        a.read(None);
                    }>{words::error_again(lang)}</button>
                </p>
            }
            .into_any(),
            Listing::Ready if rows.is_empty() => {
                view! { <p role="status">{words::admin_none(lang)}</p> }.into_any()
            }
            Listing::Ready => {
                let count = rows.len().to_string();
                let items = rows.iter().map(|row| row_view(a, row)).collect_view();
                let more = a.more.get().then(|| {
                    view! {
                        <p class="actions">
                            <button type="button" class="button" aria-disabled=move || a.adding.get().then_some("true")
                                on:click=move |_| a.more()>{words::admin_more(lang)}</button>
                        </p>
                    }
                });
                view! {
                    <p role="status" class="meta">{words::admin_count(lang, &count)}</p>
                    <ul class="admin-accounts">{items}</ul>
                    {more}
                }
                .into_any()
            }
        }
    };
    view! {
        <div class="admin-list" hidden=move || a.step.with(|step| !matches!(step, Step::List))>
            <section class="card" aria-labelledby="admin-search-title">
                <h2 id="admin-search-title">{words::admin_find(lang)}</h2>
                <p>{words::admin_reset_lead(lang)}</p>
                <form class="form admin-search" method="post" novalidate=true on:submit=move |event| a.search(&event)>
                    <div class="field">
                        <label for="admin-search">{words::admin_search(lang)}</label>
                        <div class="with-tool">
                            <input id="admin-search" type="search" autocomplete="off" autocapitalize="none" spellcheck="false" maxlength="64"
                                aria-describedby="admin-search-hint"
                                on:input=move |ev| a.typed.set(event_target_value(&ev)) prop:value=move || a.typed.get() />
                            <button type="submit" class="button">{glyph::glyph(glyph::SEARCH)}<span>{words::admin_search_submit(lang)}</span></button>
                        </div>
                        <p class="hint" id="admin-search-hint">{words::admin_search_hint(lang)}</p>
                    </div>
                    <p class="error" role="alert">{move || a.search_error.get()}</p>
                </form>
            </section>
            <div class="admin-results">{results}</div>
        </div>
    }
}

fn alert(a: Admin) -> impl IntoView {
    view! {
        <p class="error" role="alert" id="admin-error" tabindex="-1">
            {move || a.said.get().map(|said| said_text(a.lang, said))}
        </p>
    }
}

fn back_button(a: Admin, row: Row) -> impl IntoView {
    view! {
        <button type="button" class="button quiet" on:click=move |_| a.back(&row)>
            {glyph::glyph(glyph::BACK)}<span>{words::admin_back(a.lang)}</span>
        </button>
    }
}

fn reading_view(a: Admin, row: &Row) -> AnyView {
    let lang = a.lang;
    view! {
        <section class="card step" aria-labelledby="admin-reading-title">
            <h2 id="admin-reading-title" class="admin-title">{title(lang, row)}</h2>
            <p class="meta" role="status" id="admin-reading" tabindex="-1">{move || a.said.get().is_none().then(|| words::account_loading(lang))}</p>
            {alert(a)}
            <p class="actions">
                {
                    let row = row.clone();
                    move || a.said.get().is_some().then(|| {
                        let row = row.clone();
                        view! {
                            <button type="button" class="button" on:click=move |_| a.open(row.clone())>{words::error_again(lang)}</button>
                        }
                    })
                }
                {back_button(a, row.clone())}
            </p>
        </section>
    }
    .into_any()
}

fn account_view(a: Admin, opened: &Opened) -> AnyView {
    let lang = a.lang;
    let row = opened.row.clone();
    let outstanding = row.reset_until.map(|until| {
        view! { <p class="meta">{words::admin_reset_outstanding(lang, &when(lang, until))}</p> }
    });
    // what Central would refuse is said in the button's place
    let why = refusal(&row, a.own);
    let notice = why.map(|why| {
        view! { <p class="notice" id="admin-refusal" tabindex="-1">{refusal_text(lang, why)}</p> }
    });
    let reset = why.is_none().then(|| {
        let next = opened.clone();
        view! {
            <button type="button" class="button primary" id="admin-reset"
                on:click=move |_| {
                    a.said.set(None);
                    a.go(Step::Warning(next.clone()), "#admin-warning-title");
                }>
                {glyph::glyph(glyph::KEY)}<span>{words::admin_reset(lang)}</span>
            </button>
        }
    });
    view! {
        <section class="card step" aria-labelledby="admin-account-title">
            <h2 id="admin-account-title" class="admin-title" tabindex="-1">{title(lang, &row)}</h2>
            {facts(lang, &row, Some(opened.recovery))}
            {outstanding}
            {notice}
            {alert(a)}
            <p class="actions">
                {reset}
                {back_button(a, row.clone())}
            </p>
        </section>
    }
    .into_any()
}

fn warning_view(a: Admin, opened: &Opened) -> AnyView {
    let lang = a.lang;
    let name = opened.row.name();
    let on = opened.clone();
    let off = opened.clone();
    view! {
        <section class="card step" aria-labelledby="admin-warning-title">
            <h2 id="admin-warning-title" tabindex="-1">{words::admin_warning_title(lang)}</h2>
            <p class="meta"><bdi translate="no">{name}</bdi></p>
            <p class="notice">{words::admin_warning_who(lang)}</p>
            <ul class="admin-warning">
                <li>{words::admin_warning_ends(lang)}</li>
                <li>{words::admin_warning_data(lang)}</li>
                <li>{words::admin_warning_replaces(lang)}</li>
                <li>{words::admin_warning_takeover(lang)}</li>
            </ul>
            <p class="actions">
                <button type="button" class="button primary" on:click=move |_| {
                    a.go(Step::Password(on.clone()), "#admin-password");
                }>{words::admin_warning_continue(lang)}</button>
                <button type="button" class="button quiet" on:click=move |_| {
                    a.go(Step::Account(off.clone()), "#admin-account-title");
                }>{words::admin_back(lang)}</button>
            </p>
        </section>
    }
    .into_any()
}

fn password_view(a: Admin, opened: Opened) -> AnyView {
    let lang = a.lang;
    let spec = Labelled {
        id: "admin-password",
        label: words::privacy_delete_password(lang),
        hint: Some(words::admin_password_hint(lang)),
        mark: glyph::LOCK,
        auto: "current-password",
        max: "1024",
    };
    let name = opened.row.name();
    let sent = opened.clone();
    let off = opened;
    view! {
        <section class="card step" aria-labelledby="admin-password-title">
            <h2 id="admin-password-title">{words::admin_make(lang)}</h2>
            <p class="meta"><bdi translate="no">{name}</bdi></p>
            <form class="form" method="post" novalidate=true on:submit=move |event| a.make(&event, sent.clone())>
                // for a password manager: whose password this is
                <input type="text" autocomplete="username" readonly=true hidden=true value=a.username.get_value() />
                {field::labelled(lang, spec, move || a.password.get(), move |value| a.password.set(value))}
                {alert(a)}
                <p class="actions">
                    <button type="submit" class="button primary">{glyph::glyph(glyph::KEY)}<span>{words::admin_make(lang)}</span></button>
                    <button type="button" class="button quiet" on:click=move |_| {
                        a.password.set(String::new());
                        a.said.set(None);
                        a.go(Step::Account(off.clone()), "#admin-account-title");
                    }>{words::admin_back(lang)}</button>
                </p>
            </form>
        </section>
    }
    .into_any()
}

fn code_view(a: Admin, made: &Arc<Made>) -> AnyView {
    let lang = a.lang;
    let code = made.code.display().to_string();
    let (copy, copied) = clip::copier(lang, code.clone());
    let left = Arc::clone(made);
    view! {
        <section class="card step secret-box" aria-labelledby="admin-code-title">
            <h2 id="admin-code-title" tabindex="-1">{words::admin_code_title(lang)}</h2>
            <p class="meta"><bdi translate="no">{made.row.name()}</bdi></p>
            <p class="eyebrow">{words::code_shown_once(lang)}</p>
            <p class="secret" translate="no"><code>{code}</code></p>
            <p>{words::admin_code_until(lang, &when(lang, made.expires_at))}</p>
            <p>{words::admin_code_lead(lang)}</p>
            <p class="actions">
                {copy}
                <button type="button" class="button primary" on:click=move |_| a.done(&left)>{words::admin_code_done(lang)}</button>
            </p>
            {copied}
        </section>
    }
    .into_any()
}

fn step_view(a: Admin) -> AnyView {
    let lang = a.lang;
    match a.step.get() {
        Step::List => ().into_any(),
        Step::Reading(row) => reading_view(a, &row),
        Step::Account(opened) => account_view(a, &opened),
        Step::Warning(opened) => warning_view(a, &opened),
        Step::Password(opened) => password_view(a, opened),
        Step::Working(opened) => view! {
            <section class="card step" aria-labelledby="admin-working-title">
                <h2 id="admin-working-title"><bdi translate="no">{opened.row.name()}</bdi></h2>
                <p class="working" role="status" tabindex="-1" id="admin-working">{words::signin_working(lang)}</p>
            </section>
        }
        .into_any(),
        Step::Code(made) => code_view(a, &made),
    }
}

/// The page for an administrator: the search, the list, and the steps to
/// a code. `own` is the account this browser is signed in as, `username`
/// its sign-in name (for a password manager); `signed_out` runs when this
/// browser turns out not to be signed in.
pub fn page(
    lang: Lang,
    worker: String,
    username: String,
    own: [u8; 16],
    signed_out: Callback<()>,
) -> impl IntoView {
    let moderation = super::moderation::section(lang, worker.clone());
    let a = Admin {
        lang,
        worker: StoredValue::new(worker),
        username: StoredValue::new(username),
        own,
        step: RwSignal::new(Step::List),
        rows: RwSignal::new(Vec::new()),
        more: RwSignal::new(false),
        listing: RwSignal::new(Listing::Loading),
        adding: RwSignal::new(false),
        asked: StoredValue::new(None),
        generation: StoredValue::new(0),
        typed: RwSignal::new(String::new()),
        search_error: RwSignal::new(None),
        said: RwSignal::new(None),
        password: RwSignal::new(String::new()),
        signed_out,
    };
    // Effects run in the browser only: the newest accounts first.
    Effect::new(move |_| a.read(None));
    // A page put away (another page opened, the tab closed, the
    // back-forward cache) takes no code with it. A page taken down drops
    // its signals, and the code with them.
    #[cfg(target_arch = "wasm32")]
    {
        let away = window_event_listener_untyped("pagehide", move |_| a.forget());
        on_cleanup(move || away.remove());
    }
    view! {
        {list_view(a)}
        <div id="admin-steps" class="steps">{move || step_view(a)}</div>
        {moderation}
    }
}

/// What the page shows anyone without the role: one sentence and the way
/// back. Nothing is asked of Central.
pub fn refused(lang: Lang) -> impl IntoView {
    view! {
        <section class="card app-empty">
            <p>{words::admin_only(lang)}</p>
            <p><a class="button" href=format!("/{}/account", lang.code())>{words::account_home(lang)}</a></p>
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OWN: [u8; 16] = [1; 16];

    fn row() -> Row {
        Row {
            id: [2; 16],
            username: "erika".into(),
            display_name: Some("Erika".into()),
            tag: Some(0x1a2b),
            moved: false,
            state: AccountState::Restored,
            created_at: 1_794_000_000,
            last_sign_in_at: None,
            admin: false,
            reset_until: None,
        }
    }

    const FAILURES: [Failure; 7] = [
        Failure::Rejected,
        Failure::Forbidden,
        Failure::Conflict,
        Failure::Busy,
        Failure::Unavailable,
        Failure::Malformed,
        Failure::Network,
    ];

    #[test]
    fn an_account_is_named_by_its_handle_or_its_sign_in_name() {
        assert_eq!(row().name(), "Erika#1a2b");
        let unnamed = Row {
            display_name: None,
            tag: None,
            ..row()
        };
        assert_eq!(unnamed.name(), "erika");
        assert_eq!(row().key(), "02".repeat(16));
    }

    /// What the row tells before anything is asked: the refusals Central
    /// gives for an administrator's account, one's own, a pending one and
    /// an unfinished move.
    #[test]
    fn the_row_says_which_accounts_get_no_code() {
        assert_eq!(refusal(&row(), OWN), None);
        let own = Row { id: OWN, ..row() };
        assert_eq!(refusal(&own, OWN), Some(Refusal::Own));
        let admin = Row {
            admin: true,
            ..row()
        };
        assert_eq!(refusal(&admin, OWN), Some(Refusal::Administrator));
        let pending = Row {
            state: AccountState::Pending,
            ..row()
        };
        assert_eq!(refusal(&pending, OWN), Some(Refusal::Pending));
        // a moved account not in use has not finished its move
        let moving = Row {
            moved: true,
            state: AccountState::Enrolled,
            ..row()
        };
        assert_eq!(refusal(&moving, OWN), Some(Refusal::Moving));
        // in use, it may have: Central decides
        let moved = Row {
            moved: true,
            ..row()
        };
        assert_eq!(refusal(&moved, OWN), None);
        // a registered account in use or only set up gets one
        let enrolled = Row {
            state: AccountState::Enrolled,
            ..row()
        };
        assert_eq!(refusal(&enrolled, OWN), None);
    }

    /// Central's refusal of a code is explained by what it says afterwards.
    #[test]
    fn a_refusal_is_explained_by_what_central_holds_now() {
        let now = 1_800_000_000;
        let fresh = now - 60;
        let old = now - FRESH;
        let explained = |admin, signed_in, read: Result<&Row, Failure>| {
            explain(admin, signed_in, now, read, OWN)
        };
        assert_eq!(
            explained(false, fresh, Ok(&row())),
            Said::Refused(Refusal::Role)
        );
        assert_eq!(
            explained(true, fresh, Err(Failure::Rejected)),
            Said::Refused(Refusal::Gone)
        );
        assert_eq!(
            explained(true, fresh, Err(Failure::Network)),
            Said::Other(Problem::Network)
        );
        let admin = Row {
            admin: true,
            ..row()
        };
        assert_eq!(
            explained(true, fresh, Ok(&admin)),
            Said::Refused(Refusal::Administrator)
        );
        let pending = Row {
            state: AccountState::Pending,
            ..row()
        };
        assert_eq!(
            explained(true, fresh, Ok(&pending)),
            Said::Refused(Refusal::Pending)
        );
        assert_eq!(
            explained(true, fresh, Ok(&Row { id: OWN, ..row() })),
            Said::Refused(Refusal::Own)
        );
        // a resettable registered account is refused only for the sign-in's age
        assert_eq!(
            explained(true, fresh, Ok(&row())),
            Said::Refused(Refusal::Stale)
        );
        assert_eq!(
            explained(true, old, Ok(&row())),
            Said::Refused(Refusal::Stale)
        );
        // a moved one in use: its move, unless the sign-in is old
        let moved = Row {
            moved: true,
            ..row()
        };
        assert_eq!(
            explained(true, fresh, Ok(&moved)),
            Said::Refused(Refusal::Moving)
        );
        assert_eq!(
            explained(true, old, Ok(&moved)),
            Said::Refused(Refusal::Stale)
        );
    }

    /// An answer that did not arrive is never a refusal, and a refusal is
    /// never one: only `Rejected` is explained, the rest is said as it is.
    #[test]
    fn a_lost_answer_is_no_refusal() {
        for failure in FAILURES {
            let said = at_reset(failure);
            match failure {
                Failure::Rejected => assert_eq!(said, None),
                Failure::Unavailable | Failure::Malformed | Failure::Network => {
                    assert_eq!(said, Some(Said::Lost));
                }
                Failure::Busy => assert_eq!(said, Some(Said::Other(Problem::Busy))),
                Failure::Forbidden | Failure::Conflict => {
                    assert_eq!(said, Some(Said::Other(Problem::Failed)));
                }
            }
            assert_ne!(said, Some(Said::Password));
        }
        assert_eq!(at_sign_in(Trouble::Current), Some(Said::Password));
        assert_eq!(at_sign_in(Trouble::SignedOut), None);
        assert_eq!(
            at_sign_in(Trouble::Other(Problem::Busy)),
            Some(Said::Other(Problem::Busy))
        );
    }

    /// Every refusal has words of its own in every language, and none
    /// reads as another's or as a lost answer.
    #[test]
    fn every_refusal_has_its_own_sentence() {
        let all = [
            Refusal::Administrator,
            Refusal::Own,
            Refusal::Pending,
            Refusal::Moving,
            Refusal::Stale,
            Refusal::Role,
            Refusal::Gone,
        ];
        for lang in Lang::ALL {
            let texts: Vec<&str> = all.iter().map(|r| refusal_text(lang, *r)).collect();
            for (at, text) in texts.iter().enumerate() {
                assert_ne!(*text, "");
                assert!(!texts[..at].contains(text), "{lang:?}: {text}");
                assert_ne!(*text, said_text(lang, Said::Lost));
            }
        }
    }

    /// A search is checked by Central's own rule before it is sent.
    #[test]
    fn a_search_central_refuses_is_not_sent() {
        assert_eq!(query(""), Ok(None));
        assert_eq!(query("   "), Ok(None));
        assert_eq!(query(" Erika "), Ok(Some("Erika".into())));
        assert_eq!(query("#1a2b"), Ok(Some("#1a2b".into())));
        for bad in ["#", "#xyz", "#123456789", "jürgen", &"e".repeat(65)] {
            assert_eq!(query(bad), Err(()), "{bad}");
        }
    }

    /// The requests as Central reads them.
    #[test]
    fn the_requests_serialize_as_central_reads_them() {
        let text = |request: AdminRequest| {
            serde_json::to_string(&Request::Admin { request }).expect("json")
        };
        assert_eq!(
            text(AdminRequest::Accounts {
                query: Some("erika".into()),
                before: None,
            }),
            r#"{"operation":"admin","request":{"action":"accounts","query":"erika","before":null}}"#
        );
        assert!(
            text(AdminRequest::ResetLogin { account: [2; 16] }).starts_with(
                r#"{"operation":"admin","request":{"action":"reset_login","account":[2,"#
            )
        );
        assert!(
            text(AdminRequest::Account { account: [2; 16] })
                .starts_with(r#"{"operation":"admin","request":{"action":"account","account":[2,"#)
        );
    }
}
