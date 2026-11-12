//! `/{lang}/account/admin/feedback`: the reports, triaged. The views with
//! their counts, the search and every filter in the address (`query`),
//! shown as pills; facets, sortable columns, pages, saved lists, the
//! chart, the crash groups; a selection's actions and the CSV. A report
//! opens on its own page, which steps through this list. The list never
//! waits for its counts or its facets.

use central_api::admin::AdminRequest;
use central_api::reports::{
    BulkAction, Channel, CrashGroup, MAX_BULK, MAX_PAGE, MAX_SEEN, MAX_VIEW_NAME, MAX_VIEWS,
    ReportChange, ReportFacets, ReportFilter, ReportKind, ReportSort, ReportStatus, ReportSummary,
    SavedView, TRASH_DAYS, valid_label, valid_view,
};
use leptos::prelude::*;

use super::super::account::when;
use super::super::central::Failure;
use super::super::chat::glyph;
use super::super::{clip, focus};
use super::api::{self, browser};
use super::chart;
use super::csv::{EXPORT_MAX, file_name, to_csv};
use super::github::issue_url;
use super::query::{
    self, Assignee, DONE, ListQuery, OPEN, PAGE_SIZES, PRIORITIES, Pill, View, changed, list_path,
    report_path, request_of, starts_ascending, toggled, view_of, without,
};
use super::shared::{
    Ctx, DOWNLOAD, FILTER, NEXT, PREVIOUS, Purge, Purged, REFRESH, TRASH, kind_pill, lost,
    notices_view, priority_pill, status_pill,
};
use super::text::{
    alias, day_of, first_line, hex, kind_word, priority_text, short_id, size, status_word,
};
use crate::words::{self, Lang};

/// The list as it was read.
#[derive(Clone, Debug)]
enum Load {
    Loading,
    Failed(&'static str),
    Ready(Vec<ReportSummary>, u64),
}

/// A part read when it is first shown.
#[derive(Clone, Debug)]
enum Lazy<T> {
    Idle,
    Loading,
    Failed(&'static str),
    Ready(T),
}

/// The list page's signals. All of it is `Copy`.
#[derive(Clone, Copy)]
struct List {
    c: Ctx,
    query: RwSignal<ListQuery>,
    load: RwSignal<Load>,
    /// A read is on its way.
    busy: RwSignal<bool>,
    /// The newest list request: an older one's answer is not drawn.
    generation: StoredValue<u32>,
    facets: RwSignal<Option<ReportFacets>>,
    /// What the facets shown count over (`query::scope`, written).
    facet_scope: StoredValue<Option<String>>,
    counts: RwSignal<[Option<u64>; 7]>,
    checked: RwSignal<Vec<[u8; 16]>>,
    filters_open: RwSignal<bool>,
    groups: RwSignal<bool>,
    crash: RwSignal<Lazy<Vec<CrashGroup>>>,
    /// A selection's request is on its way.
    working: RwSignal<bool>,
    exporting: RwSignal<bool>,
    typed: RwSignal<String>,
    /// Typing's newest pause, for the search that follows it.
    typing: StoredValue<u32>,
    saved: RwSignal<Lazy<Vec<SavedView>>>,
    view_name: RwSignal<String>,
    bulk_label: RwSignal<String>,
    purge: Purge,
}

fn count(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

impl List {
    fn code(self) -> &'static str {
        self.c.lang.code()
    }

    /// The reports shown.
    fn rows(self) -> Vec<ReportSummary> {
        self.load.with(|load| match load {
            Load::Ready(rows, _) => rows.clone(),
            _ => Vec::new(),
        })
    }

    fn total(self) -> u64 {
        self.load.with(|load| match load {
            Load::Ready(_, total) => *total,
            _ => 0,
        })
    }

    /// The selected reports as the list shows them.
    fn selected(self) -> Vec<ReportSummary> {
        let checked = self.checked.get();
        self.rows()
            .into_iter()
            .filter(|r| checked.contains(&r.id))
            .collect()
    }

    /// The ids of the selected reports the list shows.
    fn chosen(self) -> Vec<[u8; 16]> {
        self.selected().into_iter().map(|r| r.id).collect()
    }

    /// Shows the list `q`: its address, its reports, its facets.
    fn go(self, q: ListQuery) {
        // another list brings its own search; typing that reads the same stays
        if query::clean_query(&self.typed.get_untracked()) != q.q {
            self.typed.set(q.q.clone().unwrap_or_default());
        }
        self.checked.set(Vec::new());
        remember(&list_path(self.code(), &q));
        self.query.set(q);
        self.read();
        self.read_facets(false);
        if self.groups.get_untracked() {
            self.read_groups();
        }
    }

    /// Changes the list, back on its first page.
    fn set(self, change: impl FnOnce(&mut ListQuery)) {
        self.go(changed(&self.query.get_untracked(), change));
    }

    /// Reads the page the query asks for.
    fn read(self) {
        let Some(generation) = self.generation.try_get_value() else {
            return;
        };
        let generation = generation.wrapping_add(1);
        self.generation.set_value(generation);
        self.busy.set(true);
        let asked = request_of(&self.query.get_untracked());
        leptos::task::spawn_local(async move {
            let answer = api::reports(&asked).await;
            if self.generation.try_get_value() != Some(generation) {
                return;
            }
            let load = match answer {
                Ok((rows, total)) => {
                    // a report the list no longer shows is no longer selected
                    let _ = self
                        .checked
                        .try_update(|c| c.retain(|id| rows.iter().any(|r| r.id == *id)));
                    Load::Ready(rows, total)
                }
                Err(Failure::Rejected) => match self.c.explain().await {
                    Some(why) => {
                        Load::Failed(self.c.why_text(why, words::fb_err_list(self.c.lang)))
                    }
                    None => return,
                },
                Err(failure) => Load::Failed(self.c.read_failed(failure)),
            };
            let _ = self.load.try_set(load);
            let _ = self.busy.try_set(false);
        });
    }

    /// Reads the facets of the list's scope, unless they are the ones shown.
    fn read_facets(self, again: bool) {
        let scope = query::scope(&self.query.get_untracked());
        let key = query::write(&scope);
        if !again && self.facet_scope.try_get_value().flatten().as_deref() == Some(&key) {
            return;
        }
        self.facet_scope.set_value(Some(key.clone()));
        let filter = query::filter_of(&scope);
        leptos::task::spawn_local(async move {
            // the filters work without their counts
            if let Ok(facets) = api::facets(filter).await
                && self.facet_scope.try_get_value().flatten().as_deref() == Some(&key)
            {
                let _ = self.facets.try_set(Some(facets));
            }
        });
    }

    /// The views' counts: the open reports' facets, and the trash's total.
    fn recount(self) {
        let me = self.c.me;
        leptos::task::spawn_local(async move {
            let open = ReportFilter {
                statuses: OPEN.to_vec(),
                ..ReportFilter::default()
            };
            if let Ok(f) = api::facets(open).await {
                let status = |s: ReportStatus| {
                    f.statuses
                        .iter()
                        .find(|c| c.value == s)
                        .map_or(0, |c| c.count)
                };
                let assignee = |a: Option<[u8; 16]>| {
                    f.assignees
                        .iter()
                        .find(|c| c.value == a)
                        .map_or(0, |c| c.count)
                };
                let _ = self.counts.try_update(|counts| {
                    counts[0] = Some(status(ReportStatus::New));
                    counts[1] = Some(f.total);
                    counts[2] = Some(assignee(Some(me)));
                    counts[3] = Some(assignee(None));
                    counts[4] = Some(DONE.iter().map(|s| status(*s)).sum());
                    counts[5] = Some(f.statuses.iter().map(|c| c.count).sum());
                });
            }
            let trash = query::Asked {
                filter: ReportFilter {
                    trash: true,
                    ..ReportFilter::default()
                },
                sort: ReportSort::Received,
                descending: true,
                offset: 0,
                limit: 1,
            };
            if let Ok((_, total)) = api::reports(&trash).await {
                let _ = self.counts.try_update(|counts| counts[6] = Some(total));
            }
        });
    }

    /// The list, its counts and its facets read again.
    fn again(self) {
        self.read();
        self.recount();
        self.read_facets(true);
        if self.groups.get_untracked() {
            self.read_groups();
        }
    }

    fn read_groups(self) {
        self.crash.set(Lazy::Loading);
        let filter = query::filter_of(&query::scope(&self.query.get_untracked()));
        let lang = self.c.lang;
        leptos::task::spawn_local(async move {
            let load = match api::crash_groups(filter).await {
                Ok(groups) => Lazy::Ready(groups),
                Err(Failure::Rejected) => match self.c.explain().await {
                    Some(why) => Lazy::Failed(self.c.why_text(why, words::fb_err_list(lang))),
                    None => return,
                },
                Err(failure) => Lazy::Failed(self.c.read_failed(failure)),
            };
            let _ = self.crash.try_set(load);
        });
    }

    /// A change, trash or restore of `ids` in one request: every one or none.
    fn bulk(self, ids: Vec<[u8; 16]>, action: BulkAction) {
        if ids.is_empty() || ids.len() > MAX_BULK || self.working.get_untracked() {
            return;
        }
        if let BulkAction::Change { change } = &action
            && change.validate(true).is_err()
        {
            return;
        }
        self.working.set(true);
        let lang = self.c.lang;
        let notices = self.c.notices;
        leptos::task::spawn_local(async move {
            let answer = api::bulk(ids.clone(), action.clone()).await;
            match answer {
                Ok(n) => {
                    let n = words::number(lang, u64::from(n));
                    match action {
                        BulkAction::Trash => {
                            let undo = Callback::new(move |()| self.restore(ids.clone()));
                            notices.say_undo(words::fb_trashed_n(lang, &n), undo);
                        }
                        BulkAction::Restore => notices.say(words::fb_restored_n(lang, &n)),
                        _ => notices.say(words::fb_changed_n(lang, &n)),
                    }
                    if !matches!(action, BulkAction::Change { .. }) {
                        self.checked.set(Vec::new());
                    }
                }
                Err(failure) => self.failed(failure).await,
            }
            let _ = self.working.try_set(false);
            self.again();
        });
    }

    /// "Undo" of moving to the trash.
    fn restore(self, ids: Vec<[u8; 16]>) {
        let lang = self.c.lang;
        leptos::task::spawn_local(async move {
            match api::bulk(ids, BulkAction::Restore).await {
                Ok(n) => self.c.notices.say(words::fb_restored_n(
                    lang,
                    &words::number(lang, u64::from(n)),
                )),
                Err(failure) => self.failed(failure).await,
            }
            self.again();
        });
    }

    /// A change that failed, said; an answer that did not arrive is no refusal.
    async fn failed(self, failure: Failure) {
        let lang = self.c.lang;
        let text = match failure {
            Failure::Rejected => match self.c.explain().await {
                Some(why) => self.c.why_text(why, words::fb_err_refused(lang)),
                None => return,
            },
            Failure::Busy => words::error_busy(lang),
            failure if lost(failure) => words::fb_lost_list(lang),
            _ => words::error_failed(lang),
        };
        self.c.notices.fail(text);
    }

    /// Marks `of` opened or not for this administrator.
    fn seen(self, of: Vec<[u8; 16]>, seen: bool) {
        if of.is_empty() || of.len() > MAX_SEEN {
            return;
        }
        let lang = self.c.lang;
        // the mark is a convenience: shown at once, the next read says how it is
        self.load.update(|load| {
            if let Load::Ready(rows, _) = load {
                for row in rows.iter_mut().filter(|r| of.contains(&r.id)) {
                    row.seen = seen;
                }
            }
        });
        leptos::task::spawn_local(async move {
            let n = words::number(lang, count(of.len()));
            match api::done(AdminRequest::SetReportsSeen { reports: of, seen }).await {
                Ok(()) => self.c.notices.say(if seen {
                    words::fb_marked_read(lang, &n)
                } else {
                    words::fb_marked_unread(lang, &n)
                }),
                Err(failure) => {
                    self.failed(failure).await;
                    self.read();
                }
            }
        });
    }

    fn copy_ids(self, ids: Vec<[u8; 16]>) {
        let lang = self.c.lang;
        let text = ids.iter().map(|id| hex(id)).collect::<Vec<_>>().join("\n");
        leptos::task::spawn_local(async move {
            if clip::write(&text).await {
                self.c.notices.say(words::fb_ids_copied(
                    lang,
                    &words::number(lang, count(ids.len())),
                ));
            } else {
                self.c.notices.fail(words::fb_copy_failed(lang));
            }
        });
    }

    /// `rows` as a CSV file the browser saves.
    fn export(self, rows: &[ReportSummary]) {
        if rows.is_empty() {
            return;
        }
        let lang = self.c.lang;
        let c = self.c;
        let csv = to_csv(rows, &|id| c.name(Some(id)));
        let day = day_of(i64::try_from(super::super::pow::now_ms() / 1000).unwrap_or(0));
        if browser::save(&file_name(&day), csv.as_bytes(), "text/csv;charset=utf-8") {
            self.c.notices.say(words::fb_exported(
                lang,
                &words::number(lang, count(rows.len())),
            ));
        }
    }

    /// Every report the filter matches, page by page, up to `EXPORT_MAX`.
    fn export_all(self) {
        if self.exporting.get_untracked() {
            return;
        }
        self.exporting.set(true);
        let base = request_of(&self.query.get_untracked());
        let total = self.total().min(EXPORT_MAX);
        leptos::task::spawn_local(async move {
            let mut all: Vec<ReportSummary> = Vec::new();
            let mut offset: u32 = 0;
            let mut failure = None;
            while u64::from(offset) < total {
                let left = u32::try_from(EXPORT_MAX - u64::from(offset)).unwrap_or(MAX_PAGE);
                let asked = query::Asked {
                    offset,
                    limit: MAX_PAGE.min(left),
                    ..base.clone()
                };
                match api::reports(&asked).await {
                    Ok((rows, _)) if rows.is_empty() => break,
                    Ok((rows, _)) => all.extend(rows),
                    Err(e) => {
                        failure = Some(e);
                        break;
                    }
                }
                offset = offset.saturating_add(MAX_PAGE);
            }
            match failure {
                Some(e) => self.failed(e).await,
                None => self.export(&all),
            }
            let _ = self.exporting.try_set(false);
        });
    }

    fn read_views(self, request: AdminRequest, said: Option<String>) {
        // a change keeps the lists shown while it goes
        if matches!(request, AdminRequest::ReportViews) {
            self.saved.set(Lazy::Loading);
        }
        let lang = self.c.lang;
        leptos::task::spawn_local(async move {
            match api::views(request).await {
                Ok(views) => {
                    let _ = self.saved.try_set(Lazy::Ready(views));
                    if let Some(said) = said {
                        self.c.notices.say(said);
                        let _ = self.view_name.try_set(String::new());
                    }
                }
                Err(failure) if said.is_none() => {
                    let text = match failure {
                        Failure::Rejected => words::fb_err_views(lang),
                        failure => self.c.read_failed(failure),
                    };
                    let _ = self.saved.try_set(Lazy::Failed(text));
                }
                Err(failure) => self.failed(failure).await,
            }
        });
    }
}

/// Puts `path` in the address bar in place of the list's address.
fn remember(path: &str) {
    #[cfg(target_arch = "wasm32")]
    if let Some(history) = leptos::web_sys::window().and_then(|w| w.history().ok()) {
        let _ =
            history.replace_state_with_url(&leptos::wasm_bindgen::JsValue::NULL, "", Some(path));
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = path;
}

/// The query string of the page's address.
fn address() -> String {
    leptos::web_sys::window()
        .and_then(|w| w.location().search().ok())
        .unwrap_or_default()
}

fn view_word(lang: Lang, view: View) -> &'static str {
    match view {
        View::Inbox => words::fb_view_inbox(lang),
        View::Open => words::fb_view_open(lang),
        View::Mine => words::fb_view_mine(lang),
        View::Unassigned => words::fb_view_unassigned(lang),
        View::Done => words::fb_view_done(lang),
        View::All => words::fb_view_all(lang),
        View::Trash => words::fb_view_trash(lang),
    }
}

fn sort_word(lang: Lang, sort: ReportSort) -> &'static str {
    match sort {
        ReportSort::Received => words::fb_sort_received(lang),
        ReportSort::Changed => words::fb_sort_changed(lang),
        ReportSort::Priority => words::fb_sort_priority(lang),
        ReportSort::Status => words::fb_sort_status(lang),
        ReportSort::Kind => words::fb_sort_kind(lang),
        ReportSort::Reporter => words::fb_sort_reporter(lang),
        ReportSort::Gateway => words::fb_sort_gateway(lang),
    }
}

/// A count beside a choice, where the facets name one.
fn counted(lang: Lang, n: Option<u64>) -> impl IntoView + use<> {
    n.map(|n| view! { " " <span class="fb-n">{words::number(lang, n)}</span> })
}

/// A pill's words.
fn pill_text(l: &List, pill: &Pill) -> String {
    let l = *l;
    let lang = l.c.lang;
    let part = |name: &str, value: &str| format!("{name}: {value}");
    match pill {
        Pill::Status(s) => part(words::fb_status(lang), status_word(lang, *s)),
        Pill::Kind(k) => part(words::fb_kind(lang), kind_word(lang, *k)),
        Pill::Priority(p) => part(words::fb_priority(lang), priority_text(lang, *p)),
        Pill::Assignee(Assignee::Nobody) => part(words::fb_assignee(lang), words::fb_nobody(lang)),
        Pill::Assignee(Assignee::Admin(id)) => part(words::fb_assignee(lang), &l.c.who(*id)),
        Pill::Label(label) => part(words::fb_label(lang), label),
        Pill::Gateway(name) => part(words::fb_gateway(lang), name),
        Pill::Channel(Channel::Gateway) => words::fb_channel_gateway(lang).to_owned(),
        Pill::Channel(Channel::Direct) => words::fb_channel_direct(lang).to_owned(),
        Pill::Reporter(r) => part(words::fb_reporter(lang), &alias(r).0),
        Pill::Signature(s) => words::fb_crash_group(lang, &s.chars().take(8).collect::<String>()),
        Pill::DuplicateOf(id) => words::fb_duplicates_of(lang, &short_id(id)),
        Pill::Search(text) => words::fb_search_pill(lang, text),
        Pill::From(day) => part(words::fb_from(lang), day),
        Pill::To(day) => part(words::fb_to(lang), day),
        Pill::Record(true) => words::fb_with_record(lang).to_owned(),
        Pill::Record(false) => words::fb_without_record(lang).to_owned(),
        Pill::Unseen => words::fb_only_unread(lang).to_owned(),
        Pill::Trash => words::fb_view_trash(lang).to_owned(),
    }
}

fn views_view(l: &List) -> impl IntoView + use<> {
    let l = *l;
    let lang = l.c.lang;
    let me = l.c.me;
    let chips = View::ALL
        .into_iter()
        .enumerate()
        .map(|(at, view)| {
            let pressed = move || l.query.with(|q| view_of(q, me) == Some(view)).to_string();
            let n = move || counted(lang, l.counts.with(|c| c[at]));
            view! {
                <button type="button" class="chip-btn" id=format!("fb-view-{at}") aria-pressed=pressed
                    on:click=move |_| l.go(view.applied(&l.query.get_untracked(), me))>
                    {view_word(lang, view)}{n}
                </button>
            }
        })
        .collect_view();
    view! {
        <fieldset class="fb-views">
            <legend class="visually-hidden">{words::fb_views(lang)}</legend>
            {chips}
        </fieldset>
    }
}

fn search_view(l: &List) -> impl IntoView + use<> {
    let l = *l;
    let lang = l.c.lang;
    let ask = move || {
        let q = query::clean_query(&l.typed.get_untracked());
        if q != l.query.with_untracked(|now| now.q.clone()) {
            l.set(|n| n.q = q);
        }
    };
    let on_input = move |ev: leptos::ev::Event| {
        l.typed.set(event_target_value(&ev));
        // the list follows once typing pauses
        let Some(pause) = l.typing.try_get_value() else {
            return;
        };
        let pause = pause.wrapping_add(1);
        l.typing.set_value(pause);
        #[cfg(target_arch = "wasm32")]
        let _ = set_timeout(
            move || {
                if l.typing.try_get_value() == Some(pause) {
                    ask();
                }
            },
            std::time::Duration::from_millis(300),
        );
    };
    view! {
        <form class="fb-search" method="post" novalidate=true role="search"
            on:submit=move |ev| {
                ev.prevent_default();
                l.typing.update_value(|t| *t = t.wrapping_add(1));
                ask();
            }>
            <label class="visually-hidden" for="fb-search">{words::fb_search(lang)}</label>
            <input id="fb-search" type="search" autocomplete="off" spellcheck="false" maxlength="400"
                aria-describedby="fb-search-hint" prop:value=move || l.typed.get() on:input=on_input
                on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                    if ev.key() == "Escape" && !l.typed.get_untracked().is_empty() {
                        ev.prevent_default();
                        l.typed.set(String::new());
                        ask();
                    }
                } />
            <button type="submit" class="button">{glyph::glyph(glyph::SEARCH)}<span>{words::fb_search_submit(lang)}</span></button>
            <p class="hint" id="fb-search-hint">{words::fb_search_hint(lang)}</p>
        </form>
    }
}

/// One dimension the panel's chips filter by.
#[derive(Clone, Copy)]
struct Dim<T: 'static> {
    id: &'static str,
    legend: &'static str,
    all: &'static [T],
    word: fn(Lang, T) -> &'static str,
    on: fn(&ListQuery) -> &Vec<T>,
    put: fn(&mut ListQuery, Vec<T>),
    facet: fn(&ReportFacets, T) -> Option<u64>,
}

fn chips<T: Copy + PartialEq + Send + Sync + 'static>(
    l: &List,
    dim: Dim<T>,
) -> impl IntoView + use<T> {
    let l = *l;
    let lang = l.c.lang;
    let Dim {
        id,
        legend,
        all,
        word,
        on,
        put,
        facet,
    } = dim;
    let buttons = all
        .iter()
        .enumerate()
        .map(|(at, value)| {
            let value = *value;
            let pressed = move || l.query.with(|q| on(q).contains(&value)).to_string();
            let n = move || {
                counted(
                    lang,
                    l.facets.with(|f| f.as_ref().and_then(|f| facet(f, value))),
                )
            };
            view! {
                <button type="button" class="chip-btn" id=format!("{id}-{at}") aria-pressed=pressed
                    on:click=move |_| l.set(|q| {
                        let next = toggled(on(q), value, all);
                        put(q, next);
                    })>
                    {word(lang, value)}{n}
                </button>
            }
        })
        .collect_view();
    view! {
        <fieldset class="fb-chips">
            <legend>{legend}</legend>
            <div class="fb-chip-row">{buttons}</div>
        </fieldset>
    }
}

fn status_count(f: &ReportFacets, s: ReportStatus) -> Option<u64> {
    f.statuses.iter().find(|c| c.value == s).map(|c| c.count)
}

fn kind_count(f: &ReportFacets, k: ReportKind) -> Option<u64> {
    f.kinds.iter().find(|c| c.value == k).map(|c| c.count)
}

fn priority_count(f: &ReportFacets, p: Option<central_api::reports::Priority>) -> Option<u64> {
    f.priorities.iter().find(|c| c.value == p).map(|c| c.count)
}

/// A select of the filter panel: its label and its options.
fn field(id: &'static str, label: &'static str, select: impl IntoView) -> impl IntoView {
    view! {
        <div class="field fb-field">
            <label for=id>{label}</label>
            {select}
        </div>
    }
}

/// An option, chosen while `current` names its value.
fn opt(value: String, text: String, current: Signal<String>) -> impl IntoView {
    let v = value.clone();
    view! { <option value=value prop:selected=move || current.get() == v>{text}</option> }
}

/// A count in brackets after a choice's words, where the facets name one.
fn with_count(lang: Lang, word: &str, n: Option<u64>) -> String {
    match n {
        Some(n) => format!("{word} ({})", words::number(lang, n)),
        None => word.to_owned(),
    }
}

fn assignee_select(l: &List) -> impl IntoView + use<> {
    let l = *l;
    let lang = l.c.lang;
    let current = Signal::derive(move || {
        l.query.with(|q| match q.assignee {
            None => String::new(),
            Some(Assignee::Nobody) => "none".into(),
            Some(Assignee::Admin(id)) => hex(&id),
        })
    });
    let options = move || {
        let n = |a: Option<[u8; 16]>| {
            l.facets.with(|f| {
                f.as_ref()
                    .and_then(|f| f.assignees.iter().find(|c| c.value == a).map(|c| c.count))
            })
        };
        let chosen = l.query.with(|q| match q.assignee {
            Some(Assignee::Admin(id)) => Some(id),
            _ => None,
        });
        let mut options = vec![
            opt(String::new(), words::fb_anyone(lang).to_owned(), current).into_any(),
            opt(
                "none".into(),
                with_count(lang, words::fb_nobody(lang), n(None)),
                current,
            )
            .into_any(),
        ];
        for id in l.c.people(chosen) {
            options.push(
                opt(
                    hex(&id),
                    with_count(lang, &l.c.who(id), n(Some(id))),
                    current,
                )
                .into_any(),
            );
        }
        options.collect_view()
    };
    view! {
        <select id="fb-f-assignee" on:change=move |ev| {
            let v = event_target_value(&ev);
            l.set(|q| q.assignee = match v.as_str() {
                "" => None,
                "none" => Some(Assignee::Nobody),
                other => super::text::id_of(other).map(Assignee::Admin),
            });
        }>{options}</select>
    }
}

/// A select over the values a facet counts, the chosen one kept even
/// where the facets no longer name it.
#[derive(Clone, Copy)]
struct Named {
    id: &'static str,
    now: fn(&ListQuery) -> Option<String>,
    values: fn(&ReportFacets) -> Vec<(String, u64)>,
    shown: fn(&str) -> String,
    put: fn(&mut ListQuery, Option<String>),
}

fn named_select(l: &List, named: Named) -> impl IntoView + use<> {
    let l = *l;
    let lang = l.c.lang;
    let Named {
        id,
        now,
        values,
        shown,
        put,
    } = named;
    let current = Signal::derive(move || l.query.with(|q| now(q).unwrap_or_default()));
    let options = move || {
        let mut list = l
            .facets
            .with(|f| f.as_ref().map(values).unwrap_or_default());
        let chosen = current.get();
        if !chosen.is_empty() && !list.iter().any(|(v, _)| *v == chosen) {
            list.push((chosen.clone(), 0));
        }
        let mut options =
            vec![opt(String::new(), words::fb_any(lang).to_owned(), current).into_any()];
        for (value, n) in list {
            let text = with_count(lang, &shown(&value), (n > 0).then_some(n));
            options.push(opt(value, text, current).into_any());
        }
        options.collect_view()
    };
    view! {
        <select id=id on:change=move |ev| {
            let v = event_target_value(&ev);
            l.set(|q| put(q, (!v.is_empty()).then_some(v)));
        }>{options}</select>
    }
}

fn channel_select(l: &List) -> impl IntoView + use<> {
    let l = *l;
    let lang = l.c.lang;
    let current = Signal::derive(move || {
        l.query.with(|q| match q.channel {
            None => String::new(),
            Some(Channel::Gateway) => "gateway".into(),
            Some(Channel::Direct) => "direct".into(),
        })
    });
    let options = move || {
        let n = |c: Channel| {
            l.facets.with(|f| {
                f.as_ref()
                    .and_then(|f| f.channels.iter().find(|x| x.value == c).map(|x| x.count))
            })
        };
        vec![
            opt(String::new(), words::fb_any(lang).to_owned(), current).into_any(),
            opt(
                "gateway".into(),
                with_count(lang, words::fb_channel_gateway(lang), n(Channel::Gateway)),
                current,
            )
            .into_any(),
            opt(
                "direct".into(),
                with_count(lang, words::fb_channel_direct(lang), n(Channel::Direct)),
                current,
            )
            .into_any(),
        ]
        .collect_view()
    };
    view! {
        <select id="fb-f-channel" on:change=move |ev| {
            let v = event_target_value(&ev);
            l.set(|q| q.channel = match v.as_str() {
                "gateway" => Some(Channel::Gateway),
                "direct" => Some(Channel::Direct),
                _ => None,
            });
        }>{options}</select>
    }
}

fn record_select(l: &List) -> impl IntoView + use<> {
    let l = *l;
    let lang = l.c.lang;
    let current = Signal::derive(move || {
        l.query.with(|q| match q.record {
            None => String::new(),
            Some(true) => "yes".into(),
            Some(false) => "no".into(),
        })
    });
    view! {
        <select id="fb-f-record" on:change=move |ev| {
            let v = event_target_value(&ev);
            l.set(|q| q.record = match v.as_str() {
                "yes" => Some(true),
                "no" => Some(false),
                _ => None,
            });
        }>
            {opt(String::new(), words::fb_any(lang).to_owned(), current)}
            {opt("yes".into(), words::fb_with_record(lang).to_owned(), current)}
            {opt("no".into(), words::fb_without_record(lang).to_owned(), current)}
        </select>
    }
}

fn day_input(
    l: &List,
    id: &'static str,
    now: fn(&ListQuery) -> Option<String>,
    put: fn(&mut ListQuery, Option<String>),
) -> impl IntoView + use<> {
    let l = *l;
    view! {
        <input id=id type="date" prop:value=move || l.query.with(|q| now(q).unwrap_or_default())
            on:change=move |ev| {
                let v = event_target_value(&ev);
                l.set(|q| put(q, query::valid_day(&v).then_some(v)));
            } />
    }
}

fn sort_select(l: &List) -> impl IntoView + use<> {
    let l = *l;
    let lang = l.c.lang;
    let current = Signal::derive(move || {
        l.query
            .with(|q| format!("{}:{}", q.sort.as_str(), if q.asc { "asc" } else { "desc" }))
    });
    let sorts = ReportSort::ALL
        .iter()
        .flat_map(|sort| {
            let dirs = if starts_ascending(*sort) {
                [true, false]
            } else {
                [false, true]
            };
            dirs.map(|asc| {
                let word = sort_word(lang, *sort);
                let text = if asc {
                    words::fb_ascending(lang, word)
                } else {
                    words::fb_descending(lang, word)
                };
                let value = format!("{}:{}", sort.as_str(), if asc { "asc" } else { "desc" });
                opt(value, text, current)
            })
        })
        .collect_view();
    view! {
        <select id="fb-f-sort" on:change=move |ev| {
            let v = event_target_value(&ev);
            let (sort, dir) = v.split_once(':').unwrap_or(("received", "desc"));
            let sort = ReportSort::ALL.iter().copied().find(|s| s.as_str() == sort).unwrap_or(ReportSort::Received);
            l.set(|q| {
                q.sort = sort;
                q.asc = dir == "asc";
            });
        }>{sorts}</select>
    }
}

fn filters_view(l: &List) -> impl IntoView + use<> {
    let l = *l;
    let lang = l.c.lang;
    let q = l.query;
    let label = named_select(
        &l,
        Named {
            id: "fb-f-label",
            now: |q| q.label.clone(),
            values: |f| {
                f.labels
                    .iter()
                    .map(|c| (c.value.clone(), c.count))
                    .collect()
            },
            shown: str::to_owned,
            put: |q, v| q.label = v,
        },
    );
    let gateway = named_select(
        &l,
        Named {
            id: "fb-f-gateway",
            now: |q| q.gateway.clone(),
            values: |f| {
                f.gateways
                    .iter()
                    .map(|c| (c.value.clone(), c.count))
                    .collect()
            },
            shown: str::to_owned,
            put: |q, v| q.gateway = v,
        },
    );
    let reporter = named_select(
        &l,
        Named {
            id: "fb-f-reporter",
            now: |q| q.reporter.clone(),
            values: |f| {
                f.reporters
                    .iter()
                    .map(|c| (c.value.clone(), c.count))
                    .collect()
            },
            shown: |r| {
                let (name, short) = alias(r);
                format!("{name} {short}")
            },
            put: |q, v| q.reporter = v,
        },
    );
    view! {
        <div id="fb-filters" class="fb-filters" hidden=move || !l.filters_open.get()>
            <div class="fb-chip-groups">
                {chips(&l, Dim { id: "fb-f-status", legend: words::fb_status(lang), all: ReportStatus::ALL, word: status_word,
                    on: |q| &q.statuses, put: |q, v| q.statuses = v, facet: status_count })}
                {chips(&l, Dim { id: "fb-f-kind", legend: words::fb_kind(lang), all: ReportKind::ALL, word: kind_word,
                    on: |q| &q.kinds, put: |q, v| q.kinds = v, facet: kind_count })}
                {chips(&l, Dim { id: "fb-f-priority", legend: words::fb_priority(lang), all: &PRIORITIES, word: priority_text,
                    on: |q| &q.priorities, put: |q, v| q.priorities = v, facet: priority_count })}
            </div>
            <div class="fb-fields">
                {field("fb-f-assignee", words::fb_assignee(lang), assignee_select(&l))}
                {field("fb-f-label", words::fb_label(lang), label)}
                {field("fb-f-gateway", words::fb_gateway(lang), gateway)}
                {field("fb-f-channel", words::fb_channel(lang), channel_select(&l))}
                {field("fb-f-record", words::fb_record(lang), record_select(&l))}
                {field("fb-f-reporter", words::fb_reporter(lang), reporter)}
                {field("fb-f-from", words::fb_from(lang), day_input(&l, "fb-f-from", |q| q.from.clone(), |q, v| q.from = v))}
                {field("fb-f-to", words::fb_to(lang), day_input(&l, "fb-f-to", |q| q.to.clone(), |q, v| q.to = v))}
                {field("fb-f-sort", words::fb_sort(lang), sort_select(&l))}
            </div>
            <p class="fb-check-line">
                <input type="checkbox" id="fb-f-unseen" prop:checked=move || q.with(|q| q.unseen)
                    on:change=move |ev| {
                        let on = event_target_checked(&ev);
                        l.set(|q| q.unseen = on);
                    } />
                <label for="fb-f-unseen">{words::fb_only_unread(lang)}</label>
            </p>
        </div>
    }
}

fn pills_view(l: &List) -> impl IntoView + use<> {
    let l = *l;
    let lang = l.c.lang;
    move || {
        let q = l.query.get();
        let view = view_of(&q, l.c.me);
        let pills = query::pills_of(&q, view);
        if pills.is_empty() {
            return None;
        }
        let several = pills.len() > 1;
        let items = pills
            .iter()
            .map(|pill| {
                let text = pill_text(&l, pill);
                let pill = pill.clone();
                let q = q.clone();
                view! {
                    <li>
                        <button type="button" class="chip-btn fb-pill" on:click=move |_| {
                            let next = without(&q, &pill);
                            let left = query::pills_of(&next, view_of(&next, l.c.me)).len();
                            l.go(next);
                            focus::after_render(if left > 0 { "#fb-pills li:first-child button" } else { "#fb-filters-toggle" });
                        }>
                            <bdi>{text.clone()}</bdi>{glyph::glyph(glyph::CLOSE)}
                            <span class="visually-hidden">{words::fb_remove_filter(lang, &text)}</span>
                        </button>
                    </li>
                }
            })
            .collect_view();
        let clear = several.then(|| {
            let q = q.clone();
            view! {
                <li>
                    <button type="button" class="button button-ghost button-sm" on:click=move |_| {
                        let me = l.c.me;
                        let base = view.map_or_else(ListQuery::default, |v| v.applied(&q, me));
                        let next = ListQuery { sort: q.sort, asc: q.asc, limit: q.limit, ..base };
                        l.go(next);
                        focus::after_render("#fb-filters-toggle");
                    }>{words::fb_clear_filters(lang)}</button>
                </li>
            }
        });
        Some(view! {
            <ul id="fb-pills" class="fb-pills" aria-label=words::fb_active_filters(lang)>{items}{clear}</ul>
        })
    }
}

fn saved_view(l: &List) -> impl IntoView + use<> {
    let l = *l;
    let lang = l.c.lang;
    let current = move || query::write(&query::without_page(&l.query.get()));
    let body = move || {
        match l.saved.get() {
        Lazy::Idle | Lazy::Loading => {
            view! { <p class="meta" role="status">{words::account_loading(lang)}</p> }.into_any()
        }
        Lazy::Failed(text) => view! {
            <p class="error" role="alert">{text}</p>
            <p><button type="button" class="button button-ghost button-sm"
                on:click=move |_| l.read_views(AdminRequest::ReportViews, None)>{words::error_again(lang)}</button></p>
        }
        .into_any(),
        Lazy::Ready(views) => {
            let here = current();
            let saved_here = views.iter().any(|v| v.query == here);
            let full = views.len() >= MAX_VIEWS;
            let list = if views.is_empty() {
                view! { <p class="meta">{words::fb_no_views(lang)}</p> }.into_any()
            } else {
                let items = views
                    .iter()
                    .map(|v| {
                        let href = list_path(l.code(), &query::read(&v.query));
                        let target = query::read(&v.query);
                        let id = v.id;
                        let name = v.name.clone();
                        let gone = name.clone();
                        view! {
                            <li>
                                <a href=href aria-current=(v.query == here).then_some("true")
                                    on:click=move |ev| {
                                        ev.prevent_default();
                                        l.go(target.clone());
                                    }><bdi>{name.clone()}</bdi></a>
                                <button type="button" class="round-btn" on:click=move |_| {
                                    l.read_views(AdminRequest::DeleteReportView { view: id },
                                        Some(words::fb_view_removed(lang, &gone)));
                                }>
                                    {glyph::glyph(glyph::CLOSE)}
                                    <span class="visually-hidden">{words::fb_remove_view(lang, &name)}</span>
                                </button>
                            </li>
                        }
                    })
                    .collect_view();
                view! { <ul class="fb-view-list">{items}</ul> }.into_any()
            };
            let save = (!saved_here && !full).then(|| {
                view! {
                    <form class="fb-inline" method="post" novalidate=true on:submit=move |ev| {
                        ev.prevent_default();
                        let name = l.view_name.get_untracked().trim().to_owned();
                        let query = current();
                        if valid_view(&name, &query) {
                            l.read_views(AdminRequest::SaveReportView { name: name.clone(), query },
                                Some(words::fb_view_saved(lang, &name)));
                        }
                    }>
                        <label class="visually-hidden" for="fb-view-name">{words::fb_view_name(lang)}</label>
                        <input id="fb-view-name" type="text" autocomplete="off" maxlength=MAX_VIEW_NAME.to_string()
                            placeholder=words::fb_view_name(lang)
                            prop:value=move || l.view_name.get() on:input=move |ev| l.view_name.set(event_target_value(&ev)) />
                        <button type="submit" class="button button-ghost button-sm">{words::fb_save_view(lang)}</button>
                    </form>
                }
            });
            let full = full.then(|| {
                let n = words::number(lang, count(MAX_VIEWS));
                view! { <p class="meta">{words::fb_views_full(lang, &n)}</p> }
            });
            view! { {list}{save}{full} }.into_any()
        }
    }
    };
    view! {
        <details class="fb-menu" on:toggle=move |ev| {
            let open = event_target::<leptos::web_sys::HtmlDetailsElement>(&ev).open();
            if open && matches!(l.saved.get_untracked(), Lazy::Idle) {
                l.read_views(AdminRequest::ReportViews, None);
            }
        }>
            <summary class="button button-ghost button-sm">{words::fb_saved_views(lang)}</summary>
            <div class="card fb-menu-body">{body}</div>
        </details>
    }
}

/// A menu that acts as soon as a choice is made, then shows its question again.
fn acting(
    l: &List,
    id: &'static str,
    question: &'static str,
    options: AnyView,
    act: Callback<String>,
) -> impl IntoView + use<> {
    let l = *l;
    view! {
        <span class="fb-bulk-field">
            <label class="visually-hidden" for=id>{question}</label>
            <select id=id aria-disabled=move || l.working.get().then_some("true") on:change=move |ev| {
                let select = event_target::<leptos::web_sys::HtmlSelectElement>(&ev);
                let v = select.value();
                select.set_value("");
                if !v.is_empty() && !l.working.get_untracked() {
                    act.run(v);
                }
            }>
                <option value="" selected=true>{question}</option>
                {options}
            </select>
        </span>
    }
}

/// A selection's change to every selected report.
fn change_all(l: &List, change: ReportChange) {
    l.bulk(l.chosen(), BulkAction::Change { change });
}

/// The menus that set a status, a priority and an assignee.
fn setting_menus(l: &List) -> impl IntoView + use<> {
    let l = *l;
    let lang = l.c.lang;
    let statuses = ReportStatus::ALL
        .iter()
        .filter(|s| **s != ReportStatus::Duplicate)
        .map(|s| view! { <option value=s.as_str()>{status_word(lang, *s)}</option> })
        .collect_view()
        .into_any();
    let priorities = PRIORITIES
        .iter()
        .map(
            |p| view! { <option value=query::priority_word(*p)>{priority_text(lang, *p)}</option> },
        )
        .collect_view()
        .into_any();
    let mut people: Vec<AnyView> =
        l.c.people(None)
            .into_iter()
            .map(|id| view! { <option value=hex(&id)>{l.c.who(id)}</option> }.into_any())
            .collect();
    people.push(view! { <option value="none">{words::fb_nobody(lang)}</option> }.into_any());
    let status = acting(
        &l,
        "fb-bulk-status",
        words::fb_set_status(lang),
        statuses,
        Callback::new(move |v: String| {
            if let Ok(status) = ReportStatus::from_stored(&v) {
                change_all(
                    &l,
                    ReportChange {
                        status: Some(status),
                        ..ReportChange::default()
                    },
                );
            }
        }),
    );
    let priority = acting(
        &l,
        "fb-bulk-priority",
        words::fb_set_priority(lang),
        priorities,
        Callback::new(move |v: String| {
            if let Some(p) = PRIORITIES.iter().find(|p| query::priority_word(**p) == v) {
                change_all(
                    &l,
                    ReportChange {
                        priority: Some(*p),
                        ..ReportChange::default()
                    },
                );
            }
        }),
    );
    let assign = acting(
        &l,
        "fb-bulk-assign",
        words::fb_assign_to(lang),
        people.collect_view().into_any(),
        Callback::new(move |v: String| {
            let whom = if v == "none" {
                Some(None)
            } else {
                super::text::id_of(&v).map(Some)
            };
            if let Some(whom) = whom {
                change_all(
                    &l,
                    ReportChange {
                        assignee: Some(whom),
                        ..ReportChange::default()
                    },
                );
            }
        }),
    );
    view! { {status}{priority}{assign} }
}

/// Adding a label to every selected report, and taking one off.
fn label_menus(l: &List) -> impl IntoView + use<> {
    let l = *l;
    let lang = l.c.lang;
    let add_label = move || {
        let label = l.bulk_label.get_untracked().trim().to_ascii_lowercase();
        if valid_label(&label) {
            l.bulk_label.set(String::new());
            change_all(
                &l,
                ReportChange {
                    labels_add: vec![label],
                    ..ReportChange::default()
                },
            );
        }
    };
    let datalist = move || {
        l.facets
            .with(|f| {
                f.as_ref()
                    .map(|f| f.labels.iter().map(|c| c.value.clone()).collect::<Vec<_>>())
                    .unwrap_or_default()
            })
            .into_iter()
            .map(|label| view! { <option value=label></option> })
            .collect_view()
    };
    let mut theirs: Vec<String> = l.selected().into_iter().flat_map(|r| r.labels).collect();
    theirs.sort();
    theirs.dedup();
    let unlabel = (!theirs.is_empty()).then(|| {
        let options = theirs
            .into_iter()
            .map(|label| view! { <option value=label.clone()>{label.clone()}</option> })
            .collect_view()
            .into_any();
        acting(
            &l,
            "fb-bulk-unlabel",
            words::fb_remove_label_all(lang),
            options,
            Callback::new(move |v: String| {
                if valid_label(&v) {
                    change_all(
                        &l,
                        ReportChange {
                            labels_remove: vec![v],
                            ..ReportChange::default()
                        },
                    );
                }
            }),
        )
    });
    view! {
        <span class="fb-bulk-field fb-inline">
            <label class="visually-hidden" for="fb-bulk-label">{words::fb_add_label(lang)}</label>
            <input id="fb-bulk-label" type="text" list="fb-bulk-labels" autocomplete="off" spellcheck="false"
                maxlength="32" placeholder=words::fb_add_label(lang)
                prop:value=move || l.bulk_label.get() on:input=move |ev| l.bulk_label.set(event_target_value(&ev))
                on:keydown=move |ev: leptos::ev::KeyboardEvent| if ev.key() == "Enter" {
                    ev.prevent_default();
                    add_label();
                } />
            <datalist id="fb-bulk-labels">{datalist}</datalist>
            <button type="button" class="round-btn" aria-disabled=move || l.working.get().then_some("true") on:click=move |_| add_label()>
                {glyph::glyph(glyph::PLUS)}<span class="visually-hidden">{words::fb_add_label(lang)}</span>
            </button>
        </span>
        {unlabel}
    }
}

/// Merging the selection into one of its reports: the rest become its duplicates.
fn merge_menu(l: &List) -> Option<impl IntoView + use<>> {
    let l = *l;
    let lang = l.c.lang;
    let selected = l.selected();
    (selected.len() > 1).then(|| {
        let options = selected
            .iter()
            .filter(|r| r.duplicate_of.is_none())
            .map(|r| {
                let line = first_line(&r.excerpt, 40);
                let text = if line.is_empty() { words::fb_no_text(lang).to_owned() } else { line };
                view! { <option value=hex(&r.id)>{format!("…{} · {text}", short_id(&r.id))}</option> }
            })
            .collect_view()
            .into_any();
        acting(&l, "fb-bulk-merge", words::fb_merge_into(lang), options, Callback::new(move |v: String| {
            let Some(original) = super::text::id_of(&v) else { return };
            let rest: Vec<[u8; 16]> = l.chosen().into_iter().filter(|id| *id != original).collect();
            l.bulk(rest, BulkAction::Change {
                change: ReportChange { duplicate_of: Some(Some(original)), ..ReportChange::default() },
            });
        }))
    })
}

fn bulk_view(l: &List) -> impl IntoView + use<> {
    let l = *l;
    let lang = l.c.lang;
    let ids = move || l.chosen();
    let n = move || words::number(lang, count(l.chosen().len()));
    let working = move || l.working.get().then_some("true");
    let actions = move || {
        if l.query.with(|q| q.trash) {
            view! {
                <button type="button" class="button button-ghost button-sm" aria-disabled=working
                    on:click=move |_| if !l.working.get_untracked() { l.bulk(ids(), BulkAction::Restore) }>
                    {words::fb_restore(lang)}
                </button>
                <button type="button" class="button button-danger button-sm" id="fb-bulk-purge" aria-disabled=working
                    on:click=move |_| if !l.working.get_untracked() { l.purge.open(ids()) }>
                    {glyph::glyph(TRASH)}<span>{words::fb_purge(lang)}</span>
                </button>
            }
            .into_any()
        } else {
            view! {
                {setting_menus(&l)}
                {label_menus(&l)}
                {merge_menu(&l)}
                <button type="button" class="button button-danger button-sm" aria-disabled=working
                    on:click=move |_| if !l.working.get_untracked() { l.bulk(ids(), BulkAction::Trash) }>
                    {glyph::glyph(TRASH)}<span>{words::fb_to_trash(lang)}</span>
                </button>
            }
            .into_any()
        }
    };
    view! {
        <section class="fb-bulk" aria-label=words::fb_selection(lang) hidden=move || l.checked.with(Vec::is_empty) || l.groups.get()>
            <p class="fb-bulk-count">
                <strong>{move || words::fb_selected(lang, &n())}</strong>
                <button type="button" class="button button-ghost button-sm" on:click=move |_| {
                    l.checked.set(Vec::new());
                    focus::after_render("#fb-check-all");
                }>{words::fb_deselect(lang)}</button>
            </p>
            <div class="fb-bulk-actions">
                {actions}
                <button type="button" class="button button-ghost button-sm" on:click=move |_| l.seen(ids(), true)>{words::fb_mark_read(lang)}</button>
                <button type="button" class="button button-ghost button-sm" on:click=move |_| l.seen(ids(), false)>{words::fb_mark_unread(lang)}</button>
                <button type="button" class="button button-ghost button-sm" on:click=move |_| l.copy_ids(ids())>{words::fb_copy_ids(lang)}</button>
                <button type="button" class="button button-ghost button-sm" on:click=move |_| l.export(&l.selected())>
                    {move || words::fb_export_selected(lang, &n())}
                </button>
            </div>
        </section>
    }
}

/// A column head that orders the list by it.
fn sort_head(
    l: &List,
    sort: ReportSort,
    label: &'static str,
    class: &'static str,
) -> impl IntoView + use<> {
    let l = *l;
    let lang = l.c.lang;
    let current = move || l.query.with(|q| q.sort == sort);
    let aria = move || {
        current().then(|| {
            if l.query.with(|q| q.asc) {
                "ascending"
            } else {
                "descending"
            }
        })
    };
    let mark = move || {
        if current() {
            if l.query.with(|q| q.asc) {
                "↑"
            } else {
                "↓"
            }
        } else {
            ""
        }
    };
    view! {
        <th scope="col" class=class aria-sort=aria>
            <button type="button" class="fb-sort" id=format!("fb-sort-{}", sort.as_str()) on:click=move |_| {
                let (now, asc) = l.query.with_untracked(|q| (q.sort == sort, q.asc));
                l.set(|q| {
                    q.sort = sort;
                    q.asc = if now { !asc } else { starts_ascending(sort) };
                });
                focus::after_render(format!("#fb-sort-{}", sort.as_str()));
            }>
                {label}<span class="fb-sort-mark" aria-hidden="true">{mark}</span>
                <span class="visually-hidden">{words::fb_sort_by(lang, label)}</span>
            </button>
        </th>
    }
}

/// What a row says under the report's first line: its status (beside a
/// narrow column), labels, notes, original, record and issue.
fn row_marks(l: &List, r: &ReportSummary) -> impl IntoView + use<> {
    let l = *l;
    let lang = l.c.lang;
    let labels = r
        .labels
        .iter()
        .map(|label| {
            let pick = label.clone();
            let title = words::fb_only_label(lang, label);
            view! {
                <button type="button" class="fb-label" translate="no" title=title.clone() aria-label=title
                    on:click=move |_| l.set(|q| q.label = Some(pick.clone()))>{label.clone()}</button>
            }
        })
        .collect_view();
    let notes = (r.notes > 0).then(|| {
        view! { <span class="meta">{words::fb_notes_n(lang, &words::number(lang, u64::from(r.notes)))}</span> }
    });
    let dup = r
        .duplicate_of
        .map(|d| view! { <span class="meta">{words::fb_dup_of(lang, &short_id(&d))}</span> });
    let record = r.record_bytes.map(|b| {
        let by = (r.record_origin == Some(central_api::reports::RecordOrigin::Client))
            .then(|| format!(" · {}", words::fb_by_client_short(lang)));
        view! { <span class="meta">{words::fb_record_mark(lang, &size(lang, u64::from(b)))}{by}</span> }
    });
    // an address built from the number alone, in Baylee's repository
    let issue = r
        .issue
        .and_then(|n| issue_url(n).map(|href| (n, href)))
        .map(|(n, href)| {
            view! {
                <a class="fb-issue" href=href target="_blank" rel="noopener noreferrer">
                    {format!("#{n}")}<span class="visually-hidden">{" "}{words::fb_new_tab(lang)}</span>
                </a>
            }
        });
    view! {
        <span class="fb-marks">
            <span class="fb-status-inline">{status_pill(lang, r.status)}</span>
            {labels}{notes}{dup}{record}{issue}
        </span>
    }
}

/// The row's assignee, gateway and reporter, each a filter.
fn row_people(l: &List, r: &ReportSummary) -> impl IntoView + use<> {
    let l = *l;
    let lang = l.c.lang;
    let assignee = match r.assignee {
        None => view! { <span class="meta">{"—"}</span> }.into_any(),
        Some(a) => {
            let name = l.c.who(a);
            let title = words::fb_only_assignee(lang, &name);
            view! {
                <button type="button" class="fb-linkish" title=title.clone() aria-label=title
                    on:click=move |_| l.set(|q| q.assignee = Some(Assignee::Admin(a)))><bdi translate="no">{name.clone()}</bdi></button>
            }
            .into_any()
        }
    };
    let gateway = if r.channel == Channel::Direct {
        view! { <span class="meta">{words::fb_channel_direct(lang)}</span> }.into_any()
    } else {
        let name = r.gateway.clone();
        let pick = name.clone();
        let title = words::fb_only_gateway(lang, &name);
        view! {
            <button type="button" class="fb-linkish" title=title.clone() aria-label=title
                on:click=move |_| l.set(|q| q.gateway = Some(pick.clone()))><bdi translate="no">{name}</bdi></button>
        }
        .into_any()
    };
    let (name, short) = alias(&r.reporter);
    let pick = r.reporter.clone();
    let title = words::fb_only_reporter(lang, &format!("{name} {short}"));
    view! {
        <td>{assignee}</td>
        <td>{gateway}</td>
        <td>
            <button type="button" class="fb-linkish fb-reporter" translate="no" title=title.clone() aria-label=title
                on:click=move |_| l.set(|q| q.reporter = Some(pick.clone()))>
                <span>{name}</span>" "<code>{short}</code>
            </button>
        </td>
    }
}

fn row_view(l: &List, r: &ReportSummary) -> impl IntoView + use<> {
    let l = *l;
    let lang = l.c.lang;
    let id = r.id;
    let key = hex(&id);
    let q = l.query.get_untracked();
    let line = first_line(&r.excerpt, 120);
    let short = first_line(&r.excerpt, 40);
    let what = if short.is_empty() {
        short_id(&id)
    } else {
        short
    };
    let unread = r.status == ReportStatus::New && !r.seen;
    let checked = move || l.checked.with(|c| c.contains(&id));
    let toggle = move |_| {
        l.checked.update(|c| {
            if let Some(at) = c.iter().position(|x| *x == id) {
                c.remove(at);
            } else {
                c.push(id);
            }
        });
    };
    let link_text = if line.is_empty() {
        words::fb_no_text(lang).to_owned()
    } else {
        line
    };
    let dot = unread.then(|| {
        view! {
            <span class="fb-dot" title=words::fb_unread(lang)><span class="visually-hidden">{words::fb_unread(lang)}": "</span></span>
        }
    });
    let priority = match r.priority {
        Some(p) => priority_pill(lang, Some(p)).into_any(),
        None => view! { <span class="meta">{"—"}</span> }.into_any(),
    };
    view! {
        <tr class:fb-unread=unread class:fb-checked=checked>
            <td class="fb-check">
                <label class="fb-hit" for=format!("fb-check-{key}")>
                    <input type="checkbox" id=format!("fb-check-{key}") prop:checked=checked on:change=toggle />
                    <span class="visually-hidden">{words::fb_select_one(lang, &what)}</span>
                </label>
            </td>
            <th scope="row" class="fb-cell-report">
                {dot}
                <a href=report_path(l.code(), &id, &q) dir="auto" id=format!("fb-open-{key}")>{link_text}</a>
                {row_marks(&l, r)}
            </th>
            <td class="fb-when">{when(lang, r.created_at)}</td>
            <td>{kind_pill(lang, r.kind)}</td>
            <td class="fb-status">{status_pill(lang, r.status)}</td>
            <td>{priority}</td>
            {row_people(&l, r)}
        </tr>
    }
}

fn table_view(l: &List) -> impl IntoView + use<> {
    let l = *l;
    let lang = l.c.lang;
    let rows = move || l.rows().iter().map(|r| row_view(&l, r)).collect_view();
    let all = move || {
        let rows = l.rows();
        !rows.is_empty() && l.checked.with(|c| rows.iter().all(|r| c.contains(&r.id)))
    };
    let some = move || l.checked.with(|c| !c.is_empty()) && !all();
    view! {
        <div class="fb-table-box" role="region" aria-label=words::fb_reports(lang) tabindex="0"
            hidden=move || l.load.with(|load| !matches!(load, Load::Ready(rows, _) if !rows.is_empty()))>
            <table class="fb-table">
                <thead>
                    <tr>
                        <td class="fb-check">
                            <label class="fb-hit" for="fb-check-all">
                                <input type="checkbox" id="fb-check-all" prop:checked=all prop:indeterminate=some
                                    on:change=move |_| {
                                        let rows = l.rows();
                                        if all() { l.checked.set(Vec::new()) } else { l.checked.set(rows.iter().map(|r| r.id).collect()) }
                                    } />
                                <span class="visually-hidden">{words::fb_select_all(lang)}</span>
                            </label>
                        </td>
                        <th scope="col">{words::fb_col_report(lang)}</th>
                        {sort_head(&l, ReportSort::Received, words::fb_sort_received(lang), "")}
                        {sort_head(&l, ReportSort::Kind, words::fb_kind(lang), "")}
                        {sort_head(&l, ReportSort::Status, words::fb_status(lang), "fb-status")}
                        {sort_head(&l, ReportSort::Priority, words::fb_priority(lang), "")}
                        <th scope="col">{words::fb_assignee(lang)}</th>
                        {sort_head(&l, ReportSort::Gateway, words::fb_gateway(lang), "")}
                        {sort_head(&l, ReportSort::Reporter, words::fb_reporter(lang), "")}
                    </tr>
                </thead>
                <tbody>{rows}</tbody>
            </table>
        </div>
    }
}

fn pager_view(l: &List) -> impl IntoView + use<> {
    let l = *l;
    let lang = l.c.lang;
    let pages = move || {
        let size = u64::from(l.query.with(|q| q.limit).max(1));
        l.total().div_ceil(size).max(1)
    };
    let page = move || u64::from(l.query.with(|q| q.page));
    let to = move |p: u64| {
        let p = u32::try_from(p.clamp(1, pages())).unwrap_or(1);
        let mut q = l.query.get_untracked();
        q.page = p;
        l.go(q);
    };
    let first = move || page() <= 1;
    let last = move || page() >= pages();
    let size_now = Signal::derive(move || l.query.with(|q| q.limit.to_string()));
    let sizes = PAGE_SIZES
        .iter()
        .map(|s| {
            let v = s.to_string();
            let c = v.clone();
            view! { <option value=v prop:selected=move || size_now.get() == c>{words::number(lang, u64::from(*s))}</option> }
        })
        .collect_view();
    view! {
        <nav class="fb-pager" aria-label=words::fb_pages(lang)
            hidden=move || l.total() <= u64::from(l.query.with(|q| q.limit))>
            <button type="button" class="round-btn" id="fb-page-prev" disabled=first on:click=move |_| {
                to(page() - 1);
                focus::after_render("#fb-page-prev:not([disabled]), #fb-page-next");
            }>
                {glyph::glyph(PREVIOUS)}<span class="visually-hidden">{words::fb_prev_page(lang)}</span>
            </button>
            <span class="meta">{move || words::fb_page_of(lang, &words::number(lang, page()), &words::number(lang, pages()))}</span>
            <button type="button" class="round-btn" id="fb-page-next" disabled=last on:click=move |_| {
                to(page() + 1);
                focus::after_render("#fb-page-next:not([disabled]), #fb-page-prev");
            }>
                {glyph::glyph(NEXT)}<span class="visually-hidden">{words::fb_next_page(lang)}</span>
            </button>
            <label class="meta" for="fb-page-size">{words::fb_page_size(lang)}</label>
            <select id="fb-page-size" on:change=move |ev| {
                let v: u32 = event_target_value(&ev).parse().unwrap_or(central_api::reports::PAGE);
                l.set(|q| q.limit = v);
            }>{sizes}</select>
        </nav>
    }
}

fn groups_view(l: &List) -> impl IntoView + use<> {
    let l = *l;
    let lang = l.c.lang;
    move || {
        if !l.groups.get() {
            return None;
        }
        Some(match l.crash.get() {
            Lazy::Idle | Lazy::Loading => {
                view! { <p class="meta" role="status">{words::account_loading(lang)}</p> }.into_any()
            }
            Lazy::Failed(text) => view! {
                <p class="error" role="alert">{text}</p>
                <p><button type="button" class="button" on:click=move |_| l.read_groups()>{words::error_again(lang)}</button></p>
            }
            .into_any(),
            Lazy::Ready(groups) if groups.is_empty() => {
                view! { <p class="meta fb-empty">{words::fb_no_groups(lang)}</p> }.into_any()
            }
            Lazy::Ready(groups) => {
                let rows = groups
                    .into_iter()
                    .map(|g| {
                        let signature = g.signature.clone();
                        view! {
                            <tr>
                                <th scope="row" class="fb-cell-report">
                                    <code class="fb-code">{g.message}</code>
                                    <span class="fb-marks meta"><code>{g.location}</code></span>
                                </th>
                                <td class="num">{words::number(lang, g.count)}</td>
                                <td class="num">{words::number(lang, g.open)}</td>
                                <td class="fb-when">{when(lang, g.first)}</td>
                                <td class="fb-when">{when(lang, g.last)}</td>
                                <td>
                                    <button type="button" class="button button-ghost button-sm" on:click=move |_| {
                                        let s = signature.clone();
                                        l.groups.set(false);
                                        l.set(|q| q.signature = Some(s));
                                        focus::after_render("#fb-mode-list");
                                    }>{words::fb_show_group(lang)}</button>
                                </td>
                            </tr>
                        }
                    })
                    .collect_view();
                view! {
                    <div class="fb-table-box" role="region" aria-label=words::fb_groups(lang) tabindex="0">
                        <table class="fb-table">
                            <thead>
                                <tr>
                                    <th scope="col">{words::fb_col_crash(lang)}</th>
                                    <th scope="col">{words::fb_col_count(lang)}</th>
                                    <th scope="col">{words::fb_col_open(lang)}</th>
                                    <th scope="col">{words::fb_col_first(lang)}</th>
                                    <th scope="col">{words::fb_col_last(lang)}</th>
                                    <th scope="col"><span class="visually-hidden">{words::fb_show_group(lang)}</span></th>
                                </tr>
                            </thead>
                            <tbody>{rows}</tbody>
                        </table>
                    </div>
                }
                .into_any()
            }
        })
    }
}

fn results_view(l: &List) -> impl IntoView + use<> {
    let l = *l;
    let lang = l.c.lang;
    let line = move || match l.load.get() {
        Load::Ready(rows, total) if total == 0 || rows.is_empty() => {
            words::fb_none(lang).to_owned()
        }
        Load::Ready(rows, total) => {
            let first = u64::from((l.query.with(|q| q.page) - 1) * l.query.with(|q| q.limit));
            let unread = rows
                .iter()
                .filter(|r| r.status == ReportStatus::New && !r.seen)
                .count();
            let range = words::fb_range(
                lang,
                &words::number(lang, first + 1),
                &words::number(lang, first + count(rows.len())),
                &words::number(lang, total),
            );
            if unread > 0 {
                format!(
                    "{range} · {}",
                    words::fb_unread_here(lang, &words::number(lang, count(unread)))
                )
            } else {
                range
            }
        }
        _ => String::new(),
    };
    let state = move || {
        match l.load.get() {
        Load::Loading => Some(view! { <p class="meta" role="status">{words::account_loading(lang)}</p> }.into_any()),
        Load::Failed(text) => Some(
            view! {
                <p class="error" role="alert">{text}</p>
                <p><button type="button" class="button" on:click=move |_| l.again()>{words::error_again(lang)}</button></p>
            }
            .into_any(),
        ),
        Load::Ready(..) => None,
    }
    };
    let export_all = move || {
        let total = l.total();
        (total > 0 && !l.groups.get()).then(|| {
            let n = words::number(lang, total.min(EXPORT_MAX));
            view! {
                <button type="button" class="button button-ghost button-sm" aria-disabled=move || l.exporting.get().then_some("true")
                    on:click=move |_| l.export_all()>
                    {glyph::glyph(DOWNLOAD)}<span>{words::fb_export_all(lang, &n)}</span>
                </button>
            }
        })
    };
    let mode = move || {
        (!l.query.with(|q| q.trash)).then(|| {
            view! {
                <fieldset class="fb-mode">
                    <legend class="visually-hidden">{words::fb_shown_as(lang)}</legend>
                    <button type="button" class="chip-btn" id="fb-mode-list" aria-pressed=move || (!l.groups.get()).to_string()
                        on:click=move |_| l.groups.set(false)>{words::fb_as_list(lang)}</button>
                    <button type="button" class="chip-btn" id="fb-mode-groups" aria-pressed=move || l.groups.get().to_string()
                        on:click=move |_| {
                            if !l.groups.get_untracked() {
                                l.groups.set(true);
                                l.read_groups();
                            }
                        }>{words::fb_as_groups(lang)}</button>
                </fieldset>
            }
        })
    };
    view! {
        <div class="fb-countline">
            <p class="meta" role="status">{line}</p>
            {export_all}
            {mode}
        </div>
        {bulk_view(&l)}
        {move || l.purge.view()}
        <div class="fb-results" aria-busy=move || l.busy.get().to_string() hidden=move || l.groups.get()>
            {state}
            {table_view(&l)}
            {pager_view(&l)}
        </div>
        {groups_view(&l)}
    }
}

/// The page for an administrator.
pub fn page(c: Ctx) -> AnyView {
    let lang = c.lang;
    let start = query::read(&address());
    let l = List {
        c,
        query: RwSignal::new(start.clone()),
        load: RwSignal::new(Load::Loading),
        busy: RwSignal::new(true),
        generation: StoredValue::new(0),
        facets: RwSignal::new(None),
        facet_scope: StoredValue::new(None),
        counts: RwSignal::new([None; 7]),
        checked: RwSignal::new(Vec::new()),
        filters_open: RwSignal::new(query::panel_count(&start, view_of(&start, c.me)) > 0),
        groups: RwSignal::new(false),
        crash: RwSignal::new(Lazy::Idle),
        working: RwSignal::new(false),
        exporting: RwSignal::new(false),
        typed: RwSignal::new(start.q.clone().unwrap_or_default()),
        typing: StoredValue::new(0),
        saved: RwSignal::new(Lazy::Idle),
        view_name: RwSignal::new(String::new()),
        bulk_label: RwSignal::new(String::new()),
        purge: Purge::new(c, false, "#fb-bulk-purge"),
    };
    // deleting for good ends here: said, the selection gone, the list read again
    Effect::new(move |_| {
        let Some(how) = l.purge.ended.get() else {
            return;
        };
        l.purge.ended.set(None);
        match how {
            Purged::Done(n) => {
                c.notices
                    .say(words::fb_purged_n(lang, &words::number(lang, u64::from(n))));
                l.checked.set(Vec::new());
            }
            Purged::Refused => c.notices.fail(words::fb_purge_refused(lang)),
            Purged::Lost => c.notices.fail(words::fb_purge_lost(lang)),
        }
        l.again();
        focus::after_render("#fb-list-title");
    });
    l.c.read_admins();
    // Effects run in the browser only: the list, its counts and facets.
    Effect::new(move |_| {
        l.read();
        l.recount();
        l.read_facets(true);
    });
    let toggle = move |_| l.filters_open.update(|open| *open = !*open);
    let filters_label = move || {
        let n = l.query.with(|q| query::panel_count(q, view_of(q, l.c.me)));
        if n > 0 {
            words::fb_filters_n(lang, &words::number(lang, count(n)))
        } else {
            words::fb_filters(lang).to_owned()
        }
    };
    let trash_hint = move || {
        l.query.with(|q| q.trash).then(|| {
            let days = words::number(lang, u64::from(TRASH_DAYS));
            view! { <p class="meta">{words::fb_trash_hint(lang, &days)}</p> }
        })
    };
    let overview = format!("/{}/account/admin/overview", lang.code());
    view! {
        <p class="fb-back"><a href=overview class="text-link">{glyph::glyph(glyph::BACK)}<span>{words::fb_to_overview(lang)}</span></a></p>
        <div class="card fb-panel">
            <div class="fb-panel-head">
                <h2 id="fb-list-title" tabindex="-1">{words::fb_reports(lang)}</h2>
                <button type="button" class="button button-ghost button-sm" aria-busy=move || l.busy.get().to_string()
                    on:click=move |_| l.again()>{glyph::glyph(REFRESH)}<span>{words::fb_refresh(lang)}</span></button>
            </div>
            {views_view(&l)}
            <div class="fb-toolbar">
                {search_view(&l)}
                <button type="button" class="button button-ghost button-sm" id="fb-filters-toggle" aria-controls="fb-filters"
                    aria-expanded=move || l.filters_open.get().to_string() on:click=toggle>
                    {glyph::glyph(FILTER)}<span>{filters_label}</span>
                </button>
                {saved_view(&l)}
            </div>
            {filters_view(&l)}
            {pills_view(&l)}
            {chart::view(lang)}
            {trash_hint}
            {results_view(&l)}
        </div>
        {notices_view(lang, l.c.notices)}
    }
    .into_any()
}
