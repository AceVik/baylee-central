//! What the list and a report's page share: who reads (the administrator,
//! the administrators a report may go to), the notices that say what an
//! action did (with "Undo" where it can be undone), the pills of a kind,
//! a status and a priority, how a refusal is explained, and the
//! confirmation before deleting for good.

use central_api::admin::AdminRequest;
use central_api::reports::{BulkAction, Priority, ReportAdmin, ReportKind, ReportStatus};
use leptos::prelude::*;

use super::super::auth::{Problem, problem_text};
use super::super::central::Failure;
use super::super::chat::glyph::{self, Shape};
use super::super::crypto::Crypto;
use super::super::field::{self, Labelled};
use super::super::password::{Trouble, sign_in_again};
use super::super::{focus, session};
use super::api::{self, Standing};
use super::text::{kind_word, priority_text, priority_tone, short_id, status_tone, status_word};
use crate::words::{self, Lang};

/// Central's password sign-in that deleting for good needs, in seconds, and
/// a minute's margin for the clocks.
const FRESH: i64 = 30 * 60 - 60;

/// `trash-2`.
pub const TRASH: &[Shape] = &[
    Shape::Path("M10 11v6"),
    Shape::Path("M14 11v6"),
    Shape::Path("M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"),
    Shape::Path("M3 6h18"),
    Shape::Path("M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"),
];
/// `refresh-cw`.
pub const REFRESH: &[Shape] = &[
    Shape::Path("M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"),
    Shape::Path("M21 3v5h-5"),
    Shape::Path("M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"),
    Shape::Path("M8 16H3v5"),
];
/// `chevron-left`.
pub const PREVIOUS: &[Shape] = &[Shape::Path("m15 18-6-6 6-6")];
/// `chevron-right`.
pub const NEXT: &[Shape] = &[Shape::Path("m9 18 6-6-6-6")];
/// `download`.
pub const DOWNLOAD: &[Shape] = &[
    Shape::Path("M12 15V3"),
    Shape::Path("M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"),
    Shape::Path("m7 10 5 5 5-5"),
];
/// `funnel`.
pub const FILTER: &[Shape] = &[Shape::Path(
    "M10 20a1 1 0 0 0 .553.895l2 1A1 1 0 0 0 14 21v-7a2 2 0 0 1 .517-1.341L21.74 4.67A1 1 0 0 0 21 3H3a1 1 0 0 0-.742 1.67l7.225 7.989A2 2 0 0 1 10 14z",
)];
/// `external-link`.
pub const EXTERNAL: &[Shape] = &[
    Shape::Path("M15 3h6v6"),
    Shape::Path("M10 14 21 3"),
    Shape::Path("M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"),
];

/// One notice: what an action did, or that it failed.
#[derive(Clone)]
pub struct Notice {
    id: u32,
    text: String,
    bad: bool,
    undo: Option<Callback<()>>,
}

/// The page's notices: a live region at the foot of the screen, at most
/// three at once. A failure stays until it is dismissed.
#[derive(Clone, Copy)]
pub struct Notices {
    items: RwSignal<Vec<Notice>>,
    /// The newest notice's words alone, for a screen reader.
    said: RwSignal<String>,
    next: StoredValue<u32>,
}

impl Notices {
    fn new() -> Self {
        Notices {
            items: RwSignal::new(Vec::new()),
            said: RwSignal::new(String::new()),
            next: StoredValue::new(1),
        }
    }

    fn push(self, text: String, bad: bool, undo: Option<Callback<()>>) {
        let Some(id) = self.next.try_get_value() else {
            return;
        };
        self.next.set_value(id.wrapping_add(1));
        let lasts = undo.is_some();
        self.items.update(|items| {
            items.push(Notice {
                id,
                text: text.clone(),
                bad,
                undo,
            });
            let extra = items.len().saturating_sub(3);
            items.drain(..extra);
        });
        // the same words twice are announced twice
        self.said.update(|said| {
            *said = if *said == text {
                format!("{text} ")
            } else {
                text
            };
        });
        if !bad {
            #[cfg(target_arch = "wasm32")]
            let _ = set_timeout(
                move || self.dismiss(id),
                std::time::Duration::from_secs(if lasts { 10 } else { 5 }),
            );
            #[cfg(not(target_arch = "wasm32"))]
            let _ = lasts;
        }
    }

    /// Says what happened.
    pub fn say(self, text: impl Into<String>) {
        self.push(text.into(), false, None);
    }

    /// Says what happened, with "Undo" beside it for a while.
    pub fn say_undo(self, text: impl Into<String>, undo: Callback<()>) {
        self.push(text.into(), false, Some(undo));
    }

    /// Says what failed; it stays until dismissed.
    pub fn fail(self, text: impl Into<String>) {
        self.push(text.into(), true, None);
    }

    fn dismiss(self, id: u32) {
        let _ = self.items.try_update(|items| items.retain(|n| n.id != id));
    }
}

/// The notices as the page draws them.
pub fn notices_view(lang: Lang, notices: Notices) -> impl IntoView {
    let items = move || {
        notices
            .items
            .get()
            .into_iter()
            .map(|n| {
                let id = n.id;
                let undo = n.undo.map(|undo| {
                    view! {
                        <button type="button" class="button button-ghost button-sm" on:click=move |_| {
                            notices.dismiss(id);
                            undo.run(());
                        }>{words::fb_undo(lang)}</button>
                    }
                });
                view! {
                    <div class="fb-notice" class:bad=n.bad>
                        <span>{n.text}</span>
                        {undo}
                        <button type="button" class="round-btn" on:click=move |_| notices.dismiss(id)>
                            {glyph::glyph(glyph::CLOSE)}
                            <span class="visually-hidden">{words::fb_dismiss(lang)}</span>
                        </button>
                    </div>
                }
            })
            .collect_view()
    };
    view! {
        <p class="visually-hidden" role="status">{move || notices.said.get()}</p>
        <div class="fb-notices">{items}</div>
    }
}

/// What both pages are given and share.
#[derive(Clone, Copy)]
pub struct Ctx {
    /// The page's language.
    pub lang: Lang,
    /// The reading administrator.
    pub me: [u8; 16],
    /// The crypto Worker's address, for a sign-in once more.
    pub worker: StoredValue<String>,
    /// The administrator's sign-in name, for a password manager.
    pub username: StoredValue<String>,
    signed_out: Callback<()>,
    /// The administrators a report may be assigned to, once read.
    pub admins: RwSignal<Vec<ReportAdmin>>,
    /// The page's notices.
    pub notices: Notices,
}

/// How a refused read or change is explained.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    /// This browser's account lost the administrator role.
    Role,
    /// Central refused it for this report or this list.
    Refused,
    /// Where the account stands is not known.
    Failed,
}

impl Ctx {
    /// The context of a page for the administrator `me`.
    pub fn new(
        lang: Lang,
        me: [u8; 16],
        worker: String,
        username: String,
        signed_out: Callback<()>,
    ) -> Self {
        Ctx {
            lang,
            me,
            worker: StoredValue::new(worker),
            username: StoredValue::new(username),
            signed_out,
            admins: RwSignal::new(Vec::new()),
            notices: Notices::new(),
        }
    }

    /// This browser is not signed in any more: the area says so.
    pub fn leave(self) {
        session::forget();
        self.signed_out.run(());
    }

    /// Reads the administrators once; without them, ids are shown short
    /// and assigning offers the reader alone.
    pub fn read_admins(self) {
        leptos::task::spawn_local(async move {
            if let Ok(admins) = api::admins().await {
                let _ = self.admins.try_set(admins);
            }
        });
    }

    /// An administrator by their handle, a former one by the end of their
    /// id; nobody as a dash.
    #[must_use]
    pub fn name(self, id: Option<[u8; 16]>) -> String {
        let Some(id) = id else {
            return "—".to_owned();
        };
        self.admins
            .with_untracked(|admins| {
                admins.iter().find(|a| a.id == id).and_then(|a| {
                    a.display_name
                        .as_deref()
                        .zip(a.tag)
                        .map(|(name, tag)| central_api::profile::handle(name, tag))
                })
            })
            .unwrap_or_else(|| format!("…{}", short_id(&id)))
    }

    /// As [`Self::name`], and "Me" for the reader.
    #[must_use]
    pub fn who(self, id: [u8; 16]) -> String {
        if id == self.me {
            words::fb_me(self.lang).to_owned()
        } else {
            self.name(Some(id))
        }
    }

    /// Everyone a report may be assigned to: the reader first, then the
    /// other administrators, then `also` where it is none of them.
    #[must_use]
    pub fn people(self, also: Option<[u8; 16]>) -> Vec<[u8; 16]> {
        let mut people = vec![self.me];
        self.admins.with(|admins| {
            people.extend(admins.iter().map(|a| a.id).filter(|id| *id != self.me));
        });
        if let Some(also) = also.filter(|a| !people.contains(a)) {
            people.push(also);
        }
        people
    }

    /// What a refusal (401) means, from where this browser's account stands
    /// now; `None` once the area was told that nobody is signed in.
    pub async fn explain(self) -> Option<Why> {
        match api::standing().await {
            Standing::SignedOut => {
                self.leave();
                None
            }
            Standing::NotAdmin => Some(Why::Role),
            Standing::Admin(_) => Some(Why::Refused),
            Standing::Unknown => Some(Why::Failed),
        }
    }

    /// The words of `why`, with `refused` for Central's refusal.
    #[must_use]
    pub fn why_text(self, why: Why, refused: &'static str) -> &'static str {
        match why {
            Why::Role => words::admin_refused_role(self.lang),
            Why::Refused => refused,
            Why::Failed => words::error_failed(self.lang),
        }
    }

    /// A read that failed for another reason than a refusal, in words.
    #[must_use]
    pub fn read_failed(self, failure: Failure) -> &'static str {
        problem_text(self.lang, Problem::outside_sign_in(failure))
    }
}

/// Whether a change's failure means its answer did not arrive (it may
/// have been made).
#[must_use]
pub const fn lost(failure: Failure) -> bool {
    matches!(
        failure,
        Failure::Unavailable | Failure::Malformed | Failure::Network
    )
}

/// A kind as a pill.
pub fn kind_pill(lang: Lang, kind: ReportKind) -> impl IntoView + use<> {
    let class = if kind == ReportKind::Crash {
        "pill bad"
    } else {
        "pill"
    };
    view! { <span class=class>{kind_word(lang, kind)}</span> }
}

/// A status as a pill.
pub fn status_pill(lang: Lang, status: ReportStatus) -> impl IntoView + use<> {
    view! { <span class=status_tone(status)>{status_word(lang, status)}</span> }
}

/// A priority as a pill; nothing without one.
pub fn priority_pill(lang: Lang, priority: Option<Priority>) -> impl IntoView + use<> {
    priority.map(|p| {
        view! {
            <span class=priority_tone(p)>
                <span class="visually-hidden">{words::fb_priority(lang)}": "</span>
                {priority_text(lang, Some(p))}
            </span>
        }
    })
}

/// Where the confirmation before deleting for good stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Asking whether.
    Confirm,
    /// Asking for the password once more.
    Password,
    /// Sent, or signing in once more.
    Working,
}

/// How deleting for good ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Purged {
    /// Deleted, this many.
    Done(u32),
    /// Central deleted nothing (`fb-purge-refused`).
    Refused,
    /// No answer came back: some may be gone.
    Lost,
}

/// The confirmation before deleting for good: the question, the password
/// once more where Central counts the sign-in as old (a purge is the one
/// triage change that needs a fresh one, `AdminRequest::fresh`), and one
/// request, never two.
#[derive(Clone, Copy)]
pub struct Purge {
    c: Ctx,
    /// The reports asked about; none while nothing is asked.
    pub asking: RwSignal<Option<Vec<[u8; 16]>>>,
    stage: RwSignal<Stage>,
    password: RwSignal<String>,
    said: RwSignal<Option<&'static str>>,
    /// One report's page (`PurgeReport`), or a selection (`ReportsBulk`).
    one: bool,
    /// Where the keyboard goes back to when it is cancelled.
    back: &'static str,
    /// How it ended, for the page to act on (taken there).
    pub ended: RwSignal<Option<Purged>>,
}

impl Purge {
    /// The confirmation of a page.
    pub fn new(c: Ctx, one: bool, back: &'static str) -> Self {
        Purge {
            c,
            asking: RwSignal::new(None),
            stage: RwSignal::new(Stage::Confirm),
            password: RwSignal::new(String::new()),
            said: RwSignal::new(None),
            one,
            back,
            ended: RwSignal::new(None),
        }
    }

    /// Asks whether `reports` are to be deleted for good. Nothing is sent.
    pub fn open(self, reports: Vec<[u8; 16]>) {
        if reports.is_empty() {
            return;
        }
        self.asking.set(Some(reports));
        self.stage.set(Stage::Confirm);
        self.said.set(None);
        self.password.set(String::new());
        focus::after_render("#fb-purge-title");
    }

    fn cancel(self) {
        self.asking.set(None);
        self.password.set(String::new());
        self.said.set(None);
        focus::after_render(self.back);
    }

    fn end(self, how: Purged) {
        self.asking.set(None);
        self.password.set(String::new());
        self.said.set(None);
        self.stage.set(Stage::Confirm);
        self.ended.set(Some(how));
    }

    fn say(self, said: &'static str, stage: Stage) {
        self.said.set(Some(said));
        self.stage.set(stage);
        focus::after_render(if stage == Stage::Password {
            "#fb-purge-password"
        } else {
            "#fb-purge-error"
        });
    }

    /// "Delete them for good": whether the sign-in is fresh, then the
    /// request or the password. A second press while it works does nothing.
    fn confirm(self) {
        if self.stage.get_untracked() != Stage::Confirm {
            return;
        }
        self.stage.set(Stage::Working);
        self.said.set(None);
        let lang = self.c.lang;
        leptos::task::spawn_local(async move {
            match api::standing().await {
                Standing::SignedOut => self.c.leave(),
                Standing::NotAdmin => self.say(words::admin_refused_role(lang), Stage::Confirm),
                Standing::Unknown => self.say(words::error_failed(lang), Stage::Confirm),
                Standing::Admin(at) if now() - at < FRESH => self.send().await,
                Standing::Admin(_) => {
                    self.said.set(None);
                    self.stage.set(Stage::Password);
                    focus::after_render("#fb-purge-password");
                }
            }
        });
    }

    /// The password's submit: a sign-in once more, then the request.
    fn submit(self, event: &leptos::ev::SubmitEvent) {
        event.prevent_default();
        if self.stage.get_untracked() != Stage::Password {
            return;
        }
        let lang = self.c.lang;
        let password = self.password.get_untracked();
        if password.is_empty() {
            self.say(words::admin_password_missing(lang), Stage::Password);
            return;
        }
        self.password.set(String::new());
        self.stage.set(Stage::Working);
        self.said.set(None);
        let worker = self.c.worker.get_value();
        leptos::task::spawn_local(async move {
            let Some(crypto) = Crypto::start(&worker) else {
                self.say(words::error_failed(lang), Stage::Password);
                return;
            };
            let signed = sign_in_again(&crypto, &password).await;
            crypto.close();
            drop(password);
            match signed {
                Ok((identity, _)) if identity.account == self.c.me => self.send().await,
                Ok(_) => self.say(words::fb_purge_switched(lang), Stage::Confirm),
                Err(Trouble::Current) => {
                    self.say(words::password_error_current(lang), Stage::Password);
                }
                Err(Trouble::SignedOut) => self.c.leave(),
                Err(Trouble::Unsure) => self.say(words::error_failed(lang), Stage::Password),
                Err(Trouble::Other(problem)) => {
                    self.say(problem_text(lang, problem), Stage::Password);
                }
            }
        });
    }

    /// The one request.
    async fn send(self) {
        let lang = self.c.lang;
        let Some(reports) = self.asking.get_untracked() else {
            return;
        };
        let sent = match (self.one, reports.as_slice()) {
            (true, [report]) => api::done(AdminRequest::PurgeReport { report: *report })
                .await
                .map(|()| 1),
            _ => api::bulk(reports, BulkAction::Purge).await,
        };
        match sent {
            Ok(n) => self.end(Purged::Done(n)),
            Err(Failure::Rejected) => match api::standing().await {
                Standing::SignedOut => self.c.leave(),
                Standing::NotAdmin => self.say(words::admin_refused_role(lang), Stage::Confirm),
                Standing::Admin(at) if now() - at >= FRESH => {
                    self.say(words::admin_refused_stale(lang), Stage::Password);
                }
                Standing::Admin(_) => self.end(Purged::Refused),
                Standing::Unknown => self.say(words::error_failed(lang), Stage::Confirm),
            },
            Err(Failure::Busy) => self.say(words::error_busy(lang), Stage::Confirm),
            Err(failure) if lost(failure) => self.end(Purged::Lost),
            Err(_) => self.say(words::error_failed(lang), Stage::Confirm),
        }
    }

    /// The confirmation as the page draws it, where something is asked.
    pub fn view(self) -> impl IntoView {
        let lang = self.c.lang;
        move || {
            let count = self.asking.with(|a| a.as_ref().map(Vec::len))?;
            let n = words::number(lang, u64::try_from(count).unwrap_or(u64::MAX));
            let working = move || self.stage.get() == Stage::Working;
            let password = move || {
                (self.stage.get() == Stage::Password).then(|| {
                    let spec = Labelled {
                        id: "fb-purge-password",
                        label: words::privacy_delete_password(lang),
                        hint: Some(words::fb_purge_password(lang)),
                        mark: glyph::LOCK,
                        auto: "current-password",
                        max: "1024",
                    };
                    view! {
                        <form class="form" method="post" novalidate=true on:submit=move |event| self.submit(&event)>
                            // for a password manager: whose password this is
                            <input type="text" autocomplete="username" readonly=true hidden=true value=self.c.username.get_value() />
                            {field::labelled(lang, spec, move || self.password.get(), move |value| self.password.set(value))}
                            <p class="actions">
                                <button type="submit" class="button button-danger">{glyph::glyph(TRASH)}<span>{words::fb_purge_go(lang)}</span></button>
                                <button type="button" class="button quiet" on:click=move |_| self.cancel()>{words::fb_cancel(lang)}</button>
                            </p>
                        </form>
                    }
                })
            };
            let asking = move || {
                (self.stage.get() != Stage::Password).then(|| {
                    view! {
                        <p class="actions">
                            <button type="button" class="button button-danger" id="fb-purge-go"
                                aria-disabled=move || working().then_some("true") on:click=move |_| self.confirm()>
                                {glyph::glyph(TRASH)}<span>{words::fb_purge_go(lang)}</span>
                            </button>
                            <button type="button" class="button quiet" aria-disabled=move || working().then_some("true")
                                on:click=move |_| if !working() { self.cancel() }>{words::fb_cancel(lang)}</button>
                        </p>
                    }
                })
            };
            Some(view! {
                <section class="card fb-confirm" aria-labelledby="fb-purge-title">
                    <h2 id="fb-purge-title" tabindex="-1">{words::fb_purge_title(lang)}</h2>
                    <p>{words::fb_purge_text(lang, &n)}</p>
                    <p class="error" role="alert" id="fb-purge-error" tabindex="-1">{move || self.said.get()}</p>
                    {password}
                    {asking}
                    <p class="meta" role="status">{move || working().then(|| words::account_loading(lang))}</p>
                </section>
            })
        }
    }
}

/// The browser's clock, in Unix seconds.
fn now() -> i64 {
    i64::try_from(super::super::pow::now_ms() / 1000).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An answer that did not arrive is never a refusal: a change may have
    /// been made, so the page reads again instead of saying "refused".
    #[test]
    fn a_lost_answer_is_no_refusal() {
        for failure in [Failure::Unavailable, Failure::Malformed, Failure::Network] {
            assert!(lost(failure), "{failure:?}");
        }
        for failure in [
            Failure::Rejected,
            Failure::Forbidden,
            Failure::Conflict,
            Failure::Busy,
        ] {
            assert!(!lost(failure), "{failure:?}");
        }
    }

    /// Deleting for good asks for the password a minute before Central
    /// would refuse, so the two clocks never disagree on the edge.
    #[test]
    fn a_purge_wants_a_sign_in_younger_than_central_s_limit() {
        const { assert!(FRESH < 30 * 60) };
        const { assert!(FRESH > 25 * 60) };
    }
}
